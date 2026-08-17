use hl_md_handler::{
    config::Cfg,
    engine::Engine,
    feed::{Feed, HlL2BookDiff},
    orderbook::L2BookDiffUpdate,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Cfg::load()?;
    let (tx, rx) = mpsc::channel::<L2BookDiffUpdate>(1024);

    let hl_feed = HlL2BookDiff::new(config, tx);

    let producer = tokio::spawn(async move {
        let stream = match hl_feed.initialise().await {
            Ok(stream) => stream,
            Err(e) => {
                // eprintln!("feed initialise failed: {}", e);
                return;
            }
        };

        if let Err(e) = hl_feed.produce(stream).await {
            // eprintln!("producer failed: {}", e);
        }
    });
    
    let mut engine = Engine::new(rx);
    engine.run().await;

    producer.await?;

    Ok(())
}
