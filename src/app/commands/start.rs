use crate::app::{
    error::{AppError, ConvertError},
    permission::is_adult,
    response::ResponseContext,
    util::respond_with_prompt,
    AppContext,
};
use rossbot::services::{
    database::{
        session::{Active, LobbyWithSessions, SessionRepository},
        DbError,
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
    //#[description = "Whether to find games where you already played"] allow_repeats: Option<bool>,
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
    let sr: SessionRepository = ctx.data().get();
    let user = ctx.author();
    let uid = user.id.0;
    let round = round.unwrap_or(1).sub(1);
    let nsfw = nsfw.unwrap_or(false);

    if nsfw && !is_adult(&ctx, user).await? {
        rsx.respond(|b| b.content("You need the `+18` role to participate in NSFW games."))
            .await?;
        return Ok(());
    }

    sr.ensure_user(uid, &user.name)
        .await
        .map_internal("Failed to create user")?;

    sr.stop_expired()
        .await
        .map_internal("Failed to unlock expired sessions")?;
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();

    let maybe_lobby = sr.get(uid).await;
    if let Ok(lobby) = maybe_lobby {
        return respond_with_prompt(rsx, &ctx, &lobby, true).await;
    }

    let lobby = find_or_create_session(sr, uid, mode, round, nsfw).await?;
    respond_with_prompt(rsx, &ctx, &lobby, false).await?;
    let waker: StatusUpdateWaker = ctx.data().get();
    waker.wake();
    Ok(())
}

async fn find_or_create_session(
    sr: SessionRepository,
    uid: u64,
    mode: GameArg,
    round: u64,
    nsfw: bool,
) -> Result<LobbyWithSessions<Active>, AppError> {
    let mode = map_game(mode);

    if round > mode.last_round() {
        None.map_user("Gamemode does not support this many rounds")?;
    }

    let maybe_lobby = sr.find_attach(uid, mode, round, nsfw, false).await;
    let lobby = match (maybe_lobby, round) {
        (Ok(lobby), _) => lobby,
        (Err(DbError::NotFound), 0) => sr
            .create_attach(uid, mode, nsfw)
            .await
            .map_internal("Failed to create game session")?,
        (e, _) => e.map_user("Did not find pending sessions")?,
    };

    Ok(lobby)
}

fn map_game(mode: GameArg) -> Mode {
    match mode {
        GameArg::Ross => Mode::Ross,
    }
}
