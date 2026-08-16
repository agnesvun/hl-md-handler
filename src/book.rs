#[derive(Clone, Copy)]
pub enum Side {
    Bid,
    Ask,
}

pub type Px = u64;
pub type Sz = u64;

const MULTIPLIER: u64 = 1_000_000;
const MAX_LEVEL: usize = 20;

pub struct OrderBook {
    bids: BookSide,
    asks: BookSide,
}

struct BookSide {
    side: Side,
    levels: [Level; MAX_LEVEL],
    len: usize, // valid levels
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
        }
    }

    pub fn apply(&mut self, side: Side, px: Px, sz: Sz) {
        match side {
            Side::Bid => self.bids.apply(px, sz),
            Side::Ask => self.asks.apply(px, sz),
        }
    }
}

impl BookSide {
    fn new(side: Side) -> Self {
        Self {
            side,
            levels: [Level::default(); MAX_LEVEL],
            len: 0,
        }
    }

    fn levels(&self) -> &[Level] {
        &self.levels[..self.len]
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

        Err(self.len)
    }

    fn insert(&mut self, idx: usize, level: Level) {
        if idx >= MAX_LEVEL {
            return;
        }

        // end is [0, MAX_LEVEL)
        let end = self.len.min(MAX_LEVEL - 1);
        self.levels.copy_within(idx..end, idx + 1);
        self.levels[idx] = level;
        self.len = (self.len + 1).min(MAX_LEVEL)
    }

    fn remove(&mut self, idx: usize) {
        self.levels.copy_within(idx + 1..self.len, idx);
        self.len -= 1;
    }
}
