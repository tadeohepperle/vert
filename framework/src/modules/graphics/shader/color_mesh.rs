use std::sync::{LazyLock, Mutex};

use log::{error, info};
use wgpu::{
    BufferUsages, FragmentState, RenderPipelineDescriptor, ShaderModuleDescriptor, VertexState,
};

use crate::{
    modules::graphics::{
        elements::{
            buffer::GrowableBuffer,
            color::Color,
            immediate_geometry::{ImmediateMesh, ImmediateMeshQueue},
            transform::{Transform, TransformRaw},
        },
        graphics_context::GraphicsContext,
        renderer::PipelineSettings,
        settings::GraphicsSettings,
        statics::{camera::Camera, StaticBindGroup},
    },
    utils::watcher::FileChangeWatcher,
    wgsl_file,
};

use super::{Attribute, RendererT, VertexT, FRAGMENT_ENTRY_POINT, VERTEX_ENTRY_POINT};

// /////////////////////////////////////////////////////////////////////////////
// Interface
// /////////////////////////////////////////////////////////////////////////////

impl ColorMeshRenderer {
    pub fn draw_immediate(vertices: &[Vertex], indices: &[u32], transforms: &[Transform]) {
        let mut queue = COLORMESH_QUEUE.lock().unwrap();
        queue.add_mesh(vertices, indices, transforms);
    }

    pub fn draw_cubes(transforms: &[Transform], color: Option<Color>) {
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
        Self::draw_immediate(&vertices, &indices, transforms)
    }
}

type ColorMeshQueue = ImmediateMeshQueue<Vertex, Transform>;
static COLORMESH_QUEUE: LazyLock<Mutex<ColorMeshQueue>> =
    LazyLock::new(|| Mutex::new(ColorMeshQueue::default()));

// /////////////////////////////////////////////////////////////////////////////
// Renderer
// /////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct ColorMeshRenderer {
    pipeline: wgpu::RenderPipeline,
    immediate_meshes: Vec<ImmediateMesh>,

    // buffers for immediate geometry, cleared each frame:
    vertex_buffer: GrowableBuffer<Vertex>,
    index_buffer: GrowableBuffer<u32>,
    instance_buffer: GrowableBuffer<TransformRaw>,

    // watcher for hot-reloading the shader:
    watcher: FileChangeWatcher,

    // saved for recreating the pipeline later.
    pipeline_settings: PipelineSettings,
}

impl RendererT for ColorMeshRenderer {
    fn new(graphics_context: &GraphicsContext, pipeline_settings: PipelineSettings) -> Self {
        let device = &graphics_context.device;
        let wgsl = include_str!("color_mesh.wgsl");
        dbg!(wgsl_file!());
        let watcher = FileChangeWatcher::new(&[&wgsl_file!()]);
        let pipeline = create_render_pipeline(device, pipeline_settings.clone(), wgsl);

        let renderer = ColorMeshRenderer {
            pipeline,
            immediate_meshes: vec![],
            vertex_buffer: GrowableBuffer::new(device, 512, BufferUsages::VERTEX),
            index_buffer: GrowableBuffer::new(device, 512, BufferUsages::INDEX),
            // instance_buffer also uses BufferUsages::VERTEX
            instance_buffer: GrowableBuffer::new(device, 512, BufferUsages::VERTEX),
            watcher,
            pipeline_settings,
        };

        renderer
    }

    fn prepare(&mut self, context: &GraphicsContext, _encoder: &mut wgpu::CommandEncoder) {
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

        let mut color_mesh_queue = COLORMESH_QUEUE.lock().unwrap();
        let queue = &context.queue;
        let device = &context.device;
        self.vertex_buffer
            .prepare(color_mesh_queue.vertices(), queue, device);
        self.index_buffer
            .prepare(color_mesh_queue.indices(), queue, device);
        self.instance_buffer
            .prepare(color_mesh_queue.instances(), queue, device);
        color_mesh_queue.clear_and_take_meshes(&mut self.immediate_meshes);
    }

    fn render<'s: 'encoder, 'pass, 'encoder>(
        &'s self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        _graphics_settings: &GraphicsSettings,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, Camera::bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.buffer().slice(..));
        render_pass.set_index_buffer(
            self.index_buffer.buffer().slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.set_vertex_buffer(1, self.instance_buffer.buffer().slice(..));

        for mesh in self.immediate_meshes.iter() {
            render_pass.draw_indexed(mesh.index_range.clone(), 0, mesh.instance_range.clone())
        }
    }
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

    let _empty1 = &mut vec![];
    let _empty2 = &mut vec![];
    let vertex_buffers_layout = &[
        Vertex::vertex_buffer_layout(0, false, _empty1),
        TransformRaw::vertex_buffer_layout(Vertex::ATTRIBUTES.len(), true, _empty2),
    ];

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} PipelineLayout")),
        bind_group_layouts: &[Camera::bind_group_layout()],
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
        primitive: ColorMeshRenderer::primitive(),
        depth_stencil: ColorMeshRenderer::depth_stencil(),
        multisample: settings.multisample,
        multiview: None,
    })
}

// /////////////////////////////////////////////////////////////////////////////
// Data Definition
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
