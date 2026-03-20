mod material;
mod nnue;

use shakmaty::Chess;

/// Static evaluation backend for search.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EvalKind {
    /// Piece values + tempo (default).
    #[default]
    Material,
    /// Embedded NNUE weights (experimental).
    Nnue,
}

impl EvalKind {
    pub fn from_uci_value(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "material" | "classic" => Some(Self::Material),
            "nnue" => Some(Self::Nnue),
            _ => None,
        }
    }
}

pub(crate) fn evaluate_as(kind: EvalKind, pos: &Chess) -> i32 {
    match kind {
        EvalKind::Material => material::evaluate_material(pos),
        EvalKind::Nnue => nnue::evaluate_nnue(pos),
    }
}

/// Material static evaluation (side to move, centipawns).
///
/// For the evaluation selected by UCI option `Eval`, use [`Searcher::evaluate_position`].
pub fn evaluate(pos: &Chess) -> i32 {
    material::evaluate_material(pos)
}

pub use nnue::is_insufficient_material;
