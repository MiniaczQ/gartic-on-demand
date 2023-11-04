use crate::app::{config::CONFIG, permission::has_admin};

use super::{AppData, AppError, AssetHandler};
use async_trait::async_trait;
use gartic_bot::services::{database::assets::ImageRepository, provider::Provider};
use poise::{Event, FrameworkContext};
use serenity::prelude::Context;
use std::cmp::Ordering;
use tracing::error;

#[derive(Debug)]
pub struct RemoveAsset;

#[async_trait]
impl AssetHandler for RemoveAsset {
    async fn handle<'a>(
        &self,
        ctx: &Context,
        event: &Event<'a>,
        _fcx: FrameworkContext<'a, AppData, AppError>,
        data: &AppData,
    ) -> Result<(), AppError> {
        let ir: ImageRepository = data.get();
        let channels = [CONFIG.channels.draw_this, CONFIG.channels.in_contruction];
        match event {
            Event::ReactionAdd { add_reaction } => {
                let user = add_reaction.user(&ctx).await?;
                if !channels.contains(&add_reaction.channel_id) {
                    return Ok(());
                }
                if user.bot {
                    return Ok(());
                }
                if add_reaction
                    .emoji
                    .unicode_partial_cmp(&CONFIG.reactions.delete)
                    != Some(Ordering::Equal)
                {
                    return Ok(());
                }
                has_admin(&ctx, &user).await?;
                let result = ir.delete(add_reaction.message_id.0).await;
                match result {
                    Ok(_) => {
                        add_reaction
                            .channel_id
                            .edit_message(ctx, add_reaction.message_id, |f| f.content("Deleted"))
                            .await
                            .ok();
                    }
                    Err(e) => error!(error = ?e),
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
