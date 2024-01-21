use std::ops::Range;

use log::warn;
use wgpu::{
    BufferUsages, FragmentState, MultisampleState, RenderPipelineDescriptor,
    ShaderModuleDescriptor, VertexState,
};

use crate::{
    elements::{
        immediate_geometry::TexturedInstancesQueue,
        texture::{create_white_px_texture, rgba_bind_group_layout},
        BindableTexture, Color, GrowableBuffer, Rect, ScreenGR,
    },
    modules::{
        arenas::{Key, OwnedKey},
        renderer::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT},
        Arenas, Attribute, GraphicsContext, VertexT,
    },
    utils::Timing,
    Prepare,
};

// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl UiRectRenderer {
    pub fn draw_textured_rect(&mut self, rect: UiRect, texture: Key<BindableTexture>) {
        self.queue.add(rect, texture);
    }

    pub fn draw_rect(&mut self, rect: UiRect) {
        self.queue.add(rect, self.white_px_texture_key.key());
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Module
// /////////////////////////////////////////////////////////////////////////////

pub struct UiRectRenderer {
    pipeline: wgpu::RenderPipeline,
    white_px_texture_key: OwnedKey<BindableTexture>,
    queue: TexturedInstancesQueue<UiRect>,
    instance_ranges: Vec<(Range<u32>, Key<BindableTexture>)>,
    instance_buffer: GrowableBuffer<UiRect>,
}

impl UiRectRenderer {
    pub fn new(arenas: &mut Arenas, ctx: &GraphicsContext, screen: &ScreenGR) -> Self {
        let white_texture = create_white_px_texture(&ctx.device, &ctx.queue);
        let white_px_texture_key = arenas.insert(white_texture);

        let pipeline = create_render_pipeline(&ctx.device, include_str!("ui_rect.wgsl"), screen);

        UiRectRenderer {
            pipeline,
            instance_ranges: vec![],
            instance_buffer: GrowableBuffer::new(&ctx.device, 512, BufferUsages::VERTEX),
            white_px_texture_key,
            queue: TexturedInstancesQueue::new(),
        }
    }

    pub fn render<'encoder>(
        &'encoder self,
        render_pass: &mut wgpu::RenderPass<'encoder>,
        screen: &'encoder ScreenGR,
        arenas: &'encoder Arenas,
    ) {
        if self.instance_ranges.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, screen.bind_group(), &[]);
        // set the instance buffer: (no vertex buffer is used, instead just one big instance buffer that contains the sorted texture group ranges.)
        render_pass.set_vertex_buffer(0, self.instance_buffer.buffer().slice(..));

        // 6 indices to draw two triangles
        const VERTEX_COUNT: u32 = 6;
        let textures = arenas.arena::<BindableTexture>();
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

impl Prepare for UiRectRenderer {
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
pub struct UiRect {
    pub pos: Rect,
    pub uv: Rect,
    pub color: Color,
    pub border_radius: [f32; 4],
}

impl VertexT for UiRect {
    const ATTRIBUTES: &'static [Attribute] = &[
        Attribute::new("pos", wgpu::VertexFormat::Float32x4),
        Attribute::new("uv", wgpu::VertexFormat::Float32x4),
        Attribute::new("color", wgpu::VertexFormat::Float32x4),
        Attribute::new("border_radius", wgpu::VertexFormat::Float32x4),
    ];
}

fn create_render_pipeline(
    device: &wgpu::Device,
    wgsl: &str,
    screen: &ScreenGR,
) -> wgpu::RenderPipeline {
    let label = "UiRect";
    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(&format!("{label} ShaderModule")),
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });

    let _empty = &mut vec![];
    let vertex_buffers_layout = &[UiRect::vertex_buffer_layout(0, true, _empty)];

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} PipelineLayout")),
        bind_group_layouts: &[screen.bind_group_layout(), rgba_bind_group_layout(device)],
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
