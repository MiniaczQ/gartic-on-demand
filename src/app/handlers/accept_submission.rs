use super::{AppData, AppError, AssetHandler};
use crate::app::{
    config::CONFIG,
    error::{ConvertError, OptionEmptyError},
    permission::has_mod,
    renderer::Renderer,
    util::{fetch_raw_image_from_attachment, raw_image_to_attachment},
};
use async_trait::async_trait;
use poise::{Event, FrameworkContext};
use rossbot::services::{
    database::session::SessionRepository, provider::Provider, status_update::StatusUpdateWaker,
};
use serenity::prelude::Context;
use std::cmp::Ordering;

#[derive(Debug)]
pub struct AcceptSubmission;

#[async_trait]
impl AssetHandler for AcceptSubmission {
    async fn handle<'a>(
        &self,
        ctx: &Context,
        event: &Event<'a>,
        _fcx: FrameworkContext<'a, AppData, AppError>,
        data: &AppData,
    ) -> Result<(), AppError> {
        let sr: SessionRepository = data.get();
        match event {
            Event::ReactionAdd { add_reaction } => {
                let user = add_reaction.user(&ctx).await?;
                if add_reaction.channel_id != CONFIG.channels.moderation {
                    return Ok(());
                }
                if user.bot {
                    return Ok(());
                }
                has_mod(&ctx, &user).await?;

                let accept = add_reaction
                    .emoji
                    .unicode_partial_cmp(&CONFIG.reactions.accept)
                    == Some(Ordering::Equal);
                let reject = add_reaction
                    .emoji
                    .unicode_partial_cmp(&CONFIG.reactions.reject)
                    == Some(Ordering::Equal);

                let accepted = match (accept, reject) {
                    (true, false) => true,
                    (false, true) => false,
                    (false, false) => return Ok(()),
                    (true, true) => unreachable!(),
                };

                let old_aid = add_reaction.message_id.0;
                let lobby = sr
                    .get_pending(old_aid)
                    .await
                    .map_internal("Failed to get pending session")?;

                let old_message = add_reaction.message(&ctx).await?;
                let old_attachment = &old_message.attachments[0];

                let uid = user.id.0;

                if accepted {
                    let (channel, attachment, content) = if lobby.active.last {
                        let channel = match lobby.lobby.nsfw {
                            true => CONFIG.channels.complete_nsfw,
                            false => CONFIG.channels.complete,
                        };
                        let attachment = lobby
                            .active
                            .mode
                            .render_complete(&ctx, &lobby, &data.get(), old_attachment)
                            .await?;
                        let content = lobby.description_long();
                        (channel, attachment, content)
                    } else {
                        let channel = match lobby.lobby.nsfw {
                            true => CONFIG.channels.partial_nsfw,
                            false => CONFIG.channels.partial,
                        };
                        let attachment = lobby.active.mode.render_partial(old_attachment).await?;
                        let content = lobby.description_short();
                        (channel, attachment, content)
                    };
                    let new_message = channel
                        .send_message(ctx, |m| m.add_file(attachment).content(content))
                        .await?;
                    sr.accept_pending(uid, old_aid, new_message.id.0)
                        .await
                        .map_internal("Failed to accept/reject session")?;
                } else {
                    let channel = CONFIG.channels.rejects;
                    let raw_image = fetch_raw_image_from_attachment(old_attachment)
                        .await
                        .ok_or(AppError::internal(
                            OptionEmptyError,
                            "Failed to fetch image",
                        ))?;
                    let attachment = raw_image_to_attachment(raw_image.into());
                    let content = lobby.description_short();
                    let new_message = channel
                        .send_message(ctx, |m| m.add_file(attachment).content(content))
                        .await?;
                    sr.reject_pending(uid, old_aid, new_message.id.0)
                        .await
                        .map_internal("Failed to accept/reject session")?;
                }

                old_message.delete(&ctx).await?;

                let sw: StatusUpdateWaker = data.get();
                sw.wake();
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
