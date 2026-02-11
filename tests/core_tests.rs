//! Core Module Tests
//!
//! Tests for bitboard, board, moves, and zobrist modules.

use duck_chess::core::bitboard::Bitboard;
use duck_chess::core::board::{Board, CastlingRights, Color, Piece, PieceType, Square};
use duck_chess::core::moves::Move;
use duck_chess::core::zobrist::ZobristKeys;
use std::collections::HashSet;

// ============================================================================
// Bitboard Tests
// ============================================================================

#[test]
fn test_bitboard_empty() {
    let bb = Bitboard::EMPTY;
    assert!(bb.is_empty());
    assert_eq!(bb.count(), 0);
}

#[test]
fn test_bitboard_from_square() {
    let bb = Bitboard::from_square(0);
    assert!(!bb.is_empty());
    assert_eq!(bb.count(), 1);
    assert!(bb.contains(0));
    assert!(!bb.contains(1));
}

#[test]
fn test_bitboard_lsb() {
    let bb = Bitboard::new(0b1010);
    assert_eq!(bb.lsb(), 1);
}

#[test]
fn test_bitboard_pop_lsb() {
    let mut bb = Bitboard::new(0b1010);
    assert_eq!(bb.pop_lsb(), 1);
    assert_eq!(bb.0, 0b1000);
    assert_eq!(bb.pop_lsb(), 3);
    assert!(bb.is_empty());
}

#[test]
fn test_bitboard_count() {
    assert_eq!(Bitboard::EMPTY.count(), 0);
    assert_eq!(Bitboard::ALL.count(), 64);
    assert_eq!(Bitboard::FILE_A.count(), 8);
    assert_eq!(Bitboard::RANK_1.count(), 8);
}

#[test]
fn test_bitboard_shifts() {
    let a1 = Bitboard::from_square(0);
    assert_eq!(a1.north(), Bitboard::from_square(8));
    assert_eq!(a1.east(), Bitboard::from_square(1));
    let h1 = Bitboard::from_square(7);
    assert_eq!(h1.east(), Bitboard::EMPTY);
    let a8 = Bitboard::from_square(56);
    assert_eq!(a8.north(), Bitboard::EMPTY);
}

#[test]
fn test_bitboard_iter() {
    let bb = Bitboard::new(0b10101);
    let squares: Vec<u8> = bb.iter().collect();
    assert_eq!(squares, vec![0, 2, 4]);
}

#[test]
fn test_file_and_rank_masks() {
    assert_eq!(Bitboard::file_mask(0), Bitboard::FILE_A);
    assert_eq!(Bitboard::file_mask(7), Bitboard::FILE_H);
    assert_eq!(Bitboard::rank_mask(0), Bitboard::RANK_1);
    assert_eq!(Bitboard::rank_mask(7), Bitboard::RANK_8);
}

#[test]
fn test_bitboard_operations() {
    let a = Bitboard::new(0b1100);
    let b = Bitboard::new(0b1010);
    assert_eq!((a & b).0, 0b1000);
    assert_eq!((a | b).0, 0b1110);
    assert_eq!((a ^ b).0, 0b0110);
    assert_eq!((!Bitboard::EMPTY).0, !0u64);
}

// ============================================================================
// Square Tests
// ============================================================================

#[test]
fn test_square_from_algebraic() {
    assert_eq!(Square::from_algebraic("a1"), Some(Square(0)));
    assert_eq!(Square::from_algebraic("h8"), Some(Square(63)));
    assert_eq!(Square::from_algebraic("e4"), Some(Square(28)));
    assert_eq!(Square::from_algebraic("invalid"), None);
}

#[test]
fn test_square_to_algebraic() {
    assert_eq!(Square(0).to_algebraic(), "a1");
    assert_eq!(Square(63).to_algebraic(), "h8");
    assert_eq!(Square(28).to_algebraic(), "e4");
}

#[test]
fn test_square_file_rank() {
    let sq = Square::from_file_rank(4, 3);
    assert_eq!(sq.file(), 4);
    assert_eq!(sq.rank(), 3);
    assert_eq!(sq.0, 28);
}

// ============================================================================
// Board Tests
// ============================================================================

#[test]
fn test_startpos() {
    let board = Board::startpos();
    assert_eq!(board.side_to_move, Color::White);
    assert!(board.castling.can_castle_kingside(Color::White));
    assert!(board.castling.can_castle_queenside(Color::White));
    assert!(board.castling.can_castle_kingside(Color::Black));
    assert!(board.castling.can_castle_queenside(Color::Black));
    assert!(board.en_passant.is_none());
}

#[test]
fn test_fen_roundtrip() {
    let fens = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/8/8/8/8/8/8/4K2k w - - 0 1",
    ];
    for fen in fens {
        let board = Board::from_fen(fen).unwrap();
        assert_eq!(board.to_fen(), fen);
    }
}

#[test]
fn test_piece_counts() {
    let board = Board::startpos();
    assert_eq!(board.pieces(PieceType::Pawn).count(), 16);
    assert_eq!(board.pieces(PieceType::Knight).count(), 4);
    assert_eq!(board.pieces(PieceType::Bishop).count(), 4);
    assert_eq!(board.pieces(PieceType::Rook).count(), 4);
    assert_eq!(board.pieces(PieceType::Queen).count(), 2);
    assert_eq!(board.pieces(PieceType::King).count(), 2);
}

#[test]
fn test_king_square() {
    let board = Board::startpos();
    assert_eq!(board.king_square(Color::White), Square::E1);
    assert_eq!(board.king_square(Color::Black), Square::E8);
}

#[test]
fn test_insufficient_material() {
    let board = Board::from_fen("8/8/8/4k3/8/8/8/4K3 w - - 0 1").unwrap();
    assert!(board.is_insufficient_material());
    let board = Board::from_fen("8/8/8/4k3/8/8/8/4KB2 w - - 0 1").unwrap();
    assert!(board.is_insufficient_material());
    let board = Board::from_fen("8/8/8/4k3/8/8/8/4KR2 w - - 0 1").unwrap();
    assert!(!board.is_insufficient_material());
}

// ============================================================================
// Move Tests
// ============================================================================

#[test]
fn test_move_encoding() {
    let mv = Move::new(Square::E2, Square::E4);
    assert_eq!(mv.from(), Square::E2);
    assert_eq!(mv.to(), Square::E4);
    assert!(!mv.is_promotion());
    assert!(!mv.is_castling());
    assert!(!mv.is_en_passant());
}

#[test]
fn test_promotion_move() {
    let mv = Move::new_promotion(Square::E7, Square::E8, PieceType::Queen, false);
    assert_eq!(mv.from(), Square::E7);
    assert_eq!(mv.to(), Square::E8);
    assert!(mv.is_promotion());
    assert_eq!(mv.promotion_piece(), Some(PieceType::Queen));
}

#[test]
fn test_castling_move() {
    let mv = Move::new_castling(Square::E1, Square::G1);
    assert!(mv.is_castling());
    assert_eq!(mv.from(), Square::E1);
    assert_eq!(mv.to(), Square::G1);
}

#[test]
fn test_en_passant_move() {
    let mv = Move::new_en_passant(Square::E5, Square::D6);
    assert!(mv.is_en_passant());
    assert!(mv.is_capture());
}

#[test]
fn test_uci_conversion() {
    let mv = Move::new(Square::E2, Square::E4);
    assert_eq!(mv.to_uci(), "e2e4");
    let mv = Move::new_promotion(Square::E7, Square::E8, PieceType::Queen, false);
    assert_eq!(mv.to_uci(), "e7e8q");
    let parsed = Move::from_uci("e2e4").unwrap();
    assert_eq!(parsed.from(), Square::E2);
    assert_eq!(parsed.to(), Square::E4);
}

#[test]
fn test_move_list() {
    let mut list = duck_chess::core::moves::MoveList::new();
    assert!(list.is_empty());
    list.push(Move::new(Square::E2, Square::E4));
    list.push(Move::new(Square::D2, Square::D4));
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].to_uci(), "e2e4");
    assert_eq!(list[1].to_uci(), "d2d4");
}

// ============================================================================
// Zobrist Tests
// ============================================================================

#[test]
fn test_zobrist_keys_unique() {
    let keys = ZobristKeys::instance();
    let mut seen = HashSet::new();
    // Just check first few to avoid too much iteration
    for i in 0..6 {
        for j in 0..64 {
            let piece = Piece::new(PieceType::ALL[i], Color::White);
            let key = keys.piece(piece, Square(j as u8));
            assert!(seen.insert(key), "Duplicate Zobrist key found");
        }
    }
}

#[test]
fn test_zobrist_piece_key() {
    let keys = ZobristKeys::instance();
    let piece = Piece::new(PieceType::Pawn, Color::White);
    let key1 = keys.piece(piece, Square::E2);
    let key2 = keys.piece(piece, Square::E4);
    assert_ne!(key1, key2);
}

#[test]
fn test_zobrist_side_key() {
    let keys = ZobristKeys::instance();
    assert_ne!(keys.side_to_move(), 0);
}

#[test]
fn test_zobrist_castling_keys() {
    let keys = ZobristKeys::instance();
    let k1 = keys.castling(CastlingRights::new(0b0001));
    let k2 = keys.castling(CastlingRights::new(0b0010));
    assert_ne!(k1, k2);
}
