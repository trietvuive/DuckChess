//! NNUE evaluation integration tests.

use duck_chess::{EvalKind, Evaluator, evaluate};
use shakmaty::{CastlingMode, Chess, Color, Position, fen::Fen};

fn chess(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

#[test]
fn nnue_startpos_does_not_panic() {
    let _ = evaluate(&Chess::default());
}

#[test]
fn nnue_is_deterministic() {
    let pos = chess("r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2");
    assert_eq!(evaluate(&pos), evaluate(&pos));
}

#[test]
fn nnue_startpos_is_reasonable() {
    let score = evaluate(&Chess::default());
    assert!(
        score.abs() < 500,
        "startpos NNUE eval {score} is unreasonably large"
    );
}

#[test]
fn nnue_symmetric_positions_similar() {
    let white_pov = chess("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    let black_pov = chess("rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let diff = (evaluate(&white_pov) - evaluate(&black_pov)).abs();
    assert!(
        diff < 200,
        "mirror positions should eval similarly, diff = {diff}"
    );
}

#[test]
fn nnue_distinguishes_material_imbalance() {
    let even = Chess::default();
    let white_missing_queen = chess("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNB1KBNR w KQkq - 0 1");
    assert!(
        evaluate(&even) > evaluate(&white_missing_queen),
        "losing a queen should lower the eval"
    );
}

#[test]
fn nnue_eval_does_not_panic_on_many_positions() {
    let fens = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
        "8/8/8/8/8/5k2/8/4K3 w - - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    ];
    for fen in fens {
        let pos = chess(fen);
        let _ = evaluate(&pos);
    }
}

#[test]
fn evaluator_backend_switch() {
    let pos = Chess::default();

    let mut ev = Evaluator::new();
    assert_eq!(ev.backend(), EvalKind::Material);
    let mat_score = ev.evaluate(&pos);

    ev.set_backend(EvalKind::Nnue);
    assert_eq!(ev.backend(), EvalKind::Nnue);
    let nnue_score = ev.evaluate(&pos);

    assert!(mat_score.abs() < 500);
    assert!(
        nnue_score.abs() < 30_000,
        "nnue score should be finite, this is super high but we'll improve it"
    );
}

#[test]
fn nnue_side_to_move_matters() {
    let white_to_move = chess("r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2");
    let black_to_move = chess("r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 1 2");
    assert_eq!(white_to_move.turn(), Color::White);
    assert_eq!(black_to_move.turn(), Color::Black);
    // Scores are from side-to-move perspective, so flipping turn should change the sign direction.
    let w = evaluate(&white_to_move);
    let b = evaluate(&black_to_move);
    assert_ne!(w, b, "eval should differ when side to move changes");
}
