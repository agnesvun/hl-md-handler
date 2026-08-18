use hl_md_handler::{
    config::Cfg,
    engine::Engine,
    feed::{Feed, HlL2BookDiff},
    orderbook::L2BookDiffUpdate,
};
use tokio::{signal, sync::mpsc};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // log to stdout
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());
    tracing_subscriber::fmt()
        .with_writer(non_blocking_writer)
        .init();

    info!("App started");

    let config = match Cfg::load() {
        Ok(cfg) => {
            info!("Loaded config: coins={:?}", cfg.feed.coins);
            cfg
        }
        Err(e) => {
            error!("Failed to load config: {}", e);
            return Err(e);
        }
    };

    let (tx, rx) = mpsc::channel::<L2BookDiffUpdate>(64);

    let feed = HlL2BookDiff::new(config, tx);
    let producer = tokio::spawn(async move {
        let stream = match feed.initialise().await {
            Ok(stream) => stream,
            Err(e) => {
                error!("Failed to initialise feed: {}", e);
                return;
            }
        };

        if let Err(e) = feed.produce(stream).await {
            error!("Producer error: {}", e);
        }
    });

    // consumer
    let mut engine = Engine::new(rx);

    tokio::select! {
        _ = engine.run() => {},
        _ = signal::ctrl_c() => {
            info!("App shutting down...");
        },
    }

    producer.abort();

    Ok(())
}
