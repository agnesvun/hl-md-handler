use crate::config::Cfg;
use std::error::Error;
use tokio::sync::mpsc::Sender;

pub type BoxError = Box<dyn Error + Send + Sync>;

pub trait Feed<T> {
    type Stream;
    fn new(config: Cfg, tx: Sender<T>) -> Self;
    async fn initialise(&self) -> Result<Self::Stream, BoxError>;
    async fn produce(&self, stream: Self::Stream) -> Result<(), BoxError>;
}
