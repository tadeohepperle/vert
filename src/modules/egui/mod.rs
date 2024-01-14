use egui::{ClippedPrimitive, Context};



use winit::{event::WindowEvent};

use crate::{modules::Schedule, modules::WinitMain, utils::Timing, Dependencies, Handle, Module};

use self::platform::{Platform, PlatformDescriptor};

use super::{renderer::SdrSurfaceRenderer, GraphicsContext, Prepare, Renderer, Scheduler, Time};

pub mod platform;

/// ## How to use the functions exposed by EguiState:
///
/// ### Outside of a frame (window event):
/// receive_window_event
/// `receive_window_event`: whenever some event is issued by winit:
/// e.g. keyboard input, resizing, etc...
///
/// ### In a Frame (redraw requested)
/// - `begin_frame`: call at the start of a new frame. Sets the total time.
/// - ...
/// - ... other game code can make updates to self.context()
/// - ...
/// - `prepare`: clears all previous buffers + textures, closes the frame and updates buffers and textures on the gpu => creates and caches paint jobs
/// - `render`: draws the paint jobs in a render pass
///
pub struct Egui {
    pub platform: platform::Platform,
    pub renderer: egui_wgpu::Renderer,
    paint_jobs: Vec<ClippedPrimitive>,
    textures_delta: egui::TexturesDelta,
    deps: Deps,
    // demo_windows: DemoWindows,
}

#[derive(Debug, Dependencies)]
pub struct Deps {
    renderer: Handle<Renderer>,
    ctx: Handle<GraphicsContext>,
    winit: Handle<WinitMain>,
    time: Handle<Time>,
    scheduler: Handle<Scheduler>,
}

impl Module for Egui {
    type Config = ();

    type Dependencies = Deps;

    fn new(_config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        // Important note: pixels_per_point is the inverse of the devices scale_factor.

        let platform = Platform::new(PlatformDescriptor {
            physical_size: deps.ctx.size,
            pixels_per_point: 1.0 / deps.ctx.scale_factor() as f32, // ??? is this updated properly?
            font_definitions: Default::default(),
            style: Default::default(),
        });

        let renderer = egui_wgpu::Renderer::new(&deps.ctx.device, deps.ctx.surface_format, None, 1);
        // renderer.render(render_pass, paint_jobs, self.platform);
        Ok(Egui {
            platform,
            renderer,
            textures_delta: Default::default(),
            paint_jobs: Vec::new(),
            deps,
            // demo_windows: DemoWindows::default(),
        })
    }

    fn intialize(handle: Handle<Self>) -> anyhow::Result<()> {
        let mut winit = handle.deps.winit;
        winit.register_window_event_listener(handle, Self::receive_window_event);

        let mut renderer = handle.deps.renderer;
        renderer.register_prepare(handle);
        renderer.register_surface_renderer(handle, Timing::DEFAULT);

        let mut scheduler = handle.deps.scheduler;
        scheduler.register(handle, Schedule::Update, Timing::EARLY, Self::begin_frame);
        Ok(())
    }
}

impl Prepare for Egui {
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let output = self.platform.end_frame();
        self.paint_jobs.clear();
        for id in self.textures_delta.free.drain(..) {
            self.renderer.free_texture(&id)
        }
        self.textures_delta = output.textures_delta;
        for (id, image_delta) in self.textures_delta.set.iter() {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.paint_jobs = self
            .context()
            .tessellate(output.shapes, output.pixels_per_point);

        let screen_descriptor = self.platform.screen_descriptor();
        self.renderer
            .update_buffers(device, queue, encoder, &self.paint_jobs, &screen_descriptor);
    }
}

impl SdrSurfaceRenderer for Egui {
    fn render<'e>(&'e self, encoder: &'e mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let color_attachment = wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Discard,
            },
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Renderpass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let screen_descriptor = self.platform.screen_descriptor();
        self.renderer
            .render(&mut render_pass, &self.paint_jobs, &screen_descriptor);
    }
}

impl Egui {
    pub fn context(&self) -> Context {
        self.platform.context()
    }

    pub fn receive_window_event(&mut self, event: &WindowEvent) {
        self.platform.handle_event(event);
    }

    pub fn begin_frame(&mut self) {
        let total_elapsed_seconds = self.deps.time.total().as_secs_f64();
        self.platform.begin_frame(total_elapsed_seconds);
        // self.demo_windows.ui(&self.context()); // activate this to see the demo windows!
    }
}

/*

# A Brief overview of how it is done in this example: https://github.com/hasenbanck/egui_example

(not 1:1 the same because the example uses older versions of egui and winit and wgpu)


on window event:  platform.handle_event(event)

on redraw:  platform.update_time(start_time.elapsed().as_secs_f64());
            platform.begin_frame();
            demo_app.ui(&platform.context());
            let full_output = platform.end_frame(Some(&window));



            start new render pass:
                encoder = ...

            // upload stuff to GPU:
            let screen_descriptor = ScreenDescriptor {
                physical_width: surface_config.width,
                physical_height: surface_config.height,
                scale_factor: window.scale_factor() as f32,
            };
            let tdelta: egui::TexturesDelta = full_output.textures_delta;
            egui_rpass
                .add_textures(&device, &queue, &tdelta)
                .expect("add texture ok");
            egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

             // Record all render passes.
            egui_rpass
                .execute(
                    &mut encoder,
                    &output_view,
                    &paint_jobs,
                    &screen_descriptor,
                    Some(wgpu::Color::BLACK),
                )
                .unwrap();


            // queue.submit(iter::once(encoder.finish()));

            // Redraw egui
            output_frame.present();

            egui_rpass
                .remove_textures(tdelta)
                .expect("remove texture ok");


*/
