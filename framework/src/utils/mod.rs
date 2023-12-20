use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub mod watcher;

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
        let e = format!("assa");

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

/// Returns the file location of a .wgsl file with the same name as the .rs file, this was invoked in.
#[macro_export]
macro_rules! wgsl_file {
    () => {{
        // drop the rs, add wgsl
        let mut wgsl_file = format!("./{}", file!()).replace("./framework", ".");
        // replace because we want it to not be from the workspace parent folder.
        wgsl_file.pop();
        wgsl_file.pop();
        wgsl_file.push_str("wgsl");
        wgsl_file
    }};
}
