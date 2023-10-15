use std::time::Duration;

use chrono::Utc;
use rossbot::services::{
    database::{session::SessionRepository, Database},
    provider::Provider,
};
use serenity::prelude::Context;
use tokio::time::interval;
use tracing::{error, info};

use super::{
    config::CONFIG,
    error::{AppError, ConvertError},
};

pub struct StatsPrinter {
    sr: SessionRepository,
    ctx: Context,
}

impl StatsPrinter {
    pub fn new(db: Database, ctx: Context) -> Self {
        let sr = db.get();
        Self { sr, ctx }
    }

    pub async fn run(self) {
        loop {
            if let Err(e) = self.run_internal().await {
                error!(error = %e, "Stats printer error");
            }
        }
    }

    pub async fn run_internal(&self) -> Result<(), AppError> {
        info!("Starting stats printer");
        let mut ticker = interval(Duration::from_secs(60));
        let channel = CONFIG.channels.stats;
        let messages = channel.messages(&self.ctx, |b| b).await?;
        for message in messages {
            message.delete(&self.ctx).await?;
        }
        let mut message = channel
            .send_message(&self.ctx, |b| b.embed(|b| b.description("Setting up...")))
            .await?;

        loop {
            info!("Updating stats printer");
            self.sr
                .stop_expired()
                .await
                .map_internal("Failed to stop expired sessions")?;

            let mut active = self
                .sr
                .active_users()
                .await
                .map_internal("Failed to fetch active users")?
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
            ticker.tick().await;
        }
    }
}
