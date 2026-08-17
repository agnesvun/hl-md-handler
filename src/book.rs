#[derive(Clone, Copy)]
pub enum Side {
    Bid,
    Ask,
}

pub enum BookStatus {
    Init,
    Active,
    Error,
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

#[derive(Clone, Copy, Default)]
struct Level {
    px: Px,
    sz: Sz,
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

    pub fn check_seq(&mut self, prev_seq: u64) -> bool {
        if prev_seq != self.seq {
            println!("gap detected");
            self.status = BookStatus::Error;
            return false;
        }

        true
    }

    pub fn update_seq(&mut self, seq: u64) {
        self.seq = seq;
        self.status = BookStatus::Active;
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
