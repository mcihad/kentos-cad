//! A packed Hilbert R-tree over boxes (the flatbush layout): the boxes sorted
//! along a Hilbert curve, sixteen to a node, built in one pass. It is static;
//! the store rebuilds it after enough edits and scans what changed since.
//! The tree only narrows the candidates: every query applies its exact test
//! afterwards, so its order and its boxes never decide a result.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::geometry::{Bounds, empty_bounds};
use crate::jsmath::{js_max, js_min};

const NODE: usize = 16;

/// A step of a best-first search (`PackedTree::nearest`): a node's box or an
/// item's not yet measured (rank 0), or a measured item (rank 1).
#[derive(Clone, Copy, Debug)]
struct Step {
    /// The least distance a box's contents can have; a measured item's own.
    key: f64,
    rank: u8,
    /// A measured item's tie-breaker; 0 for a box.
    tie: u32,
    level: u32,
    /// A box's place in its level; a measured item's token.
    at: u32,
}

impl Ord for Step {
    /// The heap pops the greatest: here the least key, at equal keys a box
    /// before a measured item, measured items by their tie-breakers.
    fn cmp(&self, o: &Self) -> Ordering {
        o.key
            .total_cmp(&self.key)
            .then(o.rank.cmp(&self.rank))
            .then(o.tie.cmp(&self.tie))
            .then(o.level.cmp(&self.level))
            .then(o.at.cmp(&self.at))
    }
}

impl PartialOrd for Step {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl PartialEq for Step {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}

impl Eq for Step {}

/// The queue of a best-first search, kept between searches for its memory.
#[derive(Default)]
pub struct NearestQueue(BinaryHeap<Step>);

/// What a best-first search (`PackedTree::nearest`) asks of its caller.
pub trait Nearer {
    /// The least distance a box's contents can have; never more than
    /// `measure` gives for an item in the box, so the order is exact.
    fn gap(&self, b: &Bounds) -> f64;
    /// An item's own distance, its tie-breaker and a token for it; none:
    /// the item takes no part.
    fn measure(&mut self, item: u32) -> Option<(f64, u32, u32)>;
    /// A measured item's token, by distance, equal ones by tie-breaker;
    /// false: stop.
    fn take(&mut self, token: u32) -> bool;
}

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

    /// The items nearest something, best first (docs/adr/0215 §4): the
    /// caller's `gap`s order the boxes, its `measure`s the items, and `take`
    /// is given the measured items in order until it says stop or none is
    /// left within `max` (inclusive). A box comes off the queue before any
    /// measured item no nearer than it; equal measures by tie-breaker.
    pub fn nearest(&self, queue: &mut NearestQueue, max: f64, search: &mut impl Nearer) {
        let heap = &mut queue.0;
        heap.clear();
        let Some(top) = self.levels.len().checked_sub(1) else {
            return;
        };
        self.push_boxes(heap, search, max, top, 0..self.levels[top].len());
        while let Some(s) = heap.pop() {
            if s.key > max {
                break;
            }
            if s.rank == 1 {
                if search.take(s.at) {
                    continue;
                }
                break;
            }
            if s.level == 0 {
                if let Some((key, tie, at)) = search.measure(self.items[s.at as usize])
                    && key <= max
                {
                    heap.push(Step {
                        key,
                        rank: 1,
                        tie,
                        level: 0,
                        at,
                    });
                }
                continue;
            }
            let level = s.level as usize - 1;
            let start = s.at as usize * NODE;
            let end = (start + NODE).min(self.levels[level].len());
            self.push_boxes(heap, search, max, level, start..end);
        }
    }

    /// Queues a level's boxes in `range` by their gaps, those within `max`.
    fn push_boxes(
        &self,
        heap: &mut BinaryHeap<Step>,
        search: &impl Nearer,
        max: f64,
        level: usize,
        range: std::ops::Range<usize>,
    ) {
        for at in range {
            let key = search.gap(&self.levels[level][at]);
            if key <= max {
                heap.push(Step {
                    key,
                    rank: 0,
                    tie: 0,
                    level: level as u32,
                    at: at as u32,
                });
            }
        }
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

    /// Measures a box's distance from a point; ties by the item.
    struct FromPoint {
        x: f64,
        y: f64,
        boxes: Vec<Bounds>,
        k: usize,
        got: Vec<(f64, u32)>,
    }

    impl FromPoint {
        fn distance(&self, b: &Bounds) -> f64 {
            let dx = js_max(0.0, js_max(b.min_x - self.x, self.x - b.max_x));
            let dy = js_max(0.0, js_max(b.min_y - self.y, self.y - b.max_y));
            (dx * dx + dy * dy).sqrt()
        }
    }

    impl Nearer for FromPoint {
        fn gap(&self, b: &Bounds) -> f64 {
            self.distance(b)
        }
        fn measure(&mut self, item: u32) -> Option<(f64, u32, u32)> {
            // Every seventh item takes no part.
            (item % 7 != 3).then(|| (self.distance(&self.boxes[item as usize]), item, item))
        }
        fn take(&mut self, token: u32) -> bool {
            let d = self.distance(&self.boxes[token as usize]);
            self.got.push((d, token));
            self.k == 0 || self.got.len() < self.k
        }
    }

    #[test]
    fn the_nearest_come_by_distance_then_tie_breaker() {
        // Boxes on a coarse grid (many equal distances) of varied sizes.
        let mut seed = 12_345u32;
        let mut next = || {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            f64::from((seed >> 16) % 50)
        };
        let boxes: Vec<Bounds> = (0..600)
            .map(|_| {
                let (x, y, w) = (next(), next(), next() / 10.0);
                b(x, y, x + w, y + w / 2.0)
            })
            .collect();
        let entries: Vec<(u32, Bounds)> = boxes
            .iter()
            .enumerate()
            .map(|(i, b)| (i as u32, *b))
            .collect();
        let tree = PackedTree::build(&entries);
        let mut queue = NearestQueue::default();
        for (x, y, k, max) in [
            (25.0, 25.0, 1, f64::INFINITY),
            (0.0, 0.0, 10, f64::INFINITY),
            (60.0, -5.0, 40, f64::INFINITY),
            (12.5, 30.0, 0, 6.0),
            (12.5, 30.0, 0, f64::INFINITY),
        ] {
            let mut search = FromPoint {
                x,
                y,
                boxes: boxes.clone(),
                k,
                got: Vec::new(),
            };
            tree.nearest(&mut queue, max, &mut search);
            let mut want: Vec<(f64, u32)> = (0..boxes.len() as u32)
                .filter(|i| i % 7 != 3)
                .map(|i| (search.distance(&boxes[i as usize]), i))
                .filter(|(d, _)| *d <= max)
                .collect();
            want.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
            if k > 0 {
                want.truncate(k);
            }
            assert_eq!(search.got, want, "({x}, {y}) k {k} max {max}");
        }
        // An empty tree gives nothing.
        let mut none = FromPoint {
            x: 0.0,
            y: 0.0,
            boxes: Vec::new(),
            k: 0,
            got: Vec::new(),
        };
        PackedTree::empty().nearest(&mut queue, f64::INFINITY, &mut none);
        assert!(none.got.is_empty());
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
