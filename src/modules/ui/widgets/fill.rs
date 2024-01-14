use crate::modules::ui::{
    board::{Board, ContainerId, Id},
    Len,
};

use super::Widget;

pub fn h_fill(width: Len) -> HFill {
    HFill { width }
}

pub fn v_fill(height: Len) -> VFill {
    VFill { height }
}

pub struct HFill {
    width: Len,
}

pub struct VFill {
    height: Len,
}

impl Widget for HFill {
    type Response<'a> = ();

    fn add_to_board<'a>(
        self,
        board: &'a mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> Self::Response<'a> {
        board.add_div(id, parent).width = self.width;
    }
}

impl Widget for VFill {
    type Response<'a> = ();

    fn add_to_board<'a>(
        self,
        board: &'a mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> Self::Response<'a> {
        board.add_div(id, parent).height = self.height;
    }
}
