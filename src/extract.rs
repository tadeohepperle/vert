//! There is one issue we need to solve:
//!
//! We want to get stuff out of the world a la Bevy `FromWorld`
//! because we do not want to pass the entire world around all the time.
//! Passing around the entire world all the time would also lead to borrowing issues.
//!
//! We need an Extract<Part> Trait that is implemented for the World state
//! to get any part out of the world state.
//!
//! When something is gotten out of the world state, we need to keep track of it.
//! Can we do this with a sort of state machine pattern?
//! Can we use const generics to verify at compile time that everything is good?
//! Everything
//!
//! Each System should be a function whose arguments are all Extract<Part> of World.
//!
//!
//!

// pub trait Extract<'a, Part<'a>> {
//     fn extract(&'a) -> Part{

//     }

//     fn access() -> &'static[TypeId]
// }

// impl<A,B> Extract<(A,B)> for i
