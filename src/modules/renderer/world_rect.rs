use std::ops::Range;

use log::warn;
use wgpu::{
    BufferUsages, FragmentState, MultisampleState, RenderPipelineDescriptor,
    ShaderModuleDescriptor, VertexState,
};

use crate::{
    elements::{
        camera3d::Camera3dGR,
        immediate_geometry::TexturedInstancesQueue,
        texture::{create_white_px_texture, rgba_bind_group_layout},
        BindableTexture, GrowableBuffer, ToRaw, Transform, TransformRaw,
    },
    modules::{
        renderer::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT},
        Attribute, GraphicsContext, VertexT,
    },
    OwnedPtr, Prepare, Ptr,
};

use super::ui_rect::UiRect;

// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl WorldRectRenderer {
    pub fn draw_textured_rect(
        &mut self,
        rect: UiRect,
        transform: Transform,
        texture: Ptr<BindableTexture>,
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
            self.white_texture.ptr(),
        );
    }
}

/// Pretty much a copy paste of UiRectRenderer, but we want to stay flexible, so keep both duplicated for now, with their own minor adjustments.
/// Let's not abstract too early.
pub struct WorldRectRenderer {
    pipeline: wgpu::RenderPipeline,
    white_texture: OwnedPtr<BindableTexture>,
    queue: TexturedInstancesQueue<WorldRect>,
    instance_ranges: Vec<(Range<u32>, Ptr<BindableTexture>)>,
    instance_buffer: GrowableBuffer<WorldRect>,
}

impl WorldRectRenderer {
    pub fn new(ctx: &GraphicsContext, camera: &Camera3dGR) -> Self {
        let white_texture = OwnedPtr::new(create_white_px_texture(&ctx.device, &ctx.queue));
        let pipeline = create_render_pipeline(&ctx.device, include_str!("world_rect.wgsl"), camera);

        WorldRectRenderer {
            pipeline,
            instance_ranges: vec![],
            instance_buffer: GrowableBuffer::new(&ctx.device, 512, BufferUsages::VERTEX),
            white_texture,
            queue: TexturedInstancesQueue::new(),
        }
    }

    pub fn render<'encoder>(
        &'encoder self,
        render_pass: &mut wgpu::RenderPass<'encoder>,
        camera: &'encoder Camera3dGR,
    ) {
        if self.instance_ranges.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera.bind_group(), &[]);
        // set the instance buffer: (no vertex buffer is used, instead just one big instance buffer that contains the sorted texture group ranges.)
        render_pass.set_vertex_buffer(0, self.instance_buffer.buffer().slice(..));

        // 6 indices to draw two triangles
        const VERTEX_COUNT: u32 = 6;
        for (range, texture) in self.instance_ranges.iter() {
            render_pass.set_bind_group(1, &texture.bind_group, &[]);
            render_pass.draw(0..VERTEX_COUNT, range.start..range.end);
        }
    }
}

impl Prepare for WorldRectRenderer {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) {
        // todo! queue.clear should handle z-sorting back to front. Then disable z-buffer writes.
        let (instances, ranges) = self.queue.clear();
        self.instance_ranges = ranges;
        self.instance_buffer.prepare(&instances, device, queue);
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
    camera: &Camera3dGR,
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
        bind_group_layouts: &[camera.bind_group_layout(), rgba_bind_group_layout(device)],
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
