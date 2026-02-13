//! Principal variation reconstruction and UCI info reporting.

use shakmaty::zobrist::ZobristHash;
use shakmaty::{Chess, Move, Position};
use std::time::Instant;

use crate::engine::tt::TranspositionTable;

use super::types::{SearchStats, MATE_SCORE, MAX_DEPTH};

pub(super) fn get_hash(pos: &Chess) -> u64 {
    let z: shakmaty::zobrist::Zobrist64 = pos.zobrist_hash(shakmaty::EnPassantMode::Legal);
    z.0
}

pub(super) fn get_pv_from_tt(tt: &TranspositionTable, pos: &Chess, max_plies: usize) -> Vec<Move> {
    let mut pv = Vec::with_capacity(max_plies.min(MAX_DEPTH as usize));
    let mut cur = pos.clone();
    for _ in 0..max_plies {
        let entry = match tt.probe(get_hash(&cur)) {
            Some(e) if e.best_move.is_some() => e,
            _ => break,
        };
        let mv = entry.best_move.clone().unwrap();
        pv.push(mv.clone());
        cur = match cur.play(&mv) {
            Ok(p) => p,
            Err(_) => break,
        };
        if cur.is_game_over() {
            break;
        }
    }
    pv
}

pub(super) fn format_score(score: i32) -> String {
    if score.abs() >= MATE_SCORE - MAX_DEPTH {
        let mate_in = if score > 0 {
            (MATE_SCORE - score + 1) / 2
        } else {
            -(MATE_SCORE + score) / 2
        };
        format!("mate {}", mate_in)
    } else {
        format!("cp {}", score)
    }
}

pub(super) fn report_info(
    tt: &TranspositionTable,
    stats: &SearchStats,
    start_time: Instant,
    depth: i32,
    multipv: u32,
    score: i32,
    pv: &[Move],
) {
    let elapsed = start_time.elapsed();
    let nps = if elapsed.as_millis() > 0 {
        (stats.nodes as u128 * 1000) / elapsed.as_millis()
    } else {
        0
    };
    let score_str = format_score(score);
    let pv_str: String = pv
        .iter()
        .map(|m| m.to_uci(shakmaty::CastlingMode::Standard).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    if multipv <= 1 {
        println!(
            "info depth {} score {} nodes {} nps {} time {} hashfull {} pv {}",
            depth,
            score_str,
            stats.nodes,
            nps,
            elapsed.as_millis(),
            tt.hashfull(),
            pv_str
        );
    } else {
        println!(
            "info depth {} multipv {} score {} nodes {} nps {} time {} hashfull {} pv {}",
            depth,
            multipv,
            score_str,
            stats.nodes,
            nps,
            elapsed.as_millis(),
            tt.hashfull(),
            pv_str
        );
    }
}
