//! A radix heap of float64 keys (docs/adr/0235 §3): the priority flood
//! takes its cells in rising height and never pushes below what it last
//! took, so the keys are monotone. Heights become 64-bit keys that keep
//! their order (sign flipped for the positive, every bit for the negative);
//! an entry sits in the bucket of the highest bit its key differs from the
//! last key taken in, and a bucket is only spread again when it is the
//! lowest one left.

/// A float64's key in the same order.
#[inline]
pub fn key_of(v: f64) -> u64 {
    let b = v.to_bits();
    if b >> 63 == 1 { !b } else { b | (1 << 63) }
}

/// The float64 a key came from.
#[inline]
pub fn value_of(k: u64) -> f64 {
    f64::from_bits(if k >> 63 == 1 { k & !(1 << 63) } else { !k })
}

/// Cells by their keys, the least first.
#[derive(Debug)]
pub struct RadixHeap {
    last: u64,
    buckets: Vec<Vec<(u64, u32)>>,
    len: usize,
}

impl Default for RadixHeap {
    fn default() -> RadixHeap {
        RadixHeap::new()
    }
}

impl RadixHeap {
    pub fn new() -> RadixHeap {
        RadixHeap {
            last: 0,
            buckets: (0..65).map(|_| Vec::new()).collect(),
            len: 0,
        }
    }

    #[inline]
    fn bucket(&self, key: u64) -> usize {
        64 - (key ^ self.last).leading_zeros() as usize
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// A cell at `key`; never below the key last taken.
    #[inline]
    pub fn push(&mut self, key: u64, cell: u32) {
        debug_assert!(key >= self.last, "a key below the last taken");
        let b = self.bucket(key);
        self.buckets[b].push((key, cell));
        self.len += 1;
    }

    /// The least key and its cell (of equal keys any).
    pub fn pop(&mut self) -> Option<(u64, u32)> {
        if self.len == 0 {
            return None;
        }
        if self.buckets[0].is_empty() {
            let b = (1..65).find(|&b| !self.buckets[b].is_empty())?;
            let items = std::mem::take(&mut self.buckets[b]);
            self.last = items.iter().map(|e| e.0).min().unwrap_or(self.last);
            for e in &items {
                let to = self.bucket(e.0);
                self.buckets[to].push(*e);
            }
            // The spread bucket's storage comes back for the next time.
            let mut spare = items;
            spare.clear();
            if self.buckets[b].is_empty() {
                self.buckets[b] = spare;
            }
        }
        self.len -= 1;
        self.buckets[0].pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_keep_the_order_and_come_back() {
        let vs = [
            -1e300,
            -2.5,
            -1.0,
            -f64::MIN_POSITIVE,
            0.0,
            1e-300,
            0.5,
            1.0,
            3.25,
            1e300,
        ];
        for w in vs.windows(2) {
            assert!(key_of(w[0]) < key_of(w[1]), "{w:?}");
        }
        for v in vs {
            assert_eq!(value_of(key_of(v)).to_bits(), v.to_bits());
        }
    }

    #[test]
    fn pops_in_rising_order() {
        let mut h = RadixHeap::new();
        let mut taken = Vec::new();
        let vals = [5.0, 3.0, 9.5, 3.0, 7.25, 100.0, 0.5];
        for (k, v) in vals.iter().enumerate() {
            h.push(key_of(*v), k as u32);
        }
        // A push at or above the last taken is allowed between pops.
        while let Some((k, c)) = h.pop() {
            taken.push(value_of(k));
            if c == 1 {
                h.push(key_of(4.0), 99);
            }
        }
        assert_eq!(taken, vec![0.5, 3.0, 3.0, 4.0, 5.0, 7.25, 9.5, 100.0]);
        assert!(h.is_empty());
    }
}
