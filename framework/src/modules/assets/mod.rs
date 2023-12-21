use std::{
    any::TypeId,
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, Mutex, Weak},
};

use self::fetchable_asset::AssetSource;

pub mod asset_loader;
pub mod asset_store;
pub mod fetchable_asset;

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
