//! EPD format opening book reader.

use shakmaty::{CastlingMode, Chess, fen::Fen};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::types::{BookEntries, position_hash};

/// Reader for EPD format opening books.
pub struct EpdReader;

impl EpdReader {
    /// Read EPD file and populate book entries with positions.
    pub fn read(path: &Path, entries: &mut BookEntries) -> std::io::Result<()> {
        let f = File::open(path)?;
        let reader = BufReader::new(f);

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // EPD format: FEN + optional ops after semicolon. We just need the FEN part.
            let fen_part = trimmed.split(';').next().unwrap_or(trimmed).trim();

            // Parse FEN to position
            let fen: Fen = match fen_part.parse() {
                Ok(f) => f,
                Err(_) => continue,
            };
            let pos: Chess = match fen.into_position(CastlingMode::Standard) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Store position with empty move list (EPD is positions only)
            let key = position_hash(&pos);
            entries.entry(key).or_default();
        }

        Ok(())
    }
}
