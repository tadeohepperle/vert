use wgpu::VertexFormat;

use crate::modules::graphics::{
    elements::{
        color::Color,
        rect::{PreparedRects, Rect, RectT},
        texture::Texture,
    },
    graphics_context::GraphicsContext,
    statics::screen_size::ScreenSize,
};

use super::{
    vertex::{VertexAttribute, VertexT},
    ShaderPipelineConfig, ShaderRendererT, ShaderT,
};

pub struct UiRectShader;
impl ShaderT for UiRectShader {
    type BindGroups = (ScreenSize, Texture);
    type Vertex = ();
    type Instance = UiRect;
    type VertexOutput = VertexOutput;
    type Renderer = UiRectRenderer;

    fn naga_module() -> anyhow::Result<wgpu::naga::Module> {
        let wgsl = include_str!("ui_rect.wgsl");
        let module = wgpu::naga::front::wgsl::parse_str(wgsl)?;
        Ok(module)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiRect {
    // min x, min y, size x, size y
    pub pos: Rect,
    // min x, min y, size x, size y
    pub uv: Rect,
    pub color: Color,
    pub border_radius: [f32; 4],
}

impl RectT for UiRect {}

impl VertexT for UiRect {
    const ATTRIBUTES: &'static [VertexAttribute] = &[
        VertexAttribute::new("pos", VertexFormat::Float32x4),
        VertexAttribute::new("uv", VertexFormat::Float32x4),
        VertexAttribute::new("color", VertexFormat::Float32x4),
        VertexAttribute::new("border_radius", VertexFormat::Float32x4),
    ];
}

pub struct VertexOutput;

impl VertexT for VertexOutput {
    const ATTRIBUTES: &'static [super::vertex::VertexAttribute] = &[
        VertexAttribute::new("color", VertexFormat::Float32x4),
        VertexAttribute::new("uv", VertexFormat::Float32x2),
        VertexAttribute::new("offset", VertexFormat::Float32x2),
        VertexAttribute::new("size", VertexFormat::Float32x2),
        VertexAttribute::new("border_radius", VertexFormat::Float32x4),
    ];
}

pub struct UiRectRenderer {
    prepared_ui_rects: PreparedRects<UiRect>,
    pipeline: wgpu::RenderPipeline,
}

impl ShaderRendererT for UiRectRenderer {
    fn new(graphics_context: &GraphicsContext, pipeline_config: ShaderPipelineConfig) -> Self
    where
        Self: Sized,
    {
        let pipeline =
            UiRectShader::build_pipeline(&graphics_context.device, pipeline_config).unwrap();

        UiRectRenderer {
            prepared_ui_rects: PreparedRects::new(&graphics_context.device),
            pipeline,
        }
    }

    fn prepare(
        &mut self,
        context: &crate::modules::graphics::graphics_context::GraphicsContext,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        todo!()
    }

    fn render<'s: 'encoder, 'pass, 'encoder>(
        &'s self,
        render_pass: &'pass mut wgpu::RenderPass<'encoder>,
        graphics_settings: &crate::modules::graphics::settings::GraphicsSettings,
    ) {
        todo!()
    }
}
