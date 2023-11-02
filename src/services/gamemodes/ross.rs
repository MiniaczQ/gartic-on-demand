use super::GameLogic;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Ross;

impl GameLogic for Ross {
    fn last_round(&self) -> u64 {
        4
    }

    fn time_limit(&self, round: u64) -> Duration {
        if round < self.last_round() {
            Duration::from_secs(900)
        } else {
            Duration::from_secs(5200)
        }
    }

    fn prompt(&self, round: u64) -> &'static str {
        if round < self.last_round() {
            "Draw an attribute."
        } else {
            "Draw a character using the attributes."
        }
    }

    fn multiplex(&self, round: u64) -> u64 {
        match round {
            0 => 1,
            1..=4 => 2,
            _ => 0,
        }
    }
}
