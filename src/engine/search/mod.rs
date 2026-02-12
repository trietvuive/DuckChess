//! Search: iterative deepening, alpha-beta, quiescence.

mod alphabeta;
mod ordering;
mod pv;
mod searcher;
mod types;

pub use searcher::Searcher;
pub use types::{SearchLimits, SearchStats};
