//! Opening book: load PGN or EPD, index by position hash, probe for a random book move.

mod epd;
mod pgn;
mod types;

use shakmaty::Position;
use std::ffi::OsStr;
use std::path::Path;

use types::{position_hash, BookEntries};

/// Opening book: position hash -> list of UCI moves (duplicates = weight).
#[derive(Default)]
pub struct OpeningBook {
    /// For each position hash, one or more UCI move strings. Multiple same move = higher weight.
    entries: BookEntries,
}

impl OpeningBook {
    /// Load opening book from a file. Auto-detects PGN or EPD format from extension.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let ext = path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("")
            .to_lowercase();
        if ext == "epd" {
            Self::load_epd(path)
        } else {
            Self::load_pgn(path)
        }
    }

    /// Load opening book from a PGN file. Each game contributes (position, move) pairs.
    pub fn load_pgn(path: &Path) -> std::io::Result<Self> {
        let mut book = Self::default();
        pgn::PgnReader::read(path, &mut book.entries)?;
        Ok(book)
    }

    /// Load opening book from an EPD file. Each line is a position (FEN format).
    /// Stores positions without moves (useful for "is book position" checks).
    pub fn load_epd(path: &Path) -> std::io::Result<Self> {
        let mut book = Self::default();
        epd::EpdReader::read(path, &mut book.entries)?;
        Ok(book)
    }

    /// Check if a position is in the book (has any entry).
    pub fn contains(&self, pos: &shakmaty::Chess) -> bool {
        let key = position_hash(pos);
        self.entries.contains_key(&key)
    }

    /// Probe the book for the current position. Returns a random legal book move if any.
    pub fn probe(&self, pos: &shakmaty::Chess) -> Option<shakmaty::Move> {
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
