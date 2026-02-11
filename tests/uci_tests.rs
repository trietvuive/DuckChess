//! UCI Protocol Tests

use shakmaty::{fen::Fen, CastlingMode, Chess, Color, Position};
use duck_chess::uci::UCI;

fn from_fen(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

#[test]
fn test_uci_new() {
    let uci = UCI::new();
    assert_eq!(uci.board.legal_moves().len(), 20);
}

#[test]
fn test_uci_position_startpos() {
    let mut uci = UCI::new();
    uci.cmd_position(&["position", "startpos"]);
    assert_eq!(uci.board.legal_moves().len(), 20);
}

#[test]
fn test_uci_position_fen() {
    let mut uci = UCI::new();
    uci.cmd_position(&["position", "fen", "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R", "w", "KQkq", "-", "0", "1"]);
    assert_eq!(uci.board.legal_moves().len(), 48);
}

#[test]
fn test_uci_position_startpos_moves() {
    let mut uci = UCI::new();
    uci.cmd_position(&["position", "startpos", "moves", "e2e4", "e7e5"]);
    assert_eq!(uci.board.turn(), Color::White);
}

#[test]
fn test_uci_newgame() {
    let mut uci = UCI::new();
    uci.cmd_position(&["position", "startpos", "moves", "e2e4"]);
    uci.cmd_ucinewgame();
    assert_eq!(uci.board.legal_moves().len(), 20);
}

#[test]
fn test_fen_parsing() {
    let pos = from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
    assert_eq!(pos.turn(), Color::Black);
}

#[test]
fn test_move_parsing() {
    let uci = UCI::new();
    let mv = uci.parse_move("e2e4");
    assert!(mv.is_some());
}

#[test]
fn test_castling_parsing() {
    let mut uci = UCI::new();
    uci.cmd_position(&["position", "fen", "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R", "w", "KQkq", "-", "0", "1"]);
    let mv = uci.parse_move("e1g1");
    assert!(mv.is_some());
}

#[test]
fn test_promotion_parsing() {
    let mut uci = UCI::new();
    uci.cmd_position(&["position", "fen", "8/P7/8/8/8/8/8/4K2k", "w", "-", "-", "0", "1"]);
    let mv = uci.parse_move("a7a8q");
    assert!(mv.is_some());
}
