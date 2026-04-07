//! Classical material + mobility evaluation (centipawns, side to move).

use shakmaty::{Bitboard, Chess, Color, Position, Role, Square, attacks};

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

/// Mobility score for one side: sum of (weight × reachable squares) for each piece.
/// Reachable squares exclude squares occupied by friendly pieces.
fn mobility(pos: &Chess, color: Color) -> i32 {
    let board = pos.board();
    let occupied = board.occupied();
    let friendly = board.by_color(color);
    let mut score = 0i32;

    const ROLES: [(Role, i32); 4] = [
        (Role::Knight, 4),
        (Role::Bishop, 5),
        (Role::Rook, 2),
        (Role::Queen, 1),
    ];

    for &(role, weight) in &ROLES {
        let pieces = board.by_role(role) & friendly;
        for sq in pieces {
            let att = match role {
                Role::Knight => attacks::knight_attacks(sq),
                Role::Bishop => attacks::bishop_attacks(sq, occupied),
                Role::Rook => attacks::rook_attacks(sq, occupied),
                Role::Queen => attacks::queen_attacks(sq, occupied),
                _ => Bitboard::EMPTY,
            };
            score += weight * (att & !friendly).count() as i32;
        }
    }
    score
}

/// Material + mobility in centipawns from the **side to move** perspective, **before** shared
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

    white_pov += mobility(pos, Color::White) - mobility(pos, Color::Black);

    stm_from_white_pov(white_pov, pos.turn())
}
