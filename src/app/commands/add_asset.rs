use crate::app::{
    config::CONFIG, error::ConvertError, response::ResponseContext,
    util::fetch_image_from_attachment, AppContext, AppError,
};
use poise::serenity_prelude::{Attachment, AttachmentType, ChannelId};
use rossbot::services::{
    database::images::{AssetKind, Image, ImageRepository},
    image_processing::{normalize_image, RgbaConvert},
    provider::Provider,
};
use std::borrow::Cow;
use tracing::error;

#[derive(Debug, poise::ChoiceParameter)]
pub enum AssetKindArg {
    InConstruction,
    DrawThis,
}

#[poise::command(slash_command, guild_only)]
pub async fn add_asset(
    ctx: AppContext<'_>,
    attachment: Attachment,
    kind: AssetKindArg,
) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    rsx.init().await?;
    if let Err(e) = process(&mut rsx, ctx, attachment, kind).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

pub async fn process(
    rcx: &mut ResponseContext<'_>,
    ctx: AppContext<'_>,
    attachment: Attachment,
    kind: AssetKindArg,
) -> Result<(), AppError> {
    let kind: AssetKind = kind.into();

    let image = fetch_image_from_attachment(&attachment)
        .await
        .map_user("Attachment is not an image")?;
    let image = normalize_image(&image, CONFIG.image.width, CONFIG.image.height);
    let image = AttachmentType::Bytes {
        data: Cow::Owned(image.to_png().to_vec()),
        filename: ctx.id().to_string() + ".png",
    };

    let user = ctx.author();

    let message = kind_to_channel(kind)
        .send_message(ctx, |m| {
            m.add_file(image)
                .content(format!("<@{}>", user.id))
                .reactions([CONFIG.reactions.delete])
        })
        .await?;

    let ir: ImageRepository = ctx.data().get();
    ir.create(message.id.0, Image::new(kind.into(), user.id))
        .await
        .map_internal("Failed to add image to database")?;

    rcx.respond(|f| f.content("Added")).await?;

    Ok(())
}

impl From<AssetKindArg> for AssetKind {
    fn from(value: AssetKindArg) -> Self {
        match value {
            AssetKindArg::InConstruction => AssetKind::InConstruction,
            AssetKindArg::DrawThis => AssetKind::DrawThis,
        }
    }
}

fn kind_to_channel(kind: AssetKind) -> ChannelId {
    match kind {
        AssetKind::InConstruction => CONFIG.channels.in_contruction,
        AssetKind::DrawThis => CONFIG.channels.draw_this,
    }
}
