//! Shared types and utilities for opening book readers.

use shakmaty::{
    Chess,
    zobrist::{Zobrist64, ZobristHash},
};
use std::collections::HashMap;

/// Position hash for indexing book entries.
pub fn position_hash(pos: &Chess) -> u64 {
    let z: Zobrist64 = pos.zobrist_hash(shakmaty::EnPassantMode::Legal);
    z.0
}

/// Book storage: position hash -> list of UCI moves (duplicates = weight).
pub type BookEntries = HashMap<u64, Vec<String>>;
