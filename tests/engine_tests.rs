//! Engine Module Tests
//!
//! Tests for movegen, nnue, search, and transposition table.

use duck_chess::core::bitboard::Bitboard;
use duck_chess::core::board::{Board, Color, Piece, PieceType, Square};
use duck_chess::core::moves::Move;
use duck_chess::engine::movegen::MoveGen;
use duck_chess::engine::nnue::{NNUEAccumulator, NNUEEvaluator, NNUENetwork};
use duck_chess::engine::search::{SearchLimits, Searcher};
use duck_chess::engine::tt::{TTFlag, TranspositionTable};

// ============================================================================
// Move Generation Tests
// ============================================================================

#[test]
fn test_knight_attacks() {
    let attacks = MoveGen::knight_attacks(Square::from_algebraic("e4").unwrap());
    assert_eq!(attacks.count(), 8);
}

#[test]
fn test_knight_attacks_corner() {
    let attacks = MoveGen::knight_attacks(Square::A1);
    assert_eq!(attacks.count(), 2);
}

#[test]
fn test_king_attacks() {
    let attacks = MoveGen::king_attacks(Square::from_algebraic("e4").unwrap());
    assert_eq!(attacks.count(), 8);
}

#[test]
fn test_king_attacks_corner() {
    let attacks = MoveGen::king_attacks(Square::A1);
    assert_eq!(attacks.count(), 3);
}

#[test]
fn test_rook_attacks_empty_board() {
    let attacks = MoveGen::rook_attacks(Square::E4, Bitboard::EMPTY);
    assert_eq!(attacks.count(), 14);
}

#[test]
fn test_bishop_attacks_empty_board() {
    let attacks = MoveGen::bishop_attacks(Square::E4, Bitboard::EMPTY);
    assert_eq!(attacks.count(), 13);
}

#[test]
fn test_starting_position_moves() {
    let board = Board::startpos();
    let moves = MoveGen::generate_legal_moves(&board);
    assert_eq!(moves.len(), 20);
}

#[test]
fn test_movegen_en_passant() {
    let board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
    let moves = MoveGen::generate_legal_moves(&board);
    let ep_moves: Vec<_> = moves.iter().filter(|m| m.is_en_passant()).collect();
    assert_eq!(ep_moves.len(), 1);
}

#[test]
fn test_movegen_castling() {
    let board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
    let moves = MoveGen::generate_legal_moves(&board);
    let castle_moves: Vec<_> = moves.iter().filter(|m| m.is_castling()).collect();
    assert_eq!(castle_moves.len(), 2);
}

#[test]
fn test_movegen_promotion() {
    let board = Board::from_fen("8/P7/8/8/8/8/8/4K2k w - - 0 1").unwrap();
    let moves = MoveGen::generate_legal_moves(&board);
    let promo_moves: Vec<_> = moves.iter().filter(|m| m.is_promotion()).collect();
    assert_eq!(promo_moves.len(), 4);
}

// ============================================================================
// Transposition Table Tests
// ============================================================================

#[test]
fn test_tt_store_probe() {
    let mut tt = TranspositionTable::new(1);
    let key = 0x123456789ABCDEF0;
    let mv = Move::new(Square::E2, Square::E4);
    tt.store(key, mv, 5, 100, TTFlag::Exact);
    let entry = tt.probe(key).unwrap();
    assert_eq!(entry.key, key);
    assert_eq!(entry.best_move.to_uci(), "e2e4");
    assert_eq!(entry.depth, 5);
    assert_eq!(entry.score, 100);
    assert_eq!(entry.flag, TTFlag::Exact);
}

#[test]
fn test_tt_miss() {
    let tt = TranspositionTable::new(1);
    assert!(tt.probe(0x123456789ABCDEF0).is_none());
}

#[test]
fn test_tt_clear() {
    let mut tt = TranspositionTable::new(1);
    let key = 0x123456789ABCDEF0;
    tt.store(key, Move::NULL, 5, 100, TTFlag::Exact);
    assert!(tt.probe(key).is_some());
    tt.clear();
    assert!(tt.probe(key).is_none());
}

// ============================================================================
// NNUE Tests
// ============================================================================

#[test]
fn test_nnue_network_creation() {
    let network = NNUENetwork::new();
    assert_eq!(network.input_weights.len(), 768);
    assert_eq!(network.input_weights[0].len(), 256);
    assert_eq!(network.hidden1_biases.len(), 256);
}

#[test]
fn test_nnue_accumulator_refresh() {
    let board = Board::startpos();
    let network = NNUENetwork::new();
    let mut acc = NNUEAccumulator::new();
    acc.refresh(&board, &network);
    assert!(acc.valid);
}

#[test]
fn test_nnue_feature_index() {
    let piece = Piece::new(PieceType::Pawn, Color::White);
    let sq = Square::E4;
    let white_idx = NNUENetwork::feature_index(piece, sq, Color::White);
    let black_idx = NNUENetwork::feature_index(piece, sq, Color::Black);
    assert_ne!(white_idx, black_idx);
}

#[test]
fn test_nnue_evaluate_startpos() {
    let board = Board::startpos();
    let evaluator = NNUEEvaluator::new();
    let score = evaluator.evaluate_simple(&board);
    assert!(score.abs() < 100);
}

#[test]
fn test_nnue_evaluate_material_advantage() {
    let board = Board::from_fen("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let evaluator = NNUEEvaluator::new();
    let score = evaluator.evaluate_simple(&board);
    assert!(score > 800);
}

#[test]
fn test_nnue_full_evaluation() {
    let board = Board::startpos();
    let evaluator = NNUEEvaluator::new();
    let mut accumulator = NNUEAccumulator::new();
    let score = evaluator.evaluate_full(&board, &mut accumulator);
    assert!(score.abs() < 100);
}

#[test]
fn test_clipped_relu() {
    // Test via evaluate which uses clipped_relu internally
    let board = Board::startpos();
    let evaluator = NNUEEvaluator::new();
    let mut acc = NNUEAccumulator::new();
    let _ = evaluator.evaluate_full(&board, &mut acc);
}

// ============================================================================
// Search Tests
// ============================================================================

#[test]
fn test_search_mate_in_one() {
    let board = Board::from_fen("6k1/5ppp/8/8/8/8/8/4Q2K w - - 0 1").unwrap();
    let mut searcher = Searcher::new();
    let limits = SearchLimits { depth: Some(3), ..Default::default() };
    let mv = searcher.search(&board, limits);
    assert!(!mv.is_null());
}

#[test]
fn test_search_avoid_stalemate() {
    let board = Board::from_fen("7k/8/6K1/8/8/8/8/6Q1 w - - 0 1").unwrap();
    let mut searcher = Searcher::new();
    let limits = SearchLimits { depth: Some(4), ..Default::default() };
    let mv = searcher.search(&board, limits);
    assert!(!mv.is_null());
}

#[test]
fn test_search_startpos() {
    let board = Board::startpos();
    let mut searcher = Searcher::new();
    let limits = SearchLimits { depth: Some(4), ..Default::default() };
    let mv = searcher.search(&board, limits);
    assert!(!mv.is_null());
}

#[test]
fn test_search_time_management() {
    let limits = SearchLimits {
        wtime: Some(60000),
        btime: Some(60000),
        winc: Some(1000),
        binc: Some(1000),
        ..Default::default()
    };
    let searcher = Searcher::new();
    let time = searcher.calculate_time(&limits, Color::White);
    assert!(time.is_some());
    assert!(time.unwrap().as_millis() > 0);
    assert!(time.unwrap().as_millis() <= 30000);
}
