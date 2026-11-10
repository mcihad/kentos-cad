//! The seamline polygons' shared boundary (plan/map-cad.md MAP-12, docs/adr/4460), `cad.orthophoto.compare`'s
//! `seam.polygons`: where a mosaic turns from one image to the other, so where their shift shows.
//!
//! A mosaic keeps a seamline polygon for each image, the part of it the mosaic uses (ArcGIS Pro's mosaic dataset
//! seamlines); the polygons do not overlap, so two images meet along the boundary their polygons share. Here that
//! boundary is the runs of the first polygon's rings (an area's outer rings and holes, a closed polyline's ring; edges
//! straight) that lie on the second's within 1 mm (`spatial_query::TOLERANCE`, docs/adr/0200 §1):
//!
//! - an edge of the first and one of the second share the stretch of the first's edge between the second's ends
//!   projected on it (clamped to the first's edge) when both ends of one of them lie within the tolerance of the
//!   other's line; projections meeting in a point share nothing;
//! - each edge of the first keeps the union of such stretches (stretches apart by no more than the tolerance joined);
//! - runs follow the first's rings in their order: a stretch reaching its edge's end (within the tolerance) goes on
//!   with the next edge's stretch starting at its start, through their corner; a ring's last run reaching the ring's
//!   end and its first run starting at its start are one, through the ring's start;
//! - a run's vertices are the first's own points (its vertices, or the points of its edges where a run starts or
//!   ends); a run no longer than the tolerance is dropped.

use kentos_commands::Entity;
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::ops::spatial_query::TOLERANCE;

use crate::checks::{Stop, error};
use crate::codes;

fn fail(message: String, path: &str) -> Stop {
    Stop::failed(error(codes::INVALID_INPUT, message, Some(path.to_owned())))
}

/// A seamline polygon's rings (east, north; each closed back to its start by its last edge): an area's outer rings
/// and holes, every part's; a closed polyline's ring. Refused at `path` when it is not in the drawing, not an area or
/// a closed polyline, or has an arc.
pub(crate) fn rings(doc: &Document, uid: &str, path: &str) -> Result<Vec<Vec<[f64; 2]>>, Stop> {
    let e = Uuid::parse_str(uid.trim())
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
        .ok_or_else(|| fail(format!("“{uid}” kimlikli dikiş çokgeni çizimde yok."), path))?;
    let straight = |b: &Option<Vec<f64>>| b.as_ref().is_none_or(|b| b.iter().all(|x| *x == 0.0));
    let ring = |pts: &[kentos_commands::Vec2]| -> Vec<[f64; 2]> { pts.iter().map(|v| [v.x, v.y]).collect() };
    let arcs = || fail(format!("“{uid}” dikiş çokgeninin yaylı kenarı var; kenarları düz olmalı."), path);
    match e {
        Entity::Polygon(p) => {
            let mut out = Vec::new();
            let mut parts = vec![(&p.pts, &p.bulges, &p.holes)];
            parts.extend(p.parts.iter().flatten().map(|q| (&q.pts, &q.bulges, &q.holes)));
            for (pts, bulges, holes) in parts {
                if !straight(bulges) {
                    return Err(arcs());
                }
                out.push(ring(pts));
                for h in holes.iter().flatten() {
                    if !straight(&h.bulges) {
                        return Err(arcs());
                    }
                    out.push(ring(&h.pts));
                }
            }
            Ok(out)
        }
        Entity::Polyline(p) if p.closed => {
            if !straight(&p.bulges) {
                return Err(arcs());
            }
            Ok(vec![ring(&p.pts)])
        }
        _ => Err(fail(format!("“{uid}” bir alan ya da kapalı çoklu çizgi değil; dikiş çokgeni olamaz."), path)),
    }
}

/// A ring's edges, its last back to its start; edges of no length left out.
fn edges(ring: &[[f64; 2]]) -> Vec<([f64; 2], [f64; 2])> {
    (0..ring.len())
        .map(|k| (ring[k], ring[(k + 1) % ring.len()]))
        .filter(|(a, b)| (b[0] - a[0]).hypot(b[1] - a[1]) > 0.0)
        .collect()
}

/// How far `v` lies from the line through `p` and `q` (`l` its length).
fn off(p: [f64; 2], q: [f64; 2], l: f64, v: [f64; 2]) -> f64 {
    ((v[0] - p[0]) * (q[1] - p[1]) - (v[1] - p[1]) * (q[0] - p[0])).abs() / l
}

/// The stretches (from, to; distances along it) of edge `p`–`q` shared with the edges `theirs`, joined.
fn stretches(p: [f64; 2], q: [f64; 2], theirs: &[([f64; 2], [f64; 2])], tol: f64) -> Vec<(f64, f64)> {
    let l = (q[0] - p[0]).hypot(q[1] - p[1]);
    let (ux, uy) = ((q[0] - p[0]) / l, (q[1] - p[1]) / l);
    let along = |v: [f64; 2]| (v[0] - p[0]) * ux + (v[1] - p[1]) * uy;
    let mut found: Vec<(f64, f64)> = Vec::new();
    for &(r, s) in theirs {
        let m = (s[0] - r[0]).hypot(s[1] - r[1]);
        let near = (off(p, q, l, r) <= tol && off(p, q, l, s) <= tol) || (off(r, s, m, p) <= tol && off(r, s, m, q) <= tol);
        if !near {
            continue;
        }
        let (t0, t1) = (along(r), along(s));
        let (lo, hi) = (t0.min(t1).max(0.0), t0.max(t1).min(l));
        if hi > lo {
            found.push((lo, hi));
        }
    }
    found.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (lo, hi) in found {
        match out.last_mut() {
            Some(last) if lo <= last.1 + tol => last.1 = last.1.max(hi),
            _ => out.push((lo, hi)),
        }
    }
    out
}

/// The point at distance `t` along `p`–`q` (`l` its length): its ends themselves at 0 and `l`.
fn at(p: [f64; 2], q: [f64; 2], l: f64, t: f64) -> [f64; 2] {
    if t <= 0.0 {
        p
    } else if t >= l {
        q
    } else {
        let f = t / l;
        [p[0] + f * (q[0] - p[0]), p[1] + f * (q[1] - p[1])]
    }
}

/// A run's length.
pub(crate) fn run_length(run: &[[f64; 2]]) -> f64 {
    run.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum()
}

/// The runs of `first`'s rings lying on `second`'s within `tol` (the module's rules), in `first`'s ring order.
pub(crate) fn shared(first: &[Vec<[f64; 2]>], second: &[Vec<[f64; 2]>], tol: f64) -> Vec<Vec<[f64; 2]>> {
    let theirs: Vec<([f64; 2], [f64; 2])> = second.iter().flat_map(|r| edges(r)).collect();
    let mut out = Vec::new();
    for ring in first {
        let mine = edges(ring);
        // A run, and whether it started at its ring's first edge's start.
        let mut runs: Vec<(Vec<[f64; 2]>, bool)> = Vec::new();
        let mut open = false;
        for (k, &(p, q)) in mine.iter().enumerate() {
            let l = (q[0] - p[0]).hypot(q[1] - p[1]);
            let found = stretches(p, q, &theirs, tol);
            if found.is_empty() {
                open = false;
                continue;
            }
            for (lo, hi) in found {
                let (a, b) = (at(p, q, l, lo), at(p, q, l, hi));
                let starts = lo <= tol;
                match runs.last_mut() {
                    // Going on over the corner: the run passes through it.
                    Some((run, _)) if open && starts => {
                        if run.last() != Some(&p) {
                            run.push(p);
                        }
                        run.push(b);
                    }
                    _ => runs.push((vec![a, b], k == 0 && starts)),
                }
                open = hi >= l - tol;
            }
        }
        // The ring's last run reaching its end goes on with its first run starting at its start, through the start.
        if open && runs.len() > 1 && runs[0].1 {
            let (first_run, _) = runs.remove(0);
            let start = mine[0].0;
            if let Some((last, _)) = runs.last_mut() {
                if last.last() != Some(&start) {
                    last.push(start);
                }
                last.extend(first_run.into_iter().skip_while(|v| *v == start));
            }
        }
        out.extend(runs.into_iter().map(|(run, _)| run).filter(|run| run_length(run) > tol));
    }
    out
}

/// The two seamline polygons' shared boundary, refused at `seam.polygons` when they share none.
pub(crate) fn shared_of(doc: &Document, polygons: &[String; 2]) -> Result<Vec<Vec<[f64; 2]>>, Stop> {
    let a = rings(doc, &polygons[0], "seam.polygons[0]")?;
    let b = rings(doc, &polygons[1], "seam.polygons[1]")?;
    let runs = shared(&a, &b, TOLERANCE);
    if runs.is_empty() {
        return Err(fail(
            "İki dikiş çokgeninin ortak sınırı yok (1 mm içinde): mozaikte birbirine değmiyorlar; iki görüntünün dikiş çokgenlerini verin.".into(),
            "seam.polygons",
        ));
    }
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
        vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
    }

    #[test]
    fn two_squares_side_by_side_share_their_common_side() {
        let runs = shared(&[square(0.0, 0.0, 1.0, 1.0)], &[square(1.0, 0.0, 2.0, 1.0)], TOLERANCE);
        assert_eq!(runs, vec![vec![[1.0, 0.0], [1.0, 1.0]]]);
    }

    #[test]
    fn a_run_over_the_ring_s_start_is_one() {
        // The first's ring starts in the middle of the shared side: its last edge and its first are one run.
        let first = vec![[1.0, 0.5], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]];
        let runs = shared(&[first], &[square(1.0, 0.0, 2.0, 1.0)], TOLERANCE);
        assert_eq!(runs, vec![vec![[1.0, 0.0], [1.0, 0.5], [1.0, 1.0]]]);
    }

    #[test]
    fn corners_touching_share_nothing_and_a_near_side_is_within_the_tolerance() {
        assert!(shared(&[square(0.0, 0.0, 1.0, 1.0)], &[square(1.0, 1.0, 2.0, 2.0)], TOLERANCE).is_empty());
        let near = shared(&[square(0.0, 0.0, 1.0, 1.0)], &[square(1.0004, 0.0, 2.0, 1.0)], TOLERANCE);
        assert_eq!(near.len(), 1);
        assert!(shared(&[square(0.0, 0.0, 1.0, 1.0)], &[square(1.002, 0.0, 2.0, 1.0)], TOLERANCE).is_empty());
    }

    #[test]
    fn a_partial_side_is_its_stretch() {
        let runs = shared(&[square(0.0, 0.0, 1.0, 1.0)], &[square(1.0, 0.25, 2.0, 0.75)], TOLERANCE);
        assert_eq!(runs, vec![vec![[1.0, 0.25], [1.0, 0.75]]]);
    }
}
