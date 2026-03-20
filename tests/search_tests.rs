//! Search smoke tests: return legal moves under common limits — not tied to best move or scores.

use duck_chess::engine::search::{SearchLimits, Searcher};
use duck_chess::evaluate;
use shakmaty::{fen::Fen, CastlingMode, Chess, Position};

fn from_fen(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

fn assert_legal(pos: &Chess, mv: &shakmaty::Move) {
    assert!(
        pos.legal_moves().iter().any(|m| m == mv),
        "search must return a legal move"
    );
}

#[test]
fn search_finds_move_from_startpos() {
    let pos = Chess::default();
    let mut s = Searcher::new();
    let mv = s
        .search(
            &pos,
            SearchLimits {
                depth: Some(3),
                ..Default::default()
            },
        )
        .expect("move");
    assert_legal(&pos, &mv);
}

#[test]
fn search_finds_move_from_non_startpos() {
    let pos = from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    let mut s = Searcher::new();
    let mv = s
        .search(
            &pos,
            SearchLimits {
                depth: Some(2),
                ..Default::default()
            },
        )
        .expect("move");
    assert_legal(&pos, &mv);
}

#[test]
fn search_completes_with_node_cap() {
    let pos = Chess::default();
    let mut s = Searcher::new();
    let mv = s
        .search(
            &pos,
            SearchLimits {
                nodes: Some(800),
                ..Default::default()
            },
        )
        .expect("move");
    assert_legal(&pos, &mv);
}

#[test]
fn search_completes_with_multipv() {
    let pos = Chess::default();
    let mut s = Searcher::new();
    let mv = s
        .search(
            &pos,
            SearchLimits {
                depth: Some(2),
                multi_pv: 3,
                ..Default::default()
            },
        )
        .expect("move");
    assert_legal(&pos, &mv);
}

#[test]
fn search_still_works_after_clear_and_hash_resize() {
    let pos = Chess::default();
    let mut s = Searcher::new();
    let limits = SearchLimits {
        depth: Some(2),
        ..Default::default()
    };
    let _ = s.search(&pos, limits.clone());
    s.clear();
    s.set_hash_size(16);
    let mv = s.search(&pos, limits).expect("move");
    assert_legal(&pos, &mv);
}

#[test]
fn search_produces_move_in_sharp_position() {
    let pos = from_fen("6k1/5ppp/8/8/8/8/8/4Q2K w - - 0 1");
    let mut s = Searcher::new();
    let mv = s
        .search(
            &pos,
            SearchLimits {
                depth: Some(2),
                ..Default::default()
            },
        )
        .expect("move");
    assert_legal(&pos, &mv);
}

#[test]
fn evaluate_runs_without_panic() {
    let _ = evaluate(&Chess::default());
}
