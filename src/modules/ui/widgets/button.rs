use std::borrow::Cow;

use fontdue::Font;

use crate::{
    elements::Color,
    modules::{
        arenas::Key,
        ui::{
            board::{
                Align, Board, BorderRadius, ContainerId, DivProps, DivStyle, HotActive, Id, Len,
                MainAlign, Text,
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
            color: Color::DARKGREY,
            hover_color: Color::WHITE,
            font: None,
            click_color: Color::LIGHTBLUE,
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
        let mouse_in_rect = board
            .add_text_div(
                DivProps {
                    width: Len::Px(200.0),
                    height: Len::ChildrenFraction(1.5),
                    main_align: MainAlign::Center,
                    cross_align: Align::Center,
                    ..Default::default()
                },
                DivStyle {
                    color: match hot_active {
                        HotActive::None => self.color,
                        HotActive::Hot => self.hover_color,
                        HotActive::Active => self.click_color,
                    },
                    border_color: Color::BLACK,
                    border_radius: BorderRadius::all(16.0),
                    border_thickness: 10.0,
                    border_softness: 16.0,
                    z_bias: 0,
                    offset_x: Len::ZERO,
                    offset_y: Len::ZERO,
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
            )
            .mouse_in_rect();

        /*
            Shout out to Casey Muratori, our lord and savior. (See this Video as well for an exmplanation: https://www.youtube.com/watch?v=geZwWo-qNR4)
        */

        let mut clicked = false;

        match hot_active {
            HotActive::None => {
                if mouse_in_rect {
                    board.set_hot_active(id, HotActive::Hot);
                }
            }
            HotActive::Hot => {
                if mouse_in_rect {
                    if board.input().mouse_buttons.left().just_pressed() {
                        board.set_hot_active(id, HotActive::Active);
                    }
                } else {
                    board.set_hot_active(id, HotActive::None);
                }
            }
            HotActive::Active => {
                if board.input().mouse_buttons.left().just_released() {
                    if mouse_in_rect {
                        clicked = true;
                        board.set_hot_active(id, HotActive::Hot);
                    } else {
                        board.set_hot_active(id, HotActive::None);
                    }
                }
            }
        }

        ButtonResponse { clicked }
    }
}
