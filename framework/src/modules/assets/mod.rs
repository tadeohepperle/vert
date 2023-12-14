use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

use self::fetchable_asset::AssetSource;

pub mod fetchable_asset;

type CacheKey = (AssetSource, TypeId);

/// cheap to clone
#[derive(Debug, Clone)]
pub struct AssetServer {
    assets: Arc<Mutex<HashMap<CacheKey, StoredErasedAsset>>>,
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

impl AssetServer {
    pub fn new() -> Self {
        AssetServer {
            assets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // pub fn get<T: AssetT>(&asset_source);
}

// /// if `store_forever` is true, the asset is stored in the AssetServer forever until the end of the program.
// /// This is achieved by storing an Arc in the AssetServer instead of a Weak.
// ///
// /// Always returns a shared handle.
// ///
// /// todo!() maybe be smarter about a parameter `unique` and how it can interact with store_forever.
// /// e.g. if unique requested and not store_forever, we do not need to create an entry with an arc to begin with.
// ///
// /// Or new Storemode enum:
// /// ```
// /// enum StoreMode{
// ///     Never,
// ///     UntilAllDropped,
// ///     Forever,
// /// }
// /// ```
// pub async fn load<T: AssetT>(
//     &self,
//     source: &AssetSource,
//     store_forever: bool,
// ) -> anyhow::Result<Arc<T>> {
//     // check if the asset has been loaded AND is still loaded: we can return an Arc to it directly.
//     if let Some(cached) = self.load_from_cache(&source) {
//         return Ok(cached);
//     }

//     // load the asset from disk or network:
//     let bytes: Vec<u8> = source.fetch_bytes().await?;
//     let asset = T::from_bytes(&bytes)?;

//     // insert this arc into the cache:
//     let asset_arc = self.store_in_cache(&source, asset, store_forever); // handed out
//     Ok(Handle::Shared(asset_arc))
// }

// /// should only be called if we are sure the key is not in the hashmap yet.
// fn store_in_cache<T: AssetT>(
//     &self,
//     source: &AssetSource,
//     asset: T,
//     store_forever: bool,
// ) -> Arc<T> {
//     // create a new arc for this asset:
//     let asset_arc = Arc::new(asset);

//     // if `store_forever`, we clone this arc before storing it, so the reference count will stay >= 1 forever.
//     // otherwise we
//     let arc_inner_ptr: *const () = if store_forever {
//         let strong = asset_arc.clone();
//         unsafe { std::mem::transmute(strong) }
//     } else {
//         let weak = Arc::downgrade(&asset_arc);
//         unsafe { std::mem::transmute(weak) }
//     };
//     let stored_asset = StoredErasedAsset {
//         type_id: TypeId::of::<T>(),
//         is_weak: !store_forever,
//         arc_inner_ptr,
//     };

//     // insert into hashmap
//     let key = (source.clone(), TypeId::of::<T>());
//     let mut assets = self.assets.lock().unwrap();
//     debug_assert!(assets.get(&key).is_none());
//     assets.insert(key, stored_asset);

//     // return the arc:
//     asset_arc
// }

// /// check if the asset has been loaded AND is still loaded. If so, give out an Arc to it.
// fn load_from_cache<T: AssetT>(&self, source: &AssetSource) -> Option<Arc<T>> {
//     let key = (source.clone(), TypeId::of::<T>());
//     let assets = self.assets.lock().unwrap();
//     if let Some(stored_asset) = assets.get(&key) {
//         let stored_arc = stored_asset.get_arc::<T>();
//         if let Some(arc) = stored_arc {
//             return Some(arc);
//         }
//     }
//     None
// }

// /// version of load for sync use: You can try_resv the oneshot channel every frame, until the asset is loaded.
// pub fn initiate_load<T: AssetT>(
//     &self,
//     source: &AssetSource,
//     store_forever: bool,
// ) -> LoadingAsset<T> {
//     // if the asset is already there, we can return in directly:
//     if let Some(asset_arc) = self.load_from_cache::<T>(&source) {
//         return LoadingAsset::Done(Handle::Shared(asset_arc));
//     }

//     // otherwise start the loading process and return a LoadingAsset::Loading(..).
//     let (tx, rx) = oneshot::channel::<anyhow::Result<Handle<T>>>();
//     let asset_server = self.clone();
//     let source = source.clone();
//     tokio::spawn(async move {
//         let load_result = asset_server.load::<T>(&source, store_forever).await;
//         _ = tx.send(load_result);
//     });
//     LoadingAsset::Loading(rx)
// }

// pub fn load_blocking<T: AssetT>(
//     &self,
//     path_or_url: &str,
//     store_forever: bool,
// ) -> anyhow::Result<Handle<T>> {
//     let source = &AssetSource::from(path_or_url);
//     let loading = self.initiate_load::<T>(source, store_forever);
//     match loading {
//         LoadingAsset::Done(e) => return Ok(e),
//         LoadingAsset::Loading(loading) => loading.blocking_recv().expect("should be fine"),
//     }

/// Anything that can be stored in the AssetServer.
pub trait AssetT: Sized + 'static + Sync + Send {}
impl<T> AssetT for T where T: Sized + 'static + Sync + Send {}
