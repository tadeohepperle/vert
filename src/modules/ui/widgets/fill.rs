use crate::modules::ui::{
    board::{Board, ContainerId, Id},
    DivProps, Len,
};

use super::Widget;

pub fn h_fill(len: Len) -> HFill {
    HFill { len }
}

pub fn v_fill(len: Len) -> VFill {
    VFill { len }
}

pub struct HFill {
    len: Len,
}

pub struct VFill {
    len: Len,
}

impl Widget for HFill {
    type Response<'a> = ();

    fn add_to_board<'a>(
        self,
        board: &'a mut Board,
        id: Id,
        parent: Option<ContainerId>,
    ) -> Self::Response<'a> {
        board.add_div(
            DivProps {
                width: self.len,
                ..Default::default()
            },
            id,
            parent,
        );
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
        board.add_div(
            DivProps {
                height: self.len,
                ..Default::default()
            },
            id,
            parent,
        );
    }
}
