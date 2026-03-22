use std::io::{self, BufRead, Write};

use shakmaty::{CastlingMode, Chess};
use vampirc_uci::{parser, UciMessage};

use crate::engine::eval::EvalKind;
use crate::engine::search::{SearchLimits, Searcher};

use super::debug;
use super::limits::{go_to_limits, limits_from_go_tokens, parse_multipv_from_line};
use super::position::{apply_uci_position, apply_uci_position_from_vampirc, parse_uci_move};

pub struct UCI {
    pub board: Chess,
    searcher: Searcher,
}

impl UCI {
    pub fn new() -> Self {
        UCI {
            board: Chess::default(),
            searcher: Searcher::new(),
        }
    }

    /// Current MultiPV setting (UCI option; stored on [`Searcher`]).
    pub fn multi_pv(&self) -> u32 {
        self.searcher.multi_pv()
    }

    /// Active static evaluation mode (UCI option `Eval`).
    pub fn eval_kind(&self) -> EvalKind {
        self.searcher.eval_kind()
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
                UciMessage::Position {
                    startpos,
                    fen,
                    moves,
                } => {
                    apply_uci_position_from_vampirc(&mut self.board, startpos, &fen, &moves);
                }
                UciMessage::Go {
                    time_control,
                    search_control,
                } => {
                    let mut limits = go_to_limits(time_control.as_ref(), search_control.as_ref());
                    limits.multi_pv = self.searcher.multi_pv();
                    if let Some(n) = parse_multipv_from_line(line) {
                        limits.multi_pv = n;
                    }
                    self.do_go(limits, &mut stdout);
                }
                UciMessage::Stop => {
                    self.searcher
                        .stop_flag()
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                }
                UciMessage::Quit => break,
                UciMessage::Unknown(ref s, _) => {
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    if let Some(&first) = parts.first() {
                        match first {
                            "d" | "display" => debug::cmd_display(&self.board, &mut stdout),
                            "eval" => debug::cmd_eval(&self.searcher, &self.board, &mut stdout),
                            "perft" => debug::cmd_perft(&self.board, &parts, &mut stdout),
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
        writeln!(
            stdout,
            "option name Hash type spin default 256 min 1 max 4096"
        )
        .unwrap();
        writeln!(
            stdout,
            "option name Threads type spin default 1 min 1 max 1"
        )
        .unwrap();
        writeln!(
            stdout,
            "option name MultiPV type spin default 1 min 1 max 5"
        )
        .unwrap();
        writeln!(stdout, "option name BookPath type string default").unwrap();
        writeln!(stdout, "option name OwnBook type check default true").unwrap();
        writeln!(
            stdout,
            "option name Eval type combo default Material var Material var NNUE"
        )
        .unwrap();
        writeln!(stdout, "uciok").unwrap();
    }

    pub fn cmd_setoption(&mut self, parts: &[&str]) {
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
                self.searcher.set_multi_pv(n);
            }
        } else if opt == "bookpath" {
            self.searcher.set_book_pgn_path(value);
        } else if opt == "ownbook" {
            self.searcher
                .set_own_book(value.eq_ignore_ascii_case("true") || value == "1");
        } else if opt == "eval" {
            if let Some(k) = EvalKind::from_uci_value(value) {
                self.searcher.set_eval_kind(k);
            }
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
        apply_uci_position(&mut self.board, startpos, fen, move_strs);
    }

    pub fn parse_move(&self, move_str: &str) -> Option<shakmaty::Move> {
        parse_uci_move(&self.board, move_str)
    }

    #[allow(dead_code)] // used by tests
    fn cmd_go(&mut self, parts: &[&str], stdout: &mut io::Stdout) {
        let limits = limits_from_go_tokens(parts, self.searcher.multi_pv());
        self.do_go(limits, stdout);
    }

    /// Run search (book probe when configured happens inside [`Searcher::search`]).
    fn do_go(&mut self, limits: SearchLimits, stdout: &mut io::Stdout) {
        if let Some(mv) = self.searcher.search(&self.board, limits) {
            writeln!(stdout, "bestmove {}", mv.to_uci(CastlingMode::Standard)).unwrap();
        } else {
            writeln!(stdout, "bestmove 0000").unwrap();
        }
    }
}

impl Default for UCI {
    fn default() -> Self {
        Self::new()
    }
}
