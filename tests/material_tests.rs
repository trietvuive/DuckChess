//! Static evaluation smoke tests (no exact scores — those change with piece values / tempo).

use duck_chess::evaluate;
use shakmaty::{CastlingMode, Chess, fen::Fen};

fn chess(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

#[test]
fn eval_startpos_is_finite_and_small() {
    let s = evaluate(&Chess::default());
    assert!(s.abs() < 500);
}

#[test]
fn eval_changes_when_material_differs() {
    let start = Chess::default();
    let down_bishop = chess("rn1qkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    assert_ne!(evaluate(&start), evaluate(&down_bishop));
}
