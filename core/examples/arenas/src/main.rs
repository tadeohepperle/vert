use vert_core::{
    arenas::{arena, Arenas},
    prelude::*,
};

reflect!(Mesh: Render);
impl Component for Mesh {}
#[derive(Debug, Clone)]
struct Mesh {
    verts: Vec<f32>,
    indices: Vec<u32>,
}

reflect!(Vert: Render);
impl Component for Vert {}
#[derive(Debug, Clone)]
struct Vert {
    b: Box<f32>,
}

impl Render for Mesh {
    fn pipeline(&self) -> &'static str {
        println!("Mesh with verts: {}", self.verts.len());
        "mesh"
    }

    fn modify(&mut self, factor: f32) {
        self.verts.push(factor);
    }
}

impl Render for Vert {
    fn pipeline(&self) -> &'static str {
        println!("vert with float: {}", self.b);
        "vert"
    }

    fn modify(&mut self, factor: f32) {
        *self.b *= factor;
    }
}

reflect!(Render);
pub trait Render {
    fn pipeline(&self) -> &'static str;
    fn modify(&mut self, factor: f32);
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(main2());
}

async fn main2() {
    let mut arenas = Arenas::new();

    for i in 0..10 {
        arenas.insert(Mesh {
            verts: vec![4.5, 2.3, 4.5],
            indices: vec![i, 4, 3, 5],
        });
    }

    for i in 0..7 {
        arenas.insert(Vert {
            b: Box::new(i as f32),
        });
    }

    for (i, m) in arenas.iter::<Mesh>() {
        dbg!(m);
    }

    println!("--------------------------------");

    for (i, m) in arenas.iter_mut::<Mesh>() {
        for i in m.indices.iter_mut() {
            *i += 4;
        }
    }

    for (i, m) in arenas.iter::<Mesh>() {
        dbg!(m);
    }

    println!("--------------------------------");

    for (i, rend) in arenas.iter_component_traits_mut::<dyn Render>().enumerate() {
        rend.modify(3.0);
        println!("modified {i}");
    }

    for (i, rend) in arenas.iter_component_traits::<dyn Render>().enumerate() {
        println!("   {i}: {}", rend.pipeline())
    }

    arenas.free_arena::<Mesh>();
    arenas.free_arena::<Vert>();
}
