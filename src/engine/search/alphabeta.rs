//! Alpha-beta search and quiescence.

use shakmaty::{Chess, Move, Position};

use crate::engine::tt::TTFlag;

use crate::engine::eval::{evaluate, is_insufficient_material};
use super::pv::get_hash;
use super::searcher::Searcher;
use super::types::{DRAW_SCORE, MATE_SCORE};

impl Searcher {
    pub(super) fn alpha_beta(
        &mut self,
        pos: &Chess,
        mut depth: i32,
        mut alpha: i32,
        beta: i32,
        ply: usize,
        is_pv: bool,
    ) -> i32 {
        if self.should_stop() {
            return 0;
        }
        self.stats.nodes += 1;

        let mate_value = MATE_SCORE - ply as i32;
        if alpha >= mate_value {
            return mate_value;
        }
        if beta <= -mate_value {
            return -mate_value;
        }

        if ply > 0 && is_insufficient_material(pos) {
            return DRAW_SCORE;
        }

        let hash = get_hash(pos);
        let mut tt_move: Option<Move> = None;

        if let Some(entry) = self.tt.probe(hash) {
            self.stats.tt_hits += 1;
            tt_move = entry.best_move.clone();
            if !is_pv && entry.depth >= depth as i8 {
                match entry.flag {
                    TTFlag::Exact => return entry.score as i32,
                    TTFlag::LowerBound if entry.score as i32 >= beta => {
                        self.stats.tt_cutoffs += 1;
                        return entry.score as i32;
                    }
                    TTFlag::UpperBound if (entry.score as i32) <= alpha => {
                        self.stats.tt_cutoffs += 1;
                        return entry.score as i32;
                    }
                    _ => {}
                }
            }
        }

        if depth <= 0 {
            return self.quiescence(pos, alpha, beta, ply);
        }

        let in_check = pos.is_check();
        if in_check {
            depth += 1;
        }

        if !is_pv && !in_check && depth >= 3 && ply > 0 {
            let dominated = pos.board().knights()
                | pos.board().bishops()
                | pos.board().rooks()
                | pos.board().queens();
            if (dominated & pos.us()).any() {
                if let Ok(null_pos) = pos.clone().swap_turn() {
                    let reduction = 3 + depth / 6;
                    let null_score = -self.alpha_beta(
                        &null_pos,
                        depth - 1 - reduction,
                        -beta,
                        -beta + 1,
                        ply + 1,
                        false,
                    );
                    if null_score >= beta {
                        return beta;
                    }
                }
            }
        }

        let legals = pos.legal_moves();
        if legals.is_empty() {
            return if in_check {
                -MATE_SCORE + ply as i32
            } else {
                DRAW_SCORE
            };
        }

        let ordered = self.order_moves(pos, &legals, tt_move.as_ref(), ply);
        let mut best_score = -super::types::INFINITY;
        let mut best_move: Option<Move> = None;

        for (i, mv) in ordered.iter().enumerate() {
            let new_pos = pos.clone().play(mv).unwrap();

            let score = if i == 0 {
                -self.alpha_beta(&new_pos, depth - 1, -beta, -alpha, ply + 1, is_pv)
            } else {
                let mut reduction = 0;
                if depth >= 3 && i >= 4 && !mv.is_capture() && !mv.is_promotion() && !in_check {
                    reduction = 1 + (i / 8) as i32;
                }
                let mut s = -self.alpha_beta(
                    &new_pos,
                    depth - 1 - reduction,
                    -alpha - 1,
                    -alpha,
                    ply + 1,
                    false,
                );
                if s > alpha && (reduction > 0 || !is_pv) {
                    s = -self.alpha_beta(&new_pos, depth - 1, -beta, -alpha, ply + 1, is_pv);
                }
                s
            };

            if score > best_score {
                best_score = score;
                best_move = Some(mv.clone());
            }

            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                if !mv.is_capture() {
                    self.killers.add(mv.clone(), ply);
                    self.history.add(mv, depth);
                }
                break;
            }
        }

        let flag = if best_score >= beta {
            TTFlag::LowerBound
        } else if best_score <= alpha {
            TTFlag::UpperBound
        } else {
            TTFlag::Exact
        };
        self.tt
            .store(hash, best_move, depth as i8, best_score as i16, flag);

        best_score
    }

    pub(super) fn quiescence(&mut self, pos: &Chess, mut alpha: i32, beta: i32, _ply: usize) -> i32 {
        self.stats.qnodes += 1;
        let stand_pat = evaluate(pos);

        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let legals = pos.legal_moves();
        for mv in legals.iter() {
            if !mv.is_capture() && !mv.is_promotion() {
                continue;
            }
            if stand_pat + 1000 < alpha && !mv.is_promotion() {
                continue;
            }

            let new_pos = pos.clone().play(mv).unwrap();
            let score = -self.quiescence(&new_pos, -beta, -alpha, _ply + 1);

            if score > alpha {
                alpha = score;
            }
            if alpha >= beta {
                return beta;
            }
        }

        alpha
    }
}
