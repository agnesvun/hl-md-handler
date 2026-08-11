mod config;
mod engine;
mod feed;
mod models;

use orderbook::L2BookDiffRequest;
use orderbook::order_book_streaming_client::OrderBookStreamingClient;
use tonic::Request;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, ClientTlsConfig};

use crate::config::Cfg;

pub mod orderbook {
    tonic::include_proto!("hyperliquid");
}

async fn orderbook_client(
    endpoint: String,
) -> Result<OrderBookStreamingClient<Channel>, Box<dyn std::error::Error>> {
    let channel = Channel::from_shared(endpoint)?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;
    Ok(OrderBookStreamingClient::new(channel))
}

fn with_auth<T>(message: T, token: String) -> Result<Request<T>, Box<dyn std::error::Error>> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert("x-token", token.parse::<MetadataValue<_>>()?);
    Ok(request)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Cfg::load()?;
    let mut client = orderbook_client(config.grpc.endpoint).await?;

    let request = L2BookDiffRequest {
        coins: config.stream.coins,
        n_levels: config.stream.n_levels,
        n_sig_figs: None,
        mantissa: None,
        skip_initial_snapshot: false,
    };

    let mut stream = client
        .stream_l2_book_diff(with_auth(request, config.grpc.auth_token)?)
        .await?
        .into_inner();

    const MAX_MSG: u32 = 5;
    let mut msg_count = 0;

    loop {
        match stream.message().await {
            Ok(Some(update)) => {
                println!(
                    "L2 diff {} {} {}",
                    update.time, update.height, update.snapshot,
                );

                for diff in update.diffs {
                    println!("{} {} {}", diff.coin, diff.seq, diff.prev_seq,);

                    for bid in diff.bids {
                        println!("{} {} {}", bid.px, bid.sz, bid.n)
                    }

                    for ask in diff.asks {
                        println!("{} {} {}", ask.px, ask.sz, ask.n)
                    }
                }

                msg_count += 1;
                if msg_count >= MAX_MSG {
                    return Ok(());
                }
            }
            Ok(None) => break,
            Err(status) => {
                println!("{}", status);
            }
        }
    }

    Ok(())
}
