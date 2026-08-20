use crate::engine::MULTIPLIER;
use std::fmt::Display;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Bid,
    Ask,
}

impl Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Bid => write!(f, "Bid"),
            Side::Ask => write!(f, "Ask"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BookStatus {
    Init,
    Active,
    Error,
}

impl Display for BookStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BookStatus::Init => write!(f, "INIT"),
            BookStatus::Active => write!(f, "ACTIVE"),
            BookStatus::Error => write!(f, "ERROR"),
        }
    }
}

pub type Px = u64;
pub type Sz = u64;

const MAX_LEVEL: usize = 20;

pub struct OrderBook {
    bids: BookSide,
    asks: BookSide,
    seq: u64,
    status: BookStatus,
    ts_ms: u64,
}

struct BookSide {
    side: Side,
    levels: [Level; MAX_LEVEL],
    valid_len: usize, // valid levels
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
struct Level {
    px: Px,
    sz: Sz,
}

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} @ {}",
            self.sz as f64 / MULTIPLIER as f64,
            self.px as f64 / MULTIPLIER as f64
        )
    }
}

impl Default for OrderBook {
    fn default() -> Self {
        Self {
            bids: BookSide::new(Side::Bid),
            asks: BookSide::new(Side::Ask),
            seq: Default::default(),
            status: BookStatus::Init,
            ts_ms: Default::default(),
        }
    }
}

impl Display for OrderBook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OrderBook: seq={} status={} ts_ms={} bids={} asks={}",
            self.seq, self.status, self.ts_ms, self.bids, self.asks,
        )?;

        Ok(())
    }
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BookSide::new(Side::Bid),
            asks: BookSide::new(Side::Ask),
            seq: 0,
            status: BookStatus::Init,
            ts_ms: 0,
        }
    }

    pub fn is_in_seq(&self, prev_seq: u64) -> bool {
        prev_seq == self.seq
    }

    pub fn is_crossed(&self) -> bool {
        match (self.bids.best(), self.asks.best()) {
            (Some(best_bid_lv), Some(best_ask_lv)) => best_bid_lv.px >= best_ask_lv.px,
            _ => false,
        }
    }

    pub fn best_bid_level(&self) -> Option<(Px, Sz)> {
        self.bids.best().map(|level| (level.px, level.sz))
    }

    pub fn best_ask_level(&self) -> Option<(Px, Sz)> {
        self.asks.best().map(|level| (level.px, level.sz))
    }

    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn set_seq(&mut self, seq: u64) {
        self.seq = seq;
    }

    pub fn status(&self) -> BookStatus {
        self.status
    }

    pub fn set_status(&mut self, status: BookStatus) {
        self.status = status;
    }

    pub fn ts_ms(&self) -> u64 {
        self.ts_ms
    }

    pub fn set_ts_ms(&mut self, ts_ms: u64) {
        self.ts_ms = ts_ms;
    }

    pub fn apply(&mut self, side: Side, px: Px, sz: Sz) {
        match side {
            Side::Bid => self.bids.apply(px, sz),
            Side::Ask => self.asks.apply(px, sz),
        }
    }

    pub fn reset(&mut self) {
        self.bids.reset();
        self.asks.reset();
        self.seq = 0;
        self.status = BookStatus::Init;
        self.ts_ms = 0;
    }
}

pub const DISPLAY_LEVEL: usize = 3;

impl Display for BookSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_levels = self.levels().iter().take(DISPLAY_LEVEL);
        for lv in display_levels {
            write!(f, "{} | ", lv)?;
        }

        Ok(())
    }
}

impl BookSide {
    fn new(side: Side) -> Self {
        Self {
            side,
            levels: [Level::default(); MAX_LEVEL],
            valid_len: 0,
        }
    }

    fn levels(&self) -> &[Level] {
        &self.levels[..self.valid_len]
    }

    fn best(&self) -> Option<&Level> {
        self.levels().first()
    }

    fn apply(&mut self, px: Px, sz: Sz) {
        match self.search(px) {
            Ok(idx) => {
                if sz == 0 {
                    self.remove(idx);
                } else {
                    self.levels[idx].sz = sz;
                }
            }
            Err(idx) => {
                if sz != 0 {
                    self.insert(idx, Level { px, sz });
                }
            }
        }
    }

    fn search(&self, px: Px) -> Result<usize, usize> {
        // linear search from best price
        for (idx, level) in self.levels().iter().enumerate() {
            // find the right idx for px
            let reached = match self.side {
                Side::Bid => level.px <= px,
                Side::Ask => level.px >= px,
            };

            if reached {
                if level.px == px {
                    // update
                    return Ok(idx);
                } else {
                    // insert
                    return Err(idx);
                }
            }
        }

        Err(self.valid_len)
    }

    fn insert(&mut self, idx: usize, level: Level) {
        if idx >= MAX_LEVEL {
            return;
        }

        // end is [0, MAX_LEVEL)
        let end = self.valid_len.min(MAX_LEVEL - 1);
        self.levels.copy_within(idx..end, idx + 1);
        self.levels[idx] = level;
        self.valid_len = (self.valid_len + 1).min(MAX_LEVEL)
    }

    fn remove(&mut self, idx: usize) {
        self.levels.copy_within(idx + 1..self.valid_len, idx);
        self.valid_len -= 1;
    }

    fn reset(&mut self) {
        self.levels = [Level::default(); MAX_LEVEL];
        self.valid_len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_bookside(side: Side, levels: &[(Px, Sz)]) -> BookSide {
        let mut bs = BookSide::new(side);
        for &(px, sz) in levels {
            bs.apply(px, sz);
        }

        bs
    }

    fn test_book(bids: &[(Px, Sz)], asks: &[(Px, Sz)]) -> OrderBook {
        let mut book = OrderBook::new();
        book.bids = test_bookside(Side::Bid, bids);
        book.asks = test_bookside(Side::Ask, asks);

        book
    }

    fn prices(bs: &BookSide) -> Vec<Px> {
        bs.levels().iter().map(|level| level.px).collect()
    }

    #[test]
    fn bids_sorted_descending() {
        let bs = test_bookside(Side::Bid, &[(100, 1), (102, 1), (101, 1), (99, 1)]);
        assert_eq!(prices(&bs), vec![102, 101, 100, 99]);
    }

    #[test]
    fn asks_sorted_ascending() {
        let bs = test_bookside(Side::Ask, &[(102, 1), (100, 1), (101, 1), (103, 1)]);
        assert_eq!(prices(&bs), vec![100, 101, 102, 103]);
    }

    #[test]
    fn update_size_for_existing_price() {
        let bs = test_bookside(Side::Bid, &[(100, 1), (99, 2), (100, 7)]);
        assert_eq!(bs.valid_len, 2);
        assert_eq!(bs.levels()[0], Level { px: 100, sz: 7 });
    }

    #[test]
    fn zero_size_removes_the_level() {
        let bs = test_bookside(Side::Bid, &[(100, 1), (99, 2), (98, 3), (99, 0)]);
        assert_eq!(prices(&bs), vec![100, 98]);
    }

    #[test]
    fn zero_size_for_non_existing_price() {
        let bs = test_bookside(Side::Bid, &[(100, 1), (77, 0)]);
        assert_eq!(bs.valid_len, 1);
        assert_eq!(prices(&bs), vec![100]);
    }

    #[test]
    fn remove_the_only_level() {
        let bs = test_bookside(Side::Bid, &[(100, 1), (100, 0)]);
        assert_eq!(bs.valid_len, 0);
        assert!(bs.best().is_none());
    }

    #[test]
    fn better_price_into_a_full_book() {
        // full book
        let mut bs = BookSide::new(Side::Bid);
        for i in 0..MAX_LEVEL {
            bs.apply(100 - i as Px, 1);
        }

        assert_eq!(bs.valid_len, MAX_LEVEL);
        assert_eq!(*prices(&bs).last().unwrap(), 81);

        bs.apply(200, 1);

        assert_eq!(bs.valid_len, MAX_LEVEL);
        assert_eq!(prices(&bs)[0], 200);
        assert_eq!(*prices(&bs).last().unwrap(), 82);
    }

    #[test]
    fn worse_price_into_a_full_book() {
        // full book
        let mut bs = BookSide::new(Side::Bid);
        for i in 0..MAX_LEVEL {
            bs.apply(100 - i as Px, 1);
        }

        bs.apply(1, 1);

        assert_eq!(bs.valid_len, MAX_LEVEL);
        assert!(!prices(&bs).contains(&1));
        assert_eq!(*prices(&bs).last().unwrap(), 81);
    }

    #[test]
    fn check_seq() {
        let mut book = OrderBook::new();
        assert!(book.is_in_seq(0));
        book.set_seq(123);
        assert!(book.is_in_seq(123));
        assert!(!book.is_in_seq(122));
        assert!(!book.is_in_seq(124));
    }

    #[test]
    fn book_is_crossed() {
        assert!(test_book(&[(101, 1)], &[(100, 1)]).is_crossed());
        assert!(test_book(&[(100, 1)], &[(100, 1)]).is_crossed());
        assert!(!test_book(&[(100, 1)], &[(101, 1)]).is_crossed());
    }

    #[test]
    fn one_sided_book_not_crossed() {
        assert!(!test_book(&[(100, 1)], &[]).is_crossed());
        assert!(!test_book(&[], &[(100, 1)]).is_crossed());
        assert!(!test_book(&[], &[]).is_crossed());
    }

    #[test]
    fn reset_book() {
        let mut book = test_book(&[(100, 1)], &[(101, 1)]);
        book.set_seq(123);
        book.set_status(BookStatus::Active);
        book.set_ts_ms(1780000000000);

        book.reset();

        assert_eq!(book.bids.valid_len, 0);
        assert_eq!(book.asks.valid_len, 0);
        assert_eq!(book.seq(), 0);
        assert_eq!(book.status(), BookStatus::Init);
        assert_eq!(book.ts_ms(), 0);
    }
}
