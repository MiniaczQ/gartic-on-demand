use std::time::Duration;

use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
};
use chrono::{DateTime, Utc};
use poise::serenity_prelude::Message;
use rossbot::services::{
    database::{session::SessionRepository, Database},
    provider::Provider,
    status_update::StatusUpdateWaiter,
};
use serenity::prelude::Context;
use tracing::{error, info};

pub struct StatsPrinter {
    sr: SessionRepository,
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
        let sr = db.get();
        Self { sr, sw, ctx }
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

        let mut activity = Activity::None;

        loop {
            info!("Updating stats printer");
            self.sr
                .stop_expired()
                .await
                .map_internal("Failed to stop expired sessions")?;

            let active = self
                .sr
                .active_users()
                .await
                .map_internal("Failed to fetch active users")?;

            match (active.is_empty(), &activity) {
                (false, Activity::None) => {
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
                    activity = Activity::Active(message);
                }
                (true, Activity::Active(message)) => {
                    message.delete(&self.ctx).await?;
                    activity = Activity::Cooldown(Utc::now() + Duration::from_secs(300));
                }
                (false, Activity::Cooldown(until)) => {
                    if Utc::now() > *until {
                        activity = Activity::None;
                    }
                }
                _ => {}
            }

            let mut active = active
                .iter()
                .map(|u| format!("<@{}>", u))
                .collect::<Vec<_>>()
                .join(", ");

            if active.is_empty() {
                active.push_str("None");
            }

            let mut incomplete = self
                .sr
                .incomplete_games()
                .await
                .map_internal("Failed to fetch incomplete games")?
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            if incomplete.is_empty() {
                incomplete.push_str("None");
            }

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
        }
    }
}
