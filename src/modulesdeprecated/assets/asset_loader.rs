use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

use super::{fetchable_asset::AssetSource, AssetT};

type CacheKey = (AssetSource, TypeId);

/// cheap to clone
#[derive(Debug, Clone)]
pub struct AssetLoader {
    inner: Arc<Mutex<AssetServerInner>>,
}

#[derive(Debug, Clone, Default)]
struct AssetServerInner {
    current_key: u32,
    assets: HashMap<u32, StoredErasedAsset>,
}

/// Stores an Arc<T> or a Weak<T> for any type T, via type punning.
#[derive(Debug, Clone, Copy)]
struct StoredErasedAsset {
    type_id: TypeId,
    is_weak: bool,
    /// depending on `is_weak`, this is an Arc<T> or a Weak<T>.
    arc_inner_ptr: *const (),
}
unsafe impl Send for StoredErasedAsset {}
unsafe impl Sync for StoredErasedAsset {}

impl StoredErasedAsset {
    /// if !self.is_weak, the `arc_inner_ptr` should always refer to an Arc instead of a Weak and the function will never return None.
    /// None is only returned if the `arc_inner_ptr` belongs to a Weak where all references are already dropped.
    fn get_arc<T: AssetT>(&self) -> Option<Arc<T>> {
        debug_assert_eq!(self.type_id, TypeId::of::<T>());
        if self.is_weak {
            let weak: Weak<T> = unsafe { std::mem::transmute(self.arc_inner_ptr) };
            // this returns None if the value was already dropped:
            weak.upgrade()
        } else {
            let arc: Arc<T> = unsafe { std::mem::transmute(self.arc_inner_ptr) };
            Some(arc)
        }
    }
}

impl AssetLoader {
    pub fn new() -> Self {
        AssetLoader {
            inner: Arc::new(Mutex::new(Default::default())),
        }
    }

    // right now not very useful:

    /// if `store_forever` is true, the asset is stored in the AssetServer forever until the end of the program.
    /// This is achieved by storing an Arc in the AssetServer instead of a Weak.
    pub fn store<T: AssetT>(&self, asset: T, store_forever: bool) -> Arc<T> {
        let mut inner = self.inner.lock().expect("poison");
        // determine the key and create the arc:
        let key = inner.current_key;
        inner.current_key += 1;
        let asset_arc = Arc::new(asset);

        // if `store_forever`, we clone this arc before storing it, so the reference count will stay >= 1 forever.
        // otherwise we
        let arc_inner_ptr: *const () = if store_forever {
            let strong = asset_arc.clone();
            unsafe { std::mem::transmute(strong) }
        } else {
            let weak = Arc::downgrade(&asset_arc);
            unsafe { std::mem::transmute(weak) }
        };
        let stored_asset = StoredErasedAsset {
            type_id: TypeId::of::<T>(),
            is_weak: !store_forever,
            arc_inner_ptr,
        };

        // insert into hashmap
        debug_assert!(inner.assets.get(&key).is_none());
        inner.assets.insert(key, stored_asset);

        // return the arc:
        asset_arc
    }

    // pub fn get<T: AssetT>(&asset_source);
}
