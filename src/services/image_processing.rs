use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use image::{
    codecs::png::PngEncoder,
    imageops::{self, FilterType},
    io::Reader,
    GenericImage, GenericImageView, ImageBuffer, Pixel, Rgba, RgbaImage,
};
use std::{cmp::max, io::Cursor, path::Path};
use tokio::fs::OpenOptions;
use tokio_util::io::read_buf;

const WHITE: Rgba<u8> = Rgba([255u8, 255u8, 255u8, 255u8]);

#[async_trait]
pub trait RgbaConvert {
    fn from_png(bytes: &Bytes) -> Self;
    fn to_png(&self) -> Vec<u8>;
    async fn load(path: impl AsRef<Path> + Send + Sync) -> Self;
}

#[async_trait]
impl RgbaConvert for RgbaImage {
    fn from_png(bytes: &Bytes) -> Self {
        let mut reader = Reader::new(Cursor::new(bytes));
        reader.set_format(image::ImageFormat::Png);
        reader.decode().unwrap().into_rgba8()
    }

    fn to_png(&self) -> Vec<u8> {
        let mut writer = Vec::new();
        let encoder = PngEncoder::new(&mut writer);
        self.write_with_encoder(encoder).unwrap();
        writer
    }

    async fn load(path: impl AsRef<Path> + Send + Sync) -> Self {
        let mut file = OpenOptions::default().read(true).open(path).await.unwrap();
        let mut buf = BytesMut::new();
        read_buf(&mut file, &mut buf).await.unwrap();
        Self::from_png(&buf.freeze())
    }
}

pub trait RgbaProcess {
    fn focus_aoi(&self, pad: u32) -> Self;
    fn remove_alpha(&self) -> Self;
    fn scale_to_fit(&self, new_width: u32, new_height: u32) -> Self;
    fn pad_to_size(&self, new_width: u32, new_height: u32) -> Self;
}

fn content_aabb(image: &impl GenericImage<Pixel = Rgba<u8>>) -> (u32, u32, u32, u32) {
    let (mut x_min, mut y_min, mut x_max, mut y_max) = (u32::MAX, u32::MAX, u32::MIN, u32::MIN);
    for (x, y, pixel) in image.pixels() {
        match pixel.0 {
            [_, _, _, 0] | [255, 255, 255, _] => {}
            _ => {
                x_min = x_min.min(x);
                y_min = y_min.min(y);
                x_max = x_max.max(x);
                y_max = y_max.max(y);
            }
        }
    }
    (x_min, y_min, x_max, y_max)
}

fn resize_dimensions(
    width: u32,
    height: u32,
    new_width: u32,
    new_height: u32,
    fill: bool,
) -> (u32, u32) {
    let wratio = new_width as f64 / width as f64;
    let hratio = new_height as f64 / height as f64;

    let ratio = if fill {
        f64::max(wratio, hratio)
    } else {
        f64::min(wratio, hratio)
    };

    let nw = max((width as f64 * ratio).round() as u64, 1);
    let nh = max((height as f64 * ratio).round() as u64, 1);

    if nw > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / width as f64;
        (u32::MAX, max((height as f64 * ratio).round() as u32, 1))
    } else if nh > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / height as f64;
        (max((width as f64 * ratio).round() as u32, 1), u32::MAX)
    } else {
        (nw as u32, nh as u32)
    }
}

impl RgbaProcess for RgbaImage {
    fn focus_aoi(&self, pad: u32) -> Self {
        let (x_min, y_min, x_max, y_max) = content_aabb(self);
        let (width, height) = self.dimensions();

        let y_min = y_min.saturating_sub(pad);
        let y_max = (y_max + pad).min(height - 1);
        let x_min = x_min.saturating_sub(pad);
        let x_max = (x_max + pad).min(width - 1);

        let (new_width, new_height) = (x_max - x_min, y_max - y_min);
        self.view(x_min, y_min, new_width, new_height).to_image()
    }

    fn remove_alpha(&self) -> Self {
        let (width, height) = self.dimensions();
        let mut new_image = ImageBuffer::from_pixel(width, height, WHITE);
        new_image
            .pixels_mut()
            .zip(self.pixels())
            .for_each(|(new, old)| new.blend(old));
        new_image
    }

    fn scale_to_fit(&self, new_width: u32, new_height: u32) -> Self {
        let (width, height) = self.dimensions();
        if (new_width, new_height) == (width, height) {
            return self.clone();
        }
        let (new_width, new_height) =
            resize_dimensions(width, height, new_width, new_height, false);
        imageops::resize(self, new_width, new_height, FilterType::CatmullRom)
    }

    fn pad_to_size(&self, new_width: u32, new_height: u32) -> Self {
        let mut new_image = RgbaImage::from_pixel(new_width, new_height, WHITE);
        let (width, height) = self.dimensions();
        let x = (new_width - width) / 2;
        let y = (new_height - height) / 2;
        new_image.copy_from(self, x, y).unwrap();
        new_image
    }
}

pub fn concat_2_2(images: &[RgbaImage]) -> RgbaImage {
    assert!(images.len() == 4);
    let h = images.iter().map(|i| i.height()).max().unwrap();
    let w = images.iter().map(|i| i.width()).max().unwrap();
    let mut concated = ImageBuffer::new(w * 2, h * 2);
    for (i, img) in images.iter().enumerate() {
        let i = i as u32;
        let x = (i % 2) * w;
        let y = (i / 2) * h;
        concated.copy_from(img, x, y).unwrap();
    }
    concated
}

pub fn concat_vertical(images: &[RgbaImage]) -> RgbaImage {
    let mut delta_height = 0;
    let w = images.iter().map(|i| i.width()).max().unwrap();
    let total_height = images.iter().map(|i| i.height()).sum();
    let mut concated = ImageBuffer::new(w, total_height);
    for img in images.iter() {
        let x = 0;
        let y = delta_height;
        delta_height += img.height();
        concated.copy_from(img, x, y).unwrap();
    }
    concated
}

pub fn normalize_image(image: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    image
        .remove_alpha()
        .scale_to_fit(width, height)
        .pad_to_size(width, height)
}

pub fn normalize_image_aoi(image: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    normalize_image(&image.focus_aoi(5), width, height)
}

pub fn normalize_images_aoi(images: &[&RgbaImage], width: u32, height: u32) -> Vec<RgbaImage> {
    images
        .iter()
        .map(|i| normalize_image_aoi(i, width, height))
        .collect()
}

pub fn combined(images: &[&RgbaImage], width: u32, height: u32) -> Vec<u8> {
    let images = normalize_images_aoi(images, width, height);
    let image = concat_2_2(&images);
    image.to_png()
}
