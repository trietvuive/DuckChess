//! Move representation
//!
//! Moves are encoded in a compact 16-bit format:
//! - bits 0-5: from square (0-63)
//! - bits 6-11: to square (0-63)
//! - bits 12-13: promotion piece (0=Knight, 1=Bishop, 2=Rook, 3=Queen)
//! - bits 14-15: move type (0=Normal, 1=Promotion, 2=En Passant, 3=Castling)

use super::board::{PieceType, Square};
use std::fmt;

/// Move type flags
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MoveType {
    Normal = 0,
    Promotion = 1,
    EnPassant = 2,
    Castling = 3,
}

/// A chess move encoded in 16 bits
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Move(u16);

impl Move {
    pub const NULL: Move = Move(0);

    const FROM_MASK: u16 = 0x003F;
    const TO_MASK: u16 = 0x0FC0;
    const TO_SHIFT: u16 = 6;
    const PROMO_MASK: u16 = 0x3000;
    const PROMO_SHIFT: u16 = 12;
    const TYPE_MASK: u16 = 0xC000;
    const TYPE_SHIFT: u16 = 14;

    // Flag to indicate capture (stored in upper bit of promotion field when not promoting)
    const CAPTURE_FLAG: u16 = 0x1000;

    /// Create a normal move
    #[inline]
    pub const fn new(from: Square, to: Square) -> Self {
        Move((from.0 as u16) | ((to.0 as u16) << Self::TO_SHIFT))
    }

    /// Create a capture move
    #[inline]
    pub const fn new_capture(from: Square, to: Square) -> Self {
        Move((from.0 as u16) | ((to.0 as u16) << Self::TO_SHIFT) | Self::CAPTURE_FLAG)
    }

    /// Create a promotion move
    #[inline]
    pub const fn new_promotion(from: Square, to: Square, piece: PieceType, is_capture: bool) -> Self {
        let promo_bits = match piece {
            PieceType::Knight => 0,
            PieceType::Bishop => 1,
            PieceType::Rook => 2,
            PieceType::Queen => 3,
            _ => 0,
        };
        let mut bits = (from.0 as u16)
            | ((to.0 as u16) << Self::TO_SHIFT)
            | (promo_bits << Self::PROMO_SHIFT)
            | ((MoveType::Promotion as u16) << Self::TYPE_SHIFT);
        if is_capture {
            bits |= Self::CAPTURE_FLAG;
        }
        Move(bits)
    }

    /// Create an en passant move
    #[inline]
    pub const fn new_en_passant(from: Square, to: Square) -> Self {
        Move(
            (from.0 as u16)
                | ((to.0 as u16) << Self::TO_SHIFT)
                | ((MoveType::EnPassant as u16) << Self::TYPE_SHIFT)
                | Self::CAPTURE_FLAG, // En passant is always a capture
        )
    }

    /// Create a castling move
    #[inline]
    pub const fn new_castling(from: Square, to: Square) -> Self {
        Move(
            (from.0 as u16)
                | ((to.0 as u16) << Self::TO_SHIFT)
                | ((MoveType::Castling as u16) << Self::TYPE_SHIFT),
        )
    }

    /// Get the source square
    #[inline]
    pub const fn from(self) -> Square {
        Square((self.0 & Self::FROM_MASK) as u8)
    }

    /// Get the destination square
    #[inline]
    pub const fn to(self) -> Square {
        Square(((self.0 & Self::TO_MASK) >> Self::TO_SHIFT) as u8)
    }

    /// Get the move type
    #[inline]
    pub const fn move_type(self) -> MoveType {
        match (self.0 & Self::TYPE_MASK) >> Self::TYPE_SHIFT {
            0 => MoveType::Normal,
            1 => MoveType::Promotion,
            2 => MoveType::EnPassant,
            3 => MoveType::Castling,
            _ => MoveType::Normal,
        }
    }

    /// Check if this is a promotion
    #[inline]
    pub const fn is_promotion(self) -> bool {
        matches!(self.move_type(), MoveType::Promotion)
    }

    /// Check if this is en passant
    #[inline]
    pub const fn is_en_passant(self) -> bool {
        matches!(self.move_type(), MoveType::EnPassant)
    }

    /// Check if this is castling
    #[inline]
    pub const fn is_castling(self) -> bool {
        matches!(self.move_type(), MoveType::Castling)
    }

    /// Check if this is a capture
    #[inline]
    pub const fn is_capture(self) -> bool {
        (self.0 & Self::CAPTURE_FLAG) != 0 || self.is_en_passant()
    }

    /// Get the promotion piece type (if any)
    #[inline]
    pub const fn promotion_piece(self) -> Option<PieceType> {
        if !self.is_promotion() {
            return None;
        }
        Some(match (self.0 & Self::PROMO_MASK) >> Self::PROMO_SHIFT {
            0 => PieceType::Knight,
            1 => PieceType::Bishop,
            2 => PieceType::Rook,
            3 => PieceType::Queen,
            _ => PieceType::Queen,
        })
    }

    /// Get the raw 16-bit value
    #[inline]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Check if this is a null move
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Convert to UCI notation (e.g., "e2e4", "e7e8q")
    pub fn to_uci(self) -> String {
        let from = self.from().to_algebraic();
        let to = self.to().to_algebraic();
        let promo = match self.promotion_piece() {
            Some(PieceType::Knight) => "n",
            Some(PieceType::Bishop) => "b",
            Some(PieceType::Rook) => "r",
            Some(PieceType::Queen) => "q",
            _ => "",
        };
        format!("{}{}{}", from, to, promo)
    }

    /// Parse from UCI notation
    pub fn from_uci(s: &str) -> Option<Self> {
        if s.len() < 4 || s.len() > 5 {
            return None;
        }

        let from = Square::from_algebraic(&s[0..2])?;
        let to = Square::from_algebraic(&s[2..4])?;

        if s.len() == 5 {
            let promo = match s.chars().nth(4)? {
                'n' | 'N' => PieceType::Knight,
                'b' | 'B' => PieceType::Bishop,
                'r' | 'R' => PieceType::Rook,
                'q' | 'Q' => PieceType::Queen,
                _ => return None,
            };
            Some(Move::new_promotion(from, to, promo, false))
        } else {
            // Check for castling
            if from == Square::E1 && (to == Square::G1 || to == Square::C1) {
                Some(Move::new_castling(from, to))
            } else if from == Square::E8 && (to == Square::G8 || to == Square::C8) {
                Some(Move::new_castling(from, to))
            } else {
                Some(Move::new(from, to))
            }
        }
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_uci())
    }
}

/// A list of moves (stack-allocated for performance)
#[derive(Clone)]
pub struct MoveList {
    moves: [Move; 256],
    len: usize,
}

impl MoveList {
    /// Create a new empty move list
    pub fn new() -> Self {
        MoveList {
            moves: [Move::NULL; 256],
            len: 0,
        }
    }

    /// Add a move to the list
    #[inline]
    pub fn push(&mut self, mv: Move) {
        if self.len < 256 {
            self.moves[self.len] = mv;
            self.len += 1;
        }
    }

    /// Get the number of moves
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the list is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a move by index
    #[inline]
    pub fn get(&self, index: usize) -> Option<Move> {
        if index < self.len {
            Some(self.moves[index])
        } else {
            None
        }
    }

    /// Iterate over moves
    pub fn iter(&self) -> impl Iterator<Item = &Move> {
        self.moves[..self.len].iter()
    }

    /// Clear the list
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;
    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}

