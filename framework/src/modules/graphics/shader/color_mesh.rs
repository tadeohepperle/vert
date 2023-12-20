use std::{
    borrow::Cow,
    ops::Range,
    sync::{LazyLock, Mutex, OnceLock},
};

use wgpu::BufferUsages;

use crate::modules::graphics::{
    elements::{
        buffer::{GrowableBuffer, ToRaw},
        color::Color,
        transform::{Transform, TransformRaw},
    },
    graphics_context::GraphicsContext,
    settings::GraphicsSettings,
    shader::bind_group::StaticBindGroup,
    statics::camera::Camera,
};

use super::{
    vertex::{VertexAttribute, VertexT},
    ShaderCodeSource, ShaderPipelineConfig, ShaderRendererT, ShaderStump, ShaderT,
};

pub struct ColorMeshShader;

impl ShaderT for ColorMeshShader {
    type BindGroups = Camera;
    type Vertex = Vertex;
    type Instance = TransformRaw;
    type VertexOutput = Color;

    type Renderer = ColorMeshShaderRenderer;

    const CODE_SOURCE: ShaderCodeSource = ShaderCodeSource::File {
        path: "./assets/color_mesh.stump.wgsl",
        fallback: Some(COLOR_MESH_SHADER_STUMP),
    };
}

const COLOR_MESH_SHADER_STUMP: ShaderStump = ShaderStump {
    vertex: Cow::Borrowed(
        "
            let model_matrix = mat4x4<f32>(
                instance.col1,
                instance.col2,
                instance.col3,
                instance.translation,
            );
            let world_position = vec4<f32>(vertex.pos, 1.0);
            var out: VertexOutput;
            out.clip_position = camera.view_proj * model_matrix * world_position;
            out.color = vertex.color * vec4(1.0,0.3,0.3,1.0);
            return out;
        ",
    ),
    fragment: Cow::Borrowed("return in.color;"),
    other_code: Cow::Borrowed(""),
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: Color,
}

impl VertexT for Vertex {
    const ATTRIBUTES: &'static [VertexAttribute] = &[
        VertexAttribute::new("pos", wgpu::VertexFormat::Float32x3),
        VertexAttribute::new("color", wgpu::VertexFormat::Float32x4),
    ];
}

// /////////////////////////////////////////////////////////////////////////////
// Implement a shader renderer with global state
// /////////////////////////////////////////////////////////////////////////////

impl ColorMeshShader {
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

/// todo! needs to be optimized to swap allocated vecs instead of creating new ones.
#[derive(Debug)]
struct ImmediateMeshQueue<V: Copy, I: ToRaw> {
    /// index and instance ranges into the other vecs.
    immediate_meshes: Vec<ImmediateMesh>,
    // buffers for immediate geometry, cleared each frame:
    vertices: Vec<V>,
    indices: Vec<u32>,
    instances: Vec<I::Raw>,
}

impl<V: Copy, I: ToRaw> Default for ImmediateMeshQueue<V, I> {
    fn default() -> Self {
        Self {
            immediate_meshes: Default::default(),
            vertices: Default::default(),
            indices: Default::default(),
            instances: Default::default(),
        }
    }
}

impl<V: Copy, I: ToRaw> ImmediateMeshQueue<V, I> {
    fn add_mesh(&mut self, vertices: &[V], indices: &[u32], transforms: &[I]) {
        let v_count = self.vertices.len() as u32;
        let i_count = self.indices.len() as u32;
        let t_count = self.instances.len() as u32;
        self.vertices.extend(vertices.iter().copied());
        self.indices.extend(indices.iter().map(|e| *e + v_count));
        self.instances.extend(transforms.iter().map(|e| e.to_raw()));
        self.immediate_meshes.push(ImmediateMesh {
            index_range: i_count..(i_count + indices.len() as u32),
            instance_range: t_count..(t_count + transforms.len() as u32),
        });
    }

    /// Note: does not clear immediate meshes, those should be swapped out instead.
    fn clear_and_take_meshes(&mut self, out: &mut Vec<ImmediateMesh>) {
        self.vertices.clear();
        self.indices.clear();
        self.instances.clear();
        out.clear();
        std::mem::swap(out, &mut self.immediate_meshes);
    }
}

#[derive(Debug)]
pub struct ColorMeshShaderRenderer {
    pipeline: wgpu::RenderPipeline,
    immediate_meshes: Vec<ImmediateMesh>,

    // buffers for immediate geometry, cleared each frame:
    vertex_buffer: GrowableBuffer<Vertex>,
    index_buffer: GrowableBuffer<u32>,
    instance_buffer: GrowableBuffer<TransformRaw>,
}

#[derive(Debug)]
struct ImmediateMesh {
    index_range: Range<u32>,
    instance_range: Range<u32>,
}

static COLORMESH_QUEUE: LazyLock<Mutex<ColorMeshQueue>> =
    LazyLock::new(|| Mutex::new(ColorMeshQueue::default()));

impl ShaderRendererT for ColorMeshShaderRenderer {
    fn new(graphics_context: &GraphicsContext, pipeline_config: ShaderPipelineConfig) -> Self {
        let pipeline =
            ColorMeshShader::build_pipeline(&graphics_context.device, pipeline_config).unwrap();

        let renderer = ColorMeshShaderRenderer {
            pipeline,
            immediate_meshes: vec![],
            vertex_buffer: GrowableBuffer::new(&graphics_context.device, 512, BufferUsages::VERTEX),
            index_buffer: GrowableBuffer::new(&graphics_context.device, 512, BufferUsages::INDEX),
            instance_buffer: GrowableBuffer::new(
                &graphics_context.device,
                512,
                BufferUsages::VERTEX, // instance also needs BufferUsages::VERTEX
            ),
        };

        renderer
    }

    fn prepare(
        &mut self,
        context: &crate::modules::graphics::graphics_context::GraphicsContext,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut color_mesh_queue = COLORMESH_QUEUE.lock().unwrap();
        let queue = &context.queue;
        let device = &context.device;
        self.vertex_buffer
            .prepare(&color_mesh_queue.vertices, queue, device);
        self.index_buffer
            .prepare(&color_mesh_queue.indices, queue, device);
        self.instance_buffer
            .prepare(&color_mesh_queue.instances, queue, device);
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

    fn rebuild(
        &mut self,
        graphics_context: &GraphicsContext,
        pipeline_config: ShaderPipelineConfig,
    ) {
        *self = Self::new(graphics_context, pipeline_config)
    }
}
