#![feature(generic_const_exprs, min_specialization)]
#![feature(ptr_metadata)]
use std::{
    any::TypeId,
    collections::HashMap,
    default,
    mem::MaybeUninit,
    ptr::{metadata, DynMetadata, Pointee},
};

struct Renderer {
    queue: usize,
    name: String,
}

impl Renderer {
    fn submit_triag(&mut self, a: f32, b: f32, c: f32) {
        println!("render triag: {a} {b} {c}");
    }
}

trait Component: Default {}

trait Collectable {
    type Dyn: ?Sized;
}

struct SaveS;
impl Collectable for SaveS {
    type Dyn = dyn SaveT;
}

trait SaveT {
    fn save(&mut self);
}

struct RenderS;
impl Collectable for RenderS {
    type Dyn = dyn RenderT;
}

trait RenderT {
    fn render(&self, renderer: &mut Renderer);
}

#[derive(Debug, Clone, Default)]
struct Circle {
    rad: f32,
}

#[derive(Debug, Clone, Default)]
struct Rect {
    a: f32,
    b: f32,
}

impl Component for Circle {}
impl Component for Rect {}

impl RenderT for Rect {
    fn render(&self, renderer: &mut Renderer) {
        renderer.submit_triag(self.a, self.b, 0.0);
    }
}

impl RenderT for Circle {
    fn render(&self, renderer: &mut Renderer) {
        renderer.submit_triag(2.0, 2.0, self.rad);
    }
}

impl SaveT for Rect {
    fn save(&mut self) {
        println!("Saved the rect")
    }
}

/*
Lets say we have a derive macro that create a lazystatic
typemap at compile time.


*/

// pretty much created at compile time:

struct CircleTraitMap {
    save: Box<dyn SaveT>,
}

struct RectTraitMap {
    save: Box<dyn SaveT>,
    render: Box<dyn RenderT>,
}

// impl TraitMapT for Rect {
//     // fn get_dyn_obj<X: Collectable>(&self) -> Option<&'static X::Dyn> {
//     //     None
//     // }

//     fn get_dyn_obj<X: Collectable = SaveS>(&self) -> Option<&'static <SaveS as Collectable>::Dyn> {
//         None
//     }
// }

// // impl TraitMapT for BoxTraitMap {
// //     fn get_dyn_obj<X: Collectable>(&self) -> Option<&'static X::Dyn> {
// //         todo!()
// //     }
// // }

// trait IntoTraitMap {
//     type TraitMapType: TraitMapT;
//     fn trait_map() -> &'static Self::TraitMapType;
// }

// trait TraitMapT {
//     fn get_dyn_obj<X: Collectable>(&self) -> Option<&'static X::Dyn> {
//         None
//     }
// }

// idea: create a zeroed concrete instance of C.
// make it into a trait object for any trait that is registered.
// store this trait object in the arena, because it contains the full vtable.
// when iterating over the arena, make a copy of this trait object but switch out
// the data section pointing at some memory inside of the arena.
// still we need to know at runtime, if a struct implements a trait or not.
// e.g. if it can be made into a trait object or not...

// fn create_dyn_box<X: Collectable, C: Component>() -> Option<Box<X::Dyn>> {
//     let uninit_component: C = unsafe { MaybeUninit::uninit().assume_init() };
//     Some((Box::new(uninit_component) as Box<X::Dyn>))
// }

// fn register_trait<X: Collectable>() {
//     let unit_box = Box::new(());

//     *unit_box as &

//     let e: *const u8 = std::ptr::null();

//     let b: *mut X::Dyn = e as *mut X::Dyn;
// }

fn main() {
    let c = Circle { rad: 0.3 };
    let r = Rect { a: 21.2, b: 0.2 };

    // let d = GetDynForCollectible::<RenderS>::get_dyn(&c);

    dbg!(get_dyn::<RenderS>(&c).is_some());
    dbg!(get_dyn::<RenderS>(&r).is_some());
    dbg!(get_dyn::<SaveS>(&c).is_some());
    dbg!(get_dyn::<SaveS>(&r).is_some());
    // let circle_box: Box<Circle> = Box::new(Circle { rad: 0.2 });
    // let render_box: Box<dyn RenderT> = Box::new(Circle { rad: 0.2 });

    // let render_box2: Box<dyn RenderT> = circle_box;
    // let render_box2: Box<dyn SaveT> = todo!();

    // // let e: *const u8 = std::ptr::null();
    // // let circle = Circle { rad: 3.0 };

    // // let renderable: Box<dyn RenderT> = Box::new(circle);
    // // let e: <dyn RenderT as Pointee>::Metadata = metadata(&*renderable);
    // // let e: DynMetadata<dyn RenderT> = e;
    // // dbg!(e);
}

fn get_dyn<X: Collectable>(c: &impl Component) -> Option<&'static X::Dyn> {
    GetDynForCollectible::<X>::get_dyn(c)
}

trait GetDynForCollectible<X: Collectable> {
    fn get_dyn(&self) -> Option<&'static X::Dyn>;
}

impl<C, X> GetDynForCollectible<X> for C
where
    C: Component,
    X: Collectable,
{
    default fn get_dyn(&self) -> Option<&'static <X as Collectable>::Dyn> {
        None
    }
}

// so here we still need a macro like this:
// #[is(RenderS, SaveS)]

impl GetDynForCollectible<RenderS> for Circle {
    fn get_dyn(&self) -> Option<&'static dyn RenderT> {
        const CIRCLE: Circle = Circle { rad: 2.0 };
        Some(&CIRCLE as &dyn RenderT)
    }
}

trait TypePrinter: 'static {
    fn print(&self);
}

impl<G: 'static> TypePrinter for G {
    default fn print(&self) {
        let ty = TypeId::of::<Self>();
        let ty_name = std::any::type_name::<Self>();
        println!("I am {ty_name} with id {ty:?}");
    }
}

impl TypePrinter for Circle {
    fn print(&self) {
        println!("I am just a circle...");
    }
}

// fn printme<G: 'static>() {
//     let ty = TypeId::of::<G>();
//     let ty_name = std::any::type_name::<G>();
//     println!("I am {ty_name} with id {ty:?}");
// }
