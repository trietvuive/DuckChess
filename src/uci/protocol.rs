use shakmaty::{fen::Fen, uci::UciMove, CastlingMode, Chess, Position};
use crate::engine::book::OpeningBook;
use crate::engine::nnue::evaluate;
use crate::engine::search::{SearchLimits, Searcher};
use std::io::{self, BufRead, Write};
use std::path::Path;

pub struct UCI {
    pub board: Chess,
    searcher: Searcher,
    /// Number of principal variations to report (UCI MultiPV).
    multi_pv: u32,
    /// Opening book; loaded when BookPath is set.
    book: Option<OpeningBook>,
    /// Use book move when available (UCI OwnBook).
    own_book: bool,
}

impl UCI {
    pub fn new() -> Self {
        UCI {
            board: Chess::default(),
            searcher: Searcher::new(),
            multi_pv: 1,
            book: None,
            own_book: true,
        }
    }

    /// Current MultiPV setting (for tests).
    pub fn multi_pv(&self) -> u32 {
        self.multi_pv
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }

            match parts[0] {
                "uci" => self.cmd_uci(&mut stdout),
                "isready" => writeln!(stdout, "readyok").unwrap(),
                "setoption" => self.cmd_setoption(&parts),
                "ucinewgame" => self.cmd_ucinewgame(),
                "position" => self.cmd_position(&parts),
                "go" => self.cmd_go(&parts, &mut stdout),
                "stop" => self.searcher.stop_flag().store(true, std::sync::atomic::Ordering::Relaxed),
                "quit" => break,
                "d" | "display" => self.cmd_display(&mut stdout),
                "eval" => self.cmd_eval(&mut stdout),
                "perft" => self.cmd_perft(&parts, &mut stdout),
                _ => {}
            }
            stdout.flush().unwrap();
        }
    }

    fn cmd_uci(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, "id name DuckChess 1.0.0").unwrap();
        writeln!(stdout, "id author DuckChess Team").unwrap();
        writeln!(stdout).unwrap();
        writeln!(stdout, "option name Hash type spin default 256 min 1 max 4096").unwrap();
        writeln!(stdout, "option name Threads type spin default 1 min 1 max 1").unwrap();
        writeln!(stdout, "option name MultiPV type spin default 1 min 1 max 5").unwrap();
        writeln!(stdout, "option name BookPath type string default").unwrap();
        writeln!(stdout, "option name OwnBook type check default true").unwrap();
        writeln!(stdout, "uciok").unwrap();
    }

    pub fn cmd_setoption(&mut self, parts: &[&str]) {
        let mut name = String::new();
        let mut value = String::new();
        let mut in_name = false;
        let mut in_value = false;

        for part in parts.iter().skip(1) {
            match *part {
                "name" => { in_name = true; in_value = false; }
                "value" => { in_name = false; in_value = true; }
                _ => {
                    if in_name { if !name.is_empty() { name.push(' '); } name.push_str(part); }
                    else if in_value { if !value.is_empty() { value.push(' '); } value.push_str(part); }
                }
            }
        }

        let opt = name.to_lowercase().replace(' ', "").replace('_', "");
        if opt == "hash" {
            if let Ok(size) = value.parse::<usize>() {
                self.searcher.set_hash_size(size);
            }
        } else if opt == "multipv" {
            if let Ok(n) = value.parse::<u32>() {
                self.multi_pv = n.clamp(1, 5);
            }
        } else if opt == "bookpath" {
            let path = value.trim();
            self.book = if path.is_empty() {
                None
            } else {
                OpeningBook::load_pgn(Path::new(path)).ok()
            };
        } else if opt == "ownbook" {
            self.own_book = value.eq_ignore_ascii_case("true") || value == "1";
        }
    }

    pub fn cmd_ucinewgame(&mut self) {
        self.board = Chess::default();
        self.searcher.clear();
    }

    pub fn cmd_position(&mut self, parts: &[&str]) {
        let mut idx = 1;
        
        if idx < parts.len() && parts[idx] == "startpos" {
            self.board = Chess::default();
            idx += 1;
        } else if idx < parts.len() && parts[idx] == "fen" {
            idx += 1;
            let mut fen_parts = Vec::new();
            while idx < parts.len() && parts[idx] != "moves" {
                fen_parts.push(parts[idx]);
                idx += 1;
            }
            let fen_str = fen_parts.join(" ");
            if let Ok(fen) = fen_str.parse::<Fen>() {
                if let Ok(pos) = fen.into_position::<Chess>(CastlingMode::Standard) {
                    self.board = pos;
                }
            }
        }

        if idx < parts.len() && parts[idx] == "moves" {
            idx += 1;
            while idx < parts.len() {
                if let Some(mv) = self.parse_move(parts[idx]) {
                    self.board = self.board.clone().play(&mv).unwrap();
                }
                idx += 1;
            }
        }
    }

    pub fn parse_move(&self, move_str: &str) -> Option<shakmaty::Move> {
        let uci: UciMove = move_str.parse().ok()?;
        let mv = uci.to_move(&self.board).ok()?;
        if self.board.is_legal(&mv) { Some(mv) } else { None }
    }

    fn cmd_go(&mut self, parts: &[&str], stdout: &mut io::Stdout) {
        let mut limits = SearchLimits::default();
        limits.multi_pv = self.multi_pv;
        let mut i = 1;

        while i < parts.len() {
            match parts[i] {
                "depth" if i + 1 < parts.len() => { limits.depth = parts[i + 1].parse().ok(); i += 2; }
                "nodes" if i + 1 < parts.len() => { limits.nodes = parts[i + 1].parse().ok(); i += 2; }
                "movetime" if i + 1 < parts.len() => { limits.movetime = parts[i + 1].parse().ok(); i += 2; }
                "wtime" if i + 1 < parts.len() => { limits.wtime = parts[i + 1].parse().ok(); i += 2; }
                "btime" if i + 1 < parts.len() => { limits.btime = parts[i + 1].parse().ok(); i += 2; }
                "winc" if i + 1 < parts.len() => { limits.winc = parts[i + 1].parse().ok(); i += 2; }
                "binc" if i + 1 < parts.len() => { limits.binc = parts[i + 1].parse().ok(); i += 2; }
                "movestogo" if i + 1 < parts.len() => { limits.movestogo = parts[i + 1].parse().ok(); i += 2; }
                "multipv" if i + 1 < parts.len() => {
                    if let Ok(n) = parts[i + 1].parse::<u32>() {
                        limits.multi_pv = n.clamp(1, 5);
                    }
                    i += 2;
                }
                "infinite" => { limits.infinite = true; i += 1; }
                _ => { i += 1; }
            }
        }

        if self.own_book {
            if let Some(ref book) = self.book {
                if let Some(mv) = book.probe(&self.board) {
                    writeln!(stdout, "bestmove {}", mv.to_uci(CastlingMode::Standard)).unwrap();
                    return;
                }
            }
        }

        if let Some(mv) = self.searcher.search(&self.board, limits) {
            writeln!(stdout, "bestmove {}", mv.to_uci(CastlingMode::Standard)).unwrap();
        } else {
            writeln!(stdout, "bestmove 0000").unwrap();
        }
    }

    fn cmd_display(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, "\n{:?}", self.board).unwrap();
    }

    fn cmd_eval(&self, stdout: &mut io::Stdout) {
        let score = evaluate(&self.board);
        writeln!(stdout, "Evaluation: {} cp", score).unwrap();
        writeln!(stdout, "(Positive = {} advantage)", 
            if self.board.turn() == shakmaty::Color::White { "White" } else { "Black" }).unwrap();
    }

    fn cmd_perft(&self, parts: &[&str], stdout: &mut io::Stdout) {
        let depth: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
        let start = std::time::Instant::now();
        let nodes = perft(&self.board, depth);
        let elapsed = start.elapsed();
        let nps = if elapsed.as_millis() > 0 { nodes as u128 * 1000 / elapsed.as_millis() } else { 0 };
        writeln!(stdout, "Nodes: {} ({} ms, {} nps)", nodes, elapsed.as_millis(), nps).unwrap();
    }
}

fn perft(pos: &Chess, depth: u32) -> u64 {
    if depth == 0 { return 1; }
    let moves = pos.legal_moves();
    if depth == 1 { return moves.len() as u64; }
    moves.iter().map(|mv| perft(&pos.clone().play(mv).unwrap(), depth - 1)).sum()
}

impl Default for UCI {
    fn default() -> Self { Self::new() }
}
