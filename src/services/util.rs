use crate::services::image_processing::RgbaConvert;
use image::RgbaImage;
use mime::IMAGE_PNG;
use reqwest::header::{self, HeaderValue};

pub async fn fetch_image(url: &str) -> Option<RgbaImage> {
    let response = reqwest::get(url).await;
    let Ok(data) = response else {
        return None;
    };
    if data.headers().get(header::CONTENT_TYPE)
        != Some(&HeaderValue::from_str(IMAGE_PNG.essence_str()).unwrap())
    {
        return None;
    }
    let bytes = data.bytes().await.unwrap();
    let image = RgbaImage::from_png(&bytes);
    Some(image)
}
