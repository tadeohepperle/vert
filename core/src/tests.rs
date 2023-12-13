use crate::{arenas::Arenas, component::Component, prelude::*};

#[test]
fn spawning_components() {
    reflect!(C: );
    impl Component for C {}
    pub struct C {
        hello: String,
        n: u8,
    }

    let mut arenas = Arenas::new();
    let component = C {
        hello: format!("Hello"),
        n: 3,
    };
    arenas.insert(component);

    for i in 0..1000 {
        let component = C {
            hello: format!("i: {i}"),
            n: (i % 200) as u8,
        };
        arenas.insert(component);
    }
    arenas.free_arena::<C>();
}
