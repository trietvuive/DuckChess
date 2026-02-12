//! Opening book: load PGN, index by position hash, probe for a random book move.

use shakmaty::san::SanPlus;
use shakmaty::{
    zobrist::{Zobrist64, ZobristHash},
    CastlingMode, Chess, Position,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

fn position_hash(pos: &Chess) -> u64 {
    let z: Zobrist64 = pos.zobrist_hash(shakmaty::EnPassantMode::Legal);
    z.0
}

/// Opening book: position hash -> list of UCI moves (duplicates = weight).
#[derive(Default)]
pub struct OpeningBook {
    /// For each position hash, one or more UCI move strings. Multiple same move = higher weight.
    entries: HashMap<u64, Vec<String>>,
}

impl OpeningBook {
    /// Load opening book from a PGN file. Each game contributes (position, move) pairs.
    pub fn load_pgn(path: &Path) -> std::io::Result<Self> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        let mut book = OpeningBook::default();
        let mut movetext = String::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !movetext.is_empty() {
                    book.ingest_movetext(&movetext);
                    movetext.clear();
                }
                continue;
            }
            if trimmed.starts_with('[') {
                continue;
            }
            movetext.push(' ');
            movetext.push_str(trimmed);
        }
        if !movetext.is_empty() {
            book.ingest_movetext(&movetext);
        }

        Ok(book)
    }

    /// Parse one game's move text (e.g. "1. Nf3 d5 2. g3 c6 ... 1/2-1/2") and add (position, move) pairs.
    fn ingest_movetext(&mut self, text: &str) {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let mut pos = Chess::default();
        let castling = CastlingMode::Standard;

        for token in tokens {
            if token == "1-0" || token == "0-1" || token == "1/2-1/2" || token == "*" {
                break;
            }
            if token
                .chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
            {
                continue;
            }
            let san: SanPlus = match token.parse() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let mv = match san.san.to_move(&pos) {
                Ok(m) => m,
                Err(_) => break,
            };
            let uci = mv.to_uci(castling).to_string();
            let key = position_hash(&pos);
            self.entries.entry(key).or_default().push(uci);
            pos = match pos.play(&mv) {
                Ok(p) => p,
                Err(_) => break,
            };
        }
    }

    /// Probe the book for the current position. Returns a random legal book move if any.
    pub fn probe(&self, pos: &Chess) -> Option<shakmaty::Move> {
        let key = position_hash(pos);
        let moves = self.entries.get(&key)?;
        if moves.is_empty() {
            return None;
        }
        let uci_move = moves.get(rand::random::<usize>() % moves.len())?;
        let uci: shakmaty::uci::UciMove = uci_move.parse().ok()?;
        let mv = uci.to_move(pos).ok()?;
        if pos.is_legal(&mv) {
            Some(mv)
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
