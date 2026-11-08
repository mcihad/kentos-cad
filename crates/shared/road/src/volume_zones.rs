//! Kazı ve dolgu bölgeleri (docs/adr/1840; plan/map-cad.md MAP-15): where a comparison surface lies below a base (cut)
//! and above it (fill), as areas with their plan areas and volumes. ArcGIS's Surface Difference groups contiguous
//! triangles and parts of triangles of one class into a polygon; Civil 3D bands a volume surface from a datum. Here
//! each convex piece of the two surfaces' overlay ([`Ground::overlay`], where the difference `d = z_comparison − z_base`
//! is linear) is cut at zero and, with an interval `i`, at its multiples: band `k` is `[k·i, (k+1)·i)`; without an
//! interval band −1 is the cut (`d < 0`) and band 0 the fill. `k < 0` is cut and `k ≥ 0` fill, so no band mixes them.
//! A band's pieces are dissolved by the core's area algebra (`geoprocess::union_all`: its overlay takes places within
//! 1 µm as one and drops slivers thinner than that) into zones, outer rings counterclockwise and holes clockwise. A
//! zone's area and volume are its pieces' sums (a sliver the overlay dropped goes to the nearest zone of its band), so
//! the zones' volumes are the difference's own ([`crate::volume::difference`]). A difference within 1 nm of zero is
//! taken as zero ([`ZERO`]): where a design surface is the ground itself, the two triangles' planes interpolated at one
//! place differ by rounding alone (10⁻¹⁴ m), and would otherwise strew the coincident ground with cut and fill specks.
//! A piece where the two surfaces coincide (d zero all over it) is in no zone; its area is said. A ring's vertex within
//! 1 µm of the line through its neighbours is dropped (the area algebra's own tolerance), so the pieces' corners along a
//! straight edge leave it, at a project's coordinates (10⁶ m) as near the origin.

use std::collections::BTreeMap;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::region::inside_area;
use kentos_geometry_core::ops::geoprocess::union_all;

use crate::volume::{Ground, Triangle, cut_at, integral, split};

/// A difference this near zero (m) is zero: 1 nm, below any survey's resolution and above the rounding of two planes
/// interpolated at one place at heights of thousands of metres.
pub const ZERO: f64 = 1e-9;

/// A ring's vertex this near the line through its neighbours (m) is dropped: the area algebra's tolerance (1 µm).
pub const STRAIGHT: f64 = 1e-6;

/// A zone: its band (`k`: `[k·i, (k+1)·i)` with an interval; without one −1 the cut, 0 the fill), its plan area (m²), the
/// integral of the difference over it (m³; negative in the cut), its outer ring (counterclockwise, from its smallest
/// place, x then y) and its holes (clockwise, each from its smallest place, ordered by it); its deepest place and how
/// deep (docs/adr/2740): the corner of its pieces where the difference is largest in size (the difference is linear
/// over a piece, so its largest is at a corner), the cut's depth and the fill's height alike above 0; among corners
/// within [`ZERO`] of the largest, the westmost, then the southmost.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Zone {
    pub band: i64,
    pub area: f64,
    pub volume: f64,
    pub outer: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
    pub deepest: [f64; 2],
    pub depth: f64,
}

/// Two surfaces' zones, ordered by band and then by their outer ring's first place; the cut and fill volumes, summed
/// as [`crate::volume::difference`] sums them; the cut and fill plan areas (a coincident piece in neither); the area
/// where the two coincide; the area both cover and the comparison's area off the base; the least and the largest
/// difference.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Zones {
    pub zones: Vec<Zone>,
    pub cut: f64,
    pub fill: f64,
    pub cut_area: f64,
    pub fill_area: f64,
    pub coincident: f64,
    pub covered: f64,
    pub uncovered: f64,
    pub least: Option<f64>,
    pub largest: Option<f64>,
}

/// More bands than allowed: how many the interval would give.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TooManyBands(pub u64);

/// A band's convex piece: its corners with the difference there, plan area and the difference's integral over it.
struct Piece {
    pts: Vec<[f64; 2]>,
    ds: Vec<f64>,
    area: f64,
    volume: f64,
}

fn push(bands: &mut BTreeMap<i64, Vec<Piece>>, band: i64, part: &[([f64; 2], f64)]) {
    if part.len() < 3 {
        return;
    }
    let (area, volume) = integral(part);
    if area > 0.0 {
        bands.entry(band).or_default().push(Piece {
            pts: part.iter().map(|(p, _)| *p).collect(),
            ds: part.iter().map(|(_, d)| *d).collect(),
            area,
            volume,
        });
    }
}

/// Whether the corner `p` with `depth` is deeper than the deepest so far (`best`): deeper by more than [`ZERO`], or as
/// deep within it and west of it, then south (docs/adr/2740).
fn deeper(p: [f64; 2], depth: f64, best: Option<([f64; 2], f64)>) -> bool {
    match best {
        None => true,
        Some((_, d)) if depth > d + ZERO => true,
        Some((q, d)) if depth >= d - ZERO => (p[0], p[1]) < (q[0], q[1]),
        Some(_) => false,
    }
}

/// How many bands of `interval` the differences from `least` to `largest` meet.
pub fn band_count(least: f64, largest: f64, interval: f64) -> f64 {
    (largest / interval).floor() - (least / interval).floor() + 1.0
}

/// The zones of `comparison` over `base` (each a set of triangles that do not overlap in plan), over the comparison's
/// plan; `interval` (m, above 0) bands the difference from zero. Refused when the interval would give more than
/// `most_bands` bands.
pub fn cut_fill(
    comparison: &[Triangle],
    base: &[Triangle],
    interval: Option<f64>,
    most_bands: u64,
) -> Result<Zones, TooManyBands> {
    let mut out = Zones::default();
    let mut kept: Vec<Vec<([f64; 2], f64)>> = Vec::new();
    let mut ground = Ground::new(base);
    let (covered, uncovered) = ground.overlay(comparison, |raw| {
        let with: Vec<([f64; 2], f64)> = raw.iter().map(|(p, d)| (*p, if d.abs() <= ZERO { 0.0 } else { *d })).collect();
        if with.iter().all(|(_, d)| *d == 0.0) {
            out.coincident += integral(&with).0;
            return;
        }
        let (fa, fv) = integral(&split(&with, true));
        let (ca, cv) = integral(&split(&with, false));
        out.fill += fv;
        out.fill_area += fa;
        out.cut -= cv;
        out.cut_area += ca;
        for (_, d) in &with {
            out.least = Some(out.least.map_or(*d, |v| v.min(*d)));
            out.largest = Some(out.largest.map_or(*d, |v| v.max(*d)));
        }
        kept.push(with);
    });
    out.covered = covered;
    out.uncovered = uncovered;
    let (Some(least), Some(largest)) = (out.least, out.largest) else { return Ok(out) };
    if let Some(i) = interval {
        let count = band_count(least, largest, i);
        if !(count <= most_bands as f64) {
            return Err(TooManyBands(if count.is_finite() { count as u64 } else { u64::MAX }));
        }
    }
    let mut bands: BTreeMap<i64, Vec<Piece>> = BTreeMap::new();
    for with in &kept {
        match interval {
            None => {
                push(&mut bands, -1, &split(with, false));
                push(&mut bands, 0, &split(with, true));
            }
            Some(i) => {
                let (lo, hi) = with.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), (_, d)| (a.min(*d), b.max(*d)));
                for k in (lo / i).floor() as i64..=(hi / i).floor() as i64 {
                    let low = cut_at(with, k as f64 * i, true);
                    if low.len() >= 3 {
                        push(&mut bands, k, &cut_at(&low, (k + 1) as f64 * i, false));
                    }
                }
            }
        }
    }
    for (band, pieces) in bands {
        out.zones.extend(dissolve(band, &pieces));
    }
    out.zones.sort_by(|a, b| {
        (a.band, a.outer.first().map(|p| (p[0], p[1])))
            .partial_cmp(&(b.band, b.outer.first().map(|p| (p[0], p[1]))))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}

/// A band's pieces dissolved into its zones, each with its pieces' area and volume.
fn dissolve(band: i64, pieces: &[Piece]) -> Vec<Zone> {
    let areas: Vec<Area> = pieces
        .iter()
        .map(|p| Area {
            outer: Ring { pts: p.pts.iter().map(|q| Vec2::new(q[0], q[1])).collect(), bulges: None },
            holes: Vec::new(),
        })
        .collect();
    let merged = union_all(&areas);
    if merged.is_empty() {
        return Vec::new();
    }
    let boxes: Vec<[f64; 4]> = merged
        .iter()
        .map(|a| {
            a.outer.pts.iter().fold([f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY], |b, p| {
                [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)]
            })
        })
        .collect();
    let mut sums = vec![(0.0, 0.0); merged.len()];
    let mut deepest: Vec<Option<([f64; 2], f64)>> = vec![None; merged.len()];
    for p in pieces {
        // A convex piece's corners' mean is inside it, so inside the zone it went to (unless it was a sliver).
        let n = p.pts.len() as f64;
        let c = p.pts.iter().fold([0.0, 0.0], |s, q| [s[0] + q[0] / n, s[1] + q[1] / n]);
        let holding: Vec<usize> =
            (0..merged.len()).filter(|&k| boxes[k][0] <= c[0] && c[0] <= boxes[k][2] && boxes[k][1] <= c[1] && c[1] <= boxes[k][3]).collect();
        let k = match holding.as_slice() {
            [one] => *one,
            _ => holding
                .iter()
                .copied()
                .find(|&k| inside_area(&merged[k], Vec2::new(c[0], c[1])))
                .unwrap_or_else(|| nearest(&merged, c)),
        };
        sums[k].0 += p.area;
        sums[k].1 += p.volume;
        for (q, d) in p.pts.iter().zip(&p.ds) {
            if deeper(*q, d.abs(), deepest[k]) {
                deepest[k] = Some((*q, d.abs()));
            }
        }
    }
    merged
        .iter()
        .zip(sums)
        .zip(deepest)
        .map(|((a, (area, volume)), deep)| {
            let mut holes: Vec<Vec<[f64; 2]>> = a.holes.iter().map(|h| ring(h, false)).filter(|h| h.len() >= 3).collect();
            holes.sort_by(|p, q| (p[0][0], p[0][1]).partial_cmp(&(q[0][0], q[0][1])).unwrap_or(std::cmp::Ordering::Equal));
            let (deepest, depth) = deep.unwrap_or(([0.0, 0.0], 0.0));
            Zone { band, area, volume, outer: ring(&a.outer, true), holes, deepest, depth }
        })
        .collect()
}

/// The zone whose boundary is nearest `c` (a sliver's place).
fn nearest(areas: &[Area], c: [f64; 2]) -> usize {
    let away = |r: &Ring| {
        let n = r.pts.len();
        (0..n)
            .map(|k| {
                let (a, b) = (r.pts[k], r.pts[(k + 1) % n]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len = dx * dx + dy * dy;
                let t = if len > 0.0 { (((c[0] - a.x) * dx + (c[1] - a.y) * dy) / len).clamp(0.0, 1.0) } else { 0.0 };
                (a.x + dx * t - c[0]).hypot(a.y + dy * t - c[1])
            })
            .fold(f64::INFINITY, f64::min)
    };
    (0..areas.len())
        .map(|k| (k, std::iter::once(&areas[k].outer).chain(&areas[k].holes).map(away).fold(f64::INFINITY, f64::min)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map_or(0, |(k, _)| k)
}

/// Whether `b` lies between `a` and `c`, within [`STRAIGHT`] of the line through them.
fn between(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    let (ux, uy) = (c[0] - a[0], c[1] - a[1]);
    let (vx, vy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = ux * ux + uy * uy;
    if len2 == 0.0 {
        return vx.hypot(vy) <= STRAIGHT;
    }
    let t = (vx * ux + vy * uy) / len2;
    0.0 < t && t < 1.0 && (ux * vy - uy * vx).abs() / len2.sqrt() <= STRAIGHT
}

/// A closed ring's vertices without those between their neighbours (`between`): one pass keeping a stack, the last kept
/// dropped while it lies between the one before it and the next; then the closing ones.
fn simplify(pts: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(pts.len());
    for &p in pts {
        while out.len() >= 2 && between(out[out.len() - 2], out[out.len() - 1], p) {
            out.pop();
        }
        out.push(p);
    }
    while out.len() > 3 {
        let n = out.len();
        if between(out[n - 2], out[n - 1], out[0]) {
            out.pop();
        } else if between(out[n - 1], out[0], out[1]) {
            out.remove(0);
        } else {
            break;
        }
    }
    out
}

/// A ring as a zone holds it: its vertices between their neighbours dropped (`simplify`), counterclockwise (`ccw`) or
/// clockwise, from its smallest place (x, then y).
fn ring(r: &Ring, ccw: bool) -> Vec<[f64; 2]> {
    let mut raw: Vec<[f64; 2]> = r.pts.iter().map(|p| [p.x, p.y]).collect();
    if raw.len() > 1 && raw.first() == raw.last() {
        raw.pop();
    }
    let mut pts = simplify(&raw);
    let n = pts.len();
    let twice: f64 = (0..n).map(|k| pts[k][0] * pts[(k + 1) % n][1] - pts[(k + 1) % n][0] * pts[k][1]).sum();
    if (twice > 0.0) != ccw {
        pts.reverse();
    }
    if let Some(k) = (0..n).min_by(|&a, &b| (pts[a][0], pts[a][1]).partial_cmp(&(pts[b][0], pts[b][1])).unwrap_or(std::cmp::Ordering::Equal)) {
        pts.rotate_left(k);
    }
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rectangle's two triangles with heights from `z`.
    fn rect(x0: f64, y0: f64, x1: f64, y1: f64, z: impl Fn(f64, f64) -> f64) -> Vec<Triangle> {
        let p = |x: f64, y: f64| [x, y, z(x, y)];
        vec![[p(x0, y0), p(x1, y0), p(x1, y1)], [p(x0, y0), p(x1, y1), p(x0, y1)]]
    }

    /// A lattice's triangles: corners every `step` from (x0, y0), `n` × `n` cells.
    fn lattice(x0: f64, y0: f64, step: f64, n: usize, z: impl Fn(f64, f64) -> f64) -> Vec<Triangle> {
        let mut out = Vec::new();
        for j in 0..n {
            for i in 0..n {
                let (a, b) = (x0 + step * i as f64, y0 + step * j as f64);
                out.extend(rect(a, b, a + step, b + step, &z));
            }
        }
        out
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * (1.0 + a.abs().max(b.abs()))
    }

    #[test]
    fn a_tilted_plane_over_a_flat_one_is_one_cut_and_one_fill_zone_of_prisms() {
        // z = x − 5 over 0 ≤ x, y ≤ 10: below the flat for x < 5 (a wedge, −12.5·10 m³), above for x > 5.
        let base = rect(0.0, 0.0, 10.0, 10.0, |_, _| 0.0);
        let comparison = rect(0.0, 0.0, 10.0, 10.0, |x, _| x - 5.0);
        let z = cut_fill(&comparison, &base, None, 1000).expect("zones");
        assert_eq!(z.zones.len(), 2);
        let (cut, fill) = (&z.zones[0], &z.zones[1]);
        assert_eq!((cut.band, fill.band), (-1, 0));
        assert!(close(cut.area, 50.0) && close(fill.area, 50.0));
        assert!(close(cut.volume, -125.0) && close(fill.volume, 125.0));
        assert!(close(z.cut, 125.0) && close(z.fill, 125.0));
        assert_eq!(cut.outer, vec![[0.0, 0.0], [5.0, 0.0], [5.0, 10.0], [0.0, 10.0]]);
        assert_eq!(fill.outer, vec![[5.0, 0.0], [10.0, 0.0], [10.0, 10.0], [5.0, 10.0]]);
        // The same totals as the difference's.
        let d = crate::volume::difference(&comparison, &base, &[-10.0, 10.0]);
        assert_eq!((z.cut, z.fill), (d.volumes.cut, d.volumes.fill));
    }

    #[test]
    fn a_mound_s_bands_have_holes_and_corners_apart() {
        // d = 0.4 − 0.1·(|x − 5| + |y − 5|) on a 1 m lattice: the fill a diamond of radius 4 (32 m², 32·0.4/3 m³), the
        // cut the square less it; bands of 0.2 m: two fill zones (one with a hole), the cut's ring and eight corners.
        let base = lattice(0.0, 0.0, 1.0, 10, |_, _| 100.0);
        let mound = |x: f64, y: f64| 100.4 - 0.1 * ((x - 5.0).abs() + (y - 5.0).abs());
        let comparison = lattice(0.0, 0.0, 1.0, 10, mound);
        let two = cut_fill(&comparison, &base, None, 1000).expect("zones");
        assert_eq!(two.zones.len(), 2);
        assert!(close(two.zones[1].area, 32.0) && close(two.zones[1].volume, 32.0 * 0.4 / 3.0));
        assert!(close(two.zones[0].area, 68.0) && two.zones[0].holes.len() == 1);
        assert_eq!(two.zones[1].outer, vec![[1.0, 5.0], [5.0, 1.0], [9.0, 5.0], [5.0, 9.0]]);
        assert!(close(two.zones[0].volume + two.zones[1].volume, -10.0));
        let banded = cut_fill(&comparison, &base, Some(0.2), 1000).expect("zones");
        let count = |k: i64| banded.zones.iter().filter(|z| z.band == k).count();
        assert_eq!((count(-3), count(-2), count(-1), count(0), count(1)), (4, 4, 1, 1, 1));
        assert_eq!(banded.zones.iter().find(|z| z.band == 0).map(|z| z.holes.len()), Some(1));
        let sum: f64 = banded.zones.iter().map(|z| z.volume).sum();
        assert!(close(sum, -10.0), "{sum}");
        let areas: f64 = banded.zones.iter().map(|z| z.area).sum();
        assert!(close(areas, 100.0), "{areas}");
        // Too fine an interval is refused.
        assert_eq!(cut_fill(&comparison, &base, Some(0.0001), 1000), Err(TooManyBands(10_001)));
    }

    #[test]
    fn a_design_that_is_the_ground_beyond_its_work_leaves_no_specks_there() {
        // The ground's 1 m lattice cut the other way: where the design is the ground, the two triangles' planes meet
        // at the cells' middles only up to rounding; a 4 m wide bank (0.1·(2 − |x − 5|)) is the only zone.
        let ground = |x: f64, y: f64| 812.37 + 0.013 * x - 0.021 * y;
        let mut base = Vec::new();
        for j in 0..10 {
            for i in 0..10 {
                let (a, b) = (i as f64, j as f64);
                let p = |x: f64, y: f64| [x, y, ground(x, y)];
                base.push([p(a, b), p(a + 1.0, b), p(a, b + 1.0)]);
                base.push([p(a + 1.0, b), p(a + 1.0, b + 1.0), p(a, b + 1.0)]);
            }
        }
        let bank = |x: f64, y: f64| ground(x, y) + 0.1 * (2.0 - (x - 5.0).abs()).max(0.0);
        let design = lattice(0.0, 0.0, 1.0, 10, bank);
        let z = cut_fill(&design, &base, None, 1000).expect("zones");
        assert_eq!(z.zones.len(), 1, "{:?}", z.zones.iter().map(|z| (z.band, z.area)).collect::<Vec<_>>());
        assert!(close(z.zones[0].area, 40.0) && close(z.zones[0].volume, 0.1 * 2.0 * 10.0 * 2.0 / 2.0 * 2.0 / 2.0 * 2.0));
        assert!(close(z.coincident, 60.0) && z.cut == 0.0);
        assert_eq!(z.zones[0].outer, vec![[3.0, 0.0], [7.0, 0.0], [7.0, 10.0], [3.0, 10.0]]);
    }

    #[test]
    fn a_far_project_s_zones_are_as_near_the_origin() {
        // The mound at TM coordinates (x 512 000, y 4 512 000): the same diamond, its edges' corners left.
        let (x0, y0) = (512_000.0, 4_512_000.0);
        let base = lattice(x0, y0, 1.0, 10, |_, _| 1250.0);
        let mound = move |x: f64, y: f64| 1250.4 - 0.1 * ((x - x0 - 5.0).abs() + (y - y0 - 5.0).abs());
        let z = cut_fill(&lattice(x0, y0, 1.0, 10, mound), &base, None, 1000).expect("zones");
        assert_eq!(z.zones.len(), 2);
        assert_eq!(z.zones[1].outer.len(), 4, "{:?}", z.zones[1].outer);
        for (p, q) in z.zones[1].outer.iter().zip([[1.0, 5.0], [5.0, 1.0], [9.0, 5.0], [5.0, 9.0]]) {
            assert!((p[0] - x0 - q[0]).abs() < 1e-6 && (p[1] - y0 - q[1]).abs() < 1e-6, "{p:?}");
        }
        assert!(close(z.zones[1].area, 32.0) && (z.zones[1].volume - 32.0 * 0.4 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn coinciding_and_apart_surfaces_have_no_zones() {
        let base = rect(0.0, 0.0, 10.0, 10.0, |x, y| x + y);
        let same = cut_fill(&base.clone(), &base, None, 1000).expect("zones");
        assert!(same.zones.is_empty() && close(same.coincident, 100.0));
        let far = rect(100.0, 0.0, 110.0, 10.0, |_, _| 0.0);
        let apart = cut_fill(&far, &base, None, 1000).expect("zones");
        assert!(apart.zones.is_empty() && apart.covered == 0.0 && close(apart.uncovered, 100.0));
    }
}
