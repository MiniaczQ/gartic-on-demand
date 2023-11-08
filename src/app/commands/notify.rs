use crate::app::{
    config::CONFIG, error::ConvertError, response::ResponseContext, AppContext, AppError,
};
use gartic_on_demand::services::{
    database::{
        user::{User, UserRepository},
        Record,
    },
    provider::Provider,
};
use poise::serenity_prelude::Member;
use tracing::error;

#[derive(Debug, poise::ChoiceParameter)]
pub enum NotificationArg {
    Disable,
    Once,
    Always,
}

/// Enable or disable activity notifications
#[poise::command(slash_command, guild_only)]
pub async fn notify(ctx: AppContext<'_>, notification: NotificationArg) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    rsx.init().await?;
    if let Err(e) = process(&mut rsx, ctx, notification).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

async fn process(
    rsx: &mut ResponseContext<'_>,
    ctx: AppContext<'_>,
    notification: NotificationArg,
) -> Result<(), AppError> {
    let ur: UserRepository = ctx.data().get();
    let discord_user = ctx.author();
    let user = ur
        .create_or_update_user(discord_user.id.0, &discord_user.name)
        .await
        .map_internal("Failed to get user")?;
    let mut member = CONFIG.guild.member(&ctx, discord_user.id).await?;
    match notification {
        NotificationArg::Disable => disable(&ctx, &ur, &user, &mut member).await?,
        NotificationArg::Once => once(&ctx, &ur, &user, &mut member).await?,
        NotificationArg::Always => always(&ctx, &ur, &user, &mut member).await?,
    }
    rsx.respond(|b| b.content("Done!")).await?;
    Ok(())
}

async fn disable(
    ctx: &AppContext<'_>,
    ur: &UserRepository,
    user: &Record<User>,
    member: &mut Member,
) -> Result<(), AppError> {
    if user.notify_once {
        ur.update_notify_once(user.id(), false)
            .await
            .map_internal("Failed to update user")?;
    }
    if member.roles.contains(&CONFIG.roles.notify_always) {
        member.remove_role(ctx, &CONFIG.roles.notify_always).await?;
    }
    Ok(())
}

async fn once(
    ctx: &AppContext<'_>,
    ur: &UserRepository,
    user: &Record<User>,
    member: &mut Member,
) -> Result<(), AppError> {
    if !user.notify_once {
        ur.update_notify_once(user.id(), true)
            .await
            .map_internal("Failed to update user")?;
    }
    if member.roles.contains(&CONFIG.roles.notify_always) {
        member.remove_role(ctx, &CONFIG.roles.notify_always).await?;
    }
    Ok(())
}

async fn always(
    ctx: &AppContext<'_>,
    ur: &UserRepository,
    user: &Record<User>,
    member: &mut Member,
) -> Result<(), AppError> {
    if user.notify_once {
        ur.update_notify_once(user.id(), false)
            .await
            .map_internal("Failed to update user")?;
    }
    if !member.roles.contains(&CONFIG.roles.notify_always) {
        member.add_role(ctx, &CONFIG.roles.notify_always).await?;
    }
    Ok(())
}
