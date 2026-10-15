//! Çizimi temizle's geometry (docs/adr/0305; AutoCAD's OVERKILL): what is the same within a tolerance, what lies on
//! one line, and which vertices of a path add nothing. Distances are `sqrt(dx·dx + dy·dy)`, rounded at each step;
//! the loss of a change is the farthest any point it takes away lies from what stays.

use std::collections::HashMap;

use crate::Vec2;
use crate::jsmath::{js_max, js_min};

/// The distance between two points.
pub fn far(a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    (dx * dx + dy * dy).sqrt()
}

/// How far `p` lies from the line through `a` and `b` (from `a` when they are one point).
pub fn off_line(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l = (dx * dx + dy * dy).sqrt();
    if l == 0.0 {
        return far(p, a);
    }
    ((p.x - a.x) * dy - (p.y - a.y) * dx).abs() / l
}

/// The farthest two lists of points lie apart, point by point: `None` when their counts differ.
fn apart(a: &[Vec2], b: &[Vec2]) -> Option<f64> {
    (a.len() == b.len()).then(|| {
        a.iter()
            .zip(b)
            .fold(0.0_f64, |m, (p, q)| js_max(m, far(*p, *q)))
    })
}

/// Whether two vertex lists are one within `tol`, and the farthest their vertices lie apart: in order, or reversed
/// (`bulges` then reversed and negated: the caller compares them), or, closed, from any start. The match tried first
/// is in order, then reversed; a closed ring from each start in order, then reversed.
pub fn same_points(a: &[Vec2], b: &[Vec2], closed: bool, tol: f64) -> Option<(f64, Turn)> {
    let n = a.len();
    if n != b.len() || n == 0 {
        return None;
    }
    let reversed: Vec<Vec2> = b.iter().rev().copied().collect();
    if !closed {
        for (pts, turn) in [(b, Turn::Same), (&reversed[..], Turn::Reversed)] {
            if let Some(d) = apart(a, pts).filter(|d| *d <= tol) {
                return Some((d, turn));
            }
        }
        return None;
    }
    for (pts, reverse) in [(b, false), (&reversed[..], true)] {
        for start in 0..n {
            let shifted: Vec<Vec2> = (0..n).map(|i| pts[(start + i) % n]).collect();
            if let Some(d) = apart(a, &shifted).filter(|d| *d <= tol) {
                return Some((
                    d,
                    if reverse {
                        Turn::ReversedFrom(start)
                    } else {
                        Turn::From(start)
                    },
                ));
            }
        }
    }
    None
}

/// How the second list matched the first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    Same,
    Reversed,
    /// Closed, starting at this vertex.
    From(usize),
    /// Closed, reversed, starting at this vertex of the reversed list.
    ReversedFrom(usize),
}

/// The vertices of a path kept when the ones that add nothing are dropped, and the loss (the farthest a dropped
/// vertex lies from the kept path). An inner vertex goes when both its edges are straight (`bulges[i]` is the edge
/// from vertex i) and it lies within `tol` of the straight line from the last kept vertex to the next one, between
/// them: a vertex repeating the one before (only these without `straight`) or one on the line. A vertex repeating the
/// one before it whose own edge is an arc goes too: the arc starts at the one before. An open path's ends stay (its
/// last vertex repeating the one before takes that one's place); a closed path's first vertex stays. `keeps(i)`
/// keeps a vertex (it carries its own elevation, its widths change there). The kept path's edge from a kept vertex
/// is the one non-straight edge among the dropped ones after it, or straight. `None`: nothing dropped, or too few
/// vertices would stay.
pub fn thin(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    closed: bool,
    tol: f64,
    straight: bool,
    keeps: &dyn Fn(usize) -> bool,
) -> Option<(Vec<usize>, f64)> {
    let n = pts.len();
    let least = if closed { 3 } else { 2 };
    if n <= least {
        return None;
    }
    let bulge = |i: usize| bulges.map_or(0.0, |b| b.get(i).copied().unwrap_or(0.0));
    let mut kept: Vec<usize> = vec![0];
    let mut loss = 0.0_f64;
    let inner = if closed { n } else { n - 1 };
    for i in 1..inner {
        let prev = *kept.last().unwrap_or(&0);
        let next = (i + 1) % n;
        let mut drop = None;
        if !keeps(i) && bulge(i - 1) == 0.0 {
            let d = far(pts[i], pts[prev]);
            if bulge(i) == 0.0 {
                let off = off_line(pts[i], pts[prev], pts[next]);
                let (ux, uy) = (pts[next].x - pts[prev].x, pts[next].y - pts[prev].y);
                let span = (ux * ux + uy * uy).sqrt();
                let t = if span == 0.0 {
                    0.0
                } else {
                    ((pts[i].x - pts[prev].x) * ux + (pts[i].y - pts[prev].y) * uy) / span
                };
                let between = t >= -tol && t <= span + tol;
                if (straight || d <= tol) && off <= tol && between {
                    drop = Some(off);
                }
            } else if d <= tol && prev == i - 1 {
                drop = Some(d);
            }
        }
        match drop {
            Some(d) => loss = js_max(loss, d),
            None => kept.push(i),
        }
    }
    if !closed {
        let end = n - 1;
        let prev = *kept.last().unwrap_or(&0);
        if prev == end - 1 && prev != 0 && !keeps(prev) && bulge(prev) == 0.0 {
            let d = far(pts[end], pts[prev]);
            if d <= tol {
                loss = js_max(loss, d);
                kept.pop();
            }
        }
        kept.push(end);
    }
    (kept.len() < n && kept.len() >= least).then_some((kept, loss))
}

/// The kept path's edges: from each kept vertex, the one non-straight edge among the dropped ones after it.
pub fn kept_bulges(bulges: &[f64], kept: &[usize], n: usize, closed: bool) -> Vec<f64> {
    let mut out = Vec::with_capacity(kept.len());
    for (j, &k) in kept.iter().enumerate() {
        let to = match kept.get(j + 1) {
            Some(&next) => next,
            None if closed => n,
            None => k + 1,
        };
        let b = (k..to)
            .map(|i| bulges.get(i % n).copied().unwrap_or(0.0))
            .find(|b| *b != 0.0)
            .unwrap_or(0.0);
        out.push(b);
    }
    out
}

/// A line among those `lines` merged into another: its group, and its ends.
#[derive(Clone, Debug, PartialEq)]
pub struct Merged {
    /// The line that stays (an index of `lines`): the first of the cluster.
    pub kept: usize,
    /// The lines that go into it, in the input's order.
    pub absorbed: Vec<usize>,
    /// The kept line's new ends: the farthest ends of the cluster along it, on it.
    pub a: Vec2,
    pub b: Vec2,
    /// Which line's end each new end is (an index of `lines` and 0 for its `a`, 1 for its `b`).
    pub from: [(usize, usize); 2],
    /// The farthest an absorbed end lies from the kept line.
    pub loss: f64,
}

/// The lines that lie on one line within `tol` and overlap (with `touching`, also meet end to end within `tol`),
/// merged into the first of each cluster: only lines of one `group` merge. A line joins a cluster when both its ends
/// lie within `tol` of the cluster's first line and its span along it meets the cluster's; clusters joined by a later
/// line merge. Lines in the input's order; clusters by their first line.
pub fn merge_lines(lines: &[[Vec2; 2]], groups: &[u64], tol: f64, touching: bool) -> Vec<Merged> {
    struct Cluster {
        kept: usize,
        members: Vec<usize>,
        lo: f64,
        hi: f64,
    }
    let unit = |i: usize| {
        let [a, b] = lines[i];
        let l = far(a, b);
        ((b.x - a.x) / l, (b.y - a.y) / l)
    };
    let along = |i: usize, p: Vec2| {
        let (ux, uy) = unit(i);
        let a = lines[i][0];
        (p.x - a.x) * ux + (p.y - a.y) * uy
    };
    let on = |k: usize, j: usize| {
        let [a, b] = lines[k];
        off_line(lines[j][0], a, b) <= tol && off_line(lines[j][1], a, b) <= tol
    };
    // A grid of cells as wide as the median line, each holding the clusters whose span crosses it.
    let mut lengths: Vec<f64> = lines.iter().map(|[a, b]| far(*a, *b)).collect();
    lengths.sort_by(f64::total_cmp);
    let median = lengths.get(lengths.len() / 2).copied().unwrap_or(1.0);
    let cell = js_max(js_max(median, tol * 4.0), 1e-6);
    let key = |p: Vec2| ((p.x / cell).floor() as i64, (p.y / cell).floor() as i64);
    let cells_of = |a: Vec2, b: Vec2| {
        let (x0, y0) = key(Vec2::new(js_min(a.x, b.x) - tol, js_min(a.y, b.y) - tol));
        let (x1, y1) = key(Vec2::new(js_max(a.x, b.x) + tol, js_max(a.y, b.y) + tol));
        let mut out = Vec::new();
        // A line much longer than the cells is listed in at most 4096 of them; the rest is found through its ends.
        if (x1 - x0 + 1).saturating_mul(y1 - y0 + 1) > 4096 {
            out.push(key(a));
            out.push(key(b));
            return out;
        }
        for x in x0..=x1 {
            for y in y0..=y1 {
                out.push((x, y));
            }
        }
        out
    };
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for i in 0..lines.len() {
        if far(lines[i][0], lines[i][1]) <= tol {
            continue;
        }
        let [a, b] = lines[i];
        let mut near: Vec<usize> = cells_of(a, b)
            .iter()
            .filter_map(|c| grid.get(c))
            .flatten()
            .copied()
            .collect();
        near.sort_unstable();
        near.dedup();
        let mut joined: Option<usize> = None;
        for c in near {
            // A cluster merged into another is empty and takes no more lines.
            if clusters[c].members.is_empty() {
                continue;
            }
            let k = clusters[c].kept;
            if groups[k] != groups[i] || !on(k, i) || !on(i, k) {
                continue;
            }
            let (s, e) = {
                let (s, e) = (along(k, a), along(k, b));
                (js_min(s, e), js_max(s, e))
            };
            // Overlapping is sharing a length; meeting end to end (within `tol`) only with `touching`.
            let meets = if touching {
                s <= clusters[c].hi + tol && e >= clusters[c].lo - tol
            } else {
                s < clusters[c].hi && e > clusters[c].lo
            };
            if !meets {
                continue;
            }
            match joined {
                None => {
                    let cl = &mut clusters[c];
                    cl.members.push(i);
                    cl.lo = js_min(cl.lo, s);
                    cl.hi = js_max(cl.hi, e);
                    joined = Some(c);
                }
                Some(first) => {
                    // The line bridges two clusters: the later one's lines join the earlier.
                    let (keep, take) = if clusters[c].kept < clusters[first].kept {
                        (c, first)
                    } else {
                        (first, c)
                    };
                    let moved = std::mem::take(&mut clusters[take].members);
                    let kk = clusters[keep].kept;
                    for m in &moved {
                        for p in lines[*m] {
                            let t = along(kk, p);
                            clusters[keep].lo = js_min(clusters[keep].lo, t);
                            clusters[keep].hi = js_max(clusters[keep].hi, t);
                        }
                    }
                    clusters[keep].members.extend(moved);
                    joined = Some(keep);
                }
            }
        }
        let c = match joined {
            Some(c) => c,
            None => {
                clusters.push(Cluster {
                    kept: i,
                    members: vec![i],
                    lo: 0.0,
                    hi: far(a, b),
                });
                clusters.len() - 1
            }
        };
        let k = clusters[c].kept;
        let (ka, (ux, uy)) = (lines[k][0], unit(k));
        let (lo, hi) = (clusters[c].lo, clusters[c].hi);
        let from = Vec2::new(ka.x + ux * lo, ka.y + uy * lo);
        let to = Vec2::new(ka.x + ux * hi, ka.y + uy * hi);
        for cell in cells_of(from, to) {
            let list = grid.entry(cell).or_default();
            if !list.contains(&c) {
                list.push(c);
            }
        }
    }
    let mut out = Vec::new();
    for cl in clusters {
        if cl.members.len() < 2 {
            continue;
        }
        let mut members = cl.members;
        members.sort_unstable();
        let k = members[0];
        let mut lo = (0.0, (k, 0));
        let mut hi = (far(lines[k][0], lines[k][1]), (k, 1));
        let mut loss = 0.0_f64;
        for &m in &members[1..] {
            for (end, p) in lines[m].iter().enumerate() {
                let t = along(k, *p);
                if t < lo.0 {
                    lo = (t, (m, end));
                }
                if t > hi.0 {
                    hi = (t, (m, end));
                }
                loss = js_max(loss, off_line(*p, lines[k][0], lines[k][1]));
            }
        }
        let (ka, (ux, uy)) = (lines[k][0], unit(k));
        let point = |t: f64| Vec2::new(ka.x + ux * t, ka.y + uy * t);
        // An end that stays the kept line's own stays exactly where it was.
        let a = if lo.1 == (k, 0) {
            lines[k][0]
        } else {
            point(lo.0)
        };
        let b = if hi.1 == (k, 1) {
            lines[k][1]
        } else {
            point(hi.0)
        };
        out.push(Merged {
            kept: k,
            absorbed: members[1..].to_vec(),
            a,
            b,
            from: [lo.1, hi.1],
            loss,
        });
    }
    out.sort_by_key(|m| m.kept);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    #[test]
    fn overlapping_lines_on_one_line_merge_into_the_first() {
        let lines = [
            [v(0.0, 0.0), v(10.0, 0.0)],
            [v(5.0, 0.0), v(15.0, 0.0)],
            [v(20.0, 0.0), v(25.0, 0.0)],
            [v(14.0, 0.0), v(21.0, 0.0)],
            [v(0.0, 1.0), v(5.0, 1.0)],
        ];
        let merged = merge_lines(&lines, &[0; 5], 1e-9, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].kept, 0);
        assert_eq!(merged[0].absorbed, vec![1, 2, 3]);
        assert_eq!((merged[0].a, merged[0].b), (v(0.0, 0.0), v(25.0, 0.0)));
        // Another group does not merge.
        assert!(merge_lines(&lines, &[0, 1, 2, 3, 4], 1e-9, false).is_empty());
        // Lines meeting end to end merge only when asked.
        let ends = [[v(0.0, 0.0), v(10.0, 0.0)], [v(10.0, 0.0), v(20.0, 0.0)]];
        assert!(merge_lines(&ends, &[0, 0], 1e-9, false).is_empty());
        assert_eq!(merge_lines(&ends, &[0, 0], 1e-9, true)[0].b, v(20.0, 0.0));
    }

    #[test]
    fn a_path_loses_its_repeats_and_straight_vertices() {
        let pts = [
            v(0.0, 0.0),
            v(0.0, 0.0),
            v(5.0, 0.0),
            v(10.0, 0.0),
            v(10.0, 5.0),
        ];
        let (kept, loss) = thin(&pts, None, false, 1e-9, true, &|_| false).expect("thinned");
        assert_eq!(kept, vec![0, 3, 4]);
        assert_eq!(loss, 0.0);
        let (kept, _) = thin(&pts, None, false, 1e-9, false, &|_| false).expect("thinned");
        assert_eq!(kept, vec![0, 2, 3, 4]);
    }
}
