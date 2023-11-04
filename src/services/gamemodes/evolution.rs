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
            0 => Duration::seconds(1800),
            1 => Duration::seconds(2700),
            _ => Duration::seconds(3600),
        }
    }

    fn prompt(&self, round_no: u64) -> &'static str {
        match round_no {
            0 => "Draw the first, base evolution",
            1 => "Draw the second evolution.",
            _ => "Draw the third, final evolution.",
        }
    }

    fn multiplex(&self, _round_no: u64) -> u64 {
        1
    }
}
