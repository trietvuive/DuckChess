//! Chess engine components
//!
//! This module contains the core engine functionality:
//! - Move generation with magic bitboards
//! - NNUE neural network evaluation
//! - Alpha-beta search
//! - Transposition table

pub mod movegen;
pub mod nnue;
pub mod search;
pub mod tt;

pub use movegen::MoveGen;
pub use nnue::{evaluate, evaluator, NNUEAccumulator, NNUEEvaluator, NNUENetwork};
pub use search::{SearchLimits, SearchStats, Searcher, INFINITY, MATE_SCORE};
pub use tt::{TTEntry, TTFlag, TranspositionTable};

