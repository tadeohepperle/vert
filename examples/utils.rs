use std::{
    any::{type_name, TypeId},
    mem::{align_of, size_of},
    ptr::slice_from_raw_parts,
};

fn show(slice: &[u8]) {
    let mut s: String = String::new();
    for e in slice.iter() {
        s.push_str(&format!("{:0>8}|", format!("{:b}", *e)));
    }

    println!("{s}");
}

fn show_layout<T>(t: T) {
    let size = size_of::<T>();
    let align = align_of::<T>();
    // let type_name = type_name::<T>();
    println!("size={size} align={align}");
    let ptr = &t as *const T as *const u8;
    let s = unsafe { &*slice_from_raw_parts(ptr, size) };
    show(s)
}

pub fn main() {
    trait A {}
    trait B {}

    dbg!(TypeId::of::<dyn A>());
    dbg!(TypeId::of::<&dyn A>());
    dbg!(TypeId::of::<dyn B>());
    dbg!(TypeId::of::<&dyn B>());

    // #[repr(C, u8, align(8))]
    // enum Entry<T> {
    //     Free { next_free: Option<usize> } = 0u8,
    //     Occupied { gen: u32, value: T } = 1u8,
    // }

    // #[derive(Debug, Clone, Copy)]
    // #[repr(C)]
    // struct S {
    //     i: u8,
    //     j: u32,
    // }

    // #[derive(Debug, Clone, Copy)]
    // #[repr(C, u8)]
    // enum V {
    //     A,
    //     B(u8),
    //     // C(u32),
    // }
    // let u: u8 = 8;
    // show_layout(&u as *const _ as *const usize);
    // show_layout(Entry::<u128>::Occupied {
    //     gen: u32::MAX,
    //     value: u128::MAX,
    // });
    // show_layout(Entry::<String>::Occupied {
    //     gen: u32::MAX,
    //     value: "Hello".into(),
    // });
    // show_layout(S {
    //     i: 0,
    //     j: 255 * 256 + 255,
    // });
    // let size = size_of::<V>();
    // let align = align_of::<V>();

    // dbg!(size, align);

    // #[repr(C, packed)]
    // struct Example {
    //     a: u8,
    //     b: u32,
    //     c: u16,
    // }

    // println!("Size of Example: {}", std::mem::size_of::<Example>());
    // println!("Alignment of Example: {}", std::mem::align_of::<Example>());
    // let array: [Example; 2] = [Example { a: 1, b: 2, c: 3 }, Example { a: 4, b: 5, c: 6 }];
    // let ptr = array.as_ptr();

    // // Calculate the offset between the first and second elements in the array
    // let offset = unsafe { (ptr.add(1) as usize) - (ptr as usize) };
    // println!("Offset between elements: {}", offset);
}
