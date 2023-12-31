use std::{
    ops::Range,
    sync::{LazyLock, Mutex, OnceLock},
};

use image::RgbaImage;
use log::{error, info, warn};
use wgpu::{
    BufferUsages, ColorTargetState, FragmentState, MultisampleState, RenderPipelineDescriptor,
    ShaderModuleDescriptor, VertexState,
};

use crate::{
    elements::{
        immediate_geometry::TexturedInstancesQueue,
        texture::{create_white_px_texture, rgba_bind_group_layout},
        BindableTexture, Color, GrowableBuffer, ToRaw, Transform, TransformRaw,
    },
    modules::{
        arenas::Key,
        renderer::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT},
        Arenas, Attribute, GraphicsContext, MainCamera3D, MainScreenSize, Prepare, Renderer,
        VertexT,
    },
    utils::Timing,
    Dependencies, Handle, Module,
};

use super::{ui_rect::UiRect, MainPassRenderer};

// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl WorldRectRenderer {
    pub fn draw_textured_rect(
        &mut self,
        rect: UiRect,
        transform: Transform,
        texture: Key<BindableTexture>,
    ) {
        self.queue.add(
            WorldRect {
                ui_rect: rect,
                transform: transform.to_raw(),
            },
            texture,
        );
    }

    pub fn draw_rect(&mut self, rect: UiRect, transform: Transform) {
        self.queue.add(
            WorldRect {
                ui_rect: rect,
                transform: transform.to_raw(),
            },
            self.white_px_texture_key,
        );
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Module
// /////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Dependencies)]
pub struct Deps {
    renderer: Handle<Renderer>,
    ctx: Handle<GraphicsContext>,
    cam: Handle<MainCamera3D>,
    arenas: Handle<Arenas>,
}

/// Pretty much a copy paste of UiRectRenderer, but we want to stay flexible, so keep both duplicated for now, with their own minor adjustments.
/// Let's not abstract too early.
pub struct WorldRectRenderer {
    pipeline: wgpu::RenderPipeline,
    white_px_texture_key: Key<BindableTexture>,
    queue: TexturedInstancesQueue<WorldRect>,
    instance_ranges: Vec<(Range<u32>, Key<BindableTexture>)>,
    instance_buffer: GrowableBuffer<WorldRect>,
    deps: Deps,
}

impl Module for WorldRectRenderer {
    type Config = ();

    type Dependencies = Deps;

    fn new(config: Self::Config, mut deps: Self::Dependencies) -> anyhow::Result<Self> {
        let white_texture = create_white_px_texture(&deps.ctx.device, &deps.ctx.queue);
        let white_px_texture_key = deps.arenas.textures_mut().insert(white_texture);
        let pipeline =
            create_render_pipeline(&deps.ctx.device, include_str!("world_rect.wgsl"), &deps.cam);

        Ok(WorldRectRenderer {
            pipeline,
            instance_ranges: vec![],
            instance_buffer: GrowableBuffer::new(&deps.ctx.device, 512, BufferUsages::VERTEX),
            white_px_texture_key,
            queue: TexturedInstancesQueue::new(),
            deps,
        })
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut renderer = handle.deps.renderer;
        renderer.register_prepare(handle);
        renderer.register_main_pass_renderer(handle, Timing::LATE);
        Ok(())
    }
}

impl Prepare for WorldRectRenderer {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // todo! queue.clear should handle z-sorting back to front. Then disable z-buffer writes.
        let (instances, ranges) = self.queue.clear();
        self.instance_ranges = ranges;
        self.instance_buffer.prepare(&instances, device, queue);
    }
}

impl MainPassRenderer for WorldRectRenderer {
    fn render<'pass, 'encoder>(&'encoder self, render_pass: &'pass mut wgpu::RenderPass<'encoder>) {
        if self.instance_ranges.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, self.deps.cam.bind_group(), &[]);
        // set the instance buffer: (no vertex buffer is used, instead just one big instance buffer that contains the sorted texture group ranges.)
        render_pass.set_vertex_buffer(0, self.instance_buffer.buffer().slice(..));

        // 6 indices to draw two triangles
        const VERTEX_COUNT: u32 = 6;
        let textures = self.deps.arenas.textures();
        for (range, texture) in self.instance_ranges.iter() {
            let Some(texture) = textures.get(*texture) else {
                warn!("Texture with key {texture:?} does not exist and cannot be rendered for a UI Rect");
                continue;
            };
            render_pass.set_bind_group(1, &texture.bind_group, &[]);
            render_pass.draw(0..VERTEX_COUNT, range.start..range.end);
        }
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Rendering
// /////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorldRect {
    pub ui_rect: UiRect,
    pub transform: TransformRaw,
}

impl VertexT for WorldRect {
    const ATTRIBUTES: &'static [Attribute] = &[
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

fn create_render_pipeline(
    device: &wgpu::Device,
    wgsl: &str,
    cam: &MainCamera3D,
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
        bind_group_layouts: &[cam.bind_group_layout(), rgba_bind_group_layout(device)],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(&format!("{label} Pipeline")),
        layout: Some(&layout),
        vertex: VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: vertex_buffers_layout,
        },
        fragment: Some(FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_COLOR_FORMAT,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(), // does not really matter because no index and vertex buffer is
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: MultisampleState {
            count: MSAA_SAMPLE_COUNT,
            alpha_to_coverage_enabled: true,
            ..Default::default()
        },
        multiview: None,
    })
}
