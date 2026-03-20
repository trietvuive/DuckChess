//! Classical material evaluation (centipawns, side to move).

use shakmaty::{Chess, Color, Position, Role, Square};

/// Tempo bonus for the side to move (centipawns).
pub(crate) const MATERIAL_TEMPO_CP: i32 = 18;

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

/// Material balance from White’s perspective, then scored for the side to move.
pub fn evaluate_material(pos: &Chess) -> i32 {
    let board = pos.board();
    let mut white_pov = 0i32;
    for sq in Square::ALL {
        if let Some(piece) = board.piece_at(sq) {
            let v = piece_value(piece.role);
            white_pov += if piece.color == Color::White { v } else { -v };
        }
    }
    let s = if pos.turn() == Color::White {
        white_pov
    } else {
        -white_pov
    };
    s + MATERIAL_TEMPO_CP
}
