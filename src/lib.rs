//! DuckChess - A UCI chess engine with NNUE evaluation
//!
//! This is a from-scratch implementation of a chess engine featuring:
//! - Bitboard-based board representation
//! - Magic bitboard move generation
//! - NNUE (Efficiently Updatable Neural Network) evaluation
//! - Alpha-beta search with various optimizations
//!
//! # Module Structure
//!
//! - `core` - Fundamental types: bitboards, board, moves, zobrist hashing
//! - `engine` - Engine logic: move generation, evaluation, search
//! - `uci` - UCI protocol implementation

pub mod core;
pub mod engine;
pub mod uci;

// Re-export commonly used types at the crate root
pub use core::{Bitboard, Board, CastlingRights, Color, Move, MoveList, Piece, PieceType, Square};
pub use engine::{MoveGen, Searcher, SearchLimits};
pub use uci::UCI;
