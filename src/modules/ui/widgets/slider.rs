use crate::{
    elements::Color,
    modules::ui::{
        board::{
            Align, Axis, Board, BorderRadius, ContainerId, DivProps, DivStyle, HotActive, Id, Len,
            MainAlign, Text,
        },
        widgets::next_hot_active,
    },
};

use super::Widget;

/// This is a very rudimentary slider for float values. Nothing fancy. Mainly to show how things can be done in Immediate Mode UI.
/// No Customization options here. Just copy it and make your own adjustments.
pub struct Slider<'v> {
    value: &'v mut f32,
    min: f32,
    max: f32,
}

impl<'v> Slider<'v> {
    pub fn new(value: &'v mut f32, min: f32, max: f32) -> Self {
        Self { value, min, max }
    }
}

impl<'v> Widget for Slider<'v> {
    type Response<'a> = ();

    fn add_to_board<'a>(
        self,
        board: &'a mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> Self::Response<'a> {
        const SLIDER_CONTAINER_WIDTH: f64 = 120.0;
        const SLIDER_WIDTH: f64 = 100.0;
        const KNOB_WIDTH: f64 = 16.0;

        let knob_id = id + 2;
        let left_mouse_button = board.input().mouse_buttons.left();

        let knob_hot_active = board.hot_active(knob_id);

        let parent = board
            .add_non_text_div(
                DivProps {
                    axis: Axis::Y,
                    width: Len::Px(SLIDER_CONTAINER_WIDTH),
                    cross_align: Align::Center,
                    ..Default::default()
                },
                id + 237,
                parent,
            )
            .id;

        let slider = board.add_non_text_div(
            DivProps {
                width: Len::Px(SLIDER_WIDTH),
                height: Len::Px(20.0),
                axis: Axis::X,
                main_align: MainAlign::Start,
                cross_align: Align::Center,
                absolute: false,
            },
            id,
            Some(parent),
        );

        let slider_hovered = slider.mouse_in_rect();
        let slider = slider.id;

        // slider bar

        let mut d = board.add_non_text_div(
            DivProps {
                width: Len::PARENT,
                height: Len::Px(8.0),
                ..Default::default()
            },
            id + 1,
            Some(slider),
        );
        let style = d.style();
        style.color = Color::GREY;
        style.color = Color::from_hex("#32a852");
        style.border_radius = BorderRadius::all(4.0);
        style.border_thickness = 1.0;

        const PX_TOTAL_RANGE: f64 = SLIDER_WIDTH - KNOB_WIDTH;
        let px_delta = board.input().cursor_delta.x;

        // knob
        let mut knob = board.add_non_text_div(
            DivProps {
                width: Len::Px(KNOB_WIDTH),
                height: Len::Px(KNOB_WIDTH),
                absolute: true,
                ..Default::default()
            },
            knob_id,
            Some(slider),
        );
        knob.style().border_radius = BorderRadius::all(KNOB_WIDTH as f32 / 2.0);

        let knob_next_hot_active =
            next_hot_active(knob_hot_active, knob.mouse_in_rect(), left_mouse_button);

        // Formula:
        // px_delta / value_change = px_total_range / value_range
        // -> value_change = (px_delta * value_range) / px_total_range
        let value_range = self.max - self.min;
        if knob_next_hot_active == HotActive::Active {
            let value_change = (px_delta * value_range) / PX_TOTAL_RANGE as f32;
            *self.value += value_change;
            *self.value = self.value.clamp(self.min, self.max);
        }

        let style = knob.style();
        style.color = match knob_next_hot_active {
            HotActive::Nil => Color::BLACK,
            HotActive::Hot => Color::from_hex("#4d528a"),
            HotActive::Active => Color::from_hex("#4f5dff"),
        };
        if knob_next_hot_active == HotActive::Nil && !slider_hovered {
            style.border_color = Color::from_hex("#444455");
            style.border_thickness = 1.0;
        } else {
            style.border_color = Color::from_hex("#ffffff");
            style.border_thickness = 2.0;
        };

        // compute the desired knob position
        let fraction = ((*self.value - self.min) / value_range) as f64;
        style.offset_x = Len::Px(fraction * PX_TOTAL_RANGE);
        board.set_hot_active(knob_id, knob_next_hot_active);

        board.add_text_div(
            DivProps {
                width: Len::PARENT,
                height: Len::CONTENT,
                main_align: MainAlign::Center,
                ..Default::default()
            },
            Text {
                color: Color::DARKGREY,
                string: format!("{:.2}", self.value).into(),
                font: None,
                size: 20.into(),
                ..Default::default()
            },
            id + 4,
            Some(parent),
        );
    }
}
