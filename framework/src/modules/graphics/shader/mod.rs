use std::borrow::Cow;

use crate::constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT};
use smallvec::{smallvec, SmallVec};
use wgpu::{BindGroupLayout, PrimitiveState, ShaderModuleDescriptor};

use self::{
    bind_group::MultiBindGroupT,
    vertex::{wgpu_vertex_buffer_layout, VertexT},
};

use super::{graphics_context::GraphicsContext, settings::GraphicsSettings};

pub mod bind_group;
pub mod color_mesh;
pub mod vertex;

const VERTEX_ENTRY_POINT: &str = "vs_main";
const FRAGMENT_ENTRY_POINT: &str = "fs_main";

pub enum ShaderCode {
    Static(&'static str), // todo!() add file that can be watched
}

/// this trait does not need to be object safe.
pub trait ShaderT: 'static + Sized {
    type BindGroups: MultiBindGroupT;
    type Vertex: VertexT; // Index always u32, so not included
    type Instance: VertexT;
    type VertexOutput: VertexT; // no need to specify `builtin clip_position` here.

    type Renderer: ShaderRendererT;

    const VERTEX_SHADER_CODE: ShaderCode;
    const FRAGMENT_SHADER_CODE: ShaderCode;

    /// todo!() maybe this should not be an associated function.
    fn build_shader() -> anyhow::Result<wgpu::naga::Module> {
        let s = "
        
        struct Camera {
            view_pos: vec4<f32>,
            view_proj: mat4x4<f32>,
        }
        
        @group(0) @binding(0)
        var<uniform> camera: Camera;
        
        struct VertexInput {
            @location(0) position: vec3<f32>,
            @location(1) color: vec4<f32>,
        }
        
        struct Transform {
            @location(2) col1: vec4<f32>,
            @location(3) col2: vec4<f32>,
            @location(4) col3: vec4<f32>,
            @location(5) translation: vec4<f32>,
        }
        
        struct VertexOutput {
            @builtin(position) clip_position: vec4<f32>,
            @location(0) color: vec4<f32>,
        };
        
        
        @vertex
        fn vs_main(
            vertex: VertexInput,
            transform: Transform,
        ) -> VertexOutput {
        
            let model_matrix = mat4x4<f32>(
                transform.col1,
                transform.col2,
                transform.col3,
                transform.translation,
            );
        
            let world_position = vec4<f32>(vertex.position, 1.0);
        
            var out: VertexOutput;
            out.clip_position = camera.view_proj * model_matrix * world_position;
            out.color = vertex.color;
            return out;
        }
         
        @fragment
        fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
            return in.color;
        }
        
        
        ";

        let module = wgpu::naga::front::wgsl::parse_str(s)?;
        Ok(module)
    }

    /// todo!() maybe this should not be an associated function.
    fn build_pipeline(
        device: &wgpu::Device,
        config: ShaderPipelineConfig,
    ) -> anyhow::Result<wgpu::RenderPipeline> {
        let label = std::any::type_name::<Self>();
        let naga_module = Self::build_shader()?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{label} Shader Module")),
            source: wgpu::ShaderSource::Naga(Cow::Owned(naga_module)),
        });

        // /////////////////////////////////////////////////////////////////////////////
        // Construct Vertex and Instance Buffer layouts
        // /////////////////////////////////////////////////////////////////////////////

        let mut empty = vec![];
        let vertex_buffer_layout = wgpu_vertex_buffer_layout::<Self::Vertex>(false, 0, &mut empty);
        let shader_location_offset = if let Some(e) = &vertex_buffer_layout {
            e.attributes.len() as u32
        } else {
            0
        };

        let mut empty = vec![];
        let instance_buffer_layout =
            wgpu_vertex_buffer_layout::<Self::Instance>(true, shader_location_offset, &mut empty);
        let mut buffers: SmallVec<[wgpu::VertexBufferLayout; 2]> = smallvec![];
        if let Some(i) = vertex_buffer_layout {
            buffers.push(i);
        }
        if let Some(i) = instance_buffer_layout {
            buffers.push(i);
        }

        // todo!() we should make the bindgroup layouts static across the entire app. They should be created once and then shared by all renderers... This is an optimization for later though...
        let bind_group_layouts: Vec<BindGroupLayout> =
            Self::BindGroups::BIND_GROUP_LAYOUT_DESCRIPTORS
                .into_iter()
                .map(|desc| device.create_bind_group_layout(desc))
                .collect();
        let bind_group_layout_references: Vec<&BindGroupLayout> =
            bind_group_layouts.iter().collect();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("{label} RenderPipelineLayout")),
                bind_group_layouts: &bind_group_layout_references,
                push_constant_ranges: &[], //           todo! needle, currently not used                 -> not used.
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{label} RenderPipeline")),
            layout: Some(&render_pipeline_layout), //      -> Trait: derived from vertex_type and instance_type
            vertex: wgpu::VertexState {
                module: &module,                 //  -> Trait: explicit
                entry_point: VERTEX_ENTRY_POINT, //  -> Trait: convention
                buffers: &buffers,               //  -> Trait: convention
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,                   //  -> Trait specific / else discard
                entry_point: FRAGMENT_ENTRY_POINT, //             -> Trait: convention
                targets: &[Some(config.target)],
            }),
            primitive: Self::primitive(),
            depth_stencil: Self::depth_stencil(),
            multisample: config.multisample, // Outer
            multiview: None,
        });

        Ok(pipeline)
    }

    /// defaults, can be overriden
    fn primitive() -> wgpu::PrimitiveState {
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        }
    }

    /// defaults, can be overriden
    fn depth_stencil() -> Option<wgpu::DepthStencilState> {
        Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        })
    }
}

pub struct ShaderPipelineConfig {
    pub multisample: wgpu::MultisampleState,
    pub target: wgpu::ColorTargetState,
}

impl Default for ShaderPipelineConfig {
    fn default() -> Self {
        Self {
            multisample: wgpu::MultisampleState {
                count: MSAA_SAMPLE_COUNT,
                ..Default::default()
            },
            target: wgpu::ColorTargetState {
                format: HDR_COLOR_FORMAT,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            },
        }
    }
}

pub trait ShaderRendererT: 'static {
    fn new(graphics_context: &GraphicsContext, pipeline_config: ShaderPipelineConfig) -> Self
    where
        Self: Sized;

    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder);

    fn render<'s: 'encoder, 'pass, 'encoder>(
        &'s self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        graphics_settings: &GraphicsSettings,
    );
}

// fn build_pipeline<S: Shader>() -> wgpu::Render {}

/*
This is just text that helps me think:

General Idea of user defined shaders:
Shaders are always used WITHIN a renderpass that is created somewhere else.
That means they have no control over render pass params like:
- the render target and its format
- depth stencil
- msaa
- individual locations of vertex and instance parameters and the group index of bind groups
should not be a concern for the shader.
Instead they should be computed on the fly when the shader def is translated to wgsl -> naga Module -> create shader module.

All these things we do not want to deal with in the shader.
Instead a shader needs to just define:
- how to create a pipeline with it (that can then be set with set_pipeline on the render pass)
- what bind groups it needs
- type of vertex and instance data



Open questions:

how does bind group information get into the shader?
How does vertex and index buffer get fed in?
How to set defaults for indexed draws, e.g. always draw just 4 indices, no vertex buffer, no instance buffer, just

Should batching yes no be a concern for the shader or not?
Remember: Batching like Unity does it (combine multiple primitives, given them just one transform) can lead the individual transforms
getting lost, which is not so nice.

Oh maybe we can have the definition of vertex data be a triple enum:
None  -> No vertex buffer passed at all
Same  -> Same vertex buffer (global) always. Hardcoded? Where stored?
Custom -> Manually pass in a vertex buffer every time??
          Or specify how vertex buffer should be gathered from the ECS world?


Structure of a shader:


we need to define, how the shader will be called:
layout of vertices


vertex shader

can have index buffer
can have vertex buffer
can have instance buffer

e.g. transform by model matrix and camera matrix
e.g. transform into screen space -> UI

output stuff to vertex shader

fragment shader

just output some color.





so the definition of a shader needs to be:

- all bindgroups accessible (for all uniforms)
- draw method: instanced, batched, single???
- type of data for vertices and instances


- code for vert shader
- code for frag shader


Example for the color mesh shader:

settings controlled by the shader:

ColorMeshShader{
    vertex_type: Some(
        pos: [f32; 3],
        color: Color,
    )

    instance_type: Some(
        TransformRaw(
            Float32x4
            Float32x4
            Float32x4
            Float32x4
        )
    )

    vertex_out_put_type: {
        @builtin(position) clip_position: vec4<f32>,
        @location(0) color: vec4<f32>,
    }






}

settings not controlled by the shader:





`Trait: xxxx` means that the item can be gotten by just looking at the trait.
Trait: explicit -> something has to be defined by the user.
Trait: convention -> same everywhere in the engine, no need to be custom.
Trait: derived    -> can be derived by looking at explicitly deifned items in the trait.
Outer: -> means that these parameters are controlled by global rendering settings, e.g. msaa enabled or not



  let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("ColoredMesh Shader"),                                         -> Trait: derived from typename
            source: wgpu::ShaderSource::Wgsl(include_str!("color_mesh.wgsl").into()),  -> Trait: derived, we build up a naga module from the trait definition
  });

  let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ColoredMesh Pipelinelayout"),             -> Trait: derived by looking at typename
                bind_group_layouts: &[camera_bind_group.layout()],     -> Trait: explicit
                push_constant_ranges: &[],                             -> not used.
            });

device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("ColoredMesh Pipeline"),            -> Trait: derived by looking at typename
    layout: Some(&render_pipeline_layout),          -> Trait: derived from vertex_type and instance_type
    vertex: wgpu::VertexState {
        module: &shader,                            -> Trait: explicit
        entry_point: "vs_main",                     -> Trait: convention
        buffers: &vertex_and_transform_layout,      -> Trait: convention
    },
    fragment: Some(wgpu::FragmentState {
        module: &shader,                            -> Trait specific / else discard
        entry_point: "fs_main",                     -> Trait: convention
        targets: &[Some(wgpu::ColorTargetState {    -> Outer: defined by the creator/caller of the pipeline
            format: HDR_COLOR_FORMAT                          That means, an individual shader does not have to care about it at all.
            blend: Some(wgpu::BlendState {
                alpha: wgpu::BlendComponent::REPLACE,
                color: wgpu::BlendComponent::REPLACE,
            }),
            write_mask: wgpu::ColorWrites::ALL,
        })],
    }),
    primitive: PrimitiveState {                     -> Trait: specializable
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
    },
    depth_stencil: Some(wgpu::DepthStencilState {   -> Trait: specializable
        format: DEPTH_FORMAT,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }),
    multisample: wgpu::MultisampleState {             -> Outer:
        count: MSAA_SAMPLE_COUNT,
        ..Default::default()
    },
    multiview: None,                                  -> Outer:
});


Example color mesh: how are the


render_pass.set_pipeline(&self.pipeline);                                         -> can be the same everywhere
render_pass.set_bind_group(0, &self.camera_bind_group.bind_group(), &[]);         -> can be derived from

let single_color_meshes = arenas.iter::<SingleColorMesh>().map(|e| &e.1.inner);   -> This stuff IDK
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

Example for 3d Text/Sprites:









trait for post-processing effect as well


What ways are there to draw something with a render pipeline?

Iterate over all elements in the arenas that are drawable?
Send triangles / indices to the render pipeline directly?





Lets say we want to spawn some objects with some material.

We can just insert these objects into the arenas, or we send them direcly
to some place to be rendered. Retained vs immediate mode.

We need to expose some receiver for each ShaderType that receives:
vertex_data,



Functions to draw something can be made on the material directly?
Materials could have singletons with OnceLock

Materials must be registered before usage.

What if the material structs just have different function on them that all implement some shader trait.
E.g. we can have a ColorMeshMaterial


ColorMeshMaterial::draw(Arc<vertex-buffer>, Transform) {



}










*/
