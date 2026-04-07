mod common;
mod evaluator;
mod kind;
mod material;
mod nnue;

pub use common::is_insufficient_material;
pub use evaluator::Evaluator;
pub use kind::EvalKind;

/// NNUE score with the same shared leaf rules as UCI default `Eval` (tempo, etc.).
pub fn evaluate(pos: &shakmaty::Chess) -> i32 {
    common::finalize_leaf(nnue::raw_stm_nnue(pos))
}
