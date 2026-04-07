//! Negamax search with alpha-beta pruning, TT, null-move, LMR, and quiescence.
//!
//! # Negamax
//! All scores are from the **side to move**'s perspective. Instead of separate min and max layers,
//! child results are negated: `score = max_move (-negamax(child, -β, -α))`, with `α`/`β` swapped
//! and negated at each ply ([`SearchContext::negate`]). That is equivalent to minimax but with one
//! recursive routine.

use std::sync::atomic::Ordering;

use shakmaty::{Chess, Move, Position};

use crate::engine::tt::TTFlag;

use super::pv::get_hash;
use super::searcher::Searcher;
use super::types::{DRAW_SCORE, MATE_SCORE};
use super::types::{MoveContext, SearchContext};

impl Searcher {
    /// Core depth-limited search: negamax with α–β window `[alpha, beta]`.
    ///
    /// Returns the best score for the side to move at `pos`. At ply 0 with a full window, this is
    /// the root score. Deeper recursion uses negated returns and negated bounds (see
    /// [`search_single_move`]).
    pub(super) fn negamax(
        &mut self,
        pos: &Chess,
        depth: i32,
        alpha: i32,
        beta: i32,
        ply: usize,
        is_pv: bool,
    ) -> i32 {
        self.stats.nodes += 1;

        if self.stop.load(Ordering::Relaxed) {
            return 0;
        }
        if self.stats.nodes & 0x3FF == 0 {
            if let Some(ref gn) = self.global_nodes {
                gn.fetch_add(1024, Ordering::Relaxed);
            }
            if self.should_stop() {
                return 0;
            }
        }

        if let Some(score) = self.check_early_exits(ply, alpha, beta) {
            return score;
        }

        let hash = get_hash(pos);

        if let Some(tt_score) = self.probe_tt(hash, depth, alpha, beta, is_pv) {
            return tt_score;
        }
        let tt_move = self.get_tt_move(hash);

        if depth <= 0 {
            return self.quiescence(pos, alpha, beta, ply);
        }

        let (depth, in_check) = self.apply_check_extension(depth, pos);

        let ctx = SearchContext {
            depth,
            alpha,
            beta,
            ply,
            is_pv,
            in_check,
            hash,
        };

        if let Some(score) = self.try_null_move_pruning(pos, ctx) {
            return score;
        }

        let legals = pos.legal_moves();
        if legals.is_empty() {
            return self.score_terminal(in_check, ply);
        }

        self.search_moves(pos, &legals, tt_move, ctx)
    }

    fn check_early_exits(&self, ply: usize, alpha: i32, beta: i32) -> Option<i32> {
        let mate_value = MATE_SCORE - ply as i32;

        if alpha >= mate_value {
            return Some(mate_value);
        }
        if beta <= -mate_value {
            return Some(-mate_value);
        }

        None
    }

    fn get_tt_move(&self, hash: u64) -> Option<Move> {
        self.tt.probe(hash).and_then(|e| e.best_move)
    }

    fn probe_tt(
        &mut self,
        hash: u64,
        depth: i32,
        alpha: i32,
        beta: i32,
        is_pv: bool,
    ) -> Option<i32> {
        let entry = self.tt.probe(hash)?;
        self.stats.tt_hits += 1;

        if is_pv || entry.depth < depth as i8 {
            return None;
        }

        match entry.flag {
            TTFlag::Exact => Some(entry.score as i32),
            TTFlag::LowerBound if entry.score as i32 >= beta => {
                self.stats.tt_cutoffs += 1;
                Some(entry.score as i32)
            }
            TTFlag::UpperBound if (entry.score as i32) <= alpha => {
                self.stats.tt_cutoffs += 1;
                Some(entry.score as i32)
            }
            _ => None,
        }
    }

    fn apply_check_extension(&self, depth: i32, pos: &Chess) -> (i32, bool) {
        let in_check = pos.is_check();
        let new_depth = if in_check { depth + 1 } else { depth };
        (new_depth, in_check)
    }

    fn try_null_move_pruning(&mut self, pos: &Chess, ctx: SearchContext) -> Option<i32> {
        if ctx.is_pv || ctx.in_check || ctx.depth < 3 || ctx.ply == 0 {
            return None;
        }

        let has_major_piece = (pos.board().knights()
            | pos.board().bishops()
            | pos.board().rooks()
            | pos.board().queens())
            & pos.us()
            != shakmaty::Bitboard::EMPTY;

        if !has_major_piece {
            return None;
        }

        let null_pos = pos.clone().swap_turn().ok()?;
        let reduction = 3 + ctx.depth / 6;

        let null_ctx = SearchContext {
            depth: ctx.depth - 1 - reduction,
            alpha: -ctx.beta,
            beta: -ctx.beta + 1,
            ply: ctx.ply + 1,
            is_pv: false,
            in_check: false,
            hash: get_hash(&null_pos),
        };

        let null_score = -self.negamax(
            &null_pos,
            null_ctx.depth,
            null_ctx.alpha,
            null_ctx.beta,
            null_ctx.ply,
            null_ctx.is_pv,
        );

        if null_score >= ctx.beta {
            Some(ctx.beta)
        } else {
            None
        }
    }

    fn score_terminal(&self, in_check: bool, ply: usize) -> i32 {
        if in_check {
            -MATE_SCORE + ply as i32
        } else {
            DRAW_SCORE
        }
    }

    fn search_moves(
        &mut self,
        pos: &Chess,
        legals: &shakmaty::MoveList,
        tt_move: Option<Move>,
        mut ctx: SearchContext,
    ) -> i32 {
        let alpha0 = ctx.alpha;
        let ordered = self.order_moves(pos, legals, tt_move.as_ref(), ctx.ply);
        let mut best_score = -super::types::INFINITY;
        let mut best_move: Option<Move> = None;

        for (i, mv) in ordered.iter().enumerate() {
            if self.should_stop() {
                break;
            }

            let mv_ctx = MoveContext {
                mv,
                index: i,
                search: ctx,
            };

            let score = self.search_single_move(pos, mv_ctx);

            if score > best_score {
                best_score = score;
                best_move = Some(mv.clone());
            }

            if score > ctx.alpha {
                ctx.alpha = score;
            }

            if ctx.alpha >= ctx.beta {
                self.update_heuristics_on_cutoff(mv, ctx);
                break;
            }
        }

        self.store_tt_result(ctx.hash, best_move, ctx.depth, best_score, alpha0, ctx.beta);
        best_score
    }

    fn search_single_move(&mut self, pos: &Chess, mv_ctx: MoveContext<'_>) -> i32 {
        let MoveContext { mv, index, search } = mv_ctx;
        let new_pos = pos.clone().play(mv).unwrap();

        let child_ctx = search.descend().next_ply().negate();

        if index == 0 {
            return -self.negamax(
                &new_pos,
                child_ctx.depth,
                child_ctx.alpha,
                child_ctx.beta,
                child_ctx.ply,
                search.is_pv,
            );
        }

        let reduction = self.calculate_lmr_reduction(search.depth, index, mv, search.in_check);
        let reduced_depth = child_ctx.depth - reduction;

        let null_window_alpha = -search.alpha - 1;
        let null_window_beta = -search.alpha;
        let mut score = -self.negamax(
            &new_pos,
            reduced_depth,
            null_window_alpha,
            null_window_beta,
            child_ctx.ply,
            false,
        );

        let needs_research = score > search.alpha && (reduction > 0 || !search.is_pv);
        if needs_research {
            score = -self.negamax(
                &new_pos,
                child_ctx.depth,
                child_ctx.alpha,
                child_ctx.beta,
                child_ctx.ply,
                search.is_pv,
            );
        }

        score
    }

    fn calculate_lmr_reduction(
        &self,
        depth: i32,
        move_index: usize,
        mv: &Move,
        in_check: bool,
    ) -> i32 {
        if depth < 3 || move_index < 4 || mv.is_capture() || mv.is_promotion() || in_check {
            0
        } else {
            1 + (move_index / 8) as i32
        }
    }

    fn update_heuristics_on_cutoff(&mut self, mv: &Move, ctx: SearchContext) {
        if ctx.in_check || mv.is_capture() {
            return;
        }
        self.killers.add(mv.clone(), ctx.ply);
        self.history.add(mv, ctx.depth);
    }

    fn store_tt_result(
        &self,
        hash: u64,
        best_move: Option<Move>,
        depth: i32,
        best_score: i32,
        alpha: i32,
        beta: i32,
    ) {
        let flag = if best_score >= beta {
            TTFlag::LowerBound
        } else if best_score <= alpha {
            TTFlag::UpperBound
        } else {
            TTFlag::Exact
        };

        self.tt
            .store(hash, best_move, depth as i8, best_score as i16, flag);
    }

    /// Quiescence: negamax on tactical moves only (captures / promotions), with standing pat.
    pub(super) fn quiescence(
        &mut self,
        pos: &Chess,
        mut alpha: i32,
        beta: i32,
        _ply: usize,
    ) -> i32 {
        self.stats.qnodes += 1;

        let hash = get_hash(pos);

        if let Some(entry) = self.tt.probe(hash).filter(|e| e.depth >= 0) {
            match entry.flag {
                TTFlag::Exact => return entry.score as i32,
                TTFlag::LowerBound if entry.score as i32 >= beta => return beta,
                TTFlag::UpperBound if entry.score as i32 <= alpha => return alpha,
                _ => {}
            }
        }

        let stand_pat = self.evaluate_position(pos);
        let alpha0 = alpha;
        let mut best_score = stand_pat;
        let mut best_move: Option<Move> = None;

        if stand_pat >= beta {
            self.tt
                .store(hash, None, 0, stand_pat as i16, TTFlag::LowerBound);
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        for mv in pos.legal_moves().iter() {
            if self.should_stop() {
                return best_score;
            }

            if !mv.is_capture() && !mv.is_promotion() {
                continue;
            }
            if stand_pat + 1000 < alpha && !mv.is_promotion() {
                continue;
            }

            let new_pos = pos.clone().play(mv).unwrap();
            let score = -self.quiescence(&new_pos, -beta, -alpha, _ply + 1);

            if score > best_score {
                best_score = score;
                best_move = Some(mv.clone());
            }

            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                best_score = beta;
                break;
            }
        }

        let flag = if best_score >= beta {
            TTFlag::LowerBound
        } else if best_score <= alpha0 {
            TTFlag::UpperBound
        } else {
            TTFlag::Exact
        };
        self.tt.store(
            hash,
            best_move,
            0,
            best_score.clamp(-32768, 32767) as i16,
            flag,
        );

        alpha
    }
}
