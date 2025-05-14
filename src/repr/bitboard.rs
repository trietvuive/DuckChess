use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bitboard(pub u64);

// Bitboard is a 64-bit representation of a chess board
// Each bit corresponds to a square on the board, with bit 0 representing a1,
// bit 1 representing b1, and so on up to bit 63 representing h8.
// This allows for efficient board operations using bitwise operations.
impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const FULL: Bitboard = Bitboard(0xFFFFFFFFFFFFFFFF);

    // Create a bitboard with a single bit set at the given square
    pub fn from_square(square: u8) -> Bitboard {
        if square >= 64 {
            Bitboard::EMPTY
        } else {
            Bitboard(1u64 << square)
        }
    }

    // Check if a square is set
    pub fn is_set(&self, square: u8) -> bool {
        if square >= 64 {
            false
        } else {
            (self.0 & (1u64 << square)) != 0
        }
    }

    // Set a bit at the given square
    pub fn set_bit(&mut self, square: u8) {
        if square < 64 {
            self.0 |= 1u64 << square;
        }
    }

    // Clear a bit at the given square
    pub fn clear_bit(&mut self, square: u8) {
        if square < 64 {
            self.0 &= !(1u64 << square);
        }
    }

    // Count the number of set bits (population count)
    pub fn pop_count(&self) -> u32 {
        self.0.count_ones()
    }

    // Get the least significant bit
    pub fn lsb(&self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0.trailing_zeros() as u8)
        }
    }

    // Get the most significant bit
    pub fn msb(&self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            Some(63 - self.0.leading_zeros() as u8)
        }
    }
}

impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            for file in 0..8 {
                let square = rank * 8 + file;
                let bit = if self.is_set(square) { '1' } else { '.' };
                write!(f, "{} ", bit)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

// Bitwise operations
impl std::ops::BitOr for Bitboard {
    type Output = Bitboard;

    fn bitor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for Bitboard {
    type Output = Bitboard;

    fn bitand(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 & rhs.0)
    }
}

impl std::ops::BitXor for Bitboard {
    type Output = Bitboard;

    fn bitxor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl std::ops::Not for Bitboard {
    type Output = Bitboard;

    fn not(self) -> Bitboard {
        Bitboard(!self.0)
    }
} 