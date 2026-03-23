//! Shared evaluation helpers used by all backends (tempo, finalization, drawish heuristics).

use shakmaty::{Bitboard, Chess, Color, Position};

/// Tempo bonus for the side to move (centipawns), applied in one place for every backend.
pub const TEMPO_CP: i32 = 18;

/// Convert a White-POV centipawn score to side-to-move perspective.
#[inline]
pub fn stm_from_white_pov(white_pov: i32, stm: Color) -> i32 {
    if stm == Color::White {
        white_pov
    } else {
        -white_pov
    }
}

/// Last-step leaf processing shared by material, NNUE, and future evaluators.
#[inline]
pub fn finalize_leaf(raw_stm_centipawns: i32) -> i32 {
    raw_stm_centipawns + TEMPO_CP
}

/// Dead draw by material (K vs K, K+B vs K+B same square color, etc.).
pub fn is_insufficient_material(pos: &Chess) -> bool {
    let dominated = pos.board().occupied();
    let dominated_count = dominated.count();

    if dominated_count == 2 {
        return true;
    }
    if dominated_count == 3
        && (pos.board().knights().count() == 1 || pos.board().bishops().count() == 1)
    {
        return true;
    }
    if dominated_count == 4 {
        let bishops = pos.board().bishops();
        if bishops.count() == 2 {
            let light = Bitboard::LIGHT_SQUARES;
            let dark = Bitboard::DARK_SQUARES;
            if (bishops & light).count() == 2 || (bishops & dark).count() == 2 {
                return true;
            }
        }
    }
    false
}
