use shakmaty::{Chess, Color, Move, MoveList, Position, zobrist::{Zobrist64, ZobristHash}};
use super::nnue::{evaluate, is_insufficient_material};
use super::tt::{TTFlag, TranspositionTable};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn get_hash(pos: &Chess) -> u64 {
    let z: Zobrist64 = pos.zobrist_hash(shakmaty::EnPassantMode::Legal);
    z.0
}

pub const INFINITY: i32 = 30000;
pub const MATE_SCORE: i32 = 29000;
pub const DRAW_SCORE: i32 = 0;
pub const MAX_DEPTH: i32 = 64;

#[derive(Clone, Debug)]
pub struct SearchLimits {
    pub depth: Option<i32>,
    pub nodes: Option<u64>,
    pub movetime: Option<u64>,
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movestogo: Option<u32>,
    pub infinite: bool,
    /// Number of principal variations to report (1 = best line only).
    pub multi_pv: u32,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            depth: None,
            nodes: None,
            movetime: None,
            wtime: None,
            btime: None,
            winc: None,
            binc: None,
            movestogo: None,
            infinite: false,
            multi_pv: 1,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SearchStats {
    pub nodes: u64,
    pub qnodes: u64,
    pub tt_hits: u64,
    pub tt_cutoffs: u64,
}

struct KillerMoves {
    moves: Vec<[Option<Move>; 2]>,
}

impl KillerMoves {
    fn new() -> Self {
        KillerMoves { moves: vec![[None, None]; MAX_DEPTH as usize] }
    }

    fn add(&mut self, mv: Move, ply: usize) {
        if ply < MAX_DEPTH as usize {
            if self.moves[ply][0].as_ref() != Some(&mv) {
                self.moves[ply][1] = self.moves[ply][0].clone();
                self.moves[ply][0] = Some(mv);
            }
        }
    }

    fn is_killer(&self, mv: &Move, ply: usize) -> bool {
        ply < MAX_DEPTH as usize && 
            (self.moves[ply][0].as_ref() == Some(mv) || self.moves[ply][1].as_ref() == Some(mv))
    }
}

struct HistoryTable {
    table: [[i32; 64]; 64],
}

impl HistoryTable {
    fn new() -> Self {
        HistoryTable { table: [[0; 64]; 64] }
    }

    fn add(&mut self, mv: &Move, depth: i32) {
        let from = mv.from().map(|s| s as usize).unwrap_or(0);
        let to = mv.to() as usize;
        self.table[from][to] += depth * depth;
    }

    fn get(&self, mv: &Move) -> i32 {
        let from = mv.from().map(|s| s as usize).unwrap_or(0);
        let to = mv.to() as usize;
        self.table[from][to]
    }

    fn clear(&mut self) {
        self.table = [[0; 64]; 64];
    }
}

pub struct Searcher {
    tt: TranspositionTable,
    killers: KillerMoves,
    history: HistoryTable,
    stats: SearchStats,
    stop: Arc<AtomicBool>,
    start_time: Instant,
    time_limit: Option<Duration>,
    node_limit: Option<u64>,
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

    fn should_stop(&self) -> bool {
        if self.stop.load(Ordering::Relaxed) { return true; }
        if let Some(limit) = self.node_limit {
            if self.stats.nodes >= limit { return true; }
        }
        if let Some(limit) = self.time_limit {
            if self.start_time.elapsed() >= limit { return true; }
        }
        false
    }

    pub fn calculate_time(&self, limits: &SearchLimits, side: Color) -> Option<Duration> {
        if limits.infinite { return None; }
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

    /// Reconstruct the principal variation from the transposition table.
    fn get_pv_from_tt(&self, pos: &Chess, max_plies: usize) -> Vec<Move> {
        let mut pv = Vec::with_capacity(max_plies.min(MAX_DEPTH as usize));
        let mut cur = pos.clone();
        for _ in 0..max_plies {
            let entry = match self.tt.probe(get_hash(&cur)) {
                Some(e) if e.best_move.is_some() => e,
                _ => break,
            };
            let mv = entry.best_move.clone().unwrap();
            pv.push(mv.clone());
            cur = match cur.play(&mv) {
                Ok(p) => p,
                Err(_) => break,
            };
            if cur.is_game_over() { break; }
        }
        pv
    }

    fn format_score(score: i32) -> String {
        if score.abs() >= MATE_SCORE - MAX_DEPTH {
            let mate_in = if score > 0 { (MATE_SCORE - score + 1) / 2 } else { -(MATE_SCORE + score) / 2 };
            format!("mate {}", mate_in)
        } else {
            format!("cp {}", score)
        }
    }

    fn report_info(&self, depth: i32, multipv: u32, score: i32, pv: &[Move]) {
        let elapsed = self.start_time.elapsed();
        let nps = if elapsed.as_millis() > 0 {
            (self.stats.nodes as u128 * 1000) / elapsed.as_millis()
        } else { 0 };
        let score_str = Self::format_score(score);
        let pv_str: String = pv.iter()
            .map(|m| m.to_uci(shakmaty::CastlingMode::Standard).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        if multipv <= 1 {
            println!("info depth {} score {} nodes {} nps {} time {} hashfull {} pv {}",
                depth, score_str, self.stats.nodes, nps, elapsed.as_millis(), self.tt.hashfull(), pv_str);
        } else {
            println!("info depth {} multipv {} score {} nodes {} nps {} time {} hashfull {} pv {}",
                depth, multipv, score_str, self.stats.nodes, nps, elapsed.as_millis(), self.tt.hashfull(), pv_str);
        }
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
            if self.should_stop() { break; }

            if multi_pv <= 1 {
                let (mut alpha, mut beta) = if depth >= 4 {
                    (best_score - 50, best_score + 50)
                } else {
                    (-INFINITY, INFINITY)
                };

                let mut score = best_score;
                loop {
                    let s = self.alpha_beta(pos, depth, alpha, beta, 0, true);
                    if self.should_stop() { break; }
                    if s <= alpha { alpha = -INFINITY; }
                    else if s >= beta { beta = INFINITY; }
                    else { score = s; break; }
                }

                if self.should_stop() { break; }

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
                    if self.should_stop() { break; }
                    let new_pos = pos.clone().play(mv).unwrap();
                    let score = -self.alpha_beta(&new_pos, depth - 1, -INFINITY, INFINITY, 1, true);
                    root_scores.push((mv.clone(), score));
                }

                if self.should_stop() { break; }

                root_scores.sort_by(|a, b| b.1.cmp(&a.1));
                let n_report = (multi_pv as usize).min(root_scores.len());
                best_score = root_scores[0].1;
                best_move = Some(root_scores[0].0.clone());

                for (pv_index, (first_mv, score)) in root_scores.into_iter().take(n_report).enumerate() {
                    let rest = self.get_pv_from_tt(&pos.clone().play(&first_mv).unwrap(), depth as usize);
                    let mut pv = vec![first_mv];
                    pv.extend(rest);
                    self.report_info(depth, (pv_index + 1) as u32, score, &pv);
                }
            }

            if best_score.abs() >= MATE_SCORE - depth { break; }
        }

        if best_move.is_none() {
            let legals = pos.legal_moves();
            if !legals.is_empty() {
                best_move = Some(legals[0].clone());
            }
        }

        best_move
    }

    fn alpha_beta(&mut self, pos: &Chess, mut depth: i32, mut alpha: i32, beta: i32, ply: usize, is_pv: bool) -> i32 {
        if self.should_stop() { return 0; }
        self.stats.nodes += 1;

        let mate_value = MATE_SCORE - ply as i32;
        if alpha >= mate_value { return mate_value; }
        if beta <= -mate_value { return -mate_value; }

        if ply > 0 && is_insufficient_material(pos) { return DRAW_SCORE; }

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

        if depth <= 0 { return self.quiescence(pos, alpha, beta, ply); }

        let in_check = pos.is_check();
        if in_check { depth += 1; }

        if !is_pv && !in_check && depth >= 3 && ply > 0 {
            let dominated = pos.board().knights() | pos.board().bishops() | pos.board().rooks() | pos.board().queens();
            if (dominated & pos.us()).any() {
                if let Ok(null_pos) = pos.clone().swap_turn() {
                    let reduction = 3 + depth / 6;
                    let null_score = -self.alpha_beta(&null_pos, depth - 1 - reduction, -beta, -beta + 1, ply + 1, false);
                    if null_score >= beta { return beta; }
                }
            }
        }

        let legals = pos.legal_moves();
        if legals.is_empty() {
            return if in_check { -MATE_SCORE + ply as i32 } else { DRAW_SCORE };
        }

        let ordered = self.order_moves(pos, &legals, tt_move.as_ref(), ply);
        let mut best_score = -INFINITY;
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
                let mut s = -self.alpha_beta(&new_pos, depth - 1 - reduction, -alpha - 1, -alpha, ply + 1, false);
                if s > alpha && (reduction > 0 || !is_pv) {
                    s = -self.alpha_beta(&new_pos, depth - 1, -beta, -alpha, ply + 1, is_pv);
                }
                s
            };

            if score > best_score {
                best_score = score;
                best_move = Some(mv.clone());
            }

            if score > alpha { alpha = score; }
            if alpha >= beta {
                if !mv.is_capture() {
                    self.killers.add(mv.clone(), ply);
                    self.history.add(mv, depth);
                }
                break;
            }
        }

        let flag = if best_score >= beta { TTFlag::LowerBound }
            else if best_score <= alpha { TTFlag::UpperBound }
            else { TTFlag::Exact };
        self.tt.store(hash, best_move, depth as i8, best_score as i16, flag);

        best_score
    }

    fn quiescence(&mut self, pos: &Chess, mut alpha: i32, beta: i32, _ply: usize) -> i32 {
        self.stats.qnodes += 1;
        let stand_pat = evaluate(pos);

        if stand_pat >= beta { return beta; }
        if stand_pat > alpha { alpha = stand_pat; }

        let legals = pos.legal_moves();
        for mv in legals.iter() {
            if !mv.is_capture() && !mv.is_promotion() { continue; }
            if stand_pat + 1000 < alpha && !mv.is_promotion() { continue; }

            let new_pos = pos.clone().play(mv).unwrap();
            let score = -self.quiescence(&new_pos, -beta, -alpha, _ply + 1);

            if score > alpha { alpha = score; }
            if alpha >= beta { return beta; }
        }

        alpha
    }

    fn order_moves(&self, pos: &Chess, moves: &MoveList, tt_move: Option<&Move>, ply: usize) -> Vec<Move> {
        let mut scored: Vec<(Move, i32)> = moves.iter().map(|mv| {
            let score = if tt_move == Some(mv) { 1_000_000 }
                else if mv.is_capture() { 100_000 + Self::mvv_lva(pos, mv) }
                else if self.killers.is_killer(mv, ply) { 90_000 }
                else { self.history.get(mv) };
            (mv.clone(), score)
        }).collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(mv, _)| mv).collect()
    }

    fn mvv_lva(pos: &Chess, mv: &Move) -> i32 {
        let victim = pos.board().piece_at(mv.to()).map(|p| role_value(p.role)).unwrap_or(0);
        let attacker = role_value(mv.role());
        victim * 10 - attacker
    }
}

fn role_value(role: shakmaty::Role) -> i32 {
    match role {
        shakmaty::Role::Pawn => 100,
        shakmaty::Role::Knight => 320,
        shakmaty::Role::Bishop => 330,
        shakmaty::Role::Rook => 500,
        shakmaty::Role::Queen => 900,
        shakmaty::Role::King => 20000,
    }
}

impl Default for Searcher {
    fn default() -> Self { Self::new() }
}
