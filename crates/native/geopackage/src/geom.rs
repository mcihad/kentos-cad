//! A GeoPackage geometry blob (OGC 12-128r19 §2.1.3): the `GP` header (version, flags, srs id, envelope) and the ISO
//! WKB after it, with Z and M (type codes +1000, +2000, +3000; EWKB's high bits read too), empty geometries (the
//! header's flag; a point's NaN coordinates), and the curve types of the non-linear geometry extension (circular
//! string, compound curve, curve polygon, multi curve, multi surface). Reading never panics: a short or nested-too-deep
//! blob is an error.

use std::fmt;

/// A position: x east, y north, and Z and M when the geometry has them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
    pub z: Option<f64>,
    pub m: Option<f64>,
}

impl Coord {
    pub fn xy(x: f64, y: f64) -> Coord {
        Coord {
            x,
            y,
            z: None,
            m: None,
        }
    }
}

/// A curve: straight segments (a line string), arcs through every three points (a circular string: 3, 5, 7 …
/// points), or both one after another (a compound curve of those two).
#[derive(Clone, Debug, PartialEq)]
pub enum Curve {
    Line(Vec<Coord>),
    Arc(Vec<Coord>),
    Compound(Vec<Curve>),
}

impl Curve {
    /// Whether any part of it is an arc.
    pub fn is_curved(&self) -> bool {
        match self {
            Curve::Line(_) => false,
            Curve::Arc(_) => true,
            Curve::Compound(parts) => parts.iter().any(Curve::is_curved),
        }
    }

    fn coords<'a>(&'a self, out: &mut Vec<&'a Coord>) {
        match self {
            Curve::Line(p) | Curve::Arc(p) => out.extend(p.iter()),
            Curve::Compound(parts) => parts.iter().for_each(|c| c.coords(out)),
        }
    }
}

/// A geometry. A polygon's first ring is its outer ring, the others its holes; a linear polygon's rings are lines.
#[derive(Clone, Debug, PartialEq)]
pub enum Geometry {
    /// A point; none for `POINT EMPTY`.
    Point(Option<Coord>),
    Curve(Curve),
    Polygon(Vec<Curve>),
    MultiPoint(Vec<Coord>),
    MultiCurve(Vec<Curve>),
    MultiPolygon(Vec<Vec<Curve>>),
    Collection(Vec<Geometry>),
}

impl Geometry {
    /// Whether it has no position at all.
    pub fn is_empty(&self) -> bool {
        let mut c = Vec::new();
        self.coords(&mut c);
        c.is_empty()
    }

    /// Whether any curve of it has an arc (it needs the non-linear geometry extension).
    pub fn is_curved(&self) -> bool {
        match self {
            Geometry::Point(_) | Geometry::MultiPoint(_) => false,
            Geometry::Curve(c) => c.is_curved(),
            Geometry::Polygon(rings) | Geometry::MultiCurve(rings) => {
                rings.iter().any(Curve::is_curved)
            }
            Geometry::MultiPolygon(polys) => polys.iter().flatten().any(Curve::is_curved),
            Geometry::Collection(gs) => gs.iter().any(Geometry::is_curved),
        }
    }

    fn coords<'a>(&'a self, out: &mut Vec<&'a Coord>) {
        match self {
            Geometry::Point(p) => out.extend(p.iter()),
            Geometry::MultiPoint(p) => out.extend(p.iter()),
            Geometry::Curve(c) => c.coords(out),
            Geometry::Polygon(rings) | Geometry::MultiCurve(rings) => {
                rings.iter().for_each(|c| c.coords(out))
            }
            Geometry::MultiPolygon(polys) => polys.iter().flatten().for_each(|c| c.coords(out)),
            Geometry::Collection(gs) => gs.iter().for_each(|g| g.coords(out)),
        }
    }

    /// Whether its positions have Z, and M.
    pub fn dims(&self) -> (bool, bool) {
        let mut c = Vec::new();
        self.coords(&mut c);
        (
            c.first().is_some_and(|p| p.z.is_some()),
            c.first().is_some_and(|p| p.m.is_some()),
        )
    }

    /// Its box (min x, min y, max x, max y) through its positions; arcs bulge past them, so for an arc the box of its
    /// points is a bound only when the arc's middle is one of them (as the spatial index's GDAL writes it too).
    pub fn bounds(&self) -> Option<[f64; 4]> {
        let mut c = Vec::new();
        self.coords(&mut c);
        let mut it = c.into_iter().filter(|p| p.x.is_finite() && p.y.is_finite());
        let first = it.next()?;
        let mut b = [first.x, first.y, first.x, first.y];
        for p in it {
            b[0] = b[0].min(p.x);
            b[1] = b[1].min(p.y);
            b[2] = b[2].max(p.x);
            b[3] = b[3].max(p.y);
        }
        Some(b)
    }

    /// The WKB type name, as `gpkg_geometry_columns` names it.
    pub fn type_name(&self) -> &'static str {
        match self {
            Geometry::Point(_) => "POINT",
            Geometry::Curve(Curve::Line(_)) => "LINESTRING",
            Geometry::Curve(Curve::Arc(_)) => "CIRCULARSTRING",
            Geometry::Curve(Curve::Compound(_)) => "COMPOUNDCURVE",
            Geometry::Polygon(r) if r.iter().any(Curve::is_curved) => "CURVEPOLYGON",
            Geometry::Polygon(_) => "POLYGON",
            Geometry::MultiPoint(_) => "MULTIPOINT",
            Geometry::MultiCurve(c) if c.iter().any(Curve::is_curved) => "MULTICURVE",
            Geometry::MultiCurve(_) => "MULTILINESTRING",
            Geometry::MultiPolygon(p) if p.iter().flatten().any(Curve::is_curved) => "MULTISURFACE",
            Geometry::MultiPolygon(_) => "MULTIPOLYGON",
            Geometry::Collection(_) => "GEOMETRYCOLLECTION",
        }
    }
}

/// Why a blob is not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlobError(pub String);

impl fmt::Display for BlobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn err<T>(why: impl Into<String>) -> Result<T, BlobError> {
    Err(BlobError(why.into()))
}

/// A blob read: its srs id and geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct Blob {
    pub srs_id: i32,
    pub geometry: Geometry,
}

const MAX_DEPTH: usize = 32;

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], BlobError> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.b.len());
        match end {
            Some(e) => {
                let s = &self.b[self.at..e];
                self.at = e;
                Ok(s)
            }
            None => err(format!("geometri {}. baytta bitiyor", self.at)),
        }
    }

    fn u8(&mut self) -> Result<u8, BlobError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self, le: bool) -> Result<u32, BlobError> {
        let s = self.take(4)?;
        let a = [s[0], s[1], s[2], s[3]];
        Ok(if le {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        })
    }

    fn f64(&mut self, le: bool) -> Result<f64, BlobError> {
        let s = self.take(8)?;
        let mut a = [0; 8];
        a.copy_from_slice(s);
        Ok(if le {
            f64::from_le_bytes(a)
        } else {
            f64::from_be_bytes(a)
        })
    }

    /// A count, no more than the bytes left could hold at `each` bytes.
    fn count(&mut self, le: bool, each: usize) -> Result<usize, BlobError> {
        let n = self.u32(le)? as usize;
        if n.saturating_mul(each.max(1)) > self.b.len() - self.at {
            return err(format!("{n} öğe sayısı geometrinin kalanına sığmıyor"));
        }
        Ok(n)
    }
}

/// A WKB geometry's header: its byte order, base type and dimensions.
struct Head {
    le: bool,
    kind: u32,
    z: bool,
    m: bool,
}

fn head(r: &mut Reader) -> Result<Head, BlobError> {
    let le = match r.u8()? {
        0 => false,
        1 => true,
        o => return err(format!("bilinmeyen bayt sırası {o}")),
    };
    let code = r.u32(le)?;
    // EWKB: Z 0x80000000, M 0x40000000, SRID 0x20000000 (an id follows).
    let (mut z, mut m) = (code & 0x8000_0000 != 0, code & 0x4000_0000 != 0);
    if code & 0x2000_0000 != 0 {
        r.u32(le)?;
    }
    let iso = code & 0x0fff_ffff;
    let kind = iso % 1000;
    match iso / 1000 {
        0 => {}
        1 => z = true,
        2 => m = true,
        3 => {
            z = true;
            m = true;
        }
        d => return err(format!("bilinmeyen WKB boyutu {d}")),
    }
    Ok(Head { le, kind, z, m })
}

fn coord(r: &mut Reader, h: &Head) -> Result<Coord, BlobError> {
    let x = r.f64(h.le)?;
    let y = r.f64(h.le)?;
    let z = if h.z { Some(r.f64(h.le)?) } else { None };
    let m = if h.m { Some(r.f64(h.le)?) } else { None };
    Ok(Coord { x, y, z, m })
}

fn width(h: &Head) -> usize {
    8 * (2 + h.z as usize + h.m as usize)
}

fn points(r: &mut Reader, h: &Head) -> Result<Vec<Coord>, BlobError> {
    let n = r.count(h.le, width(h))?;
    (0..n).map(|_| coord(r, h)).collect()
}

fn curve(r: &mut Reader, depth: usize) -> Result<Curve, BlobError> {
    if depth > MAX_DEPTH {
        return err("geometri çok derin iç içe");
    }
    let h = head(r)?;
    match h.kind {
        2 => Ok(Curve::Line(points(r, &h)?)),
        8 => Ok(Curve::Arc(points(r, &h)?)),
        9 => {
            let n = r.count(h.le, 9)?;
            let mut parts = Vec::with_capacity(n);
            for _ in 0..n {
                parts.push(curve(r, depth + 1)?);
            }
            Ok(Curve::Compound(parts))
        }
        k => err(format!("eğri beklenirken WKB türü {k}")),
    }
}

fn surface(r: &mut Reader, depth: usize) -> Result<Vec<Curve>, BlobError> {
    if depth > MAX_DEPTH {
        return err("geometri çok derin iç içe");
    }
    let h = head(r)?;
    match h.kind {
        3 => {
            let n = r.count(h.le, 4)?;
            (0..n).map(|_| Ok(Curve::Line(points(r, &h)?))).collect()
        }
        10 => {
            let n = r.count(h.le, 9)?;
            (0..n).map(|_| curve(r, depth + 1)).collect()
        }
        k => err(format!("yüzey beklenirken WKB türü {k}")),
    }
}

fn geometry(r: &mut Reader, depth: usize) -> Result<Geometry, BlobError> {
    if depth > MAX_DEPTH {
        return err("geometri çok derin iç içe");
    }
    let start = r.at;
    let h = head(r)?;
    match h.kind {
        1 => {
            let c = coord(r, &h)?;
            Ok(Geometry::Point(
                (!(c.x.is_nan() && c.y.is_nan())).then_some(c),
            ))
        }
        2 | 8 | 9 => {
            r.at = start;
            Ok(Geometry::Curve(curve(r, depth)?))
        }
        3 | 10 => {
            r.at = start;
            Ok(Geometry::Polygon(surface(r, depth)?))
        }
        4 => {
            let n = r.count(h.le, 5 + width(&h))?;
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                match geometry(r, depth + 1)? {
                    Geometry::Point(Some(c)) => out.push(c),
                    Geometry::Point(None) => {}
                    _ => return err("çok noktanın öğesi nokta değil"),
                }
            }
            Ok(Geometry::MultiPoint(out))
        }
        5 | 11 => {
            let n = r.count(h.le, 9)?;
            (0..n)
                .map(|_| curve(r, depth + 1))
                .collect::<Result<_, _>>()
                .map(Geometry::MultiCurve)
        }
        6 | 12 => {
            let n = r.count(h.le, 9)?;
            (0..n)
                .map(|_| surface(r, depth + 1))
                .collect::<Result<_, _>>()
                .map(Geometry::MultiPolygon)
        }
        7 => {
            let n = r.count(h.le, 9)?;
            (0..n)
                .map(|_| geometry(r, depth + 1))
                .collect::<Result<_, _>>()
                .map(Geometry::Collection)
        }
        k => err(format!("bilinmeyen WKB türü {k}")),
    }
}

/// Reads ISO (or extended) WKB alone.
pub fn read_wkb(wkb: &[u8]) -> Result<Geometry, BlobError> {
    let mut r = Reader { b: wkb, at: 0 };
    let g = geometry(&mut r, 0)?;
    Ok(g)
}

/// Reads a GeoPackage geometry blob.
pub fn read_blob(blob: &[u8]) -> Result<Blob, BlobError> {
    let mut r = Reader { b: blob, at: 0 };
    if r.take(2)? != b"GP" {
        return err("GeoPackage geometrisi değil: GP imzası yok");
    }
    let version = r.u8()?;
    if version != 0 {
        return err(format!("bilinmeyen GeoPackage geometri sürümü {version}"));
    }
    let flags = r.u8()?;
    let le = flags & 1 == 1;
    let envelope = match (flags >> 1) & 7 {
        0 => 0,
        1 => 4,
        2 | 3 => 6,
        4 => 8,
        e => return err(format!("bilinmeyen kutu türü {e}")),
    };
    if flags & 0x20 != 0 {
        return err("genişletilmiş GeoPackage geometrisi okunmuyor");
    }
    let srs_id = r.u32(le)? as i32;
    r.take(envelope * 8)?;
    let empty = flags & 0x10 != 0;
    let geometry = geometry(&mut r, 0)?;
    let geometry = if empty && !geometry.is_empty() {
        return err("başlığı boş diyen geometrinin noktaları var");
    } else {
        geometry
    };
    Ok(Blob { srs_id, geometry })
}

struct Writer {
    out: Vec<u8>,
    z: bool,
    m: bool,
}

impl Writer {
    fn code(&mut self, kind: u32) {
        let dims = match (self.z, self.m) {
            (false, false) => 0,
            (true, false) => 1000,
            (false, true) => 2000,
            (true, true) => 3000,
        };
        self.out.push(1);
        self.out.extend((kind + dims).to_le_bytes());
    }

    fn n(&mut self, n: usize) {
        self.out.extend((n as u32).to_le_bytes());
    }

    fn coord(&mut self, c: &Coord) {
        self.out.extend(c.x.to_le_bytes());
        self.out.extend(c.y.to_le_bytes());
        if self.z {
            self.out.extend(c.z.unwrap_or(f64::NAN).to_le_bytes());
        }
        if self.m {
            self.out.extend(c.m.unwrap_or(f64::NAN).to_le_bytes());
        }
    }

    fn points(&mut self, p: &[Coord]) {
        self.n(p.len());
        p.iter().for_each(|c| self.coord(c));
    }

    fn curve(&mut self, c: &Curve) {
        match c {
            Curve::Line(p) => {
                self.code(2);
                self.points(p);
            }
            Curve::Arc(p) => {
                self.code(8);
                self.points(p);
            }
            Curve::Compound(parts) => {
                self.code(9);
                self.n(parts.len());
                parts.iter().for_each(|p| self.curve(p));
            }
        }
    }

    fn surface(&mut self, rings: &[Curve]) {
        if rings.iter().any(Curve::is_curved) {
            self.code(10);
            self.n(rings.len());
            rings.iter().for_each(|r| self.curve(r));
        } else {
            self.code(3);
            self.n(rings.len());
            for r in rings {
                if let Curve::Line(p) = r {
                    self.points(p);
                }
            }
        }
    }

    fn geometry(&mut self, g: &Geometry) {
        match g {
            Geometry::Point(p) => {
                self.code(1);
                let nan = Coord {
                    x: f64::NAN,
                    y: f64::NAN,
                    z: None,
                    m: None,
                };
                self.coord(p.as_ref().unwrap_or(&nan));
            }
            Geometry::Curve(c) => self.curve(c),
            Geometry::Polygon(rings) => self.surface(rings),
            Geometry::MultiPoint(ps) => {
                self.code(4);
                self.n(ps.len());
                for p in ps {
                    self.code(1);
                    self.coord(p);
                }
            }
            Geometry::MultiCurve(cs) => {
                self.code(if cs.iter().any(Curve::is_curved) {
                    11
                } else {
                    5
                });
                self.n(cs.len());
                cs.iter().for_each(|c| self.curve(c));
            }
            Geometry::MultiPolygon(ps) => {
                self.code(if ps.iter().flatten().any(Curve::is_curved) {
                    12
                } else {
                    6
                });
                self.n(ps.len());
                ps.iter().for_each(|p| self.surface(p));
            }
            Geometry::Collection(gs) => {
                self.code(7);
                self.n(gs.len());
                gs.iter().for_each(|g| self.geometry(g));
            }
        }
    }
}

/// ISO WKB, little endian.
pub fn write_wkb(g: &Geometry) -> Vec<u8> {
    let (z, m) = g.dims();
    let mut w = Writer {
        out: Vec::new(),
        z,
        m,
    };
    w.geometry(g);
    w.out
}

/// A GeoPackage geometry blob: little endian, the xy box (none for a point or an empty geometry), then the WKB.
pub fn write_blob(srs_id: i32, g: &Geometry) -> Vec<u8> {
    let empty = g.is_empty();
    let bounds = if empty || matches!(g, Geometry::Point(_)) {
        None
    } else {
        g.bounds()
    };
    let mut flags = 1u8;
    if bounds.is_some() {
        flags |= 1 << 1;
    }
    if empty {
        flags |= 0x10;
    }
    let mut out = vec![b'G', b'P', 0, flags];
    out.extend(srs_id.to_le_bytes());
    if let Some(b) = bounds {
        for v in [b[0], b[2], b[1], b[3]] {
            out.extend(v.to_le_bytes());
        }
    }
    out.extend(write_wkb(g));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_polygon_with_a_hole_goes_and_comes_back() {
        let ring =
            |p: &[(f64, f64)]| Curve::Line(p.iter().map(|&(x, y)| Coord::xy(x, y)).collect());
        let g = Geometry::Polygon(vec![
            ring(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 0.0)]),
            ring(&[(2.0, 1.0), (8.0, 1.0), (8.0, 7.0), (2.0, 1.0)]),
        ]);
        let blob = write_blob(5256, &g);
        let back = read_blob(&blob).expect("read");
        assert_eq!(back.srs_id, 5256);
        assert_eq!(back.geometry, g);
    }

    #[test]
    fn an_empty_point_and_a_short_blob() {
        let blob = write_blob(4326, &Geometry::Point(None));
        assert_eq!(
            read_blob(&blob).expect("read").geometry,
            Geometry::Point(None)
        );
        assert!(read_blob(&blob[..10]).is_err());
        assert!(read_blob(b"GP\0\x01\0\0\0\0\x01\x07\0\0\0\xff\xff\xff\x7f").is_err());
    }

    #[test]
    fn z_and_m_and_arcs_keep() {
        let c = |x, y, z| Coord {
            x,
            y,
            z: Some(z),
            m: Some(x + y),
        };
        let g = Geometry::Curve(Curve::Compound(vec![
            Curve::Arc(vec![c(0.0, 0.0, 1.0), c(1.0, 1.0, 2.0), c(2.0, 0.0, 3.0)]),
            Curve::Line(vec![c(2.0, 0.0, 3.0), c(4.0, 0.0, 4.0)]),
        ]));
        let back = read_blob(&write_blob(0, &g)).expect("read").geometry;
        assert_eq!(back, g);
        assert_eq!(back.type_name(), "COMPOUNDCURVE");
    }
}
