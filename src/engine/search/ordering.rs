//! Move ordering: killer moves, history heuristic, MVV-LVA.

use shakmaty::{Chess, Move, MoveList, Position};

use super::types::MAX_DEPTH;

pub(super) struct KillerMoves {
    pub(super) moves: Vec<[Option<Move>; 2]>,
}

impl KillerMoves {
    pub(super) fn new() -> Self {
        KillerMoves {
            moves: vec![[None, None]; MAX_DEPTH as usize],
        }
    }

    pub(super) fn add(&mut self, mv: Move, ply: usize) {
        if ply < MAX_DEPTH as usize && self.moves[ply][0].as_ref() != Some(&mv) {
            self.moves[ply][1] = self.moves[ply][0].clone();
            self.moves[ply][0] = Some(mv);
        }
    }

    pub(super) fn is_killer(&self, mv: &Move, ply: usize) -> bool {
        ply < MAX_DEPTH as usize
            && (self.moves[ply][0].as_ref() == Some(mv) || self.moves[ply][1].as_ref() == Some(mv))
    }
}

pub(super) struct HistoryTable {
    table: [[i32; 64]; 64],
}

impl HistoryTable {
    pub(super) fn new() -> Self {
        HistoryTable {
            table: [[0; 64]; 64],
        }
    }

    pub(super) fn add(&mut self, mv: &Move, depth: i32) {
        let from = mv.from().map(|s| s as usize).unwrap_or(0);
        let to = mv.to() as usize;
        self.table[from][to] += depth * depth;
    }

    pub(super) fn get(&self, mv: &Move) -> i32 {
        let from = mv.from().map(|s| s as usize).unwrap_or(0);
        let to = mv.to() as usize;
        self.table[from][to]
    }

    pub(super) fn clear(&mut self) {
        self.table = [[0; 64]; 64];
    }
}

pub(super) fn order_moves(
    pos: &Chess,
    moves: &MoveList,
    tt_move: Option<&Move>,
    ply: usize,
    killers: &KillerMoves,
    history: &HistoryTable,
) -> Vec<Move> {
    let mut scored: Vec<(Move, i32)> = moves
        .iter()
        .map(|mv| {
            let score = if tt_move == Some(mv) {
                1_000_000
            } else if mv.is_capture() {
                100_000 + mvv_lva(pos, mv)
            } else if killers.is_killer(mv, ply) {
                90_000
            } else {
                history.get(mv)
            };
            (mv.clone(), score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().map(|(mv, _)| mv).collect()
}

pub(super) fn mvv_lva(pos: &Chess, mv: &Move) -> i32 {
    let victim = pos
        .board()
        .piece_at(mv.to())
        .map(|p| role_value(p.role))
        .unwrap_or(0);
    let attacker = role_value(mv.role());
    victim * 10 - attacker
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
