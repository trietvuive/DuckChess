//! UCI Protocol Tests
//!
//! These tests verify the UCI protocol implementation works correctly.

use duck_chess::core::board::Board;

// ============================================================================
// FEN Parsing Tests
// ============================================================================

#[test]
fn test_fen_startpos() {
    let board = Board::startpos();
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    assert_eq!(board.to_fen(), fen);
}

#[test]
fn test_fen_after_e4() {
    let mut board = Board::startpos();
    let moves = duck_chess::engine::movegen::MoveGen::generate_legal_moves(&board);
    
    // Find e2e4
    for mv in moves.iter() {
        if mv.to_uci() == "e2e4" {
            board.make_move(*mv);
            break;
        }
    }
    
    assert_eq!(
        board.to_fen(),
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
    );
}

#[test]
fn test_fen_kiwipete() {
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let board = Board::from_fen(fen).unwrap();
    assert_eq!(board.to_fen(), fen);
}

#[test]
fn test_fen_en_passant() {
    let fen = "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3";
    let board = Board::from_fen(fen).unwrap();
    assert_eq!(board.to_fen(), fen);
}

#[test]
fn test_fen_no_castling() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1";
    let board = Board::from_fen(fen).unwrap();
    assert_eq!(board.to_fen(), fen);
}

#[test]
fn test_fen_partial_castling() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Kq - 0 1";
    let board = Board::from_fen(fen).unwrap();
    assert_eq!(board.to_fen(), fen);
}

#[test]
fn test_fen_endgame() {
    let fen = "8/8/8/4k3/8/8/4K3/4Q3 w - - 0 1";
    let board = Board::from_fen(fen).unwrap();
    assert_eq!(board.to_fen(), fen);
}

// ============================================================================
// Move Parsing Tests
// ============================================================================

#[test]
fn test_move_parsing_normal() {
    use duck_chess::core::moves::Move;
    
    let mv = Move::from_uci("e2e4").unwrap();
    assert_eq!(mv.to_uci(), "e2e4");
}

#[test]
fn test_move_parsing_promotion() {
    use duck_chess::core::moves::Move;
    
    let mv = Move::from_uci("e7e8q").unwrap();
    assert_eq!(mv.to_uci(), "e7e8q");
    assert!(mv.is_promotion());
}

#[test]
fn test_move_parsing_castling() {
    use duck_chess::core::moves::Move;
    
    let mv = Move::from_uci("e1g1").unwrap();
    assert!(mv.is_castling());
    assert_eq!(mv.to_uci(), "e1g1");
}

// ============================================================================
// Board State Tests
// ============================================================================

#[test]
fn test_side_to_move() {
    use duck_chess::core::board::Color;
    
    let board = Board::startpos();
    assert_eq!(board.side_to_move, Color::White);
    
    let board = Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
    assert_eq!(board.side_to_move, Color::Black);
}

#[test]
fn test_castling_rights() {
    let board = Board::startpos();
    assert!(board.castling.can_castle_kingside(duck_chess::core::board::Color::White));
    assert!(board.castling.can_castle_queenside(duck_chess::core::board::Color::White));
    assert!(board.castling.can_castle_kingside(duck_chess::core::board::Color::Black));
    assert!(board.castling.can_castle_queenside(duck_chess::core::board::Color::Black));
}

#[test]
fn test_en_passant_square() {
    let board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
    assert!(board.en_passant.is_some());
    assert_eq!(board.en_passant.unwrap().to_algebraic(), "d6");
}

#[test]
fn test_halfmove_clock() {
    let board = Board::from_fen("8/8/8/8/8/8/8/4K2k w - - 50 100").unwrap();
    assert_eq!(board.halfmove_clock, 50);
    assert_eq!(board.fullmove_number, 100);
}

// ============================================================================
// Piece Placement Tests
// ============================================================================

#[test]
fn test_king_positions() {
    use duck_chess::core::board::{Color, Square};
    
    let board = Board::startpos();
    assert_eq!(board.king_square(Color::White), Square::E1);
    assert_eq!(board.king_square(Color::Black), Square::E8);
}

#[test]
fn test_piece_counts_startpos() {
    use duck_chess::core::board::PieceType;
    
    let board = Board::startpos();
    assert_eq!(board.pieces(PieceType::Pawn).count(), 16);
    assert_eq!(board.pieces(PieceType::Knight).count(), 4);
    assert_eq!(board.pieces(PieceType::Bishop).count(), 4);
    assert_eq!(board.pieces(PieceType::Rook).count(), 4);
    assert_eq!(board.pieces(PieceType::Queen).count(), 2);
    assert_eq!(board.pieces(PieceType::King).count(), 2);
}

// ============================================================================
// Special Position Tests
// ============================================================================

#[test]
fn test_insufficient_material_kk() {
    let board = Board::from_fen("8/8/8/4k3/8/8/8/4K3 w - - 0 1").unwrap();
    assert!(board.is_insufficient_material());
}

#[test]
fn test_insufficient_material_kbk() {
    let board = Board::from_fen("8/8/8/4k3/8/8/8/4KB2 w - - 0 1").unwrap();
    assert!(board.is_insufficient_material());
}

#[test]
fn test_insufficient_material_knk() {
    let board = Board::from_fen("8/8/8/4k3/8/8/8/4KN2 w - - 0 1").unwrap();
    assert!(board.is_insufficient_material());
}

#[test]
fn test_sufficient_material_krk() {
    let board = Board::from_fen("8/8/8/4k3/8/8/8/4KR2 w - - 0 1").unwrap();
    assert!(!board.is_insufficient_material());
}

#[test]
fn test_sufficient_material_kqk() {
    let board = Board::from_fen("8/8/8/4k3/8/8/8/4KQ2 w - - 0 1").unwrap();
    assert!(!board.is_insufficient_material());
}

#[test]
fn test_sufficient_material_kpk() {
    let board = Board::from_fen("8/8/8/4k3/8/4P3/8/4K3 w - - 0 1").unwrap();
    assert!(!board.is_insufficient_material());
}

// ============================================================================
// Zobrist Hash Tests
// ============================================================================

#[test]
fn test_zobrist_different_positions() {
    let board1 = Board::startpos();
    let board2 = Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
    
    assert_ne!(board1.hash, board2.hash);
}

#[test]
fn test_zobrist_same_position() {
    let board1 = Board::startpos();
    let board2 = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    
    assert_eq!(board1.hash, board2.hash);
}

