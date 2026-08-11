mod engine;
mod feed;
mod models;

use orderbook::L2BookDiffRequest;
use orderbook::order_book_streaming_client::OrderBookStreamingClient;
use tonic::Request;
use tonic::metadata::MetadataValue;
use tonic::transport::{Channel, ClientTlsConfig};

pub mod orderbook {
    tonic::include_proto!("hyperliquid");
}

const GRPC_ENDPOINT: &str = "";
const AUTH_TOKEN: &str = "";

async fn orderbook_client() -> Result<OrderBookStreamingClient<Channel>, Box<dyn std::error::Error>>
{
    let channel = Channel::from_shared(GRPC_ENDPOINT)?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await?;
    Ok(OrderBookStreamingClient::new(channel))
}

fn with_auth<T>(message: T) -> Result<Request<T>, Box<dyn std::error::Error>> {
    let mut request = Request::new(message);
    request
        .metadata_mut()
        .insert("x-token", AUTH_TOKEN.parse::<MetadataValue<_>>()?);
    Ok(request)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let coin = "BTC";
    let level = 5u32;
    let n_sig_figs: Option<u32> = None;
    let mantissa: Option<u64> = None;
    let skip_initial_snapshot = false;

    println!("\n{}", "=".repeat(60));
    println!("Endpoint: {}", GRPC_ENDPOINT);
    println!("Auth: {}", AUTH_TOKEN);
    println!("{}", "=".repeat(60));

    let mut client = orderbook_client().await?;

    let request = L2BookDiffRequest {
        coins: vec![coin.to_string()],
        n_levels: level,
        n_sig_figs,
        mantissa,
        skip_initial_snapshot,
    };

    let mut stream = client
        .stream_l2_book_diff(with_auth(request)?)
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
