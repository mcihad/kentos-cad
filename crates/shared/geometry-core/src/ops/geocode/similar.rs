//! Names' similarity (docs/adr/0229 §5): `⌊100 · (L − d) / L⌋`, `d` the
//! Levenshtein distance of the two keys' characters and `L` the longer one's
//! count; a numbered name (all its words numbered) is 100 to itself and 0 to
//! anything else. A name is similar from 80. [`Names`] finds a key's similar
//! names among many: the exact one by its key, the others among the names
//! whose lengths can reach 80 (`4L/5` to `5L/4`), each measured with a
//! bounded distance that stops past `⌊L/5⌋`; the result is the one a
//! comparison with every name gives.

use std::collections::HashMap;

use super::fold::numbered_word;

/// The similarity from which a name is taken.
pub const SIMILAR: u32 = 80;

/// Whether a name is numbered: every word of it starts with an ASCII digit.
pub fn numbered(name: &str) -> bool {
    !name.is_empty() && name.split(' ').all(numbered_word)
}

/// The similarity of two keys, 0 to 100.
pub fn similarity(a: &str, b: &str) -> u32 {
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    if numbered(a) || numbered(b) {
        return if a == b { 100 } else { 0 };
    }
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let l = a.len().max(b.len());
    let d = levenshtein(&a, &b, l);
    ((100 * (l - d)) / l) as u32
}

/// The Levenshtein distance of `a` and `b`, or a number past `max` when it is past it.
pub fn levenshtein(a: &[char], b: &[char], max: usize) -> usize {
    let (a, b) = if a.len() < b.len() { (b, a) } else { (a, b) };
    if a.len() - b.len() > max {
        return max + 1;
    }
    if b.is_empty() {
        return a.len();
    }
    // The row on the stack for the names addresses have; a longer one on the heap.
    let mut stack = [0usize; 64];
    let mut heap: Vec<usize> = Vec::new();
    let row: &mut [usize] = if b.len() < stack.len() {
        &mut stack[..=b.len()]
    } else {
        heap.resize(b.len() + 1, 0);
        &mut heap
    };
    for (j, r) in row.iter_mut().enumerate() {
        *r = j;
    }
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        let mut least = row[0];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let next = (prev + cost).min(row[j] + 1).min(row[j + 1] + 1);
            prev = row[j + 1];
            row[j + 1] = next;
            least = least.min(next);
        }
        if least > max {
            return max + 1;
        }
    }
    row[b.len()]
}

/// The set of a name's letters, each in one of 64 bits (by its code point): an edit adds at most one bit and takes
/// at most one away, so two names `d` edits apart differ in at most `2d` bits.
fn letters(chars: &[char]) -> u64 {
    chars
        .iter()
        .fold(0u64, |m, &c| m | 1u64 << (u32::from(c) % 64))
}

/// A key's characters: on the stack for the names addresses have, else on the heap.
fn chars_of<'b>(s: &str, stack: &'b mut [char; 64], heap: &'b mut Vec<char>) -> &'b [char] {
    let mut n = 0;
    for c in s.chars() {
        if n == stack.len() {
            *heap = s.chars().collect();
            return heap;
        }
        stack[n] = c;
        n += 1;
    }
    &stack[..n]
}

/// The similarity of two keys when it is at least [`SIMILAR`]; none otherwise.
pub fn similar(a: &str, b: &str) -> Option<u32> {
    if a.is_empty() || b.is_empty() {
        return None;
    }
    if numbered(a) || numbered(b) {
        return (a == b).then_some(100);
    }
    if a == b {
        return Some(100);
    }
    let (mut sa, mut sb) = (['\0'; 64], ['\0'; 64]);
    let (mut ha, mut hb) = (Vec::new(), Vec::new());
    let ac = chars_of(a, &mut sa, &mut ha);
    let bc = chars_of(b, &mut sb, &mut hb);
    let l = ac.len().max(bc.len());
    // B ≥ 80 ⇔ d ≤ L / 5.
    let d = levenshtein(ac, bc, l / 5);
    (d <= l / 5).then(|| ((100 * (l - d)) / l) as u32)
}

/// Many names, found by a key's similarity.
#[derive(Debug, Default)]
pub struct Names {
    /// A name's key → its place in `names`.
    exact: HashMap<String, usize>,
    /// Each name, its characters, whether it is numbered (§5: equal or nothing) and its letters' mask.
    names: Vec<(String, Vec<char>, bool, u64)>,
    /// The names of each character count.
    by_length: HashMap<usize, Vec<usize>>,
}

impl Names {
    /// Adds a name once; its place.
    pub fn add(&mut self, name: &str) -> usize {
        if let Some(&i) = self.exact.get(name) {
            return i;
        }
        let i = self.names.len();
        let chars: Vec<char> = name.chars().collect();
        self.by_length.entry(chars.len()).or_default().push(i);
        let mask = letters(&chars);
        self.names
            .push((name.to_owned(), chars, numbered(name), mask));
        self.exact.insert(name.to_owned(), i);
        i
    }

    pub fn name(&self, i: usize) -> &str {
        &self.names[i].0
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The place of a name, if it is one.
    pub fn exact(&self, key: &str) -> Option<usize> {
        self.exact.get(key).copied()
    }

    /// The names similar to `key` with their similarities, in the order they were added.
    pub fn similar(&self, key: &str) -> Vec<(usize, u32)> {
        if key.is_empty() {
            return Vec::new();
        }
        if numbered(key) {
            return self.exact(key).map(|i| vec![(i, 100)]).unwrap_or_default();
        }
        let q: Vec<char> = key.chars().collect();
        let lq = q.len();
        let mask = letters(&q);
        // Lengths that can reach 80: L / 5 ≥ |la − lb| with L the longer.
        let low = (4 * lq).div_ceil(5);
        let high = (5 * lq) / 4;
        let mut out: Vec<(usize, u32)> = Vec::new();
        for len in low..=high {
            let Some(list) = self.by_length.get(&len) else {
                continue;
            };
            for &i in list {
                let (_, chars, numbered, letters) = &self.names[i];
                if *numbered {
                    continue;
                }
                let l = lq.max(chars.len());
                // An edit changes the letters' set by at most two: a name whose set differs by more than twice the
                // distance allowed cannot be similar (exact, never a name similar left out).
                if (mask ^ letters).count_ones() as usize > 2 * (l / 5) {
                    continue;
                }
                let d = levenshtein(&q, chars, l / 5);
                if d <= l / 5 {
                    out.push((i, ((100 * (l - d)) / l) as u32));
                }
            }
        }
        out.sort_unstable_by_key(|&(i, _)| i);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similarity_is_the_share_of_the_longer_name_kept() {
        assert_eq!(similarity("ataturk", "ataturk"), 100);
        assert_eq!(similarity("ataturk", "atatrk"), 85);
        assert_eq!(similarity("kizilay", "kizlay"), 85);
        assert_eq!(similarity("ataturk", "inonu"), 14);
        assert_eq!(similarity("1203/1", "1203/2"), 0);
        assert_eq!(similarity("15 temmuz", "15 temmz"), 88);
        assert_eq!(similar("ataturk", "atatrk"), Some(85));
        assert_eq!(similar("ataturk", "atkturk x"), None);
    }

    #[test]
    fn names_find_what_a_comparison_with_each_finds() {
        let mut names = Names::default();
        for n in [
            "ataturk",
            "ataturk bulvari",
            "inonu",
            "1203/1",
            "atatur",
            "kizilay",
            "gazi",
        ] {
            names.add(n);
        }
        for key in [
            "ataturk",
            "atatrk",
            "kizlay",
            "1203/1",
            "gaz",
            "inonu caddesi",
        ] {
            let fast: Vec<(usize, u32)> = names.similar(key);
            let slow: Vec<(usize, u32)> = (0..names.len())
                .filter_map(|i| similar(key, names.name(i)).map(|s| (i, s)))
                .collect();
            assert_eq!(fast, slow, "{key}");
        }
    }
}
