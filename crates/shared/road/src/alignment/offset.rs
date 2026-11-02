//! A parallel axis (docs/adr/0901 §8): the source alignment offset by d (right positive). It keeps the source's
//! stations; its place at a station is the source's moved along the normal. On an element of curvature k it is a
//! curve of curvature k/(1 + k·d) while 1 + k·d > 0; where 1 + k·d ≤ 0 it folds into a cusp and is refused with the
//! stations where it does. Its length is Σ (L + d·Δθ) over the elements. Far parts crossing (a hairpin) are found on
//! the offset sampled at most every metre (and 0.02 rad).

use super::chain::Alignment;
use super::element::Kind;

/// Where a parallel axis does not hold.
#[derive(Clone, Debug, PartialEq)]
pub enum OffsetError {
    /// It folds into a cusp between these stations.
    Cusp { from: f64, to: f64 },
    /// Two far parts of it cross near these stations.
    Crosses { a: f64, b: f64 },
}

/// The offset place at a station.
pub fn place(al: &Alignment, station: f64, d: f64) -> Option<[f64; 2]> {
    let at = al.at(station)?;
    // Right of the direction: (sin θ, −cos θ).
    Some([at.x + d * at.direction.sin(), at.y - d * at.direction.cos()])
}

/// The offset's length.
pub fn length(al: &Alignment, d: f64) -> f64 {
    al.elements.iter().map(|e| e.length + d * e.turn()).sum()
}

/// Whether the source offset by `d` holds: no cusp, no crossing.
pub fn check(al: &Alignment, d: f64) -> Result<(), OffsetError> {
    if d == 0.0 {
        return Ok(());
    }
    // 1 + k·d with d right positive: a left turn (k > 0) grows on the right. The left side is −d.
    let fold = |k: f64| 1.0 + k * d <= 0.0;
    for e in &al.elements {
        if e.kind == Kind::Tangent {
            continue;
        }
        let (a, b) = (fold(e.k0), fold(e.k1));
        if a || b {
            // Where k·d = −1 along a clothoid's linear curvature.
            let cross = |k_at: f64| {
                let rate = e.rate();
                if rate == 0.0 { 0.0 } else { ((k_at - e.k0) / rate).clamp(0.0, e.length) }
            };
            let s_fold = cross(-1.0 / d);
            let (from, to) = match (a, b) {
                (true, true) => (0.0, e.length),
                (true, false) => (0.0, s_fold),
                _ => (s_fold, e.length),
            };
            return Err(OffsetError::Cusp {
                from: e.station + from,
                to: e.station + to,
            });
        }
    }
    // Far parts crossing: the sampled offset's segments, on a grid of cells.
    let mut stations = Vec::new();
    for e in &al.elements {
        let turn = e.k0.abs().max(e.k1.abs()) * e.length;
        let n = ((e.length / 1.0).ceil().max((turn / 0.02).ceil()) as usize).clamp(1, 20_000);
        for i in 0..n {
            stations.push(e.station + e.length * i as f64 / n as f64);
        }
    }
    stations.push(al.length);
    let pts: Vec<[f64; 2]> = stations.iter().filter_map(|s| place(al, *s, d)).collect();
    if pts.len() < 4 {
        return Ok(());
    }
    let cell = (al.length / pts.len() as f64 * 4.0).max(d.abs()).max(1e-6);
    let key = |p: [f64; 2]| ((p[0] / cell).floor() as i64, (p[1] / cell).floor() as i64);
    let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> = std::collections::HashMap::new();
    for i in 0..pts.len() - 1 {
        let (a, b) = (key(pts[i]), key(pts[i + 1]));
        for x in a.0.min(b.0)..=a.0.max(b.0) {
            for y in a.1.min(b.1)..=a.1.max(b.1) {
                grid.entry((x, y)).or_default().push(i);
            }
        }
    }
    let mut cells: Vec<_> = grid.into_iter().collect();
    cells.sort_by_key(|(k, _)| *k);
    for (_, segs) in cells {
        for (n, &i) in segs.iter().enumerate() {
            for &j in &segs[n + 1..] {
                if j <= i + 1 && i <= j + 1 {
                    continue;
                }
                if crosses(pts[i], pts[i + 1], pts[j], pts[j + 1]) {
                    let (a, b) = (i.min(j), i.max(j));
                    return Err(OffsetError::Crosses {
                        a: stations[a],
                        b: stations[b],
                    });
                }
            }
        }
    }
    Ok(())
}

fn crosses(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let o = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let (d1, d2, d3, d4) = (o(a, b, c), o(a, b, d), o(c, d, a), o(c, d, b));
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0) && d1 != 0.0 && d2 != 0.0 && d3 != 0.0 && d4 != 0.0
}
