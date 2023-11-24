use std::any::Any;

use smallvec::SmallVec;

pub struct Events {
    this_frame: SmallVec<[Box<dyn Any>; 10]>,
}

impl Events {
    pub fn new() -> Self {
        Events {
            this_frame: Default::default(),
        }
    }

    pub fn write(&mut self, event: impl Any) {
        self.this_frame.push(Box::new(event));
    }

    pub fn read(&self) -> impl Iterator<Item = &Box<dyn Any>> {
        self.this_frame.iter()
    }

    pub fn read_t<T: 'static>(&self) -> impl Iterator<Item = &T> {
        self.this_frame.iter().filter_map(|e| e.downcast_ref::<T>())
    }

    pub fn clear(&mut self) {
        self.this_frame.clear();
    }
}
