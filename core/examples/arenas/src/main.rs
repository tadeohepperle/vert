use std::borrow::Cow;

use glam::{Quat, Vec3};
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

    // for i in 0..10 {
    //     arenas.insert(Mesh {
    //         verts: vec![4.5, 2.3, 4.5],
    //         indices: vec![i, 4, 3, 5],
    //     });
    // }

    // for i in 0..7 {
    //     arenas.insert(Vert {
    //         b: Box::new(i as f32),
    //     });
    // }

    // for (i, m) in arenas.iter::<Mesh>() {
    //     dbg!(m);
    // }

    // println!("--------------------------------");

    // for (i, m) in arenas.iter_mut::<Mesh>() {
    //     for i in m.indices.iter_mut() {
    //         *i += 4;
    //     }
    // }

    // for (i, m) in arenas.iter::<Mesh>() {
    //     dbg!(m);
    // }

    // println!("--------------------------------");

    // for (i, rend) in arenas.iter_component_traits_mut::<dyn Render>().enumerate() {
    //     rend.modify(3.0);
    //     println!("modified {i}");
    // }

    // for (i, rend) in arenas.iter_component_traits::<dyn Render>().enumerate() {
    //     println!("   {i}: {}", rend.pipeline())
    // }

    // arenas.free_arena::<Mesh>();
    // arenas.free_arena::<Vert>();

    let single_color_mesh = SingleColorMesh {
        inner: ColorMeshObj {
            mesh: ColorMesh {
                name: "hello".into(),
                mesh_data: ColorMeshData {
                    verts: vec![],
                    indices: vec![],
                },
                vertex_buffer: 98,
                index_buffer: 98,
            },
            transform: InstanceBuffer {
                values: vec![Transform::default()],
                raw_values: vec![33],
                buffer: 23,
                name: Some("Lle".to_string().into()),
                changed: false,
            },
        },
    };

    arenas.insert(single_color_mesh);

    println!("Yay");
    for m in arenas.iter::<SingleColorMesh>() {
        dbg!(&m.1.inner.mesh.name);
    }

    arenas.free_arena::<SingleColorMesh>();
}

reflect!(SingleColorMesh :);
impl Component for SingleColorMesh {}
pub struct SingleColorMesh {
    inner: ColorMeshObj,
}

pub struct InstanceBuffer<U> {
    values: Vec<U>,
    raw_values: Vec<u32>,
    buffer: u32,
    pub name: Option<Cow<'static, str>>,
    changed: bool,
}

pub struct ColorMeshObj {
    mesh: ColorMesh,
    transform: InstanceBuffer<Transform>,
}
pub struct ColorMesh {
    name: String,
    mesh_data: ColorMeshData,
    pub vertex_buffer: u32,
    pub index_buffer: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
pub struct ColorMeshData {
    pub verts: Vec<Vertex>,
    pub indices: Vec<u32>,
}
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 4],
}
