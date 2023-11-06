use crate::app::{config::CONFIG, error::ConvertError};

use super::error::AppError;
use chrono::{DateTime, Duration, Utc};
use gartic_on_demand::services::{
    database::{attempt::AttemptRepository, Database, ThingToU64},
    provider::Provider,
};
use poise::serenity_prelude::UserId;
use serde::Deserialize;
use serde_with::{serde_as, DurationSeconds};
use serenity::prelude::Context;
use tracing::{error, info};

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct ExpiryNotifierConfig {
    #[serde_as(as = "DurationSeconds<u64>")]
    check_interval: std::time::Duration,
    #[serde_as(as = "DurationSeconds<i64>")]
    in_advance: Duration,
}

pub struct ExpiryNotifier {
    ar: AttemptRepository,
    ctx: Context,
}

impl ExpiryNotifier {
    pub fn new(db: Database, ctx: Context) -> Self {
        Self { ar: db.get(), ctx }
    }

    pub async fn run(mut self) {
        loop {
            if let Err(e) = self.run_internal().await {
                error!(error = %e, "Expiry notifier error");
            }
        }
    }

    pub async fn run_internal(&mut self) -> Result<(), AppError> {
        info!("Starting expiry notifier");
        let mut last_check = Utc::now();
        loop {
            self.loop_body(&mut last_check).await?;
        }
    }

    async fn loop_body(&mut self, last_check: &mut DateTime<Utc>) -> Result<(), AppError> {
        info!("Updating stats printer");
        let now = Utc::now();

        let after = *last_check - CONFIG.expiry_notifier.in_advance;
        let until = now - CONFIG.expiry_notifier.in_advance;

        let active = self
            .ar
            .get_active_between(after, until)
            .await
            .map_internal("Failed to get active attempts")?;

        let users = active
            .into_iter()
            .map(|a| (UserId(a.who.to_u64()), a.state.until));
        for (user, when) in users {
            let dms = user.create_dm_channel(&self.ctx).await?;
            dms.send_message(&self.ctx, |b| {
                b.content(format!("Your attempt will soon expire: <t:{}:R>", when))
            })
            .await?;
        }

        *last_check = now;
        tokio::time::sleep(CONFIG.expiry_notifier.check_interval).await;
        Ok(())
    }
}
