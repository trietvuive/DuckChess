//! Searcher: iterative deepening driver, time/node limits, and search entry point.

use shakmaty::{Chess, Color, Move, Position};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::engine::tt::TranspositionTable;

use super::ordering::{self, KillerMoves, HistoryTable};
use super::pv::{self, get_hash};
use super::types::{SearchLimits, SearchStats, INFINITY, MATE_SCORE, MAX_DEPTH};

pub struct Searcher {
    pub(super) tt: TranspositionTable,
    pub(super) killers: KillerMoves,
    pub(super) history: HistoryTable,
    pub(super) stats: SearchStats,
    pub(super) stop: Arc<AtomicBool>,
    pub(super) start_time: Instant,
    pub(super) time_limit: Option<Duration>,
    pub(super) node_limit: Option<u64>,
}

impl Searcher {
    pub fn new() -> Self {
        Searcher {
            tt: TranspositionTable::default(),
            killers: KillerMoves::new(),
            history: HistoryTable::new(),
            stats: SearchStats::default(),
            stop: Arc::new(AtomicBool::new(false)),
            start_time: Instant::now(),
            time_limit: None,
            node_limit: None,
        }
    }

    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop.clone()
    }

    pub fn set_hash_size(&mut self, size_mb: usize) {
        self.tt = TranspositionTable::new(size_mb);
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
            if self.stats.nodes >= limit {
                return true;
            }
        }
        if let Some(limit) = self.time_limit {
            if self.start_time.elapsed() >= limit {
                return true;
            }
        }
        false
    }

    pub fn calculate_time(&self, limits: &SearchLimits, side: Color) -> Option<Duration> {
        if limits.infinite {
            return None;
        }
        if let Some(mt) = limits.movetime {
            return Some(Duration::from_millis(mt));
        }
        let (time, inc) = match side {
            Color::White => (limits.wtime?, limits.winc.unwrap_or(0)),
            Color::Black => (limits.btime?, limits.binc.unwrap_or(0)),
        };
        let moves_to_go = limits.movestogo.unwrap_or(30) as u64;
        let time_for_move = time / moves_to_go + inc / 2;
        Some(Duration::from_millis(time_for_move.min(time / 2)))
    }

    pub(super) fn get_pv_from_tt(&self, pos: &Chess, max_plies: usize) -> Vec<Move> {
        pv::get_pv_from_tt(&self.tt, pos, max_plies)
    }

    pub(super) fn report_info(&self, depth: i32, multipv: u32, score: i32, pv: &[Move]) {
        pv::report_info(
            &self.tt,
            &self.stats,
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

    pub fn search(&mut self, pos: &Chess, limits: SearchLimits) -> Option<Move> {
        self.stop.store(false, Ordering::Relaxed);
        self.start_time = Instant::now();
        self.stats = SearchStats::default();
        self.tt.new_search();
        self.time_limit = self.calculate_time(&limits, pos.turn());
        self.node_limit = limits.nodes;

        let max_depth = limits.depth.unwrap_or(MAX_DEPTH);
        let multi_pv = limits.multi_pv.max(1);
        let mut best_move: Option<Move> = None;
        let mut best_score = -INFINITY;

        for depth in 1..=max_depth {
            if self.should_stop() {
                break;
            }

            if multi_pv <= 1 {
                let (mut alpha, mut beta) = if depth >= 4 {
                    (best_score - 50, best_score + 50)
                } else {
                    (-INFINITY, INFINITY)
                };

                let mut score = best_score;
                loop {
                    let s = self.alpha_beta(pos, depth, alpha, beta, 0, true);
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
                if let Some(entry) = self.tt.probe(hash) {
                    if entry.best_move.is_some() {
                        best_move = entry.best_move.clone();
                    }
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
                    let score =
                        -self.alpha_beta(&new_pos, depth - 1, -INFINITY, INFINITY, 1, true);
                    root_scores.push((mv.clone(), score));
                }

                if self.should_stop() {
                    break;
                }

                root_scores.sort_by(|a, b| b.1.cmp(&a.1));
                let n_report = (multi_pv as usize).min(root_scores.len());
                best_score = root_scores[0].1;
                best_move = Some(root_scores[0].0.clone());

                for (pv_index, (first_mv, score)) in
                    root_scores.into_iter().take(n_report).enumerate()
                {
                    let rest = self.get_pv_from_tt(
                        &pos.clone().play(&first_mv).unwrap(),
                        depth as usize,
                    );
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
