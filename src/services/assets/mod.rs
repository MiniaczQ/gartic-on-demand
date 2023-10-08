use bytes::Bytes;
use image::RgbaImage;

use super::{
    database::{
        assets::{AssetEntry, AssetKind, AssetRepository},
        Database,
    },
    image_processing::RgbaConvert,
    provider::Provider,
    storage::Storage,
};

pub struct AssetManager<'a> {
    db: &'a Database,
    sg: &'a Storage,
}

impl<'a> AssetManager<'a> {
    pub async fn random(&self, n: u32) -> Option<Vec<RgbaImage>> {
        todo!()
    }

    pub async fn add(&self, id: u64, kind: AssetKind, image: RgbaImage) -> Option<()> {
        self.db.create_asset(id, AssetEntry::new(kind)).await?;

        self.sg
            .upload(id.to_string() + ".png", &image.to_png())
            .await?;

        Some(())
    }

    pub async fn remove(&self, id: u64) -> Result<(), ()> {
        todo!()
    }

    pub async fn list(&self, kind: AssetKind, limit: u64, start: u64) -> Option<Vec<(u64, Bytes)>> {
        let assets = self.db.get_assets(kind, limit, start).await?;

        let mut images = Vec::with_capacity(assets.len());
        for asset in assets {
            let bytes = self.sg.download(asset.id.to_string() + ".png").await?;
            images.push((asset.id, bytes));
        }

        Some(images)
    }
}

impl<'a, T> Provider<'a, AssetManager<'a>> for T
where
    T: Provider<'a, &'a Database>,
    T: Provider<'a, &'a Storage>,
{
    fn get(&'a self) -> AssetManager<'a> {
        AssetManager {
            db: self.get(),
            sg: self.get(),
        }
    }
}
