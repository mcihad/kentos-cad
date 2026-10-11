//! Bina sadeleştir (docs/adr/0221 §4): Ruas's short-edge removal (AGENT,
//! CartAGen's `simplify_building_ruas`). The shortest edge shorter than the
//! least length goes as its neighbours' directions say: between
//! perpendicular neighbours the neighbours meet; between parallel ones (a
//! finger or a notch) the shorter side's end is carried across; between
//! opposite ones (a step) the two sides become one mean line. An edit that
//! cannot be made leaves the edge; after one is made the short edges are
//! read again.

use crate::jsmath::{PI, atan2};
use crate::vec2::Vec2;

/// Bina sadeleştir's settings: the least edge length (m) and the parallel
/// and perpendicular limits (radians).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Building {
    pub min_edge: f64,
    pub parallel: f64,
    pub orthogonal: f64,
}

/// A ring of the building: its vertices and its edges' bulges (an arc edge is never touched).
#[derive(Clone, Debug, PartialEq)]
pub struct BuildingRing {
    pub pts: Vec<Vec2>,
    pub bulges: Vec<f64>,
}

fn length(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// The ring of `first` then `count` vertices from `from` (cyclic); the edges
/// it makes are straight (the ones they replace were).
fn rebuilt(r: &BuildingRing, first: &[Vec2], from: usize, count: usize) -> BuildingRing {
    let n = r.pts.len();
    let mut pts = first.to_vec();
    let mut bulges = vec![0.0; first.len()];
    for k in 0..count {
        let i = (from + k) % n;
        pts.push(r.pts[i]);
        bulges.push(r.bulges[i]);
    }
    if let Some(b) = bulges.last_mut() {
        *b = 0.0;
    }
    BuildingRing { pts, bulges }
}

/// Removes ring `r`'s edge from vertex `i`; none when it cannot.
fn remove_edge(r: &BuildingRing, i: usize, rule: &Building) -> Option<BuildingRing> {
    let n = r.pts.len();
    if n < 4 {
        return None;
    }
    let (ai, bi, b2) = ((i + n - 1) % n, (i + 1) % n, (i + 2) % n);
    if r.bulges[ai] != 0.0 || r.bulges[i] != 0.0 || r.bulges[bi] != 0.0 {
        return None;
    }
    let (a_, a, b, b_) = (r.pts[ai], r.pts[i], r.pts[bi], r.pts[b2]);
    let (d1x, d1y, d2x, d2y) = (a_.x - a.x, a_.y - a.y, b_.x - b.x, b_.y - b.y);
    if (d1x == 0.0 && d1y == 0.0) || (d2x == 0.0 && d2y == 0.0) {
        return None;
    }
    let mut alpha = atan2(d2y, d2x) - atan2(d1y, d1x);
    while alpha > PI {
        alpha -= 2.0 * PI;
    }
    while alpha <= -PI {
        alpha += 2.0 * PI;
    }
    let turn = alpha.abs();
    if turn >= PI / 2.0 - rule.orthogonal && turn <= PI / 2.0 + rule.orthogonal {
        // The neighbours meet: a and b become one vertex.
        let den = d1x * d2y - d1y * d2x;
        if den == 0.0 {
            return None;
        }
        let (wx, wy) = (b.x - a.x, b.y - a.y);
        let t = (wx * d2y - wy * d2x) / den;
        let c = Vec2::new(a.x + t * d1x, a.y + t * d1y);
        return Some(rebuilt(r, &[c], b2, n - 2));
    }
    // Four vertices become two: the edges into a′ and out of b′ must be straight too.
    if n < 5 || r.bulges[(ai + n - 1) % n] != 0.0 || r.bulges[b2] != 0.0 {
        return None;
    }
    if turn < rule.parallel {
        // A finger or a notch: cut across at the shorter side's end.
        let dd = d2x * d2x + d2y * d2y;
        let t = ((a_.x - b.x) * d2x + (a_.y - b.y) * d2y) / dd;
        let ca = (t > 0.0 && t < 1.0).then(|| Vec2::new(b.x + t * d2x, b.y + t * d2y));
        let ee = d1x * d1x + d1y * d1y;
        let u = ((b_.x - a.x) * d1x + (b_.y - a.y) * d1y) / ee;
        let cb = (u > 0.0 && u < 1.0).then(|| Vec2::new(a.x + u * d1x, a.y + u * d1y));
        let pair = match (ca, cb) {
            (Some(ca), None) => [ca, b_],
            (None, Some(cb)) => [a_, cb],
            (Some(ca), Some(cb)) => {
                if length(d1x, d1y) <= length(d2x, d2y) {
                    [ca, b_]
                } else {
                    [a_, cb]
                }
            }
            (None, None) => return None,
        };
        return Some(rebuilt(r, &pair, b2 + 1, n - 4));
    }
    if turn > PI - rule.parallel {
        // A step: one line through the middle, the mean of the two sides' directions.
        let (ux, uy) = ((a.x - a_.x) + (b_.x - b.x), (a.y - a_.y) + (b_.y - b.y));
        let lu = length(ux, uy);
        if lu == 0.0 {
            return None;
        }
        let (ux, uy) = (ux / lu, uy / lu);
        let m = Vec2::new((a_.x + b_.x) / 2.0, (a_.y + b_.y) / 2.0);
        let foot = |p: Vec2| {
            let s = (p.x - m.x) * ux + (p.y - m.y) * uy;
            Vec2::new(m.x + s * ux, m.y + s * uy)
        };
        return Some(rebuilt(r, &[foot(a_), foot(b_)], b2 + 1, n - 4));
    }
    None
}

/// One part's rings (the outer ring, then its holes) simplified; how many edges went.
pub fn simplify_rings(rings: &mut [BuildingRing], rule: &Building) -> usize {
    let mut done = 0;
    loop {
        let mut short: Vec<(f64, usize, usize)> = Vec::new();
        for (k, r) in rings.iter().enumerate() {
            let n = r.pts.len();
            for i in 0..n {
                if r.bulges[i] != 0.0 {
                    continue;
                }
                let (a, b) = (r.pts[i], r.pts[(i + 1) % n]);
                let l = length(b.x - a.x, b.y - a.y);
                if l < rule.min_edge {
                    short.push((l, k, i));
                }
            }
        }
        short.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
        let edited = short
            .iter()
            .find_map(|&(_, k, i)| remove_edge(&rings[k], i, rule).map(|r| (k, r)));
        match edited {
            Some((k, r)) => {
                rings[k] = r;
                done += 1;
            }
            None => return done,
        }
    }
}
