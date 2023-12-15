use std::sync::Arc;

use wgpu::RenderPassColorAttachment;
use winit::dpi::PhysicalSize;

use crate::modules::{egui::EguiState, ui::ImmediateUi};

use super::{
    elements::{
        camera::{Camera, CameraBindGroup},
        color_mesh::ColorMeshRenderPipeline,
        rect_3d::Rect3DRenderPipeline,
        screen_space::ScreenSpaceBindGroup,
        texture::Texture,
        ui_rect::UiRectRenderPipeline,
    },
    graphics_context::GraphicsContext,
    Render,
};

pub struct Renderer {
    context: GraphicsContext,
    depth_texture: Texture,
    color_mesh_render_pipeline: ColorMeshRenderPipeline,
    ui_rect_render_pipeline: UiRectRenderPipeline,
    rect_3d_render_pipeline: Rect3DRenderPipeline,
    msaa_texture: MSAATexture,
}

impl Renderer {
    pub fn initialize(
        context: GraphicsContext,
        camera_bind_group: CameraBindGroup,
        screen_space_bind_group: ScreenSpaceBindGroup,
    ) -> anyhow::Result<Self> {
        let depth_texture = create_depth_texture(&context, 4);
        let msaa_texture = create_msaa_texure(&context, 4);

        let color_mesh_render_pipeline =
            ColorMeshRenderPipeline::new(&context, camera_bind_group.clone());
        let ui_rect_render_pipeline = UiRectRenderPipeline::new(&context, screen_space_bind_group);
        let rect_3d_render_pipeline = Rect3DRenderPipeline::new(&context, camera_bind_group);

        Ok(Self {
            context,
            depth_texture,
            msaa_texture,
            color_mesh_render_pipeline,
            ui_rect_render_pipeline,
            rect_3d_render_pipeline,
        })
    }

    pub fn resize(&mut self) {
        // recreate the depth  and msaa texture
        self.depth_texture = create_depth_texture(&self.context, 4);
        self.msaa_texture = create_msaa_texure(&self.context, 4);
    }

    /// grabs the stuff he needs from the arenas and renders it.
    pub fn render(
        &self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        arenas: &vert_core::arenas::Arenas,
        ui: &ImmediateUi,
    ) {
        // create a new renderpass:
        let color_attachment = self.msaa_color_attachment(view);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Renderpass"),
            color_attachments: &[Some(color_attachment)],
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

        // render ui rectangles:
        self.ui_rect_render_pipeline.render_ui_rects(
            &mut render_pass,
            ui.prepared_ui_rects(),
            ui.text_atlas_texture(),
        );

        // render 3d triangles:

        self.rect_3d_render_pipeline.render_3d_rects(
            &mut render_pass,
            ui.prepared_3d_rects(),
            ui.text_atlas_texture(),
        );

        drop(render_pass);
    }

    fn msaa_color_attachment<'a>(
        &'a self,
        view: &'a wgpu::TextureView,
    ) -> RenderPassColorAttachment<'a> {
        wgpu::RenderPassColorAttachment {
            view: &self.msaa_texture.view,
            resolve_target: Some(&view),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        }
    }

    fn non_msaa_color_attachment<'a>(
        &'a self,
        view: &'a wgpu::TextureView,
    ) -> RenderPassColorAttachment<'a> {
        wgpu::RenderPassColorAttachment {
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
        }
    }
}

fn create_depth_texture(context: &GraphicsContext, sample_count: u32) -> Texture {
    let surface_config = context.surface_config.get();
    let depth_texture = Texture::create_depth_texture(
        &context.device,
        &surface_config,
        "depth_texture",
        sample_count,
    );
    depth_texture
}

pub struct MSAATexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sample_count: u32,
}

pub fn create_msaa_texure(context: &GraphicsContext, sample_count: u32) -> MSAATexture {
    let config = context.surface_config.get();
    let multisampled_texture_extent = wgpu::Extent3d {
        width: config.width,
        height: config.height,
        depth_or_array_layers: 1,
    };
    let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
        size: multisampled_texture_extent,
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: config.view_formats[0],
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    };

    let texture = context.device.create_texture(multisampled_frame_descriptor);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    MSAATexture {
        sample_count,
        texture,
        view,
    }
}
