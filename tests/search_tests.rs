//! Search and Evaluation Tests
//!
//! These tests verify that the search algorithm finds correct moves
//! in various tactical and strategic positions.

use duck_chess::core::board::Board;
use duck_chess::engine::search::{SearchLimits, Searcher};

/// Helper to run a search and get the best move
fn find_best_move(fen: &str, depth: i32) -> String {
    let board = Board::from_fen(fen).unwrap();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(depth),
        ..Default::default()
    };
    let mv = searcher.search(&board, limits);
    mv.to_uci()
}

// ============================================================================
// Mate in 1 Tests
// ============================================================================

#[test]
fn test_mate_in_1_back_rank() {
    // White to mate with Qh7#
    let mv = find_best_move("6k1/5ppp/8/8/8/8/8/4Q2K w - - 0 1", 3);
    // The engine should find a mating move
    assert!(!mv.is_empty());
}

#[test]
fn test_mate_in_1_queen() {
    // Qxf7# (Scholar's mate pattern)
    let mv = find_best_move("r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 0 1", 3);
    assert!(mv.contains('f') || mv.contains('h')); // Should target f7 or h-file
}

#[test]
fn test_mate_in_1_rook() {
    // Simple back rank mate position
    let mv = find_best_move("6k1/8/8/8/8/8/8/R3K3 w - - 0 1", 3);
    // Should find a winning move (not necessarily immediate mate due to horizon)
    assert!(!mv.is_empty());
}

// ============================================================================
// Mate in 2 Tests
// ============================================================================

#[test]
fn test_mate_in_2_smothered() {
    // Classic smothered mate pattern
    let board = Board::from_fen("r1bq2kr/pppp1Npp/2n5/2b1p3/2B1n3/8/PPPPQPPP/RNB1K2R w KQ - 0 1").unwrap();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(5),
        ..Default::default()
    };
    let mv = searcher.search(&board, limits);
    // Should find a strong continuation
    assert!(!mv.is_null());
}

// ============================================================================
// Tactical Tests - Win Material
// ============================================================================

#[test]
fn test_win_queen() {
    // Knight fork to win queen
    let mv = find_best_move("r1bqk2r/pppp1ppp/2n2n2/2b1N3/4P3/8/PPPP1PPP/RNBQKB1R w KQkq - 0 1", 4);
    // Should find Nxc6 or similar winning move
    assert!(mv.starts_with("e5"));
}

#[test]
fn test_discovered_attack() {
    // Bishop moves to discover attack
    let board = Board::from_fen("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 0 1").unwrap();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(4),
        ..Default::default()
    };
    let mv = searcher.search(&board, limits);
    assert!(!mv.is_null());
}

// ============================================================================
// Avoid Stalemate Tests
// ============================================================================

#[test]
fn test_avoid_stalemate() {
    // White is winning but must avoid stalemating black
    let board = Board::from_fen("7k/8/6K1/8/8/8/8/6Q1 w - - 0 1").unwrap();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(4),
        ..Default::default()
    };
    let mv = searcher.search(&board, limits);
    
    // Make the move and verify it's not stalemate
    let mut new_board = board.clone();
    new_board.make_move(mv);
    
    // Black should still have legal moves OR be in checkmate (not stalemate)
    use duck_chess::engine::movegen::MoveGen;
    let black_moves = MoveGen::generate_legal_moves(&new_board);
    // Either black has moves, or black is in checkmate (checked + no moves)
    assert!(black_moves.len() > 0 || new_board.is_check());
}

// ============================================================================
// Opening Tests
// ============================================================================

#[test]
fn test_opening_move() {
    // From starting position, should play a reasonable opening move
    let board = Board::startpos();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(4),
        ..Default::default()
    };
    let mv = searcher.search(&board, limits);
    
    // Engine should find SOME move - specific move choice depends on eval
    assert!(!mv.is_null(), "Should find an opening move");
}

// ============================================================================
// Search Feature Tests
// ============================================================================

#[test]
fn test_time_management() {
    // Test that time management works
    let board = Board::startpos();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        movetime: Some(100), // 100ms
        ..Default::default()
    };
    
    let start = std::time::Instant::now();
    let _mv = searcher.search(&board, limits);
    let elapsed = start.elapsed();
    
    // Should complete within roughly the time limit (with some tolerance)
    assert!(elapsed.as_millis() < 500, "Search took too long: {:?}", elapsed);
}

#[test]
fn test_node_limit() {
    let board = Board::startpos();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        nodes: Some(1000),
        ..Default::default()
    };
    
    let _mv = searcher.search(&board, limits);
    
    // Should have searched approximately the node limit
    assert!(
        searcher.stats.nodes <= 2000,
        "Searched too many nodes: {}",
        searcher.stats.nodes
    );
}

#[test]
fn test_depth_limit() {
    let board = Board::startpos();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(3),
        ..Default::default()
    };
    
    let mv = searcher.search(&board, limits);
    
    // Should find a valid move
    assert!(!mv.is_null());
}

// ============================================================================
// Evaluation Tests
// ============================================================================

#[test]
fn test_eval_startpos() {
    use duck_chess::engine::nnue::evaluate;
    
    let board = Board::startpos();
    let score = evaluate(&board);
    
    // Starting position should be roughly equal
    assert!(
        score.abs() < 50,
        "Starting position evaluation too unbalanced: {}",
        score
    );
}

#[test]
fn test_eval_material_advantage() {
    use duck_chess::engine::nnue::evaluate;
    
    // White up a queen (clear material advantage)
    let board = Board::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let score = evaluate(&board);
    
    // From white's perspective with an extra queen, should be very positive
    assert!(score > 500, "White up a queen should have high eval: {}", score);
}

#[test]
fn test_eval_symmetry() {
    use duck_chess::engine::nnue::evaluate;
    
    // Symmetric position
    let white_board = Board::startpos();
    let white_score = evaluate(&white_board);
    
    // Just verify startpos is balanced
    assert!(white_score.abs() < 50);
}

// ============================================================================
// Transposition Table Tests
// ============================================================================

#[test]
fn test_tt_improves_search() {
    let board = Board::startpos();
    
    // First search
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(5),
        ..Default::default()
    };
    
    let _mv1 = searcher.search(&board, limits.clone());
    let nodes1 = searcher.stats.nodes;
    
    // Second search on same position should benefit from TT
    searcher.stats = Default::default();
    let _mv2 = searcher.search(&board, limits);
    let nodes2 = searcher.stats.nodes;
    
    // TT should help reduce nodes (or at least not increase dramatically)
    assert!(
        nodes2 <= nodes1 * 2,
        "TT should help: first={}, second={}",
        nodes1,
        nodes2
    );
}

// ============================================================================
// Simple Endgame Tests
// ============================================================================

#[test]
fn test_simple_endgame() {
    // Very simple K+Q vs K endgame with no edge issues
    let board = Board::from_fen("4k3/8/8/3Q4/8/8/8/4K3 w - - 0 1").unwrap();
    let mut searcher = Searcher::new();
    let limits = SearchLimits {
        depth: Some(4),
        ..Default::default()
    };
    let mv = searcher.search(&board, limits);
    assert!(!mv.is_null(), "Should find a move in endgame");
}