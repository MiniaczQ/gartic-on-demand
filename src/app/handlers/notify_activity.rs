use super::{AppData, AppError, AssetHandler};
use crate::app::config::CONFIG;
use async_trait::async_trait;
use poise::{Event, FrameworkContext};
use serenity::prelude::Context;

#[derive(Debug)]
pub struct NotifyActivity;

#[async_trait]
impl AssetHandler for NotifyActivity {
    async fn handle<'a>(
        &self,
        ctx: &Context,
        event: &Event<'a>,
        _fcx: FrameworkContext<'a, AppData, AppError>,
        _data: &AppData,
    ) -> Result<(), AppError> {
        match event {
            Event::ReactionAdd { add_reaction } => {
                if add_reaction.message_id != CONFIG.messages.notify {
                    return Ok(());
                }
                let user = add_reaction.user(&ctx).await?;
                let mut member = CONFIG.guild.member(&ctx, user.id).await?;
                member.add_role(&ctx, CONFIG.roles.notify_always).await?;
                Ok(())
            }
            Event::ReactionRemove { removed_reaction } => {
                if removed_reaction.message_id != CONFIG.messages.notify {
                    return Ok(());
                }
                let user = removed_reaction.user(&ctx).await?;
                let mut member = CONFIG.guild.member(&ctx, user.id).await?;
                member.remove_role(&ctx, CONFIG.roles.notify_always).await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
