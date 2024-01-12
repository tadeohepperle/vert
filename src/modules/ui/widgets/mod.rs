use crate::modules::input::PressState;

use super::board::{Board, ContainerId, HotActive, Id};

pub mod button;
pub use button::Button;

pub trait Widget {
    /// lifetime to allow mutable entries inserted into the hashmap be returned.
    /// allows editing of e.g. a widgets style based on the response in the same frame.
    type Response<'a>;

    /// this is an immediate mode API, so this function just adds some divs containing other divs or text to the Board.
    fn add_to_board<'a>(
        self,
        board: &'a mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> Self::Response<'a>;
}

/// Shout out to Casey Muratori, our lord and savior. (See this Video as well for an exmplanation: https://www.youtube.com/watch?v=geZwWo-qNR4)
pub fn next_hot_active_and_clicked(
    hot_active: HotActive,
    mouse_in_rect: bool,
    button_press: PressState,
) -> HotActive {
    use HotActive::*;
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
                    Hot
                } else {
                    Nil
                }
            } else {
                Active
            }
        }
    };
    next
}
