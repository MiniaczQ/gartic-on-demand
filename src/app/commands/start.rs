use crate::app::{
    error::{AppError, ConvertError},
    permission::is_adult,
    response::ResponseContext,
    util::respond_with_prompt,
    AppContext,
};
use rossbot::services::{
    database::{
        attempt::{Active, AttemptRepository},
        round::{RoundRepository, RoundWithAttempts},
        user::{User, UserRepository},
        DbError, Record,
    },
    gamemodes::{GameLogic, Mode},
    provider::Provider,
    status_update::StatusUpdateWaker,
};
use std::ops::Sub;
use tracing::error;

#[derive(Debug, poise::ChoiceParameter)]
pub enum GameArg {
    Ross,
}

/// Start a new game session, by default from round 1
#[poise::command(slash_command, guild_only)]
pub async fn start(
    ctx: AppContext<'_>,
    #[description = "Game mode you want to play"] mode: GameArg,
    #[description = "Play the NSFW variant (+18 only)"] nsfw: Option<bool>,
    //#[description = "Round to start at"]#[min = 1] round: Option<u64>,
) -> Result<(), AppError> {
    let mut rsx = ResponseContext::new(ctx);
    rsx.init().await?;
    if let Err(e) = process(&mut rsx, ctx, mode, None, nsfw).await {
        error!(error = ?e);
        rsx.respond(|b| b.content(e.for_user())).await?
    }
    rsx.finalize().await?;
    Ok(())
}

async fn process(
    rsx: &mut ResponseContext<'_>,
    ctx: AppContext<'_>,
    mode: GameArg,
    round: Option<u64>,
    nsfw: Option<bool>,
) -> Result<(), AppError> {
    let round_no = round.unwrap_or(1).sub(1);
    let nsfw = nsfw.unwrap_or(false);

    let ar: AttemptRepository = ctx.data().get();
    let ur: UserRepository = ctx.data().get();
    let rr: RoundRepository = ctx.data().get();
    let user = ctx.author();

    if nsfw && !is_adult(&ctx, user).await? {
        rsx.respond(|b| b.content("You need the `+18` role to participate in NSFW games."))
            .await?;
        return Ok(());
    }

    let user = ur
        .create_or_update_user(user.id.0, &user.name)
        .await
        .map_internal("Failed to update user")?;

    ar.expire_active_attempts()
        .await
        .map_internal("Failed to unlock expired sessions")?;
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();

    let maybe_lobby = rr.get_active_round(&user).await;
    if let Ok(lobby) = maybe_lobby {
        return respond_with_prompt(rsx, &ctx, &lobby, true).await;
    }

    let lobby = find_or_create_session(rr, &user, mode, round_no, nsfw).await?;
    respond_with_prompt(rsx, &ctx, &lobby, false).await?;
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();
    Ok(())
}

async fn find_or_create_session(
    rr: RoundRepository,
    user: &Record<User>,
    mode: GameArg,
    round_no: u64,
    nsfw: bool,
) -> Result<RoundWithAttempts<Active>, AppError> {
    let mode = map_game(mode);

    if round_no > mode.last_round() {
        None.map_user("Gamemode does not support this many rounds")?;
    }

    let time_limit = mode.time_limit(round_no);

    let maybe_lobby = rr
        .attempt_existing_round(user, mode, nsfw, round_no, time_limit)
        .await;
    let round = match (maybe_lobby, round_no) {
        (Ok(lobby), _) => lobby,
        (Err(DbError::NotFound), 0) => rr
            .attempt_new_round(user, mode, nsfw, mode.multiplex(round_no), time_limit)
            .await
            .map_internal("Failed to create game session")?,
        (Err(e), _) => Err(e).map_user("Did not find pending sessions")?,
    };

    Ok(round)
}

fn map_game(mode: GameArg) -> Mode {
    match mode {
        GameArg::Ross => Mode::Ross,
    }
}
