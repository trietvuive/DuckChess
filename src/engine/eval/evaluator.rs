//! Single evaluation entry point: picks a backend and applies shared leaf rules.

use shakmaty::Chess;

use super::common::finalize_leaf;
use super::kind::EvalKind;
use super::material;
use super::nnue;

/// Engine evaluation harness (backend + shared material / NNUE pipeline).
#[derive(Clone, Copy, Debug)]
pub struct Evaluator {
    backend: EvalKind,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            backend: EvalKind::default(),
        }
    }

    pub fn backend(&self) -> EvalKind {
        self.backend
    }

    pub fn set_backend(&mut self, backend: EvalKind) {
        self.backend = backend;
    }

    /// Full static evaluation (centipawns, side to move), including shared [`finalize_leaf`].
    pub fn evaluate(&self, pos: &Chess) -> i32 {
        let raw_stm = match self.backend {
            EvalKind::Material => material::raw_stm_material(pos),
            EvalKind::Nnue => nnue::raw_stm_nnue(pos),
        };
        finalize_leaf(raw_stm)
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}
