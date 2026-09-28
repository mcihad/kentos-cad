//! A quick hash for slots (TODOS.md PERF-08). Slots are small numbers given
//! in turn; the document looks them up for every object a sync, a scene or
//! a run touches, hundreds of thousands at once, where the standard hash's
//! protection against chosen keys only costs time. Fx's multiply spreads
//! them well: its low bits, the table's buckets, are a permutation of the
//! slot's.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

use crate::Slot;

/// A table keyed by slot, with the quick hash.
pub type SlotMap<V> = HashMap<Slot, V, BuildHasherDefault<SlotHasher>>;

#[derive(Default)]
pub struct SlotHasher(u64);

const K: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl Hasher for SlotHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            let mut word = [0u8; 8];
            word.copy_from_slice(chunk);
            self.write_u64(u64::from_le_bytes(word));
        }
        for &b in chunks.remainder() {
            self.write_u64(u64::from(b));
        }
    }

    fn write_u32(&mut self, n: u32) {
        self.write_u64(u64::from(n));
    }

    fn write_u64(&mut self, n: u64) {
        self.0 = (self.0.rotate_left(5) ^ n).wrapping_mul(K);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_given_in_turn_fill_distinct_buckets() {
        // The table's buckets are the hash's low bits: a run of slots must
        // not fall into a few of them.
        let mask = (1u64 << 16) - 1;
        let mut buckets = std::collections::HashSet::new();
        for n in 0..65_536u32 {
            let mut h = SlotHasher::default();
            std::hash::Hash::hash(&Slot(n), &mut h);
            buckets.insert(h.finish() & mask);
        }
        assert_eq!(buckets.len(), 65_536);
    }
}
