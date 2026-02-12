//! Search limits, stats, and constants.

#[derive(Clone, Debug)]
pub struct SearchLimits {
    pub depth: Option<i32>,
    pub nodes: Option<u64>,
    pub movetime: Option<u64>,
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movestogo: Option<u32>,
    pub infinite: bool,
    /// Number of principal variations to report (1 = best line only).
    pub multi_pv: u32,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            depth: None,
            nodes: None,
            movetime: None,
            wtime: None,
            btime: None,
            winc: None,
            binc: None,
            movestogo: None,
            infinite: false,
            multi_pv: 1,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SearchStats {
    pub nodes: u64,
    pub qnodes: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
}

pub const INFINITY: i32 = 30000;
pub const MATE_SCORE: i32 = 29000;
pub const DRAW_SCORE: i32 = 0;
pub const MAX_DEPTH: i32 = 64;
