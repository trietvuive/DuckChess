mod kind;
mod material;
mod nnue;

pub(crate) use kind::evaluate_as;
pub use kind::{evaluate, EvalKind};
pub use nnue::is_insufficient_material;
