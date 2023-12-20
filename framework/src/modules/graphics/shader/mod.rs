use std::{borrow::Cow, path::PathBuf};

use crate::constants::{DEPTH_FORMAT, HDR_COLOR_FORMAT, MSAA_SAMPLE_COUNT};
use indoc::indoc;
use smallvec::{smallvec, SmallVec};
use wgpu::{BindGroupLayout, PrimitiveState, ShaderModuleDescriptor};

use self::{
    bind_group::MultiBindGroupT,
    to_wgsl::generate_wgsl,
    vertex::{wgpu_vertex_buffer_layout, VertexT},
};

mod to_wgsl;

use super::{graphics_context::GraphicsContext, settings::GraphicsSettings};

pub mod bind_group;
pub mod color_mesh;
pub mod vertex;

const VERTEX_ENTRY_POINT: &str = "vs_main";
const FRAGMENT_ENTRY_POINT: &str = "fs_main";

/// this trait does not need to be object safe.
pub trait ShaderT: 'static + Sized {
    type BindGroups: MultiBindGroupT;
    type Vertex: VertexT; // Index always u32, so not included
    type Instance: VertexT;
    type VertexOutput: VertexT; // no need to specify `builtin clip_position` here.

    type Renderer: ShaderRendererT;

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

    fn watch_paths() -> &'static [&'static str] {
        &[]
    }

    fn naga_module() -> anyhow::Result<wgpu::naga::Module> {
        let vertex = indoc! {"
            // vertex shader code here.
            var out: VertexOutput;
            // ...
            return out;
        "};

        let fragment = indoc! {"
            // fragment shader code here.
        "};

        let other = indoc! {"
            // You can also include other code.
        "};

        let wgsl_string = generate_wgsl::<Self>(vertex, fragment, other);
        let module = wgpu::naga::front::wgsl::parse_str(&wgsl_string)?;
        Ok(module)
    }

    fn build_pipeline(
        device: &wgpu::Device,
        config: ShaderPipelineConfig,
    ) -> anyhow::Result<wgpu::RenderPipeline> {
        build_pipeline::<Self>(device, config)
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

    /// Please just provide `*self = Self::new(graphics_context, pipeline_config)` in your trait implementations;
    /// Can be triggered when settings change or a shader file that is being watched.
    fn rebuild(
        &mut self,
        graphics_context: &GraphicsContext,
        pipeline_config: ShaderPipelineConfig,
    );

    fn prepare(&mut self, context: &GraphicsContext, encoder: &mut wgpu::CommandEncoder);

    fn render<'s: 'encoder, 'pass, 'encoder>(
        &'s self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        graphics_settings: &GraphicsSettings,
    );
}

/// todo!() maybe this should not be an associated function.
fn build_pipeline<S: ShaderT>(
    device: &wgpu::Device,
    config: ShaderPipelineConfig,
) -> anyhow::Result<wgpu::RenderPipeline> {
    let label = std::any::type_name::<S>();
    let naga_module = S::naga_module()?;

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{label} Shader Module")),
        source: wgpu::ShaderSource::Naga(Cow::Owned(naga_module)),
    });

    // /////////////////////////////////////////////////////////////////////////////
    // Construct Vertex and Instance Buffer layouts
    // /////////////////////////////////////////////////////////////////////////////

    let mut empty = vec![];
    let vertex_buffer_layout = wgpu_vertex_buffer_layout::<S::Vertex>(false, 0, &mut empty);

    let shader_location_offset = S::Vertex::ATTRIBUTES.len() as u32;
    let mut empty = vec![];
    let instance_buffer_layout =
        wgpu_vertex_buffer_layout::<S::Instance>(true, shader_location_offset, &mut empty);
    let mut buffers: SmallVec<[wgpu::VertexBufferLayout; 2]> = smallvec![];
    if let Some(i) = vertex_buffer_layout {
        buffers.push(i);
    }
    if let Some(i) = instance_buffer_layout {
        buffers.push(i);
    }

    // todo!() we should make the bindgroup layouts static across the entire app. They should be created once and then shared by all renderers... This is an optimization for later though...
    let bind_group_layouts = <S::BindGroups as MultiBindGroupT>::create_bind_group_layouts(device);
    let bind_group_layout_references: Vec<&BindGroupLayout> = bind_group_layouts.iter().collect();

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
        primitive: S::primitive(),
        depth_stencil: S::depth_stencil(),
        multisample: config.multisample, // Outer
        multiview: None,
    });

    Ok(pipeline)
}
