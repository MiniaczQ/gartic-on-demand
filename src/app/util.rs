use super::{config::CONFIG, error::AppError, response::ResponseContext, AppContext};
use image::RgbaImage;
use mime::IMAGE_PNG;
use poise::serenity_prelude::{Attachment, AttachmentType, Message, MessageId};
use reqwest::{
    header::{self, HeaderValue},
    Url,
};
use rossbot::services::{database::session::Session, image_processing::RgbaConvert};

pub fn get_image_attachment_url(message: &Message) -> Option<Url> {
    let attachment = message.attachments.get(0)?;
    if attachment.content_type.as_deref() != Some(IMAGE_PNG.essence_str()) {
        return None;
    }
    let url = match attachment.url.find('?') {
        Some(i) => &attachment.url[..i],
        None => &attachment.url,
    };
    Some(url.try_into().unwrap())
}

pub async fn fetch_image_from_attachment(attachment: Attachment) -> Option<RgbaImage> {
    if attachment.content_type.as_deref() != Some(IMAGE_PNG.essence_str()) {
        return None;
    }
    let url = match attachment.url.find('?') {
        Some(i) => &attachment.url[..i],
        None => &attachment.url,
    };
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

pub async fn extract_images<'a>(
    ctx: AppContext<'_>,
    session: &Session,
) -> Result<Vec<AttachmentType<'a>>, AppError> {
    let mut attachments = Vec::with_capacity(session.game.images.len());
    for image in session.game.images.iter() {
        let msg = CONFIG
            .channels
            .attributes
            .message(ctx, MessageId(*image))
            .await?;
        let url = get_image_attachment_url(&msg).unwrap();
        let attachment = AttachmentType::Image(url);
        attachments.push(attachment);
    }
    Ok(attachments)
}

pub async fn display_started_round(
    rsx: &mut ResponseContext<'_>,
    ctx: AppContext<'_>,
    session: Session,
) -> Result<(), AppError> {
    let attachments = extract_images(ctx, &session).await?;
    rsx.respond(|f| {
        f.attachments = attachments;
        f.content(session.prompt_started())
    })
    .await?;
    Ok(())
}
