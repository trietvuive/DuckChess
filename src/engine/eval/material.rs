//! Classical material evaluation (centipawns, side to move).

use shakmaty::{Chess, Color, Position, Role, Square};

use super::common::stm_from_white_pov;

#[inline]
fn piece_value(role: Role) -> i32 {
    match role {
        Role::Pawn => 100,
        Role::Knight => 320,
        Role::Bishop => 330,
        Role::Rook => 500,
        Role::Queen => 900,
        Role::King => 0,
    }
}

/// Material balance in centipawns from the **side to move** perspective, **before** shared
/// [`super::common::finalize_leaf`] (no tempo).
pub(crate) fn raw_stm_material(pos: &Chess) -> i32 {
    let board = pos.board();
    let mut white_pov = 0i32;
    for sq in Square::ALL {
        if let Some(piece) = board.piece_at(sq) {
            let v = piece_value(piece.role);
            white_pov += if piece.color == Color::White { v } else { -v };
        }
    }
    stm_from_white_pov(white_pov, pos.turn())
}
