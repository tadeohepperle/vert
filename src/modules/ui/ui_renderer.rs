use std::vec;

use log::warn;
use wgpu::BufferUsages;

use wgpu::Operations;
use wgpu::RenderPassColorAttachment;

use wgpu::RenderPassDescriptor;
use wgpu::ShaderModule;
use wgpu::ShaderModuleDescriptor;

use crate::elements::screen::ScreenGR;
use crate::elements::texture::rgba_bind_group_layout;
use crate::elements::BindableTexture;
use crate::elements::GrowableBuffer;

use crate::modules::ui::board::BoardPhase;
use crate::modules::GraphicsContext;

use crate::utils::watcher::ShaderFileWatcher;

use crate::Prepare;

use super::batching::get_batches;
use super::batching::BatchRegion;
use super::batching::BatchingResult;
use super::batching::GlyphRaw;
use super::batching::RectRaw;
use super::batching::RectRawTextured;
use super::board::Board;
use super::font_cache::FontCache;

pub struct UiRenderer {
    shader_watcher: Option<ShaderFileWatcher>,
    glyph_pipeline: wgpu::RenderPipeline,
    rect_pipeline: wgpu::RenderPipeline,
    textured_rect_pipeline: wgpu::RenderPipeline,
    // textured_rect_pipeline: wgpu::RenderPipeline,
    collected_batches: BatchingResult,
    draw_batches: Vec<BatchRegion>,
    rect_buffer: GrowableBuffer<RectRaw>,
    textured_rect_buffer: GrowableBuffer<RectRawTextured>,
    glyph_buffer: GrowableBuffer<GlyphRaw>,
}

impl UiRenderer {
    pub fn new(ctx: &GraphicsContext, screen: &ScreenGR) -> Self {
        let device = &ctx.device;
        let rect_buffer = GrowableBuffer::new(device, 256, BufferUsages::VERTEX);
        let glyph_buffer = GrowableBuffer::new(device, 512, BufferUsages::VERTEX);
        let textured_rect_buffer = GrowableBuffer::new(device, 256, BufferUsages::VERTEX);

        let shader_watcher = None;
        let shader_module = ctx.device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Ui Renderer Shaders"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui.wgsl").into()),
        });

        let glyph_pipeline = create_glyph_pipeline(&shader_module, &ctx.device, screen);
        let rect_pipeline = create_rect_pipeline(&shader_module, &ctx.device, screen);
        let textured_rect_pipeline =
            create_textured_rect_pipeline(&shader_module, &ctx.device, screen);

        UiRenderer {
            shader_watcher,
            glyph_pipeline,
            rect_pipeline,
            textured_rect_pipeline,
            collected_batches: BatchingResult::new(),
            draw_batches: vec![],
            rect_buffer,
            textured_rect_buffer,
            glyph_buffer,
        }
    }

    pub fn watch_shader_file(&mut self, path: &str) {
        self.shader_watcher = Some(ShaderFileWatcher::new(path));
    }

    /// Warning: only call AFTER layout has been performed for this frame. (needs to be BillboardPhase::Rendering)
    /// Assumes all rects and text layouts are calculated.
    pub fn draw_ui_board(&mut self, board: &Board) {
        // ensure layout has been done by checking the phase
        assert_eq!(board.phase(), BoardPhase::Rendering);
        let batches = get_batches(board);
        self.collected_batches.combine(batches);
    }

    pub fn render<'e>(
        &'e self,
        encoder: &'e mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        screen: &ScreenGR,
        fonts: &FontCache,
    ) {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Ui Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        assert!(self.collected_batches.is_empty()); // only information left should be in draw_batches.
                                                    // println!("render UiRenderer");
        render_pass.set_bind_group(0, screen.bind_group(), &[]);

        // 6 indices to draw two triangles

        const VERTEX_COUNT: u32 = 6;
        let atlas_texture = fonts.atlas_texture();
        for batch in self.draw_batches.iter() {
            match batch {
                BatchRegion::Rect(r) => {
                    render_pass.set_pipeline(&self.rect_pipeline);
                    // set the instance buffer (no vertex buffer used, vertex positions computed from instances)
                    render_pass.set_vertex_buffer(0, self.rect_buffer.buffer().slice(..));
                    // todo!() maybe not set entire buffer and then adjust the instance indexes that are drawn???
                    render_pass.draw(0..VERTEX_COUNT, r.start as u32..r.end as u32);
                }
                BatchRegion::Text(r, _font) => {
                    render_pass.set_bind_group(1, &atlas_texture.bind_group, &[]);
                    render_pass.set_pipeline(&self.glyph_pipeline);
                    // set the instance buffer (no vertex buffer used, vertex positions computed from instances)
                    render_pass.set_vertex_buffer(0, self.glyph_buffer.buffer().slice(..));
                    // todo!() maybe not set entire buffer and then adjust the instance indexes that are drawn???
                    render_pass.draw(0..VERTEX_COUNT, r.start as u32..r.end as u32);
                }
                BatchRegion::TexturedRect(r, texture) => {
                    render_pass.set_bind_group(1, &texture.bind_group, &[]);
                    render_pass.set_pipeline(&self.textured_rect_pipeline);
                    // set the instance buffer (no vertex buffer used, vertex positions computed from instances)
                    render_pass.set_vertex_buffer(0, self.textured_rect_buffer.buffer().slice(..));
                    // todo!() maybe not set entire buffer and then adjust the instance indexes that are drawn???
                    render_pass.draw(0..VERTEX_COUNT, r.start as u32..r.end as u32);
                }
            }
        }
    }

    // pub fn watch_for_changes(&mut self, ctx: &GraphicsContext) {
    //     // recreate the pipelines if watching some file:
    //     if let Some(watcher) = &self.shader_watcher {
    //         if let Some(new_wgsl) = watcher.check_for_changes() {
    //             let shader_module = ctx.device.create_shader_module(ShaderModuleDescriptor {
    //                 label: Some("Ui Renderer Shaders"),
    //                 source: wgpu::ShaderSource::Wgsl(new_wgsl.into()),
    //             });
    //             self.glyph_pipeline = create_glyph_pipeline(&shader_module, &ctx.device);
    //             self.rect_pipeline = create_rect_pipeline(&shader_module, &self.deps);
    //             self.textured_rect_pipeline =
    //                 create_textured_rect_pipeline(&shader_module, &self.deps);
    //         }
    //     }
    // }

    // todo! draw board in 3d space
}

impl Prepare for UiRenderer {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) {
        // update buffers:
        self.rect_buffer
            .prepare(&self.collected_batches.rects, device, queue);
        self.textured_rect_buffer
            .prepare(&self.collected_batches.textured_rects, device, queue);
        self.glyph_buffer
            .prepare(&self.collected_batches.glyphs, device, queue);
        self.draw_batches.clear();
        std::mem::swap(&mut self.draw_batches, &mut self.collected_batches.batches);
        self.collected_batches.glyphs.clear();
        self.collected_batches.rects.clear();
        self.collected_batches.textured_rects.clear();
    }
}

use crate::modules::VertexT;

fn create_rect_pipeline(
    shader_module: &ShaderModule,
    device: &wgpu::Device,
    screen: &ScreenGR,
) -> wgpu::RenderPipeline {
    create_pipeline::<RectRaw>(
        shader_module,
        "rect_vs",
        "rect_fs",
        "Rect",
        device,
        &[screen.bind_group_layout()],
    )
}

fn create_textured_rect_pipeline(
    shader_module: &ShaderModule,
    device: &wgpu::Device,
    screen: &ScreenGR,
) -> wgpu::RenderPipeline {
    create_pipeline::<RectRawTextured>(
        shader_module,
        "textured_rect_vs",
        "textured_rect_fs",
        "Textured Rect",
        device,
        &[screen.bind_group_layout(), rgba_bind_group_layout(device)],
    )
}

fn create_glyph_pipeline(
    shader_module: &ShaderModule,
    device: &wgpu::Device,
    screen: &ScreenGR,
) -> wgpu::RenderPipeline {
    create_pipeline::<GlyphRaw>(
        shader_module,
        "glyph_vs",
        "glyph_fs",
        "Glyph",
        device,
        &[screen.bind_group_layout(), rgba_bind_group_layout(device)],
    )
}

/// Shared function for both rects and glyphs (both are transparent quads)
fn create_pipeline<I: VertexT>(
    shader_module: &ShaderModule,
    vertex_entry: &str,
    fragment_entry: &str,
    label: &str,
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::RenderPipeline {
    let _empty = &mut vec![];
    let vertex_buffers_layout = &[I::vertex_buffer_layout(0, true, _empty)];

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label} PipelineLayout")),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("{label} Pipeline")),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader_module,
            entry_point: vertex_entry,
            buffers: vertex_buffers_layout,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader_module,
            entry_point: fragment_entry,
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(), // does not really matter because no index and vertex buffer
        depth_stencil: None,

        //  Some(wgpu::DepthStencilState {
        //     format: DEPTH_FORMAT,
        //     depth_write_enabled: false,
        //     depth_compare: wgpu::CompareFunction::Always,
        //     stencil: wgpu::StencilState::default(),
        //     bias: wgpu::DepthBiasState::default(),
        // }),
        multisample: wgpu::MultisampleState {
            alpha_to_coverage_enabled: false,
            count: 1,
            mask: !0,
        },
        multiview: None,
    })
}
