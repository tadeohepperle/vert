use std::borrow::Cow;

use fontdue::Font;

use crate::{
    elements::Color,
    modules::{
        arenas::Key,
        ui::{
            board::{
                Board, BorderRadius, ContainerId, DivProps, DivStyle, Id, Len, MainAlign, Text,
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
    pub font: Option<Key<Font>>,
}

pub struct ButtonResponse {
    clicked: bool,
    hovered: bool,
}

impl Widget for Button {
    type Response<'a> = ButtonResponse;

    fn add_to_board<'a>(
        self,
        board: &'a mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> ButtonResponse {
        let mut div_res = board.add_text_div(
            DivProps {
                width: Len::HugContent,
                height: Len::HugContent,
                ..Default::default()
            },
            DivStyle {
                color: self.color,
                border_color: Color::BLACK,
                border_radius: BorderRadius::all(8.0),
                border_thickness: 4.0,
                border_softness: 3.0,
                z_bias: 0,
            },
            Text {
                color: self.text_color,
                string: self.text,
                font: self.font,
                size: FontSize(24),
            },
            id,
            parent,
        );

        if div_res.is_hovered() {
            div_res.style_mut().color = self.hover_color;
        }

        ButtonResponse {
            clicked: false,
            hovered: false,
        }
    }
}
