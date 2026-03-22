use shakmaty::{Chess, Position};
use std::io::Write;

use crate::engine::search::Searcher;

pub(crate) fn perft(pos: &Chess, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = pos.legal_moves();
    if depth == 1 {
        return moves.len() as u64;
    }
    moves
        .iter()
        .map(|mv| perft(&pos.clone().play(mv).unwrap(), depth - 1))
        .sum()
}

pub(crate) fn cmd_display(board: &Chess, stdout: &mut std::io::Stdout) {
    writeln!(stdout, "\n{:?}", board).unwrap();
}

pub(crate) fn cmd_eval(searcher: &Searcher, board: &Chess, stdout: &mut std::io::Stdout) {
    let score = searcher.evaluate_position(board);
    writeln!(stdout, "Evaluation: {} cp", score).unwrap();
    writeln!(
        stdout,
        "(Positive = {} advantage)",
        if board.turn() == shakmaty::Color::White {
            "White"
        } else {
            "Black"
        }
    )
    .unwrap();
}

pub(crate) fn cmd_perft(board: &Chess, parts: &[&str], stdout: &mut std::io::Stdout) {
    let depth: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    let start = std::time::Instant::now();
    let nodes = perft(board, depth);
    let elapsed = start.elapsed();
    let nps = if elapsed.as_millis() > 0 {
        nodes as u128 * 1000 / elapsed.as_millis()
    } else {
        0
    };
    writeln!(
        stdout,
        "Nodes: {} ({} ms, {} nps)",
        nodes,
        elapsed.as_millis(),
        nps
    )
    .unwrap();
}
