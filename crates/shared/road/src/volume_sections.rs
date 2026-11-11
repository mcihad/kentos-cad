//! Two surfaces' volume by parallel sections (docs/adr/4340; plan/map-cad.md MAP-15; Civil 3D's sample line volumes:
//! Average End Area, Prismoidal): the section method beside the exact TIN–TIN and the grid.
//!
//! The axis points to `azimuth` (degrees from north, clockwise); the sections are the lines across it, the first at the
//! comparison's first place along it, then every `interval`, the last at its last place. On a section both surfaces are
//! planes triangle by triangle, so the difference `d = z_comparison − z_base` is linear between the places where the
//! section crosses an edge of either: each piece where both are is integrated exactly, split where `d` is zero, the
//! fill where `d > 0`, the cut where `d < 0` (m²). Between two consecutive sections the volume is the average end area
//! `L·(A₀ + A₁)/2`, the prismoidal `L/6·(A₀ + 4·Aₘ + A₁)` with a section at the middle (`volume::prismoidal`) or
//! (docs/adr/5140) the frustum's `L/3·(A₀ + A₁ + √(A₀·A₁))`, cut and fill each on its own.
//!
//! The places are measured from the comparison's first corner, so large map coordinates keep their digits; the four
//! quarters' axes are exact (0°, 90°, 180°, 270°).
//!
//! Along a drawn axis (docs/adr/4760; Civil 3D's sample lines, Netcad's Enkesit Al): the axis is a line or an open
//! polyline whose segments may be arcs (a bulge: the tangent of a quarter of the arc's angle, counterclockwise above 0).
//! The sections stand at its stations, from its start every `interval` while short of its end, then its end, and asked
//! for at its inner corners too. Each section is square to the axis there (on an arc along its radius; at a corner
//! square to the bisector of the two segments, at the ends to their segment) and runs `left` metres to the left and
//! `right` to the right of it; its areas are reckoned as above on that segment of the line. Between two sections the
//! length is their stations' difference along the axis (no correction for the axis's curvature).

use crate::volume::{self, SectionAreas, Triangle};

/// Why no sections are measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionsError {
    /// The interval is not a length above zero.
    Interval,
    /// The azimuth is not in [0, 360).
    Azimuth,
    /// More sections than allowed.
    TooMany,
    /// A width is not a length of zero or more, or both are zero.
    Width,
    /// The axis has no length, or a bulge is not finite.
    Axis,
}

/// How the volume between two sections is reckoned (docs/adr/4340, 5140).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formula {
    EndArea,
    Prismoidal,
    Frustum,
}

/// The frustum's volume between two sections of areas `a0` and `a1`, `l` apart: `l/3·(a0 + a1 + √(a0·a1))`.
pub fn frustum(a0: f64, a1: f64, l: f64) -> f64 {
    l / 3.0 * (a0 + a1 + (a0 * a1).sqrt())
}

/// A drawn axis (docs/adr/4760): its corners in order and each segment's bulge (none: all straight).
#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    pub pts: Vec<[f64; 2]>,
    pub bulges: Vec<f64>,
}

/// A section along a drawn axis: its station (metres from the axis's start along it), the axis's place there, the
/// section's left and right ends, and its cut and fill areas (m²).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisRow {
    pub station: f64,
    pub at: [f64; 2],
    pub left: [f64; 2],
    pub right: [f64; 2],
    pub cut: f64,
    pub fill: f64,
}

/// The sections along a drawn axis, the cut and fill volume between each two in order (m³), their sums and the axis's
/// length.
#[derive(Clone, Debug, PartialEq)]
pub struct AxisSections {
    pub rows: Vec<AxisRow>,
    pub between: Vec<[f64; 2]>,
    pub cut: f64,
    pub fill: f64,
    pub length: f64,
}

/// A segment of the axis: straight from `a` to `b`, or an arc of its centre, radius, start angle and signed sweep.
#[derive(Clone, Copy, Debug)]
enum Segment {
    Line { a: [f64; 2], b: [f64; 2], len: f64 },
    Arc { c: [f64; 2], r: f64, start: f64, sweep: f64, len: f64 },
}

impl Segment {
    fn of(a: [f64; 2], b: [f64; 2], bulge: f64) -> Option<Segment> {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let chord = dx.hypot(dy);
        if !(chord > 0.0) {
            return None;
        }
        if bulge == 0.0 {
            return Some(Segment::Line { a, b, len: chord });
        }
        // The arc's angle, counterclockwise above 0; its centre left of the chord for a counterclockwise arc.
        let sweep = 4.0 * bulge.atan();
        let r = chord / (2.0 * (sweep / 2.0).sin()).abs();
        let m = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
        let along = (chord / 2.0) / (sweep / 2.0).tan();
        let left = [-dy / chord, dx / chord];
        let c = [m[0] + left[0] * along, m[1] + left[1] * along];
        let start = (a[1] - c[1]).atan2(a[0] - c[0]);
        Some(Segment::Arc { c, r, start, sweep, len: r * sweep.abs() })
    }

    fn len(&self) -> f64 {
        match self {
            Segment::Line { len, .. } | Segment::Arc { len, .. } => *len,
        }
    }

    /// The place and the unit direction `l` metres from the segment's start.
    fn at(&self, l: f64) -> ([f64; 2], [f64; 2]) {
        match *self {
            Segment::Line { a, b, len } => {
                let u = [(b[0] - a[0]) / len, (b[1] - a[1]) / len];
                if l == len {
                    return (b, u);
                }
                ([a[0] + u[0] * l, a[1] + u[1] * l], u)
            }
            Segment::Arc { c, r, start, sweep, .. } => {
                let turn = sweep.signum();
                let angle = start + turn * l / r;
                let p = [c[0] + r * angle.cos(), c[1] + r * angle.sin()];
                // Counterclockwise the way is the radius turned left; clockwise turned right.
                (p, [-turn * angle.sin(), turn * angle.cos()])
            }
        }
    }
}

/// The unit vector of `v`, or none for a zero vector.
fn unit(v: [f64; 2]) -> Option<[f64; 2]> {
    let n = v[0].hypot(v[1]);
    (n > 0.0).then(|| [v[0] / n, v[1] / n])
}

/// The axis's segments (zero-length ones left out) and each one's start station.
fn segments(axis: &Axis) -> Result<(Vec<Segment>, Vec<f64>), SectionsError> {
    if !axis.bulges.is_empty() && axis.bulges.len() + 1 != axis.pts.len() || axis.bulges.iter().any(|b| !b.is_finite()) {
        return Err(SectionsError::Axis);
    }
    let mut out = Vec::new();
    let mut starts = Vec::new();
    let mut s = 0.0;
    for (k, w) in axis.pts.windows(2).enumerate() {
        if let Some(seg) = Segment::of(w[0], w[1], axis.bulges.get(k).copied().unwrap_or(0.0)) {
            starts.push(s);
            s += seg.len();
            out.push(seg);
        }
    }
    if out.is_empty() || !(s.is_finite() && s > 0.0) {
        return Err(SectionsError::Axis);
    }
    Ok((out, starts))
}

/// The place and the unit way of the axis at station `s` (within its length): inside a segment its own; at an inner
/// corner the bisector of the two segments' ways (the later one's when they turn back); at the ends their segments'.
fn place(segs: &[Segment], starts: &[f64], s: f64) -> ([f64; 2], [f64; 2]) {
    let k = starts.partition_point(|x| *x <= s).saturating_sub(1).min(segs.len() - 1);
    let (p, u) = segs[k].at((s - starts[k]).min(segs[k].len()));
    if k > 0 && s == starts[k] {
        let (_, before) = segs[k - 1].at(segs[k - 1].len());
        return (p, unit([before[0] + u[0], before[1] + u[1]]).unwrap_or(u));
    }
    (p, u)
}

/// A section: its distance from the first along the axis, its cut and fill areas (m²).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub station: f64,
    pub cut: f64,
    pub fill: f64,
}

/// The sections and the volumes between them (m³).
#[derive(Clone, Debug, PartialEq)]
pub struct Sections {
    pub rows: Vec<Row>,
    pub cut: f64,
    pub fill: f64,
}

/// The axis of an azimuth (degrees from north, clockwise) as an east, north unit vector; the quarters exact.
pub fn axis(azimuth: f64) -> [f64; 2] {
    match azimuth {
        a if a == 0.0 => [0.0, 1.0],
        a if a == 90.0 => [1.0, 0.0],
        a if a == 180.0 => [0.0, -1.0],
        a if a == 270.0 => [-1.0, 0.0],
        a => {
            let r = a.to_radians();
            [r.sin(), r.cos()]
        }
    }
}

/// A surface's piece along a section: from `t0` to `t1` across it, its height linear from `z0` to `z1`.
#[derive(Clone, Copy, Debug)]
struct Piece {
    t0: f64,
    z0: f64,
    t1: f64,
    z1: f64,
}

impl Piece {
    fn z(&self, t: f64) -> f64 {
        if t == self.t0 {
            self.z0
        } else if t == self.t1 {
            self.z1
        } else {
            self.z0 + (t - self.t0) / (self.t1 - self.t0) * (self.z1 - self.z0)
        }
    }
}

/// A triangle in the axis's frame: each corner's place along the axis (s), across it (t) and height.
type Local = [[f64; 3]; 3];

fn local(tris: &[Triangle], o: [f64; 2], u: [f64; 2]) -> Vec<Local> {
    tris.iter()
        .map(|t| {
            t.map(|p| {
                let (x, y) = (p[0] - o[0], p[1] - o[1]);
                [x * u[0] + y * u[1], x * u[1] - y * u[0], p[2]]
            })
        })
        .collect()
}

/// Where the section at `s` crosses the triangle: the piece between its two farthest places across it.
fn cut_by(tri: &Local, s: f64) -> Option<Piece> {
    let mut lo: Option<(f64, f64)> = None;
    let mut hi: Option<(f64, f64)> = None;
    let mut put = |t: f64, z: f64| {
        if lo.is_none_or(|(a, _)| t < a) {
            lo = Some((t, z));
        }
        if hi.is_none_or(|(b, _)| t > b) {
            hi = Some((t, z));
        }
    };
    for k in 0..3 {
        let (a, b) = (tri[k], tri[(k + 1) % 3]);
        if a[0] == s {
            put(a[1], a[2]);
        }
        if (a[0] - s) * (b[0] - s) < 0.0 {
            let f = (s - a[0]) / (b[0] - a[0]);
            put(a[1] + f * (b[1] - a[1]), a[2] + f * (b[2] - a[2]));
        }
    }
    let ((t0, z0), (t1, z1)) = (lo?, hi?);
    (t1 > t0).then_some(Piece { t0, z0, t1, z1 })
}

/// A surface's pieces along a section, in order, none over another (a section along an edge two triangles share meets
/// both: the later is cut back to where the earlier ends; their heights agree there).
fn along(tris: &[Local], which: &[u32], s: f64) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = which.iter().filter_map(|&i| cut_by(&tris[i as usize], s)).collect();
    pieces.sort_by(|a, b| a.t0.total_cmp(&b.t0));
    let mut out: Vec<Piece> = Vec::with_capacity(pieces.len());
    for mut p in pieces {
        if let Some(last) = out.last()
            && p.t0 < last.t1
        {
            if p.t1 <= last.t1 {
                continue;
            }
            p = Piece { t0: last.t1, z0: p.z(last.t1), t1: p.t1, z1: p.z1 };
        }
        out.push(p);
    }
    out
}

/// The cut and fill areas of the difference `d` linear from `d0` to `d1` over `len`.
fn integrate(d0: f64, d1: f64, len: f64) -> (f64, f64) {
    if d0 >= 0.0 && d1 >= 0.0 {
        (0.0, (d0 + d1) / 2.0 * len)
    } else if d0 <= 0.0 && d1 <= 0.0 {
        (-(d0 + d1) / 2.0 * len, 0.0)
    } else {
        let f = d0 / (d0 - d1);
        let (first, second) = (d0 * f * len / 2.0, d1 * (1.0 - f) * len / 2.0);
        if d0 > 0.0 { (-second, first) } else { (-first, second) }
    }
}

/// The cut and fill areas of a section where both surfaces are.
fn areas(comparison: &[Piece], base: &[Piece]) -> (f64, f64) {
    let (mut cut, mut fill) = (0.0, 0.0);
    let (mut i, mut j) = (0, 0);
    while i < comparison.len() && j < base.len() {
        let (c, b) = (comparison[i], base[j]);
        let (lo, hi) = (c.t0.max(b.t0), c.t1.min(b.t1));
        if hi > lo {
            let (k, f) = integrate(c.z(lo) - b.z(lo), c.z(hi) - b.z(hi), hi - lo);
            cut += k;
            fill += f;
        }
        if c.t1 <= b.t1 {
            i += 1;
        } else {
            j += 1;
        }
    }
    (cut, fill)
}

/// Each section's triangles: the triangles whose run along the axis holds its place.
fn buckets(tris: &[Local], stations: &[f64]) -> Vec<Vec<u32>> {
    let mut out = vec![Vec::new(); stations.len()];
    for (i, t) in tris.iter().enumerate() {
        let lo = t[0][0].min(t[1][0]).min(t[2][0]);
        let hi = t[0][0].max(t[1][0]).max(t[2][0]);
        let mut k = stations.partition_point(|s| *s < lo);
        while k < stations.len() && stations[k] <= hi {
            out[k].push(i as u32);
            k += 1;
        }
    }
    out
}

/// The comparison's volume over the base by sections (the module's rules); `most` sections at most.
pub fn measure(
    comparison: &[Triangle],
    base: &[Triangle],
    azimuth: f64,
    interval: f64,
    formula: Formula,
    most: f64,
) -> Result<Sections, SectionsError> {
    let prismoidal = formula == Formula::Prismoidal;
    if !(interval.is_finite() && interval > 0.0) {
        return Err(SectionsError::Interval);
    }
    if !(azimuth.is_finite() && (0.0..360.0).contains(&azimuth)) {
        return Err(SectionsError::Azimuth);
    }
    let Some(first) = comparison.first() else {
        return Ok(Sections { rows: Vec::new(), cut: 0.0, fill: 0.0 });
    };
    let u = axis(azimuth);
    let o = [first[0][0], first[0][1]];
    let (c, b) = (local(comparison, o, u), local(base, o, u));
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in c.iter().flatten() {
        lo = lo.min(p[0]);
        hi = hi.max(p[0]);
    }
    let span = hi - lo;
    if span / interval + 2.0 > most {
        return Err(SectionsError::TooMany);
    }
    // The sections' distances from the first: every interval while short of the last place, then the last.
    let mut rel: Vec<f64> = Vec::new();
    let mut k = 0u32;
    while f64::from(k) * interval < span {
        rel.push(f64::from(k) * interval);
        k += 1;
    }
    rel.push(span);
    // The prismoidal formula's middle sections, between each two.
    let middles: Vec<f64> = if prismoidal { rel.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect() } else { Vec::new() };
    let mut places: Vec<f64> = rel.iter().chain(&middles).map(|r| lo + r).collect();
    places.sort_by(f64::total_cmp);
    places.dedup();
    let (bc, bb) = (buckets(&c, &places), buckets(&b, &places));
    let area_at = |s: f64| -> (f64, f64) {
        let k = places.partition_point(|p| *p < s);
        areas(&along(&c, &bc[k], s), &along(&b, &bb[k], s))
    };
    let rows: Vec<Row> = rel
        .iter()
        .map(|r| {
            let (cut, fill) = area_at(lo + r);
            Row { station: *r, cut, fill }
        })
        .collect();
    let (mut cut, mut fill) = (0.0, 0.0);
    if prismoidal {
        for (w, m) in rows.windows(2).zip(&middles) {
            let (mc, mf) = area_at(lo + m);
            let l = w[1].station - w[0].station;
            cut += volume::prismoidal(w[0].cut, mc, w[1].cut, l);
            fill += volume::prismoidal(w[0].fill, mf, w[1].fill, l);
        }
    } else if formula == Formula::Frustum {
        for w in rows.windows(2) {
            let l = w[1].station - w[0].station;
            cut += frustum(w[0].cut, w[1].cut, l);
            fill += frustum(w[0].fill, w[1].fill, l);
        }
    } else {
        let sections: Vec<SectionAreas> =
            rows.iter().map(|r| SectionAreas { station: r.station, areas: Some((r.cut, r.fill)) }).collect();
        for between in volume::end_areas(&sections) {
            if let Some((k, f)) = between.volumes {
                cut += k;
                fill += f;
            }
        }
    }
    Ok(Sections { rows, cut, fill })
}

/// The pieces cut back to `lo..hi` across the section.
fn clip(pieces: Vec<Piece>, lo: f64, hi: f64) -> Vec<Piece> {
    pieces
        .into_iter()
        .filter_map(|p| {
            let (t0, t1) = (p.t0.max(lo), p.t1.min(hi));
            (t1 > t0).then(|| Piece { t0, z0: p.z(t0), t1, z1: p.z(t1) })
        })
        .collect()
}

/// The cut and fill areas of the section through `o` square to the way `u`, from `left` metres left of it to `right`
/// metres right: the triangles whose box meets the section's, in the section's frame.
fn section_at(comparison: &[Triangle], base: &[Triangle], o: [f64; 2], u: [f64; 2], left: f64, right: f64) -> (f64, f64) {
    let n = [u[1], -u[0]];
    let (a, b) = ([o[0] - n[0] * left, o[1] - n[1] * left], [o[0] + n[0] * right, o[1] + n[1] * right]);
    let (x0, x1, y0, y1) = (a[0].min(b[0]), a[0].max(b[0]), a[1].min(b[1]), a[1].max(b[1]));
    let near = |tris: &[Triangle]| -> Vec<Triangle> {
        tris.iter()
            .filter(|t| {
                let (tx0, tx1) = (t[0][0].min(t[1][0]).min(t[2][0]), t[0][0].max(t[1][0]).max(t[2][0]));
                let (ty0, ty1) = (t[0][1].min(t[1][1]).min(t[2][1]), t[0][1].max(t[1][1]).max(t[2][1]));
                tx1 >= x0 && tx0 <= x1 && ty1 >= y0 && ty0 <= y1
            })
            .copied()
            .collect()
    };
    let (c, bl) = (local(&near(comparison), o, u), local(&near(base), o, u));
    let all = |l: &[Local]| (0..l.len() as u32).collect::<Vec<u32>>();
    let (pc, pb) = (clip(along(&c, &all(&c), 0.0), -left, right), clip(along(&bl, &all(&bl), 0.0), -left, right));
    areas(&pc, &pb)
}

/// The comparison's volume over the base by sections along a drawn axis (the module's rules); `most` sections at most.
#[allow(clippy::too_many_arguments)]
pub fn along_axis(
    comparison: &[Triangle],
    base: &[Triangle],
    axis: &Axis,
    interval: f64,
    left: f64,
    right: f64,
    corners: bool,
    formula: Formula,
    most: f64,
) -> Result<AxisSections, SectionsError> {
    if !(interval.is_finite() && interval > 0.0) {
        return Err(SectionsError::Interval);
    }
    if !(left.is_finite() && right.is_finite() && left >= 0.0 && right >= 0.0 && left + right > 0.0) {
        return Err(SectionsError::Width);
    }
    let (segs, starts) = segments(axis)?;
    let length = starts.last().copied().unwrap_or(0.0) + segs.last().map_or(0.0, Segment::len);
    let inner = if corners { starts.len().saturating_sub(1) } else { 0 };
    if length / interval + 2.0 + inner as f64 > most {
        return Err(SectionsError::TooMany);
    }
    // The stations: every interval while short of the end, then the end; asked for, the inner corners too.
    let mut stations: Vec<f64> = Vec::new();
    let mut k = 0u32;
    while f64::from(k) * interval < length {
        stations.push(f64::from(k) * interval);
        k += 1;
    }
    stations.push(length);
    if corners {
        stations.extend(starts.iter().skip(1).copied());
    }
    stations.sort_by(f64::total_cmp);
    stations.dedup();
    let section = |s: f64| -> AxisRow {
        let (at, u) = place(&segs, &starts, s);
        let n = [u[1], -u[0]];
        let (cut, fill) = section_at(comparison, base, at, u, left, right);
        AxisRow {
            station: s,
            at,
            left: [at[0] - n[0] * left, at[1] - n[1] * left],
            right: [at[0] + n[0] * right, at[1] + n[1] * right],
            cut,
            fill,
        }
    };
    let rows: Vec<AxisRow> = stations.iter().map(|s| section(*s)).collect();
    let mut between: Vec<[f64; 2]> = Vec::with_capacity(rows.len().saturating_sub(1));
    if formula == Formula::Frustum {
        for w in rows.windows(2) {
            let l = w[1].station - w[0].station;
            between.push([frustum(w[0].cut, w[1].cut, l), frustum(w[0].fill, w[1].fill, l)]);
        }
    } else if formula == Formula::Prismoidal {
        for w in rows.windows(2) {
            let m = section((w[0].station + w[1].station) / 2.0);
            let l = w[1].station - w[0].station;
            between.push([volume::prismoidal(w[0].cut, m.cut, w[1].cut, l), volume::prismoidal(w[0].fill, m.fill, w[1].fill, l)]);
        }
    } else {
        let sections: Vec<SectionAreas> =
            rows.iter().map(|r| SectionAreas { station: r.station, areas: Some((r.cut, r.fill)) }).collect();
        for b in volume::end_areas(&sections) {
            between.push(b.volumes.map_or([0.0, 0.0], |(k, f)| [k, f]));
        }
    }
    let (mut cut, mut fill) = (0.0, 0.0);
    for [k, f] in &between {
        cut += k;
        fill += f;
    }
    Ok(AxisSections { rows, between, cut, fill, length })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two triangles covering the rectangle (x0, y0)–(x1, y1), heights by `z`.
    fn rect(x0: f64, y0: f64, x1: f64, y1: f64, z: impl Fn(f64, f64) -> f64) -> Vec<Triangle> {
        let p = |x: f64, y: f64| [x, y, z(x, y)];
        vec![[p(x0, y0), p(x1, y0), p(x1, y1)], [p(x0, y0), p(x1, y1), p(x0, y1)]]
    }

    /// A plane difference over a rectangle: the area varies linearly along either axis, so the average end area is the
    /// exact volume, 30·∫₀⁴⁰(1 + 0.05x)dx = 2400 m³, whichever the direction.
    #[test]
    fn a_plane_over_a_rectangle_is_exact() {
        let base = rect(-5.0, -5.0, 45.0, 35.0, |_, _| 100.0);
        let comparison = rect(0.0, 0.0, 40.0, 30.0, |x, _| 101.0 + 0.05 * x);
        for az in [0.0, 90.0, 180.0, 270.0] {
            let s = measure(&comparison, &base, az, 10.0, Formula::EndArea, 1e4).expect("measured");
            assert!((s.fill - 2400.0).abs() < 1e-9, "{az}: {}", s.fill);
            assert_eq!(s.cut, 0.0);
        }
        let s = measure(&comparison, &base, 90.0, 10.0, Formula::EndArea, 1e4).expect("measured");
        let stations: Vec<f64> = s.rows.iter().map(|r| r.station).collect();
        assert_eq!(stations, [0.0, 10.0, 20.0, 30.0, 40.0]);
        assert!((s.rows[0].fill - 30.0).abs() < 1e-12 && (s.rows[4].fill - 90.0).abs() < 1e-12);
    }

    /// A triangular plan under a plane difference: the area along x is quadratic, so the average end area misses by
    /// L³/12·|A″| a stretch (under, A being concave) and the prismoidal formula is exact: ∫∫ (1 + 0.05x) over the
    /// triangle (0,0), (40,0), (0,30) is 600 + 0.05·(40²·30/6) = 1000 m³.
    #[test]
    fn a_triangle_s_quadratic_area_is_prismoidal() {
        let base = rect(-5.0, -5.0, 45.0, 35.0, |_, _| 0.0);
        let p = |x: f64, y: f64| [x, y, 1.0 + 0.05 * x];
        let comparison = vec![[p(0.0, 0.0), p(40.0, 0.0), p(0.0, 30.0)]];
        let exact = 1000.0;
        let s = measure(&comparison, &base, 90.0, 10.0, Formula::Prismoidal, 1e4).expect("measured");
        assert!((s.fill - exact).abs() < 1e-9, "{}", s.fill);
        // A(x) = 30(1 − x/40)(1 + 0.05x): A″ = −2·30·0.05/40 = −0.075; four stretches of 10 m: 4·10³/12·0.075 = 25.
        let e = measure(&comparison, &base, 90.0, 10.0, Formula::EndArea, 1e4).expect("measured");
        assert!((e.fill - (exact - 25.0)).abs() < 1e-9, "{}", e.fill);
    }

    /// A difference changing sign along a section is split at its zero: cut and fill apart.
    #[test]
    fn a_section_is_split_where_the_difference_is_zero() {
        let base = rect(0.0, 0.0, 10.0, 10.0, |_, _| 100.0);
        let comparison = rect(0.0, 0.0, 10.0, 10.0, |x, _| 99.0 + 0.2 * x);
        let s = measure(&comparison, &base, 0.0, 5.0, Formula::EndArea, 1e4).expect("measured");
        // Along y the area is constant: the cut 2.5 m² (x 0..5), the fill 2.5 m²; volumes 25 and 25.
        for r in &s.rows {
            assert!((r.cut - 2.5).abs() < 1e-12 && (r.fill - 2.5).abs() < 1e-12, "{r:?}");
        }
        assert!((s.cut - 25.0).abs() < 1e-9 && (s.fill - 25.0).abs() < 1e-9);
    }

    /// A straight axis gives the parallel sections' answer: the plane over the rectangle along x (an axis from (0, 15)
    /// to (40, 15), 20 m each side) is 2400 m³ whatever the axis, its areas 30·(1 + 0.05x).
    #[test]
    fn a_straight_axis_is_the_parallel_sections() {
        let base = rect(-5.0, -5.0, 45.0, 35.0, |_, _| 100.0);
        let comparison = rect(0.0, 0.0, 40.0, 30.0, |x, _| 101.0 + 0.05 * x);
        let axis = Axis { pts: vec![[0.0, 15.0], [40.0, 15.0]], bulges: Vec::new() };
        let s = along_axis(&comparison, &base, &axis, 10.0, 20.0, 20.0, false, Formula::EndArea, 1e4).expect("measured");
        assert!((s.fill - 2400.0).abs() < 1e-9, "{}", s.fill);
        assert_eq!(s.rows.iter().map(|r| r.station).collect::<Vec<_>>(), [0.0, 10.0, 20.0, 30.0, 40.0]);
        assert!((s.rows[2].fill - 60.0).abs() < 1e-12, "{:?}", s.rows[2]);
        // Square to an eastward axis: the left end north, the right end south.
        assert_eq!((s.rows[1].left, s.rows[1].right), ([10.0, 35.0], [10.0, -5.0]));
        // Narrower than the surfaces: only the width counts, 10 m each side.
        let n = along_axis(&comparison, &base, &axis, 10.0, 10.0, 10.0, false, Formula::EndArea, 1e4).expect("measured");
        assert!((n.fill - 2400.0 * 20.0 / 30.0).abs() < 1e-9, "{}", n.fill);
    }

    /// On an arc the sections run along its radius: a ring of radii 10 and 30 round (0, 0) a little more than half
    /// round, one metre of fill, measured along the middle arc (radius 20) from (20, 0) to (−20, 0) counterclockwise,
    /// 10 m each side: every section's fill is 20 m², the axis 20π long, the volume 20·20π (the half ring's area
    /// ½π(30² − 10²) = 400π, a metre high, is the same: on a circle the middle radius's length is exact).
    #[test]
    fn on_an_arc_the_sections_follow_the_radius() {
        let p = |r: f64, a: f64| [r * a.cos(), r * a.sin(), 1.0];
        let mut comparison = Vec::new();
        let n = 720;
        let (from, span) = (-0.1, std::f64::consts::PI + 0.2);
        for k in 0..n {
            let (a0, a1) = (from + span * k as f64 / n as f64, from + span * (k + 1) as f64 / n as f64);
            comparison.push([p(10.0, a0), p(30.0, a0), p(30.0, a1)]);
            comparison.push([p(10.0, a0), p(30.0, a1), p(10.0, a1)]);
        }
        let base = rect(-40.0, -15.0, 40.0, 40.0, |_, _| 0.0);
        let axis = Axis { pts: vec![[20.0, 0.0], [-20.0, 0.0]], bulges: vec![1.0] };
        let s = along_axis(&comparison, &base, &axis, 5.0, 10.0, 10.0, false, Formula::EndArea, 1e5).expect("measured");
        assert!((s.length - 20.0 * std::f64::consts::PI).abs() < 1e-9, "{}", s.length);
        for r in &s.rows {
            // The polygonal ring's chords leave a sliver at its edges: within 0.002 m².
            assert!((r.fill - 20.0).abs() < 2e-3, "{r:?}");
            let radius = r.at[0].hypot(r.at[1]);
            assert!((radius - 20.0).abs() < 1e-9, "{r:?}");
            // The left end towards the centre (counterclockwise), the right end out.
            assert!((r.left[0].hypot(r.left[1]) - 10.0).abs() < 1e-9 && (r.right[0].hypot(r.right[1]) - 30.0).abs() < 1e-9, "{r:?}");
        }
        assert!((s.fill - 400.0 * std::f64::consts::PI).abs() < 0.2, "{}", s.fill);
    }

    /// At an inner corner asked for, the section stands square to the bisector.
    #[test]
    fn a_corner_s_section_is_square_to_the_bisector() {
        let base = rect(-50.0, -50.0, 50.0, 50.0, |_, _| 0.0);
        let comparison = rect(-50.0, -50.0, 50.0, 50.0, |_, _| 1.0);
        let axis = Axis { pts: vec![[-30.0, 0.0], [0.0, 0.0], [0.0, 30.0]], bulges: Vec::new() };
        let s = along_axis(&comparison, &base, &axis, 100.0, 5.0, 5.0, true, Formula::EndArea, 1e4).expect("measured");
        assert_eq!(s.rows.iter().map(|r| r.station).collect::<Vec<_>>(), [0.0, 30.0, 60.0]);
        let c = s.rows[1];
        let h = 5.0 / 2f64.sqrt();
        assert!((c.left[0] + h).abs() < 1e-12 && (c.left[1] - h).abs() < 1e-12, "{c:?}");
        assert!((c.fill - 10.0).abs() < 1e-12, "{c:?}");
    }

    #[test]
    fn the_refusals() {
        let t = rect(0.0, 0.0, 10.0, 10.0, |_, _| 1.0);
        assert_eq!(measure(&t, &t, 0.0, 0.0, Formula::EndArea, 1e4), Err(SectionsError::Interval));
        assert_eq!(measure(&t, &t, 360.0, 1.0, Formula::EndArea, 1e4), Err(SectionsError::Azimuth));
        assert_eq!(measure(&t, &t, 0.0, 0.001, Formula::EndArea, 1e4), Err(SectionsError::TooMany));
        let a = Axis { pts: vec![[0.0, 0.0], [10.0, 0.0]], bulges: Vec::new() };
        assert_eq!(along_axis(&t, &t, &a, 1.0, 0.0, 0.0, false, Formula::EndArea, 1e4), Err(SectionsError::Width));
        assert_eq!(along_axis(&t, &t, &a, 1.0, -1.0, 2.0, false, Formula::EndArea, 1e4), Err(SectionsError::Width));
        let dot = Axis { pts: vec![[1.0, 1.0], [1.0, 1.0]], bulges: Vec::new() };
        assert_eq!(along_axis(&t, &t, &dot, 1.0, 1.0, 1.0, false, Formula::EndArea, 1e4), Err(SectionsError::Axis));
        assert_eq!(along_axis(&t, &t, &a, 0.0001, 1.0, 1.0, false, Formula::EndArea, 1e4), Err(SectionsError::TooMany));
    }

    /// The frustum's formula is exact for a pyramid's frustum (areas a² and b², height h: h/3·(a² + b² + ab)) and is
    /// the end area for equal areas.
    #[test]
    fn the_frustum_is_a_pyramid_s_frustum() {
        let (a, b, h) = (2.0_f64, 5.0_f64, 7.0_f64);
        assert!((frustum(a * a, b * b, h) - h / 3.0 * (a * a + b * b + a * b)).abs() < 1e-12);
        assert_eq!(frustum(4.0, 4.0, 3.0), 12.0);
        assert_eq!(frustum(0.0, 9.0, 3.0), 9.0);
    }
}
