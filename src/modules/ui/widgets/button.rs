use std::borrow::Cow;

use fontdue::Font;

use crate::{
    elements::Color,
    modules::ui::{
        board::{
            Align, Board, BorderRadius, ContainerId,
            HotActive::{self, *},
            Id, Len, MainAlign, Text,
        },
        font_cache::FontSize,
        Padding,
    },
    Ptr,
};

use super::{next_hot_active, Widget};

pub struct Button {
    pub text: Cow<'static, str>,
    pub text_color: Color,
    pub color: Color,
    pub hover_color: Color,
    pub click_color: Color,
    pub font: Option<Ptr<Font>>,
}

impl Default for Button {
    fn default() -> Self {
        Button {
            text: "Button 1".into(),
            text_color: Color::BLACK,
            color: Color::u8_srgb(77, 130, 176),
            hover_color: Color::u8_srgb(151, 174, 194),
            font: None,
            click_color: Color::u8_srgb(188, 115, 201),
        }
    }
}

pub struct ButtonResponse {
    pub clicked: bool,
}

impl Widget for Button {
    type Response<'a> = ButtonResponse;

    fn add_to_board(
        self,
        board: &mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> ButtonResponse {
        let hot_active = board.hot_active(id);
        let left_button = board.input().mouse_buttons.left();
        let mut btn = board.add_text_div(
            {
                let mut text = Text::default();
                text.font = self.font;
                text.sections[0].color = self.text_color;
                text.sections[0].size = FontSize(24);
                text
            },
            id,
            parent,
        );

        let mouse_in_rect = btn.mouse_in_rect();

        btn.width(Len::px(200.0));
        // add padding
        btn.main_align = MainAlign::Start;
        btn.cross_align = Align::Center;

        btn.color = self.color;
        btn.border_color = Color::BLACK;
        btn.border_radius = BorderRadius::all(16.0);
        btn.border_thickness = 10.0;
        btn.border_softness = 16.0;
        btn.padding = Padding::new().top(Len::px(8.0)).bottom(Len::px(16.0));

        let next_hot_active = next_hot_active(hot_active, mouse_in_rect, left_button);
        let clicked = hot_active == Active && next_hot_active == Hot;

        // we can now update the style immediately. Using the hot_active only on insertion instead of next_hot_active
        // would always be 1 frame behind. Just add a 150ms of workload on each frame (7fps) and you will feel the different.
        btn.color = match next_hot_active {
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
