use std::{
    ops::{FromResidual, Try},
    sync::Arc,
};

use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub enum UpdateFlow {
    Exit(String),
    Continue,
}

impl FromResidual<UpdateFlow> for UpdateFlow {
    fn from_residual(residual: UpdateFlow) -> Self {
        residual
    }
}

impl Try for UpdateFlow {
    type Output = ();

    type Residual = Self;

    fn from_output(output: Self::Output) -> Self {
        Self::Continue
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            UpdateFlow::Exit(e) => std::ops::ControlFlow::Break(UpdateFlow::Exit(e)),
            UpdateFlow::Continue => std::ops::ControlFlow::Continue(()),
        }
    }
}

pub trait App {
    fn receive_window_event(&mut self, event: &WindowEvent);

    fn update(&mut self) -> UpdateFlow;
}

pub struct WinitConfig {
    pub window_name: &'static str,
    pub width: u32,
    pub height: u32,
}

impl Default for WinitConfig {
    fn default() -> Self {
        Self {
            window_name: "Vert App",
            width: 1200,
            height: 700,
        }
    }
}

pub struct WinitRunner {
    event_loop: EventLoop<()>,
    window: Arc<Window>,
}

impl WinitRunner {
    pub fn window(&self) -> Arc<Window> {
        self.window.clone()
    }

    pub fn new(config: WinitConfig) -> Self {
        let event_loop = EventLoop::new().unwrap();

        let monitor = event_loop.primary_monitor().unwrap();
        let _video_mode = monitor.video_modes().next();
        // let size = video_mode
        //     .clone()
        //     .map_or(PhysicalSize::new(800, 600), |vm| vm.size());

        let size = PhysicalSize::new(1200, 700);

        let window = WindowBuilder::new()
            .with_visible(true)
            .with_title(config.window_name)
            .with_inner_size(size)
            .build(&event_loop)
            .unwrap();
        let window = Arc::new(window);

        Self { event_loop, window }
    }

    pub fn run(self, app: &mut dyn App) -> anyhow::Result<()> {
        let window = self.window.clone();
        self.event_loop.run(move |event, window_target| {
            // check what kinds of events received:
            match &event {
                Event::NewEvents(_) => {}
                Event::WindowEvent { window_id, event } => {
                    if *window_id != self.window.id() {
                        return;
                    }

                    app.receive_window_event(event);

                    if matches!(event, WindowEvent::RedrawRequested) {
                        //  this is called every frame:
                        match app.update() {
                            UpdateFlow::Exit(reason) => {
                                println!("Exit: {reason}");
                                window_target.exit();
                            }
                            UpdateFlow::Continue => window.request_redraw(),
                        }
                    }
                }
                Event::DeviceEvent { .. } => {}
                Event::UserEvent(_) => {}
                Event::Suspended => {}
                Event::Resumed => {}
                Event::AboutToWait => {}
                Event::LoopExiting => {}
                Event::MemoryWarning => {}
            }
        })?;
        Ok(())
    }
}
