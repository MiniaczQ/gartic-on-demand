use tokio::sync::mpsc;

pub struct StatusUpdateWaiter {
    rx: mpsc::Receiver<()>,
}

impl StatusUpdateWaiter {
    pub async fn wait(&mut self) {
        self.rx.recv().await;
    }
}

#[derive(Clone)]
pub struct StatusUpdateWaker {
    sx: mpsc::Sender<()>,
}

impl StatusUpdateWaker {
    pub fn wake(&self) {
        self.sx.try_send(()).ok();
    }
}

pub fn status_update_pair() -> (StatusUpdateWaker, StatusUpdateWaiter) {
    let (sx, rx) = mpsc::channel(1);
    (StatusUpdateWaker { sx }, StatusUpdateWaiter { rx })
}
