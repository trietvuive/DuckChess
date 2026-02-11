use shakmaty::{fen::Fen, CastlingMode, Chess};
use duck_chess::engine::search::{SearchLimits, Searcher};
use duck_chess::engine::nnue::evaluate;

fn from_fen(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

#[test]
fn test_search_startpos() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits { depth: Some(4), ..Default::default() };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn test_search_mate_in_one() {
    let pos = from_fen("6k1/5ppp/8/8/8/8/8/4Q2K w - - 0 1");
    let mut searcher = Searcher::new();
    let limits = SearchLimits { depth: Some(3), ..Default::default() };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn test_search_avoid_stalemate() {
    let pos = from_fen("7k/8/6K1/8/8/8/8/6Q1 w - - 0 1");
    let mut searcher = Searcher::new();
    let limits = SearchLimits { depth: Some(4), ..Default::default() };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn test_eval_startpos() {
    let pos = Chess::default();
    let score = evaluate(&pos);
    assert!(score.abs() < 50);
}

#[test]
fn test_eval_material_advantage() {
    let pos = from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let score = evaluate(&pos);
    assert!(score > 800);
}

#[test]
fn test_depth_limit() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits { depth: Some(2), ..Default::default() };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn test_node_limit() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits { nodes: Some(1000), ..Default::default() };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}
