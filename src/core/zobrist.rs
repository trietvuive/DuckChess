//! Zobrist hashing for position identification
//!
//! Zobrist hashing uses random bitstrings XOR'd together to create
//! a unique hash for each chess position.

use super::board::{CastlingRights, Piece, Square};

/// Zobrist random keys
pub struct ZobristKeys {
    /// Keys for each piece on each square [piece][color][square]
    pieces: [[u64; 64]; 12],
    /// Key for black to move
    side: u64,
    /// Keys for castling rights (one for each of the 16 possible combinations)
    castling: [u64; 16],
    /// Keys for en passant file
    en_passant: [u64; 8],
}

impl ZobristKeys {
    /// Get the global Zobrist keys instance
    pub fn instance() -> &'static ZobristKeys {
        static KEYS: std::sync::OnceLock<ZobristKeys> = std::sync::OnceLock::new();
        KEYS.get_or_init(ZobristKeys::new)
    }

    /// Generate new Zobrist keys using a PRNG
    fn new() -> Self {
        let mut rng = SimpleRng::new(0x12345678_9ABCDEF0);
        
        let mut pieces = [[0u64; 64]; 12];
        for piece_keys in pieces.iter_mut() {
            for sq_key in piece_keys.iter_mut() {
                *sq_key = rng.next();
            }
        }

        let side = rng.next();

        let mut castling = [0u64; 16];
        for key in castling.iter_mut() {
            *key = rng.next();
        }

        let mut en_passant = [0u64; 8];
        for key in en_passant.iter_mut() {
            *key = rng.next();
        }

        ZobristKeys {
            pieces,
            side,
            castling,
            en_passant,
        }
    }

    /// Get the key for a piece on a square
    #[inline]
    pub fn piece(&self, piece: Piece, sq: Square) -> u64 {
        let idx = piece.color as usize * 6 + piece.piece_type as usize;
        self.pieces[idx][sq.index()]
    }

    /// Get the side to move key
    #[inline]
    pub fn side_to_move(&self) -> u64 {
        self.side
    }

    /// Get the castling rights key
    #[inline]
    pub fn castling(&self, rights: CastlingRights) -> u64 {
        self.castling[rights.raw() as usize]
    }

    /// Get the en passant key
    #[inline]
    pub fn en_passant(&self, sq: Square) -> u64 {
        self.en_passant[sq.file() as usize]
    }
}

/// Simple PRNG for generating Zobrist keys
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        SimpleRng { state: seed }
    }

    fn next(&mut self) -> u64 {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::board::{CastlingRights, Color, Piece, PieceType, Square};

    #[test]
    fn test_zobrist_keys_unique() {
        let keys = ZobristKeys::instance();
        
        // Check that piece keys are unique
        let mut seen = std::collections::HashSet::new();
        for piece_keys in &keys.pieces {
            for key in piece_keys {
                assert!(seen.insert(*key), "Duplicate Zobrist key found");
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
}

