use std::path::PathBuf;

use anyhow::anyhow;
use image::RgbaImage;
use tokio::sync::oneshot;

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
        return AssetSource::File(PathBuf::from(value));
    }
}
