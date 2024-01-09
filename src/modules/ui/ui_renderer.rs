use std::vec;

use wgpu::BufferUsages;
use wgpu::MultisampleState;
use wgpu::RenderPipelineDescriptor;
use wgpu::ShaderModuleDescriptor;

use crate::Dependencies;

use crate::elements::texture::rgba_bind_group_layout;
use crate::elements::GrowableBuffer;
use crate::modules::renderer::DEPTH_FORMAT;
use crate::modules::renderer::HDR_COLOR_FORMAT;
use crate::modules::renderer::MSAA_SAMPLE_COUNT;
use crate::modules::ui::board::BoardPhase;
use crate::modules::GraphicsContext;
use crate::modules::MainPassRenderer;
use crate::modules::MainScreenSize;
use crate::modules::Prepare;
use crate::modules::Renderer;
use crate::utils::watcher::ShaderFileWatcher;
use crate::utils::Timing;
use crate::Handle;
use crate::Module;

use super::batching::get_batches;
use super::batching::BatchRegion;
use super::batching::BatchingResult;
use super::batching::GlyphRaw;
use super::batching::RectRaw;
use super::board::Board;
use super::font_cache::FontCache;

pub struct UiRenderer {
    glyph_shader_watcher: Option<ShaderFileWatcher>,
    glyph_pipeline: wgpu::RenderPipeline,
    rect_shader_watcher: Option<ShaderFileWatcher>,
    rect_pipeline: wgpu::RenderPipeline,
    collected_batches: BatchingResult,
    draw_batches: Vec<BatchRegion>,
    rect_buffer: GrowableBuffer<RectRaw>,
    glyph_buffer: GrowableBuffer<GlyphRaw>,
    deps: Deps,
}

#[derive(Debug, Dependencies)]
pub struct Deps {
    renderer: Handle<Renderer>,
    fonts: Handle<FontCache>,
    ctx: Handle<GraphicsContext>,
    main_screen: Handle<MainScreenSize>,
}

impl Module for UiRenderer {
    type Config = ();
    type Dependencies = Deps;

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        let device = &deps.ctx.device;
        let rect_buffer = GrowableBuffer::new(device, 512, BufferUsages::VERTEX);
        let glyph_buffer = GrowableBuffer::new(device, 512, BufferUsages::VERTEX);

        let text_shader_watcher = None;
        let rect_shader_watcher = None;

        let text_pipeline = create_pipeline::<GlyphRaw>(
            include_str!("glyph.wgsl"),
            "Glyph",
            device,
            &[
                deps.main_screen.bind_group_layout(),
                rgba_bind_group_layout(device),
            ],
        );
        let rect_pipeline = create_pipeline::<RectRaw>(
            include_str!("rect.wgsl"),
            "Rect",
            device,
            &[deps.main_screen.bind_group_layout()],
        );

        Ok(UiRenderer {
            glyph_shader_watcher: text_shader_watcher,
            glyph_pipeline: text_pipeline,
            rect_shader_watcher,
            rect_pipeline,
            collected_batches: BatchingResult::new(),
            draw_batches: vec![],
            rect_buffer,
            glyph_buffer,
            deps,
        })
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut renderer = handle.deps.renderer;
        renderer.register_prepare(handle);
        renderer.register_main_pass_renderer(handle, Timing::LATE + 100);
        Ok(())
    }
}

impl UiRenderer {
    pub fn watch_rect_shader_file(&mut self, path: &str) {
        self.rect_shader_watcher = Some(ShaderFileWatcher::new(path));
    }

    /// Warning: only call AFTER layout has been performed for this frame. (needs to be BillboardPhase::Rendering)
    /// Assumes all rects and text layouts are calculated.
    pub fn draw_billboard(&mut self, board: &Board) {
        // ensure layout has been done by checking the phase
        assert_eq!(board.phase(), BoardPhase::Rendering);
        let batches = get_batches(board);
        // dbg!(&batches);
        self.collected_batches.combine(batches);

        // println!("draw_billboard");
    }
}

impl Prepare for UiRenderer {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if let Some(s) = &self.rect_shader_watcher {
            if let Some(new_wgsl) = s.check_for_changes() {
                self.rect_pipeline = create_pipeline::<RectRaw>(
                    &new_wgsl,
                    "Rect",
                    device,
                    &[self.deps.main_screen.bind_group_layout()],
                );
            }
        }

        self.rect_buffer
            .prepare(&self.collected_batches.rects, device, queue);
        self.glyph_buffer
            .prepare(&self.collected_batches.glyphs, device, queue);
        self.draw_batches.clear();
        std::mem::swap(&mut self.draw_batches, &mut self.collected_batches.batches);
        self.collected_batches.glyphs.clear();
        self.collected_batches.rects.clear();
    }
}

impl MainPassRenderer for UiRenderer {
    fn render<'pass, 'encoder>(&'encoder self, render_pass: &'pass mut wgpu::RenderPass<'encoder>) {
        assert!(self.collected_batches.is_empty()); // only information left should be in draw_batches.
                                                    // println!("render UiRenderer");
        render_pass.set_bind_group(0, self.deps.main_screen.bind_group(), &[]);

        // 6 indices to draw two triangles
        const VERTEX_COUNT: u32 = 6;
        for batch in self.draw_batches.iter() {
            match batch {
                BatchRegion::Rect(r) => {
                    render_pass.set_pipeline(&self.rect_pipeline);
                    // set the instance buffer (no vertex buffer used, vertex positions computed from instances)
                    render_pass.set_vertex_buffer(0, self.rect_buffer.buffer().slice(..));
                    // todo!() maybe not set entire buffer and then adjust the instance indexes that are drawn???
                    render_pass.draw(0..VERTEX_COUNT, r.start as u32..r.end as u32);
                }
                BatchRegion::Text(r, font) => {
                    let atlas_texture = self.deps.fonts.atlas_texture();
                    render_pass.set_bind_group(1, &atlas_texture.bind_group, &[]);
                    render_pass.set_pipeline(&self.glyph_pipeline);
                    // set the instance buffer (no vertex buffer used, vertex positions computed from instances)
                    render_pass.set_vertex_buffer(0, self.glyph_buffer.buffer().slice(..));
                    // todo!() maybe not set entire buffer and then adjust the instance indexes that are drawn???
                    render_pass.draw(0..VERTEX_COUNT, r.start as u32..r.end as u32);
                }
            }
        }
    }
}

use crate::modules::VertexT;
/// Shared function for both rects and glyphs (both are transparent quads)
pub fn create_pipeline<I: VertexT>(
    shader_wgsl: &str,
    label: &str,
    device: &wgpu::Device,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
) -> wgpu::RenderPipeline {
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("{label} ShaderModule")),
        source: wgpu::ShaderSource::Wgsl(shader_wgsl.into()),
    });

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
            module: &shader_module,
            entry_point: "vs_main",
            buffers: vertex_buffers_layout,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: HDR_COLOR_FORMAT,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(), // does not really matter because no index and vertex buffer
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: MultisampleState {
            count: MSAA_SAMPLE_COUNT,
            ..Default::default()
        },
        multiview: None,
    })
}
