use std::fmt::Display;

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy, Default)]
struct Level {
    px: Px,
    sz: Sz,
}

impl Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} @ {}", self.sz, self.px)
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
        writeln!(
            f,
            "OrderBook: seq={} status={} ts_ms={}",
            self.seq, self.status, self.ts_ms,
        )?;

        writeln!(f, "--------------- Ask ---------------")?;
        write!(f, "{}", self.asks)?;
        writeln!(f, "---------------")?;
        write!(f, "{}", self.bids)?;
        write!(f, "--------------- Bid ---------------")?;

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

    pub fn seq(&self) -> u64 {
        self.seq
    }

    pub fn set_seq(&mut self, seq: u64) {
        self.seq = seq;
    }

    pub fn set_status(&mut self, status: BookStatus) {
        self.status = status;
    }

    pub fn set_ts(&mut self, ts_ms: u64) {
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

const DISPLAY_LEVEL: usize = 3;

impl Display for BookSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_levels = self.levels().iter().take(DISPLAY_LEVEL);
        match self.side {
            Side::Bid => {
                for lv in display_levels {
                    writeln!(f, "{}", lv)?;
                }
            }
            Side::Ask => {
                for lv in display_levels.rev() {
                    writeln!(f, "{}", lv)?;
                }
            }
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
