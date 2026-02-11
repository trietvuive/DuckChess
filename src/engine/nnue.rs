use shakmaty::{Bitboard, Chess, Color, Piece, Position, Role, Square};

fn role_value(role: Role) -> i32 {
    match role {
        Role::Pawn => 100,
        Role::Knight => 320,
        Role::Bishop => 330,
        Role::Rook => 500,
        Role::Queen => 900,
        Role::King => 20000,
    }
}

pub fn evaluate(pos: &Chess) -> i32 {
    let mut score = 0i32;

    for sq in Square::ALL {
        if let Some(piece) = pos.board().piece_at(sq) {
            let value = piece_value(piece, sq, pos);
            score += if piece.color == Color::White { value } else { -value };
        }
    }

    let white_bishops = (pos.board().bishops() & pos.board().white()).count();
    let black_bishops = (pos.board().bishops() & pos.board().black()).count();
    if white_bishops >= 2 { score += 30; }
    if black_bishops >= 2 { score -= 30; }

    for file in 0..8 {
        let file_mask = Bitboard::from(Square::new(file));
        let file_bb = file_mask
            | file_mask.shift(8)
            | file_mask.shift(16)
            | file_mask.shift(24)
            | file_mask.shift(32)
            | file_mask.shift(40)
            | file_mask.shift(48)
            | file_mask.shift(56);
        
        let white_pawns = (pos.board().pawns() & pos.board().white() & file_bb).count();
        let black_pawns = (pos.board().pawns() & pos.board().black() & file_bb).count();
        if white_pawns > 1 { score -= (white_pawns - 1) as i32 * 15; }
        if black_pawns > 1 { score += (black_pawns - 1) as i32 * 15; }
    }

    if pos.turn() == Color::White { score } else { -score }
}

fn piece_value(piece: Piece, sq: Square, pos: &Chess) -> i32 {
    let base = role_value(piece.role);
    let file = sq.file() as i32;
    let rank = sq.rank() as i32;
    
    let bonus = match piece.role {
        Role::Knight | Role::Bishop => {
            let center_dist = (3.5 - file as f32).abs() + (3.5 - rank as f32).abs();
            (15.0 - center_dist * 3.0) as i32
        }
        Role::Pawn => {
            let advancement = if piece.color == Color::White { rank } else { 7 - rank };
            advancement * 5
        }
        Role::King => {
            if pos.board().queens().any() {
                let edge_dist = file.min(7 - file).min(rank.min(7 - rank));
                -edge_dist * 5
            } else {
                let center_dist = (3.5 - file as f32).abs() + (3.5 - rank as f32).abs();
                (15.0 - center_dist * 3.0) as i32
            }
        }
        _ => 0,
    };
    
    base + bonus
}

pub fn is_insufficient_material(pos: &Chess) -> bool {
    let dominated = pos.board().occupied();
    let dominated_count = dominated.count();
    
    if dominated_count == 2 { return true; }
    if dominated_count == 3 {
        if pos.board().knights().count() == 1 || pos.board().bishops().count() == 1 {
            return true;
        }
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
