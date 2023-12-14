use std::path::PathBuf;

use image::RgbaImage;
use tokio::sync::oneshot;

use super::AssetT;

/// An Asset that can be fetched from bytes. The bytes could come from anywhere, e.g. the network, the disk, embedded in the binary, don't care.
pub trait FetchableAssetT: AssetT {
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error>;
}

#[derive(Debug, Clone)]
pub struct ImageAsset {
    pub rgba: RgbaImage,
}

impl FetchableAssetT for ImageAsset {
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        let image = image::load_from_memory(bytes)?;
        let rgba = image.to_rgba8();
        Ok(ImageAsset { rgba })
    }
}

pub struct TextAsset {
    pub text: String,
}

impl FetchableAssetT for TextAsset {
    // Note: expects bytes to be utf8 encoded
    fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        let text = String::from_utf8(bytes.to_vec())?;
        Ok(TextAsset { text })
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
    pub fn fetch_in_background<T: FetchableAssetT>(self) -> LoadingAsset<T> {
        let (tx, rx) = oneshot::channel::<anyhow::Result<T>>();
        tokio::spawn(async move {
            let load_result = self.fetch().await;
            _ = tx.send(load_result);
        });
        LoadingAsset { rx }
    }

    pub async fn fetch<T: FetchableAssetT>(&self) -> anyhow::Result<T> {
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
