use std::path::PathBuf;

use serde::Deserialize;
use tracing_appender::rolling;
use tracing_subscriber::{
    fmt, prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};

use crate::app::config::CONFIG;

pub fn setup() {
    let json = CONFIG.log.json.as_ref().map(|json| {
        let writer = rolling::hourly(json, "logs");
        fmt::layer().json().with_writer(writer)
    });
    let console = CONFIG.log.console.then_some(fmt::layer().pretty());
    registry()
        .with(json)
        .with(console)
        .with(EnvFilter::new(&CONFIG.log.directives))
        .init();
}

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    directives: String,
    console: bool,
    json: Option<PathBuf>,
}
