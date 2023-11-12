use super::GameLogic;
use chrono::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Ross;

impl GameLogic for Ross {
    fn last_round(&self) -> u64 {
        4
    }

    fn time_limit(&self, round_no: u64) -> Duration {
        if round_no < self.last_round() {
            Duration::seconds(900)
        } else {
            Duration::seconds(5200)
        }
    }

    fn prompt(&self, round_no: u64) -> &'static str {
        if round_no < self.last_round() {
            "Draw an attribute."
        } else {
            "Draw a character using the attributes."
        }
    }

    fn multiplex(&self, round_no: u64) -> u64 {
        match round_no {
            0 => 1,
            1..=4 => 1,
            _ => 0,
        }
    }
}
