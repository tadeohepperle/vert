#![feature(alloc_layout_extra, unsize)]

// an animal trait with the methods make_sound and name
// and a struct Dog and a struct Cat that implement the trait

use std::{os::raw::c_void, usize};

trait Pet {
    fn sound(&self) -> String;
    fn name(&self) -> String;
}

struct Dog {
    _age: u8,
    name: String,
}

impl Dog {
    fn new(name: impl Into<String>) -> Self {
        Self {
            _age: 0,
            name: name.into(),
        }
    }
}

struct Cat {
    _life: u8,
    _age: u8,
    name: String,
}

impl Cat {
    fn new(name: impl Into<String>) -> Self {
        Self {
            _life: 9,
            _age: 0,
            name: name.into(),
        }
    }
}

impl Pet for Dog {
    fn sound(&self) -> String {
        "Woof!".to_string()
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}

impl Pet for Cat {
    fn sound(&self) -> String {
        "Meow!".to_string()
    }
    fn name(&self) -> String {
        self.name.clone()
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct PetVtable {
    drop: fn(*mut c_void),
    size: usize,
    align: usize,
    sound: fn(*const c_void) -> String,
    name: fn(*const c_void) -> String,
}

fn bark(_this: *const c_void) -> String {
    "Woof!".to_string()
}

const POINTER_SIZE: usize = std::mem::size_of::<*const c_void>();
fn main() {
    unsafe {
        // create boxed instances of Dog and Cat
        // but use the trait Animal as the type
        let doggo: Box<dyn Pet> = Box::new(Dog::new("Doggo"));
        let mut kitty: Box<dyn Pet> = Box::new(Cat::new("Kitty"));

        let addr_of_data_ptr = &mut kitty as *mut _ as *mut c_void as usize;
        let addr_of_pointer_to_vtable = addr_of_data_ptr + POINTER_SIZE;
        let ptr_to_ptr_to_vtable = addr_of_pointer_to_vtable as *mut *const PetVtable;
        let mut new_vtable = **ptr_to_ptr_to_vtable;
        new_vtable.sound = bark;
        *ptr_to_ptr_to_vtable = &new_vtable;

        greet_pet(doggo);
        greet_pet(kitty);
    }
}

fn greet_pet(pet: Box<dyn Pet>) {
    println!("You: Hello, {}!", pet.name());
    println!("{}: {}\n", pet.name(), pet.sound());
}
