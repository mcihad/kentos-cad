//! A profile view split to a fixed span of elevation (plan/road.md Y84, docs/adr/0990). KGM's 2019 technical
//! specification: "Profillerin paftalarda ayrılan genişliğe sığmaması durumunda kot taşıma çizgisi yapılacaktır"; the
//! June 2024 sample sheets draw it as a line across the frame with the heights of the parts on its two sides (955–980,
//! 975–1000, 990–1015 on one sheet). Civil 3D's automatic split does the same: a user height, each part's datum
//! rounded to the grid.
//!
//! The parts are taken greedily: from where the last one ends, as far as a whole metre of station that keeps the lines
//! inside, the part's datum a grid step under its lowest elevation less a metre and its top `span` over it, its highest
//! elevation a metre under the top at least. What must fit is the design line's elevations (exact: its pieces' ends and
//! curves' extremes inside, and the two ends) and the terrain's points inside with its straight line's height at the
//! two ends. How much of the lines a part holds only grows with its end, so the farthest end is bisected.

use kentos_road::profile::vertical::Vertical;

/// A part: its stations and its datum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Part {
    pub from: f64,
    pub to: f64,
    pub datum: f64,
}

/// The lines whose elevations must fit: the design line and the terrain's points with a height, in station order.
pub(crate) struct Lines<'a> {
    pub vertical: &'a Vertical,
    pub terrain: Vec<(f64, f64)>,
    /// The terrain points' stations with no height (the line breaks there).
    pub gaps: Vec<f64>,
}

impl Lines<'_> {
    fn design(&self, s: f64) -> Option<f64> {
        self.vertical.at(s).map(|r| r.0)
    }

    /// The terrain's height at `s` on the straight line between its two neighbours with a height (none where a
    /// neighbour has none or `s` is off the points).
    fn terrain_at(&self, s: f64) -> Option<f64> {
        let t = &self.terrain;
        let i = t.partition_point(|p| p.0 < s - 1e-9);
        if i < t.len() && (t[i].0 - s).abs() <= 1e-9 {
            return Some(t[i].1);
        }
        if i == 0 || i == t.len() {
            return None;
        }
        let ((sa, za), (sb, zb)) = (t[i - 1], t[i]);
        if self.gaps.iter().any(|g| *g > sa + 1e-9 && *g < sb - 1e-9) {
            return None;
        }
        Some(za + (zb - za) * (s - sa) / (sb - sa))
    }

    /// The lowest and highest elevation of the lines between `a` and `b`.
    fn bounds(&self, a: f64, b: f64) -> (f64, f64) {
        let mut zs: Vec<f64> = Vec::new();
        for p in &self.vertical.pieces {
            let (pa, pb) = p.range();
            for s in [pa, pb] {
                if s > a - 1e-9 && s < b + 1e-9 {
                    zs.push(p.at(s).0);
                }
            }
        }
        zs.extend(self.vertical.curves.iter().filter_map(|c| c.extreme).filter(|e| e[0] > a - 1e-9 && e[0] < b + 1e-9).map(|e| e[1]));
        zs.extend([a, b].into_iter().filter_map(|s| self.design(s)));
        let i = self.terrain.partition_point(|p| p.0 < a - 1e-9);
        zs.extend(self.terrain[i..].iter().take_while(|p| p.0 < b + 1e-9).map(|p| p.1));
        zs.extend([a, b].into_iter().filter_map(|s| self.terrain_at(s)));
        zs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), z| (lo.min(*z), hi.max(*z)))
    }
}

/// The datum of a part whose lowest elevation is `lo`: a grid step under a metre below it.
pub fn datum_of(lo: f64, grid: f64) -> f64 {
    ((lo - 1.0) / grid).floor() * grid
}

/// The parts of the view `s0`–`s1` in a frame `span` metres of elevation high; `Err` the stations between which the
/// lines rise more than the span holds.
pub(crate) fn split(lines: &Lines<'_>, s0: f64, s1: f64, span: f64) -> Result<Vec<Part>, (f64, f64)> {
    let grid = crate::road_profiles_draw::grid_step(0.0, span);
    let fits = |a: f64, b: f64| {
        let (lo, hi) = lines.bounds(a, b);
        let datum = datum_of(lo, grid);
        (hi + 1.0 <= datum + span + 1e-9).then_some(datum)
    };
    let mut parts = Vec::new();
    let mut a = s0;
    loop {
        if let Some(datum) = fits(a, s1) {
            parts.push(Part { from: a, to: s1, datum });
            return Ok(parts);
        }
        // The whole metres after `a` and before `s1`: the farthest one that fits.
        let first = a.floor() + 1.0;
        let last = (s1 - 1e-9).ceil() - 1.0;
        if first > last + 1e-9 || fits(a, first).is_none() {
            return Err((a, first.min(s1)));
        }
        let (mut lo, mut hi) = (first, last);
        while hi - lo > 0.5 {
            let mid = ((lo + hi) * 0.5).ceil();
            if fits(a, mid).is_some() { lo = mid } else { hi = mid - 1.0 }
        }
        let datum = fits(a, lo).unwrap_or_default();
        parts.push(Part { from: a, to: lo, datum });
        a = lo;
    }
}

/// The points of a polyline between `a` and `b`, with its height on the line at an end it runs across; none when
/// fewer than two.
pub fn cut(points: &[[f64; 2]], a: f64, b: f64) -> Option<Vec<[f64; 2]>> {
    let (first, last) = (points.first()?[0], points.last()?[0]);
    if last <= a + 1e-9 || first >= b - 1e-9 {
        return None;
    }
    let at = |s: f64| {
        let i = points.partition_point(|p| p[0] < s);
        let (p, q) = (points[i - 1], points[i]);
        [s, p[1] + (q[1] - p[1]) * (s - p[0]) / (q[0] - p[0])]
    };
    let mut out = Vec::new();
    if first < a - 1e-9 && points.iter().all(|p| (p[0] - a).abs() > 1e-9) {
        out.push(at(a));
    }
    out.extend(points.iter().filter(|p| p[0] > a - 1e-9 && p[0] < b + 1e-9));
    if last > b + 1e-9 && points.iter().all(|p| (p[0] - b).abs() > 1e-9) {
        out.push(at(b));
    }
    (out.len() >= 2).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuts_keep_a_line_whole_inside() {
        let pts = [[0.0, 1.0], [10.0, 3.0], [20.0, 1.0]];
        assert_eq!(cut(&pts, 0.0, 20.0).unwrap(), pts.to_vec());
        assert_eq!(cut(&pts, 5.0, 15.0).unwrap(), vec![[5.0, 2.0], [10.0, 3.0], [15.0, 2.0]]);
        assert_eq!(cut(&pts, 10.0, 30.0).unwrap(), vec![[10.0, 3.0], [20.0, 1.0]]);
        assert!(cut(&pts, 20.0, 30.0).is_none());
    }

    #[test]
    fn datums_fall_on_the_grid() {
        assert_eq!(datum_of(957.3, 5.0), 955.0);
        assert_eq!(datum_of(956.0, 5.0), 955.0);
        assert_eq!(datum_of(955.5, 5.0), 950.0);
    }
}
