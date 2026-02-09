//! Core chess types and representations
//!
//! This module contains the fundamental building blocks of the chess engine:
//! - Bitboard representation
//! - Board state and position
//! - Move encoding
//! - Zobrist hashing

pub mod bitboard;
pub mod board;
pub mod moves;
pub mod zobrist;

pub use bitboard::{Bitboard, BitboardIter};
pub use board::{Board, CastlingRights, Color, Piece, PieceType, Square};
pub use moves::{Move, MoveList, MoveType};
pub use zobrist::ZobristKeys;

