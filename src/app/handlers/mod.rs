pub mod accept_submission;
pub mod notify_activity;
pub mod remove_asset;

use super::{AppData, AppError};
use async_trait::async_trait;
use poise::{Event, FrameworkContext};
use serenity::prelude::Context;
use std::fmt::Debug;

#[async_trait]
pub trait AssetHandler: Debug + Sync + Send + 'static {
    async fn handle<'a>(
        &self,
        ctx: &Context,
        event: &Event<'a>,
        _fcx: FrameworkContext<'a, AppData, AppError>,
        data: &AppData,
    ) -> Result<(), AppError>;
}
