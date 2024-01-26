use crate::{
    elements::Color,
    modules::ui::{
        board::{
            Align, Axis, Board, BorderRadius, ContainerId, HotActive, Id, Len, MainAlign, Text,
        },
        widgets::next_hot_active,
        FontSize,
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

    fn add_to_board(
        self,
        board: &mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> Self::Response<'_> {
        const SLIDER_CONTAINER_WIDTH: f64 = 120.0;
        const SLIDER_WIDTH: f64 = 100.0;
        const KNOB_WIDTH: f64 = 16.0;

        let knob_id = id + 2;
        let left_mouse_button = board.input().mouse_buttons.left();

        let knob_hot_active = board.hot_active(knob_id);

        let mut parent = board.add_div(id + 237, parent);
        parent.axis = Axis::Y;
        parent.width(Len::px(SLIDER_CONTAINER_WIDTH));
        parent.cross_align = Align::Center;
        let parent = Some(parent.id);

        let mut slider = board.add_div(id, parent);
        slider.width(Len::px(SLIDER_WIDTH));
        slider.height(Len::px(20.0));
        slider.axis = Axis::X;
        slider.main_align = MainAlign::Start;
        slider.cross_align = Align::Center;

        let slider_hovered = slider.mouse_in_rect();
        let slider = Some(slider.id);

        // slider bar

        let mut bar = board.add_div(id + 1, slider);
        bar.width(Len::PARENT);
        bar.height(Len::px(8.0));
        bar.color = Color::GREY;
        bar.color = Color::from_hex("#32a852");
        bar.border_radius = BorderRadius::all(4.0);
        bar.border_thickness = 1.0;

        const PX_TOTAL_RANGE: f64 = SLIDER_WIDTH - KNOB_WIDTH;
        let px_delta = board.input().cursor_delta.x;

        // knob
        let mut knob = board.add_div(knob_id, slider);
        knob.width(Len::px(KNOB_WIDTH));
        knob.height(Len::px(KNOB_WIDTH));
        knob.absolute = true;
        knob.border_radius = BorderRadius::all(KNOB_WIDTH as f32 / 2.0);
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

        knob.color = match knob_next_hot_active {
            HotActive::Nil => Color::BLACK,
            HotActive::Hot => Color::from_hex("#4d528a"),
            HotActive::Active => Color::from_hex("#4f5dff"),
        };
        if knob_next_hot_active == HotActive::Nil && !slider_hovered {
            knob.border_color = Color::from_hex("#444455");
            knob.border_thickness = 1.0;
        } else {
            knob.border_color = Color::RED;
            knob.border_thickness = 2.0;
        };

        // compute the desired knob position
        let fraction = ((*self.value - self.min) / value_range) as f64;
        knob.offset_x = Len::px(fraction * PX_TOTAL_RANGE);
        board.set_hot_active(knob_id, knob_next_hot_active);

        let mut text_div =
            board.add_text_div(Text::new(format!("{:.2}", self.value)), id + 4, parent);
        text_div.width(Len::PARENT);
        text_div.main_align = MainAlign::Center;
    }
}
