//! A surface's survey quality (plan/road.md Y18.4, docs/adr/0916 §4; ROAD-SURVEY-04, 05): corners nearly at one place
//! (a duplicate survey shot), corners that stand off the corners round them (spikes: Esri's Locate Outliers' height
//! comparison), triangles too long to say what the ground does (coverage gaps), and check points against the surface
//! by class (hard ground apart from vegetated: ASPRS' non-vegetated and vegetated accuracy, by the RMSE). Every limit is
//! the user's; nothing is changed: the raw records stay as they are.

use super::Surface;

/// Two corners closer than the limit: their places and heights, their plan distance and the height difference (b − a).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearPair {
    pub a: [f64; 3],
    pub b: [f64; 3],
    pub distance: f64,
    pub dz: f64,
}

/// A corner off the mean height of the corners within the radius: its place, the mean, the difference (z − mean) and
/// how many corners were round it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spike {
    pub at: [f64; 3],
    pub mean: f64,
    pub difference: f64,
    pub neighbours: usize,
}

/// The triangles with an edge longer than the limit: how many, their plan area and the longest edge of the surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gaps {
    pub triangles: usize,
    pub area: f64,
    pub longest: f64,
}

/// The corners in cells of `size`.
struct Buckets {
    size: f64,
    cells: std::collections::HashMap<(i64, i64), Vec<usize>>,
}

impl Buckets {
    fn new(points: &[[f64; 3]], size: f64) -> Self {
        let mut cells: std::collections::HashMap<(i64, i64), Vec<usize>> = std::collections::HashMap::new();
        for (i, p) in points.iter().enumerate() {
            cells.entry(Self::key(p[0], p[1], size)).or_default().push(i);
        }
        Self { size, cells }
    }

    fn key(x: f64, y: f64, size: f64) -> (i64, i64) {
        ((x / size).floor() as i64, (y / size).floor() as i64)
    }

    /// The corners in the cells round (x, y), a superset of those within `size`.
    fn near(&self, x: f64, y: f64) -> impl Iterator<Item = usize> + '_ {
        let (cx, cy) = Self::key(x, y, self.size);
        (cx - 1..=cx + 1).flat_map(move |i| (cy - 1..=cy + 1).flat_map(move |j| self.cells.get(&(i, j)).into_iter().flatten().copied()))
    }
}

fn plan(a: [f64; 3], b: [f64; 3]) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

fn before(a: [f64; 3], b: [f64; 3]) -> bool {
    (a[0], a[1], a[2]) < (b[0], b[1], b[2])
}

impl Surface {
    /// The pairs of corners closer than `near` in plan (each pair once, the smaller place first; in order of the
    /// first, then the second).
    pub fn near_pairs(&self, near: f64) -> Vec<NearPair> {
        if !(near.is_finite() && near > 0.0) {
            return Vec::new();
        }
        let v = &self.vertices;
        let buckets = Buckets::new(v, near);
        let mut out = Vec::new();
        for (i, p) in v.iter().enumerate() {
            for j in buckets.near(p[0], p[1]) {
                let q = v[j];
                if j == i || !before(*p, q) {
                    continue;
                }
                let d = plan(*p, q);
                if d < near {
                    out.push(NearPair { a: *p, b: q, distance: d, dz: q[2] - p[2] });
                }
            }
        }
        out.sort_by(|x, y| {
            (x.a[0], x.a[1], x.b[0], x.b[1]).partial_cmp(&(y.a[0], y.a[1], y.b[0], y.b[1])).unwrap_or(std::cmp::Ordering::Equal)
        });
        out
    }

    /// The corners whose height is more than `limit` off the mean of the other corners within `radius` in plan (those
    /// with none round them left out); the largest off first (then by place).
    pub fn spikes(&self, radius: f64, limit: f64) -> Vec<Spike> {
        if !(radius.is_finite() && radius > 0.0 && limit.is_finite() && limit >= 0.0) {
            return Vec::new();
        }
        let v = &self.vertices;
        let buckets = Buckets::new(v, radius);
        let mut out = Vec::new();
        for (i, p) in v.iter().enumerate() {
            // The heights summed from the lowest, so equal answers are equal bit for bit whatever the corners' order.
            let mut heights: Vec<f64> =
                buckets.near(p[0], p[1]).filter(|j| *j != i && plan(*p, v[*j]) <= radius).map(|j| v[j][2]).collect();
            heights.sort_by(f64::total_cmp);
            let n = heights.len();
            let sum: f64 = heights.iter().sum();
            if n == 0 {
                continue;
            }
            let mean = sum / n as f64;
            let difference = p[2] - mean;
            if difference.abs() > limit {
                out.push(Spike { at: *p, mean, difference, neighbours: n });
            }
        }
        out.sort_by(|a, b| {
            b.difference
                .abs()
                .partial_cmp(&a.difference.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then((a.at[0], a.at[1]).partial_cmp(&(b.at[0], b.at[1])).unwrap_or(std::cmp::Ordering::Equal))
        });
        out
    }

    /// The kept triangles with an edge longer than `limit` in plan.
    pub fn gaps(&self, limit: f64) -> Gaps {
        let mut out = Gaps { triangles: 0, area: 0.0, longest: 0.0 };
        for t in &self.triangles {
            let p = t.map(|i| self.vertices[i as usize]);
            let longest = plan(p[0], p[1]).max(plan(p[1], p[2])).max(plan(p[2], p[0]));
            out.longest = out.longest.max(longest);
            if longest > limit {
                out.triangles += 1;
                out.area += ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1])).abs() / 2.0;
            }
        }
        out
    }
}
