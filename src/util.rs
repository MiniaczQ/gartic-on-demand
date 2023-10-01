use image::RgbaImage;
use mime::IMAGE_PNG;
use reqwest::header::{self, HeaderValue};
use serenity::model::prelude::Message;

use crate::image_processing::RgbaConvert;

pub fn get_image_attachment_link(message: &Message) -> Option<&str> {
    let attachment = message.attachments.get(0)?;
    if attachment.content_type.as_deref() != Some(IMAGE_PNG.essence_str()) {
        return None;
    }
    let url = match attachment.url.find('?') {
        Some(i) => &attachment.url[..i],
        None => &attachment.url,
    };
    Some(url)
}

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
