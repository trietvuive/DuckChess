use shakmaty::{CastlingMode, Chess, Position, fen::Fen, uci::UciMove as SmUciMove};
use vampirc_uci::{UciFen, UciMove as VampUciMove};

pub(crate) fn parse_uci_move(board: &Chess, move_str: &str) -> Option<shakmaty::Move> {
    let uci: SmUciMove = move_str.parse().ok()?;
    let mv = uci.to_move(board).ok()?;
    if board.is_legal(&mv) { Some(mv) } else { None }
}

/// Applies a vampirc-parsed `position` message (`fen` + `moves` wire types → [`apply_uci_position`]).
pub(crate) fn apply_uci_position_from_vampirc(
    board: &mut Chess,
    startpos: bool,
    fen: &Option<UciFen>,
    moves: &[VampUciMove],
) {
    let fen_str = fen.as_ref().map(UciFen::as_str);
    let move_strs: Vec<String> = moves.iter().map(ToString::to_string).collect();
    let move_refs: Vec<&str> = move_strs.iter().map(String::as_str).collect();
    apply_uci_position(board, startpos, fen_str, &move_refs);
}

fn parse_fen_to_position(fen_str: &str) -> Option<Chess> {
    fen_str
        .parse::<Fen>()
        .ok()?
        .into_position::<Chess>(CastlingMode::Standard)
        .ok()
}

pub(crate) fn apply_uci_position(
    board: &mut Chess,
    startpos: bool,
    fen: Option<&str>,
    move_strs: &[&str],
) {
    if startpos {
        *board = Chess::default();
    } else if let Some(pos) = fen.and_then(parse_fen_to_position) {
        *board = pos;
    }

    for &s in move_strs {
        if let Some(mv) = parse_uci_move(board, s) {
            *board = board.clone().play(&mv).unwrap();
        }
    }
}
