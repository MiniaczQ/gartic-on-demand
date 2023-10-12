pub mod ross;

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameSession {
    pub images: Vec<u64>,
    pub mode: Gamemode,
    pub active: bool,
}

impl GameSession {
    pub fn new(mode: Gamemode) -> Self {
        Self {
            images: vec![],
            mode,
            active: true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Gamemode {
    Ross,
}

impl Gamemode {
    pub fn round_time(&self, round: u64) -> Duration {
        match self {
            Gamemode::Ross => {
                if round < 4 {
                    Duration::from_secs(900)
                } else {
                    Duration::from_secs(5200)
                }
            }
        }
    }
}

pub trait GamemodeLogic {
    fn is_last_round(&self, round: u32) -> bool;
    fn get_round_info(&self, round: u32) -> RoundInfo;
}

pub struct RoundInfo {
    pub time_limit: Duration,
    pub prompt: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GameError {}
