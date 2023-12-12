use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use crate::{flow::Flow, modules::Modules};

pub struct App<S: StateT> {
    event_loop: EventLoop<()>,
    window: Window,
    modules: Modules,
    state: S,
}

/// User defined application state
pub trait StateT: Sized {
    async fn initialize(modules: &Modules) -> anyhow::Result<Self>;
}

impl<S: StateT> App<S> {
    pub fn run() -> anyhow::Result<()> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let sel = rt.block_on(Self::initialize())?;
        rt.block_on(sel.run_event_loop())?;
        Ok(())
    }

    async fn initialize() -> anyhow::Result<Self> {
        let event_loop = EventLoop::new().unwrap();
        let monitor = event_loop.primary_monitor().unwrap();
        let video_mode = monitor.video_modes().next();
        let size = video_mode
            .clone()
            .map_or(PhysicalSize::new(800, 600), |vm| vm.size());

        let window = WindowBuilder::new()
            .with_visible(true)
            .with_title("Nice App")
            .with_inner_size(size)
            .build(&event_loop)
            .unwrap();

        let modules = Modules::initialize(&window).await?;
        let state = S::initialize(&modules).await?;
        let app = Self {
            event_loop,
            window,
            modules,
            state,
        };
        Ok(app)
    }

    async fn run_event_loop(self) -> anyhow::Result<()> {
        let Self {
            event_loop,
            window,
            mut modules,
            mut state,
        } = self;

        event_loop.run(move |event, window_target| {
            // check what kinds of events received:
            match &event {
                Event::NewEvents(_) => {}
                Event::WindowEvent { window_id, event } => {
                    if *window_id != window.id() {
                        return;
                    }
                    modules.receive_window_event(event);
                    if matches!(event, WindowEvent::RedrawRequested) {
                        //  this is called every frame:
                        let should_exit = run_frame(&mut modules, &mut state);
                        match should_exit {
                            Flow::Continue => {
                                window.request_redraw();
                            }
                            Flow::Exit => {
                                window_target.exit();
                            }
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

fn run_frame<S: StateT>(modules: &mut Modules, state: &mut S) -> Flow {
    // preupdate

    // update

    // prepare

    // render

    // end frame

    Flow::Continue
}
