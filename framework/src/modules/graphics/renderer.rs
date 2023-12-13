use std::sync::Arc;

use winit::dpi::PhysicalSize;

use super::{
    elements::{
        camera::{Camera, CameraBindGroup},
        color_mesh::ColorMeshRenderPipeline,
        texture::Texture,
    },
    graphics_context::GraphicsContext,
};

pub struct Renderer {
    context: GraphicsContext,
    depth_texture: Texture,
    color_mesh_render_pipeline: ColorMeshRenderPipeline,
}

impl Renderer {
    pub async fn initialize(
        context: GraphicsContext,
        camera_bind_group: CameraBindGroup,
    ) -> anyhow::Result<Self> {
        let depth_texture = create_depth_texture(&context);
        let color_mesh_render_pipeline = ColorMeshRenderPipeline::new(&context, camera_bind_group);
        Ok(Self {
            context,
            depth_texture,
            color_mesh_render_pipeline,
        })
    }

    pub fn resize(&mut self) {
        // recreate the depth texture
        self.depth_texture = create_depth_texture(&self.context);
    }

    /// grabs the stuff he needs from the arenas and renders it.
    pub fn render(
        &self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        arenas: &vert_core::arenas::Arenas,
    ) {
        // create a new renderpass:
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Renderpass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // render color meshes:
        self.color_mesh_render_pipeline
            .render_color_meshes(&mut render_pass, arenas);
    }
}

fn create_depth_texture(context: &GraphicsContext) -> Texture {
    let surface_config = context.surface_config.get();
    let depth_texture =
        Texture::create_depth_texture(&context.device, &surface_config, "depth_texture");
    depth_texture
}
