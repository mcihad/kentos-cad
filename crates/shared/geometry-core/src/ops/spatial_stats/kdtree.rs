//! A balanced k-d tree over places (docs/adr/0238 §12): the k nearest to a
//! place by (d², order) and the places within a distance, both exact. An
//! outlier far from the rest does not spoil it as it spoils a grid of
//! buckets (one bucket would hold nearly everything).
//!
//! A node is the median of its range of `order`, split on the axis the range
//! spreads most along; the left range holds places before it by (coordinate,
//! order), the right range those after. The pruning is exact in floating
//! point: a far-side place is never nearer on the axis than the node, and its
//! squared distance is never smaller than the axis's square.

use crate::vec2::Vec2;

/// The squared distance, as every rule of the module takes it.
#[inline]
pub fn dist2(a: Vec2, b: Vec2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

pub struct KdTree<'p> {
    pts: &'p [Vec2],
    order: Vec<u32>,
    /// The split axis of the node at each position of `order` (0 x, 1 y).
    axis: Vec<u8>,
}

#[inline]
fn coord(p: Vec2, axis: u8) -> f64 {
    if axis == 0 { p.x } else { p.y }
}

impl<'p> KdTree<'p> {
    pub fn new(pts: &'p [Vec2]) -> KdTree<'p> {
        let mut tree = KdTree {
            pts,
            order: (0..pts.len() as u32).collect(),
            axis: vec![0; pts.len()],
        };
        let mut stack = vec![(0usize, pts.len())];
        while let Some((lo, hi)) = stack.pop() {
            if hi <= lo {
                continue;
            }
            let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
            for &i in &tree.order[lo..hi] {
                let p = pts[i as usize];
                if p.x < x0 {
                    x0 = p.x;
                }
                if p.x > x1 {
                    x1 = p.x;
                }
                if p.y < y0 {
                    y0 = p.y;
                }
                if p.y > y1 {
                    y1 = p.y;
                }
            }
            let axis = u8::from(y1 - y0 > x1 - x0);
            let mid = lo + (hi - lo) / 2;
            tree.order[lo..hi].select_nth_unstable_by(mid - lo, |&a, &b| {
                coord(pts[a as usize], axis)
                    .total_cmp(&coord(pts[b as usize], axis))
                    .then(a.cmp(&b))
            });
            tree.axis[mid] = axis;
            stack.push((lo, mid));
            stack.push((mid + 1, hi));
        }
        tree
    }

    /// The `k` places nearest to `q` other than `skip`, nearest first by
    /// (d², order); fewer when there are fewer.
    pub fn nearest(&self, q: Vec2, k: usize, skip: Option<u32>) -> Vec<(f64, u32)> {
        let mut best: Vec<(f64, u32)> = Vec::with_capacity(k + 1);
        if k > 0 {
            self.near(0, self.pts.len(), q, k, skip, &mut best);
        }
        best
    }

    fn near(
        &self,
        lo: usize,
        hi: usize,
        q: Vec2,
        k: usize,
        skip: Option<u32>,
        best: &mut Vec<(f64, u32)>,
    ) {
        if hi <= lo {
            return;
        }
        let mid = lo + (hi - lo) / 2;
        let i = self.order[mid];
        let p = self.pts[i as usize];
        if skip != Some(i) {
            let d2 = dist2(q, p);
            let better = best.len() < k || {
                let w = best[best.len() - 1];
                d2 < w.0 || (d2 == w.0 && i < w.1)
            };
            if better {
                let at = best.partition_point(|&(d, j)| d < d2 || (d == d2 && j < i));
                best.insert(at, (d2, i));
                best.truncate(k);
            }
        }
        let axis = self.axis[mid];
        let diff = coord(q, axis) - coord(p, axis);
        let (first, second) = if diff < 0.0 {
            ((lo, mid), (mid + 1, hi))
        } else {
            ((mid + 1, hi), (lo, mid))
        };
        self.near(first.0, first.1, q, k, skip, best);
        if best.len() < k || diff * diff <= best[best.len() - 1].0 {
            self.near(second.0, second.1, q, k, skip, best);
        }
    }

    /// Every place whose distance from `q` is at most `r` (d = √d², the
    /// module's rule), in any order; `skip` left out.
    pub fn within(&self, q: Vec2, r: f64, skip: Option<u32>, out: &mut Vec<u32>) {
        // A far-side place's distance is at least the axis difference less a
        // few roundings: a range is skipped only beyond that.
        let reach = r * (1.0 + 4.0 * f64::EPSILON);
        self.inside(0, self.pts.len(), q, r, reach, skip, out);
    }

    #[allow(clippy::too_many_arguments)]
    fn inside(
        &self,
        lo: usize,
        hi: usize,
        q: Vec2,
        r: f64,
        reach: f64,
        skip: Option<u32>,
        out: &mut Vec<u32>,
    ) {
        if hi <= lo {
            return;
        }
        let mid = lo + (hi - lo) / 2;
        let i = self.order[mid];
        let p = self.pts[i as usize];
        if skip != Some(i) && dist2(q, p).sqrt() <= r {
            out.push(i);
        }
        // The left range's places are at least `diff` from q on the axis, the right range's at least `−diff`.
        let axis = self.axis[mid];
        let diff = coord(q, axis) - coord(p, axis);
        if diff <= reach {
            self.inside(lo, mid, q, r, reach, skip, out);
        }
        if -diff <= reach {
            self.inside(mid + 1, hi, q, r, reach, skip, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute(pts: &[Vec2], q: Vec2, k: usize, skip: Option<u32>) -> Vec<(f64, u32)> {
        let mut all: Vec<(f64, u32)> = (0..pts.len() as u32)
            .filter(|&i| Some(i) != skip)
            .map(|i| (dist2(q, pts[i as usize]), i))
            .collect();
        all.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        all.truncate(k);
        all
    }

    #[test]
    fn nearest_and_within_are_exact_with_ties_and_an_outlier() {
        // A lattice (many ties), duplicates and one place far away.
        let mut pts = Vec::new();
        for j in 0..17 {
            for i in 0..23 {
                pts.push(Vec2::new(f64::from(i) * 2.0, f64::from(j) * 2.0));
            }
        }
        pts.push(Vec2::new(10.0, 10.0));
        pts.push(Vec2::new(10.0, 10.0));
        pts.push(Vec2::new(1.0e6, -3.0e5));
        let tree = KdTree::new(&pts);
        for (n, &q) in pts.iter().enumerate().step_by(7) {
            for k in [1, 4, 9] {
                let skip = Some(n as u32);
                assert_eq!(tree.nearest(q, k, skip), brute(&pts, q, k, skip), "{n} {k}");
            }
            for r in [0.0, 2.0, 4.5] {
                let mut got = Vec::new();
                tree.within(q, r, None, &mut got);
                got.sort_unstable();
                let want: Vec<u32> = (0..pts.len() as u32)
                    .filter(|&i| dist2(q, pts[i as usize]).sqrt() <= r)
                    .collect();
                assert_eq!(got, want, "{n} {r}");
            }
        }
        let far = pts.len() as u32 - 1;
        assert_eq!(tree.nearest(pts[far as usize], 1, Some(far)).len(), 1);
    }
}
