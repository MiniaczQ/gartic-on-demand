use serenity::{
    model::prelude::{Message, Reaction},
    prelude::{Context, EventHandler},
};

use crate::{config::CONFIG, database::Database};

use self::assets::AssetsHandler;

pub mod assets;

pub struct AppContext {
    pub db: Database,
}

impl AppContext {
    pub async fn setup() -> Self {
        let db = CONFIG.database.connect().await;
        Self { db }
    }
}

pub struct AppHandler {
    acx: AppContext,
}

impl AppHandler {
    pub fn new(acx: AppContext) -> Self {
        Self { acx }
    }
}

#[serenity::async_trait]
impl EventHandler for AppHandler {
    async fn reaction_add(&self, scx: Context, add_reaction: Reaction) {
        AssetsHandler::reaction_add(&self.acx, &scx, &add_reaction).await;
    }

    async fn message(&self, scx: Context, msg: Message) {
        AssetsHandler::message(&self.acx, &scx, &msg).await;

        //msg.is_private()
        //CONFIG.guild.member(&scx, &msg.author).await.is_err()
    }
}
