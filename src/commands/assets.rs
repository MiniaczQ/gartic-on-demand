use std::borrow::Cow;

use poise::{
    serenity_prelude::{Attachment, AttachmentType},
    Context,
};
use rossbot::services::{
    assets::AssetManager,
    database::assets::{AssetKind, AssetRepository},
    image_processing::normalize_image,
    provider::Provider,
    util::fetch_image_from_attachment,
};

use crate::{
    app::{config::CONFIG, AppError, ApplicationError, UserError},
    AppData,
};

#[derive(Debug, poise::ChoiceParameter)]
pub enum AssetKindArg {
    InConstruction,
    DrawThis,
}

impl From<AssetKindArg> for AssetKind {
    fn from(value: AssetKindArg) -> Self {
        match value {
            AssetKindArg::InConstruction => AssetKind::InConstruction,
            AssetKindArg::DrawThis => AssetKind::DrawThis,
        }
    }
}

#[poise::command(slash_command, guild_cooldown = 60, guild_only)]
pub async fn add_asset(
    ctx: Context<'_, AppData, AppError>,
    attachment: Attachment,
    kind: AssetKindArg,
) -> Result<(), AppError> {
    let image = fetch_image_from_attachment(attachment)
        .await
        .ok_or(UserError("Attachment is not an image"))?;

    let image = normalize_image(&image, CONFIG.image.width, CONFIG.image.height);

    let asset_manager: AssetManager = ctx.data().get();
    asset_manager
        .add(ctx.id(), kind.into(), image)
        .await
        .ok_or(ApplicationError("Failed to add asset"))?;

    ctx.reply("Asset uploaded!").await?;

    Ok(())
}

#[poise::command(slash_command)]
pub async fn show_assets(
    ctx: Context<'_, AppData, AppError>,
    kind: AssetKindArg,
    #[min = 1]
    #[max = 9]
    limit: Option<u64>,
    #[min = 1] initial_page: Option<u64>,
) -> Result<(), AppError> {
    let db = &ctx.data().db;
    let am: AssetManager = ctx.data().get();
    let kind: AssetKind = kind.into();
    let limit = limit.unwrap_or(9);

    async fn pull<'a>(
        asset_manager: &'a AssetManager<'_>,
        kind: AssetKind,
        limit: u64,
        start: u64,
    ) -> Option<Vec<AttachmentType<'a>>> {
        let assets = asset_manager.list(kind.into(), limit, start).await?;
        let attachments: Vec<_> = assets
            .iter()
            .map(|(id, bytes)| AttachmentType::Bytes {
                data: Cow::Owned(bytes.to_vec()),
                filename: id.to_string() + ".png",
            })
            .collect();
        Some(attachments)
    }

    let mut total = db
        .get_asset_count(kind)
        .await
        .ok_or(ApplicationError("Failed to list assets"))?;
    let max_page = total.saturating_sub(1) / limit;
    let mut current_page = (initial_page.unwrap_or(1) - 1).clamp(0, max_page);
    let attachments = pull(&am, kind, limit, current_page * limit)
        .await
        .ok_or(ApplicationError("Failed to list assets"))?;

    let ctx_id = ctx.id();
    let prev_button_id = format!("{}-prev", ctx_id);
    let next_button_id = format!("{}-next", ctx_id);
    let handle = ctx
        .send(|b| {
            b.attachments = attachments.clone();
            b.embed(|b| {
                b.footer(|f| f.text(format!("Page {}/{}", current_page + 1, max_page + 1)))
            });
            b.components(|b| {
                b.create_action_row(|b| {
                    b.create_button(|b| b.custom_id(&prev_button_id).emoji('◀'))
                        .create_button(|b| b.custom_id(&next_button_id).emoji('▶'))
                })
            })
        })
        .await?;

    while let Some(press) = poise::serenity_prelude::CollectComponentInteraction::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(std::time::Duration::from_secs(60))
        .await
    {
        let max_page = total.saturating_sub(1) / limit;
        if press.data.custom_id == next_button_id {
            current_page += 1;
            if current_page > max_page {
                current_page = 0;
            }
        } else if press.data.custom_id == prev_button_id {
            current_page = current_page.checked_sub(1).unwrap_or(max_page);
        } else {
            continue;
        }

        total = db
            .get_asset_count(kind)
            .await
            .ok_or(ApplicationError("Failed to list assets"))?;
        let attachments = pull(&am, kind, limit, current_page * limit)
            .await
            .ok_or(ApplicationError("Failed to list assets"))?;

        press
            .create_interaction_response(ctx, |b| {
                b.kind(poise::serenity_prelude::InteractionResponseType::UpdateMessage)
                    .interaction_response_data(|b| {
                        b.embed(|b| {
                            b.footer(|f| {
                                f.text(format!("Page {}/{}", current_page + 1, max_page + 1))
                            })
                        });
                        b.1 = attachments;
                        b
                    })
            })
            .await?;
    }

    handle.delete(ctx).await.ok();

    Ok(())
}

#[poise::command(slash_command)]
pub async fn remove_asset(ctx: Context<'_, AppData, AppError>, id: u64) -> Result<(), AppError> {
    Ok(())
}
