//! A room drawn beside walls that are already there (docs/adr/0770 §1): the parts of a closed wall's path that run on
//! an existing wall's matching line are not drawn; the existing wall is adopted. The path is counter-clockwise (its
//! left the room); its reference line is the path itself (Inner: the clear face, Center: the axis, Outer: the outer
//! face). A straight edge of a wall on the same level, parallel to a side within 0.001°, whose line of the same kind
//! (the face towards the room, the axis, the face away from it) lies on the side within the joins' tolerance, covers
//! the side where the wall's body (its two faces' edges) runs along it. What is left, walked round the path, is the new
//! walls' paths.
//! `scripts/fixtures/wall_adopt_cases.py` writes the same without this code.

use crate::geom::wall::line_at;
use crate::geom::wall_join::{JOIN_TOLERANCE, WallIn};
use crate::ops::wall_shift::{PARALLEL, edge_count};
use crate::vec2::Vec2;

/// Which line of the room's walls its path is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reference {
    Inner,
    Outer,
    Center,
}

/// What is left to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct Adopted {
    /// The new walls' paths; one closed path when nothing was adopted.
    pub paths: Vec<Vec<Vec2>>,
    pub closed: bool,
    /// The existing walls adopted (their indices in the walls given), in their order.
    pub walls: Vec<usize>,
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn len(a: Vec2) -> f64 {
    crate::jsmath::js_hypot(a.x, a.y)
}

/// The covered stretches of side `a → b` (lengths along it from `a`), and the walls that cover it.
fn covered(
    a: Vec2,
    b: Vec2,
    reference: Reference,
    walls: &[WallIn<'_>],
) -> (Vec<(f64, f64)>, Vec<usize>) {
    let l = len(sub(b, a));
    let u = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
    let n = Vec2::new(-u.y, u.x);
    let mut out = Vec::new();
    let mut by = Vec::new();
    for (j, w) in walls.iter().enumerate() {
        let lf = crate::geom::wall::left_face(w.spec);
        let rf = lf - w.spec.width;
        let offsets = [lf, rf, (lf + rf) / 2.0];
        let lines: Vec<Option<Vec<Vec2>>> = offsets
            .iter()
            .map(|&o| {
                line_at(w.pts, w.bulges, w.spec.closed, o)
                    .ok()
                    .map(|l| l.pts)
            })
            .collect();
        for e in 0..edge_count(w) {
            if w.bulges.and_then(|bs| bs.get(e)).copied().unwrap_or(0.0) != 0.0 {
                continue;
            }
            let (c, d) = (w.pts[e], w.pts[(e + 1) % w.pts.len()]);
            let le = len(sub(d, c));
            if le <= 0.0 {
                continue;
            }
            let v = Vec2::new((d.x - c.x) / le, (d.y - c.y) / le);
            if cross(u, v).abs() > PARALLEL {
                continue;
            }
            let ne = Vec2::new(-v.y, v.x);
            let s = |o: f64| dot(n, sub(Vec2::new(c.x + ne.x * o, c.y + ne.y * o), a));
            let (sl, sr) = (s(lf), s(rf));
            // The line that must lie on the side: its index in `offsets` and its place across the side.
            let (k, at) = match reference {
                Reference::Inner => {
                    if sl >= sr {
                        (0, sl)
                    } else {
                        (1, sr)
                    }
                }
                Reference::Outer => {
                    if sl <= sr {
                        (0, sl)
                    } else {
                        (1, sr)
                    }
                }
                Reference::Center => (2, s((lf + rf) / 2.0)),
            };
            let _ = k;
            if at.abs() > JOIN_TOLERANCE {
                continue;
            }
            // The stretch the wall's body runs along the side: both faces' edges projected on it.
            let (Some(lp), Some(rp)) = (&lines[0], &lines[1]) else {
                continue;
            };
            let ts: Vec<f64> = [lp[e], lp[(e + 1) % lp.len()], rp[e], rp[(e + 1) % rp.len()]]
                .iter()
                .map(|p| dot(u, sub(*p, a)))
                .collect();
            let lo = crate::jsmath::js_max(
                ts.iter()
                    .copied()
                    .fold(f64::INFINITY, crate::jsmath::js_min),
                0.0,
            );
            let hi = crate::jsmath::js_min(
                ts.iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, crate::jsmath::js_max),
                l,
            );
            if hi - lo > JOIN_TOLERANCE {
                out.push((lo, hi));
                if !by.contains(&j) {
                    by.push(j);
                }
            }
        }
    }
    (out, by)
}

/// The room's path `ring` (counter-clockwise, closed) less what `walls` already draw.
pub fn adopt(ring: &[Vec2], reference: Reference, walls: &[WallIn<'_>]) -> Adopted {
    let m = ring.len();
    // Each side's start along the whole path, and its covered stretches there.
    let mut start = vec![0.0; m + 1];
    let mut cover: Vec<(f64, f64)> = Vec::new();
    let mut adopted: Vec<usize> = Vec::new();
    for k in 0..m {
        let (a, b) = (ring[k], ring[(k + 1) % m]);
        let l = len(sub(b, a));
        start[k + 1] = start[k] + l;
        if l <= 0.0 {
            continue;
        }
        let (c, by) = covered(a, b, reference, walls);
        cover.extend(c.into_iter().map(|(x, y)| (start[k] + x, start[k] + y)));
        for j in by {
            if !adopted.contains(&j) {
                adopted.push(j);
            }
        }
    }
    adopted.sort_unstable();
    let total = start[m];
    if cover.is_empty() {
        return Adopted {
            paths: vec![ring.to_vec()],
            closed: true,
            walls: Vec::new(),
        };
    }
    // The covered stretches merged (a gap no longer than the tolerance closes), then what lies between them.
    cover.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (x, y) in cover {
        match merged.last_mut() {
            Some(last) if x <= last.1 + JOIN_TOLERANCE => last.1 = crate::jsmath::js_max(last.1, y),
            _ => merged.push((x, y)),
        }
    }
    if merged.len() > 1
        && merged[0].0 <= JOIN_TOLERANCE
        && merged[merged.len() - 1].1 >= total - JOIN_TOLERANCE
    {
        let first = merged.remove(0);
        if let Some(last) = merged.last_mut() {
            last.1 = total + first.1;
        }
    }
    let point = |t: f64| {
        let t = t.rem_euclid(total);
        let k = (0..m).rfind(|&k| start[k] <= t).unwrap_or(0);
        let (a, b) = (ring[k], ring[(k + 1) % m]);
        let l = start[k + 1] - start[k];
        let f = if l > 0.0 { (t - start[k]) / l } else { 0.0 };
        Vec2::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f)
    };
    let mut paths = Vec::new();
    for (i, &(_, end)) in merged.iter().enumerate() {
        let next = if i + 1 < merged.len() {
            merged[i + 1].0
        } else {
            merged[0].0 + total
        };
        if next - end <= JOIN_TOLERANCE {
            continue;
        }
        let mut path = vec![point(end)];
        for k in 0..2 * m {
            let corner = start[k % m] + total * (k / m) as f64;
            if corner > end + JOIN_TOLERANCE && corner < next - JOIN_TOLERANCE {
                path.push(ring[k % m]);
            }
        }
        path.push(point(next));
        paths.push(path);
    }
    Adopted {
        paths,
        closed: false,
        walls: adopted,
    }
}
