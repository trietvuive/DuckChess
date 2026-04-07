/// UCI / user-facing eval backend selector; [`super::Evaluator`] dispatches and applies shared leaf rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EvalKind {
    /// Classical piece values + tempo.
    Material,
    /// NNUE evaluation with embedded weights (default).
    #[default]
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
