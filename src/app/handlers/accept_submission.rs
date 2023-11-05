use super::{AppData, AppError, AssetHandler};
use crate::app::{
    config::CONFIG,
    error::ConvertError,
    permission::has_mod,
    rendering::{ModeRenderer, RoundRenderer},
    util::{fetch_raw_image_from_attachment, raw_image_to_attachment},
};
use async_trait::async_trait;
use gartic_on_demand::services::{
    database::{attempt::AttemptRepository, user::UserRepository, ThingToU64},
    gamemodes::GameLogic,
    provider::Provider,
    status_update::StatusUpdateWaker,
};
use poise::{Event, FrameworkContext};
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
        let ar: AttemptRepository = data.get();
        let ur: UserRepository = data.get();
        match event {
            Event::ReactionAdd { add_reaction } => {
                let user = add_reaction.user(&ctx).await?;
                let reviewer = ur
                    .create_or_update_user(user.id.0, &user.name)
                    .await
                    .map_internal("Failed to update user")?;

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
                let round = ar
                    .get_pending_attempt(old_aid)
                    .await
                    .map_internal("Failed to get pending session")?;
                let user = ur
                    .get_user(round.attempt.who.to_u64())
                    .await
                    .map_internal("Failed to get user")?;

                let old_message = add_reaction.message(&ctx).await?;
                let old_attachment = &old_message.attachments[0];

                if accepted {
                    let (channel, attachment, content) =
                        if round.round.round_no == round.round.mode.last_round() {
                            let channel = match round.round.nsfw {
                                true => CONFIG.channels.complete_nsfw,
                                false => CONFIG.channels.complete,
                            };
                            let attachment = round
                                .round
                                .mode
                                .render_complete_image(&ctx, &round, &data.get(), old_attachment)
                                .await?;
                            let content = round.render_complete_text();
                            (channel, attachment, content)
                        } else {
                            let channel = match round.round.nsfw {
                                true => CONFIG.channels.partial_nsfw,
                                false => CONFIG.channels.partial,
                            };
                            let attachment = round
                                .round
                                .mode
                                .render_partial_image(old_attachment)
                                .await?;
                            let content = round.render_partial_text();
                            (channel, attachment, content)
                        };
                    let new_message = channel
                        .send_message(ctx, |m| m.add_file(attachment).content(content))
                        .await?;
                    ar.approve_pending_attempt(&user, &reviewer, new_message.id.0)
                        .await
                        .map_internal("Failed to accept/reject session")?;
                } else {
                    let channel = CONFIG.channels.rejects;
                    let raw_image = fetch_raw_image_from_attachment(old_attachment)
                        .await
                        .map_internal("Failed to fetch image")?;
                    let attachment = raw_image_to_attachment(raw_image.into());
                    let content = round.render_partial_text();
                    let new_message = channel
                        .send_message(ctx, |m| m.add_file(attachment).content(content))
                        .await?;
                    ar.reject_pending_attempt(&user, &reviewer, new_message.id.0)
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
