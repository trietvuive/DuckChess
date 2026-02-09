//! UCI (Universal Chess Interface) Protocol Handler
//!
//! Implements the UCI protocol for communication with chess GUIs.

use crate::core::board::Board;
use crate::engine::movegen::MoveGen;
use crate::core::moves::Move;
use crate::engine::search::{SearchLimits, Searcher};
use std::io::{self, BufRead, Write};
use std::sync::atomic::Ordering;

/// Engine name and author
const ENGINE_NAME: &str = "DuckChess";
const ENGINE_AUTHOR: &str = "DuckChess Team";
const ENGINE_VERSION: &str = "1.0.0";

/// UCI Engine handler
pub struct UCI {
    /// Current board position
    pub board: Board,
    /// Search engine
    searcher: Searcher,
}

impl UCI {
    /// Create a new UCI handler
    pub fn new() -> Self {
        UCI {
            board: Board::startpos(),
            searcher: Searcher::new(),
        }
    }

    /// Main UCI loop
    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "uci" => self.cmd_uci(&mut stdout),
                "debug" => self.cmd_debug(&parts),
                "isready" => self.cmd_isready(&mut stdout),
                "setoption" => self.cmd_setoption(&parts),
                "ucinewgame" => self.cmd_ucinewgame(),
                "position" => self.cmd_position(&parts),
                "go" => self.cmd_go(&parts, &mut stdout),
                "stop" => self.cmd_stop(),
                "ponderhit" => {} // Not implemented
                "quit" => break,
                // Non-standard but useful commands
                "d" | "display" => self.cmd_display(&mut stdout),
                "perft" => self.cmd_perft(&parts, &mut stdout),
                "eval" => self.cmd_eval(&mut stdout),
                _ => {}
            }
        }
    }

    /// Handle 'uci' command
    fn cmd_uci(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, "id name {} {}", ENGINE_NAME, ENGINE_VERSION).unwrap();
        writeln!(stdout, "id author {}", ENGINE_AUTHOR).unwrap();
        writeln!(stdout).unwrap();
        
        // Options
        writeln!(stdout, "option name Hash type spin default 64 min 1 max 4096").unwrap();
        writeln!(stdout, "option name Threads type spin default 1 min 1 max 1").unwrap();
        
        writeln!(stdout, "uciok").unwrap();
        stdout.flush().unwrap();
    }

    /// Handle 'debug' command
    fn cmd_debug(&self, _parts: &[&str]) {
        // Debug mode not implemented
    }

    /// Handle 'isready' command
    fn cmd_isready(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, "readyok").unwrap();
        stdout.flush().unwrap();
    }

    /// Handle 'setoption' command
    pub fn cmd_setoption(&mut self, parts: &[&str]) {
        // Parse: setoption name <name> value <value>
        let mut name = String::new();
        let mut value = String::new();
        let mut in_name = false;
        let mut in_value = false;

        for part in parts.iter().skip(1) {
            match *part {
                "name" => {
                    in_name = true;
                    in_value = false;
                }
                "value" => {
                    in_name = false;
                    in_value = true;
                }
                _ => {
                    if in_name {
                        if !name.is_empty() {
                            name.push(' ');
                        }
                        name.push_str(part);
                    } else if in_value {
                        if !value.is_empty() {
                            value.push(' ');
                        }
                        value.push_str(part);
                    }
                }
            }
        }

        match name.to_lowercase().as_str() {
            "hash" => {
                if let Ok(size) = value.parse::<usize>() {
                    self.searcher.set_hash_size(size);
                }
            }
            _ => {}
        }
    }

    /// Handle 'ucinewgame' command
    pub fn cmd_ucinewgame(&mut self) {
        self.board = Board::startpos();
        self.searcher.clear();
    }

    /// Handle 'position' command
    pub fn cmd_position(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            return;
        }

        let mut idx = 1;

        // Parse position
        if parts[idx] == "startpos" {
            self.board = Board::startpos();
            idx += 1;
        } else if parts[idx] == "fen" {
            // Collect FEN string (up to "moves" or end)
            let mut fen_parts = Vec::new();
            idx += 1;
            while idx < parts.len() && parts[idx] != "moves" {
                fen_parts.push(parts[idx]);
                idx += 1;
            }
            let fen = fen_parts.join(" ");
            match Board::from_fen(&fen) {
                Ok(board) => self.board = board,
                Err(e) => {
                    eprintln!("Invalid FEN: {}", e);
                    return;
                }
            }
        } else {
            return;
        }

        // Parse moves
        if idx < parts.len() && parts[idx] == "moves" {
            idx += 1;
            while idx < parts.len() {
                if let Some(mv) = self.parse_move(parts[idx]) {
                    if !self.board.make_move(mv) {
                        eprintln!("Illegal move: {}", parts[idx]);
                        break;
                    }
                } else {
                    eprintln!("Invalid move format: {}", parts[idx]);
                    break;
                }
                idx += 1;
            }
        }
    }

    /// Parse a move in UCI notation and validate it's legal
    pub fn parse_move(&self, move_str: &str) -> Option<Move> {
        // Generate legal moves and find matching one
        let legal_moves = MoveGen::generate_legal_moves(&self.board);
        
        for mv in legal_moves.iter() {
            if mv.to_uci() == move_str {
                return Some(*mv);
            }
        }

        // Try parsing as basic move if not found (handle promotion capture flag)
        let parsed = Move::from_uci(move_str)?;
        for mv in legal_moves.iter() {
            if mv.from() == parsed.from() && mv.to() == parsed.to() {
                if parsed.is_promotion() {
                    if mv.promotion_piece() == parsed.promotion_piece() {
                        return Some(*mv);
                    }
                } else {
                    return Some(*mv);
                }
            }
        }

        None
    }

    /// Handle 'go' command
    fn cmd_go(&mut self, parts: &[&str], stdout: &mut io::Stdout) {
        let mut limits = SearchLimits::default();

        let mut idx = 1;
        while idx < parts.len() {
            match parts[idx] {
                "searchmoves" => {
                    // Not implemented
                    idx += 1;
                    while idx < parts.len() && !parts[idx].starts_with(char::is_alphabetic) {
                        idx += 1;
                    }
                }
                "ponder" => {
                    idx += 1;
                }
                "wtime" => {
                    if idx + 1 < parts.len() {
                        limits.wtime = parts[idx + 1].parse().ok();
                    }
                    idx += 2;
                }
                "btime" => {
                    if idx + 1 < parts.len() {
                        limits.btime = parts[idx + 1].parse().ok();
                    }
                    idx += 2;
                }
                "winc" => {
                    if idx + 1 < parts.len() {
                        limits.winc = parts[idx + 1].parse().ok();
                    }
                    idx += 2;
                }
                "binc" => {
                    if idx + 1 < parts.len() {
                        limits.binc = parts[idx + 1].parse().ok();
                    }
                    idx += 2;
                }
                "movestogo" => {
                    if idx + 1 < parts.len() {
                        limits.movestogo = parts[idx + 1].parse().ok();
                    }
                    idx += 2;
                }
                "depth" => {
                    if idx + 1 < parts.len() {
                        limits.depth = parts[idx + 1].parse().ok();
                    }
                    idx += 2;
                }
                "nodes" => {
                    if idx + 1 < parts.len() {
                        limits.nodes = parts[idx + 1].parse().ok();
                    }
                    idx += 2;
                }
                "mate" => {
                    // Search for mate in X moves
                    if idx + 1 < parts.len() {
                        if let Ok(mate_in) = parts[idx + 1].parse::<i32>() {
                            limits.depth = Some(mate_in * 2);
                        }
                    }
                    idx += 2;
                }
                "movetime" => {
                    if idx + 1 < parts.len() {
                        limits.movetime = parts[idx + 1].parse().ok();
                    }
                    idx += 2;
                }
                "infinite" => {
                    limits.infinite = true;
                    idx += 1;
                }
                _ => {
                    idx += 1;
                }
            }
        }

        // Default to some depth if nothing specified
        if limits.depth.is_none()
            && limits.nodes.is_none()
            && limits.movetime.is_none()
            && limits.wtime.is_none()
            && limits.btime.is_none()
            && !limits.infinite
        {
            limits.depth = Some(8);
        }

        // Run search
        let best_move = self.searcher.search(&self.board, limits);
        writeln!(stdout, "bestmove {}", best_move.to_uci()).unwrap();
        stdout.flush().unwrap();
    }

    /// Handle 'stop' command
    fn cmd_stop(&mut self) {
        self.searcher.stop.store(true, Ordering::Relaxed);
    }

    /// Handle 'd' (display) command - non-standard but useful
    fn cmd_display(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, "{}", self.board).unwrap();
        stdout.flush().unwrap();
    }

    /// Handle 'perft' command - non-standard but useful for testing
    fn cmd_perft(&self, parts: &[&str], stdout: &mut io::Stdout) {
        let depth = if parts.len() > 1 {
            parts[1].parse().unwrap_or(1)
        } else {
            1
        };

        let start = std::time::Instant::now();
        let nodes = self.perft(&self.board, depth, depth);
        let elapsed = start.elapsed();

        let nps = if elapsed.as_millis() > 0 {
            (nodes as u128 * 1000) / elapsed.as_millis()
        } else {
            0
        };

        writeln!(stdout).unwrap();
        writeln!(stdout, "Nodes: {}", nodes).unwrap();
        writeln!(stdout, "Time: {:?}", elapsed).unwrap();
        writeln!(stdout, "NPS: {}", nps).unwrap();
        stdout.flush().unwrap();
    }

    /// Perft function for testing move generation
    fn perft(&self, board: &Board, depth: u32, root_depth: u32) -> u64 {
        if depth == 0 {
            return 1;
        }

        let moves = MoveGen::generate_legal_moves(board);
        let mut nodes = 0;

        for mv in moves.iter() {
            let mut new_board = board.clone();
            if !new_board.make_move(*mv) {
                continue;
            }

            let count = self.perft(&new_board, depth - 1, root_depth);
            
            if depth == root_depth {
                println!("{}: {}", mv.to_uci(), count);
            }
            
            nodes += count;
        }

        nodes
    }

    /// Handle 'eval' command - non-standard but useful
    fn cmd_eval(&self, stdout: &mut io::Stdout) {
        let score = crate::engine::nnue::evaluate(&self.board);
        writeln!(stdout, "Evaluation: {} cp", score).unwrap();
        writeln!(stdout, "(Positive = White advantage)").unwrap();
        stdout.flush().unwrap();
    }
}

impl Default for UCI {
    fn default() -> Self {
        Self::new()
    }
}
