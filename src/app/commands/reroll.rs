use crate::app::{
    error::ConvertError, response::ResponseContext, util::respond_with_prompt, AppContext, AppError,
};
use gartic_on_demand::services::{
    database::{attempt::AttemptRepository, round::RoundRepository, user::UserRepository},
    gamemodes::GameLogic,
    provider::Provider,
    status_update::StatusUpdateWaker,
};
use tracing::error;

/// Reroll the current prompt
#[poise::command(slash_command, guild_only)]
pub async fn reroll(ctx: AppContext<'_>) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    rsx.init().await?;
    if let Err(e) = process(&mut rsx, ctx).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

async fn process(rsx: &mut ResponseContext<'_>, ctx: AppContext<'_>) -> Result<(), AppError> {
    let ar: AttemptRepository = ctx.data().get();
    let ur: UserRepository = ctx.data().get();
    let rr: RoundRepository = ctx.data().get();
    let discord_user = ctx.author();
    let user = ur
        .create_or_update_user(discord_user.id.0, &discord_user.name)
        .await
        .map_internal("Failed to update user")?;

    let round = rr
        .get_active_round(&user)
        .await
        .map_user("Failed to find existing session")?;

    ar.cancel_active_attempt(&user)
        .await
        .map_internal("Failed to cancel active session")?;

    if let Ok(round) = rr
        .attempt_existing_round(
            &user,
            round.round.mode,
            round.round.nsfw,
            round.round.round_no,
            round.round.mode.time_limit(round.round.round_no),
        )
        .await
    {
        respond_with_prompt(rsx, &ctx, &round, false).await?;
    } else {
        rsx.respond(|b| b.content("No rounds available currently.\nUse `/start` to play again."))
            .await?
    }

    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();
    Ok(())
}
