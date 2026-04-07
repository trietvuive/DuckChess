mod common;
mod evaluator;
mod kind;
mod material;
mod nnue;

pub use common::is_insufficient_material;
pub use evaluator::Evaluator;
pub use kind::EvalKind;

/// Default evaluation with shared leaf rules (tempo, etc.).
pub fn evaluate(pos: &shakmaty::Chess) -> i32 {
    Evaluator::default().evaluate(pos)
}
