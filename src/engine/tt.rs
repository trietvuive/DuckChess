//! Transposition Table
//!
//! A hash table that stores previously searched positions to avoid
//! redundant work and improve search efficiency.

use crate::core::moves::Move;

/// Entry type in the transposition table
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum TTFlag {
    /// Exact score
    Exact = 0,
    /// Lower bound (beta cutoff)
    LowerBound = 1,
    /// Upper bound (failed low)
    UpperBound = 2,
}

/// A single entry in the transposition table
#[derive(Clone, Copy)]
pub struct TTEntry {
    /// Zobrist hash key (for verification)
    pub key: u64,
    /// Best move found
    pub best_move: Move,
    /// Search depth
    pub depth: i8,
    /// Score
    pub score: i16,
    /// Entry type
    pub flag: TTFlag,
    /// Age (for replacement)
    pub age: u8,
}

impl TTEntry {
    pub const EMPTY: TTEntry = TTEntry {
        key: 0,
        best_move: Move::NULL,
        depth: 0,
        score: 0,
        flag: TTFlag::Exact,
        age: 0,
    };
}

/// Transposition table
pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    size: usize,
    age: u8,
}

impl TranspositionTable {
    /// Create a new transposition table with the given size in MB
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;
        // Round down to power of 2 for efficient indexing
        let size = num_entries.next_power_of_two() / 2;
        
        TranspositionTable {
            entries: vec![TTEntry::EMPTY; size],
            size,
            age: 0,
        }
    }

    /// Get the index for a hash key
    #[inline]
    fn index(&self, key: u64) -> usize {
        (key as usize) & (self.size - 1)
    }

    /// Probe the table for an entry
    pub fn probe(&self, key: u64) -> Option<&TTEntry> {
        let entry = &self.entries[self.index(key)];
        if entry.key == key {
            Some(entry)
        } else {
            None
        }
    }

    /// Store an entry in the table
    pub fn store(&mut self, key: u64, best_move: Move, depth: i8, score: i16, flag: TTFlag) {
        let idx = self.index(key);
        let entry = &mut self.entries[idx];

        // Always replace if:
        // - Entry is empty (key == 0)
        // - New entry is from current search and has higher depth
        // - Old entry is from previous search
        let should_replace = entry.key == 0
            || entry.age != self.age
            || (depth >= entry.depth);

        if should_replace {
            *entry = TTEntry {
                key,
                best_move,
                depth,
                score,
                flag,
                age: self.age,
            };
        }
    }

    /// Clear the table
    pub fn clear(&mut self) {
        self.entries.fill(TTEntry::EMPTY);
        self.age = 0;
    }

    /// Increment the age counter (call at the start of each search)
    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    /// Get the fill rate (percentage of entries used)
    pub fn hashfull(&self) -> usize {
        let sample_size = 1000.min(self.size);
        let used = self.entries[..sample_size]
            .iter()
            .filter(|e| e.key != 0)
            .count();
        (used * 1000) / sample_size
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(256) // 256 MB default for deeper searches
    }
}

