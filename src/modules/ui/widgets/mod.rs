use crate::{modules::input::PressState, Ptr};

use super::{
    board::{Board, DivId, HotActive, Id, Response},
    Len, Span, Text, TextSection,
};

mod button;
pub use button::Button;

mod fill;
pub use fill::{h_fill, v_fill};

mod slider;
use fontdue::Font;
pub use slider::Slider;
use smallvec::smallvec;

pub trait Widget {
    /// lifetime to allow mutable entries inserted into the hashmap be returned.
    /// allows editing of e.g. a widgets style based on the response in the same frame.
    type Response<'a>;

    /// this is an immediate mode API, so this function just adds some divs containing other divs or text to the Board.
    fn add_to_board(self, board: &mut Board, id: Id, parent: Option<DivId>) -> Self::Response<'_>;
}

/// Shout out to Casey Muratori, our lord and savior. (See this Video as well for an exmplanation: https://www.youtube.com/watch?v=geZwWo-qNR4)
pub fn next_hot_active(
    hot_active: HotActive,
    mouse_in_rect: bool,
    button_press: PressState,
) -> HotActive {
    use HotActive::*;

    match hot_active {
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
                    Hot
                } else {
                    Nil
                }
            } else {
                Active
            }
        }
    }
}

impl Widget for (TextSection, Ptr<Font>) {
    type Response<'a> = Response<'a, DivId>;

    fn add_to_board(self, board: &mut Board, id: Id, parent: Option<DivId>) -> Self::Response<'_> {
        board.add_text_div(
            Text {
                spans: smallvec![Span::Text(self.0)],
                font: Some(self.1),
                offset_x: Len::ZERO,
                offset_y: Len::ZERO,
                line_height: 1.0,
            },
            id,
            parent,
        )
    }
}
