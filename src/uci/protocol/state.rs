use std::io::{self, BufRead, Write};
use std::sync::atomic::Ordering;
use std::sync::mpsc;

use shakmaty::{CastlingMode, Chess};
use vampirc_uci::{UciMessage, parser};

use crate::engine::eval::EvalKind;
use crate::engine::search::{SearchLimits, Searcher};

use super::debug;
use super::limits::{
    go_to_limits, limits_from_go_tokens, parse_movetime_from_line, parse_multipv_from_line,
};
use super::position::{apply_uci_position, apply_uci_position_from_vampirc, parse_uci_move};

fn log(msg: &str) {
    use std::io::Write as _;
    let _ = writeln!(std::io::stderr(), "debug {}", msg);
    let _ = std::io::stderr().flush();
}

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
        let (tx, rx) = mpsc::channel::<String>();

        std::thread::spawn(move || {
            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let mut stdout = io::stdout();

        while let Ok(line) = rx.recv() {
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            if self.process_command(&line, &rx, &mut stdout) {
                break;
            }
            stdout.flush().unwrap();
        }
    }

    /// Process a single UCI command. Returns `true` if the engine should quit.
    fn process_command(
        &mut self,
        line: &str,
        rx: &mpsc::Receiver<String>,
        stdout: &mut io::Stdout,
    ) -> bool {
        log(&format!("recv: {}", line));
        let msg = parser::parse_one(line);
        match msg {
            UciMessage::Uci => self.cmd_uci(stdout),
            UciMessage::IsReady => {
                writeln!(stdout, "readyok").unwrap();
                stdout.flush().unwrap();
            }
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
                // Parse movetime from raw line (vampirc-uci drops it when wtime/btime present).
                let raw_movetime_ms = parse_movetime_from_line(line);
                if limits.movetime.is_none() && limits.wtime.is_none() && limits.btime.is_none() {
                    limits.movetime = raw_movetime_ms;
                }
                // When wtime/btime are present, use movetime as a minimum think time
                // so the engine ignores early `stop` from the GUI.
                let min_think_ms = if limits.wtime.is_some() || limits.btime.is_some() {
                    raw_movetime_ms.unwrap_or(0)
                } else {
                    0
                };
                log(&format!(
                    "go: movetime={:?} wtime={:?} btime={:?} min_think={}ms",
                    limits.movetime, limits.wtime, limits.btime, min_think_ms,
                ));
                self.do_go(limits, min_think_ms, rx, stdout);
            }
            UciMessage::Stop => {
                self.searcher.stop_flag().store(true, Ordering::Relaxed);
            }
            UciMessage::Quit => return true,
            UciMessage::Unknown(ref s, _) => {
                let parts: Vec<&str> = s.split_whitespace().collect();
                if let Some(&first) = parts.first() {
                    match first {
                        "d" | "display" => debug::cmd_display(&self.board, stdout),
                        "eval" => debug::cmd_eval(&self.searcher, &self.board, stdout),
                        "perft" => debug::cmd_perft(&self.board, &parts, stdout),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        false
    }

    fn cmd_uci(&self, stdout: &mut io::Stdout) {
        writeln!(stdout, "id name DuckChess 1.0.0").unwrap();
        writeln!(stdout, "id author DuckChess Team").unwrap();
        writeln!(stdout).unwrap();
        writeln!(
            stdout,
            "option name Hash type spin default 2048 min 1 max 4096"
        )
        .unwrap();
        writeln!(
            stdout,
            "option name Threads type spin default 1 min 1 max 256"
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
        match opt.as_str() {
            "hash" => {
                if let Ok(size) = value.parse::<usize>() {
                    self.searcher.set_hash_size(size);
                }
            }
            "multipv" => {
                if let Ok(n) = value.parse::<u32>() {
                    self.searcher.set_multi_pv(n);
                }
            }
            "bookpath" => self.searcher.set_book_pgn_path(value),
            "ownbook" => self
                .searcher
                .set_own_book(value.eq_ignore_ascii_case("true") || value == "1"),
            "threads" => {
                if let Ok(n) = value.parse::<usize>() {
                    self.searcher.set_threads(n);
                }
            }
            "eval" => {
                if let Some(k) = EvalKind::from_uci_value(value) {
                    self.searcher.set_eval_kind(k);
                }
            }
            _ => {}
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
        // Synchronous for tests (no rx channel needed).
        let result = self.searcher.search(&self.board, limits);
        if let Some(mv) = result {
            writeln!(stdout, "bestmove {}", mv.to_uci(CastlingMode::Standard)).unwrap();
        } else {
            writeln!(stdout, "bestmove 0000").unwrap();
        }
    }

    /// Run search while polling `rx` for `stop`/`quit` so the GUI can interrupt.
    /// `min_think_ms`: ignore `stop` until at least this many ms have elapsed
    /// (allows the engine to think longer even when the GUI sends early stops).
    fn do_go(
        &mut self,
        limits: SearchLimits,
        min_think_ms: u64,
        rx: &mpsc::Receiver<String>,
        stdout: &mut io::Stdout,
    ) {
        let stop = self.searcher.stop_flag();
        let board = self.board.clone();
        let searcher = &mut self.searcher;
        let search_start = std::time::Instant::now();
        let min_think = std::time::Duration::from_millis(min_think_ms);

        let result = std::thread::scope(|s| {
            let search_handle = s.spawn(|| searcher.search(&board, limits));

            while !search_handle.is_finished() {
                match rx.recv_timeout(std::time::Duration::from_millis(5)) {
                    Ok(line) => {
                        let cmd = line.trim();
                        if cmd == "quit" {
                            log("recv: quit (during search)");
                            stop.store(true, Ordering::Relaxed);
                            break;
                        }
                        if cmd == "stop" {
                            if search_start.elapsed() >= min_think {
                                log(&format!(
                                    "recv: stop (during search, {}ms elapsed, honoring)",
                                    search_start.elapsed().as_millis()
                                ));
                                stop.store(true, Ordering::Relaxed);
                                break;
                            } else {
                                log(&format!(
                                    "recv: stop (during search, {}ms elapsed, ignoring — min {}ms)",
                                    search_start.elapsed().as_millis(),
                                    min_think_ms
                                ));
                            }
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }

            search_handle.join().unwrap()
        });

        if let Some(mv) = result {
            let uci_mv = mv.to_uci(CastlingMode::Standard);
            log(&format!("bestmove {}", uci_mv));
            writeln!(stdout, "bestmove {}", uci_mv).unwrap();
        } else {
            log("bestmove 0000");
            writeln!(stdout, "bestmove 0000").unwrap();
        }
        stdout.flush().unwrap();
    }
}

impl Default for UCI {
    fn default() -> Self {
        Self::new()
    }
}
