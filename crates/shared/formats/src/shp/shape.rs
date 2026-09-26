//! The .shp main file (ESRI Shapefile Technical Description, July 1998):
//! a 100-byte header, then records of one shape each. Every length is
//! checked against the bytes there are, so no file can make the reader
//! read past its end or allocate what it does not hold.
//!
//! Points keep their height (PointZ, MultiPointZ); lines and areas are
//! plane in the app, so their Z is dropped, and M (measure) values always
//! are; both are said. An area's rings are sorted as the specification
//! defines them: clockwise rings are outlines, counter-clockwise ones holes
//! of the first outline that contains them (the rules, which the
//! independent reader in tools/formats/gis.py follows too, are in
//! docs/adr/0046).

use kentos_contracts::Vec2;

use crate::gis::{Shape, open_ring, shoelace};

/// The main file's header.
pub struct Header {
    pub shape_type: i32,
    pub version: i32,
}

/// What reading the records found that the report says (per file).
#[derive(Default)]
pub struct Found {
    /// Records whose lines or areas had heights (dropped).
    pub z_dropped: u32,
    /// Records with M values (dropped).
    pub m: u32,
    /// Records shorter than their shape needs.
    pub short: u32,
    /// Records whose parts are out of order or out of range.
    pub bad_parts: u32,
    /// Points, parts or areas with a coordinate that is not a finite number.
    pub not_finite: u32,
    /// Lines of fewer than two points; rings of fewer than three or of no area.
    pub short_parts: u32,
    pub short_rings: u32,
    /// Holes no outline contains, read as areas of their own.
    pub lone_holes: u32,
    /// Area records without a clockwise ring, every ring read as an outline.
    pub no_outline: u32,
    /// MultiPatch records (3D surfaces), and records of unknown types.
    pub multipatch: u32,
    pub unknown: u32,
}

pub fn type_name(t: i32) -> String {
    match t {
        0 => "Null".into(),
        1 => "Point".into(),
        3 => "PolyLine".into(),
        5 => "Polygon".into(),
        8 => "MultiPoint".into(),
        11 => "PointZ".into(),
        13 => "PolyLineZ".into(),
        15 => "PolygonZ".into(),
        18 => "MultiPointZ".into(),
        21 => "PointM".into(),
        23 => "PolyLineM".into(),
        25 => "PolygonM".into(),
        28 => "MultiPointM".into(),
        31 => "MultiPatch".into(),
        other => format!("bilinmeyen tür {other}"),
    }
}

fn i32_be(b: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn i32_le(b: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn f64_le(b: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// Reads the header of a .shp (or .shx) file.
pub fn header(b: &[u8], what: &str) -> Result<Header, String> {
    if b.len() < 100 {
        return Err(format!(
            "“{what}” dosyası 100 baytlık başlıktan kısa ({} bayt): Shapefile değil ya da yarım kalmış.",
            b.len()
        ));
    }
    if i32_be(b, 0) != Some(9994) {
        return Err(format!(
            "“{what}” bir Shapefile dosyası değil (dosya kodu 9994 değil). Katmanın .shp, .shx ve .dbf dosyalarını seçin."
        ));
    }
    Ok(Header {
        shape_type: i32_le(b, 32).unwrap_or(0),
        version: i32_le(b, 28).unwrap_or(0),
    })
}

/// The records of a .shp file in order: each one's content, until the file
/// ends or a record claims more bytes than follow (then `cut` says where).
pub struct Records<'a> {
    b: &'a [u8],
    at: usize,
    /// The byte where reading stopped because a record ran past the end.
    pub cut: Option<usize>,
}

impl<'a> Records<'a> {
    pub fn new(shp: &'a [u8]) -> Records<'a> {
        Records {
            b: shp,
            at: 100,
            cut: None,
        }
    }
}

impl<'a> Iterator for Records<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.cut.is_some() || self.at + 8 > self.b.len() {
            return None;
        }
        // Content length in 16-bit words, big-endian, after the record number.
        let words = i32_be(self.b, self.at + 4)?;
        let len = usize::try_from(words).ok().and_then(|w| w.checked_mul(2));
        let start = self.at + 8;
        match len.and_then(|l| start.checked_add(l)) {
            Some(end) if end <= self.b.len() => {
                self.at = end;
                self.b.get(start..end)
            }
            _ => {
                self.cut = Some(self.at);
                None
            }
        }
    }
}

fn point(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

/// Bytes a record needs up to its point list, given counts (None: out of range).
fn need(base: usize, parts: usize, points: usize) -> Option<usize> {
    base.checked_add(parts.checked_mul(4)?)?
        .checked_add(points.checked_mul(16)?)
}

/// The shapes of one record's content, in the order they come.
pub fn shapes(c: &[u8], out: &mut Vec<Shape>, found: &mut Found) {
    let Some(t) = i32_le(c, 0) else {
        found.short += 1;
        return;
    };
    let z_type = matches!(t, 11 | 13 | 15 | 18);
    let m_type = matches!(t, 21 | 23 | 25 | 28);
    // After the points: a Z type's Z block (range and one value a point), then its M
    // block, which may be missing; an M type's M block, which may not.
    let block = |end: usize, n: usize| -> Option<usize> {
        n.checked_mul(8)
            .and_then(|b| end.checked_add(16)?.checked_add(b))
    };
    match t {
        0 => {}
        1 | 11 | 21 => {
            // x, y; PointZ: z [, m]; PointM: m
            let need_len = if t == 1 { 20 } else { 28 };
            if c.len() < need_len {
                found.short += 1;
                return;
            }
            let (Some(x), Some(y)) = (f64_le(c, 4), f64_le(c, 12)) else {
                found.short += 1;
                return;
            };
            let z = if t == 11 { f64_le(c, 20) } else { None };
            if t == 21 || (t == 11 && c.len() >= 36) {
                found.m += 1;
            }
            if !x.is_finite() || !y.is_finite() || z.is_some_and(|z| !z.is_finite()) {
                found.not_finite += 1;
                return;
            }
            out.push(Shape::Point { p: point(x, y), z });
        }
        8 | 18 | 28 => {
            // Box (32 bytes), count, points; then Z range and array, then M range and array.
            let Some(n) = i32_le(c, 36).and_then(|n| usize::try_from(n).ok()) else {
                found.short += 1;
                return;
            };
            let Some(pts_end) = need(40, 0, n) else {
                found.short += 1;
                return;
            };
            let end = if z_type || m_type {
                block(pts_end, n)
            } else {
                Some(pts_end)
            };
            let Some(end) = end.filter(|&e| e <= c.len()) else {
                found.short += 1;
                return;
            };
            if n > 0 && (m_type || (z_type && block(end, n).is_some_and(|e| e <= c.len()))) {
                found.m += 1;
            }
            for i in 0..n {
                let (Some(x), Some(y)) = (f64_le(c, 40 + 16 * i), f64_le(c, 48 + 16 * i)) else {
                    found.short += 1;
                    return;
                };
                let z = if t == 18 {
                    f64_le(c, pts_end + 16 + 8 * i)
                } else {
                    None
                };
                if !x.is_finite() || !y.is_finite() || z.is_some_and(|z| !z.is_finite()) {
                    found.not_finite += 1;
                    continue;
                }
                out.push(Shape::Point { p: point(x, y), z });
            }
        }
        3 | 13 | 23 | 5 | 15 | 25 => {
            let (Some(np), Some(n)) = (
                i32_le(c, 36).and_then(|n| usize::try_from(n).ok()),
                i32_le(c, 40).and_then(|n| usize::try_from(n).ok()),
            ) else {
                found.short += 1;
                return;
            };
            let Some(pts_end) = need(44, np, n) else {
                found.short += 1;
                return;
            };
            let end = if z_type || m_type {
                block(pts_end, n)
            } else {
                Some(pts_end)
            };
            let Some(end) = end.filter(|&e| e <= c.len()) else {
                found.short += 1;
                return;
            };
            if n > 0 && (m_type || (z_type && block(end, n).is_some_and(|e| e <= c.len()))) {
                found.m += 1;
            }
            // Parts: starting indices, from 0, in order, within the points.
            let mut starts = Vec::with_capacity(np);
            for i in 0..np {
                match i32_le(c, 44 + 4 * i).and_then(|s| usize::try_from(s).ok()) {
                    Some(s) => starts.push(s),
                    None => {
                        found.bad_parts += 1;
                        return;
                    }
                }
            }
            let ordered = starts.first() == Some(&0)
                && starts.windows(2).all(|w| w[0] <= w[1])
                && starts.last().is_some_and(|&s| s <= n);
            if !ordered {
                found.bad_parts += 1;
                return;
            }
            let base = 44 + 4 * np;
            let at = |i: usize| -> Vec2 {
                point(
                    f64_le(c, base + 16 * i).unwrap_or(f64::NAN),
                    f64_le(c, base + 16 * i + 8).unwrap_or(f64::NAN),
                )
            };
            let parts: Vec<Vec<Vec2>> = (0..np)
                .map(|k| {
                    let end = starts.get(k + 1).copied().unwrap_or(n);
                    (starts[k]..end).map(at).collect()
                })
                .collect();
            if z_type && n > 0 {
                found.z_dropped += 1;
            }
            let finite = |pts: &[Vec2]| pts.iter().all(|p| p.x.is_finite() && p.y.is_finite());
            if matches!(t, 3 | 13 | 23) {
                for pts in parts {
                    if !finite(&pts) {
                        found.not_finite += 1;
                        continue;
                    }
                    match pts.len() {
                        0 | 1 => found.short_parts += 1,
                        2 => out.push(Shape::Line(pts[0], pts[1])),
                        _ => out.push(Shape::Polyline(pts)),
                    }
                }
            } else {
                if !parts.iter().all(|p| finite(p)) {
                    found.not_finite += 1;
                    return;
                }
                areas(parts, out, found);
            }
        }
        31 => found.multipatch += 1,
        _ => found.unknown += 1,
    }
}

/// Even-odd ray casting, exactly as the rules write it (docs/adr/0046).
fn contains(ring: &[Vec2], p: Vec2) -> bool {
    let n = ring.len();
    let mut inside = false;
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (xi, yi) = (ring[i].x, ring[i].y);
        let (xj, yj) = (ring[j].x, ring[j].y);
        if (yi > p.y) != (yj > p.y) && p.x < (xj - xi) * (p.y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// An area record's rings as areas with holes.
fn areas(parts: Vec<Vec<Vec2>>, out: &mut Vec<Shape>, found: &mut Found) {
    // (part index, ring, clockwise)
    let mut rings: Vec<(usize, Vec<Vec2>, bool)> = Vec::new();
    for (k, pts) in parts.into_iter().enumerate() {
        let (ring, _) = open_ring(pts);
        if ring.len() < 3 {
            found.short_rings += 1;
            continue;
        }
        let s = shoelace(&ring);
        if s == 0.0 {
            found.short_rings += 1;
            continue;
        }
        rings.push((k, ring, s < 0.0));
    }
    if rings.is_empty() {
        return;
    }
    if !rings.iter().any(|r| r.2) {
        // No clockwise ring: every ring is an area of its own.
        found.no_outline += 1;
        for (_, ring, _) in rings {
            out.push(Shape::Polygon(ring, Vec::new()));
        }
        return;
    }
    // Outlines in part order; each hole to the first outline holding its first vertex.
    let mut polys: Vec<(usize, Vec<Vec2>, Vec<Vec<Vec2>>)> = rings
        .iter()
        .filter(|r| r.2)
        .map(|(k, ring, _)| (*k, ring.clone(), Vec::new()))
        .collect();
    let outlines = polys.len();
    for (k, ring, cw) in rings {
        if cw {
            continue;
        }
        match polys[..outlines]
            .iter()
            .position(|(_, o, _)| contains(o, ring[0]))
        {
            Some(i) => polys[i].2.push(ring),
            None => {
                found.lone_holes += 1;
                polys.push((k, ring, Vec::new()));
            }
        }
    }
    polys.sort_by_key(|p| p.0);
    for (_, outline, holes) in polys {
        out.push(Shape::Polygon(outline, holes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    /// A polygon record of rings (type 5).
    fn polygon(rings: &[&[(f64, f64)]]) -> Vec<u8> {
        let n: usize = rings.iter().map(|r| r.len()).sum();
        let mut c = Vec::new();
        c.extend_from_slice(&5i32.to_le_bytes());
        c.extend_from_slice(&[0u8; 32]);
        c.extend_from_slice(&(rings.len() as i32).to_le_bytes());
        c.extend_from_slice(&(n as i32).to_le_bytes());
        let mut s = 0;
        for r in rings {
            c.extend_from_slice(&(s as i32).to_le_bytes());
            s += r.len();
        }
        for r in rings {
            for (x, y) in r.iter() {
                c.extend_from_slice(&x.to_le_bytes());
                c.extend_from_slice(&y.to_le_bytes());
            }
        }
        c
    }

    #[test]
    fn clockwise_rings_are_outlines_and_holes_find_theirs() {
        // Two clockwise outlines; a hole inside the second; a hole inside neither.
        let a: &[(f64, f64)] = &[
            (0.0, 0.0),
            (0.0, 10.0),
            (10.0, 10.0),
            (10.0, 0.0),
            (0.0, 0.0),
        ];
        let b: &[(f64, f64)] = &[
            (20.0, 0.0),
            (20.0, 10.0),
            (30.0, 10.0),
            (30.0, 0.0),
            (20.0, 0.0),
        ];
        let hole: &[(f64, f64)] = &[(22.0, 2.0), (24.0, 2.0), (24.0, 4.0), (22.0, 2.0)];
        let lone: &[(f64, f64)] = &[(50.0, 50.0), (52.0, 50.0), (52.0, 52.0), (50.0, 50.0)];
        let mut out = Vec::new();
        let mut found = Found::default();
        shapes(&polygon(&[a, hole, b, lone]), &mut out, &mut found);
        assert_eq!(out.len(), 3);
        let Shape::Polygon(o, h) = &out[0] else {
            panic!()
        };
        assert_eq!((o[0], h.len()), (v(0.0, 0.0), 0));
        let Shape::Polygon(_, h) = &out[1] else {
            panic!()
        };
        assert_eq!(h.len(), 1);
        assert_eq!(found.lone_holes, 1);
    }

    #[test]
    fn broken_records_give_nothing_and_never_panic() {
        let good = polygon(&[&[(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.0, 0.0)]]);
        // Every cut of a good record, and counts that point past the end.
        for len in 0..good.len() {
            let mut out = Vec::new();
            shapes(&good[..len], &mut out, &mut Found::default());
            assert!(out.is_empty(), "cut at {len}");
        }
        let mut huge = good.clone();
        huge[36..40].copy_from_slice(&i32::MAX.to_le_bytes());
        huge[40..44].copy_from_slice(&i32::MAX.to_le_bytes());
        let mut out = Vec::new();
        shapes(&huge, &mut out, &mut Found::default());
        assert!(out.is_empty());
        let mut negative = good.clone();
        negative[40..44].copy_from_slice(&(-3i32).to_le_bytes());
        shapes(&negative, &mut out, &mut Found::default());
        assert!(out.is_empty());
        // Parts out of order.
        let two = polygon(&[
            &[(0.0, 0.0), (0.0, 1.0), (1.0, 1.0)],
            &[(5.0, 5.0), (5.0, 6.0), (6.0, 6.0)],
        ]);
        let mut swapped = two.clone();
        swapped[44..48].copy_from_slice(&3i32.to_le_bytes());
        swapped[48..52].copy_from_slice(&0i32.to_le_bytes());
        let mut found = Found::default();
        shapes(&swapped, &mut out, &mut found);
        assert!(out.is_empty());
        assert_eq!(found.bad_parts, 1);
    }

    #[test]
    fn records_stop_where_a_length_runs_past_the_end() {
        let mut shp = vec![0u8; 100];
        // One record of 20 bytes content (a point), then one claiming a gigabyte.
        shp.extend_from_slice(&1i32.to_be_bytes());
        shp.extend_from_slice(&10i32.to_be_bytes());
        shp.extend_from_slice(&1i32.to_le_bytes());
        shp.extend_from_slice(&1.5f64.to_le_bytes());
        shp.extend_from_slice(&2.5f64.to_le_bytes());
        shp.extend_from_slice(&2i32.to_be_bytes());
        shp.extend_from_slice(&(1i32 << 29).to_be_bytes());
        let mut r = Records::new(&shp);
        let first = r.next().expect("first");
        assert_eq!(first.len(), 20);
        assert!(r.next().is_none());
        assert_eq!(r.cut, Some(128));
    }
}
