use std::fmt::Debug;

use glam::{vec2, Vec2, Vec3};
use smallvec::SmallVec;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

use crate::{
    app::{FunctionHandle, ModuleId, UntypedHandle},
    utils::{Timing, TimingQueue},
    Dependencies, Handle, Module, WinitMain,
};

use super::{winit_main, Schedule, Scheduler};

#[derive(Debug, Dependencies)]
pub struct InputDependencies {
    winit_main: Handle<WinitMain>,
    scheduler: Handle<Scheduler>,
}

#[derive(Debug)]
pub struct Input {
    keys: ElementStateCache<KeyCode>,
    mouse_buttons: ElementStateCache<MouseButton>,
    resized: Option<PhysicalSize<u32>>,
    close_requested: bool,
    cursor_just_moved: bool,
    cursor_just_entered: bool,
    cursor_just_left: bool,
    cursor_pos: Vec2,
    cursor_delta: Vec2,
    scroll: Option<f32>,
    deps: InputDependencies,
    resize_event_listerners: TimingQueue<FunctionHandle<ResizeEvent>>,
}

#[derive(Debug, Clone, Copy)]
pub struct ResizeEvent {
    pub new_size: PhysicalSize<u32>,
}

impl Module for Input {
    type Config = ();
    type Dependencies = InputDependencies;

    fn new(config: Self::Config, deps: Self::Dependencies) -> anyhow::Result<Self> {
        Ok(Input {
            keys: Default::default(),
            mouse_buttons: Default::default(),
            resized: Default::default(),
            close_requested: Default::default(),
            cursor_just_moved: Default::default(),
            cursor_just_entered: Default::default(),
            cursor_just_left: Default::default(),
            cursor_pos: Default::default(),
            cursor_delta: Default::default(),
            scroll: Default::default(),
            deps,
            resize_event_listerners: TimingQueue::new(),
        })
    }

    fn intialize(mut handle: Handle<Self>) -> anyhow::Result<()> {
        let cloned_handle = handle;
        handle
            .deps
            .winit_main
            .register_window_event_listener(cloned_handle, Self::receive_window_event)
            .unwrap();

        handle.deps.scheduler.register(
            cloned_handle,
            Schedule::Update,
            Timing::START,
            Self::start_of_frame,
        );

        handle.deps.scheduler.register(
            cloned_handle,
            Schedule::Update,
            Timing::END,
            Self::end_of_frame,
        );
        Ok(())
    }
}

impl Input {
    fn receive_window_event(&mut self, window_event: &WindowEvent) {
        match window_event {
            WindowEvent::Resized(new_size) => {
                self.resized = Some(*new_size);
            }
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state,
                    ..
                } = event
                {
                    self.keys.receive_element_state(*key, *state)
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.cursor_just_moved = true;
                let new_cursor_pos = vec2(position.x as f32, position.y as f32);
                self.cursor_delta = new_cursor_pos - self.cursor_pos;
                self.cursor_pos = new_cursor_pos;
            }
            WindowEvent::CursorEntered { device_id: _ } => {
                self.cursor_just_entered = true;
            }
            WindowEvent::CursorLeft { device_id: _ } => {
                self.cursor_just_left = true;
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                println!("scroll: {delta:?}");
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(right, down) => {
                        let scroll = self.scroll.get_or_insert(0.0);
                        *scroll += down;
                    }
                    winit::event::MouseScrollDelta::PixelDelta(_) => {
                        // Default::default()
                    }
                }
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                self.mouse_buttons.receive_element_state(*button, *state);
            }
            // /////////////////////////////////////////////////////////////////////////////
            // Currently unused:
            // /////////////////////////////////////////////////////////////////////////////
            WindowEvent::Moved(_) => {}
            WindowEvent::Destroyed => {}
            WindowEvent::DroppedFile(_) => {}
            WindowEvent::HoveredFile(_) => {}
            WindowEvent::HoveredFileCancelled => {}
            WindowEvent::Focused(_) => {}
            WindowEvent::ModifiersChanged(_) => {}
            WindowEvent::Ime(_) => {}

            WindowEvent::TouchpadMagnify {
                device_id: _,
                delta: _,
                phase: _,
            } => {}
            WindowEvent::SmartMagnify { device_id: _ } => {}
            WindowEvent::TouchpadRotate {
                device_id: _,
                delta: _,
                phase: _,
            } => {}
            WindowEvent::TouchpadPressure {
                device_id: _,
                pressure: _,
                stage: _,
            } => {}
            WindowEvent::AxisMotion {
                device_id: _,
                axis: _,
                value: _,
            } => {}
            WindowEvent::Touch(_) => {}
            WindowEvent::ScaleFactorChanged {
                scale_factor: _,
                inner_size_writer: _,
            } => {
                // Default::default()
            }
            WindowEvent::ThemeChanged(_) => {}
            WindowEvent::Occluded(_) => {}
            WindowEvent::RedrawRequested => {}
            WindowEvent::ActivationTokenDone {
                serial: _,
                token: _,
            } => {}
        }
    }

    fn start_of_frame(&mut self) {
        // notify resize event listeners at the start of the frame.
        if let Some(new_size) = self.resized {
            let event = ResizeEvent { new_size };
            for listener in self.resize_event_listerners.iter() {
                listener.call(event);
            }
        }
    }

    fn end_of_frame(&mut self) {
        // dbg!(self.keys.just_pressed.len());
        // dbg!(self.mouse_buttons.just_pressed.len());
        if self.keys.just_pressed(KeyCode::KeyW) {
            println!("W");
        }

        if self.close_requested {
            self.deps.scheduler.request_exit("Close Button Pressed");
        }

        self.keys.clear_at_end_of_frame();
        self.mouse_buttons.clear_at_end_of_frame();
        self.resized = None;
        self.scroll = None;
        self.close_requested = false;
        self.cursor_just_entered = false;
        self.cursor_just_left = false;
        self.cursor_just_moved = false;
        self.cursor_delta = Vec2::ZERO;
    }

    pub fn wasd_vec(&self) -> glam::Vec2 {
        let mut v = Vec2::ZERO;
        if self.keys.is_pressed(KeyCode::KeyW) {
            v.y += 1.0;
        }
        if self.keys.is_pressed(KeyCode::KeyS) {
            v.y -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::KeyA) {
            v.x -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::KeyD) {
            v.x += 1.0;
        }
        if v != Vec2::ZERO {
            v.normalize()
        } else {
            v
        }
    }

    pub fn space_shift_updown(&self) -> f32 {
        let mut v = 0.0;
        if self.keys.is_pressed(KeyCode::ShiftLeft) {
            v -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::Space) {
            v += 1.0;
        }
        v
    }

    pub fn rf_updown(&self) -> f32 {
        let mut v = 0.0;
        if self.keys.is_pressed(KeyCode::KeyF) {
            v -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::KeyR) {
            v += 1.0;
        }
        v
    }

    pub fn arrow_vec(&self) -> glam::Vec2 {
        let mut v = Vec2::ZERO;
        if self.keys.is_pressed(KeyCode::ArrowUp) {
            v.y += 1.0;
        }
        if self.keys.is_pressed(KeyCode::ArrowDown) {
            v.y -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::ArrowLeft) {
            v.x -= 1.0;
        }
        if self.keys.is_pressed(KeyCode::ArrowRight) {
            v.x += 1.0;
        }
        if v != Vec2::ZERO {
            v.normalize()
        } else {
            v
        }
    }

    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    pub fn cursor_just_moved(&self) -> bool {
        self.cursor_just_moved
    }

    pub fn cursor_just_entered(&self) -> bool {
        self.cursor_just_entered
    }

    pub fn cursor_just_left(&self) -> bool {
        self.cursor_just_left
    }

    pub fn cursor_pos(&self) -> Vec2 {
        self.cursor_pos
    }

    pub fn cursor_delta(&self) -> Vec2 {
        self.cursor_delta
    }

    pub fn resized(&self) -> Option<PhysicalSize<u32>> {
        self.resized
    }

    pub fn keys(&self) -> &ElementStateCache<KeyCode> {
        &self.keys
    }

    pub fn mouse_buttons(&self) -> &ElementStateCache<MouseButton> {
        &self.mouse_buttons
    }

    pub fn scroll(&self) -> Option<f32> {
        self.scroll
    }

    pub fn register_resize_event_listener<M: Module>(
        &mut self,
        handle: Handle<M>,
        func: fn(&mut M, new_size: ResizeEvent),
        timing: Timing,
    ) {
        self.resize_event_listerners
            .insert(FunctionHandle::new(handle, func), timing);
    }
}

#[derive(Debug, Clone)]
pub struct ElementStateCache<T> {
    just_pressed: SmallVec<[T; 8]>,
    pressed: SmallVec<[T; 8]>,
    just_released: SmallVec<[T; 8]>,
}
impl<T> Default for ElementStateCache<T> {
    fn default() -> Self {
        Self {
            just_pressed: Default::default(),
            pressed: Default::default(),
            just_released: Default::default(),
        }
    }
}

impl<T: Copy + PartialEq + Debug> ElementStateCache<T> {
    pub fn is_pressed(&self, key: T) -> bool {
        self.pressed.contains(&key)
    }
    pub fn just_pressed(&self, key: T) -> bool {
        self.just_pressed.contains(&key)
    }
    pub fn just_released(&self, key: T) -> bool {
        self.just_released.contains(&key)
    }

    pub fn clear_at_end_of_frame(&mut self) {
        // A weird note: forgetting to clear these leads to performance drops from 1400 fps to about 300 fps.
        // Even though they don't seem to grow at all.
        // - Tadeo Hepperle, 2023-12-13
        self.just_pressed.clear();
        self.just_released.clear();
    }

    pub fn receive_element_state(&mut self, value: T, element_state: ElementState) {
        let pressed_already = self.pressed.contains(&value);

        match element_state {
            ElementState::Released => {
                if pressed_already {
                    // remove it from pressed:
                    self.pressed.retain(|e| *e != value);
                }
                self.just_released.push(value);
            }
            ElementState::Pressed => {
                self.just_pressed.push(value);
                self.pressed.push(value);
            }
        }
    }
}
