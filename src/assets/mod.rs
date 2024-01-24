//! We provide 3 different types of pointers: Own, Ref and Eternal:
//!
//! | -       | clonable | always has value |
//! | ------- | -------- | ---------------- |
//! | Own     | no       | yes              |
//! | Ref     | yes      | no               |
//! | Eternal | yes      | yes              |

use std::{
    borrow::{Borrow, BorrowMut},
    cell::SyncUnsafeCell,
    hash::Hash,
    ops::{Deref, DerefMut},
    path::PathBuf,
    ptr::NonNull,
    sync::Arc,
    sync::Weak,
};

use anyhow::anyhow;
use image::RgbaImage;
use tokio::sync::oneshot;

/// Has one owner, cannot be cloned.
#[derive(Debug)]
pub struct Own<T> {
    _inner: Box<T>,
}

impl<T> Own<T> {
    /// todo! add `new_in` custom allocator
    pub fn new(value: T) -> Self {
        Own {
            _inner: Box::new(value),
        }
    }

    /// Warning! The Own<T> will be deallocated when dropped. Manually make sure all Ref<T> given out to it are not around anymore by then.
    #[inline]
    pub fn share(&self) -> Ref<T> {
        Ref {
            _inner: unsafe { std::mem::transmute(&*self._inner) },
        }
    }
}

impl<T> Deref for Own<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self._inner
    }
}

impl<T> DerefMut for Own<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self._inner
    }
}

impl<T> Borrow<T> for Own<T> {
    fn borrow(&self) -> &T {
        &self._inner
    }
}

impl<T> BorrowMut<T> for Own<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self._inner
    }
}

/// Ref is basically a raw pointer. When using it be very careful, that the backing memory is still around.
/// A common patterns should be to get one Own<T> at the start of the program or a Ref from Ref::eternal.
/// Then you give out as many Refs as you want but make sure that they are no longer in circulation when you end the program / go to the next stage.
///
/// Better than lifetime hell in games.
#[derive(Debug)]
pub struct Ref<T> {
    _inner: NonNull<T>,
}

impl<T> Hash for Ref<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self._inner.hash(state);
    }
}

impl<T> Borrow<T> for Ref<T> {
    fn borrow(&self) -> &T {
        unsafe { &*self._inner.as_ptr() }
    }
}

impl<T> Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self._inner.as_ptr() }
    }
}

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Self {
            _inner: self._inner.clone(),
        }
    }
}

impl<T> Copy for Ref<T> {}

impl<T> Ref<T> {
    pub fn eternal(value: T) -> Ref<T> {
        let reference = Box::leak(Box::new(value));
        let _inner = NonNull::from(reference);
        Ref { _inner }
    }

    pub fn as_u64_hash(&self) -> u64 {
        self._inner.as_ptr() as u64
    }
}

// checks if they are pointing to the same thing
impl<T> PartialEq for Ref<T> {
    fn eq(&self, other: &Self) -> bool {
        self._inner.as_ptr() == self._inner.as_ptr()
    }
}

impl<T> PartialOrd for Ref<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self._inner.as_ptr().partial_cmp(&other._inner.as_ptr())
    }
}

impl<T> Ord for Ref<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self._inner.as_ptr().cmp(&other._inner.as_ptr())
    }
}

impl<T> Eq for Ref<T> {}

/// An Asset that can be fetched from bytes. The bytes could come from anywhere, e.g. the network, the disk, embedded in the binary, don't care.
pub trait AssetT: Sized {
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error>;

    fn load(src: &str) -> impl std::future::Future<Output = Result<Self, anyhow::Error>> + Send {
        async move {
            let src = AssetSource::from(src);
            src.fetch().await
        }
    }
}

impl AssetT for RgbaImage {
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        let image = image::load_from_memory(bytes)?;
        let rgba = image.to_rgba8();
        Ok(rgba)
    }
}

impl AssetT for fontdue::Font {
    /// ttf bytes
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| anyhow!("fontdue error: {e}"))?;
        Ok(font)
    }
}

impl AssetT for String {
    // Note: expects bytes to be utf8 encoded
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        let text = String::from_utf8(bytes.to_vec())?;
        Ok(text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetSource {
    File(PathBuf),
    Url(reqwest::Url),
}

#[derive(Debug)]
pub struct LoadingAsset<T: AssetT> {
    rx: oneshot::Receiver<anyhow::Result<T>>,
}

impl<T: AssetT> LoadingAsset<T> {
    pub fn get(&mut self) -> Option<anyhow::Result<T>> {
        self.rx.try_recv().ok()
    }
}

impl AssetSource {
    pub fn fetch_in_background<T: AssetT + Send + 'static>(self) -> LoadingAsset<T> {
        let (tx, rx) = oneshot::channel::<anyhow::Result<T>>();
        tokio::spawn(async move {
            let load_result = self.fetch().await;
            _ = tx.send(load_result);
        });
        LoadingAsset { rx }
    }

    pub async fn fetch<T: AssetT>(&self) -> anyhow::Result<T> {
        let bytes = self.fetch_bytes().await?;
        let asset = T::from_bytes(&bytes)?;
        Ok(asset)
    }

    async fn fetch_bytes(&self) -> anyhow::Result<Vec<u8>> {
        match self {
            AssetSource::File(path) => {
                let bytes = tokio::fs::read(path).await?;
                Ok(bytes)
            }
            AssetSource::Url(url) => {
                let response = reqwest::get(url.clone()).await?;
                let bytes = response.bytes().await?;
                Ok(bytes.into_iter().collect())
            }
        }
    }
}

impl From<&str> for AssetSource {
    fn from(value: &str) -> Self {
        if let Ok(url) = reqwest::Url::parse(value) {
            return AssetSource::Url(url);
        }
        AssetSource::File(PathBuf::from(value))
    }
}
