//! Search limits, stats, constants, and context types.

use shakmaty::Move;

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

/// Search context holds the current search state and bounds.
/// Used to reduce argument count when passing search parameters.
#[derive(Clone, Copy)]
pub struct SearchContext {
    /// Current search depth remaining.
    pub depth: i32,
    /// Alpha bound (lower bound on achievable score).
    pub alpha: i32,
    /// Beta bound (upper bound on acceptable score).
    pub beta: i32,
    /// Ply from root (0 = root).
    pub ply: usize,
    /// Whether this is a principal variation node.
    pub is_pv: bool,
    /// Whether the side to move is in check.
    pub in_check: bool,
    /// Transposition table hash.
    pub hash: u64,
}

impl SearchContext {
    /// Negate bounds for recursive call.
    pub fn negate(self) -> Self {
        Self {
            alpha: -self.beta,
            beta: -self.alpha,
            ..self
        }
    }

    /// Increment ply for recursive descent.
    pub fn next_ply(self) -> Self {
        Self {
            ply: self.ply + 1,
            ..self
        }
    }

    /// Decrement depth by 1 for normal recursive calls.
    pub fn descend(self) -> Self {
        Self {
            depth: self.depth - 1,
            ..self
        }
    }
}

/// Move search context for searching a single move.
#[derive(Clone, Copy)]
pub struct MoveContext<'a> {
    /// The move to search.
    pub mv: &'a Move,
    /// Index in the move list.
    pub index: usize,
    /// Parent search context.
    pub search: SearchContext,
}
