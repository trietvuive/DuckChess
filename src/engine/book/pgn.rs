//! PGN format opening book reader.

use shakmaty::san::SanPlus;
use shakmaty::{CastlingMode, Chess, Position};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::types::{BookEntries, position_hash};

/// Reader for PGN format opening books.
pub struct PgnReader;

impl PgnReader {
    /// Read PGN file and populate book entries.
    pub fn read(path: &Path, entries: &mut BookEntries) -> std::io::Result<()> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        let mut movetext = String::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if !movetext.is_empty() {
                    Self::ingest_movetext(&movetext, entries);
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
            Self::ingest_movetext(&movetext, entries);
        }

        Ok(())
    }

    /// Parse one game's move text and add (position, move) pairs.
    fn ingest_movetext(text: &str, entries: &mut BookEntries) {
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
            entries.entry(key).or_default().push(uci);
            pos = match pos.play(&mv) {
                Ok(p) => p,
                Err(_) => break,
            };
        }
    }
}
