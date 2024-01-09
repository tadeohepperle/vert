use super::board::{Board, ContainerId, Id};

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
