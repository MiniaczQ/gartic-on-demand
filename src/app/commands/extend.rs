use crate::app::{
    error::ConvertError, response::ResponseContext, util::respond_with_prompt, AppContext, AppError,
};
use gartic_on_demand::services::{
    database::{attempt::AttemptRepository, round::RoundRepository, user::UserRepository},
    gamemodes::GameLogic,
    provider::Provider,
};
use tracing::error;

/// Reset the expiry timer on current game session
#[poise::command(slash_command, guild_only)]
pub async fn extend(ctx: AppContext<'_>) -> Result<(), AppError> {
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
    let user = ctx.author();
    let user = ur
        .create_or_update_user(user.id.0, &user.name)
        .await
        .map_internal("Failed to update user")?;

    let round = rr
        .get_active_round(&user)
        .await
        .map_user("No active game session")?;

    let round = ar
        .extend_active_attempt(&user, round.round.mode.time_limit(round.round.round_no))
        .await
        .map_internal("Failed to extend timer")?;

    respond_with_prompt(rsx, &ctx, &round, true).await?;
    Ok(())
}
