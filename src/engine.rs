use crate::{
    book::{BookStatus, OrderBook, Side},
    orderbook::{L2BookDiffUpdate, L2CoinDiff, L2Level},
};
use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;
use tracing::{error, info};

const MULTIPLIER: u64 = 1_000_000;

pub struct Engine {
    rx: Receiver<L2BookDiffUpdate>,
    books: HashMap<String, OrderBook>,
}

impl Engine {
    pub fn new(rx: Receiver<L2BookDiffUpdate>) -> Self {
        Self {
            rx,
            books: HashMap::new(),
        }
    }

    pub async fn run(&mut self) {
        while let Some(update) = self.rx.recv().await {
            self.on_update(update);
        }
    }

    fn on_update(&mut self, update: L2BookDiffUpdate) {
        for diff in update.diffs {
            self.on_diff(diff, update.time);
        }
    }

    fn on_diff(&mut self, diff: L2CoinDiff, ts_ms: u64) {
        let book = match self.books.get_mut(&diff.coin) {
            Some(book) => book,
            None => self.books.entry(diff.coin.clone()).or_default(),
        };

        // whenever a snapshot is received, reset the book and start over
        if diff.snapshot {
            book.reset();
        }

        if !book.is_in_seq(diff.prev_seq) {
            book.set_status(BookStatus::Error);
            error!(
                "Gap detected, book_seq ({}) != diff.prev_seq ({})",
                book.seq(),
                diff.prev_seq
            );
            return;
        }

        Self::apply_levels(book, &diff.bids, &diff.asks);
        book.set_seq(diff.seq);
        book.set_status(BookStatus::Active);
        book.set_ts(ts_ms);

        Self::publish(&diff.coin, book);
    }

    fn apply_levels(book: &mut OrderBook, bids: &[L2Level], asks: &[L2Level]) {
        for lv in bids {
            book.apply(
                Side::Bid,
                Engine::parse_to_u64_with_mul(&lv.px),
                Engine::parse_to_u64_with_mul(&lv.sz),
            );
        }

        for lv in asks {
            book.apply(
                Side::Ask,
                Engine::parse_to_u64_with_mul(&lv.px),
                Engine::parse_to_u64_with_mul(&lv.sz),
            );
        }
    }

    // dummy publish
    fn publish(coin: &str, book: &OrderBook) {
        info!("Publish (mock): coin={} book={}", coin, book);
    }

    #[inline(always)]
    pub fn parse_to_u64_with_mul(s: &str) -> u64 {
        let mut acc: u64 = 0;
        let mut bytes = s.bytes();

        for b in bytes.by_ref() {
            if b == b'.' {
                break;
            }
            acc = acc * 10 + (b & 0x0F) as u64;
        }
        acc *= MULTIPLIER;

        let mut w = MULTIPLIER / 10;
        for b in bytes {
            if w == 0 {
                break;
            }
            acc += (b & 0x0F) as u64 * w;
            w /= 10;
        }
        acc
    }
}
