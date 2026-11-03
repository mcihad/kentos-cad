//! A junction of two alignments (plan/road.md Y12.2, docs/adr/0910 §2; ROAD-JUNC-01, 02, 04): where they cross, and at
//! an at-grade junction the curb return in each of its four quadrants, a circular fillet of the quadrant's radius
//! tangent to the two legs' edges (Civil 3D's intersection wizard's circular curb return). The legs are taken straight
//! along their tangents at the crossing: a leg that curves within the returns' reach is said, not followed. The edges'
//! offsets and the radii are the project's; nothing here is a standard's number.

use crate::alignment::Alignment;

/// The stations of `a` and `b` where they cross, in `a`'s order: the alignments sampled every `step` metres, crossing
/// segments refined by Newton's method on both stations.
pub fn crossings(a: &Alignment, b: &Alignment, step: f64) -> Vec<(f64, f64)> {
    let sample = |al: &Alignment| -> Vec<(f64, [f64; 2])> {
        let n = (al.length / step).ceil().max(1.0) as usize;
        (0..=n)
            .filter_map(|k| {
                let s = (k as f64 * step).min(al.length);
                al.at(s).map(|p| (s, [p.x, p.y]))
            })
            .collect()
    };
    let (pa, pb) = (sample(a), sample(b));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for wa in pa.windows(2) {
        let (a0, a1) = (wa[0].1, wa[1].1);
        for wb in pb.windows(2) {
            let (b0, b1) = (wb[0].1, wb[1].1);
            let (da, db) = ([a1[0] - a0[0], a1[1] - a0[1]], [b1[0] - b0[0], b1[1] - b0[1]]);
            let den = da[0] * db[1] - da[1] * db[0];
            if den.abs() < 1e-15 {
                continue;
            }
            let w = [b0[0] - a0[0], b0[1] - a0[1]];
            let t = (w[0] * db[1] - w[1] * db[0]) / den;
            let u = (w[0] * da[1] - w[1] * da[0]) / den;
            if !(-1e-9..=1.0 + 1e-9).contains(&t) || !(-1e-9..=1.0 + 1e-9).contains(&u) {
                continue;
            }
            let (mut sa, mut sb) = (wa[0].0 + t * (wa[1].0 - wa[0].0), wb[0].0 + u * (wb[1].0 - wb[0].0));
            for _ in 0..30 {
                let (Some(p), Some(q)) = (a.at(sa), b.at(sb)) else { break };
                let f = [p.x - q.x, p.y - q.y];
                if f[0].abs() + f[1].abs() < 1e-12 {
                    break;
                }
                let (ta, tb) = ([p.direction.cos(), p.direction.sin()], [q.direction.cos(), q.direction.sin()]);
                // F + [ta, -tb] · [dsa, dsb] = 0.
                let d = -ta[0] * tb[1] + ta[1] * tb[0];
                if d.abs() < 1e-15 {
                    break;
                }
                let dsa = (-f[0] * -tb[1] - -f[1] * -tb[0]) / d;
                let dsb = (ta[0] * -f[1] - ta[1] * -f[0]) / d;
                sa = (sa + dsa).clamp(0.0, a.length);
                sb = (sb + dsb).clamp(0.0, b.length);
            }
            if !out.iter().any(|c| (c.0 - sa).abs() < 1e-6 && (c.1 - sb).abs() < 1e-6) {
                out.push((sa, sb));
            }
        }
    }
    out.sort_by(|x, y| x.0.total_cmp(&y.0));
    out
}

/// A curb return: its centre, its tangent points on the first and the second leg's edge, the arc's sweep (radians,
/// counter-clockwise positive, from the first tangent point to the second) and its length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Return {
    pub center: [f64; 2],
    pub on_a: [f64; 2],
    pub on_b: [f64; 2],
    pub sweep: f64,
    pub length: f64,
}

/// A leg's edge at the crossing: the leg's unit tangent, the edge's offset (right positive) and its side (+1 right,
/// −1 left: the side the quadrant lies outward of).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    pub tangent: [f64; 2],
    pub offset: f64,
    pub side: f64,
}

/// The fillet of radius `r` between edge `a` of leg A and edge `b` of leg B, both legs through `p`, in the quadrant
/// outward of both edges. None when the legs are parallel.
pub fn fillet(p: [f64; 2], a: Edge, b: Edge, r: f64) -> Option<Return> {
    let (ta, tb, oa, ob, sa, sb) = (a.tangent, b.tangent, a.offset, b.offset, a.side, b.side);
    let right = |t: [f64; 2]| [t[1], -t[0]];
    let (na, nb) = (right(ta), right(tb));
    let (ka, kb) = (oa + sa * r, ob + sb * r);
    // p + ka na + u ta = p + kb nb + v tb
    let w = [kb * nb[0] - ka * na[0], kb * nb[1] - ka * na[1]];
    let den = ta[0] * -tb[1] - ta[1] * -tb[0];
    if den.abs() < 1e-12 {
        return None;
    }
    let u = (w[0] * -tb[1] - w[1] * -tb[0]) / den;
    let center = [p[0] + ka * na[0] + u * ta[0], p[1] + ka * na[1] + u * ta[1]];
    let on_a = [center[0] - sa * r * na[0], center[1] - sa * r * na[1]];
    let on_b = [center[0] - sb * r * nb[0], center[1] - sb * r * nb[1]];
    let (va, vb) = ([on_a[0] - center[0], on_a[1] - center[1]], [on_b[0] - center[0], on_b[1] - center[1]]);
    let sweep = (va[0] * vb[1] - va[1] * vb[0]).atan2(va[0] * vb[0] + va[1] * vb[1]);
    Some(Return { center, on_a, on_b, sweep, length: r * sweep.abs() })
}

/// A road's deck over or under a separated junction: its alignment and its edges, left and right of it (positive).
#[derive(Clone, Copy, Debug)]
pub struct Deck<'a> {
    pub alignment: &'a Alignment,
    pub left: f64,
    pub right: f64,
}

/// A place where the two decks overlap in plan: its place, and its station and offset (right positive) on the upper
/// and the lower road.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Overlap {
    pub x: f64,
    pub y: f64,
    pub upper_station: f64,
    pub upper_offset: f64,
    pub lower_station: f64,
    pub lower_offset: f64,
}

/// How many lower stations and offsets the overlap is sampled at around `crossing`: `(stations, offsets)` lines,
/// and the stations' range (docs/adr/0917 §2).
pub fn overlap_grid(upper: Deck<'_>, lower: Deck<'_>, crossing: f64, sine: f64, step: f64) -> (usize, usize, f64, f64) {
    let reach = (upper.left + upper.right + lower.left + lower.right) / sine.abs() + step;
    let (s0, s1) = ((crossing - reach).max(0.0), (crossing + reach).min(lower.alignment.length));
    let n = ((s1 - s0) / step).ceil().max(1.0) as usize;
    let m = ((lower.left + lower.right) / step).ceil().max(1.0) as usize;
    (n + 1, m + 1, s0, s1)
}

/// The places where `upper` lies over `lower` in plan around the lower road's `crossing` station (docs/adr/0917 §2):
/// the lower road's width and stations each side of the crossing cut into equal steps no longer than `step`, every
/// grid place stationed on the upper road (its nearest foot) and kept when within its edges; and on every grid line,
/// along and across, the upper road's edges found by bisection where the line crosses them. `sine` is the sine of
/// the angle between the roads at the crossing.
#[allow(clippy::needless_range_loop)] // the grid's columns, along each line
pub fn overlap(upper: Deck<'_>, lower: Deck<'_>, crossing: f64, sine: f64, step: f64) -> Vec<Overlap> {
    let (n, m, s0, s1) = overlap_grid(upper, lower, crossing, sine, step);
    let station = |i: usize| if i + 1 == n { s1 } else { s0 + (s1 - s0) * i as f64 / (n - 1) as f64 };
    let width = lower.left + lower.right;
    let offset = |j: usize| if j + 1 == m { lower.right } else { -lower.left + width * j as f64 / (m - 1) as f64 };
    let on_upper = |s: f64, o: f64| -> Option<Overlap> {
        let q = crate::alignment::offset::place(lower.alignment, s, o)?;
        let foot = upper.alignment.locate(q).nearest?;
        Some(Overlap {
            x: q[0],
            y: q[1],
            upper_station: foot.at.station,
            upper_offset: foot.offset,
            lower_station: s,
            lower_offset: o,
        })
    };
    let inside = |p: &Overlap| p.upper_offset >= -upper.left - 1e-9 && p.upper_offset <= upper.right + 1e-9;
    let grid: Vec<Vec<Option<Overlap>>> = (0..n).map(|i| (0..m).map(|j| on_upper(station(i), offset(j))).collect()).collect();
    let mut out: Vec<Overlap> = grid.iter().flatten().flatten().filter(|p| inside(p)).copied().collect();
    let edges = [-upper.left, upper.right];
    // Where the upper offset passes an edge between two grid places: bisected along the line between them.
    let mut cross = |a: (f64, f64, &Option<Overlap>), b: (f64, f64, &Option<Overlap>)| {
        let (Some(pa), Some(pb)) = (a.2, b.2) else { return };
        for e in edges {
            if (pa.upper_offset - e) * (pb.upper_offset - e) >= 0.0 {
                continue;
            }
            let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
            let below = pa.upper_offset < e;
            let mut found = None;
            for _ in 0..200 {
                let t = 0.5 * (lo + hi);
                let Some(p) = on_upper(a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)) else { break };
                found = Some(p);
                if (p.upper_offset < e) == below {
                    lo = t;
                } else {
                    hi = t;
                }
                let span = (hi - lo) * ((b.0 - a.0).abs() + (b.1 - a.1).abs());
                if span < 1e-12 {
                    break;
                }
            }
            if let Some(p) = found {
                out.push(p);
            }
        }
    };
    for j in 0..m {
        for i in 0..n - 1 {
            cross((station(i), offset(j), &grid[i][j]), (station(i + 1), offset(j), &grid[i + 1][j]));
        }
    }
    for (i, row) in grid.iter().enumerate() {
        for j in 0..m - 1 {
            cross((station(i), offset(j), &row[j]), (station(i), offset(j + 1), &row[j + 1]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_right_angle_crossing_has_quarter_circle_returns() {
        // A east, B north, edges 3 m each side, radius 10: the north-east quadrant is left of A and right of B.
        let e = |t: [f64; 2], offset: f64, side: f64| Edge { tangent: t, offset, side };
        let f = fillet([0.0, 0.0], e([1.0, 0.0], -3.0, -1.0), e([0.0, 1.0], 3.0, 1.0), 10.0).expect("a return");
        assert!((f.center[0] - 13.0).abs() < 1e-12 && (f.center[1] - 13.0).abs() < 1e-12, "{f:?}");
        assert!((f.on_a[1] - 3.0).abs() < 1e-12 && (f.on_a[0] - 13.0).abs() < 1e-12);
        assert!((f.on_b[0] - 3.0).abs() < 1e-12 && (f.on_b[1] - 13.0).abs() < 1e-12);
        assert!((f.length - 5.0 * std::f64::consts::PI).abs() < 1e-12);
        assert!(fillet([0.0, 0.0], e([1.0, 0.0], 3.0, 1.0), e([1.0, 0.0], 3.0, 1.0), 10.0).is_none());
    }
}
