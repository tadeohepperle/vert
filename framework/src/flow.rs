use std::{
    fmt::Display,
    ops::{ControlFlow, FromResidual, Try},
};

use Flow::*;

pub enum Flow {
    Exit,
    Continue,
}

impl FromResidual<Flow> for Flow {
    fn from_residual(_: Flow) -> Self {
        Self::Exit
    }
}

impl<T, E: Display> FromResidual<Result<T, E>> for Flow {
    fn from_residual(res: Result<T, E>) -> Self {
        match res {
            Ok(_) => Continue,
            Err(err) => {
                eprintln!("Error, exit Flow: {err}");
                Exit
            }
        }
    }
}

impl Try for Flow {
    type Output = ();

    type Residual = Self;

    fn from_output(_: Self::Output) -> Self {
        Continue
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Exit => ControlFlow::Break(Self::Exit),
            Continue => ControlFlow::Continue(()),
        }
    }
}
