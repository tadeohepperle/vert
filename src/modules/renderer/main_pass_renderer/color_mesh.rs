use wgpu::{
    BufferUsages, FragmentState, PrimitiveState, RenderPipelineDescriptor, ShaderModuleDescriptor,
    VertexState,
};

use crate::{
    elements::{
        Color, GrowableBuffer, ImmediateMeshQueue, ImmediateMeshRanges, Transform, TransformRaw,
    },
    modules::{
        renderer::{Attribute, VertexT, DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT},
        GraphicsContext, MainCamera3D, Prepare, Renderer,
    },
    utils::Timing,
    Dependencies, Handle, Module,
};

use super::MainPassRenderer;

// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl ColorMeshRenderer {
    #[inline(always)]
    pub fn draw_geometry(
        &mut self,
        vertices: &[Vertex],
        indices: &[u32],
        transforms: &[Transform],
    ) {
        self.color_mesh_queue
            .add_mesh(vertices, indices, transforms);
    }

    pub fn draw_cubes(&mut self, transforms: &[Transform], color: Option<Color>) {
        const P: f32 = 0.5;
        const M: f32 = -0.5;
        let positions = vec![
            [M, M, M],
            [P, M, M],
            [P, M, P],
            [M, M, P],
            [M, P, M],
            [P, P, M],
            [P, P, P],
            [M, P, P],
        ];

        let vertices: Vec<Vertex> = positions
            .into_iter()
            .map(|p| {
                let x = p[0];
                let y = p[1];
                let z = p[2];
                Vertex {
                    pos: [x, y, z],
                    color: color.unwrap_or_else(|| Color::new(x, y, z)),
                }
            })
            .collect();

        let indices = vec![
            0, 1, 2, 0, 2, 3, 4, 7, 6, 4, 6, 5, 1, 5, 6, 1, 6, 2, 0, 3, 7, 0, 7, 4, 2, 6, 3, 6, 7,
            3, 0, 4, 1, 4, 5, 1,
        ];
        self.draw_geometry(&vertices, &indices, transforms)
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
}

#[derive(Debug)]
pub struct ColorMeshRenderer {
    pipeline: wgpu::RenderPipeline,
    /// immediate geometry, cleared every frame
    color_mesh_queue: ImmediateMeshQueue<Vertex, Transform>,
    /// information about index ranges
    render_data: RenderData,
    deps: Deps,
}

/// buffers for immediate geometry
#[derive(Debug)]
struct RenderData {
    mesh_ranges: Vec<ImmediateMeshRanges>,
    vertex_buffer: GrowableBuffer<Vertex>,
    index_buffer: GrowableBuffer<u32>,
    instance_buffer: GrowableBuffer<TransformRaw>,
}

impl RenderData {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            mesh_ranges: vec![],
            vertex_buffer: GrowableBuffer::new(device, 512, BufferUsages::VERTEX),
            index_buffer: GrowableBuffer::new(device, 512, BufferUsages::INDEX),
            instance_buffer: GrowableBuffer::new(device, 512, BufferUsages::VERTEX),
        }
    }
}

impl Module for ColorMeshRenderer {
    type Config = ();

    type Dependencies = Deps;

    fn new(_config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let device = &deps.ctx.device;
        let pipeline = create_render_pipeline(device, include_str!("color_mesh.wgsl"), &deps.cam);

        Ok(ColorMeshRenderer {
            pipeline,
            color_mesh_queue: ImmediateMeshQueue::default(),
            render_data: RenderData::new(device),
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

impl Prepare for ColorMeshRenderer {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) {
        self.render_data
            .vertex_buffer
            .prepare(self.color_mesh_queue.vertices(), device, queue);
        self.render_data
            .index_buffer
            .prepare(self.color_mesh_queue.indices(), device, queue);
        self.render_data
            .instance_buffer
            .prepare(self.color_mesh_queue.instances(), device, queue);
        self.color_mesh_queue
            .clear_and_take_meshes(&mut self.render_data.mesh_ranges);
    }
}

impl MainPassRenderer for ColorMeshRenderer {
    fn render<'encoder>(&'encoder self, render_pass: &mut wgpu::RenderPass<'encoder>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, self.deps.cam.bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.render_data.vertex_buffer.buffer().slice(..));
        render_pass.set_index_buffer(
            self.render_data.index_buffer.buffer().slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_vertex_buffer(1, self.render_data.instance_buffer.buffer().slice(..));
        for mesh in self.render_data.mesh_ranges.iter() {
            render_pass.draw_indexed(mesh.index_range.clone(), 0, mesh.instance_range.clone())
        }
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Render Pipeline
// /////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: Color,
}

impl VertexT for Vertex {
    const ATTRIBUTES: &'static [Attribute] = &[
        Attribute::new("pos", wgpu::VertexFormat::Float32x3),
        Attribute::new("color", wgpu::VertexFormat::Float32x4),
    ];
}

fn create_render_pipeline(
    device: &wgpu::Device,
    wgsl: &str,
    cam: &MainCamera3D,
) -> wgpu::RenderPipeline {
    let label = "ColorMeshRenderer";
    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: Some(&format!("{label} ShaderModule")),
        source: wgpu::ShaderSource::Wgsl(wgsl.into()),
    });

    let _empty1 = &mut vec![];
    let _empty2 = &mut vec![];
    let vertex_buffers_layout = &[
        Vertex::vertex_buffer_layout(0, false, _empty1),
        TransformRaw::vertex_buffer_layout(Vertex::ATTRIBUTES.len(), true, _empty2),
    ];

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} PipelineLayout")),
        bind_group_layouts: &[cam.bind_group_layout()],
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
        primitive: PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        },
        multiview: None,
    })
}
