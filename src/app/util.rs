use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
    AppContext,
};
use image::RgbaImage;
use mime::IMAGE_PNG;
use poise::serenity_prelude::{Attachment, AttachmentType, ChannelId, MessageId};
use reqwest::header::{self, HeaderValue};
use rossbot::services::{
    database::{
        assets::{AssetKind, ImageRepository},
        session::LobbyWithSessions,
    },
    image_processing::{concat_2_2, RgbaConvert},
    provider::Provider,
};

pub async fn fetch_image_from_attachment(attachment: &Attachment) -> Option<RgbaImage> {
    if attachment.content_type.as_deref() != Some(IMAGE_PNG.essence_str()) {
        return None;
    }
    let response = reqwest::get(&attachment.url).await;
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

pub async fn extract_2x2_image<T>(
    ctx: AppContext<'_>,
    lobby: &LobbyWithSessions<T>,
) -> Result<RgbaImage, AppError> {
    let ar: ImageRepository = ctx.data().get();
    let mut images = Vec::with_capacity(4);
    for what in lobby.accepted.iter().map(|a| a.state.what) {
        let image = fetch_image_from_channel(ctx, CONFIG.channels.partial, what).await?;
        images.push(image);
    }
    if images.len() < 4 {
        let assets = ar
            .random(AssetKind::DrawThis, 1)
            .await
            .map_internal("Missing DrawThis assets")?;
        for image in assets.into_iter().map(|a| a.id) {
            let image = fetch_image_from_channel(ctx, CONFIG.channels.draw_this, image).await?;
            images.push(image);
        }
    }
    if images.len() < 4 {
        let assets = ar
            .random(AssetKind::InConstruction, 4 - images.len() as u32)
            .await
            .map_internal("Missing InConstruction assets")?;
        for image in assets.into_iter().map(|a| a.id) {
            let image =
                fetch_image_from_channel(ctx, CONFIG.channels.in_contruction, image).await?;
            images.push(image);
        }
    }
    let image = concat_2_2(&images);
    Ok(image)
}

pub fn image_to_attachment<'a>(image: RgbaImage) -> AttachmentType<'a> {
    AttachmentType::Bytes {
        data: std::borrow::Cow::Owned(image.to_png()),
        filename: "image.png".to_owned(),
    }
}

pub async fn fetch_image_from_channel(
    ctx: AppContext<'_>,
    channel: ChannelId,
    image_id: u64,
) -> Result<RgbaImage, AppError> {
    let msg = channel.message(ctx, MessageId(image_id)).await?;
    let image = fetch_image_from_attachment(&msg.attachments[0])
        .await
        .unwrap();
    Ok(image)
}
