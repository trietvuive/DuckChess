//! Search: iterative deepening, negamax (α–β), quiescence.

mod negamax;
mod ordering;
mod pv;
mod searcher;
mod types;

pub use searcher::Searcher;
pub use types::{MoveContext, SearchContext, SearchLimits, SearchStats};
