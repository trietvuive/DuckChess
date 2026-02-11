//! Bitboard representation and operations
//!
//! A bitboard is a 64-bit integer where each bit represents a square on the chess board.
//! This allows for very fast set operations using bitwise operators.

use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, Shr};

/// A 64-bit bitboard representing squares on the chess board
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const ALL: Bitboard = Bitboard(!0u64);

    // File masks
    pub const FILE_A: Bitboard = Bitboard(0x0101010101010101);
    pub const FILE_B: Bitboard = Bitboard(0x0202020202020202);
    pub const FILE_C: Bitboard = Bitboard(0x0404040404040404);
    pub const FILE_D: Bitboard = Bitboard(0x0808080808080808);
    pub const FILE_E: Bitboard = Bitboard(0x1010101010101010);
    pub const FILE_F: Bitboard = Bitboard(0x2020202020202020);
    pub const FILE_G: Bitboard = Bitboard(0x4040404040404040);
    pub const FILE_H: Bitboard = Bitboard(0x8080808080808080);

    // Rank masks
    pub const RANK_1: Bitboard = Bitboard(0x00000000000000FF);
    pub const RANK_2: Bitboard = Bitboard(0x000000000000FF00);
    pub const RANK_3: Bitboard = Bitboard(0x0000000000FF0000);
    pub const RANK_4: Bitboard = Bitboard(0x00000000FF000000);
    pub const RANK_5: Bitboard = Bitboard(0x000000FF00000000);
    pub const RANK_6: Bitboard = Bitboard(0x0000FF0000000000);
    pub const RANK_7: Bitboard = Bitboard(0x00FF000000000000);
    pub const RANK_8: Bitboard = Bitboard(0xFF00000000000000);

    // Diagonal masks
    pub const DARK_SQUARES: Bitboard = Bitboard(0xAA55AA55AA55AA55);
    pub const LIGHT_SQUARES: Bitboard = Bitboard(0x55AA55AA55AA55AA);

    // Center squares
    pub const CENTER: Bitboard = Bitboard(0x0000001818000000);
    pub const EXTENDED_CENTER: Bitboard = Bitboard(0x00003C3C3C3C0000);

    /// Create a new bitboard from a raw u64 value
    #[inline]
    pub const fn new(value: u64) -> Self {
        Bitboard(value)
    }

    /// Create a bitboard with a single bit set at the given square index (0-63)
    #[inline]
    pub const fn from_square(sq: u8) -> Self {
        Bitboard(1u64 << sq)
    }

    /// Check if the bitboard is empty
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Check if the bitboard is not empty
    #[inline]
    pub const fn is_not_empty(self) -> bool {
        self.0 != 0
    }

    /// Count the number of set bits (population count)
    #[inline]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Get the index of the least significant bit (LSB)
    #[inline]
    pub const fn lsb(self) -> u8 {
        self.0.trailing_zeros() as u8
    }

    /// Get the index of the most significant bit (MSB)
    #[inline]
    pub const fn msb(self) -> u8 {
        63 - self.0.leading_zeros() as u8
    }

    /// Pop the least significant bit and return its index
    #[inline]
    pub fn pop_lsb(&mut self) -> u8 {
        let sq = self.lsb();
        self.0 &= self.0 - 1;
        sq
    }

    /// Check if a specific square is set
    #[inline]
    pub const fn contains(self, sq: u8) -> bool {
        (self.0 & (1u64 << sq)) != 0
    }

    /// Set a specific square
    #[inline]
    pub fn set(&mut self, sq: u8) {
        self.0 |= 1u64 << sq;
    }

    /// Clear a specific square
    #[inline]
    pub fn clear(&mut self, sq: u8) {
        self.0 &= !(1u64 << sq);
    }

    /// Toggle a specific square
    #[inline]
    pub fn toggle(&mut self, sq: u8) {
        self.0 ^= 1u64 << sq;
    }

    /// Shift the bitboard north (up the board for white)
    #[inline]
    pub const fn north(self) -> Self {
        Bitboard(self.0 << 8)
    }

    /// Shift the bitboard south (down the board for white)
    #[inline]
    pub const fn south(self) -> Self {
        Bitboard(self.0 >> 8)
    }

    /// Shift the bitboard east (right on the board)
    #[inline]
    pub const fn east(self) -> Self {
        Bitboard((self.0 << 1) & !Self::FILE_A.0)
    }

    /// Shift the bitboard west (left on the board)
    #[inline]
    pub const fn west(self) -> Self {
        Bitboard((self.0 >> 1) & !Self::FILE_H.0)
    }

    /// Shift the bitboard north-east
    #[inline]
    pub const fn north_east(self) -> Self {
        Bitboard((self.0 << 9) & !Self::FILE_A.0)
    }

    /// Shift the bitboard north-west
    #[inline]
    pub const fn north_west(self) -> Self {
        Bitboard((self.0 << 7) & !Self::FILE_H.0)
    }

    /// Shift the bitboard south-east
    #[inline]
    pub const fn south_east(self) -> Self {
        Bitboard((self.0 >> 7) & !Self::FILE_A.0)
    }

    /// Shift the bitboard south-west
    #[inline]
    pub const fn south_west(self) -> Self {
        Bitboard((self.0 >> 9) & !Self::FILE_H.0)
    }

    /// Get file mask for a given file index (0-7)
    #[inline]
    pub const fn file_mask(file: u8) -> Self {
        Bitboard(Self::FILE_A.0 << file)
    }

    /// Get rank mask for a given rank index (0-7)
    #[inline]
    pub const fn rank_mask(rank: u8) -> Self {
        Bitboard(Self::RANK_1.0 << (rank * 8))
    }

    /// Iterate over all set bits
    pub fn iter(self) -> BitboardIter {
        BitboardIter(self)
    }
}

/// Iterator over set bits in a bitboard
pub struct BitboardIter(Bitboard);

impl Iterator for BitboardIter {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.pop_lsb())
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.0.count() as usize;
        (count, Some(count))
    }
}

impl ExactSizeIterator for BitboardIter {}

// Implement bitwise operations
impl BitAnd for Bitboard {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for Bitboard {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitXor for Bitboard {
    type Output = Self;
    #[inline]
    fn bitxor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl BitXorAssign for Bitboard {
    #[inline]
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl Not for Bitboard {
    type Output = Self;
    #[inline]
    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl Shl<u8> for Bitboard {
    type Output = Self;
    #[inline]
    fn shl(self, rhs: u8) -> Self::Output {
        Bitboard(self.0 << rhs)
    }
}

impl Shr<u8> for Bitboard {
    type Output = Self;
    #[inline]
    fn shr(self, rhs: u8) -> Self::Output {
        Bitboard(self.0 >> rhs)
    }
}

impl fmt::Debug for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        for rank in (0..8).rev() {
            write!(f, "  {} ", rank + 1)?;
            for file in 0..8 {
                let sq = rank * 8 + file;
                if self.contains(sq) {
                    write!(f, "X ")?;
                } else {
                    write!(f, ". ")?;
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "    a b c d e f g h")?;
        Ok(())
    }
}

impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
