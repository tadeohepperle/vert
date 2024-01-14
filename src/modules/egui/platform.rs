//! Mainly taken from https://github.com/hasenbanck/egui_winit_platform/blob/master/src/lib.rs

use egui::{
    emath::{pos2, vec2},
    Context, Key, Pos2,
};
use egui_wgpu::renderer::ScreenDescriptor;
use std::collections::HashMap;
use winit::{
    dpi::PhysicalSize,
    event::{
        TouchPhase,
        WindowEvent::{self, *},
    },
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::CursorIcon,
};

/// Configures the creation of the `Platform`.
#[derive(Debug, Default)]
pub struct PlatformDescriptor {
    /// Width and Height of the window in physical pixel.
    pub physical_size: PhysicalSize<u32>,
    /// HiDPI scale factor.
    pub pixels_per_point: f32,
    /// Egui font configuration.
    pub font_definitions: egui::FontDefinitions,
    /// Egui style configuration.
    pub style: egui::Style,
}

/// Provides the integration between egui and winit.
pub struct Platform {
    physical_size: PhysicalSize<u32>,
    pixels_per_point: f32,
    context: Context,
    raw_input: egui::RawInput,
    modifier_state: ModifiersState,
    pointer_pos: Option<egui::Pos2>,

    // For emulating pointer events from touch events we merge multi-touch
    // pointers, and ref-count the press state.
    touch_pointer_pressed: u32,

    // Egui requires unique u64 device IDs for touch events but Winit's
    // device IDs are opaque, so we have to create our own ID mapping.
    device_indices: HashMap<winit::event::DeviceId, u64>,
    next_device_index: u64,
}

fn screen_rect(physical_size: PhysicalSize<u32>, pixels_per_point: f32) -> egui::Rect {
    egui::Rect::from_min_size(
        Pos2::default(),
        vec2(physical_size.width as f32, physical_size.height as f32) / pixels_per_point,
    )
}

impl Platform {
    /// Creates a new `Platform`.
    pub fn new(descriptor: PlatformDescriptor) -> Self {
        let context = Context::default();

        context.set_fonts(descriptor.font_definitions.clone());
        context.set_style(descriptor.style);
        let raw_input = egui::RawInput {
            // pixels_per_point: Some(descriptor.scale_factor as f32),
            screen_rect: Some(screen_rect(
                descriptor.physical_size,
                descriptor.pixels_per_point,
            )),
            ..Default::default()
        };

        Self {
            pixels_per_point: descriptor.pixels_per_point,
            physical_size: descriptor.physical_size,
            context,
            raw_input,
            modifier_state: ModifiersState::empty(),
            pointer_pos: Some(Pos2::default()),
            touch_pointer_pressed: 0,
            device_indices: HashMap::new(),
            next_device_index: 1,
        }
    }

    pub fn screen_descriptor(&self) -> ScreenDescriptor {
        ScreenDescriptor {
            size_in_pixels: [self.physical_size.width, self.physical_size.height],
            pixels_per_point: self.pixels_per_point,
        }
    }

    /// Handles the given winit event and updates the egui context. Should be called before starting a new frame with `start_frame()`.
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
            // See: https://github.com/rust-windowing/winit/issues/208
            // There is nothing to do for minimize events, so it is ignored here. This solves an issue where
            // egui window positions would be changed when minimizing on Windows.
            Resized(PhysicalSize {
                width: 0,
                height: 0,
            }) => {}
            Resized(physical_size) => {
                self.physical_size = *physical_size;
                self.raw_input.screen_rect =
                    Some(screen_rect(self.physical_size, self.pixels_per_point));
            }

            ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => {
                self.pixels_per_point = 1.0 / *scale_factor as f32;

                self.raw_input.screen_rect =
                    Some(screen_rect(self.physical_size, self.pixels_per_point));
            }
            MouseInput { state, button, .. } => {
                if let winit::event::MouseButton::Other(..) = button {
                } else {
                    // push event only if the cursor is inside the window
                    if let Some(pointer_pos) = self.pointer_pos {
                        self.raw_input.events.push(egui::Event::PointerButton {
                            pos: pointer_pos,
                            button: match button {
                                winit::event::MouseButton::Left => egui::PointerButton::Primary,
                                winit::event::MouseButton::Right => egui::PointerButton::Secondary,
                                winit::event::MouseButton::Middle => egui::PointerButton::Middle,
                                winit::event::MouseButton::Other(_) => unreachable!(),
                                winit::event::MouseButton::Back => egui::PointerButton::Extra1,
                                winit::event::MouseButton::Forward => egui::PointerButton::Extra2,
                            },
                            pressed: *state == winit::event::ElementState::Pressed,
                            modifiers: Default::default(),
                        });
                    }
                }
            }
            Touch(touch) => {
                let pointer_pos = pos2(
                    touch.location.x as f32 / self.pixels_per_point,
                    touch.location.y as f32 / self.pixels_per_point,
                );

                let device_id = match self.device_indices.get(&touch.device_id) {
                    Some(id) => *id,
                    None => {
                        let device_id = self.next_device_index;
                        self.device_indices.insert(touch.device_id, device_id);
                        self.next_device_index += 1;
                        device_id
                    }
                };
                let egui_phase = match touch.phase {
                    TouchPhase::Started => egui::TouchPhase::Start,
                    TouchPhase::Moved => egui::TouchPhase::Move,
                    TouchPhase::Ended => egui::TouchPhase::End,
                    TouchPhase::Cancelled => egui::TouchPhase::Cancel,
                };

                let force = match touch.force {
                    Some(winit::event::Force::Calibrated { force, .. }) => force as f32,
                    Some(winit::event::Force::Normalized(force)) => force as f32,
                    None => 0.0f32, // hmmm, egui can't differentiate unsupported from zero pressure
                };

                self.raw_input.events.push(egui::Event::Touch {
                    device_id: egui::TouchDeviceId(device_id),
                    id: egui::TouchId(touch.id),
                    phase: egui_phase,
                    pos: pointer_pos,
                    force: Some(force),
                });

                // Currently Winit doesn't emulate pointer events based on
                // touch events but Egui requires pointer emulation.
                //
                // For simplicity we just merge all touch pointers into a
                // single virtual pointer and ref-count the press state
                // (i.e. the pointer will remain pressed during multi-touch
                // events until the last pointer is lifted up)

                let was_pressed = self.touch_pointer_pressed > 0;

                match touch.phase {
                    TouchPhase::Started => {
                        self.touch_pointer_pressed += 1;
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        self.touch_pointer_pressed = match self.touch_pointer_pressed.checked_sub(1)
                        {
                            Some(count) => count,
                            None => {
                                eprintln!("Pointer emulation error: Unbalanced touch start/stop events from Winit");
                                0
                            }
                        };
                    }
                    TouchPhase::Moved => {
                        self.raw_input
                            .events
                            .push(egui::Event::PointerMoved(pointer_pos));
                    }
                }

                if !was_pressed && self.touch_pointer_pressed > 0 {
                    self.raw_input.events.push(egui::Event::PointerButton {
                        pos: pointer_pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    });
                } else if was_pressed && self.touch_pointer_pressed == 0 {
                    // Egui docs say that the pressed=false should be sent _before_
                    // the PointerGone.
                    self.raw_input.events.push(egui::Event::PointerButton {
                        pos: pointer_pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    });
                    self.raw_input.events.push(egui::Event::PointerGone);
                }
            }
            MouseWheel { delta, .. } => {
                let mut delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        let line_height = 8.0; // TODO as in egui_glium
                        vec2(*x, *y) * line_height
                    }
                    winit::event::MouseScrollDelta::PixelDelta(delta) => {
                        vec2(delta.x as f32, delta.y as f32)
                    }
                };
                if cfg!(target_os = "macos") {
                    // See https://github.com/rust-windowing/winit/issues/1695 for more info.
                    delta.x *= -1.0;
                }

                // The ctrl (cmd on macos) key indicates a zoom is desired.
                if self.raw_input.modifiers.ctrl || self.raw_input.modifiers.command {
                    self.raw_input
                        .events
                        .push(egui::Event::Zoom((delta.y / 200.0).exp()));
                } else {
                    self.raw_input.events.push(egui::Event::Scroll(delta));
                }
            }
            CursorMoved { position, .. } => {
                let pointer_pos = pos2(
                    position.x as f32 / self.pixels_per_point,
                    position.y as f32 / self.pixels_per_point,
                );
                self.pointer_pos = Some(pointer_pos);
                self.raw_input
                    .events
                    .push(egui::Event::PointerMoved(pointer_pos));
            }
            CursorLeft { .. } => {
                self.pointer_pos = None;
                self.raw_input.events.push(egui::Event::PointerGone);
            }
            ModifiersChanged(input) => {
                self.modifier_state = input.state();
                self.raw_input.modifiers = winit_to_egui_modifiers(input.state());
            }
            KeyboardInput { event, .. } => {
                let pressed = event.state.is_pressed();
                let ctrl = self.modifier_state.control_key();
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    match (pressed, ctrl, keycode) {
                        (true, true, KeyCode::KeyC) => {
                            self.raw_input.events.push(egui::Event::Copy)
                        }
                        (true, true, KeyCode::KeyX) => self.raw_input.events.push(egui::Event::Cut),
                        // (true, true, KeyCode::KeyV) => {
                        //     #[cfg(feature = "clipboard")]
                        //     if let Some(ref mut clipboard) = self.clipboard {
                        //         if let Ok(contents) = clipboard.get_contents() {
                        //             self.raw_input.events.push(egui::Event::Text(contents))
                        //         }
                        //     }
                        // }
                        _ => {
                            if let Some(key) = winit_to_egui_key_code(keycode) {
                                // This is super annoying but let's do it better later...
                                if pressed {
                                    if let Some(c) =
                                        key_code_to_char(keycode, self.modifier_state.shift_key())
                                    {
                                        self.raw_input
                                            .events
                                            .push(egui::Event::Text(c.to_string()));
                                    }
                                }

                                self.raw_input.events.push(egui::Event::Key {
                                    key,
                                    pressed,
                                    modifiers: winit_to_egui_modifiers(self.modifier_state),
                                    repeat: false,
                                    physical_key: None, // Some(key),
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Returns `true` if egui should handle the event exclusively. Check this to
    /// avoid unexpected interactions, e.g. a mouse click registering "behind" the UI.
    // fn captures_event<T>(&self, winit_event: &Event<T>) -> bool {
    //     match winit_event {
    //         Event::WindowEvent {
    //             window_id: _window_id,
    //             event,
    //         } => match event {
    //             KeyboardInput { .. } | ModifiersChanged(_) => self.context().wants_keyboard_input(),

    //             MouseWheel { .. } | MouseInput { .. } => self.context().wants_pointer_input(),

    //             CursorMoved { .. } => self.context().is_using_pointer(),

    //             Touch { .. } => self.context().is_using_pointer(),

    //             _ => false,
    //         },

    //         _ => false,
    //     }
    // }

    /// Starts a new frame by providing a new `Ui` instance to write into.
    pub fn begin_frame(&mut self, total_elapsed_seconds: f64) {
        self.raw_input.time = Some(total_elapsed_seconds);
        self.context.begin_frame(self.raw_input.take());
    }

    /// Ends the frame. Returns what has happened as `Output` and gives you the draw instructions
    /// as `PaintJobs`. If the optional `window` is set, it will set the cursor key based on
    /// egui's instructions.
    ///
    ///  window: Option<&winit::window::Window>
    pub fn end_frame(&mut self) -> egui::FullOutput {
        // otherwise the below line gets flagged by clippy if both clipboard and webbrowser features are disabled
        #[allow(clippy::let_and_return)]
        let output = self.context.end_frame();
        // if let Some(window) = window {
        //     if let Some(cursor_icon) = egui_to_winit_cursor_icon(output.platform_output.cursor_icon)
        //     {
        //         window.set_cursor_visible(true);
        //         // if the pointer is located inside the window, set cursor icon
        //         if self.pointer_pos.is_some() {
        //             window.set_cursor_icon(cursor_icon);
        //         }
        //     } else {
        //         window.set_cursor_visible(false);
        //     }
        // }
        output
    }

    /// Returns the internal egui context.
    pub fn context(&self) -> Context {
        self.context.clone()
    }

    /// Returns a mutable reference to the raw input that will be passed to egui
    /// the next time [`Self::begin_frame`] is called
    pub fn raw_input_mut(&mut self) -> &mut egui::RawInput {
        &mut self.raw_input
    }
}

fn key_code_to_char(key: KeyCode, shift_pressed: bool) -> Option<char> {
    use KeyCode::*;
    let c = match key {
        Space => ' ',
        Digit1 => '1',
        Digit2 => '2',
        Digit3 => '3',
        Digit4 => '4',
        Digit5 => '5',
        Digit6 => '6',
        Digit7 => '7',
        Digit8 => '8',
        Digit9 => '9',
        Digit0 => '0',
        KeyA => 'A',
        KeyB => 'B',
        KeyC => 'C',
        KeyD => 'D',
        KeyE => 'E',
        KeyF => 'F',
        KeyG => 'G',
        KeyH => 'H',
        KeyI => 'I',
        KeyJ => 'J',
        KeyK => 'K',
        KeyL => 'L',
        KeyM => 'M',
        KeyN => 'N',
        KeyO => 'O',
        KeyP => 'P',
        KeyQ => 'Q',
        KeyR => 'R',
        KeyS => 'S',
        KeyT => 'T',
        KeyU => 'U',
        KeyV => 'V',
        KeyW => 'W',
        KeyX => 'X',
        KeyY => 'Y',
        KeyZ => 'Z',
        _ => {
            return None;
        }
    };

    Some(if !shift_pressed {
        c.to_ascii_lowercase()
    } else {
        c
    })
}

/// Translates winit to egui keycodes. Kinda shitty.
#[inline]
fn winit_to_egui_key_code(key: KeyCode) -> Option<egui::Key> {
    use KeyCode::*;
    Some(match key {
        Escape => Key::Escape,
        Insert => Key::Insert,
        Home => Key::Home,
        Delete => Key::Delete,
        End => Key::End,
        PageDown => Key::PageDown,
        PageUp => Key::PageUp,
        ArrowLeft => Key::ArrowLeft,
        ArrowUp => Key::ArrowUp,
        ArrowRight => Key::ArrowRight,
        ArrowDown => Key::ArrowDown,
        Backspace => Key::Backspace,
        Enter => Key::Enter,
        Tab => Key::Tab,
        Space => Key::Space,
        Digit1 => Key::Num1,
        Digit2 => Key::Num2,
        Digit3 => Key::Num3,
        Digit4 => Key::Num4,
        Digit5 => Key::Num5,
        Digit6 => Key::Num6,
        Digit7 => Key::Num7,
        Digit8 => Key::Num8,
        Digit9 => Key::Num9,
        Digit0 => Key::Num0,
        KeyA => Key::A,
        KeyB => Key::B,
        KeyC => Key::C,
        KeyD => Key::D,
        KeyE => Key::E,
        KeyF => Key::F,
        KeyG => Key::G,
        KeyH => Key::H,
        KeyI => Key::I,
        KeyJ => Key::J,
        KeyK => Key::K,
        KeyL => Key::L,
        KeyM => Key::M,
        KeyN => Key::N,
        KeyO => Key::O,
        KeyP => Key::P,
        KeyQ => Key::Q,
        KeyR => Key::R,
        KeyS => Key::S,
        KeyT => Key::T,
        KeyU => Key::U,
        KeyV => Key::V,
        KeyW => Key::W,
        KeyX => Key::X,
        KeyY => Key::Y,
        KeyZ => Key::Z,
        _ => {
            return None;
        }
    })
}

/// Translates winit to egui modifier keys.
#[inline]
fn winit_to_egui_modifiers(modifiers: ModifiersState) -> egui::Modifiers {
    egui::Modifiers {
        alt: modifiers.alt_key(),
        ctrl: modifiers.control_key(),
        shift: modifiers.shift_key(),
        mac_cmd: false,
        command: modifiers.control_key(),
    }
}

#[inline]
fn _unused_egui_to_winit_cursor_icon(icon: egui::CursorIcon) -> Option<winit::window::CursorIcon> {
    use egui::CursorIcon::*;

    match icon {
        Default => Some(CursorIcon::Default),
        ContextMenu => Some(CursorIcon::ContextMenu),
        Help => Some(CursorIcon::Help),
        PointingHand => Some(CursorIcon::Pointer),
        Progress => Some(CursorIcon::Progress),
        Wait => Some(CursorIcon::Wait),
        Cell => Some(CursorIcon::Cell),
        Crosshair => Some(CursorIcon::Crosshair),
        Text => Some(CursorIcon::Text),
        VerticalText => Some(CursorIcon::VerticalText),
        Alias => Some(CursorIcon::Alias),
        Copy => Some(CursorIcon::Copy),
        Move => Some(CursorIcon::Move),
        NoDrop => Some(CursorIcon::NoDrop),
        NotAllowed => Some(CursorIcon::NotAllowed),
        Grab => Some(CursorIcon::Grab),
        Grabbing => Some(CursorIcon::Grabbing),
        AllScroll => Some(CursorIcon::AllScroll),
        ResizeHorizontal => Some(CursorIcon::EwResize),
        ResizeNeSw => Some(CursorIcon::NeswResize),
        ResizeNwSe => Some(CursorIcon::NwseResize),
        ResizeVertical => Some(CursorIcon::NsResize),
        ResizeEast => Some(CursorIcon::EResize),
        ResizeSouthEast => Some(CursorIcon::SeResize),
        ResizeSouth => Some(CursorIcon::SResize),
        ResizeSouthWest => Some(CursorIcon::SwResize),
        ResizeWest => Some(CursorIcon::WResize),
        ResizeNorthWest => Some(CursorIcon::NwResize),
        ResizeNorth => Some(CursorIcon::NResize),
        ResizeNorthEast => Some(CursorIcon::NeResize),
        ResizeColumn => Some(CursorIcon::ColResize),
        ResizeRow => Some(CursorIcon::RowResize),
        ZoomIn => Some(CursorIcon::ZoomIn),
        ZoomOut => Some(CursorIcon::ZoomOut),
        None => Option::None,
    }
}

/// We only want printable characters and ignore all special keys.
#[inline]
fn _unused_is_printable(chr: char) -> bool {
    let is_in_private_use_area = ('\u{e000}'..='\u{f8ff}').contains(&chr)
        || ('\u{f0000}'..='\u{ffffd}').contains(&chr)
        || ('\u{100000}'..='\u{10fffd}').contains(&chr);

    !is_in_private_use_area && !chr.is_ascii_control()
}
