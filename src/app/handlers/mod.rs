pub mod remove_asset;

use super::{AppData, AppError};
use poise::{Event, FrameworkContext};
use serenity::prelude::Context;
use std::{future::Future, pin::Pin};

pub trait AssetHandler {
    fn handle(
        &self,
        ctx: &Context,
        event: &Event<'_>,
        _fcx: FrameworkContext<'_, AppData, AppError>,
        data: &AppData,
    ) -> Option<Pin<Box<dyn Future<Output = Result<(), AppError>> + Send>>>;
}
