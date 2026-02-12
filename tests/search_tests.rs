use duck_chess::evaluate;
use duck_chess::engine::search::{SearchLimits, SearchStats, Searcher};
use shakmaty::{fen::Fen, CastlingMode, Chess, Position};

fn from_fen(fen: &str) -> Chess {
    let f: Fen = fen.parse().unwrap();
    f.into_position(CastlingMode::Standard).unwrap()
}

#[test]
fn test_search_startpos() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(4),
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn test_search_mate_in_one() {
    let pos = from_fen("6k1/5ppp/8/8/8/8/8/4Q2K w - - 0 1");
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(3),
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn test_search_avoid_stalemate() {
    let pos = from_fen("7k/8/6K1/8/8/8/8/6Q1 w - - 0 1");
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(4),
        ..Default::default()
    };
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
    let limits = SearchLimits {
        depth: Some(2),
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn test_node_limit() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        nodes: Some(1000),
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn test_multipv_limits_default() {
    let limits = SearchLimits::default();
    assert_eq!(limits.multi_pv, 1);
}

#[test]
fn test_search_multipv_2_returns_move() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(3),
        multi_pv: 2,
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn test_search_multipv_5_completes() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(2),
        multi_pv: 5,
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn search_limits_default() {
    let limits = SearchLimits::default();
    assert_eq!(limits.multi_pv, 1);
    assert!(!limits.infinite);
    assert!(limits.depth.is_none());
    assert!(limits.nodes.is_none());
    assert!(limits.movetime.is_none());
    assert!(limits.wtime.is_none());
    assert!(limits.btime.is_none());
    assert!(limits.movestogo.is_none());
}

#[test]
fn search_stats_default() {
    let stats = SearchStats::default();
    assert_eq!(stats.nodes, 0);
    assert_eq!(stats.qnodes, 0);
    assert_eq!(stats.tt_hits, 0);
    assert_eq!(stats.tt_cutoffs, 0);
}

#[test]
fn searcher_search_startpos_returns_legal_move() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(1),
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits).expect("search should return a move");
    let legals = pos.legal_moves();
    assert!(
        legals.iter().any(|m| m == &mv),
        "search must return a legal move"
    );
}

#[test]
fn searcher_search_depth_2_completes() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(2),
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn searcher_search_respects_node_limit() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        nodes: Some(500),
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn searcher_clear_then_search_still_works() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(2),
        ..Default::default()
    };
    let _ = searcher.search(&pos, limits.clone());
    searcher.clear();
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn searcher_set_hash_size_then_search_works() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    searcher.set_hash_size(16);
    let limits = SearchLimits {
        depth: Some(2),
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn searcher_search_from_fen_returns_legal_move() {
    let pos = from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(2),
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits).expect("search should return a move");
    let legals = pos.legal_moves();
    assert!(
        legals.iter().any(|m| m == &mv),
        "search must return a legal move"
    );
}

#[test]
fn searcher_infinite_limits_no_time_cutoff() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(3),
        infinite: true,
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}

#[test]
fn searcher_multi_pv_returns_move() {
    let pos = Chess::default();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(2),
        multi_pv: 3,
        ..Default::default()
    };
    let mv = searcher.search(&pos, limits);
    assert!(mv.is_some());
}
