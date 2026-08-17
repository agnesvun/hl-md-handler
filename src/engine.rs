use crate::{
    book::{BookStatus, OrderBook, Side},
    orderbook::{L2BookDiffUpdate, L2CoinDiff, L2Level},
};
use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

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
            println!(
                "L2 diff time={} height={} snapshot={} coins={}",
                update.time,
                update.height,
                update.snapshot,
                update.diffs.len(),
            );
            self.on_update(update);
        }
    }

    fn on_update(&mut self, update: L2BookDiffUpdate) {
        for diff in update.diffs {
            self.on_diff(diff);
        }
    }

    fn on_diff(&mut self, diff: L2CoinDiff) {
        let book = self.books.entry(diff.coin).or_default();

        // whenever a snapshot is received, reset the book and start over
        if diff.snapshot {
            book.reset();
        }

        if !book.check_seq(diff.prev_seq) {
            // out of sequence
            return;
        }

        Self::apply_levels(book, &diff.bids, &diff.asks);
        book.update_seq(diff.seq);

        // println!(
        //     "update seq={}, {}/{}",
        //     book.seq, book.bids.levels[0].px, book.asks.levels[0].px
        // );
    }

    fn apply_levels(book: &mut OrderBook, bids: &[L2Level], asks: &[L2Level]) {
        for lv in bids {
            book.apply(
                Side::Bid,
                Engine::parse_scaled(&lv.px),
                Engine::parse_scaled(&lv.sz),
            );
        }

        for lv in asks {
            book.apply(
                Side::Ask,
                Engine::parse_scaled(&lv.px),
                Engine::parse_scaled(&lv.sz),
            );
        }
    }

    #[inline(always)]
    pub fn parse_scaled(s: &str) -> u64 {
        let mut acc: u64 = 0;
        let mut bytes = s.bytes();

        while let Some(b) = bytes.next() {
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
