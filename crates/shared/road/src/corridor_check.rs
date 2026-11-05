//! A corridor's preflight (docs/adr/0956, plan/road.md Y50; ROAD-COR-13): before its surface and volumes are trusted,
//! where it folds, where it lies over itself and where its regions do not meet.
//!
//! - **Folding.** A cross's point `o` metres right of the alignment (right positive) is `P = C(s) + o·N(s)`, `N` the
//!   right normal; with `dN/ds = κ·T` (κ positive turning left), `dP/ds = (1 + κ·o)·T`. Where `1 + κ·o ≤ 0` — a point
//!   on the inside of a curve farther out than its radius — the corridor folds over itself (Civil 3D's “bowtie”). A
//!   cross's margin is the least `1 + κ·(d + o)` over its top line's points (`d` the alignment's own offset): 1 on a
//!   tangent, 0 where it folds. The sampled corridor's own fold (consecutive sections crossing, [`Built::folded`]) is
//!   the discrete form of the same.
//! - **Overlap.** Two strips of the top surface that are not neighbours (they share no station) and overlap in plan:
//!   a switchback's legs, a hairpin's slopes. Each pair of triangles is clipped one by the other (both convex,
//!   Sutherland–Hodgman); the overlap's area and, at each piece's centroid, how far apart the two surfaces are in
//!   height (a road over a road is far apart; a fold is not).
//! - **Seams.** Where one region ends and the next begins at the same station, the two crosses there should be one
//!   line: each code both have, how far apart in plan and in height; the codes only one has.

use crate::corridor::{Built, RegionCrosses, SAME_STATION, coded_points, top_line_of};

/// A cross's margin against folding: its region, station, the least `1 + κ·(d + o)`, the offset of the point that
/// gives it and the curvature there.
#[derive(Clone, Debug, PartialEq)]
pub struct Margin {
    pub region: usize,
    pub station: f64,
    pub margin: f64,
    pub offset: f64,
    pub curvature: f64,
}

/// Each solved cross's margin; `curvature(s)` the alignment's curvature at `s` (left positive), `shift` its own offset
/// (right positive, 0 for an alignment of its own). The curvature is read just before and just after the station
/// ([`SIDE`]) and the smaller margin kept: a cross where a curve meets a tangent belongs to both.
pub fn margins(regions: &[RegionCrosses], curvature: &dyn Fn(f64) -> Option<f64>, shift: f64) -> Vec<Margin> {
    let mut out = Vec::new();
    for (ri, region) in regions.iter().enumerate() {
        let t = &region.template;
        let counts = [t.left.components.len(), t.right.components.len()];
        for cross in &region.crosses {
            let Some(sec) = &cross.section else { continue };
            let line = &cross.line;
            let s = line.station;
            let sides: Vec<f64> = [s - SIDE, s + SIDE].into_iter().filter_map(curvature).collect();
            if sides.is_empty() {
                continue;
            }
            let (sin, cos) = line.direction.sin_cos();
            let mut best: Option<(f64, f64, f64)> = None;
            for p in top_line_of(line, sec, counts) {
                let o = (p[0] - line.at[0]) * sin - (p[1] - line.at[1]) * cos;
                for k in &sides {
                    let m = 1.0 + k * (shift + o);
                    if best.is_none_or(|b| m < b.0) {
                        best = Some((m, o, *k));
                    }
                }
            }
            if let Some((margin, offset, curvature)) = best {
                out.push(Margin { region: ri, station: s, margin, offset, curvature });
            }
        }
    }
    out
}

/// Two parts of the top surface over each other: the first's and the second's stations, the overlap's area in plan and
/// the least and the most height between them over it.
#[derive(Clone, Debug, PartialEq)]
pub struct Overlap {
    pub first: [f64; 2],
    pub second: [f64; 2],
    pub area: f64,
    pub least: f64,
    pub most: f64,
}

/// How far before and after a station its curvature is read (m).
pub const SIDE: f64 = 1e-9;

/// Overlaps below this (m²) are rounding.
pub const LEAST_OVERLAP: f64 = 1e-6;

fn signed2(ring: &[[f64; 2]]) -> f64 {
    let n = ring.len();
    (0..n).map(|i| ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1]).sum::<f64>() * 0.5
}

/// The part of convex `subject` inside convex counterclockwise `clip` (Sutherland–Hodgman).
fn clip(subject: &[[f64; 2]], clip: &[[f64; 2]; 3]) -> Vec<[f64; 2]> {
    let mut out = subject.to_vec();
    for i in 0..3 {
        let (a, b) = (clip[i], clip[(i + 1) % 3]);
        let side = |p: [f64; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        let input = std::mem::take(&mut out);
        let n = input.len();
        for k in 0..n {
            let (p, q) = (input[k], input[(k + 1) % n]);
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                out.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let t = sp / (sp - sq);
                out.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

/// The plane of a triangle's height at `p`.
fn height(t: &[[f64; 3]; 3], p: [f64; 2]) -> f64 {
    let (a, b, c) = (t[0], t[1], t[2]);
    let det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
    let u = ((p[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (p[1] - a[1])) / det;
    let v = ((b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1])) / det;
    a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2])
}

/// The centroid of a polygon of nonzero area.
fn centroid(ring: &[[f64; 2]]) -> [f64; 2] {
    let n = ring.len();
    let (mut cx, mut cy, mut a2) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let w = p[0] * q[1] - q[0] * p[1];
        a2 += w;
        cx += (p[0] + q[0]) * w;
        cy += (p[1] + q[1]) * w;
    }
    [cx / (3.0 * a2), cy / (3.0 * a2)]
}

/// The top surface's parts over each other: each pair of strips that share no station, their overlapping triangles'
/// area and the height between them; pairs whose strips touch on both sides merged into one.
pub fn overlaps(built: &Built) -> Vec<Overlap> {
    let strips = &built.strips;
    // Each strip's plan box, to skip pairs far apart.
    let boxes: Vec<[f64; 4]> = strips
        .iter()
        .map(|s| {
            built.triangles[s.3.clone()].iter().flatten().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, p| {
                [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])]
            })
        })
        .collect();
    let touch = |a: usize, b: usize| {
        let (x, y) = (&strips[a], &strips[b]);
        (x.1 - y.2).abs() < SAME_STATION || (x.2 - y.1).abs() < SAME_STATION || (x.1 - y.1).abs() < SAME_STATION
    };
    let mut pairs: Vec<(usize, usize, f64, f64, f64)> = Vec::new();
    for a in 0..strips.len() {
        for b in a + 1..strips.len() {
            if touch(a, b) {
                continue;
            }
            let (p, q) = (boxes[a], boxes[b]);
            if p[2] < q[0] || q[2] < p[0] || p[3] < q[1] || q[3] < p[1] {
                continue;
            }
            let (mut area, mut least, mut most) = (0.0, f64::INFINITY, f64::NEG_INFINITY);
            for ta in &built.triangles[strips[a].3.clone()] {
                let pa = [[ta[0][0], ta[0][1]], [ta[1][0], ta[1][1]], [ta[2][0], ta[2][1]]];
                for tb in &built.triangles[strips[b].3.clone()] {
                    let pb = [[tb[0][0], tb[0][1]], [tb[1][0], tb[1][1]], [tb[2][0], tb[2][1]]];
                    let piece = clip(&pa, &pb);
                    if piece.len() < 3 {
                        continue;
                    }
                    let s = signed2(&piece);
                    if s <= LEAST_OVERLAP {
                        continue;
                    }
                    area += s;
                    let c = centroid(&piece);
                    let gap = (height(ta, c) - height(tb, c)).abs();
                    least = least.min(gap);
                    most = most.max(gap);
                }
            }
            if area > LEAST_OVERLAP {
                pairs.push((a, b, area, least, most));
            }
        }
    }
    // Merge pairs whose first strips touch and whose second strips touch: one overlap of two stretches.
    let mut out: Vec<(Vec<usize>, Vec<usize>, f64, f64, f64)> = Vec::new();
    for (a, b, area, least, most) in pairs {
        match out.iter_mut().find(|o| o.0.iter().any(|x| *x == a || touch(*x, a)) && o.1.iter().any(|y| *y == b || touch(*y, b))) {
            Some(o) => {
                if !o.0.contains(&a) {
                    o.0.push(a);
                }
                if !o.1.contains(&b) {
                    o.1.push(b);
                }
                o.2 += area;
                o.3 = o.3.min(least);
                o.4 = o.4.max(most);
            }
            None => out.push((vec![a], vec![b], area, least, most)),
        }
    }
    let span = |ids: &[usize]| {
        ids.iter().fold([f64::INFINITY, f64::NEG_INFINITY], |r, i| [r[0].min(strips[*i].1), r[1].max(strips[*i].2)])
    };
    out.into_iter()
        .map(|(a, b, area, least, most)| Overlap { first: span(&a), second: span(&b), area, least, most })
        .collect()
}

/// Where one region meets the next: the station, the two regions, the largest plan shift and height step between the
/// codes both crosses have, which code gives each, and the codes only one has.
#[derive(Clone, Debug, PartialEq)]
pub struct Seam {
    pub station: f64,
    pub before: usize,
    pub after: usize,
    pub shift: f64,
    pub shift_code: Option<String>,
    pub step: f64,
    pub step_code: Option<String>,
    pub only_before: Vec<String>,
    pub only_after: Vec<String>,
}

/// The seams between consecutive regions that meet (the first's last cross and the next's first at the same station,
/// both solved).
pub fn seams(regions: &[RegionCrosses]) -> Vec<Seam> {
    let mut out = Vec::new();
    for (i, w) in regions.windows(2).enumerate() {
        let (Some(a), Some(b)) = (w[0].crosses.last(), w[1].crosses.first()) else { continue };
        if (a.line.station - b.line.station).abs() >= SAME_STATION {
            continue;
        }
        let (Some(pa), Some(pb)) = (coded_points(&w[0].template, a), coded_points(&w[1].template, b)) else { continue };
        let mut seam = Seam {
            station: a.line.station,
            before: i,
            after: i + 1,
            shift: 0.0,
            shift_code: None,
            step: 0.0,
            step_code: None,
            only_before: Vec::new(),
            only_after: Vec::new(),
        };
        for (code, p) in &pa {
            match pb.iter().find(|(c, _)| c == code) {
                Some((_, q)) => {
                    let shift = (p[0] - q[0]).hypot(p[1] - q[1]);
                    let step = (p[2] - q[2]).abs();
                    if shift > seam.shift {
                        seam.shift = shift;
                        seam.shift_code = Some(code.clone());
                    }
                    if step > seam.step {
                        seam.step = step;
                        seam.step_code = Some(code.clone());
                    }
                }
                None => seam.only_before.push(code.clone()),
            }
        }
        seam.only_after = pb.iter().filter(|(c, _)| !pa.iter().any(|(d, _)| d == c)).map(|(c, _)| c.clone()).collect();
        out.push(seam);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_triangles_overlap_by_their_common_part() {
        let a = [[0.0, 0.0], [2.0, 0.0], [0.0, 2.0]];
        let b = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
        assert!((signed2(&clip(&a, &b)) - 0.5).abs() < 1e-12);
        let far = [[5.0, 5.0], [6.0, 5.0], [5.0, 6.0]];
        assert!(clip(&a, &far).len() < 3 || signed2(&clip(&a, &far)).abs() < 1e-12);
    }

    #[test]
    fn a_plane_reads_its_height() {
        let t = [[0.0, 0.0, 1.0], [1.0, 0.0, 2.0], [0.0, 1.0, 3.0]];
        assert!((height(&t, [0.25, 0.25]) - 1.75).abs() < 1e-12);
        assert!((centroid(&[[0.0, 0.0], [3.0, 0.0], [0.0, 3.0]])[0] - 1.0).abs() < 1e-12);
    }
}
