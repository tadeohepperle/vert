use std::{mem::MaybeUninit, os::raw::c_void};

use crate::{
    arena::{Arena, TypedArena},
    component::Component,
};

pub trait Collectable {
    type TraitObj: ?Sized + 'static;
}

pub struct DescribeMeS;

impl Collectable for DescribeMeS {
    type TraitObj = dyn DescribeMeT;
}

pub trait DescribeMeT {
    fn print(&self) -> String;
}

impl Collectable for RenderMeS {
    type TraitObj = dyn RenderMeT;
}

pub struct RenderMeS;
pub trait RenderMeT {
    fn render(&self);
}

/*

# What is the functionality we want to achieve?

Users should be able to define Components independent of the rest of the app.
To define a component we just define the data it holds as a struct.
these components can then be stored in arenas in the app.
The app does not need to know beforehand what components to store.

This way we can insert, remove, modify and iterate over Components in our app.

But we also need a way to define shared behavior among components.

The way the app runs is like this:
There is a main loop. In each iteration of the loop, the App System is called.
This is a Sequence of other systems. For example:
```
InputSystem -> UpdateSystem -> RenderSystem
```
They are executed in that order every frame. We can also parallelize multiple subsystems:
```
            -> CalculateLayout |
InputSystem -> DoPathFinding   | -> UpdateSystem -> RenderSystem.
            -> CollisionChecks |
```
But what do these Systems do?

They need a way to **collect** information from the components in the game state.
But the arenas the components are stored in are completely opaque from the outside.

That means: If we know what component we want, we can CRUD it and iterate over this component.

But: If we have no clue, what components are even in the arenas,
how can we iterate over all components that e.g. need to be rendered?

So from the systems perspective we need a way to access all component that
share some behavior. For example all components that need to be rendered.

There are two ways to do this:


### Function pointers

Maybe we store some sort of function pointer (like a vtable) in each arena.
Then a system can interate over all arenas and check where this function pointer is set and where not.
If the function pointer is set, the system can call the function pointer there with:
- each object in the arena.
- its own additional arguments.

However this sounds like it is lacking ergonomics.

### collectable traits
Traits would be nice for this, because we:
- can have multiple methods on one trait.
- provide default implementations
- ergonomics are likely better

Each trait we split up into two things: the actual trait T + a Zero sized Tag S.
e.g.
struct RenderS;
trait RenderT;

RenderS can be used to map to dyn RenderT.

So when a system now wants to iterate over all renderable components,
it can just call arenas.iter::<RenderS>() to get an Iterator<Item=&dyn RenderT>;



## We need mechanisms for:
- register TraitS structs into the app at startup.
- when registering a component C as an arena, we need to iterator over all TraitS registered.
    - we need to check if

for any given Arena, for any possible Trait:
-> return an uninit trait object.

so arena needs to implement MaybeCollectable









*/

// this trait should be auto-implemented by an #[is(RenderS)] macro.
trait IsCollectable<X: Collectable> {
    unsafe fn uninit_dyn_for_vtable() -> &'static X::TraitObj;
}

trait MaybeIsCollectable<X: Collectable> {
    unsafe fn maybe_uninit_dyn_for_vtable(&self) -> Option<&'static X::TraitObj>;
}

pub struct CollectableArena {
    arena: Arena,
}

impl CollectableArena {
    pub fn new<C: Component>(arena: TypedArena<C>) -> Self {
        let arena = arena.into_untyped();

        CollectableArena { arena }
    }
}
