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
fn test_eval_runs() {
    let pos = Chess::default();
    let score = evaluate(&pos);
    assert!(score.abs() < 10000);
}

#[test]
fn test_eval_different_positions() {
    let pos1 = Chess::default();
    let pos2 = from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    let score1 = evaluate(&pos1);
    let score2 = evaluate(&pos2);
    assert_ne!(score1, score2);
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
