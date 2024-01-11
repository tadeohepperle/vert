use std::borrow::Cow;

use fontdue::Font;

use crate::{
    elements::Color,
    modules::{
        arenas::Key,
        input::{MouseButtonState, PressState},
        ui::{
            board::{
                Align, Board, BorderRadius, ContainerId, DivProps, DivStyle,
                HotActive::{self, *},
                Id, Len, MainAlign, Text,
            },
            font_cache::FontSize,
        },
    },
};

use super::Widget;

pub struct Button {
    pub text: Cow<'static, str>,
    pub text_color: Color,
    pub color: Color,
    pub hover_color: Color,
    pub click_color: Color,
    pub font: Option<Key<Font>>,
}

impl Default for Button {
    fn default() -> Self {
        Button {
            text: "Button 1".into(),
            text_color: Color::BLACK,
            color: Color::u8(77, 130, 176),
            hover_color: Color::u8(151, 174, 194),
            font: None,
            click_color: Color::u8(188, 115, 201),
        }
    }
}

pub struct ButtonResponse {
    pub clicked: bool,
}

impl Widget for Button {
    type Response<'a> = ButtonResponse;

    fn add_to_board<'a>(
        self,
        board: &'a mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> ButtonResponse {
        let hot_active = board.hot_active(id);
        let left_button = board.input().mouse_buttons.left();
        let mut response = board.add_text_div(
            DivProps {
                width: Len::Px(200.0),
                height: Len::ChildrenFraction(1.5),
                main_align: MainAlign::Center,
                cross_align: Align::Center,
                ..Default::default()
            },
            DivStyle {
                color: self.color,
                border_color: Color::BLACK,
                border_radius: BorderRadius::all(16.0),
                border_thickness: 10.0,
                border_softness: 16.0,
                z_bias: 0,
                offset_x: Len::ZERO,
                offset_y: Len::ZERO,
                texture: None,
            },
            Text {
                color: self.text_color,
                string: self.text,
                font: self.font,
                size: FontSize(24),
                offset_x: Len::ZERO,
                offset_y: Len::Px(-4.0),
            },
            id,
            parent,
        );

        let mouse_in_rect = response.mouse_in_rect();
        let (next_hot_active, clicked) =
            next_hot_active_and_clicked(hot_active, mouse_in_rect, left_button);

        // we can now update the style immediately. Using the hot_active only on insertion instead of next_hot_active
        // would always be 1 frame behind. Just add a 150ms of workload on each frame (7fps) and you will feel the different.
        let style = response.style();
        style.color = match next_hot_active {
            HotActive::Nil => self.color,
            HotActive::Hot => self.hover_color,
            HotActive::Active => self.click_color,
        };

        if next_hot_active != hot_active {
            board.set_hot_active(id, next_hot_active);
        }
        ButtonResponse { clicked }
    }
}

/// Shout out to Casey Muratori, our lord and savior. (See this Video as well for an exmplanation: https://www.youtube.com/watch?v=geZwWo-qNR4)
fn next_hot_active_and_clicked(
    hot_active: HotActive,
    mouse_in_rect: bool,
    button_press: PressState,
) -> (HotActive, bool) {
    let mut clicked: bool = false;
    let next = match hot_active {
        Nil => {
            if mouse_in_rect {
                Hot
            } else {
                Nil
            }
        }
        Hot => {
            if mouse_in_rect {
                if button_press.just_pressed() {
                    Active
                } else {
                    Hot
                }
            } else {
                Nil
            }
        }
        Active => {
            if button_press.just_released() {
                if mouse_in_rect {
                    clicked = true;
                    Hot
                } else {
                    Nil
                }
            } else {
                Active
            }
        }
    };
    (next, clicked)
}
