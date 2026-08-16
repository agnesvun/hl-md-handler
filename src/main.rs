use hl_md_handler::{
    config::Cfg, feed::{Feed, HlL2BookDiff}, orderbook::L2BookDiffUpdate,
};
use tokio::sync::mpsc;

const CHANNEL_CAP: usize = 1024;

async fn consume(mut rx: mpsc::Receiver<L2BookDiffUpdate>) {
    while let Some(update) = rx.recv().await {
        println!(
            "L2 diff time={} height={} snapshot={} coins={}",
            update.time,
            update.height,
            update.snapshot,
            update.diffs.len(),
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Cfg::load()?;
    let (tx, rx) = mpsc::channel::<L2BookDiffUpdate>(CHANNEL_CAP);

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
            eprintln!("producer failed: {}", e);
        }
    });

    consume(rx).await;

    producer.await?;

    Ok(())
}
