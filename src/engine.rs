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

        if book.status() == BookStatus::Error && !diff.snapshot {
            error!(
                "{} book is in Error state, skipping diff, seq={}",
                diff.coin, diff.seq
            );
            return;
        }

        // whenever a snapshot is received, reset the book and start over
        if diff.snapshot {
            book.reset();
        }

        if !book.is_in_seq(diff.prev_seq) {
            book.set_status(BookStatus::Error);
            error!(
                "Gap detected for {}, book_seq ({}) != diff.prev_seq ({})",
                diff.coin,
                book.seq(),
                diff.prev_seq
            );
            return;
        }

        Self::apply_levels(book, &diff.bids, &diff.asks);
        book.set_seq(diff.seq);
        book.set_ts_ms(ts_ms);

        if book.is_crossed() {
            book.set_status(BookStatus::Error);
            error!(
                "Book is crossed for {}, seq={} best_bid ({}) >= best_ask ({})",
                diff.coin,
                diff.seq,
                book.best_bid_level().map(|(px, _)| px).unwrap_or_default(),
                book.best_ask_level().map(|(px, _)| px).unwrap_or_default()
            );
            return;
        }

        book.set_status(BookStatus::Active);

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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn test_engine() -> Engine {
        let (_tx, rx) = mpsc::channel(1);
        Engine::new(rx)
    }

    fn test_l2_level(px: &str, sz: &str) -> L2Level {
        L2Level {
            px: px.to_string(),
            sz: sz.to_string(),
            n: 1,
        }
    }

    fn test_l2_coin_diff(
        coin: &str,
        seq: u64,
        prev_seq: u64,
        snapshot: bool,
        bids: Vec<L2Level>,
        asks: Vec<L2Level>,
    ) -> L2CoinDiff {
        L2CoinDiff {
            coin: coin.to_string(),
            seq,
            prev_seq,
            bids,
            asks,
            snapshot,
        }
    }

    fn test_l2_book_diff_update(time: u64, diffs: Vec<L2CoinDiff>) -> L2BookDiffUpdate {
        L2BookDiffUpdate {
            time,
            height: time,
            snapshot: diffs.iter().any(|d| d.snapshot),
            diffs,
        }
    }

    fn init_test_engine_with_snapshot() -> Engine {
        let mut e = test_engine();
        e.on_update(test_l2_book_diff_update(
            1_000,
            vec![test_l2_coin_diff(
                "BTC",
                1,
                0,
                true,
                vec![test_l2_level("100.5", "2"), test_l2_level("100.4", "3")],
                vec![test_l2_level("100.6", "1"), test_l2_level("100.7", "4")],
            )],
        ));

        e
    }

    impl Engine {
        fn book(&self, coin: &str) -> &OrderBook {
            self.books.get(coin).expect("book missing for coin")
        }
    }

    #[test]
    fn test_out_of_seq() {
        let mut engine = init_test_engine_with_snapshot();

        engine.on_update(test_l2_book_diff_update(
            1_002,
            vec![test_l2_coin_diff(
                "BTC",
                3,
                2,
                false,
                vec![test_l2_level("100.5", "9")],
                vec![],
            )],
        ));

        let book = engine.book("BTC");
        assert_eq!(book.status(), BookStatus::Error);
        assert_eq!(book.seq(), 1);
        assert_eq!(book.best_bid_level(), Some((100_500_000, 2_000_000)));
        assert_eq!(book.ts_ms(), 1_000);
    }

    #[test]
    fn error_book_resets_on_snapshot() {
        let mut engine = init_test_engine_with_snapshot();

        engine.on_update(test_l2_book_diff_update(
            1_002,
            vec![test_l2_coin_diff(
                "BTC",
                3,
                2,
                false,
                vec![test_l2_level("100.5", "9")],
                vec![],
            )],
        ));
        assert_eq!(engine.book("BTC").status(), BookStatus::Error);

        engine.on_update(test_l2_book_diff_update(
            1_003,
            vec![test_l2_coin_diff(
                "BTC",
                4,
                3,
                false,
                vec![test_l2_level("100.5", "9")],
                vec![],
            )],
        ));
        assert_eq!(engine.book("BTC").status(), BookStatus::Error);

        engine.on_update(test_l2_book_diff_update(
            1_004,
            vec![test_l2_coin_diff(
                "BTC",
                1,
                0,
                true,
                vec![test_l2_level("200.0", "1")],
                vec![],
            )],
        ));

        let book = engine.book("BTC");
        assert_eq!(book.status(), BookStatus::Active);
        assert_eq!(book.seq(), 1);
        assert_eq!(book.best_bid_level(), Some((200_000_000, 1_000_000)));
    }

    #[test]
    fn crossed_book() {
        let mut engine = init_test_engine_with_snapshot();

        engine.on_update(test_l2_book_diff_update(
            1_001,
            vec![test_l2_coin_diff(
                "BTC",
                2,
                1,
                false,
                vec![test_l2_level("101", "1")],
                vec![],
            )],
        ));

        let book = engine.book("BTC");
        assert!(book.is_crossed());
        assert_eq!(book.status(), BookStatus::Error);
    }

    #[test]
    fn separate_books() {
        let mut engine = test_engine();

        engine.on_update(test_l2_book_diff_update(
            1_000,
            vec![
                test_l2_coin_diff("BTC", 1, 0, true, vec![test_l2_level("100.0", "1")], vec![]),
                test_l2_coin_diff("ETH", 1, 0, true, vec![test_l2_level("50.0", "2")], vec![]),
            ],
        ));

        engine.on_update(test_l2_book_diff_update(
            1_001,
            vec![test_l2_coin_diff(
                "BTC",
                2,
                1,
                false,
                vec![test_l2_level("101.0", "3")],
                vec![],
            )],
        ));

        assert_eq!(engine.books.len(), 2);

        assert_eq!(
            engine.book("BTC").best_bid_level(),
            Some((101_000_000, 3_000_000))
        );
        assert_eq!(engine.book("BTC").seq(), 2);
        
        assert_eq!(
            engine.book("ETH").best_bid_level(),
            Some((50_000_000, 2_000_000))
        );
        assert_eq!(engine.book("ETH").seq(), 1);
    }

    #[test]
    fn parse_decimal_strings_to_u64() {
        let parse = Engine::parse_to_u64_with_mul;
        assert_eq!(parse("0"), 0);
        assert_eq!(parse("123"), 123 * MULTIPLIER);
        assert_eq!(parse("123.456"), 123_456_000);
        assert_eq!(parse("1.5"), 1_500_000);
        assert_eq!(parse("0.00123456"), 1_234);
        assert_eq!(parse("123456.7"), 123_456_700_000);
        assert_eq!(parse("0.1"), 100_000);
        assert_eq!(parse("0.000001"), 1);
        assert_eq!(parse("1"), MULTIPLIER);
        assert_eq!(parse("0.0000019"), 1);
        assert_eq!(parse("0.0000001"), 0);
        assert_eq!(parse("1.0000009"), MULTIPLIER);
    }
}
