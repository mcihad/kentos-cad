//! A packed Hilbert R-tree over boxes (the flatbush layout): the boxes sorted
//! along a Hilbert curve, sixteen to a node, built in one pass. It is static;
//! the store rebuilds it after enough edits and scans what changed since.
//! The tree only narrows the candidates: every query applies its exact test
//! afterwards, so its order and its boxes never decide a result.

use crate::geometry::{Bounds, empty_bounds};
use crate::jsmath::{js_max, js_min};

const NODE: usize = 16;

pub struct PackedTree {
    /// Level 0 holds the boxes in Hilbert order; each next level one box per sixteen below.
    levels: Vec<Vec<Bounds>>,
    /// The item behind each level-0 box.
    items: Vec<u32>,
}

fn union(a: Bounds, b: &Bounds) -> Bounds {
    Bounds {
        min_x: js_min(a.min_x, b.min_x),
        min_y: js_min(a.min_y, b.min_y),
        max_x: js_max(a.max_x, b.max_x),
        max_y: js_max(a.max_y, b.max_y),
    }
}

/// Whether two boxes overlap, touching included.
pub fn overlaps(a: &Bounds, b: &Bounds) -> bool {
    a.min_x <= b.max_x && a.max_x >= b.min_x && a.min_y <= b.max_y && a.max_y >= b.min_y
}

impl PackedTree {
    pub fn empty() -> PackedTree {
        PackedTree {
            levels: Vec::new(),
            items: Vec::new(),
        }
    }

    /// A tree over `(item, box)` pairs; the boxes must be finite and not empty.
    pub fn build(entries: &[(u32, Bounds)]) -> PackedTree {
        if entries.is_empty() {
            return PackedTree::empty();
        }
        let ext = entries.iter().fold(empty_bounds(), |e, (_, b)| union(e, b));
        let w = ext.max_x - ext.min_x;
        let h = ext.max_y - ext.min_y;
        // Box centres on a 16-bit grid over the extent, then along the curve.
        let cell = |v: f64, lo: f64, span: f64| -> u32 {
            if span > 0.0 {
                (65535.0 * ((v - lo) / span)).floor().clamp(0.0, 65535.0) as u32
            } else {
                0
            }
        };
        let mut keyed: Vec<(u32, u32)> = entries
            .iter()
            .enumerate()
            .map(|(i, (_, b))| {
                let x = cell((b.min_x + b.max_x) / 2.0, ext.min_x, w);
                let y = cell((b.min_y + b.max_y) / 2.0, ext.min_y, h);
                (hilbert(x, y), i as u32)
            })
            .collect();
        keyed.sort_unstable();
        let items = keyed.iter().map(|&(_, i)| entries[i as usize].0).collect();
        let mut levels = vec![
            keyed
                .iter()
                .map(|&(_, i)| entries[i as usize].1)
                .collect::<Vec<_>>(),
        ];
        while let Some(below) = levels.last().filter(|l| l.len() > 1) {
            let above = below
                .chunks(NODE)
                .map(|c| c.iter().fold(empty_bounds(), union))
                .collect();
            levels.push(above);
        }
        PackedTree { levels, items }
    }

    /// The box around every item (`None` for an empty tree).
    pub fn bounds(&self) -> Option<&Bounds> {
        self.levels.last().and_then(|top| top.first())
    }

    /// Appends the items whose boxes overlap `q` (touching counts), in no particular order.
    pub fn search(&self, q: &Bounds, out: &mut Vec<u32>) {
        let Some(top) = self.levels.len().checked_sub(1) else {
            return;
        };
        let mut stack = vec![(top, 0usize)];
        while let Some((level, i)) = stack.pop() {
            if !overlaps(&self.levels[level][i], q) {
                continue;
            }
            if level == 0 {
                out.push(self.items[i]);
                continue;
            }
            let below = self.levels[level - 1].len();
            let end = ((i + 1) * NODE).min(below);
            for child in i * NODE..end {
                stack.push((level - 1, child));
            }
        }
    }
}

/// Position of (x, y) (16 bits each) along a Hilbert curve: nearby boxes get
/// nearby keys, so a node's sixteen boxes lie close together.
fn hilbert(x: u32, y: u32) -> u32 {
    let mut a = x ^ y;
    let mut b = 0xFFFF ^ a;
    let mut c = 0xFFFF ^ (x | y);
    let mut d = x & (y ^ 0xFFFF);

    let mut aa = a | (b >> 1);
    let mut bb = (a >> 1) ^ a;
    let mut cc = ((c >> 1) ^ (b & (d >> 1))) ^ c;
    let mut dd = ((a & (c >> 1)) ^ (d >> 1)) ^ d;

    a = aa;
    b = bb;
    c = cc;
    d = dd;
    aa = (a & (a >> 2)) ^ (b & (b >> 2));
    bb = (a & (b >> 2)) ^ (b & ((a ^ b) >> 2));
    cc ^= (a & (c >> 2)) ^ (b & (d >> 2));
    dd ^= (b & (c >> 2)) ^ ((a ^ b) & (d >> 2));

    a = aa;
    b = bb;
    c = cc;
    d = dd;
    aa = (a & (a >> 4)) ^ (b & (b >> 4));
    bb = (a & (b >> 4)) ^ (b & ((a ^ b) >> 4));
    cc ^= (a & (c >> 4)) ^ (b & (d >> 4));
    dd ^= (b & (c >> 4)) ^ ((a ^ b) & (d >> 4));

    a = aa;
    b = bb;
    c = cc;
    d = dd;
    cc ^= (a & (c >> 8)) ^ (b & (d >> 8));
    dd ^= (b & (c >> 8)) ^ ((a ^ b) & (d >> 8));

    a = cc ^ (cc >> 1);
    b = dd ^ (dd >> 1);

    let mut i0 = x ^ y;
    let mut i1 = b | (0xFFFF ^ (i0 | a));

    i0 = (i0 | (i0 << 8)) & 0x00FF_00FF;
    i0 = (i0 | (i0 << 4)) & 0x0F0F_0F0F;
    i0 = (i0 | (i0 << 2)) & 0x3333_3333;
    i0 = (i0 | (i0 << 1)) & 0x5555_5555;

    i1 = (i1 | (i1 << 8)) & 0x00FF_00FF;
    i1 = (i1 | (i1 << 4)) & 0x0F0F_0F0F;
    i1 = (i1 | (i1 << 2)) & 0x3333_3333;
    i1 = (i1 | (i1 << 1)) & 0x5555_5555;

    (i1 << 1) | i0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(x0: f64, y0: f64, x1: f64, y1: f64) -> Bounds {
        Bounds {
            min_x: x0,
            min_y: y0,
            max_x: x1,
            max_y: y1,
        }
    }

    #[test]
    fn finds_exactly_the_overlapping_boxes() {
        // A 40 × 40 grid of unit boxes, some tiny, some long.
        let mut entries = Vec::new();
        for i in 0..40u32 {
            for j in 0..40u32 {
                let (x, y) = (f64::from(i) * 2.0, f64::from(j) * 2.0);
                let w = if (i + j) % 7 == 0 { 30.0 } else { 1.0 };
                entries.push((i * 40 + j, b(x, y, x + w, y + 1.0)));
            }
        }
        let tree = PackedTree::build(&entries);
        for q in [
            b(10.0, 10.0, 12.0, 12.0),
            b(-5.0, -5.0, 0.0, 0.0),
            b(79.0, 79.0, 200.0, 200.0),
            b(33.5, 0.0, 33.6, 80.0),
            b(-1.0, -1.0, 100.0, 100.0),
        ] {
            let mut got = Vec::new();
            tree.search(&q, &mut got);
            got.sort_unstable();
            let mut want: Vec<u32> = entries
                .iter()
                .filter(|(_, e)| overlaps(e, &q))
                .map(|(i, _)| *i)
                .collect();
            want.sort_unstable();
            assert_eq!(got, want, "{q:?}");
        }
    }

    #[test]
    fn one_box_and_no_box() {
        let one = PackedTree::build(&[(7, b(0.0, 0.0, 1.0, 1.0))]);
        let mut got = Vec::new();
        one.search(&b(1.0, 1.0, 2.0, 2.0), &mut got);
        assert_eq!(got, [7]);
        PackedTree::empty().search(&b(0.0, 0.0, 1.0, 1.0), &mut got);
        assert_eq!(got, [7]);
    }
}
