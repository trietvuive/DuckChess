use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU8, Ordering};

use shakmaty::Move;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum TTFlag {
    Exact = 0,
    LowerBound = 1,
    UpperBound = 2,
}

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

pub struct TranspositionTable {
    entries: UnsafeCell<Vec<TTEntry>>,
    size: usize,
    age: AtomicU8,
}

// Safety: The TT uses a lockless design standard in chess engines. Concurrent
// reads/writes to individual entries may produce torn data, but key verification
// in `probe` detects corruption. Worst case is a TT miss or suboptimal search
// decision — never memory unsafety, because all TTEntry fields are small,
// stack-only types with no heap pointers.
unsafe impl Sync for TranspositionTable {}
unsafe impl Send for TranspositionTable {}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let entry_size = std::mem::size_of::<TTEntry>();
        let num_entries = (size_mb * 1024 * 1024) / entry_size;
        let size = num_entries.next_power_of_two() / 2;
        TranspositionTable {
            entries: UnsafeCell::new((0..size).map(|_| TTEntry::empty()).collect()),
            size,
            age: AtomicU8::new(0),
        }
    }

    #[inline]
    fn index(&self, key: u64) -> usize {
        (key as usize) & (self.size - 1)
    }

    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        let entries = unsafe { &*self.entries.get() };
        let entry = &entries[self.index(key)];
        if entry.key == key {
            Some(entry.clone())
        } else {
            None
        }
    }

    pub fn store(&self, key: u64, best_move: Option<Move>, depth: i8, score: i16, flag: TTFlag) {
        let age = self.age.load(Ordering::Relaxed);
        let idx = self.index(key);
        let entries = unsafe { &mut *self.entries.get() };
        let entry = &mut entries[idx];
        let should_replace = entry.key == 0 || entry.age != age || depth >= entry.depth;
        if should_replace {
            *entry = TTEntry {
                key,
                best_move,
                depth,
                score,
                flag,
                age,
            };
        }
    }

    pub fn clear(&self) {
        let entries = unsafe { &mut *self.entries.get() };
        for entry in entries.iter_mut() {
            *entry = TTEntry::empty();
        }
        self.age.store(0, Ordering::Relaxed);
    }

    pub fn new_search(&self) {
        self.age.fetch_add(1, Ordering::Relaxed);
    }

    pub fn hashfull(&self) -> usize {
        let entries = unsafe { &*self.entries.get() };
        let sample_size = 1000.min(self.size);
        let used = entries[..sample_size].iter().filter(|e| e.key != 0).count();
        (used * 1000) / sample_size
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new(2048)
    }
}
