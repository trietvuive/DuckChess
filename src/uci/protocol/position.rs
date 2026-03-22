use shakmaty::{fen::Fen, uci::UciMove, CastlingMode, Chess, Position};

pub(crate) fn parse_uci_move(board: &Chess, move_str: &str) -> Option<shakmaty::Move> {
    let uci: UciMove = move_str.parse().ok()?;
    let mv = uci.to_move(board).ok()?;
    if board.is_legal(&mv) {
        Some(mv)
    } else {
        None
    }
}

pub(crate) fn apply_uci_position(
    board: &mut Chess,
    startpos: bool,
    fen: Option<&str>,
    move_strs: &[&str],
) {
    if startpos {
        *board = Chess::default();
    } else if let Some(fen_str) = fen {
        if let Ok(f) = fen_str.parse::<Fen>() {
            if let Ok(pos) = f.into_position::<Chess>(CastlingMode::Standard) {
                *board = pos;
            }
        }
    }

    for &s in move_strs {
        if let Some(mv) = parse_uci_move(board, s) {
            *board = board.clone().play(&mv).unwrap();
        }
    }
}
