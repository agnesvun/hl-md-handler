use crate::config::Cfg;
use crate::orderbook::order_book_streaming_client::OrderBookStreamingClient;
use crate::orderbook::{L2BookDiffRequest, L2BookDiffUpdate};
use std::error::Error;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::error::TrySendError;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, ClientTlsConfig};
use tonic::{Request, Streaming};
use tracing::error;

pub type BoxError = Box<dyn Error + Send + Sync>;

pub trait Feed<T> {
    type Stream;
    fn new(config: Cfg, tx: Sender<T>) -> Self;
    fn initialise(
        &self,
    ) -> impl std::future::Future<Output = Result<Self::Stream, BoxError>> + Send;
    fn produce(
        &self,
        stream: Self::Stream,
    ) -> impl std::future::Future<Output = Result<(), BoxError>> + Send;
}

async fn orderbook_client(endpoint: String) -> Result<OrderBookStreamingClient<Channel>, BoxError> {
    let channel = Channel::from_shared(endpoint)?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .http2_keep_alive_interval(Duration::from_secs(20))
        .tcp_nodelay(true)
        .connect()
        .await?;
    Ok(OrderBookStreamingClient::new(channel))
}

fn with_auth<T>(message: T, token: String) -> Result<Request<T>, BoxError> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert("x-token", token.parse::<MetadataValue<_>>()?);
    Ok(request)
}

pub struct HlL2BookDiff {
    config: Cfg,
    tx: Sender<L2BookDiffUpdate>,
}

impl Feed<L2BookDiffUpdate> for HlL2BookDiff {
    type Stream = Streaming<L2BookDiffUpdate>;

    fn new(config: Cfg, tx: Sender<L2BookDiffUpdate>) -> Self {
        HlL2BookDiff { config, tx }
    }

    async fn initialise(&self) -> Result<Self::Stream, BoxError> {
        let mut client = orderbook_client(self.config.grpc.endpoint.clone()).await?;

        let request = L2BookDiffRequest {
            coins: self.config.feed.coins.clone(),
            n_levels: 20,
            n_sig_figs: None,
            mantissa: None,
            skip_initial_snapshot: false,
        };

        let stream = client
            .stream_l2_book_diff(with_auth(request, self.config.grpc.auth_token.clone())?)
            .await?
            .into_inner();

        Ok(stream)
    }

    async fn produce(&self, mut stream: Self::Stream) -> Result<(), BoxError> {
        loop {
            match stream.message().await {
                Ok(Some(update)) => match self.tx.try_send(update) {
                    Ok(()) => {}
                    Err(TrySendError::Full(update)) => {
                        error!("Queue is full, drop update: time={}", update.time);
                    }
                    Err(TrySendError::Closed(update)) => {
                        return Err(TrySendError::Closed(update).into());
                    }
                },
                Ok(None) => break,
                Err(status) => {
                    return Err(status.into());
                }
            }
        }

        Ok(())
    }
}
