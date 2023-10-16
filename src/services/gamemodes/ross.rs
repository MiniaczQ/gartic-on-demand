use crate::services::database::session::SubmissionKind;

use super::GameLogic;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Ross;

pub const LAST_ROUND: u64 = 4;

impl GameLogic for Ross {
    fn last_round(&self) -> u64 {
        LAST_ROUND
    }

    fn time_limit(&self, round: u64) -> Duration {
        if round < LAST_ROUND {
            Duration::from_secs(900)
        } else {
            Duration::from_secs(5200)
        }
    }

    fn submission_kind(&self, round: u64) -> SubmissionKind {
        if round < LAST_ROUND {
            SubmissionKind::Partial
        } else {
            SubmissionKind::Complete
        }
    }

    fn prompt(&self, round: u64) -> &'static str {
        if round < LAST_ROUND {
            "Draw an attribute."
        } else {
            "Draw a character using the attributes."
        }
    }
}
