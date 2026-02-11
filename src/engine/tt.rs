//! Transposition Table
//!
//! A hash table that stores previously searched positions to avoid
//! redundant work and improve search efficiency.

use shakmaty::Move;

/// Entry type in the transposition table
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum TTFlag {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}

/// A single entry in the transposition table
#[derive(Clone)]
pub struct TTEntry {
    pub key: u64,
    pub best_move: Option<Move>,
    pub depth: i8,
    pub score: i16,
    pub flag: TTFlag,
    pub age: u8,
}

impl TTEntry {
    pub fn empty() -> Self {
        TTEntry {
            key: 0,
            best_move: None,
            depth: 0,
            score: 0,
            flag: TTFlag::Exact,
            age: 0,
        }
    }
}

/// Transposition table
pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    size: usize,
    age: u8,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;
        let size = num_entries.next_power_of_two() / 2;
        TranspositionTable {
            entries: (0..size).map(|_| TTEntry::empty()).collect(),
            size,
            age: 0,
        }
    }

    #[inline]
    fn index(&self, key: u64) -> usize {
        (key as usize) & (self.size - 1)
    }

    pub fn probe(&self, key: u64) -> Option<&TTEntry> {
        let entry = &self.entries[self.index(key)];
        if entry.key == key { Some(entry) } else { None }
    }

    pub fn store(&mut self, key: u64, best_move: Option<Move>, depth: i8, score: i16, flag: TTFlag) {
        let idx = self.index(key);
        let entry = &mut self.entries[idx];
        let should_replace = entry.key == 0 || entry.age != self.age || depth >= entry.depth;
        if should_replace {
            *entry = TTEntry { key, best_move, depth, score, flag, age: self.age };
        }
    }

    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            *entry = TTEntry::empty();
        }
        self.age = 0;
    }

    pub fn new_search(&mut self) {
        self.age = self.age.wrapping_add(1);
    }

    pub fn hashfull(&self) -> usize {
        let sample_size = 1000.min(self.size);
        let used = self.entries[..sample_size].iter().filter(|e| e.key != 0).count();
        (used * 1000) / sample_size
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(256)
    }
}
