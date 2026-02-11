//! DuckChess - A UCI Chess Engine
//!
//! This chess engine uses the shakmaty library for board representation
//! and move generation, with custom search and evaluation.

pub mod engine;
pub mod uci;

// Re-export commonly used types
pub use shakmaty;
pub use engine::search::{SearchLimits, Searcher};
pub use engine::nnue::evaluate;
pub use uci::UCI;
