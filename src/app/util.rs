use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
    response::ResponseContext,
    AppContext,
};
use bytes::Bytes;
use image::RgbaImage;
use mime::IMAGE_PNG;
use poise::serenity_prelude::{Attachment, AttachmentType, ChannelId, MessageId};
use reqwest::header::{self, HeaderValue};
use rossbot::services::{
    database::{
        assets::{AssetKind, ImageRepository},
        session::{Active, LobbyWithSessions, SubmissionKind},
    },
    image_processing::{concat_2_2, RgbaConvert},
    provider::Provider,
};

pub async fn fetch_raw_image_from_attachment(attachment: &Attachment) -> Option<Bytes> {
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
    Some(bytes)
}

pub async fn fetch_image_from_attachment(attachment: &Attachment) -> Option<RgbaImage> {
    let bytes = fetch_raw_image_from_attachment(attachment).await?;
    let image = RgbaImage::from_png(&bytes);
    Some(image)
}

pub async fn extract_2x2_image<T>(
    ctx: &AppContext<'_>,
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

pub fn raw_image_to_attachment<'a>(bytes: Vec<u8>) -> AttachmentType<'a> {
    AttachmentType::Bytes {
        data: std::borrow::Cow::Owned(bytes),
        filename: "image.png".to_owned(),
    }
}

pub fn image_to_attachment<'a>(image: RgbaImage) -> AttachmentType<'a> {
    raw_image_to_attachment(image.to_png())
}

pub async fn fetch_image_from_channel(
    ctx: &AppContext<'_>,
    channel: ChannelId,
    image_id: u64,
) -> Result<RgbaImage, AppError> {
    let msg = channel.message(ctx, MessageId(image_id)).await?;
    let image = fetch_image_from_attachment(&msg.attachments[0])
        .await
        .unwrap();
    Ok(image)
}

pub async fn show_round(
    rsx: &mut ResponseContext<'_>,
    ctx: &AppContext<'_>,
    lobby: &LobbyWithSessions<Active>,
    in_progress: bool,
) -> Result<(), AppError> {
    let image = extract_2x2_image(ctx, lobby).await?;
    let attachment = image_to_attachment(image);
    rsx.respond(|f| f.attachment(attachment).content(lobby.prompt(in_progress)))
        .await?;
    Ok(())
}

pub fn session_destination<S>(lobby: &LobbyWithSessions<S>) -> ChannelId {
    match (&lobby.active.kind, lobby.lobby.nsfw) {
        (SubmissionKind::Partial, false) => CONFIG.channels.partial,
        (SubmissionKind::Complete, false) => CONFIG.channels.complete,
        (SubmissionKind::Partial, true) => CONFIG.channels.partial_nsfw,
        (SubmissionKind::Complete, true) => CONFIG.channels.complete_nsfw,
    }
}
