use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug, Clone)]
pub struct Writer<T> {
    lock: Arc<RwLock<T>>,
}

impl<T> Writer<T> {
    pub fn new(value: T) -> Writer<T> {
        Self {
            lock: Arc::new(RwLock::new(value)),
        }
    }

    pub fn get(&self) -> RwLockReadGuard<'_, T> {
        self.lock.read().expect("poisons not expected")
    }

    pub fn get_mut(&self) -> RwLockWriteGuard<'_, T> {
        self.lock.write().expect("poisons not expected")
    }

    pub fn reader(&self) -> Reader<T> {
        Reader {
            __lock: self.lock.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reader<T> {
    __lock: Arc<RwLock<T>>,
}

impl<T> Reader<T> {
    pub fn get(&self) -> RwLockReadGuard<'_, T> {
        self.__lock.read().expect("poisons not expected")
    }
}
