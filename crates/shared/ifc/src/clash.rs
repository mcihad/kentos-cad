//! Çakışma (docs/adr/8102; plan/bim-plan.md BIM-17): two models' bodies that take the same room. The boxes are the first
//! sieve: two bodies whose boxes overlap by more than the tolerance on every axis are looked at; the bodies themselves
//! decide. Their surfaces cross — a triangle of each on both sides of the other's plane, the two crossings sharing a
//! stretch of their line — or one body holds the other whole (a point of it inside, by a ray's crossings). Faces that
//! only touch (a beam on a wall's top, a column against a wall's face) do not cross: contact is not a clash.
//!
//! A mesh is a view of a body (CLAUDE.md §23.3): what this finds is a body passing into another as the meshes show it;
//! its size is the boxes' overlap, never a measured volume.

/// One body: its owner (the caller's index), its corners and triangles.
#[derive(Clone, Copy, Debug)]
pub struct Body<'a> {
    pub owner: usize,
    pub positions: &'a [[f64; 3]],
    pub triangles: &'a [[u32; 3]],
}

/// Two bodies that take the same room: their owners, their boxes' overlap (x, y, z from, x, y, z to) and whether one
/// holds the other whole.
#[derive(Clone, Debug, PartialEq)]
pub struct Clash {
    pub a: usize,
    pub b: usize,
    pub overlap: [f64; 6],
    pub inside: bool,
}

/// How far from a plane a corner may be and still be on it, and how short a shared stretch may be and still be none,
/// metres: the arithmetic's own, far under any drawing's.
const EPS: f64 = 1e-9;

/// A body's box; none without a corner.
pub fn bounds(b: &Body<'_>) -> Option<[f64; 6]> {
    let mut out: Option<[f64; 6]> = None;
    for t in b.triangles {
        for &i in t {
            let Some(p) = b.positions.get(i as usize) else { continue };
            let k = out.get_or_insert([p[0], p[1], p[2], p[0], p[1], p[2]]);
            for j in 0..3 {
                k[j] = k[j].min(p[j]);
                k[j + 3] = k[j + 3].max(p[j]);
            }
        }
    }
    out
}

/// The overlap of two boxes, when it is more than `tolerance` on every axis.
fn overlap(a: &[f64; 6], b: &[f64; 6], tolerance: f64) -> Option<[f64; 6]> {
    let o = [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2]), a[3].min(b[3]), a[4].min(b[4]), a[5].min(b[5])];
    (0..3).all(|i| o[i + 3] - o[i] > tolerance).then_some(o)
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let r = dot(v, v).sqrt();
    (r > 0.0 && r.is_finite()).then(|| [v[0] / r, v[1] / r, v[2] / r])
}

/// Where triangle `t` meets the plane through `q` with unit normal `n`, when it has corners strictly on both sides: the
/// points of its crossing (the edges' crossings and the corners on the plane).
fn crossing(t: &[[f64; 3]; 3], q: [f64; 3], n: [f64; 3]) -> Option<Vec<[f64; 3]>> {
    let d = t.map(|p| dot(n, sub(p, q)));
    if !(d.iter().any(|&x| x > EPS) && d.iter().any(|&x| x < -EPS)) {
        return None;
    }
    let mut out = Vec::with_capacity(3);
    for i in 0..3 {
        let j = (i + 1) % 3;
        if d[i].abs() <= EPS {
            out.push(t[i]);
        } else if (d[i] > EPS && d[j] < -EPS) || (d[i] < -EPS && d[j] > EPS) {
            let s = d[i] / (d[i] - d[j]);
            out.push([t[i][0] + s * (t[j][0] - t[i][0]), t[i][1] + s * (t[j][1] - t[i][1]), t[i][2] + s * (t[j][2] - t[i][2])]);
        }
    }
    Some(out)
}

/// Whether two triangles cross: each passes through the other's plane, and the two crossings share a stretch of the
/// planes' common line longer than the arithmetic's own.
pub fn triangles_cross(t1: &[[f64; 3]; 3], t2: &[[f64; 3]; 3]) -> bool {
    let (Some(n1), Some(n2)) = (unit(cross(sub(t1[1], t1[0]), sub(t1[2], t1[0]))), unit(cross(sub(t2[1], t2[0]), sub(t2[2], t2[0]))))
    else {
        return false;
    };
    let (Some(c1), Some(c2)) = (crossing(t1, t2[0], n2), crossing(t2, t1[0], n1)) else {
        return false;
    };
    let Some(line) = unit(cross(n1, n2)) else {
        return false;
    };
    let span = |pts: &[[f64; 3]]| {
        let s: Vec<f64> = pts.iter().map(|p| dot(line, *p)).collect();
        (s.iter().copied().fold(f64::MAX, f64::min), s.iter().copied().fold(f64::MIN, f64::max))
    };
    let (a0, a1) = span(&c1);
    let (b0, b1) = span(&c2);
    a1.min(b1) - a0.max(b0) > EPS
}

/// Whether `p` is inside the closed mesh `b`: the crossings of a ray from it, odd. The ray leans off every axis so a
/// grid of boxes' edges does not lie on it.
fn inside(b: &Body<'_>, p: [f64; 3]) -> bool {
    // An arbitrary direction no face of a model lies along; its first figure only looks like Euler's constant.
    #[allow(clippy::approx_constant)]
    let dir = [0.577_215_664_9, 0.594_534_891_6, 0.559_729_867_8];
    let mut hits = 0usize;
    for t in b.triangles {
        let (Some(&v0), Some(&v1), Some(&v2)) =
            (b.positions.get(t[0] as usize), b.positions.get(t[1] as usize), b.positions.get(t[2] as usize))
        else {
            continue;
        };
        // Möller–Trumbore.
        let (e1, e2) = (sub(v1, v0), sub(v2, v0));
        let h = cross(dir, e2);
        let a = dot(e1, h);
        if a.abs() < 1e-15 {
            continue;
        }
        let f = 1.0 / a;
        let s = sub(p, v0);
        let u = f * dot(s, h);
        if !(0.0..=1.0).contains(&u) {
            continue;
        }
        let q = cross(s, e1);
        let v = f * dot(dir, q);
        if v < 0.0 || u + v > 1.0 {
            continue;
        }
        if f * dot(e2, q) > EPS {
            hits += 1;
        }
    }
    hits % 2 == 1
}

/// The corners of `b`'s triangle `t`, shifted by `-o` (the pair's overlap's corner: the arithmetic near zero).
fn corners(b: &Body<'_>, t: &[u32; 3], o: [f64; 3]) -> Option<[[f64; 3]; 3]> {
    let p = |i: u32| b.positions.get(i as usize).map(|p| sub(*p, o));
    Some([p(t[0])?, p(t[1])?, p(t[2])?])
}

/// Whether two bodies' surfaces cross within their boxes' overlap.
fn surfaces_cross(a: &Body<'_>, b: &Body<'_>, o: &[f64; 6]) -> bool {
    let at = [o[0], o[1], o[2]];
    let local = [0.0, 0.0, 0.0, o[3] - o[0], o[4] - o[1], o[5] - o[2]];
    let near = |t: &[[f64; 3]; 3]| {
        (0..3).all(|k| {
            let lo = t.iter().map(|p| p[k]).fold(f64::MAX, f64::min);
            let hi = t.iter().map(|p| p[k]).fold(f64::MIN, f64::max);
            hi >= local[k] - EPS && lo <= local[k + 3] + EPS
        })
    };
    let ta: Vec<[[f64; 3]; 3]> = a.triangles.iter().filter_map(|t| corners(a, t, at)).filter(near).collect();
    let tb: Vec<[[f64; 3]; 3]> = b.triangles.iter().filter_map(|t| corners(b, t, at)).filter(near).collect();
    ta.iter().any(|x| tb.iter().any(|y| triangles_cross(x, y)))
}

/// Points spread through the boxes' overlap, off its faces (a quarter, a half and three quarters of each side): where
/// two bodies whose faces lie on each other (a beam as wide as the wall it is sunk into) share their room although no
/// surface crosses.
fn witnesses(o: &[f64; 6]) -> impl Iterator<Item = [f64; 3]> + '_ {
    const AT: [f64; 3] = [0.5, 0.25, 0.75];
    AT.into_iter().flat_map(move |x| {
        AT.into_iter().flat_map(move |y| {
            AT.into_iter().map(move |z| {
                [o[0] + x * (o[3] - o[0]), o[1] + y * (o[4] - o[1]), o[2] + z * (o[5] - o[2])]
            })
        })
    })
}

/// The pairs of `a`'s and `b`'s bodies that take the same room: boxes overlapping by more than `tolerance` on every axis,
/// then surfaces that cross, or a point of the overlap inside both (faces on each other, or one body inside the other).
/// In `b`'s order, then `a`'s.
pub fn clashes(a: &[Body<'_>], b: &[Body<'_>], tolerance: f64) -> Vec<Clash> {
    let boxes_a: Vec<Option<[f64; 6]>> = a.iter().map(bounds).collect();
    let mut out = Vec::new();
    for bb in b {
        let Some(box_b) = bounds(bb) else { continue };
        for (i, ab) in a.iter().enumerate() {
            let Some(box_a) = &boxes_a[i] else { continue };
            let Some(o) = overlap(box_a, &box_b, tolerance) else { continue };
            let within = |x: &[f64; 6], y: &[f64; 6]| (0..3).all(|k| x[k] >= y[k] - EPS && x[k + 3] <= y[k + 3] + EPS);
            let crossed = surfaces_cross(ab, bb, &o);
            if crossed || witnesses(&o).any(|p| inside(ab, p) && inside(bb, p)) {
                let held = !crossed && (within(&box_b, box_a) || within(box_a, &box_b));
                out.push(Clash { a: ab.owner, b: bb.owner, overlap: o, inside: held });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(lo: [f64; 3], hi: [f64; 3]) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
        let p = |i: u32| {
            [
                if i & 1 == 0 { lo[0] } else { hi[0] },
                if i & 2 == 0 { lo[1] } else { hi[1] },
                if i & 4 == 0 { lo[2] } else { hi[2] },
            ]
        };
        let quads = [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]];
        ((0..8).map(p).collect(), quads.iter().flat_map(|q| [[q[0], q[1], q[2]], [q[0], q[2], q[3]]]).collect())
    }

    fn found(a: &(Vec<[f64; 3]>, Vec<[u32; 3]>), b: &(Vec<[f64; 3]>, Vec<[u32; 3]>), tolerance: f64) -> Vec<Clash> {
        let x = Body { owner: 1, positions: &a.0, triangles: &a.1 };
        let y = Body { owner: 2, positions: &b.0, triangles: &b.1 };
        clashes(&[x], &[y], tolerance)
    }

    #[test]
    fn a_column_into_a_wall_clashes_one_against_its_face_does_not() {
        // A wall 0.25 thick along x; a 30/30 column centred 0.40 m along it, through it.
        let wall = boxed([0.0, -0.125, 0.0], [6.0, 0.125, 3.0]);
        let into = boxed([0.25, -0.15, 0.0], [0.55, 0.15, 2.5]);
        let c = found(&wall, &into, 0.001);
        assert_eq!(c.len(), 1);
        assert!(!c[0].inside);
        assert!((c[0].overlap[0] - 0.25).abs() < 1e-12 && (c[0].overlap[4] - 0.125).abs() < 1e-12);
        // The column against the wall's face: touching, no clash.
        let against = boxed([1.0, 0.125, 0.0], [1.3, 0.425, 2.5]);
        assert!(found(&wall, &against, 0.001).is_empty());
        // A beam on the wall's top: touching, no clash; sunk 2 cm into it: a clash; 2 cm with a 5 cm tolerance: none.
        let on = boxed([0.0, -0.125, 3.0], [6.0, 0.125, 3.5]);
        assert!(found(&wall, &on, 0.001).is_empty());
        let sunk = boxed([0.0, -0.125, 2.98], [6.0, 0.125, 3.48]);
        assert_eq!(found(&wall, &sunk, 0.001).len(), 1);
        assert!(found(&wall, &sunk, 0.05).is_empty());
    }

    #[test]
    fn a_body_held_whole_by_another_clashes_as_inside() {
        let wall = boxed([0.0, -0.125, 0.0], [6.0, 0.125, 3.0]);
        let pipe = boxed([2.0, -0.05, 1.0], [2.1, 0.05, 1.1]);
        let c = found(&wall, &pipe, 0.001);
        assert_eq!(c.len(), 1);
        assert!(c[0].inside);
        // An L-shaped room's body and a box in the L's empty corner: their boxes overlap, the bodies do not.
        let l_shape: (Vec<[f64; 3]>, Vec<[u32; 3]>) = {
            let (mut p, mut t) = boxed([0.0, 0.0, 0.0], [4.0, 1.0, 3.0]);
            let (q, u) = boxed([0.0, 1.0, 0.0], [1.0, 4.0, 3.0]);
            let n = p.len() as u32;
            p.extend(q);
            t.extend(u.iter().map(|x| x.map(|i| i + n)));
            (p, t)
        };
        let corner = boxed([2.0, 2.0, 0.5], [3.0, 3.0, 1.5]);
        assert!(found(&l_shape, &corner, 0.001).is_empty());
        // Far apart: nothing looked at.
        let far = boxed([10.0, 0.0, 0.0], [11.0, 1.0, 1.0]);
        assert!(found(&wall, &far, 0.001).is_empty());
    }

    #[test]
    fn crossing_triangles_on_large_coordinates_are_found_as_near_the_origin() {
        let shift = |b: (Vec<[f64; 3]>, Vec<[u32; 3]>)| {
            (b.0.iter().map(|p| [p[0] + 587_312.345, p[1] + 4_400_612.789, p[2] + 1285.4]).collect::<Vec<_>>(), b.1)
        };
        let wall = shift(boxed([0.0, -0.125, 0.0], [6.0, 0.125, 3.0]));
        let into = shift(boxed([0.25, -0.15, 0.0], [0.55, 0.15, 2.5]));
        assert_eq!(found(&wall, &into, 0.001).len(), 1);
        let against = shift(boxed([1.0, 0.125, 0.0], [1.3, 0.425, 2.5]));
        assert!(found(&wall, &against, 0.001).is_empty());
    }
}
