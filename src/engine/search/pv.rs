//! Principal Variation (PV): the sequence of best moves from root to leaf.
//!
//! The PV represents the engine's predicted best line of play. After search
//! completes, we reconstruct it by walking the transposition table from the
//! root position, following stored best moves until we reach a leaf.

use shakmaty::zobrist::ZobristHash;
use shakmaty::{Chess, Move, Position};
use std::time::Instant;

use crate::engine::tt::TranspositionTable;

use super::types::{MATE_SCORE, MAX_DEPTH, SearchStats};

pub(super) fn get_hash(pos: &Chess) -> u64 {
    let z: shakmaty::zobrist::Zobrist64 = pos.zobrist_hash(shakmaty::EnPassantMode::Legal);
    z.0
}

/// Rebuild a principal variation by chaining **best moves** stored in the TT from `pos`.
///
/// This is a cheap approximation of the PV: we follow each position’s TT `best_move` for at most
/// `max_ply` half-moves (or until the TT has no entry, `play` fails, or the game ends). It does
/// not verify that every hop was a PV node when stored, so lines can be imperfect if entries were
/// replaced or came from cut nodes.
///
/// - **`pos`**: root (or sub-root after a MultiPV first move) to start walking from.
/// - **`max_ply`**: hard cap on PV length; callers usually pass something on the order of search depth.
pub(super) fn get_pv_from_tt(tt: &TranspositionTable, pos: &Chess, max_ply: usize) -> Vec<Move> {
    let mut pv = Vec::with_capacity(max_ply.min(MAX_DEPTH as usize));
    let mut cur = pos.clone();
    for _ in 0..max_ply {
        let entry = match tt.probe(get_hash(&cur)) {
            Some(e) if e.best_move.is_some() => e,
            _ => break, // no TT entry or no best move stored for this position
        };
        let mv = entry.best_move.clone().unwrap();
        pv.push(mv.clone());
        cur = match cur.play(&mv) {
            Ok(p) => p,
            Err(_) => break, // illegal or inconsistent TT move vs board
        };
        if cur.is_game_over() {
            break; // mate, stalemate, or draw — no need to extend PV
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

/// Build UCI info string for a search result.
pub(super) fn format_info(
    tt: &TranspositionTable,
    stats: &SearchStats,
    elapsed_ms: u128,
    depth: i32,
    multipv: u32,
    score: i32,
    pv: &[Move],
) -> String {
    let nps = if elapsed_ms > 0 {
        (stats.nodes as u128 * 1000) / elapsed_ms
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
        format!(
            "info depth {} score {} nodes {} nps {} time {} hashfull {} pv {}",
            depth,
            score_str,
            stats.nodes,
            nps,
            elapsed_ms,
            tt.hashfull(),
            pv_str
        )
    } else {
        format!(
            "info depth {} multipv {} score {} nodes {} nps {} time {} hashfull {} pv {}",
            depth,
            multipv,
            score_str,
            stats.nodes,
            nps,
            elapsed_ms,
            tt.hashfull(),
            pv_str
        )
    }
}

/// Report search info to UCI.
pub(super) fn report_info(
    tt: &TranspositionTable,
    stats: &SearchStats,
    start_time: Instant,
    depth: i32,
    multipv: u32,
    score: i32,
    pv: &[Move],
) {
    let info = format_info(
        tt,
        stats,
        start_time.elapsed().as_millis(),
        depth,
        multipv,
        score,
        pv,
    );
    println!("{}", info);
    // Ensure output is flushed for UCI compliance
    std::io::Write::flush(&mut std::io::stdout()).unwrap();
}
