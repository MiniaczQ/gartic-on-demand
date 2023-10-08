use serde::Deserialize;
use tracing_subscriber::{
    fmt, prelude::__tracing_subscriber_SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
};

use crate::app::config::CONFIG;

pub fn setup() {
    registry()
        .with(fmt::layer().pretty().with_thread_names(true))
        .with(EnvFilter::new(&CONFIG.log.directives))
        .init();
}

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    directives: String,
}
