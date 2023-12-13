use std::{ops::DerefMut, sync::Arc};

use glam::{vec3, vec4, Vec3};
use vert_core::{arenas::Arenas, component::Component, reflect};
use wgpu::{
    util::DeviceExt, BindGroupLayout, PipelineLayout, PrimitiveState, RenderPass, RenderPipeline,
    ShaderModuleDescriptor, TextureFormat,
};

use vert_core::prelude::*;

use crate::{
    constants::DEPTH_FORMAT,
    modules::graphics::{
        graphics_context::{GraphicsContext, COLOR_FORMAT},
        Prepare, VertexT,
    },
};

use super::{
    buffer::{InstanceBuffer, ToRaw, UniformBuffer},
    camera::CameraBindGroup,
    transform::{Transform, TransformRaw},
};

/// abstraction over 1 instance and multiple instances
pub struct ColorMeshObj {
    mesh: ColorMesh,
    transform: InstanceBuffer<Transform>,
}

pub struct ColorMeshRenderPipeline {
    pipeline: wgpu::RenderPipeline,
    camera_bind_group: CameraBindGroup,
}

impl ColorMeshRenderPipeline {
    pub fn new(context: &GraphicsContext, camera_bind_group: CameraBindGroup) -> Self {
        let device = &context.device;
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("ColoredMesh Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("color_mesh.wgsl").into()),
        });

        let vertex_and_transform_layout: [wgpu::VertexBufferLayout; 2] =
            [Vertex::desc(), TransformRaw::desc()];

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ColoredMesh Pipelinelayout"),
                bind_group_layouts: &[camera_bind_group.layout()],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ColoredMesh Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &vertex_and_transform_layout,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
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
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        ColorMeshRenderPipeline {
            pipeline,
            camera_bind_group,
        }
    }

    /// Renders all `SingleColorMesh`es and `MultiColorMesh`es found in the arenas.
    pub fn render_color_meshes<'s: 'e, 'p, 'e>(
        &'s self,
        render_pass: &'p mut RenderPass<'e>,
        arenas: &'e Arenas,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group.bind_group(), &[]);

        let single_color_meshes = arenas.iter::<SingleColorMesh>().map(|e| &e.1.inner);
        let multi_color_meshes = arenas.iter::<MultiColorMesh>().map(|e| &e.1.inner);

        for obj in single_color_meshes.chain(multi_color_meshes) {
            render_pass.set_vertex_buffer(0, obj.mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, obj.transform.buffer().slice(..));
            render_pass
                .set_index_buffer(obj.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            let instance_count = obj.transform.values().len() as u32;
            render_pass.draw_indexed(
                0..obj.mesh.mesh_data.indices.len() as u32,
                0,
                0..instance_count,
            );
        }
    }
}

pub struct ColorMeshData {
    pub verts: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// A mesh with vertex colors, that is rendered with the color_mesh shader.
/// To render it, only the Mesh
pub struct ColorMesh {
    name: String,
    mesh_data: ColorMeshData,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

impl ColorMesh {
    pub fn new(mesh_data: ColorMeshData, name: String, device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{name}_vertex_buffer")),
            contents: bytemuck::cast_slice(&mesh_data.verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{name}_index_buffer")),
            contents: bytemuck::cast_slice(&mesh_data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            name,
            mesh_data,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn cube(name: &str, device: &wgpu::Device) -> Self {
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

        let verts = positions
            .into_iter()
            .map(|p| {
                let x = p[0];
                let y = p[1];
                let z = p[2];
                Vertex {
                    pos: [x, y, z],
                    color: [x, y, z, 1.0],
                }
            })
            .collect();

        let indices = vec![
            0, 1, 2, 0, 2, 3, 4, 7, 6, 4, 6, 5, 1, 5, 6, 1, 6, 2, 0, 3, 7, 0, 7, 4, 2, 6, 3, 6, 7,
            3, 0, 4, 1, 4, 5, 1,
        ];

        let mesh_data = ColorMeshData { verts, indices };
        ColorMesh::new(mesh_data, name.to_string(), device)
    }
}

// pub struct ColorMeshes {
//     mesh_data: ColorMeshData,
//     transform: Vec<Transform>,
// }

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}

impl VertexT for Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

// /////////////////////////////////////////////////////////////////////////////
// Abstractions
// /////////////////////////////////////////////////////////////////////////////

reflect!(SingleColorMesh : Prepare);
impl Component for SingleColorMesh {}
pub struct SingleColorMesh {
    inner: ColorMeshObj,
}

impl Prepare for SingleColorMesh {
    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder) {
        self.inner.transform.update_raw_and_buffer(&context.queue);
    }
}

impl SingleColorMesh {
    pub fn into_multi(self) -> MultiColorMesh {
        MultiColorMesh { inner: self.inner }
    }

    pub fn new(transform: Transform, mesh_data: ColorMeshData, device: &wgpu::Device) -> Self {
        let inner = ColorMeshObj {
            mesh: ColorMesh::new(mesh_data, "aaa".into(), device),
            transform: InstanceBuffer::new(vec![transform], device),
        };
        SingleColorMesh { inner }
    }

    pub fn cube(transform: Transform, device: &wgpu::Device) -> Self {
        let name = "New Mesh Obj";
        let mesh = ColorMesh::cube(name, device);
        let transform = InstanceBuffer::new(vec![transform], device);
        Self {
            inner: ColorMeshObj { mesh, transform },
        }
    }

    pub fn transform(&self) -> &Transform {
        &self.inner.transform.values()[0]
    }

    pub fn transform_mut(&mut self) -> &mut Transform {
        &mut self.inner.transform.values_mut()[0]
    }
}

reflect!(MultiColorMesh : Prepare);
impl Component for MultiColorMesh {}
pub struct MultiColorMesh {
    inner: ColorMeshObj,
}

impl Prepare for MultiColorMesh {
    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder) {
        self.inner.transform.update_raw_and_buffer(&context.queue);
    }
}

impl MultiColorMesh {
    pub fn cubes(transforms: Vec<Transform>, device: &wgpu::Device) -> Self {
        let name = "New Mesh Obj";
        let mesh = ColorMesh::cube(name, device);
        let transform = InstanceBuffer::new(transforms, device);
        Self {
            inner: ColorMeshObj { mesh, transform },
        }
    }

    pub fn transforms(&mut self) -> &Vec<Transform> {
        &self.inner.transform.values()
    }

    pub fn transforms_mut(&mut self) -> &mut Vec<Transform> {
        self.inner.transform.values_mut()
    }
}
