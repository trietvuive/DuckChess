//! Move generation with magic bitboards
//!
//! This module implements efficient move generation using:
//! - Precomputed attack tables for knights and kings
//! - Magic bitboards for sliding pieces (bishops, rooks, queens)
//! - Pawn move generation with promotions and en passant

use crate::core::bitboard::Bitboard;
use crate::core::board::{Board, Color, PieceType, Square};
use crate::core::moves::{Move, MoveList};

/// Magic bitboard entry for sliding piece move generation
#[derive(Clone, Copy)]
struct Magic {
    mask: u64,
    magic: u64,
    attacks: &'static [Bitboard],
    shift: u32,
}

impl Magic {
    const fn empty() -> Self {
        Magic {
            mask: 0,
            magic: 0,
            attacks: &[],
            shift: 0,
        }
    }
}

/// Precomputed attack tables
pub struct MoveGen {
    // Knight attacks for each square
    knight_attacks: [Bitboard; 64],
    // King attacks for each square
    king_attacks: [Bitboard; 64],
    // Pawn attacks for each color and square [color][square]
    pawn_attacks: [[Bitboard; 64]; 2],
    // Rook magic bitboards
    rook_magics: [Magic; 64],
    // Bishop magic bitboards
    bishop_magics: [Magic; 64],
    // Storage for rook attacks
    rook_attacks: Box<[Bitboard]>,
    // Storage for bishop attacks
    bishop_attacks: Box<[Bitboard]>,
    // Lines between squares (for pin detection)
    between: [[Bitboard; 64]; 64],
    // Lines through squares (for ray attacks)
    line: [[Bitboard; 64]; 64],
}

// Global move generator instance
static MOVEGEN: std::sync::OnceLock<MoveGen> = std::sync::OnceLock::new();

impl MoveGen {
    /// Get the global MoveGen instance
    pub fn instance() -> &'static MoveGen {
        MOVEGEN.get_or_init(MoveGen::init)
    }

    /// Initialize all attack tables
    fn init() -> Self {
        let mut mg = MoveGen {
            knight_attacks: [Bitboard::EMPTY; 64],
            king_attacks: [Bitboard::EMPTY; 64],
            pawn_attacks: [[Bitboard::EMPTY; 64]; 2],
            rook_magics: [Magic::empty(); 64],
            bishop_magics: [Magic::empty(); 64],
            rook_attacks: vec![Bitboard::EMPTY; 102400].into_boxed_slice(),
            bishop_attacks: vec![Bitboard::EMPTY; 5248].into_boxed_slice(),
            between: [[Bitboard::EMPTY; 64]; 64],
            line: [[Bitboard::EMPTY; 64]; 64],
        };

        mg.init_knight_attacks();
        mg.init_king_attacks();
        mg.init_pawn_attacks();
        mg.init_magics();
        mg.init_between_and_line();
        mg
    }

    fn init_knight_attacks(&mut self) {
        for sq in 0..64 {
            let bb = Bitboard::from_square(sq);
            let mut attacks = Bitboard::EMPTY;

            // Knight moves: +/- 2 files and +/- 1 rank, or +/- 1 file and +/- 2 ranks
            let not_a = !Bitboard::FILE_A;
            let not_ab = !(Bitboard::FILE_A | Bitboard::FILE_B);
            let not_h = !Bitboard::FILE_H;
            let not_gh = !(Bitboard::FILE_G | Bitboard::FILE_H);

            attacks |= (bb << 17) & not_a;  // Up 2, right 1
            attacks |= (bb << 15) & not_h;  // Up 2, left 1
            attacks |= (bb << 10) & not_ab; // Up 1, right 2
            attacks |= (bb << 6) & not_gh;  // Up 1, left 2
            attacks |= (bb >> 17) & not_h;  // Down 2, left 1
            attacks |= (bb >> 15) & not_a;  // Down 2, right 1
            attacks |= (bb >> 10) & not_gh; // Down 1, left 2
            attacks |= (bb >> 6) & not_ab;  // Down 1, right 2

            self.knight_attacks[sq as usize] = attacks;
        }
    }

    fn init_king_attacks(&mut self) {
        for sq in 0..64 {
            let bb = Bitboard::from_square(sq);
            let mut attacks = Bitboard::EMPTY;

            attacks |= bb.north();
            attacks |= bb.south();
            attacks |= bb.east();
            attacks |= bb.west();
            attacks |= bb.north_east();
            attacks |= bb.north_west();
            attacks |= bb.south_east();
            attacks |= bb.south_west();

            self.king_attacks[sq as usize] = attacks;
        }
    }

    fn init_pawn_attacks(&mut self) {
        for sq in 0..64 {
            let bb = Bitboard::from_square(sq);

            // White pawn attacks (up the board)
            self.pawn_attacks[Color::White as usize][sq as usize] =
                bb.north_east() | bb.north_west();

            // Black pawn attacks (down the board)
            self.pawn_attacks[Color::Black as usize][sq as usize] =
                bb.south_east() | bb.south_west();
        }
    }

    fn init_magics(&mut self) {
        // Pre-computed magic numbers for rooks and bishops
        // These are well-known magic numbers that work efficiently
        const ROOK_MAGICS: [u64; 64] = [
            0x0080001020400080, 0x0040001000200040, 0x0080081000200080, 0x0080040800100080,
            0x0080020400080080, 0x0080010200040080, 0x0080008001000200, 0x0080002040800100,
            0x0000800020400080, 0x0000400020005000, 0x0000801000200080, 0x0000800800100080,
            0x0000800400080080, 0x0000800200040080, 0x0000800100020080, 0x0000800040800100,
            0x0000208000400080, 0x0000404000201000, 0x0000808010002000, 0x0000808008001000,
            0x0000808004000800, 0x0000808002000400, 0x0000010100020004, 0x0000020000408104,
            0x0000208080004000, 0x0000200040005000, 0x0000100080200080, 0x0000080080100080,
            0x0000040080080080, 0x0000020080040080, 0x0000010080800200, 0x0000800080004100,
            0x0000204000800080, 0x0000200040401000, 0x0000100080802000, 0x0000080080801000,
            0x0000040080800800, 0x0000020080800400, 0x0000020001010004, 0x0000800040800100,
            0x0000204000808000, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
            0x0000040008008080, 0x0000020004008080, 0x0000010002008080, 0x0000004081020004,
            0x0000204000800080, 0x0000200040008080, 0x0000100020008080, 0x0000080010008080,
            0x0000040008008080, 0x0000020004008080, 0x0000800100020080, 0x0000800041000080,
            0x00FFFCDDFCED714A, 0x007FFCDDFCED714A, 0x003FFFCDFFD88096, 0x0000040810002101,
            0x0001000204080011, 0x0001000204000801, 0x0001000082000401, 0x0001FFFAABFAD1A2,
        ];

        const BISHOP_MAGICS: [u64; 64] = [
            0x0002020202020200, 0x0002020202020000, 0x0004010202000000, 0x0004040080000000,
            0x0001104000000000, 0x0000821040000000, 0x0000410410400000, 0x0000104104104000,
            0x0000040404040400, 0x0000020202020200, 0x0000040102020000, 0x0000040400800000,
            0x0000011040000000, 0x0000008210400000, 0x0000004104104000, 0x0000002082082000,
            0x0004000808080800, 0x0002000404040400, 0x0001000202020200, 0x0000800802004000,
            0x0000800400A00000, 0x0000200100884000, 0x0000400082082000, 0x0000200041041000,
            0x0002080010101000, 0x0001040008080800, 0x0000208004010400, 0x0000404004010200,
            0x0000840000802000, 0x0000404002011000, 0x0000808001041000, 0x0000404000820800,
            0x0001041000202000, 0x0000820800101000, 0x0000104400080800, 0x0000020080080080,
            0x0000404040040100, 0x0000808100020100, 0x0001010100020800, 0x0000808080010400,
            0x0000820820004000, 0x0000410410002000, 0x0000082088001000, 0x0000002011000800,
            0x0000080100400400, 0x0001010101000200, 0x0002020202000400, 0x0001010101000200,
            0x0000410410400000, 0x0000208208200000, 0x0000002084100000, 0x0000000020880000,
            0x0000001002020000, 0x0000040408020000, 0x0004040404040000, 0x0002020202020000,
            0x0000104104104000, 0x0000002082082000, 0x0000000020841000, 0x0000000000208800,
            0x0000000010020200, 0x0000000404080200, 0x0000040404040400, 0x0002020202020200,
        ];

        let mut rook_offset = 0;
        let mut bishop_offset = 0;

        for sq in 0..64 {
            // Initialize rook magics
            let rook_mask = self.rook_mask(sq);
            let rook_bits = rook_mask.count_ones();
            let rook_size = 1 << rook_bits;

            self.rook_magics[sq as usize] = Magic {
                mask: rook_mask,
                magic: ROOK_MAGICS[sq as usize],
                attacks: unsafe {
                    std::slice::from_raw_parts(
                        self.rook_attacks.as_ptr().add(rook_offset),
                        rook_size,
                    )
                },
                shift: 64 - rook_bits,
            };

            // Fill rook attack table
            for i in 0..rook_size {
                let occ = self.index_to_occupancy(i, rook_mask);
                let idx = ((occ.wrapping_mul(ROOK_MAGICS[sq as usize])) >> (64 - rook_bits)) as usize;
                self.rook_attacks[rook_offset + idx] = self.sliding_attacks(sq, occ, true);
            }
            rook_offset += rook_size;

            // Initialize bishop magics
            let bishop_mask = self.bishop_mask(sq);
            let bishop_bits = bishop_mask.count_ones();
            let bishop_size = 1 << bishop_bits;

            self.bishop_magics[sq as usize] = Magic {
                mask: bishop_mask,
                magic: BISHOP_MAGICS[sq as usize],
                attacks: unsafe {
                    std::slice::from_raw_parts(
                        self.bishop_attacks.as_ptr().add(bishop_offset),
                        bishop_size,
                    )
                },
                shift: 64 - bishop_bits,
            };

            // Fill bishop attack table
            for i in 0..bishop_size {
                let occ = self.index_to_occupancy(i, bishop_mask);
                let idx = ((occ.wrapping_mul(BISHOP_MAGICS[sq as usize])) >> (64 - bishop_bits)) as usize;
                self.bishop_attacks[bishop_offset + idx] = self.sliding_attacks(sq, occ, false);
            }
            bishop_offset += bishop_size;
        }
    }

    fn rook_mask(&self, sq: u8) -> u64 {
        let rank = sq / 8;
        let file = sq % 8;
        let mut mask = 0u64;

        // Vertical rays (excluding edges)
        for r in (rank + 1)..7 {
            mask |= 1u64 << (r * 8 + file);
        }
        for r in 1..rank {
            mask |= 1u64 << (r * 8 + file);
        }

        // Horizontal rays (excluding edges)
        for f in (file + 1)..7 {
            mask |= 1u64 << (rank * 8 + f);
        }
        for f in 1..file {
            mask |= 1u64 << (rank * 8 + f);
        }

        mask
    }

    fn bishop_mask(&self, sq: u8) -> u64 {
        let rank = sq / 8;
        let file = sq % 8;
        let mut mask = 0u64;

        // Diagonal rays (excluding edges)
        let mut r = rank + 1;
        let mut f = file + 1;
        while r < 7 && f < 7 {
            mask |= 1u64 << (r * 8 + f);
            r += 1;
            f += 1;
        }

        r = rank.wrapping_sub(1);
        f = file + 1;
        while r > 0 && r < 8 && f < 7 {
            mask |= 1u64 << (r * 8 + f);
            r = r.wrapping_sub(1);
            f += 1;
        }

        r = rank + 1;
        f = file.wrapping_sub(1);
        while r < 7 && f > 0 && f < 8 {
            mask |= 1u64 << (r * 8 + f);
            r += 1;
            f = f.wrapping_sub(1);
        }

        r = rank.wrapping_sub(1);
        f = file.wrapping_sub(1);
        while r > 0 && r < 8 && f > 0 && f < 8 {
            mask |= 1u64 << (r * 8 + f);
            r = r.wrapping_sub(1);
            f = f.wrapping_sub(1);
        }

        mask
    }

    fn index_to_occupancy(&self, index: usize, mask: u64) -> u64 {
        let mut occ = 0u64;
        let mut m = mask;
        let mut i = 0;

        while m != 0 {
            let sq = m.trailing_zeros();
            m &= m - 1;
            if (index >> i) & 1 != 0 {
                occ |= 1u64 << sq;
            }
            i += 1;
        }

        occ
    }

    fn sliding_attacks(&self, sq: u8, occ: u64, is_rook: bool) -> Bitboard {
        let rank = sq / 8;
        let file = sq % 8;
        let mut attacks = 0u64;

        let directions: [(i8, i8); 4] = if is_rook {
            [(0, 1), (0, -1), (1, 0), (-1, 0)]
        } else {
            [(1, 1), (1, -1), (-1, 1), (-1, -1)]
        };

        for (dr, df) in directions {
            let mut r = rank as i8 + dr;
            let mut f = file as i8 + df;

            while r >= 0 && r < 8 && f >= 0 && f < 8 {
                let target = (r * 8 + f) as u8;
                attacks |= 1u64 << target;
                if (occ >> target) & 1 != 0 {
                    break;
                }
                r += dr;
                f += df;
            }
        }

        Bitboard(attacks)
    }

    fn init_between_and_line(&mut self) {
        for sq1 in 0..64u8 {
            for sq2 in 0..64u8 {
                let s1 = Square(sq1);
                let s2 = Square(sq2);

                if sq1 == sq2 {
                    continue;
                }

                let r1 = s1.rank() as i8;
                let f1 = s1.file() as i8;
                let r2 = s2.rank() as i8;
                let f2 = s2.file() as i8;

                let dr = (r2 - r1).signum();
                let df = (f2 - f1).signum();

                // Check if squares are on same rank, file, or diagonal
                let on_line = (r1 == r2) || (f1 == f2) || ((r2 - r1).abs() == (f2 - f1).abs());

                if on_line {
                    let mut between = Bitboard::EMPTY;
                    let mut line = Bitboard::EMPTY;

                    // Calculate between (exclusive of endpoints)
                    let mut r = r1 + dr;
                    let mut f = f1 + df;
                    while r != r2 || f != f2 {
                        between.set((r * 8 + f) as u8);
                        r += dr;
                        f += df;
                    }

                    // Calculate line (through both squares in both directions)
                    let mut r = r1;
                    let mut f = f1;
                    while r >= 0 && r < 8 && f >= 0 && f < 8 {
                        line.set((r * 8 + f) as u8);
                        r -= dr;
                        f -= df;
                    }
                    r = r1 + dr;
                    f = f1 + df;
                    while r >= 0 && r < 8 && f >= 0 && f < 8 {
                        line.set((r * 8 + f) as u8);
                        r += dr;
                        f += df;
                    }

                    self.between[sq1 as usize][sq2 as usize] = between;
                    self.line[sq1 as usize][sq2 as usize] = line;
                }
            }
        }
    }

    /// Get knight attacks for a square
    #[inline]
    pub fn knight_attacks(sq: Square) -> Bitboard {
        Self::instance().knight_attacks[sq.index()]
    }

    /// Get king attacks for a square
    #[inline]
    pub fn king_attacks(sq: Square) -> Bitboard {
        Self::instance().king_attacks[sq.index()]
    }

    /// Get pawn attacks for a square and color
    #[inline]
    pub fn pawn_attacks(sq: Square, color: Color) -> Bitboard {
        Self::instance().pawn_attacks[color as usize][sq.index()]
    }

    /// Get rook attacks for a square with given occupancy
    #[inline]
    pub fn rook_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
        let mg = Self::instance();
        let magic = &mg.rook_magics[sq.index()];
        let idx = (((occupied.0 & magic.mask).wrapping_mul(magic.magic)) >> magic.shift) as usize;
        magic.attacks[idx]
    }

    /// Get bishop attacks for a square with given occupancy
    #[inline]
    pub fn bishop_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
        let mg = Self::instance();
        let magic = &mg.bishop_magics[sq.index()];
        let idx = (((occupied.0 & magic.mask).wrapping_mul(magic.magic)) >> magic.shift) as usize;
        magic.attacks[idx]
    }

    /// Get queen attacks for a square with given occupancy
    #[inline]
    pub fn queen_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
        Self::rook_attacks(sq, occupied) | Self::bishop_attacks(sq, occupied)
    }

    /// Get squares between two squares (exclusive)
    #[inline]
    pub fn between(sq1: Square, sq2: Square) -> Bitboard {
        Self::instance().between[sq1.index()][sq2.index()]
    }

    /// Get all squares on the line through two squares
    #[inline]
    pub fn line(sq1: Square, sq2: Square) -> Bitboard {
        Self::instance().line[sq1.index()][sq2.index()]
    }

    /// Generate all legal moves for a position
    pub fn generate_legal_moves(board: &Board) -> MoveList {
        let mut moves = MoveList::new();
        let us = board.side_to_move;
        let them = us.opposite();
        let our_pieces = board.color(us);
        let their_pieces = board.color(them);
        let occupied = board.occupied();
        let king_sq = board.king_square(us);

        // Calculate pinned pieces and check mask
        let checkers = board.checkers;
        let num_checkers = checkers.count();

        // If double check, only king moves are legal
        if num_checkers > 1 {
            Self::generate_king_moves(board, &mut moves);
            return moves;
        }

        // Calculate pinned pieces
        let mut pinned = Bitboard::EMPTY;
        let mut pin_rays = [[Bitboard::EMPTY; 64]; 2]; // For each pinned piece, store allowed moves

        // Find potential pinners (enemy rooks, bishops, queens)
        let bishop_pinners = (board.pieces(PieceType::Bishop) | board.pieces(PieceType::Queen)) & their_pieces;
        let rook_pinners = (board.pieces(PieceType::Rook) | board.pieces(PieceType::Queen)) & their_pieces;

        for pinner_sq in (Self::bishop_attacks(king_sq, Bitboard::EMPTY) & bishop_pinners).iter() {
            let between = Self::between(king_sq, Square(pinner_sq));
            let blockers = between & occupied;
            if blockers.count() == 1 && (blockers & our_pieces).is_not_empty() {
                pinned |= blockers;
                let pinned_sq = blockers.lsb();
                pin_rays[0][pinned_sq as usize] = between | Bitboard::from_square(pinner_sq);
            }
        }

        for pinner_sq in (Self::rook_attacks(king_sq, Bitboard::EMPTY) & rook_pinners).iter() {
            let between = Self::between(king_sq, Square(pinner_sq));
            let blockers = between & occupied;
            if blockers.count() == 1 && (blockers & our_pieces).is_not_empty() {
                pinned |= blockers;
                let pinned_sq = blockers.lsb();
                pin_rays[1][pinned_sq as usize] = between | Bitboard::from_square(pinner_sq);
            }
        }

        // Calculate target squares (if in check, must block or capture checker)
        let target_mask = if num_checkers == 1 {
            let checker_sq = Square(checkers.lsb());
            Self::between(king_sq, checker_sq) | checkers
        } else {
            Bitboard::ALL
        };

        // Generate moves for each piece type
        Self::generate_pawn_moves(board, &mut moves, pinned, &pin_rays, target_mask);
        Self::generate_knight_moves(board, &mut moves, pinned, target_mask);
        Self::generate_bishop_moves(board, &mut moves, pinned, &pin_rays, target_mask);
        Self::generate_rook_moves(board, &mut moves, pinned, &pin_rays, target_mask);
        Self::generate_queen_moves(board, &mut moves, pinned, &pin_rays, target_mask);
        Self::generate_king_moves(board, &mut moves);
        Self::generate_castling_moves(board, &mut moves);

        moves
    }

    fn generate_pawn_moves(
        board: &Board,
        moves: &mut MoveList,
        pinned: Bitboard,
        pin_rays: &[[Bitboard; 64]; 2],
        target_mask: Bitboard,
    ) {
        let us = board.side_to_move;
        let them = us.opposite();
        let our_pawns = board.pieces_of(PieceType::Pawn, us);
        let their_pieces = board.color(them);
        let occupied = board.occupied();
        let empty = !occupied;

        let (push_dir, promo_rank): (fn(Bitboard) -> Bitboard, Bitboard) = match us {
            Color::White => (Bitboard::north, Bitboard::RANK_8),
            Color::Black => (Bitboard::south, Bitboard::RANK_1),
        };

        // Single pawn pushes
        let single_pushes = push_dir(our_pawns) & empty;
        let promo_pushes = single_pushes & promo_rank & target_mask;
        let normal_pushes = single_pushes & !promo_rank & target_mask;

        for to in normal_pushes.iter() {
            let from = match us {
                Color::White => to - 8,
                Color::Black => to + 8,
            };
            if pinned.contains(from) {
                let ray = pin_rays[0][from as usize] | pin_rays[1][from as usize];
                if !ray.contains(to) {
                    continue;
                }
            }
            moves.push(Move::new(Square(from), Square(to)));
        }

        for to in promo_pushes.iter() {
            let from = match us {
                Color::White => to - 8,
                Color::Black => to + 8,
            };
            if pinned.contains(from) {
                continue; // Pinned pawn can't promote by pushing
            }
            for promo in [PieceType::Queen, PieceType::Rook, PieceType::Bishop, PieceType::Knight] {
                moves.push(Move::new_promotion(Square(from), Square(to), promo, false));
            }
        }

        // Double pawn pushes
        let double_pushes = push_dir(single_pushes & (match us {
            Color::White => Bitboard::RANK_3,
            Color::Black => Bitboard::RANK_6,
        })) & empty & target_mask;

        for to in double_pushes.iter() {
            let from = match us {
                Color::White => to - 16,
                Color::Black => to + 16,
            };
            if pinned.contains(from) {
                let ray = pin_rays[0][from as usize] | pin_rays[1][from as usize];
                if !ray.contains(to) {
                    continue;
                }
            }
            moves.push(Move::new(Square(from), Square(to)));
        }

        // Pawn captures
        for from in our_pawns.iter() {
            let attacks = Self::pawn_attacks(Square(from), us);
            let captures = attacks & their_pieces & target_mask;

            for to in captures.iter() {
                if pinned.contains(from) {
                    let ray = pin_rays[0][from as usize] | pin_rays[1][from as usize];
                    if !ray.contains(to) {
                        continue;
                    }
                }
                if (Bitboard::from_square(to) & promo_rank).is_not_empty() {
                    for promo in [PieceType::Queen, PieceType::Rook, PieceType::Bishop, PieceType::Knight] {
                        moves.push(Move::new_promotion(Square(from), Square(to), promo, true));
                    }
                } else {
                    moves.push(Move::new_capture(Square(from), Square(to)));
                }
            }
        }

        // En passant
        if let Some(ep_sq) = board.en_passant {
            let ep_bb = ep_sq.bitboard();
            for from in our_pawns.iter() {
                let attacks = Self::pawn_attacks(Square(from), us);
                if (attacks & ep_bb).is_not_empty() {
                    // Verify en passant is legal (doesn't leave king in check)
                    let captured_sq = Square::new((ep_sq.0 as i8 - us.pawn_direction()) as u8);
                    
                    // Check for discovered check along the rank
                    let king_sq = board.king_square(us);
                    if king_sq.rank() == Square(from).rank() {
                        let rank_pieces = occupied & Bitboard::rank_mask(king_sq.rank());
                        let after_ep = rank_pieces ^ Bitboard::from_square(from) ^ captured_sq.bitboard();
                        let rank_attackers = (board.pieces(PieceType::Rook) | board.pieces(PieceType::Queen)) 
                            & board.color(them);
                        let attacks = Self::rook_attacks(king_sq, after_ep);
                        if (attacks & rank_attackers).is_not_empty() {
                            continue;
                        }
                    }

                    if pinned.contains(from) {
                        let ray = pin_rays[0][from as usize] | pin_rays[1][from as usize];
                        if !ray.contains(ep_sq.0) {
                            continue;
                        }
                    }

                    // Check if capturing the pawn satisfies check evasion
                    if board.checkers.is_not_empty() {
                        if !board.checkers.contains(captured_sq.0) && !target_mask.contains(ep_sq.0) {
                            continue;
                        }
                    }

                    moves.push(Move::new_en_passant(Square(from), ep_sq));
                }
            }
        }
    }

    fn generate_knight_moves(
        board: &Board,
        moves: &mut MoveList,
        pinned: Bitboard,
        target_mask: Bitboard,
    ) {
        let us = board.side_to_move;
        let our_knights = board.pieces_of(PieceType::Knight, us);
        let their_pieces = board.color(us.opposite());
        let our_pieces = board.color(us);

        for from in (our_knights & !pinned).iter() {
            let attacks = Self::knight_attacks(Square(from)) & !our_pieces & target_mask;
            for to in attacks.iter() {
                if their_pieces.contains(to) {
                    moves.push(Move::new_capture(Square(from), Square(to)));
                } else {
                    moves.push(Move::new(Square(from), Square(to)));
                }
            }
        }
        // Pinned knights can never move
    }

    fn generate_bishop_moves(
        board: &Board,
        moves: &mut MoveList,
        pinned: Bitboard,
        pin_rays: &[[Bitboard; 64]; 2],
        target_mask: Bitboard,
    ) {
        let us = board.side_to_move;
        let our_bishops = board.pieces_of(PieceType::Bishop, us);
        let their_pieces = board.color(us.opposite());
        let our_pieces = board.color(us);
        let occupied = board.occupied();

        for from in our_bishops.iter() {
            let mut attacks = Self::bishop_attacks(Square(from), occupied) & !our_pieces & target_mask;
            if pinned.contains(from) {
                attacks &= pin_rays[0][from as usize]; // Bishop can only move along diagonal pin ray
            }
            for to in attacks.iter() {
                if their_pieces.contains(to) {
                    moves.push(Move::new_capture(Square(from), Square(to)));
                } else {
                    moves.push(Move::new(Square(from), Square(to)));
                }
            }
        }
    }

    fn generate_rook_moves(
        board: &Board,
        moves: &mut MoveList,
        pinned: Bitboard,
        pin_rays: &[[Bitboard; 64]; 2],
        target_mask: Bitboard,
    ) {
        let us = board.side_to_move;
        let our_rooks = board.pieces_of(PieceType::Rook, us);
        let their_pieces = board.color(us.opposite());
        let our_pieces = board.color(us);
        let occupied = board.occupied();

        for from in our_rooks.iter() {
            let mut attacks = Self::rook_attacks(Square(from), occupied) & !our_pieces & target_mask;
            if pinned.contains(from) {
                attacks &= pin_rays[1][from as usize]; // Rook can only move along orthogonal pin ray
            }
            for to in attacks.iter() {
                if their_pieces.contains(to) {
                    moves.push(Move::new_capture(Square(from), Square(to)));
                } else {
                    moves.push(Move::new(Square(from), Square(to)));
                }
            }
        }
    }

    fn generate_queen_moves(
        board: &Board,
        moves: &mut MoveList,
        pinned: Bitboard,
        pin_rays: &[[Bitboard; 64]; 2],
        target_mask: Bitboard,
    ) {
        let us = board.side_to_move;
        let our_queens = board.pieces_of(PieceType::Queen, us);
        let their_pieces = board.color(us.opposite());
        let our_pieces = board.color(us);
        let occupied = board.occupied();

        for from in our_queens.iter() {
            let mut attacks = Self::queen_attacks(Square(from), occupied) & !our_pieces & target_mask;
            if pinned.contains(from) {
                // Queen pinned diagonally or orthogonally
                attacks &= pin_rays[0][from as usize] | pin_rays[1][from as usize];
            }
            for to in attacks.iter() {
                if their_pieces.contains(to) {
                    moves.push(Move::new_capture(Square(from), Square(to)));
                } else {
                    moves.push(Move::new(Square(from), Square(to)));
                }
            }
        }
    }

    fn generate_king_moves(board: &Board, moves: &mut MoveList) {
        let us = board.side_to_move;
        let them = us.opposite();
        let king_sq = board.king_square(us);
        let our_pieces = board.color(us);
        let their_pieces = board.color(them);
        let occupied = board.occupied();

        let attacks = Self::king_attacks(king_sq) & !our_pieces;

        for to in attacks.iter() {
            // Check if destination is attacked
            let after_occ = (occupied ^ king_sq.bitboard()) | Bitboard::from_square(to);
            if Self::is_square_attacked_with_occ(board, Square(to), them, after_occ) {
                continue;
            }
            if their_pieces.contains(to) {
                moves.push(Move::new_capture(king_sq, Square(to)));
            } else {
                moves.push(Move::new(king_sq, Square(to)));
            }
        }
    }

    fn generate_castling_moves(board: &Board, moves: &mut MoveList) {
        let us = board.side_to_move;
        let them = us.opposite();
        let occupied = board.occupied();

        // Can't castle out of check
        if board.is_check() {
            return;
        }

        let (king_sq, king_side_to, queen_side_to, king_path, queen_clear) = match us {
            Color::White => (
                Square::E1,
                Square::G1,
                Square::C1,
                Bitboard::new(0x60), // f1, g1
                Bitboard::new(0x0E), // b1, c1, d1
            ),
            Color::Black => (
                Square::E8,
                Square::G8,
                Square::C8,
                Bitboard::new(0x6000000000000000), // f8, g8
                Bitboard::new(0x0E00000000000000), // b8, c8, d8
            ),
        };

        // Kingside castling
        if board.castling.can_castle_kingside(us) {
            if (occupied & king_path).is_empty() {
                let through_sq = Square::new(king_sq.0 + 1);
                if !Self::is_square_attacked(board, through_sq, them) 
                    && !Self::is_square_attacked(board, king_side_to, them) 
                {
                    moves.push(Move::new_castling(king_sq, king_side_to));
                }
            }
        }

        // Queenside castling
        if board.castling.can_castle_queenside(us) {
            if (occupied & queen_clear).is_empty() {
                let through_sq = Square::new(king_sq.0 - 1);
                if !Self::is_square_attacked(board, through_sq, them) 
                    && !Self::is_square_attacked(board, queen_side_to, them) 
                {
                    moves.push(Move::new_castling(king_sq, queen_side_to));
                }
            }
        }
    }

    /// Check if a square is attacked by a given color
    fn is_square_attacked(board: &Board, sq: Square, by_color: Color) -> bool {
        board.is_attacked(sq, by_color)
    }

    /// Check if a square is attacked with custom occupancy
    fn is_square_attacked_with_occ(board: &Board, sq: Square, by_color: Color, occ: Bitboard) -> bool {
        let attackers = board.color(by_color);

        // Pawn attacks
        let pawn_attacks = Self::pawn_attacks(sq, by_color.opposite());
        if (pawn_attacks & board.pieces(PieceType::Pawn) & attackers).is_not_empty() {
            return true;
        }

        // Knight attacks
        let knight_attacks = Self::knight_attacks(sq);
        if (knight_attacks & board.pieces(PieceType::Knight) & attackers).is_not_empty() {
            return true;
        }

        // Bishop/Queen diagonal attacks
        let bishop_attacks = Self::bishop_attacks(sq, occ);
        if (bishop_attacks & (board.pieces(PieceType::Bishop) | board.pieces(PieceType::Queen)) & attackers).is_not_empty() {
            return true;
        }

        // Rook/Queen orthogonal attacks
        let rook_attacks = Self::rook_attacks(sq, occ);
        if (rook_attacks & (board.pieces(PieceType::Rook) | board.pieces(PieceType::Queen)) & attackers).is_not_empty() {
            return true;
        }

        // King attacks
        let king_attacks = Self::king_attacks(sq);
        if (king_attacks & board.pieces(PieceType::King) & attackers).is_not_empty() {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knight_attacks() {
        // Knight on e4 should attack 8 squares
        let attacks = MoveGen::knight_attacks(Square::from_algebraic("e4").unwrap());
        assert_eq!(attacks.count(), 8);
    }

    #[test]
    fn test_knight_attacks_corner() {
        // Knight on a1 should attack 2 squares
        let attacks = MoveGen::knight_attacks(Square::A1);
        assert_eq!(attacks.count(), 2);
    }

    #[test]
    fn test_king_attacks() {
        // King on e4 should attack 8 squares
        let attacks = MoveGen::king_attacks(Square::from_algebraic("e4").unwrap());
        assert_eq!(attacks.count(), 8);
    }

    #[test]
    fn test_king_attacks_corner() {
        // King on a1 should attack 3 squares
        let attacks = MoveGen::king_attacks(Square::A1);
        assert_eq!(attacks.count(), 3);
    }

    #[test]
    fn test_rook_attacks_empty_board() {
        let attacks = MoveGen::rook_attacks(Square::E4, Bitboard::EMPTY);
        assert_eq!(attacks.count(), 14); // 7 horizontal + 7 vertical
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
        assert_eq!(moves.len(), 20); // 16 pawn moves + 4 knight moves
    }

    #[test]
    fn test_perft_initial() {
        let board = Board::startpos();
        assert_eq!(perft(&board, 1), 20);
        assert_eq!(perft(&board, 2), 400);
        assert_eq!(perft(&board, 3), 8902);
    }

    #[test]
    fn test_perft_kiwipete() {
        // Famous test position
        let board = Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
        assert_eq!(perft(&board, 1), 48);
        assert_eq!(perft(&board, 2), 2039);
    }

    #[test]
    fn test_en_passant() {
        let board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3").unwrap();
        let moves = MoveGen::generate_legal_moves(&board);
        let ep_moves: Vec<_> = moves.iter().filter(|m| m.is_en_passant()).collect();
        assert_eq!(ep_moves.len(), 1);
    }

    #[test]
    fn test_castling() {
        let board = Board::from_fen("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1").unwrap();
        let moves = MoveGen::generate_legal_moves(&board);
        let castle_moves: Vec<_> = moves.iter().filter(|m| m.is_castling()).collect();
        assert_eq!(castle_moves.len(), 2); // Both kingside and queenside
    }

    #[test]
    fn test_promotion() {
        let board = Board::from_fen("8/P7/8/8/8/8/8/4K2k w - - 0 1").unwrap();
        let moves = MoveGen::generate_legal_moves(&board);
        let promo_moves: Vec<_> = moves.iter().filter(|m| m.is_promotion()).collect();
        assert_eq!(promo_moves.len(), 4); // Q, R, B, N
    }

    /// Perft function for testing move generation correctness
    fn perft(board: &Board, depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }

        let moves = MoveGen::generate_legal_moves(board);
        let mut nodes = 0;

        for mv in moves.iter() {
            let mut new_board = board.clone();
            if new_board.make_move(*mv) {
                nodes += perft(&new_board, depth - 1);
            }
        }

        nodes
    }
}

