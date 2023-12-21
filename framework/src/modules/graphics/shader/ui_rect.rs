use std::{
    ops::Range,
    sync::{LazyLock, Mutex, OnceLock},
};

use image::RgbaImage;
use log::{error, info, warn};
use wgpu::{FragmentState, RenderPipelineDescriptor, ShaderModuleDescriptor, VertexState};

use crate::{
    constants::DEPTH_FORMAT,
    modules::{
        assets::asset_store::{AssetStore, Key},
        graphics::{
            elements::{
                buffer::GrowableBuffer,
                color::Color,
                texture::{BindableTexture, Texture},
            },
            graphics_context::GraphicsContext,
            renderer::PipelineSettings,
            statics::{
                screen_size::ScreenSize, static_texture::RgbaBindGroupLayout, StaticBindGroup,
            },
        },
    },
    utils::watcher::FileChangeWatcher,
    wgsl_file,
};

use super::{Attribute, RendererT, VertexT, FRAGMENT_ENTRY_POINT, VERTEX_ENTRY_POINT};

// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl UiRectRenderer {
    pub fn draw_textured_rect(rect: UiRect, texture: Key<BindableTexture>) {
        let mut queue = UI_RECT_QUEUE.lock().unwrap();
        queue.add(rect, texture);
    }

    pub fn draw_rect(rect: UiRect) {
        let mut queue = UI_RECT_QUEUE.lock().unwrap();
        queue.add(rect, *WHITE_TEXTURE_KEY.get().unwrap());
    }
}

static UI_RECT_QUEUE: LazyLock<Mutex<TexturedInstancesQueue<UiRect>>> =
    LazyLock::new(|| Mutex::new(TexturedInstancesQueue::new()));

#[derive(Debug)]
pub struct TexturedInstancesQueue<T: bytemuck::Pod> {
    pub instances: Vec<(T, Key<BindableTexture>)>,
}

impl<T: bytemuck::Pod> TexturedInstancesQueue<T> {
    #[inline(always)]
    pub fn add(&mut self, instance: T, texture: Key<BindableTexture>) {
        self.instances.push((instance, texture));
    }

    pub fn new() -> Self {
        TexturedInstancesQueue { instances: vec![] }
    }

    fn clear(&mut self) -> (Vec<T>, Vec<(Range<u32>, Key<BindableTexture>)>) {
        let mut textured_instances = std::mem::take(&mut self.instances);

        if textured_instances.is_empty() {
            return (vec![], vec![]);
        }

        textured_instances.sort_by(|(_, tex1), (_, tex2)| tex1.cmp(&tex2));

        let mut instances: Vec<T> = vec![];
        let mut texture_groups: Vec<(Range<u32>, Key<BindableTexture>)> = vec![];

        let mut last_start_idx: usize = 0;
        let mut last_texture: Key<BindableTexture> = textured_instances.first().unwrap().1.clone();

        for (i, (instance, texture)) in textured_instances.into_iter().enumerate() {
            instances.push(instance);
            if texture != last_texture {
                let range = (last_start_idx as u32)..(i as u32);
                texture_groups.push((range, last_texture));
                last_start_idx = i;
                last_texture = texture;
            }
        }

        if last_start_idx < instances.len() {
            let range = (last_start_idx as u32)..(instances.len() as u32);
            texture_groups.push((range, last_texture));
        }
        (instances, texture_groups)
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Renderer
// /////////////////////////////////////////////////////////////////////////////

pub struct UiRectRenderer {
    pipeline: wgpu::RenderPipeline,
    watcher: FileChangeWatcher,
    pipeline_settings: PipelineSettings,

    instance_ranges: Vec<(Range<u32>, Key<BindableTexture>)>,
    instances: Vec<UiRect>,
    instance_buffer: GrowableBuffer<UiRect>,
}

static WHITE_TEXTURE_KEY: OnceLock<Key<BindableTexture>> = OnceLock::new();

impl RendererT for UiRectRenderer {
    fn new(context: &GraphicsContext, pipeline_settings: PipelineSettings) -> Self
    where
        Self: Sized,
    {
        let device = &context.device;

        let wgsl = include_str!("ui_rect.wgsl");
        let watcher = FileChangeWatcher::new(&[&wgsl_file!()]);
        let pipeline = create_render_pipeline(device, pipeline_settings.clone(), wgsl);

        let instance_buffer = GrowableBuffer::new(device, 512, wgpu::BufferUsages::VERTEX);

        let white_texture = create_white_px_texture(context);
        let white_texture = AssetStore::lock().store_texture(white_texture);
        WHITE_TEXTURE_KEY.set(white_texture);

        UiRectRenderer {
            pipeline,
            watcher,
            pipeline_settings,
            instance_ranges: vec![],
            instances: vec![],
            instance_buffer,
        }
    }

    fn prepare(
        &mut self,
        context: &crate::modules::graphics::graphics_context::GraphicsContext,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // recreate pipeline if the wgsl file has changed:
        if let Some(_) = self.watcher.check_for_changes() {
            // load the wgsl and verify it:
            let wgsl_file = wgsl_file!();
            let wgsl = std::fs::read_to_string(&wgsl_file).unwrap();
            if let Err(err) = wgpu::naga::front::wgsl::parse_str(&wgsl) {
                error!("wgsl file at {wgsl_file} is invalid: {err}");
            } else {
                info!("Hot reloaded wgsl from {wgsl_file}");
                let pipeline =
                    create_render_pipeline(&context.device, self.pipeline_settings.clone(), &wgsl);
                self.pipeline = pipeline;
            }
        }

        let mut rects = UI_RECT_QUEUE.lock().unwrap();
        let (instances, ranges) = rects.clear();
        self.instances = instances;
        self.instance_ranges = ranges;
        self.instance_buffer
            .prepare(&self.instances, &context.queue, &context.device);
    }

    fn render<'pass, 'encoder>(
        &'encoder self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        _graphics_settings: &crate::modules::graphics::settings::GraphicsSettings,
        asset_store: &'encoder AssetStore<'encoder>,
    ) {
        if self.instance_ranges.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, ScreenSize::bind_group(), &[]);
        // set the instance buffer: (no vertex buffer is used, instead just one big instance buffer that contains the sorted texture group ranges.)
        render_pass.set_vertex_buffer(0, self.instance_buffer.buffer().slice(..));

        // 6 indices to draw two triangles
        const INDEX_COUNT: u32 = 6;
        for (range, texture) in self.instance_ranges.iter() {
            let Some(texture) = asset_store.get_texture(*texture) else {
                warn!("Texture with key {texture:?} does not exist and cannot be rendered for a UI Rect");
                continue;
            };
            render_pass.set_bind_group(1, &texture.bind_group, &[]);
            render_pass.draw_indexed(0..INDEX_COUNT, 0, range.start..range.end);
        }
    }

    fn depth_stencil() -> Option<wgpu::DepthStencilState>
    where
        Self: Sized,
    {
        Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false, // important
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        })
    }
}

fn create_white_px_texture(context: &GraphicsContext) -> BindableTexture {
    let mut white_px = RgbaImage::new(1, 1);
    white_px.get_pixel_mut(0, 0).0 = [255, 255, 255, 255];
    let texture = Texture::from_image(&context.device, &context.queue, &white_px);
    BindableTexture::new(context, texture)
}

fn create_render_pipeline(
    device: &wgpu::Device,
    settings: PipelineSettings,
    wgsl: &str,
) -> wgpu::RenderPipeline {
    let label = "ColorMeshRenderer";
    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(&format!("{label} ShaderModule")),
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });

    let _empty = &mut vec![];
    let vertex_buffers_layout = &[UiRect::vertex_buffer_layout(0, true, _empty)];

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} PipelineLayout")),
        bind_group_layouts: &[ScreenSize::bind_group_layout(), RgbaBindGroupLayout.get()],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(&format!("{label} Pipeline")),
        layout: Some(&layout),
        vertex: VertexState {
            module: &shader_module,
            entry_point: VERTEX_ENTRY_POINT,
            buffers: vertex_buffers_layout,
        },
        fragment: Some(FragmentState {
            module: &shader_module,
            entry_point: FRAGMENT_ENTRY_POINT,
            targets: &[Some(settings.target)],
        }),
        primitive: UiRectRenderer::primitive(),
        depth_stencil: UiRectRenderer::depth_stencil(),
        multisample: settings.multisample,
        multiview: None,
    })
}

// /////////////////////////////////////////////////////////////////////////////
// Data
// /////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiRect {
    pub pos: Rect,
    pub uv: Rect,
    pub color: Color,
    pub border_radius: [f32; 4],
}

impl VertexT for UiRect {
    const ATTRIBUTES: &'static [super::Attribute] = &[
        Attribute::new("pos", wgpu::VertexFormat::Float32x4),
        Attribute::new("uv", wgpu::VertexFormat::Float32x4),
        Attribute::new("color", wgpu::VertexFormat::Float32x4),
        Attribute::new("border_radius", wgpu::VertexFormat::Float32x4),
    ];
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Rect {
    /// min x, min y (top left corner)
    pub offset: [f32; 2],
    /// size x, size y
    pub size: [f32; 2],
}

impl Rect {
    pub const fn new(offset: [f32; 2], size: [f32; 2]) -> Self {
        Self { offset, size }
    }
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            size: [1.0, 1.0],
        }
    }
}
