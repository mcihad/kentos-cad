//! Points in a regular grid of buckets (docs/adr/0232 §14), about two a
//! bucket: the k nearest to a place found by rings of buckets round it,
//! nearest first by (d², order); the points within a distance; edges by
//! the buckets their boxes reach. The answers are exact: a ring stops only
//! when no point beyond it can come nearer than the k-th.

use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::vec2::Vec2;

/// The squared distance, as every rule here takes it.
#[inline]
pub fn dist2(a: Vec2, b: Vec2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// A grid of buckets over a box.
#[derive(Clone, Debug)]
pub struct Buckets {
    minx: f64,
    miny: f64,
    cell: f64,
    nx: usize,
    ny: usize,
    /// How far rounding may move a bucket's edge (a few ulps of the
    /// coordinates, and a little of a bucket): a ring stops that much later.
    slack: f64,
}

impl Buckets {
    /// About `per` items a bucket for `n` items over `b`.
    fn over(b: [f64; 4], n: usize, per: f64) -> Buckets {
        let [minx, miny, maxx, maxy] = b;
        let (w, h) = (maxx - minx, maxy - miny);
        let buckets = (n as f64 / per).max(1.0);
        let cell = if w > 0.0 && h > 0.0 {
            (w * h / buckets).sqrt()
        } else if w.max(h) > 0.0 {
            w.max(h) / buckets
        } else {
            1.0
        };
        // No more buckets than the items allow (a long thin box).
        let most = (4 * n + 16) as f64;
        let mut cell = cell.max(f64::MIN_POSITIVE);
        while (w / cell + 1.0) * (h / cell + 1.0) > most {
            cell *= 2.0;
        }
        let size = minx.abs().max(miny.abs()).max(maxx.abs()).max(maxy.abs());
        Buckets {
            minx,
            miny,
            cell,
            nx: (w / cell) as usize + 1,
            ny: (h / cell) as usize + 1,
            slack: 16.0 * f64::EPSILON * size + 1e-9 * cell,
        }
    }

    fn col(&self, x: f64) -> usize {
        let i = ((x - self.minx) / self.cell).floor();
        if i < 0.0 || i.is_nan() {
            0
        } else {
            (i as usize).min(self.nx - 1)
        }
    }

    fn row(&self, y: f64) -> usize {
        let j = ((y - self.miny) / self.cell).floor();
        if j < 0.0 || j.is_nan() {
            0
        } else {
            (j as usize).min(self.ny - 1)
        }
    }

    fn slot(&self, i: usize, j: usize) -> usize {
        j * self.nx + i
    }

    /// The least distance from `q` to a place outside the block of buckets
    /// [i0, i1] × [j0, j1] (sides at the grid's edge have nothing beyond);
    /// infinite when the block is the whole grid.
    fn beyond(&self, q: Vec2, i0: usize, i1: usize, j0: usize, j1: usize) -> f64 {
        let mut d = f64::INFINITY;
        if i0 > 0 {
            d = d.min(q.x - (self.minx + i0 as f64 * self.cell));
        }
        if i1 + 1 < self.nx {
            d = d.min(self.minx + (i1 + 1) as f64 * self.cell - q.x);
        }
        if j0 > 0 {
            d = d.min(q.y - (self.miny + j0 as f64 * self.cell));
        }
        if j1 + 1 < self.ny {
            d = d.min(self.miny + (j1 + 1) as f64 * self.cell - q.y);
        }
        d.max(0.0)
    }
}

/// Points by bucket.
#[derive(Clone, Debug)]
pub struct Index {
    b: Buckets,
    starts: Vec<u32>,
    ids: Vec<u32>,
}

impl Index {
    /// The index of `xy` (finite places).
    pub fn new(xy: &[Vec2]) -> Index {
        let bounds = crate::points::bounds_of(xy.iter().copied()).unwrap_or([0.0, 0.0, 0.0, 0.0]);
        let b = Buckets::over(bounds, xy.len(), 2.0);
        let mut count = vec![0u32; b.nx * b.ny + 1];
        let slots: Vec<usize> = xy.iter().map(|p| b.slot(b.col(p.x), b.row(p.y))).collect();
        for &s in &slots {
            count[s + 1] += 1;
        }
        for k in 1..count.len() {
            count[k] += count[k - 1];
        }
        let starts = count.clone();
        let mut ids = vec![0u32; xy.len()];
        for (i, &s) in slots.iter().enumerate() {
            ids[count[s] as usize] = i as u32;
            count[s] += 1;
        }
        Index { b, starts, ids }
    }

    fn bucket(&self, i: usize, j: usize) -> &[u32] {
        let s = self.b.slot(i, j);
        &self.ids[self.starts[s] as usize..self.starts[s + 1] as usize]
    }

    /// The `k` points nearest to `q` into `out`, nearest first by (d², order):
    /// within `radius` when it is above 0, `skip` left out. `out` holds (d², index).
    pub fn nearest(
        &self,
        xy: &[Vec2],
        q: Vec2,
        k: usize,
        radius: f64,
        skip: Option<u32>,
        out: &mut Vec<(f64, u32)>,
    ) {
        out.clear();
        if k == 0 || xy.is_empty() {
            return;
        }
        let r2 = radius * radius;
        let (bx, by) = (self.b.col(q.x), self.b.row(q.y));
        let most = self.b.nx.max(self.b.ny);
        for ring in 0..=most {
            let (i0, i1) = (bx.saturating_sub(ring), (bx + ring).min(self.b.nx - 1));
            let (j0, j1) = (by.saturating_sub(ring), (by + ring).min(self.b.ny - 1));
            for j in j0..=j1 {
                let edge_row = j + ring == by || j == by + ring;
                for i in i0..=i1 {
                    // Only the ring's own buckets: its first and last rows whole, else its two ends.
                    if !edge_row && i + ring != bx && i != bx + ring {
                        continue;
                    }
                    for &id in self.bucket(i, j) {
                        if Some(id) == skip {
                            continue;
                        }
                        let d2 = dist2(xy[id as usize], q);
                        if radius > 0.0 && d2 > r2 {
                            continue;
                        }
                        insert(out, k, (d2, id));
                    }
                }
            }
            let beyond = self.b.beyond(q, i0, i1, j0, j1);
            if beyond.is_infinite() {
                break;
            }
            // Rounding never stops a ring early: the slack first.
            let far = beyond - self.b.slack;
            if far > 0.0 {
                if radius > 0.0 && far > radius {
                    break;
                }
                if out.len() == k && out[k - 1].0 < far * far {
                    break;
                }
            }
        }
    }

    /// Every point within `radius` of `q` (d² ≤ radius²), in no set order.
    pub fn within(&self, xy: &[Vec2], q: Vec2, radius: f64, mut f: impl FnMut(u32, f64)) {
        let r2 = radius * radius;
        let (i0, i1) = (self.b.col(q.x - radius), self.b.col(q.x + radius));
        let (j0, j1) = (self.b.row(q.y - radius), self.b.row(q.y + radius));
        for j in j0..=j1 {
            for i in i0..=i1 {
                for &id in self.bucket(i, j) {
                    let d2 = dist2(xy[id as usize], q);
                    if d2 <= r2 {
                        f(id, d2);
                    }
                }
            }
        }
    }
}

/// `item` into the sorted, at most `k` long `out`.
fn insert(out: &mut Vec<(f64, u32)>, k: usize, item: (f64, u32)) {
    let before = |a: &(f64, u32), b: &(f64, u32)| a.0 < b.0 || (a.0 == b.0 && a.1 < b.1);
    if out.len() == k {
        let last = out[k - 1];
        if !before(&item, &last) {
            return;
        }
        out.pop();
    }
    let mut at = out.len();
    while at > 0 && before(&item, &out[at - 1]) {
        at -= 1;
    }
    out.insert(at, item);
}

/// Edges by the buckets their boxes reach.
#[derive(Clone, Debug)]
pub struct EdgeIndex {
    b: Buckets,
    starts: Vec<u32>,
    ids: Vec<u32>,
}

/// An edge's box.
pub fn edge_box(e: &Edge) -> [f64; 4] {
    match *e {
        Edge::Seg { a, b } => [a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)],
        Edge::Arc { c, r, .. } => [c.x - r, c.y - r, c.x + r, c.y + r],
    }
}

impl EdgeIndex {
    pub fn new(edges: &[Edge]) -> EdgeIndex {
        let boxes: Vec<[f64; 4]> = edges.iter().map(edge_box).collect();
        let bounds = boxes.iter().fold(None, |acc: Option<[f64; 4]>, b| {
            Some(match acc {
                None => *b,
                Some(a) => [
                    a[0].min(b[0]),
                    a[1].min(b[1]),
                    a[2].max(b[2]),
                    a[3].max(b[3]),
                ],
            })
        });
        let b = Buckets::over(bounds.unwrap_or([0.0; 4]), edges.len(), 2.0);
        let mut lists: Vec<Vec<u32>> = vec![Vec::new(); b.nx * b.ny];
        for (k, bx) in boxes.iter().enumerate() {
            for j in b.row(bx[1])..=b.row(bx[3]) {
                for i in b.col(bx[0])..=b.col(bx[2]) {
                    lists[b.slot(i, j)].push(k as u32);
                }
            }
        }
        let mut starts = Vec::with_capacity(lists.len() + 1);
        let mut ids = Vec::new();
        starts.push(0);
        for l in &lists {
            ids.extend_from_slice(l);
            starts.push(ids.len() as u32);
        }
        EdgeIndex { b, starts, ids }
    }

    /// Each edge whose bucket meets the box of the disc round `q` (an edge
    /// may come more than once: `seen` stamps, as long as the edges, with
    /// `stamp` new for each query).
    pub fn near(&self, q: Vec2, radius: f64, seen: &mut [u32], stamp: u32, mut f: impl FnMut(u32)) {
        let (i0, i1) = (self.b.col(q.x - radius), self.b.col(q.x + radius));
        let (j0, j1) = (self.b.row(q.y - radius), self.b.row(q.y + radius));
        for j in j0..=j1 {
            for i in i0..=i1 {
                let s = self.b.slot(i, j);
                for &id in &self.ids[self.starts[s] as usize..self.starts[s + 1] as usize] {
                    if seen[id as usize] != stamp {
                        seen[id as usize] = stamp;
                        f(id);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic random numbers (xorshift64*).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    /// The plain answer: every point, sorted by (d², order).
    fn brute(xy: &[Vec2], q: Vec2, k: usize, radius: f64, skip: Option<u32>) -> Vec<(f64, u32)> {
        let mut all: Vec<(f64, u32)> = xy
            .iter()
            .enumerate()
            .map(|(i, p)| (dist2(*p, q), i as u32))
            .filter(|&(d2, i)| Some(i) != skip && (radius <= 0.0 || d2 <= radius * radius))
            .collect();
        all.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        all.truncate(k);
        all
    }

    #[test]
    fn the_nearest_are_the_plain_answer() {
        let mut g = Rng(31);
        // Clustered and spread points, with ties: a grid and copies.
        let mut xy: Vec<Vec2> = (0..3_000)
            .map(|_| Vec2::new(g.next() * 1_000.0, g.next() * 200.0))
            .collect();
        xy.extend((0..400).map(|k| Vec2::new(f64::from(k % 20) * 3.0, f64::from(k / 20) * 3.0)));
        xy.extend((0..50).map(|_| Vec2::new(500.0, 100.0)));
        let index = Index::new(&xy);
        let mut out = Vec::new();
        for n in 0..3_000 {
            let q = if n % 5 == 0 {
                Vec2::new((n % 20) as f64 * 3.0, ((n / 20) % 20) as f64 * 3.0)
            } else {
                Vec2::new(g.next() * 1_400.0 - 200.0, g.next() * 600.0 - 200.0)
            };
            let k = [1, 3, 12, 64][n % 4];
            let radius = if n % 3 == 0 { 25.0 } else { 0.0 };
            let skip = (n % 7 == 0).then_some((n % xy.len()) as u32);
            index.nearest(&xy, q, k, radius, skip, &mut out);
            assert_eq!(out, brute(&xy, q, k, radius, skip), "query {n} at {q:?}");
        }
    }

    #[test]
    fn within_a_distance() {
        let mut g = Rng(5);
        let xy: Vec<Vec2> = (0..2_000)
            .map(|_| Vec2::new(g.next() * 100.0, g.next() * 100.0))
            .collect();
        let index = Index::new(&xy);
        for _ in 0..500 {
            let q = Vec2::new(g.next() * 120.0 - 10.0, g.next() * 120.0 - 10.0);
            let r = g.next() * 15.0;
            let mut got = Vec::new();
            index.within(&xy, q, r, |id, _| got.push(id));
            got.sort_unstable();
            let want: Vec<u32> = (0..xy.len() as u32)
                .filter(|&i| dist2(xy[i as usize], q) <= r * r)
                .collect();
            assert_eq!(got, want);
        }
    }
}
