//! UCI-facing behaviour: position parsing, options, move encoding.

use duck_chess::EvalKind;
use duck_chess::uci::UCI;
use shakmaty::{CastlingMode, Chess, Color, Position, fen::Fen};

fn from_fen(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

#[test]
fn position_startpos_has_standard_branching() {
    let uci = UCI::new();
    assert_eq!(uci.board.legal_moves().len(), 20);
}

#[test]
fn position_applies_startpos_and_moves() {
    let mut uci = UCI::new();
    uci.cmd_position(&["position", "startpos", "moves", "e2e4", "e7e5"]);
    assert_eq!(uci.board.turn(), Color::White);
    assert!(!uci.board.legal_moves().is_empty());
}

#[test]
fn position_accepts_complex_fen() {
    let mut uci = UCI::new();
    uci.cmd_position(&[
        "position",
        "fen",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R",
        "w",
        "KQkq",
        "-",
        "0",
        "1",
    ]);
    assert!(!uci.board.legal_moves().is_empty());
}

#[test]
fn ucinewgame_resets_board() {
    let mut uci = UCI::new();
    uci.cmd_position(&["position", "startpos", "moves", "e2e4"]);
    uci.cmd_ucinewgame();
    assert_eq!(uci.board.legal_moves().len(), 20);
}

#[test]
fn parse_san_uci_moves() {
    let mut uci = UCI::new();
    assert!(uci.parse_move("e2e4").is_some());

    uci.cmd_position(&[
        "position",
        "fen",
        "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R",
        "w",
        "KQkq",
        "-",
        "0",
        "1",
    ]);
    assert!(uci.parse_move("e1g1").is_some());

    uci.cmd_position(&[
        "position",
        "fen",
        "8/P7/8/8/8/8/8/4K2k",
        "w",
        "-",
        "-",
        "0",
        "1",
    ]);
    assert!(uci.parse_move("a7a8q").is_some());
}

#[test]
fn setoption_multipv_and_clamp() {
    let mut uci = UCI::new();
    assert_eq!(uci.multi_pv(), 1);
    uci.cmd_setoption(&["setoption", "name", "MultiPV", "value", "2"]);
    assert_eq!(uci.multi_pv(), 2);
    uci.cmd_setoption(&["setoption", "name", "Multi", "PV", "value", "3"]);
    assert_eq!(uci.multi_pv(), 3);
    uci.cmd_setoption(&["setoption", "name", "MultiPV", "value", "99"]);
    assert_eq!(uci.multi_pv(), 5);
    uci.cmd_setoption(&["setoption", "name", "MultiPV", "value", "0"]);
    assert_eq!(uci.multi_pv(), 1);
}

#[test]
fn setoption_eval_material_and_nnue() {
    let mut uci = UCI::new();
    assert_eq!(uci.eval_kind(), EvalKind::Material);
    uci.cmd_setoption(&["setoption", "name", "Eval", "value", "NNUE"]);
    assert_eq!(uci.eval_kind(), EvalKind::Nnue);
    uci.cmd_setoption(&["setoption", "name", "Eval", "value", "Material"]);
    assert_eq!(uci.eval_kind(), EvalKind::Material);
    uci.cmd_setoption(&["setoption", "name", "Eval", "value", "Classic"]);
    assert_eq!(uci.eval_kind(), EvalKind::Material);
}

#[test]
fn fen_roundtrip_turn() {
    let pos = from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1");
    assert_eq!(pos.turn(), Color::Black);
}
