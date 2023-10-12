use super::{AppData, AppError};
use poise::{Context, CreateReply, ReplyHandle};

pub struct ResponseContext<'a, U = AppData, E = AppError> {
    ctx: Context<'a, U, E>,
    handle: Option<ReplyHandle<'a>>,
}

impl<'a, U, E> ResponseContext<'a, U, E> {
    pub fn new(ctx: Context<'a, U, E>) -> Self {
        Self { ctx, handle: None }
    }

    pub async fn respond<'att>(
        &mut self,
        builder: impl for<'b> FnOnce(&'b mut CreateReply<'att>) -> &'b mut CreateReply<'att>,
    ) -> Result<(), serenity::Error> {
        match &self.handle {
            None => {
                self.handle = Some(
                    self.ctx
                        .send(|b| {
                            b.ephemeral(true);
                            builder(b)
                        })
                        .await?,
                );
                Ok(())
            }
            Some(handle) => handle.edit(self.ctx, builder).await,
        }
    }

    pub fn reset(&mut self) {
        self.handle.take();
    }

    pub async fn finalize(&mut self) -> Result<(), serenity::Error> {
        if self.handle.is_none() {
            self.respond(|b| b).await?;
        }
        Ok(())
    }
}
