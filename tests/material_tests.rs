use duck_chess::{evaluate, MATERIAL_TEMPO_CP};
use shakmaty::{fen::Fen, CastlingMode, Chess};

fn chess(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

#[test]
fn equal_material_is_only_tempo() {
    assert_eq!(evaluate(&Chess::default()), MATERIAL_TEMPO_CP);
}

#[test]
fn white_advantage_respects_side_to_move() {
    let w = chess("rn1qkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let b = chess("rn1qkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
    assert_eq!(evaluate(&w), 330 + MATERIAL_TEMPO_CP);
    assert_eq!(evaluate(&b), -330 + MATERIAL_TEMPO_CP);
}
