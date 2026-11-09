//! Two surfaces' volume by parallel sections (docs/adr/4340; plan/map-cad.md MAP-15; Civil 3D's sample line volumes:
//! Average End Area, Prismoidal): the section method beside the exact TIN–TIN and the grid.
//!
//! The axis points to `azimuth` (degrees from north, clockwise); the sections are the lines across it, the first at the
//! comparison's first place along it, then every `interval`, the last at its last place. On a section both surfaces are
//! planes triangle by triangle, so the difference `d = z_comparison − z_base` is linear between the places where the
//! section crosses an edge of either: each piece where both are is integrated exactly, split where `d` is zero, the
//! fill where `d > 0`, the cut where `d < 0` (m²). Between two consecutive sections the volume is the average end area
//! `L·(A₀ + A₁)/2` or the prismoidal `L/6·(A₀ + 4·Aₘ + A₁)` with a section at the middle (`volume::prismoidal`).
//!
//! The places are measured from the comparison's first corner, so large map coordinates keep their digits; the four
//! quarters' axes are exact (0°, 90°, 180°, 270°).

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
    prismoidal: bool,
    most: f64,
) -> Result<Sections, SectionsError> {
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
            let s = measure(&comparison, &base, az, 10.0, false, 1e4).expect("measured");
            assert!((s.fill - 2400.0).abs() < 1e-9, "{az}: {}", s.fill);
            assert_eq!(s.cut, 0.0);
        }
        let s = measure(&comparison, &base, 90.0, 10.0, false, 1e4).expect("measured");
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
        let s = measure(&comparison, &base, 90.0, 10.0, true, 1e4).expect("measured");
        assert!((s.fill - exact).abs() < 1e-9, "{}", s.fill);
        // A(x) = 30(1 − x/40)(1 + 0.05x): A″ = −2·30·0.05/40 = −0.075; four stretches of 10 m: 4·10³/12·0.075 = 25.
        let e = measure(&comparison, &base, 90.0, 10.0, false, 1e4).expect("measured");
        assert!((e.fill - (exact - 25.0)).abs() < 1e-9, "{}", e.fill);
    }

    /// A difference changing sign along a section is split at its zero: cut and fill apart.
    #[test]
    fn a_section_is_split_where_the_difference_is_zero() {
        let base = rect(0.0, 0.0, 10.0, 10.0, |_, _| 100.0);
        let comparison = rect(0.0, 0.0, 10.0, 10.0, |x, _| 99.0 + 0.2 * x);
        let s = measure(&comparison, &base, 0.0, 5.0, false, 1e4).expect("measured");
        // Along y the area is constant: the cut 2.5 m² (x 0..5), the fill 2.5 m²; volumes 25 and 25.
        for r in &s.rows {
            assert!((r.cut - 2.5).abs() < 1e-12 && (r.fill - 2.5).abs() < 1e-12, "{r:?}");
        }
        assert!((s.cut - 25.0).abs() < 1e-9 && (s.fill - 25.0).abs() < 1e-9);
    }

    #[test]
    fn the_refusals() {
        let t = rect(0.0, 0.0, 10.0, 10.0, |_, _| 1.0);
        assert_eq!(measure(&t, &t, 0.0, 0.0, false, 1e4), Err(SectionsError::Interval));
        assert_eq!(measure(&t, &t, 360.0, 1.0, false, 1e4), Err(SectionsError::Azimuth));
        assert_eq!(measure(&t, &t, 0.0, 0.001, false, 1e4), Err(SectionsError::TooMany));
    }
}
