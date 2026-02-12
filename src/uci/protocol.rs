use shakmaty::{fen::Fen, uci::UciMove, CastlingMode, Chess, Position};
use crate::engine::book::OpeningBook;
use crate::engine::nnue::evaluate;
use crate::engine::search::{SearchLimits, Searcher};
use std::io::{self, BufRead, Write};
use std::path::Path;
use vampirc_uci::{UciMessage, parser};
use vampirc_uci::uci::{UciTimeControl, UciSearchControl};

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
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let msg = parser::parse_one(line);
            match msg {
                UciMessage::Uci => self.cmd_uci(&mut stdout),
                UciMessage::IsReady => writeln!(stdout, "readyok").unwrap(),
                UciMessage::SetOption { name, value } => {
                    self.apply_setoption(name.trim(), value.as_deref());
                }
                UciMessage::UciNewGame => self.cmd_ucinewgame(),
                UciMessage::Position { startpos, fen, moves } => {
                    let fen_str = fen.as_ref().map(|f| f.as_str());
                    let move_strs: Vec<String> = moves.iter().map(|m| m.to_string()).collect();
                    let refs: Vec<&str> = move_strs.iter().map(String::as_str).collect();
                    self.apply_position(startpos, fen_str, &refs);
                }
                UciMessage::Go { time_control, search_control } => {
                    let mut limits = go_to_limits(time_control.as_ref(), search_control.as_ref());
                    limits.multi_pv = self.multi_pv;
                    if let Some(n) = parse_multipv_from_line(line) {
                        limits.multi_pv = n;
                    }
                    self.do_go(limits, &mut stdout);
                }
                UciMessage::Stop => {
                    self.searcher.stop_flag().store(true, std::sync::atomic::Ordering::Relaxed);
                }
                UciMessage::Quit => break,
                UciMessage::Unknown(ref s, _) => {
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    if let Some(&first) = parts.first() {
                        match first {
                            "d" | "display" => self.cmd_display(&mut stdout),
                            "eval" => self.cmd_eval(&mut stdout),
                            "perft" => self.cmd_perft(&parts, &mut stdout),
                            _ => {}
                        }
                    }
                }
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

        self.apply_setoption(&name, Some(&value));
    }

    /// Apply setoption by name and value (used by vampirc path and cmd_setoption).
    fn apply_setoption(&mut self, name: &str, value: Option<&str>) {
        let opt = name.to_lowercase().replace([' ', '_'], "");
        let value = value.unwrap_or("").trim();
        if opt == "hash" {
            if let Ok(size) = value.parse::<usize>() {
                self.searcher.set_hash_size(size);
            }
        } else if opt == "multipv" {
            if let Ok(n) = value.parse::<u32>() {
                self.multi_pv = n.clamp(1, 5);
            }
        } else if opt == "bookpath" {
            self.book = if value.is_empty() {
                None
            } else {
                OpeningBook::load_pgn(Path::new(value)).ok()
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
        let startpos = idx < parts.len() && parts[idx] == "startpos";
        let mut fen_str: Option<String> = None;
        let mut move_strs: Vec<&str> = Vec::new();

        if startpos {
            idx += 1;
        } else if idx < parts.len() && parts[idx] == "fen" {
            idx += 1;
            let mut fen_parts = Vec::new();
            while idx < parts.len() && parts[idx] != "moves" {
                fen_parts.push(parts[idx]);
                idx += 1;
            }
            fen_str = Some(fen_parts.join(" "));
        }

        if idx < parts.len() && parts[idx] == "moves" {
            idx += 1;
            move_strs = parts[idx..].to_vec();
        }

        self.apply_position(startpos, fen_str.as_deref(), &move_strs);
    }

    /// Apply position from parsed UCI (used by vampirc path and cmd_position).
    fn apply_position(&mut self, startpos: bool, fen: Option<&str>, move_strs: &[&str]) {
        if startpos {
            self.board = Chess::default();
        } else if let Some(fen_str) = fen {
            if let Ok(f) = fen_str.parse::<Fen>() {
                if let Ok(pos) = f.into_position::<Chess>(CastlingMode::Standard) {
                    self.board = pos;
                }
            }
        }

        for &s in move_strs {
            if let Some(mv) = self.parse_move(s) {
                self.board = self.board.clone().play(&mv).unwrap();
            }
        }
    }

    pub fn parse_move(&self, move_str: &str) -> Option<shakmaty::Move> {
        let uci: UciMove = move_str.parse().ok()?;
        let mv = uci.to_move(&self.board).ok()?;
        if self.board.is_legal(&mv) { Some(mv) } else { None }
    }

    #[allow(dead_code)] // used by tests
    fn cmd_go(&mut self, parts: &[&str], stdout: &mut io::Stdout) {
        let mut limits = SearchLimits {
            multi_pv: self.multi_pv,
            ..SearchLimits::default()
        };
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

        self.do_go(limits, stdout);
    }

    /// Run search (and optional book probe), output bestmove (used by vampirc path and cmd_go).
    fn do_go(&mut self, limits: SearchLimits, stdout: &mut io::Stdout) {
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

/// Build SearchLimits from vampirc-parsed go command (time_control, search_control).
fn go_to_limits(
    time_control: Option<&UciTimeControl>,
    search_control: Option<&UciSearchControl>,
) -> SearchLimits {
    let mut limits = SearchLimits::default();

    if let Some(sc) = search_control {
        limits.depth = sc.depth.map(i32::from);
        limits.nodes = sc.nodes;
    }

    if let Some(tc) = time_control {
        match tc {
            UciTimeControl::Infinite => limits.infinite = true,
            UciTimeControl::MoveTime(d) => {
                limits.movetime = Some(duration_to_millis(d));
            }
            UciTimeControl::TimeLeft {
                white_time,
                black_time,
                white_increment,
                black_increment,
                moves_to_go,
            } => {
                limits.wtime = white_time.as_ref().map(duration_to_millis);
                limits.btime = black_time.as_ref().map(duration_to_millis);
                limits.winc = white_increment.as_ref().map(duration_to_millis);
                limits.binc = black_increment.as_ref().map(duration_to_millis);
                limits.movestogo = moves_to_go.map(u32::from);
            }
            _ => {}
        }
    }

    limits
}

fn duration_to_millis(d: &vampirc_uci::Duration) -> u64 {
    d.num_milliseconds().max(0) as u64
}

/// If the line contains "multipv N", return Some(N) clamped to 1..=5.
fn parse_multipv_from_line(line: &str) -> Option<u32> {
    let line = line.to_lowercase();
    let mut rest = line.as_str();
    while let Some(idx) = rest.find("multipv") {
        rest = &rest[idx + 7..];
        let rest = rest.trim_start();
        let num: Option<u32> = rest.split_whitespace().next().and_then(|s| s.parse().ok());
        if let Some(n) = num {
            return Some(n.clamp(1, 5));
        }
    }
    None
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
