pub mod ross;

use self::ross::Ross;
use chrono::Duration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSession {
    pub mode: Mode,
    pub images: Vec<u64>,
}

impl GameSession {
    pub fn new(mode: Mode) -> Self {
        Self {
            images: vec![],
            mode,
        }
    }

    pub fn round(&self) -> u64 {
        self.images.len() as u64
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Mode {
    Ross,
}

impl GameLogic for Mode {
    fn last_round(&self) -> u64 {
        match self {
            Mode::Ross => Ross.last_round(),
        }
    }

    fn time_limit(&self, round_no: u64) -> Duration {
        match self {
            Mode::Ross => Ross.time_limit(round_no),
        }
    }

    fn prompt(&self, round_no: u64) -> &'static str {
        match self {
            Mode::Ross => Ross.prompt(round_no),
        }
    }

    fn multiplex(&self, round_no: u64) -> u64 {
        match self {
            Mode::Ross => Ross.multiplex(round_no),
        }
    }
}

pub trait GameLogic {
    fn last_round(&self) -> u64;
    fn time_limit(&self, round_no: u64) -> Duration;
    fn prompt(&self, round_no: u64) -> &'static str;
    fn multiplex(&self, round_no: u64) -> u64;
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {}
