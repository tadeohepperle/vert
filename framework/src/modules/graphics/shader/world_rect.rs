use std::{
    ops::Range,
    sync::{LazyLock, Mutex},
};

use log::warn;
use wgpu::{
    FragmentState, MultisampleState, PrimitiveState, RenderPipelineDescriptor,
    ShaderModuleDescriptor, VertexState,
};

use crate::{
    constants::DEPTH_FORMAT,
    modules::{
        assets::asset_store::{AssetStore, Key},
        graphics::{
            elements::{
                buffer::{GrowableBuffer, ToRaw},
                texture::BindableTexture,
                transform::{Transform, TransformRaw},
            },
            graphics_context::GraphicsContext,
            renderer::PipelineSettings,
            settings::GraphicsSettings,
            statics::{camera::Camera, static_texture::RgbaBindGroupLayout, StaticBindGroup},
        },
    },
    utils::watcher::{FileChangeWatcher, ShaderFileWatcher},
    wgsl_file,
};

use super::{
    ui_rect::{TexturedInstancesQueue, UiRect, WHITE_TEXTURE_KEY},
    Attribute, RendererT, VertexT, FRAGMENT_ENTRY_POINT, VERTEX_ENTRY_POINT,
};

// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl WorldRectRenderer {
    pub fn draw_textured_rect(rect: UiRect, transform: Transform, texture: Key<BindableTexture>) {
        let mut queue = WORLD_RECT_QUEUE.lock().unwrap();
        queue.add(
            WorldRect {
                ui_rect: rect,
                transform: transform.to_raw(),
            },
            texture,
        );
    }

    pub fn draw_rect(rect: UiRect, transform: Transform) {
        let mut queue = WORLD_RECT_QUEUE.lock().unwrap();
        queue.add(
            WorldRect {
                ui_rect: rect,
                transform: transform.to_raw(),
            },
            *WHITE_TEXTURE_KEY.get().unwrap(),
        );
    }
}

pub static WORLD_RECT_QUEUE: LazyLock<Mutex<TexturedInstancesQueue<WorldRect>>> =
    LazyLock::new(|| Mutex::new(TexturedInstancesQueue::new()));

// /////////////////////////////////////////////////////////////////////////////
// Renderer
// /////////////////////////////////////////////////////////////////////////////

pub struct WorldRectRenderer {
    pipeline: wgpu::RenderPipeline,
    watcher: ShaderFileWatcher,
    pipeline_settings: PipelineSettings,

    instance_ranges: Vec<(Range<u32>, Key<BindableTexture>)>,
    instances: Vec<WorldRect>,
    instance_buffer: GrowableBuffer<WorldRect>,
}

impl RendererT for WorldRectRenderer {
    fn new(context: &GraphicsContext, pipeline_settings: PipelineSettings) -> Self
    where
        Self: Sized,
    {
        let device = &context.device;
        let wgsl = include_str!("world_rect.wgsl");
        let watcher = ShaderFileWatcher::new(&wgsl_file!());
        let pipeline = create_render_pipeline(device, pipeline_settings.clone(), wgsl);
        let instance_buffer = GrowableBuffer::new(device, 512, wgpu::BufferUsages::VERTEX);

        // WHITE_TEXTURE_KEY should be set as well, but we rely that the UiRectRenderer already did that.
        WorldRectRenderer {
            pipeline,
            watcher,
            pipeline_settings,
            instance_ranges: vec![],
            instances: vec![],
            instance_buffer,
        }
    }

    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder) {
        if let Some(new_wgsl) = self.watcher.check_for_changes() {
            let pipeline =
                create_render_pipeline(&context.device, self.pipeline_settings.clone(), &new_wgsl);
            self.pipeline = pipeline;
        }

        let mut rects = WORLD_RECT_QUEUE.lock().unwrap();
        let (instances, ranges) = rects.clear();
        self.instances = instances;
        self.instance_ranges = ranges;
        self.instance_buffer
            .prepare(&self.instances, &context.queue, &context.device);
    }

    fn render<'pass, 'encoder>(
        &'encoder self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        graphics_settings: &GraphicsSettings,
        asset_store: &'encoder AssetStore<'encoder>,
    ) {
        if self.instance_ranges.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, Camera::bind_group(), &[]);
        // set the instance buffer: (no vertex buffer is used, instead just one big instance buffer that contains the sorted texture group ranges.)
        render_pass.set_vertex_buffer(0, self.instance_buffer.buffer().slice(..));

        // 6 indices to draw two triangles
        const VERTEX_COUNT: u32 = 6;
        for (range, texture) in self.instance_ranges.iter() {
            let Some(texture) = asset_store.textures().get(*texture) else {
                warn!("Texture with key {texture:?} does not exist and cannot be rendered for a World Rect");
                continue;
            };
            render_pass.set_bind_group(1, &texture.bind_group, &[]);
            render_pass.draw(0..VERTEX_COUNT, range.start..range.end);
        }
    }

    // fn depth_stencil() -> Option<wgpu::DepthStencilState>
    // where
    //     Self: Sized,
    // {
    //     Some(wgpu::DepthStencilState {
    //         format: DEPTH_FORMAT,
    //         depth_write_enabled: false, // important
    //         depth_compare: wgpu::CompareFunction::Less,
    //         stencil: wgpu::StencilState::default(),
    //         bias: wgpu::DepthBiasState::default(),
    //     })
    // }

    // fn color_target_state(format: wgpu::TextureFormat) -> wgpu::ColorTargetState
    // where
    //     Self: Sized,
    // {
    //     wgpu::ColorTargetState {
    //         format,
    //         blend: Some(wgpu::BlendState {
    //             alpha: wgpu::BlendComponent::OVER,
    //             color: wgpu::BlendComponent::OVER,
    //         }),
    //         write_mask: wgpu::ColorWrites::ALL,
    //     }
    // }

    // fn primitive() -> wgpu::PrimitiveState
    // where
    //     Self: Sized,
    // {
    //     PrimitiveState {
    //         topology: wgpu::PrimitiveTopology::TriangleList,
    //         strip_index_format: None,
    //         front_face: wgpu::FrontFace::Ccw,
    //         cull_mode: Some(wgpu::Face::Back), // this renders both sides of the text. Not sure if it will stay like this
    //         unclipped_depth: false,
    //         polygon_mode: wgpu::PolygonMode::Fill,
    //         conservative: false,
    //     }
    // }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    settings: PipelineSettings,
    wgsl: &str,
) -> wgpu::RenderPipeline {
    let label = "WorldRect";
    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(&format!("{label} ShaderModule")),
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });

    let _empty = &mut vec![];
    let vertex_buffers_layout = &[WorldRect::vertex_buffer_layout(0, true, _empty)];

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} PipelineLayout")),
        bind_group_layouts: &[Camera::bind_group_layout(), RgbaBindGroupLayout.get()],
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
            targets: &[Some(WorldRectRenderer::color_target_state(settings.format))],
        }),
        primitive: WorldRectRenderer::primitive(),
        depth_stencil: WorldRectRenderer::depth_stencil(),
        multisample: MultisampleState {
            count: settings.multisample.count,
            alpha_to_coverage_enabled: true,
            ..Default::default()
        },
        multiview: None,
    })
}

// /////////////////////////////////////////////////////////////////////////////
// Data
// /////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorldRect {
    pub ui_rect: UiRect,
    pub transform: TransformRaw,
}

impl VertexT for WorldRect {
    const ATTRIBUTES: &'static [super::Attribute] = &[
        Attribute::new("pos", wgpu::VertexFormat::Float32x4),
        Attribute::new("uv", wgpu::VertexFormat::Float32x4),
        Attribute::new("color", wgpu::VertexFormat::Float32x4),
        Attribute::new("border_radius", wgpu::VertexFormat::Float32x4),
        Attribute::new("col1", wgpu::VertexFormat::Float32x4),
        Attribute::new("col2", wgpu::VertexFormat::Float32x4),
        Attribute::new("col3", wgpu::VertexFormat::Float32x4),
        Attribute::new("translation", wgpu::VertexFormat::Float32x4),
    ];
}
