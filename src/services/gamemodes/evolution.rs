use super::GameLogic;
use chrono::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Evolution;

impl GameLogic for Evolution {
    fn last_round(&self) -> u64 {
        2
    }

    fn time_limit(&self, round_no: u64) -> Duration {
        match round_no {
            0 => Duration::seconds(600),
            1 => Duration::seconds(900),
            _ => Duration::seconds(1200),
        }
    }

    fn prompt(&self, round_no: u64) -> &'static str {
        match round_no {
            0 => "Draw the unevolved entity. (Keep it simple)",
            1 => "Draw the first evolution.",
            _ => "Draw the final evolution.",
        }
    }

    fn multiplex(&self, _round_no: u64) -> u64 {
        1
    }
}
