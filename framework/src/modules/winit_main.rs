use crate::{app::ModuleId, Handle, MainModule, Module};
use anyhow::anyhow;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

use super::{scheduler::UpdateFlow, Scheduler, WinitWindowEventReceiver};

#[derive(Debug, Clone, PartialEq)]
pub struct WinitMainConfig {
    window_name: &'static str,
}

impl Default for WinitMainConfig {
    fn default() -> Self {
        Self {
            window_name: "Vert Game Engine",
        }
    }
}

/// A MainModule that creates a winit_app.
pub struct WinitMain {
    /// Should be some, if the WinitMain is built, and the value is taken, leaving None, when the main function is run.
    event_loop: Option<EventLoop<()>>,
    window: Window,
    event_listeners: Vec<(ModuleId, &'static mut dyn WinitWindowEventReceiver)>,
    scheduler: Handle<Scheduler>,
}

impl WinitMain {
    pub fn window(&self) -> &Window {
        &self.window
    }
}

impl Module for WinitMain {
    type Config = WinitMainConfig;

    type Dependencies = Handle<Scheduler>;

    fn new(config: Self::Config, scheduler: Self::Dependencies) -> anyhow::Result<Self> {
        let event_loop = EventLoop::new().unwrap();

        let monitor = event_loop.primary_monitor().unwrap();
        let video_mode = monitor.video_modes().next();
        // let size = video_mode
        //     .clone()
        //     .map_or(PhysicalSize::new(800, 600), |vm| vm.size());

        let size = PhysicalSize::new(400, 300);

        let window = WindowBuilder::new()
            .with_visible(true)
            .with_title(config.window_name)
            .with_inner_size(size)
            .build(&event_loop)
            .unwrap();

        Ok(WinitMain {
            event_loop: Some(event_loop),
            window,
            event_listeners: vec![],
            scheduler,
        })
    }
}

impl MainModule for WinitMain {
    fn main(&mut self, app: &crate::App) -> anyhow::Result<()> {
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, window_target| {
            // check what kinds of events received:
            match &event {
                Event::NewEvents(_) => {}
                Event::WindowEvent { window_id, event } => {
                    if *window_id != self.window.id() {
                        return;
                    }

                    self.receive_window_event(event);

                    if matches!(event, WindowEvent::RedrawRequested) {
                        //  this is called every frame:
                        let res = self.scheduler.update();
                        match res {
                            UpdateFlow::Exit(reason) => {
                                println!("Exit: {reason}");
                                window_target.exit();
                            }
                            UpdateFlow::Continue => self.window.request_redraw(),
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

impl WinitMain {
    pub fn register_event_listener<M: WinitWindowEventReceiver + Module>(
        &mut self,
        handle: &Handle<M>,
    ) -> anyhow::Result<()> {
        let m_id = ModuleId::of::<M>();
        if self.event_listeners.iter().any(|e| e.0 == m_id) {
            return Err(anyhow!("Module {m_id} is already registered, as an event_listener for the WinitMain module"));
        }
        let trait_obj: &mut dyn WinitWindowEventReceiver = handle.get_mut();
        self.event_listeners.push((m_id, trait_obj));
        Ok(())
    }

    fn receive_window_event(&mut self, event: &WindowEvent) {
        for (_, event_listener) in self.event_listeners.iter_mut() {
            event_listener.receive_window_event(event);
        }
    }
}
