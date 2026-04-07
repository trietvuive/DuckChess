//! Searcher: iterative deepening driver, time/node limits, and search entry point.

use shakmaty::{Chess, Color, Move, Position};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::engine::book::OpeningBook;
use crate::engine::eval::{EvalKind, Evaluator};
use crate::engine::tt::TranspositionTable;

use super::ordering::{self, HistoryTable, KillerMoves};
use super::pv::{self, get_hash};
use super::types::{INFINITY, MATE_SCORE, MAX_DEPTH, SearchLimits, SearchStats};

pub struct Searcher {
    pub(super) tt: Arc<TranspositionTable>,
    pub(super) killers: KillerMoves,
    pub(super) history: HistoryTable,
    pub(super) stats: SearchStats,
    pub(super) stop: Arc<AtomicBool>,
    pub(super) start_time: Instant,
    pub(super) time_limit: Option<Duration>,
    pub(super) node_limit: Option<u64>,
    evaluator: Evaluator,
    /// UCI MultiPV default (clamped 1..=5 when set); per-`go` may still override via [`SearchLimits`].
    pub(super) multi_pv: u32,
    /// Opening book from UCI `BookPath` (PGN load).
    book: Option<OpeningBook>,
    /// UCI `OwnBook`: whether to play book moves when available.
    own_book: bool,
    num_threads: usize,
    /// Thread index (0 = main thread, only main reports UCI info).
    thread_id: usize,
    /// Shared node counter across threads; `None` in single-threaded mode.
    pub(super) global_nodes: Option<Arc<AtomicU64>>,
    /// UCI `Randomness`: centipawn window for random root move selection.
    /// 0 = deterministic (always best move). Higher values widen the pool of
    /// candidate moves: any move within this many centipawns of the best is
    /// eligible, and one is chosen uniformly at random.
    randomness: u32,
}

impl Searcher {
    pub fn new() -> Self {
        Searcher {
            tt: Arc::new(TranspositionTable::default()),
            killers: KillerMoves::new(),
            history: HistoryTable::new(),
            stats: SearchStats::default(),
            stop: Arc::new(AtomicBool::new(false)),
            start_time: Instant::now(),
            time_limit: None,
            node_limit: None,
            evaluator: Evaluator::new(),
            multi_pv: 1,
            book: None,
            own_book: true,
            num_threads: 1,
            thread_id: 0,
            global_nodes: None,
            randomness: 0,
        }
    }

    /// UCI `BookPath`: load PGN book from path, or clear if `path` is empty.
    pub fn set_book_pgn_path(&mut self, path: &str) {
        self.book = if path.is_empty() {
            None
        } else {
            OpeningBook::load_pgn(Path::new(path)).ok()
        };
    }

    /// UCI `OwnBook`: use opening book on search when loaded.
    pub fn set_own_book(&mut self, enabled: bool) {
        self.own_book = enabled;
    }

    pub fn own_book(&self) -> bool {
        self.own_book
    }

    /// UCI option `MultiPV`: number of principal variations (1..=5).
    pub fn set_multi_pv(&mut self, n: u32) {
        self.multi_pv = n.clamp(1, 5);
    }

    pub fn multi_pv(&self) -> u32 {
        self.multi_pv
    }

    /// UCI `Eval`: material vs NNUE (shared leaf pipeline in [`Evaluator`]).
    pub fn set_eval_kind(&mut self, kind: EvalKind) {
        self.evaluator.set_backend(kind);
    }

    pub fn eval_kind(&self) -> EvalKind {
        self.evaluator.backend()
    }

    /// UCI `Randomness`: centipawn window for random root move selection (0..=200).
    pub fn set_randomness(&mut self, cp: u32) {
        self.randomness = cp.min(200);
    }

    pub fn randomness(&self) -> u32 {
        self.randomness
    }

    /// Static evaluation (centipawns, side to move).
    pub fn evaluate_position(&self, pos: &Chess) -> i32 {
        self.evaluator.evaluate(pos)
    }

    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop.clone()
    }

    pub fn set_hash_size(&mut self, size_mb: usize) {
        self.tt = Arc::new(TranspositionTable::new(size_mb));
    }

    /// UCI `Threads` option (1..=256).
    pub fn set_threads(&mut self, n: usize) {
        self.num_threads = n.clamp(1, 256);
    }

    pub fn threads(&self) -> usize {
        self.num_threads
    }

    pub fn clear(&mut self) {
        self.tt.clear();
        self.killers = KillerMoves::new();
        self.history.clear();
    }

    pub(super) fn should_stop(&self) -> bool {
        if self.stop.load(Ordering::Relaxed) {
            return true;
        }
        if let Some(limit) = self.node_limit {
            let nodes = self
                .global_nodes
                .as_ref()
                .map(|gn| gn.load(Ordering::Relaxed))
                .unwrap_or(self.stats.nodes);
            if nodes >= limit {
                return true;
            }
        }
        if self
            .time_limit
            .is_some_and(|limit| self.start_time.elapsed() >= limit)
        {
            return true;
        }
        false
    }

    /// Total nodes across all threads (or local count in single-threaded mode).
    pub(super) fn total_nodes(&self) -> u64 {
        self.global_nodes
            .as_ref()
            .map(|gn| gn.load(Ordering::Relaxed))
            .unwrap_or(self.stats.nodes)
    }

    pub fn calculate_time(&self, limits: &SearchLimits, side: Color) -> Option<Duration> {
        if limits.infinite {
            return None;
        }
        if let Some(mt) = limits.movetime {
            return Some(Duration::from_millis(mt));
        }

        let time = match side {
            Color::White => limits.wtime,
            Color::Black => limits.btime,
        }?;

        let inc = match side {
            Color::White => limits.winc.unwrap_or(0),
            Color::Black => limits.binc.unwrap_or(0),
        };
        let moves_to_go = limits.movestogo.unwrap_or(30) as u64;
        let time_for_move = time / moves_to_go + inc / 2;
        Some(Duration::from_millis(time_for_move.min(time / 2)))
    }

    pub(super) fn get_pv_from_tt(&self, pos: &Chess, max_ply: usize) -> Vec<Move> {
        pv::get_pv_from_tt(&self.tt, pos, max_ply)
    }

    pub(super) fn report_info(&self, depth: i32, multipv: u32, score: i32, pv: &[Move]) {
        if self.thread_id != 0 {
            return;
        }
        pv::report_info(
            &self.tt,
            self.total_nodes(),
            self.start_time,
            depth,
            multipv,
            score,
            pv,
        );
    }

    pub(super) fn order_moves(
        &self,
        pos: &Chess,
        moves: &shakmaty::MoveList,
        tt_move: Option<&Move>,
        ply: usize,
    ) -> Vec<Move> {
        ordering::order_moves(pos, moves, tt_move, ply, &self.killers, &self.history)
    }

    fn probe_book(&self, pos: &Chess) -> Option<Move> {
        self.book
            .as_ref()
            .filter(|_| self.own_book)
            .and_then(|book| book.probe(pos))
    }

    pub fn search(&mut self, pos: &Chess, limits: SearchLimits) -> Option<Move> {
        if let Some(mv) = self.probe_book(pos) {
            return Some(mv);
        }

        self.stop.store(false, Ordering::Relaxed);
        self.start_time = Instant::now();
        self.stats = SearchStats::default();
        self.tt.new_search();
        self.time_limit = self
            .calculate_time(&limits, pos.turn())
            .or(Some(Duration::from_secs(10)));
        {
            use std::io::Write as _;
            let _ = writeln!(
                std::io::stderr(),
                "debug search: side={:?} time_limit={:?}ms threads={}",
                pos.turn(),
                self.time_limit.map(|d| d.as_millis()),
                self.num_threads,
            );
            let _ = std::io::stderr().flush();
        }
        self.node_limit = limits.nodes;
        self.killers = KillerMoves::new();
        self.history.clear();

        let max_depth = limits.depth.unwrap_or(MAX_DEPTH);
        let multi_pv = limits.multi_pv.max(1);

        if self.num_threads <= 1 {
            return self.search_iterative(pos, max_depth, multi_pv);
        }

        // Lazy SMP: all threads share the TT and stop flag, each with independent
        // killer/history tables. The TT is the main communication channel — entries
        // written by helpers refine the main thread's search.
        let global_nodes = Arc::new(AtomicU64::new(0));
        self.global_nodes = Some(Arc::clone(&global_nodes));
        self.thread_id = 0;

        let num_helpers = self.num_threads - 1;
        let tt = Arc::clone(&self.tt);
        let stop = Arc::clone(&self.stop);
        let evaluator = self.evaluator;
        let start_time = self.start_time;
        let time_limit = self.time_limit;
        let node_limit = self.node_limit;

        let result = std::thread::scope(|s| {
            for i in 0..num_helpers {
                let tt = Arc::clone(&tt);
                let stop = Arc::clone(&stop);
                let global_nodes = Arc::clone(&global_nodes);
                let pos = pos.clone();

                let randomness = self.randomness;
                s.spawn(move || {
                    let mut worker = Searcher {
                        tt,
                        killers: KillerMoves::new(),
                        history: HistoryTable::new(),
                        stats: SearchStats::default(),
                        stop,
                        start_time,
                        time_limit,
                        node_limit,
                        evaluator,
                        multi_pv: 1,
                        book: None,
                        own_book: false,
                        num_threads: 1,
                        thread_id: i + 1,
                        global_nodes: Some(global_nodes),
                        randomness,
                    };
                    worker.search_iterative(&pos, max_depth, multi_pv);
                });
            }

            let result = self.search_iterative(pos, max_depth, multi_pv);
            self.stop.store(true, Ordering::Relaxed);
            result
        });

        self.global_nodes = None;
        result
    }

    /// Iterative deepening loop used by both the main thread and Lazy SMP helpers.
    fn search_iterative(&mut self, pos: &Chess, max_depth: i32, multi_pv: u32) -> Option<Move> {
        let mut best_move: Option<Move> = None;
        let mut best_score = -INFINITY;
        let use_explicit_root = multi_pv > 1 || self.randomness > 0;

        for depth in 1..=max_depth {
            if self.should_stop() {
                break;
            }

            if !use_explicit_root {
                let (mut alpha, mut beta) = if depth >= 4 {
                    (best_score - 50, best_score + 50)
                } else {
                    (-INFINITY, INFINITY)
                };

                let mut score = best_score;
                loop {
                    if self.should_stop() {
                        break;
                    }
                    let s = self.negamax(pos, depth, alpha, beta, 0, true);
                    if self.should_stop() {
                        break;
                    }
                    if s <= alpha {
                        alpha = -INFINITY;
                    } else if s >= beta {
                        beta = INFINITY;
                    } else {
                        score = s;
                        break;
                    }
                }

                if self.should_stop() {
                    break;
                }

                best_score = score;
                let hash = get_hash(pos);
                if let Some(entry) = self.tt.probe(hash).filter(|e| e.best_move.is_some()) {
                    best_move = entry.best_move;
                }

                let pv = self.get_pv_from_tt(pos, depth as usize + 1);
                self.report_info(depth, 1, best_score, &pv);
            } else {
                let legals = pos.legal_moves();
                let ordered = self.order_moves(pos, &legals, None, 0);
                let mut root_scores: Vec<(Move, i32)> = Vec::with_capacity(ordered.len());

                for mv in &ordered {
                    if self.should_stop() {
                        break;
                    }
                    let new_pos = pos.clone().play(mv).unwrap();
                    let score = -self.negamax(&new_pos, depth - 1, -INFINITY, INFINITY, 1, true);
                    root_scores.push((mv.clone(), score));
                }

                if self.should_stop() {
                    break;
                }

                root_scores.sort_by(|a, b| b.1.cmp(&a.1));
                let n_report = (multi_pv as usize).min(root_scores.len());
                best_score = root_scores[0].1;
                best_move = Some(root_scores[0].0.clone());

                if self.randomness > 0 {
                    let threshold = best_score - self.randomness as i32;
                    let candidates: Vec<&(Move, i32)> = root_scores
                        .iter()
                        .filter(|(_, s)| *s >= threshold)
                        .collect();
                    let chosen = &candidates[rand::random_range(0..candidates.len())];
                    best_move = Some(chosen.0.clone());
                    best_score = chosen.1;
                }

                for (pv_index, (first_mv, score)) in
                    root_scores.into_iter().take(n_report).enumerate()
                {
                    let rest =
                        self.get_pv_from_tt(&pos.clone().play(&first_mv).unwrap(), depth as usize);
                    let mut pv = vec![first_mv];
                    pv.extend(rest);
                    self.report_info(depth, (pv_index + 1) as u32, score, &pv);
                }
            }

            if best_score.abs() >= MATE_SCORE - depth {
                break;
            }
        }

        if best_move.is_none() {
            let legals = pos.legal_moves();
            if !legals.is_empty() {
                best_move = Some(legals[0].clone());
            }
        }

        best_move
    }
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}
