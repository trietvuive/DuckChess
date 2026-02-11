//! Search Algorithm
//!
//! Implements the alpha-beta search with various optimizations:
//! - Iterative deepening
//! - Transposition table
//! - Move ordering (MVV-LVA, killer moves, history heuristic)
//! - Null move pruning
//! - Late move reductions (LMR)
//! - Quiescence search
//! - Aspiration windows

use crate::core::board::{Board, Color, PieceType};
use super::movegen::MoveGen;
use crate::core::moves::{Move, MoveList};
use super::nnue::{evaluate, NNUEAccumulator, NNUEEvaluator};
use super::tt::{TTFlag, TranspositionTable};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Infinity score (used for mate scores)
pub const INFINITY: i32 = 30000;
/// Mate score base
pub const MATE_SCORE: i32 = 29000;
/// Draw score
pub const DRAW_SCORE: i32 = 0;

/// Maximum search depth
pub const MAX_DEPTH: i32 = 64;

/// Search limits
#[derive(Clone, Debug)]
pub struct SearchLimits {
    /// Maximum depth to search
    pub depth: Option<i32>,
    /// Maximum nodes to search
    pub nodes: Option<u64>,
    /// Time limit in milliseconds
    pub movetime: Option<u64>,
    /// White's time in milliseconds
    pub wtime: Option<u64>,
    /// Black's time in milliseconds
    pub btime: Option<u64>,
    /// White's increment in milliseconds
    pub winc: Option<u64>,
    /// Black's increment in milliseconds
    pub binc: Option<u64>,
    /// Moves to go until next time control
    pub movestogo: Option<u32>,
    /// Infinite search (until stopped)
    pub infinite: bool,
}

impl Default for SearchLimits {
    fn default() -> Self {
        SearchLimits {
            depth: None,
            nodes: None,
            movetime: None,
            wtime: None,
            btime: None,
            winc: None,
            binc: None,
            movestogo: None,
            infinite: false,
        }
    }
}

/// Search statistics
#[derive(Clone, Debug, Default)]
pub struct SearchStats {
    pub nodes: u64,
    pub qnodes: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
}

/// Killer moves (quiet moves that caused beta cutoffs)
#[derive(Clone)]
struct KillerMoves {
    moves: [[Move; 2]; MAX_DEPTH as usize],
}

impl KillerMoves {
    fn new() -> Self {
        KillerMoves {
            moves: [[Move::NULL; 2]; MAX_DEPTH as usize],
        }
    }

    fn add(&mut self, mv: Move, ply: usize) {
        if ply < MAX_DEPTH as usize && mv != self.moves[ply][0] {
            self.moves[ply][1] = self.moves[ply][0];
            self.moves[ply][0] = mv;
        }
    }

    fn is_killer(&self, mv: Move, ply: usize) -> bool {
        ply < MAX_DEPTH as usize && (mv == self.moves[ply][0] || mv == self.moves[ply][1])
    }
}

impl Default for KillerMoves {
    fn default() -> Self {
        Self::new()
    }
}

/// History heuristic table
#[derive(Clone)]
struct HistoryTable {
    table: [[i32; 64]; 64], // [from][to]
}

impl HistoryTable {
    fn new() -> Self {
        HistoryTable {
            table: [[0; 64]; 64],
        }
    }

    fn add(&mut self, mv: Move, depth: i32) {
        let from = mv.from().index();
        let to = mv.to().index();
        self.table[from][to] += depth * depth;
        
        // Prevent overflow
        if self.table[from][to] > 10000 {
            for row in self.table.iter_mut() {
                for val in row.iter_mut() {
                    *val /= 2;
                }
            }
        }
    }

    fn get(&self, mv: Move) -> i32 {
        self.table[mv.from().index()][mv.to().index()]
    }

    fn clear(&mut self) {
        self.table = [[0; 64]; 64];
    }
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self::new()
    }
}

/// The main search engine
pub struct Searcher {
    /// Transposition table
    pub tt: TranspositionTable,
    /// Stop flag (for async stopping)
    pub stop: Arc<AtomicBool>,
    /// Killer moves
    killers: KillerMoves,
    /// History heuristic
    history: HistoryTable,
    /// Search statistics
    pub stats: SearchStats,
    /// Start time
    start_time: Instant,
    /// Time limit
    time_limit: Option<Duration>,
    /// Node limit
    node_limit: Option<u64>,
    /// NNUE evaluator
    evaluator: NNUEEvaluator,
    /// NNUE accumulator
    accumulator: NNUEAccumulator,
}

impl Searcher {
    /// Create a new searcher
    pub fn new() -> Self {
        Searcher {
            tt: TranspositionTable::new(64),
            stop: Arc::new(AtomicBool::new(false)),
            killers: KillerMoves::new(),
            history: HistoryTable::new(),
            stats: SearchStats::default(),
            start_time: Instant::now(),
            time_limit: None,
            node_limit: None,
            evaluator: NNUEEvaluator::new(),
            accumulator: NNUEAccumulator::new(),
        }
    }

    /// Set the transposition table size in MB
    pub fn set_hash_size(&mut self, size_mb: usize) {
        self.tt = TranspositionTable::new(size_mb);
    }

    /// Clear the search state
    pub fn clear(&mut self) {
        self.tt.clear();
        self.killers = KillerMoves::new();
        self.history.clear();
    }

    /// Calculate time to search based on limits and side to move
    pub fn calculate_time(&self, limits: &SearchLimits, side: Color) -> Option<Duration> {
        if let Some(movetime) = limits.movetime {
            return Some(Duration::from_millis(movetime));
        }

        let (time, inc) = match side {
            Color::White => (limits.wtime, limits.winc.unwrap_or(0)),
            Color::Black => (limits.btime, limits.binc.unwrap_or(0)),
        };

        if let Some(time) = time {
            let moves_to_go = limits.movestogo.unwrap_or(30) as u64;
            let time_per_move = time / moves_to_go + inc / 2;
            // Use at most 50% of remaining time
            let max_time = time / 2;
            let allocated = time_per_move.min(max_time);
            Some(Duration::from_millis(allocated.max(10)))
        } else {
            None
        }
    }

    /// Check if we should stop searching
    #[inline]
    fn should_stop(&self) -> bool {
        if self.stop.load(Ordering::Relaxed) {
            return true;
        }

        // Check node limit
        if let Some(limit) = self.node_limit {
            if self.stats.nodes >= limit {
                return true;
            }
        }

        // Check time limit (every 1024 nodes to avoid syscall overhead)
        if self.stats.nodes & 1023 == 0 {
            if let Some(limit) = self.time_limit {
                if self.start_time.elapsed() >= limit {
                    return true;
                }
            }
        }

        false
    }

    /// Main search function with iterative deepening
    pub fn search(&mut self, board: &Board, limits: SearchLimits) -> Move {
        self.start_time = Instant::now();
        self.stats = SearchStats::default();
        self.stop.store(false, Ordering::Relaxed);
        self.tt.new_search();

        // Calculate time limit
        self.time_limit = self.calculate_time(&limits, board.side_to_move);
        self.node_limit = limits.nodes;

        // Refresh NNUE accumulator
        self.accumulator.refresh(board, &self.evaluator.network);

        let max_depth = limits.depth.unwrap_or(MAX_DEPTH);
        let mut best_move = Move::NULL;
        let mut best_score = -INFINITY;

        // Iterative deepening
        for depth in 1..=max_depth {
            if self.should_stop() {
                break;
            }

            // Aspiration windows for deeper searches
            let (mut alpha, mut beta) = if depth >= 4 {
                (best_score - 50, best_score + 50)
            } else {
                (-INFINITY, INFINITY)
            };

            let mut score;
            loop {
                score = self.alpha_beta(board, depth, alpha, beta, 0, true);

                if self.should_stop() {
                    break;
                }

                // Re-search with wider window if score is outside aspiration window
                if score <= alpha {
                    alpha = -INFINITY;
                } else if score >= beta {
                    beta = INFINITY;
                } else {
                    break;
                }
            }

            if self.should_stop() {
                break;
            }

            // Get best move from TT
            if let Some(entry) = self.tt.probe(board.hash) {
                if !entry.best_move.is_null() {
                    best_move = entry.best_move;
                    best_score = score;
                }
            }

            // Print UCI info
            let elapsed = self.start_time.elapsed();
            let nps = if elapsed.as_millis() > 0 {
                (self.stats.nodes as u128 * 1000) / elapsed.as_millis()
            } else {
                0
            };

            let score_str = if score.abs() >= MATE_SCORE - MAX_DEPTH {
                let mate_in = if score > 0 {
                    (MATE_SCORE - score + 1) / 2
                } else {
                    -(MATE_SCORE + score) / 2
                };
                format!("mate {}", mate_in)
            } else {
                format!("cp {}", score)
            };

            println!(
                "info depth {} score {} nodes {} nps {} time {} hashfull {} pv {}",
                depth,
                score_str,
                self.stats.nodes,
                nps,
                elapsed.as_millis(),
                self.tt.hashfull(),
                best_move.to_uci()
            );

            // Stop if we found a mate
            if score.abs() >= MATE_SCORE - depth {
                break;
            }
        }

        // If no move found in TT, get first legal move
        if best_move.is_null() {
            let moves = MoveGen::generate_legal_moves(board);
            if moves.len() > 0 {
                best_move = moves[0];
            }
        }

        best_move
    }

    /// Alpha-beta search with fail-soft
    fn alpha_beta(
        &mut self,
        board: &Board,
        mut depth: i32,
        mut alpha: i32,
        beta: i32,
        ply: usize,
        is_pv: bool,
    ) -> i32 {
        // Check for stop condition
        if self.should_stop() {
            return 0;
        }

        self.stats.nodes += 1;

        // Mate distance pruning
        let mate_value = MATE_SCORE - ply as i32;
        if alpha >= mate_value {
            return mate_value;
        }
        if beta <= -mate_value {
            return -mate_value;
        }

        // Check for draw
        if ply > 0 && (board.halfmove_clock >= 100 || board.is_insufficient_material()) {
            return DRAW_SCORE;
        }

        // Probe transposition table
        let tt_entry = self.tt.probe(board.hash);
        let mut tt_move = Move::NULL;
        
        if let Some(entry) = tt_entry {
            self.stats.tt_hits += 1;
            tt_move = entry.best_move;
            
            if !is_pv && entry.depth >= depth as i8 {
                let tt_score = entry.score as i32;
                match entry.flag {
                    TTFlag::Exact => return tt_score,
                    TTFlag::LowerBound if tt_score >= beta => {
                        self.stats.tt_cutoffs += 1;
                        return tt_score;
                    }
                    TTFlag::UpperBound if tt_score <= alpha => {
                        self.stats.tt_cutoffs += 1;
                        return tt_score;
                    }
                    _ => {}
                }
            }
        }

        // Quiescence search at depth 0
        if depth <= 0 {
            return self.quiescence(board, alpha, beta, ply);
        }

        let in_check = board.is_check();

        // Check extension
        if in_check {
            depth += 1;
        }

        // Null move pruning (not in PV, not in check, have non-pawn material)
        if !is_pv && !in_check && depth >= 3 && ply > 0 {
            let has_pieces = (board.pieces(PieceType::Knight)
                | board.pieces(PieceType::Bishop)
                | board.pieces(PieceType::Rook)
                | board.pieces(PieceType::Queen))
                & board.color(board.side_to_move);
            
            if has_pieces.is_not_empty() {
                // Make null move
                let mut null_board = board.clone();
                null_board.side_to_move = null_board.side_to_move.opposite();
                null_board.en_passant = None;
                null_board.update_checkers();

                let reduction = 3 + depth / 6;
                let null_score = -self.alpha_beta(&null_board, depth - 1 - reduction, -beta, -beta + 1, ply + 1, false);

                if null_score >= beta {
                    return beta;
                }
            }
        }

        // Generate moves
        let moves = MoveGen::generate_legal_moves(board);

        // Checkmate or stalemate
        if moves.is_empty() {
            return if in_check {
                -MATE_SCORE + ply as i32
            } else {
                DRAW_SCORE
            };
        }

        // Order moves
        let ordered_moves = self.order_moves(board, &moves, tt_move, ply);

        let mut best_score = -INFINITY;
        let mut best_move = Move::NULL;
        let mut move_count = 0;

        for mv in ordered_moves {
            let mut new_board = board.clone();
            if !new_board.make_move(mv) {
                continue;
            }

            move_count += 1;

            let mut score;

            // Late move reductions
            if move_count > 3 && depth >= 3 && !in_check && !mv.is_capture() && !mv.is_promotion() {
                // Reduced depth search
                let reduction = 1 + (move_count / 8).min(2) as i32;
                score = -self.alpha_beta(&new_board, depth - 1 - reduction, -alpha - 1, -alpha, ply + 1, false);

                // Re-search at full depth if the reduced search looks promising
                if score > alpha {
                    score = -self.alpha_beta(&new_board, depth - 1, -alpha - 1, -alpha, ply + 1, false);
                }
            } else if !is_pv || move_count > 1 {
                // PVS: Search with null window for non-first moves
                score = -self.alpha_beta(&new_board, depth - 1, -alpha - 1, -alpha, ply + 1, false);
            } else {
                score = alpha + 1; // Force full search for first move in PV
            }

            // Full window search if needed
            if score > alpha && (is_pv || score < beta) {
                score = -self.alpha_beta(&new_board, depth - 1, -beta, -alpha, ply + 1, is_pv);
            }

            if score > best_score {
                best_score = score;
                best_move = mv;

                if score > alpha {
                    alpha = score;

                    if score >= beta {
                        // Beta cutoff
                        if !mv.is_capture() {
                            self.killers.add(mv, ply);
                            self.history.add(mv, depth);
                        }
                        break;
                    }
                }
            }
        }

        // Store in transposition table
        let flag = if best_score >= beta {
            TTFlag::LowerBound
        } else if best_score > alpha - 1 { // was: best_score > old_alpha but we don't track old_alpha
            TTFlag::Exact
        } else {
            TTFlag::UpperBound
        };

        self.tt.store(board.hash, best_move, depth as i8, best_score as i16, flag);

        best_score
    }

    /// Quiescence search - only search captures to reach a quiet position
    fn quiescence(&mut self, board: &Board, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.stats.qnodes += 1;

        // Standing pat
        let stand_pat = evaluate(board);

        if stand_pat >= beta {
            return beta;
        }

        if stand_pat > alpha {
            alpha = stand_pat;
        }

        // Generate captures
        let moves = MoveGen::generate_legal_moves(board);

        for mv in moves.iter() {
            // Only search captures (and promotions)
            if !mv.is_capture() && !mv.is_promotion() {
                continue;
            }

            // Delta pruning: skip captures that can't improve alpha
            // (simplified - in practice you'd check the captured piece value)
            if stand_pat + 1000 < alpha && !mv.is_promotion() {
                continue;
            }

            let mut new_board = board.clone();
            if !new_board.make_move(*mv) {
                continue;
            }

            let score = -self.quiescence(&new_board, -beta, -alpha, ply + 1);

            if score >= beta {
                return beta;
            }

            if score > alpha {
                alpha = score;
            }
        }

        alpha
    }

    /// Order moves for better alpha-beta pruning
    fn order_moves(&self, board: &Board, moves: &MoveList, tt_move: Move, ply: usize) -> Vec<Move> {
        let mut scored_moves: Vec<(Move, i32)> = moves
            .iter()
            .map(|&mv| {
                let mut score = 0i32;

                // TT move gets highest priority
                if mv.raw() == tt_move.raw() {
                    score += 10000000;
                }
                // MVV-LVA for captures
                else if mv.is_capture() {
                    let victim = if mv.is_en_passant() {
                        Some(PieceType::Pawn)
                    } else {
                        board.piece_at[mv.to().index()].map(|p| p.piece_type)
                    };
                    let attacker = board.piece_at[mv.from().index()].map(|p| p.piece_type);

                    if let (Some(v), Some(a)) = (victim, attacker) {
                        let victim_val = Self::piece_value(v);
                        let attacker_val = Self::piece_value(a);
                        score += 1000000 + victim_val * 10 - attacker_val;
                    }
                }
                // Promotions
                else if mv.is_promotion() {
                    score += 900000;
                    if let Some(promo) = mv.promotion_piece() {
                        score += Self::piece_value(promo);
                    }
                }
                // Killer moves
                else if self.killers.is_killer(mv, ply) {
                    score += 800000;
                }
                // History heuristic
                else {
                    score += self.history.get(mv);
                }

                (mv, score)
            })
            .collect();

        scored_moves.sort_by(|a, b| b.1.cmp(&a.1));
        scored_moves.into_iter().map(|(mv, _)| mv).collect()
    }

    /// Get piece value for MVV-LVA
    fn piece_value(piece_type: PieceType) -> i32 {
        match piece_type {
            PieceType::Pawn => 100,
            PieceType::Knight => 320,
            PieceType::Bishop => 330,
            PieceType::Rook => 500,
            PieceType::Queen => 900,
            PieceType::King => 20000,
        }
    }
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}

