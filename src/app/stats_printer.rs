use std::time::Duration;

use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::Message;
use rossbot::services::{
    database::{
        attempt::AttemptRepository,
        stats::{ActiveUser, StatsRepository, UnallocatedRound},
        Database,
    },
    gamemodes::GameLogic,
    provider::Provider,
    status_update::StatusUpdateWaiter,
};
use serenity::prelude::Context;
use tracing::{error, info};

pub struct StatsPrinter {
    sr: StatsRepository,
    ar: AttemptRepository,
    sw: StatusUpdateWaiter,
    ctx: Context,
}

enum Activity {
    Active(Message),
    Cooldown(DateTime<Utc>),
    None,
}

impl StatsPrinter {
    pub fn new(db: Database, sw: StatusUpdateWaiter, ctx: Context) -> Self {
        Self {
            sr: db.get(),
            ar: db.get(),
            sw,
            ctx,
        }
    }

    pub async fn run(mut self) {
        loop {
            if let Err(e) = self.run_internal().await {
                error!(error = %e, "Stats printer error");
            }
        }
    }

    pub async fn run_internal(&mut self) -> Result<(), AppError> {
        info!("Starting stats printer");
        let channel = CONFIG.channels.stats;
        let messages = channel.messages(&self.ctx, |b| b).await?;
        for message in messages {
            message.delete(&self.ctx).await?;
        }
        let mut message = channel
            .send_message(&self.ctx, |b| b.embed(|b| b.description("Setting up...")))
            .await?;

        let mut activity = Activity::Cooldown(Self::cooldown());

        loop {
            self.loop_body(&mut activity, &mut message).await?;
        }
    }

    async fn loop_body(
        &mut self,
        activity: &mut Activity,
        message: &mut Message,
    ) -> Result<(), AppError> {
        info!("Updating stats printer");
        self.ar
            .expire_active_attempts()
            .await
            .map_internal("Failed to stop expired sessions")?;

        let active = self.active_users(activity).await?;
        let incomplete = self.unallocated_rounds().await?;

        let response = format!(
            "**Active users:**\n{}\n\n**Incomplete games:**\n{}",
            active, incomplete
        );
        message
            .edit(&self.ctx, |b| {
                b.embed(|b| {
                    b.title(format!("Status update <t:{}>", Utc::now().timestamp()))
                        .description(response)
                })
            })
            .await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        self.sw.wait().await;
        Ok(())
    }

    async fn unallocated_rounds(&mut self) -> Result<String, AppError> {
        let mut unallocated = self
            .sr
            .get_unallocated_rounds()
            .await
            .map_internal("Failed to fetch incomplete games")?;
        unallocated.retain(|u| u.round_no > 0 && u.round_no <= u.mode.last_round());
        let mut unallocated = unallocated
            .iter()
            .map(Self::unallocated_round_to_string)
            .collect::<Vec<_>>()
            .join("\n");
        if unallocated.is_empty() {
            unallocated.push_str("None");
        }
        Ok(unallocated)
    }

    fn unallocated_round_to_string(round: &UnallocatedRound) -> String {
        format!(
            "- {}{:?} mode round {} - available {}",
            if round.nsfw { "NSFW " } else { "" },
            round.mode,
            round.round_no + 1,
            round.unallocated
        )
    }

    async fn active_users(&mut self, activity: &mut Activity) -> Result<String, AppError> {
        let active = self
            .sr
            .get_active_users()
            .await
            .map_internal("Failed to fetch active users")?;
        self.update_activity(!active.is_empty(), activity).await?;
        let mut active = active
            .iter()
            .map(Self::active_user_to_string)
            .collect::<Vec<_>>()
            .join("\n");
        if active.is_empty() {
            active.push_str("None");
        }
        Ok(active)
    }

    fn active_user_to_string(user: &ActiveUser) -> String {
        format!(
            "- <@{}> - {}{:?} mode round {}",
            user.user.id(),
            if user.round.nsfw { "NSFW " } else { "" },
            user.round.mode,
            user.round.round_no + 1
        )
    }

    fn cooldown() -> DateTime<Utc> {
        Utc::now() + Duration::from_secs(CONFIG.notify.cooldown)
    }

    async fn update_activity(
        &mut self,
        active: bool,
        activity: &mut Activity,
    ) -> Result<(), AppError> {
        match (active, &mut *activity) {
            (true, Activity::None) => {
                let now = Utc::now();
                let content = format!(
                    "Activity detected at <t:{}> <@&{}>!",
                    now.timestamp(),
                    CONFIG.roles.notify
                );
                let message = CONFIG
                    .channels
                    .stats
                    .send_message(&self.ctx, |b| b.content(content))
                    .await?;
                *activity = Activity::Active(message);
            }
            (true, Activity::Cooldown(until)) => {
                if Utc::now() > *until {
                    *activity = Activity::None;
                } else {
                    *until = Self::cooldown();
                }
            }
            (false, Activity::Active(message)) => {
                message.delete(&self.ctx).await?;
                *activity = Activity::Cooldown(Self::cooldown());
            }
            _ => {}
        };
        Ok(())
    }
}
