use std::fmt::Debug;

use glam::{vec2, Vec2};
use smallvec::SmallVec;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Debug, Clone, Default)]
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
}

impl Input {
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
        v.normalize()
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

    pub fn receive_window_event(&mut self, window_event: &WindowEvent) {
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
                        // todo!()
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
            } => {}
            WindowEvent::ThemeChanged(_) => {}
            WindowEvent::Occluded(_) => {}
            WindowEvent::RedrawRequested => {}
            WindowEvent::ActivationTokenDone {
                serial: _,
                token: _,
            } => {}
        }
    }

    pub fn clear_at_end_of_frame(&mut self) {
        // dbg!(self.keys.just_pressed.len());
        // dbg!(self.mouse_buttons.just_pressed.len());
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
