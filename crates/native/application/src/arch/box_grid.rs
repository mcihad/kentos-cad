//! A uniform grid of boxes for the pairs and the neighbours of many things at once (plan/map-cad.md MAP-04,
//! docs/adr/2940): each box goes in every cell it covers; a pair is reported once, in the cell that holds the lower-left
//! corner of the two boxes' overlap, which both boxes cover. Exact, not approximate: every pair whose boxes meet
//! (touching counts) is reported and none twice; the caller's own test then decides, so the answer is the all-pairs
//! answer. The cell is the side of a square holding one box on average over the boxes' extent, never smaller than the
//! boxes' mean size, so a box covers a few cells and a cell a few boxes.

use std::collections::HashMap;

/// A box: west, south, east, north.
pub type Bx = [f64; 4];

fn meet(a: &Bx, b: &Bx) -> bool {
    a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]
}

pub struct BoxGrid {
    boxes: Vec<Bx>,
    origin: (f64, f64),
    cell: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
}

impl BoxGrid {
    /// The grid of `boxes` (finite; a point is a box of no size).
    pub fn new(boxes: Vec<Bx>) -> Self {
        let n = boxes.len().max(1) as f64;
        let mut ext = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        let mut size = 0.0;
        for b in &boxes {
            ext = [ext[0].min(b[0]), ext[1].min(b[1]), ext[2].max(b[2]), ext[3].max(b[3])];
            size += (b[2] - b[0]).max(b[3] - b[1]);
        }
        let (w, h) = if boxes.is_empty() { (0.0, 0.0) } else { (ext[2] - ext[0], ext[3] - ext[1]) };
        let spread = (w.max(0.0) * h.max(0.0) / n).sqrt().max(w.max(h) / n);
        let cell = spread.max(size / n);
        let cell = if cell.is_finite() && cell > 0.0 { cell } else { 1.0 };
        let origin = if boxes.is_empty() { (0.0, 0.0) } else { (ext[0], ext[1]) };
        let mut grid = BoxGrid { boxes, origin, cell, cells: HashMap::new() };
        for i in 0..grid.boxes.len() {
            let b = grid.boxes[i];
            let (c0, r0) = grid.cell_of(b[0], b[1]);
            let (c1, r1) = grid.cell_of(b[2], b[3]);
            for c in c0..=c1 {
                for r in r0..=r1 {
                    grid.cells.entry((c, r)).or_default().push(i);
                }
            }
        }
        grid
    }

    fn cell_of(&self, x: f64, y: f64) -> (i64, i64) {
        (((x - self.origin.0) / self.cell).floor() as i64, ((y - self.origin.1) / self.cell).floor() as i64)
    }

    /// Every pair (i < j) whose boxes meet, once, in order.
    pub fn pairs(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (&at, members) in &self.cells {
            for (k, &i) in members.iter().enumerate() {
                for &j in &members[k + 1..] {
                    let (a, b) = (&self.boxes[i], &self.boxes[j]);
                    if meet(a, b) && self.cell_of(a[0].max(b[0]), a[1].max(b[1])) == at {
                        out.push(if i < j { (i, j) } else { (j, i) });
                    }
                }
            }
        }
        out.sort_unstable();
        out
    }

    /// The boxes that meet `q`, in order.
    pub fn meeting(&self, q: &Bx) -> Vec<usize> {
        let (c0, r0) = self.cell_of(q[0], q[1]);
        let (c1, r1) = self.cell_of(q[2], q[3]);
        let mut out = Vec::new();
        // A box of the query's size covers few cells; a huge query reads every box once instead.
        if (c1 - c0 + 1).saturating_mul(r1 - r0 + 1) as usize > self.cells.len() {
            out.extend((0..self.boxes.len()).filter(|&i| meet(&self.boxes[i], q)));
            return out;
        }
        for c in c0..=c1 {
            for r in r0..=r1 {
                if let Some(members) = self.cells.get(&(c, r)) {
                    out.extend(members.iter().copied().filter(|&i| meet(&self.boxes[i], q)));
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small deterministic generator (SplitMix64), so the boxes are the same on every run.
    fn boxes(n: usize, seed: u64, long: bool) -> Vec<Bx> {
        let mut s = seed;
        let mut next = || {
            s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
        };
        (0..n)
            .map(|k| {
                let (x, y) = (500_000.0 + next() * 2000.0, 4_400_000.0 + next() * 2000.0);
                let (w, h) = if long && k % 17 == 0 { (next() * 1500.0, next() * 30.0) } else { (next() * 8.0, next() * 8.0) };
                [x, y, x + w, y + h]
            })
            .collect()
    }

    fn all_pairs(b: &[Bx]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for i in 0..b.len() {
            for j in i + 1..b.len() {
                if meet(&b[i], &b[j]) {
                    out.push((i, j));
                }
            }
        }
        out
    }

    #[test]
    fn the_pairs_are_every_meeting_pair_once() {
        for (seed, long) in [(1, false), (2, true), (3, true)] {
            let b = boxes(1500, seed, long);
            let grid = BoxGrid::new(b.clone());
            assert_eq!(grid.pairs(), all_pairs(&b), "seed {seed}");
        }
        // Points on one place, a box of no size, nothing.
        let same = vec![[1.0, 2.0, 1.0, 2.0]; 4];
        assert_eq!(BoxGrid::new(same.clone()).pairs(), all_pairs(&same));
        assert!(BoxGrid::new(Vec::new()).pairs().is_empty());
        // Touching boxes meet.
        let touching = vec![[0.0, 0.0, 1.0, 1.0], [1.0, 1.0, 2.0, 2.0]];
        assert_eq!(BoxGrid::new(touching).pairs(), [(0, 1)]);
    }

    #[test]
    fn meeting_is_the_boxes_a_query_meets() {
        let b = boxes(1500, 7, true);
        let grid = BoxGrid::new(b.clone());
        for q in [[500_400.0, 4_400_400.0, 500_450.0, 4_400_460.0], [499_000.0, 4_399_000.0, 503_000.0, 4_403_000.0]] {
            let want: Vec<usize> = (0..b.len()).filter(|&i| meet(&b[i], &q)).collect();
            assert_eq!(grid.meeting(&q), want);
        }
    }
}
