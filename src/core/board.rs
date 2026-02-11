//! Chess board representation
//!
//! This module provides the core board representation using bitboards.

use super::bitboard::Bitboard;
use super::moves::Move;
use super::zobrist::ZobristKeys;
use std::fmt;

/// Square representation (0-63, a1=0, h8=63)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Square(pub u8);

impl Square {
    // Rank 1
    pub const A1: Square = Square(0);
    pub const B1: Square = Square(1);
    pub const C1: Square = Square(2);
    pub const D1: Square = Square(3);
    pub const E1: Square = Square(4);
    pub const F1: Square = Square(5);
    pub const G1: Square = Square(6);
    pub const H1: Square = Square(7);
    // Rank 2
    pub const A2: Square = Square(8);
    pub const B2: Square = Square(9);
    pub const C2: Square = Square(10);
    pub const D2: Square = Square(11);
    pub const E2: Square = Square(12);
    pub const F2: Square = Square(13);
    pub const G2: Square = Square(14);
    pub const H2: Square = Square(15);
    // Rank 3
    pub const A3: Square = Square(16);
    pub const B3: Square = Square(17);
    pub const C3: Square = Square(18);
    pub const D3: Square = Square(19);
    pub const E3: Square = Square(20);
    pub const F3: Square = Square(21);
    pub const G3: Square = Square(22);
    pub const H3: Square = Square(23);
    // Rank 4
    pub const A4: Square = Square(24);
    pub const B4: Square = Square(25);
    pub const C4: Square = Square(26);
    pub const D4: Square = Square(27);
    pub const E4: Square = Square(28);
    pub const F4: Square = Square(29);
    pub const G4: Square = Square(30);
    pub const H4: Square = Square(31);
    // Rank 5
    pub const A5: Square = Square(32);
    pub const B5: Square = Square(33);
    pub const C5: Square = Square(34);
    pub const D5: Square = Square(35);
    pub const E5: Square = Square(36);
    pub const F5: Square = Square(37);
    pub const G5: Square = Square(38);
    pub const H5: Square = Square(39);
    // Rank 6
    pub const A6: Square = Square(40);
    pub const B6: Square = Square(41);
    pub const C6: Square = Square(42);
    pub const D6: Square = Square(43);
    pub const E6: Square = Square(44);
    pub const F6: Square = Square(45);
    pub const G6: Square = Square(46);
    pub const H6: Square = Square(47);
    // Rank 7
    pub const A7: Square = Square(48);
    pub const B7: Square = Square(49);
    pub const C7: Square = Square(50);
    pub const D7: Square = Square(51);
    pub const E7: Square = Square(52);
    pub const F7: Square = Square(53);
    pub const G7: Square = Square(54);
    pub const H7: Square = Square(55);
    // Rank 8
    pub const A8: Square = Square(56);
    pub const B8: Square = Square(57);
    pub const C8: Square = Square(58);
    pub const D8: Square = Square(59);
    pub const E8: Square = Square(60);
    pub const F8: Square = Square(61);
    pub const G8: Square = Square(62);
    pub const H8: Square = Square(63);

    #[inline]
    pub const fn new(sq: u8) -> Self {
        Square(sq)
    }

    #[inline]
    pub const fn from_file_rank(file: u8, rank: u8) -> Self {
        Square(rank * 8 + file)
    }

    #[inline]
    pub const fn file(self) -> u8 {
        self.0 & 7
    }

    #[inline]
    pub const fn rank(self) -> u8 {
        self.0 >> 3
    }

    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub const fn bitboard(self) -> Bitboard {
        Bitboard::from_square(self.0)
    }

    /// Parse a square from algebraic notation (e.g., "e4")
    pub fn from_algebraic(s: &str) -> Option<Self> {
        if s.len() != 2 {
            return None;
        }
        let bytes = s.as_bytes();
        let file = bytes[0].wrapping_sub(b'a');
        let rank = bytes[1].wrapping_sub(b'1');
        if file < 8 && rank < 8 {
            Some(Square::from_file_rank(file, rank))
        } else {
            None
        }
    }

    /// Convert to algebraic notation
    pub fn to_algebraic(self) -> String {
        let file = (b'a' + self.file()) as char;
        let rank = (b'1' + self.rank()) as char;
        format!("{}{}", file, rank)
    }

    /// Mirror the square vertically (for black's perspective)
    #[inline]
    pub const fn flip_vertical(self) -> Self {
        Square(self.0 ^ 56)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_algebraic())
    }
}

/// Piece color
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    #[inline]
    pub const fn opposite(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Pawn direction (+8 for white, -8 for black)
    #[inline]
    pub const fn pawn_direction(self) -> i8 {
        match self {
            Color::White => 8,
            Color::Black => -8,
        }
    }

    /// Starting rank for pawns (rank 1 for white pawns = rank index 1)
    #[inline]
    pub const fn pawn_start_rank(self) -> u8 {
        match self {
            Color::White => 1,
            Color::Black => 6,
        }
    }

    /// Promotion rank for pawns
    #[inline]
    pub const fn promotion_rank(self) -> u8 {
        match self {
            Color::White => 7,
            Color::Black => 0,
        }
    }

    /// En passant rank
    #[inline]
    pub const fn en_passant_rank(self) -> u8 {
        match self {
            Color::White => 4,
            Color::Black => 3,
        }
    }
}

impl std::ops::Not for Color {
    type Output = Color;
    fn not(self) -> Self::Output {
        self.opposite()
    }
}

/// Piece type
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
pub enum PieceType {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceType {
    pub const ALL: [PieceType; 6] = [
        PieceType::Pawn,
        PieceType::Knight,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Queen,
        PieceType::King,
    ];

    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// Get the character representation of the piece type
    pub const fn to_char(self) -> char {
        match self {
            PieceType::Pawn => 'p',
            PieceType::Knight => 'n',
            PieceType::Bishop => 'b',
            PieceType::Rook => 'r',
            PieceType::Queen => 'q',
            PieceType::King => 'k',
        }
    }

    /// Parse piece type from character
    pub fn from_char(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            'p' => Some(PieceType::Pawn),
            'n' => Some(PieceType::Knight),
            'b' => Some(PieceType::Bishop),
            'r' => Some(PieceType::Rook),
            'q' => Some(PieceType::Queen),
            'k' => Some(PieceType::King),
            _ => None,
        }
    }
}

/// A colored piece
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: Color,
}

impl Piece {
    #[inline]
    pub const fn new(piece_type: PieceType, color: Color) -> Self {
        Piece { piece_type, color }
    }

    /// Get the character representation (uppercase for white, lowercase for black)
    pub fn to_char(self) -> char {
        let c = self.piece_type.to_char();
        match self.color {
            Color::White => c.to_ascii_uppercase(),
            Color::Black => c,
        }
    }

    /// Parse a piece from character
    pub fn from_char(c: char) -> Option<Self> {
        let piece_type = PieceType::from_char(c)?;
        let color = if c.is_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        Some(Piece::new(piece_type, color))
    }

    /// Get piece index for NNUE (0-11)
    #[inline]
    pub fn nnue_index(self) -> usize {
        self.color.index() * 6 + self.piece_type.index()
    }
}

/// Castling rights
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CastlingRights(u8);

impl CastlingRights {
    pub const NONE: CastlingRights = CastlingRights(0);
    pub const WHITE_KINGSIDE: u8 = 1;
    pub const WHITE_QUEENSIDE: u8 = 2;
    pub const BLACK_KINGSIDE: u8 = 4;
    pub const BLACK_QUEENSIDE: u8 = 8;
    pub const ALL: CastlingRights = CastlingRights(0xF);

    #[inline]
    pub const fn new(rights: u8) -> Self {
        CastlingRights(rights)
    }

    #[inline]
    pub const fn has(self, right: u8) -> bool {
        (self.0 & right) != 0
    }

    #[inline]
    pub fn remove(&mut self, right: u8) {
        self.0 &= !right;
    }

    #[inline]
    pub const fn raw(self) -> u8 {
        self.0
    }

    pub fn can_castle_kingside(self, color: Color) -> bool {
        match color {
            Color::White => self.has(Self::WHITE_KINGSIDE),
            Color::Black => self.has(Self::BLACK_KINGSIDE),
        }
    }

    pub fn can_castle_queenside(self, color: Color) -> bool {
        match color {
            Color::White => self.has(Self::WHITE_QUEENSIDE),
            Color::Black => self.has(Self::BLACK_QUEENSIDE),
        }
    }

    /// Get the castling rights that should be removed when a piece moves from/to a square
    pub fn rights_for_square(sq: Square) -> u8 {
        match sq.0 {
            0 => Self::WHITE_QUEENSIDE,  // a1
            7 => Self::WHITE_KINGSIDE,   // h1
            4 => Self::WHITE_KINGSIDE | Self::WHITE_QUEENSIDE, // e1
            56 => Self::BLACK_QUEENSIDE, // a8
            63 => Self::BLACK_KINGSIDE,  // h8
            60 => Self::BLACK_KINGSIDE | Self::BLACK_QUEENSIDE, // e8
            _ => 0,
        }
    }
}

/// The chess board state
#[derive(Clone)]
pub struct Board {
    /// Bitboards for each piece type
    pub pieces: [Bitboard; 6],
    /// Bitboards for each color
    pub colors: [Bitboard; 2],
    /// Piece lookup table (None if empty)
    pub piece_at: [Option<Piece>; 64],
    /// Side to move
    pub side_to_move: Color,
    /// Castling rights
    pub castling: CastlingRights,
    /// En passant target square (if any)
    pub en_passant: Option<Square>,
    /// Halfmove clock (for 50-move rule)
    pub halfmove_clock: u8,
    /// Fullmove number
    pub fullmove_number: u16,
    /// Zobrist hash of the position
    pub hash: u64,
    /// Checkers (pieces giving check)
    pub checkers: Bitboard,
}

impl Board {
    /// Create a new empty board
    pub fn empty() -> Self {
        Board {
            pieces: [Bitboard::EMPTY; 6],
            colors: [Bitboard::EMPTY; 2],
            piece_at: [None; 64],
            side_to_move: Color::White,
            castling: CastlingRights::NONE,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            hash: 0,
            checkers: Bitboard::EMPTY,
        }
    }

    /// Create a board from the starting position
    pub fn startpos() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    /// Parse a board from FEN notation
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let mut board = Board::empty();
        let parts: Vec<&str> = fen.split_whitespace().collect();

        if parts.len() < 4 {
            return Err("Invalid FEN: too few parts".to_string());
        }

        // Parse piece placement
        let mut sq: i8 = 56; // Start at a8
        for c in parts[0].chars() {
            match c {
                '/' => {
                    sq -= 16; // Move to next rank down
                }
                '1'..='8' => {
                    sq += (c as i8) - ('0' as i8);
                }
                _ => {
                    if let Some(piece) = Piece::from_char(c) {
                        board.put_piece(piece, Square(sq as u8));
                        sq += 1;
                    } else {
                        return Err(format!("Invalid FEN: unknown piece '{}'", c));
                    }
                }
            }
        }

        // Parse side to move
        board.side_to_move = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err("Invalid FEN: invalid side to move".to_string()),
        };

        // Parse castling rights
        let mut castling = 0u8;
        for c in parts[2].chars() {
            match c {
                'K' => castling |= CastlingRights::WHITE_KINGSIDE,
                'Q' => castling |= CastlingRights::WHITE_QUEENSIDE,
                'k' => castling |= CastlingRights::BLACK_KINGSIDE,
                'q' => castling |= CastlingRights::BLACK_QUEENSIDE,
                '-' => {}
                _ => return Err(format!("Invalid FEN: unknown castling right '{}'", c)),
            }
        }
        board.castling = CastlingRights::new(castling);

        // Parse en passant
        board.en_passant = if parts[3] == "-" {
            None
        } else {
            Some(Square::from_algebraic(parts[3])
                .ok_or_else(|| format!("Invalid FEN: invalid en passant square '{}'", parts[3]))?)
        };

        // Parse halfmove clock
        if parts.len() > 4 {
            board.halfmove_clock = parts[4].parse().unwrap_or(0);
        }

        // Parse fullmove number
        if parts.len() > 5 {
            board.fullmove_number = parts[5].parse().unwrap_or(1);
        }

        // Calculate hash
        board.hash = board.calculate_hash();

        // Calculate checkers
        board.update_checkers();

        Ok(board)
    }

    /// Convert the board to FEN notation
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        // Piece placement
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                let sq = Square::from_file_rank(file, rank);
                if let Some(piece) = self.piece_at[sq.index()] {
                    if empty > 0 {
                        fen.push_str(&empty.to_string());
                        empty = 0;
                    }
                    fen.push(piece.to_char());
                } else {
                    empty += 1;
                }
            }
            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }

        // Side to move
        fen.push(' ');
        fen.push(match self.side_to_move {
            Color::White => 'w',
            Color::Black => 'b',
        });

        // Castling rights
        fen.push(' ');
        if self.castling.0 == 0 {
            fen.push('-');
        } else {
            if self.castling.has(CastlingRights::WHITE_KINGSIDE) {
                fen.push('K');
            }
            if self.castling.has(CastlingRights::WHITE_QUEENSIDE) {
                fen.push('Q');
            }
            if self.castling.has(CastlingRights::BLACK_KINGSIDE) {
                fen.push('k');
            }
            if self.castling.has(CastlingRights::BLACK_QUEENSIDE) {
                fen.push('q');
            }
        }

        // En passant
        fen.push(' ');
        match self.en_passant {
            Some(sq) => fen.push_str(&sq.to_algebraic()),
            None => fen.push('-'),
        }

        // Halfmove clock and fullmove number
        fen.push(' ');
        fen.push_str(&self.halfmove_clock.to_string());
        fen.push(' ');
        fen.push_str(&self.fullmove_number.to_string());

        fen
    }

    /// Put a piece on the board
    pub fn put_piece(&mut self, piece: Piece, sq: Square) {
        let bb = sq.bitboard();
        self.pieces[piece.piece_type.index()] |= bb;
        self.colors[piece.color.index()] |= bb;
        self.piece_at[sq.index()] = Some(piece);
    }

    /// Remove a piece from the board
    pub fn remove_piece(&mut self, sq: Square) -> Option<Piece> {
        let piece = self.piece_at[sq.index()]?;
        let bb = sq.bitboard();
        self.pieces[piece.piece_type.index()] &= !bb;
        self.colors[piece.color.index()] &= !bb;
        self.piece_at[sq.index()] = None;
        Some(piece)
    }

    /// Move a piece from one square to another
    pub fn move_piece(&mut self, from: Square, to: Square) {
        if let Some(piece) = self.remove_piece(from) {
            self.put_piece(piece, to);
        }
    }

    /// Get all pieces of a specific type
    #[inline]
    pub fn pieces(&self, piece_type: PieceType) -> Bitboard {
        self.pieces[piece_type.index()]
    }

    /// Get all pieces of a specific color
    #[inline]
    pub fn color(&self, color: Color) -> Bitboard {
        self.colors[color.index()]
    }

    /// Get pieces of a specific type and color
    #[inline]
    pub fn pieces_of(&self, piece_type: PieceType, color: Color) -> Bitboard {
        self.pieces[piece_type.index()] & self.colors[color.index()]
    }

    /// Get all occupied squares
    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.colors[0] | self.colors[1]
    }

    /// Get the king square for a color
    #[inline]
    pub fn king_square(&self, color: Color) -> Square {
        Square((self.pieces_of(PieceType::King, color)).lsb())
    }

    /// Check if the side to move is in check
    #[inline]
    pub fn is_check(&self) -> bool {
        self.checkers.is_not_empty()
    }

    /// Calculate the Zobrist hash from scratch
    pub fn calculate_hash(&self) -> u64 {
        let keys = ZobristKeys::instance();
        let mut hash = 0u64;

        // Hash pieces
        for sq in 0..64 {
            if let Some(piece) = self.piece_at[sq] {
                hash ^= keys.piece(piece, Square(sq as u8));
            }
        }

        // Hash side to move
        if self.side_to_move == Color::Black {
            hash ^= keys.side_to_move();
        }

        // Hash castling rights
        hash ^= keys.castling(self.castling);

        // Hash en passant
        if let Some(ep) = self.en_passant {
            hash ^= keys.en_passant(ep);
        }

        hash
    }

    /// Update the checkers bitboard
    pub fn update_checkers(&mut self) {
        self.checkers = self.attackers_to(self.king_square(self.side_to_move), self.side_to_move.opposite());
    }

    /// Get attackers of a specific color to a square
    pub fn attackers_to(&self, sq: Square, attacker_color: Color) -> Bitboard {
        use crate::engine::movegen::MoveGen;
        
        let occupied = self.occupied();
        let attackers = self.color(attacker_color);
        
        let pawn_attacks = MoveGen::pawn_attacks(sq, attacker_color.opposite());
        let knight_attacks = MoveGen::knight_attacks(sq);
        let bishop_attacks = MoveGen::bishop_attacks(sq, occupied);
        let rook_attacks = MoveGen::rook_attacks(sq, occupied);
        let queen_attacks = bishop_attacks | rook_attacks;
        let king_attacks = MoveGen::king_attacks(sq);

        (pawn_attacks & self.pieces(PieceType::Pawn) & attackers)
            | (knight_attacks & self.pieces(PieceType::Knight) & attackers)
            | (bishop_attacks & self.pieces(PieceType::Bishop) & attackers)
            | (rook_attacks & self.pieces(PieceType::Rook) & attackers)
            | (queen_attacks & self.pieces(PieceType::Queen) & attackers)
            | (king_attacks & self.pieces(PieceType::King) & attackers)
    }

    /// Check if a square is attacked by a specific color
    pub fn is_attacked(&self, sq: Square, by_color: Color) -> bool {
        self.attackers_to(sq, by_color).is_not_empty()
    }

    /// Make a move on the board (returns true if legal)
    pub fn make_move(&mut self, mv: Move) -> bool {
        let keys = ZobristKeys::instance();
        let from = mv.from();
        let to = mv.to();
        let piece = match self.piece_at[from.index()] {
            Some(p) => p,
            None => return false,
        };
        let us = self.side_to_move;
        let them = us.opposite();

        // Save en passant state before clearing
        if let Some(ep) = self.en_passant {
            self.hash ^= keys.en_passant(ep);
        }
        self.en_passant = None;

        // Handle castling rights changes
        let old_castling = self.castling;
        self.castling.remove(CastlingRights::rights_for_square(from));
        self.castling.remove(CastlingRights::rights_for_square(to));
        self.hash ^= keys.castling(old_castling);
        self.hash ^= keys.castling(self.castling);

        // Remove piece from source
        self.hash ^= keys.piece(piece, from);
        self.remove_piece(from);

        // Handle captures
        if mv.is_capture() {
            let capture_sq = if mv.is_en_passant() {
                Square::new((to.0 as i8 - us.pawn_direction()) as u8)
            } else {
                to
            };
            if let Some(captured) = self.piece_at[capture_sq.index()] {
                self.hash ^= keys.piece(captured, capture_sq);
                self.remove_piece(capture_sq);
            }
            self.halfmove_clock = 0;
        } else if piece.piece_type == PieceType::Pawn {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        // Handle castling
        if mv.is_castling() {
            let (rook_from, rook_to) = if to.file() > from.file() {
                // Kingside
                (Square::new(from.0 + 3), Square::new(from.0 + 1))
            } else {
                // Queenside
                (Square::new(from.0 - 4), Square::new(from.0 - 1))
            };
            if let Some(rook) = self.piece_at[rook_from.index()] {
                self.hash ^= keys.piece(rook, rook_from);
                self.remove_piece(rook_from);
                self.hash ^= keys.piece(rook, rook_to);
                self.put_piece(rook, rook_to);
            }
        }

        // Place piece at destination (handle promotion)
        let placed_piece = if mv.is_promotion() {
            Piece::new(mv.promotion_piece().unwrap(), us)
        } else {
            piece
        };
        self.hash ^= keys.piece(placed_piece, to);
        self.put_piece(placed_piece, to);

        // Handle double pawn push (set en passant)
        if piece.piece_type == PieceType::Pawn {
            let diff = (to.0 as i8 - from.0 as i8).abs();
            if diff == 16 {
                let ep_sq = Square::new((from.0 as i8 + us.pawn_direction()) as u8);
                self.en_passant = Some(ep_sq);
                self.hash ^= keys.en_passant(ep_sq);
            }
        }

        // Switch side to move
        self.side_to_move = them;
        self.hash ^= keys.side_to_move();

        // Update fullmove number
        if us == Color::Black {
            self.fullmove_number += 1;
        }

        // Update checkers
        self.update_checkers();

        // Check if the move was legal (king not in check)
        if self.is_attacked(self.king_square(us), them) {
            return false;
        }

        true
    }

    /// Check if the current position is a draw by insufficient material
    pub fn is_insufficient_material(&self) -> bool {
        let occupied = self.occupied();
        let count = occupied.count();

        // King vs King
        if count == 2 {
            return true;
        }

        // King + minor piece vs King
        if count == 3 {
            let knights = self.pieces(PieceType::Knight);
            let bishops = self.pieces(PieceType::Bishop);
            if knights.count() == 1 || bishops.count() == 1 {
                return true;
            }
        }

        // King + Bishop vs King + Bishop (same color bishops)
        if count == 4 {
            let bishops = self.pieces(PieceType::Bishop);
            if bishops.count() == 2 {
                let on_light = (bishops & Bitboard::LIGHT_SQUARES).count();
                let on_dark = (bishops & Bitboard::DARK_SQUARES).count();
                if on_light == 2 || on_dark == 2 {
                    return true;
                }
            }
        }

        false
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        for rank in (0..8).rev() {
            write!(f, "  {} ", rank + 1)?;
            for file in 0..8 {
                let sq = Square::from_file_rank(file, rank);
                match self.piece_at[sq.index()] {
                    Some(piece) => write!(f, "{} ", piece.to_char())?,
                    None => write!(f, ". ")?,
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "    a b c d e f g h")?;
        writeln!(f)?;
        writeln!(f, "  FEN: {}", self.to_fen())?;
        Ok(())
    }
}

impl fmt::Debug for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

