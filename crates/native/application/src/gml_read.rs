//! Reading GML (docs/adr/0545, plan/gis.md CBS-05): GML 2, 3.1 and 3.2 simple features as GDAL's GML driver and WFS
//! servers write them. A feature type is a table: its properties are columns typed by the application schema beside
//! the file (`.xsd`), else GDAL's `.gfs`, else by the values; its features are rows with the geometry as GeoPackage's
//! [`Geometry`] (arcs exact: `Curve` with `ArcString` and `Arc` segments, `Ring`, `CompositeCurve`). A geometry's
//! `srsName` says its axis order: the URN and URI forms follow EPSG's (TUREF's TM zones north first, EPSG:4326
//! latitude first), `EPSG:n` is east first; the coordinates come out east then north. Reading never panics: a
//! document nested too deep, a geometry too deep or a number that is none is an error of the file or of the feature.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_formats::xml::{self, Node};
use kentos_geopackage::{BlobError, Column, ColumnKind, Coord, Curve, Geometry, Row, Value};
use kentos_services::crs::{Axis, axis_order, crs_name};

/// How deep geometries may nest (a collection of collections …).
const DEPTH: usize = 32;
/// The largest document read (bytes).
pub const MAX_BYTES: usize = 256 << 20;

/// A feature type as the file reads: a table.
#[derive(Debug)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub rows: Vec<Row>,
}

/// A GML read.
#[derive(Debug, Default)]
pub struct Parsed {
    pub tables: Vec<Table>,
    /// The first EPSG system a geometry (else the collection's envelope) names.
    pub srid: Option<u32>,
    /// A system name that is no EPSG code, as written.
    pub other_crs: Option<String>,
    /// What was not carried, once each.
    pub losses: BTreeSet<String>,
    /// Where the types came from.
    pub typed_by: Typed,
}

/// Where a file's property types came from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Typed {
    Schema,
    Gfs,
    #[default]
    Values,
}

fn is_gml(n: Node<'_, '_>) -> bool {
    n.tag_name()
        .namespace()
        .is_some_and(|ns| ns == "http://www.opengis.net/gml" || ns.starts_with("http://www.opengis.net/gml/"))
}

fn local<'a>(n: Node<'a, '_>) -> &'a str {
    n.tag_name().name()
}

fn attr<'a>(n: Node<'a, '_>, name: &str) -> Option<&'a str> {
    n.attributes().find(|a| a.name() == name).map(|a| a.value())
}

fn text_of(n: Node<'_, '_>) -> String {
    n.children()
        .filter(|c| c.is_text())
        .filter_map(|c| c.text())
        .collect()
}

fn elements<'a, 'b: 'a>(n: Node<'a, 'b>) -> impl Iterator<Item = Node<'a, 'b>> {
    n.children().filter(|c| c.is_element())
}

// ── Geometry ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Reading {
    dim: usize,
    swap: bool,
}

#[derive(Default)]
struct Systems {
    seen: Vec<u32>,
    other: Option<String>,
}

impl Systems {
    fn note(&mut self, name: &str) {
        match crs_name(name) {
            Some(c) => {
                if !self.seen.contains(&c.srid) {
                    self.seen.push(c.srid);
                }
            }
            None => {
                if self.other.is_none() {
                    self.other = Some(name.to_owned());
                }
            }
        }
    }
}

/// Whether coordinates under a system name go north (latitude) first.
fn swaps(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    (upper.starts_with("URN:") || upper.starts_with("HTTP"))
        && crs_name(name)
            .is_some_and(|c| !c.lon_lat && matches!(axis_order(c.srid), Axis::Ne | Axis::LatLon))
}

fn reading_of(n: Node<'_, '_>, parent: Reading, sys: &mut Systems) -> Reading {
    let mut r = parent;
    if let Some(name) = attr(n, "srsName") {
        r.swap = swaps(name);
        sys.note(name);
    }
    if let Some(d) = attr(n, "srsDimension").and_then(|d| d.parse::<usize>().ok()) {
        r.dim = d.clamp(2, 3);
    }
    r
}

fn numbers(text: &str) -> Result<Vec<f64>, String> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("“{s}” bir koordinat sayısı değil"))
        })
        .collect()
}

fn coord(nums: &[f64], r: Reading, dim: usize) -> Coord {
    let (a, b) = (nums[0], nums[1]);
    let (x, y) = if r.swap { (b, a) } else { (a, b) };
    Coord {
        x,
        y,
        z: nums.get(2).copied().filter(|_| dim == 3),
        m: None,
    }
}

/// The positions under `n`: `posList`, `pos`, GML 2's `coordinates` and `coord`, a `pointProperty`'s point.
fn positions(n: Node<'_, '_>, r: Reading, sys: &mut Systems) -> Result<Vec<Coord>, String> {
    let r = reading_of(n, r, sys);
    let mut out = Vec::new();
    for c in elements(n) {
        let cr = reading_of(c, r, sys);
        match local(c) {
            "posList" => {
                let nums = numbers(&text_of(c))?;
                if nums.len() % cr.dim != 0 {
                    return Err(format!(
                        "posList'in {} sayısı boyuta ({}) bölünmüyor",
                        nums.len(),
                        cr.dim
                    ));
                }
                out.extend(nums.chunks_exact(cr.dim).map(|p| coord(p, cr, cr.dim)));
            }
            "pos" => {
                let nums = numbers(&text_of(c))?;
                if nums.len() < 2 || nums.len() > 3 {
                    return Err(format!("pos'ta {} sayı var; 2 ya da 3 olmalı", nums.len()));
                }
                out.push(coord(&nums, cr, nums.len()));
            }
            "coordinates" => {
                // GML 2: tuples by spaces, numbers by commas, x first, never swapped.
                let t = text_of(c);
                let cs = attr(c, "cs").unwrap_or(",");
                let ts = attr(c, "ts").unwrap_or(" ");
                let flat = Reading { swap: false, ..cr };
                for tuple in t
                    .split(|ch: char| ts.contains(ch) || (ts == " " && ch.is_whitespace()))
                    .filter(|s| !s.is_empty())
                {
                    let nums = numbers(&tuple.replace(cs, " "))?;
                    if nums.len() < 2 || nums.len() > 3 {
                        return Err(format!("coordinates'ta {} sayılı bir konum var", nums.len()));
                    }
                    out.push(coord(&nums, flat, nums.len()));
                }
            }
            "coord" => {
                let v = |k: &str| {
                    elements(c)
                        .find(|e| local(*e) == k)
                        .and_then(|e| text_of(e).trim().parse::<f64>().ok())
                };
                if let (Some(x), Some(y)) = (v("X"), v("Y")) {
                    out.push(Coord {
                        x,
                        y,
                        z: v("Z"),
                        m: None,
                    });
                }
            }
            "pointProperty" | "pointRep" | "Point" => {
                let inner = if local(c) == "Point" {
                    c
                } else {
                    match elements(c).next() {
                        Some(p) => p,
                        None => continue,
                    }
                };
                out.extend(positions(inner, cr, sys)?);
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Joins pieces end to end: a line after a line and an arc string after an arc string that meet in one place become
/// one; what is left over is a compound curve.
fn join(pieces: Vec<Curve>) -> Curve {
    let mut flat: Vec<Curve> = Vec::new();
    fn push(flat: &mut Vec<Curve>, c: Curve) {
        match c {
            Curve::Compound(parts) => parts.into_iter().for_each(|p| push(flat, p)),
            c => flat.push(c),
        }
    }
    for p in pieces {
        push(&mut flat, p);
    }
    let mut out: Vec<Curve> = Vec::new();
    for p in flat {
        let joined = match (out.last_mut(), &p) {
            (Some(Curve::Line(a)), Curve::Line(b)) | (Some(Curve::Arc(a)), Curve::Arc(b))
                if a.last().zip(b.first()).is_some_and(|(l, f)| l.x == f.x && l.y == f.y) =>
            {
                a.extend(b.iter().skip(1).copied());
                true
            }
            _ => false,
        };
        if !joined {
            out.push(p);
        }
    }
    if out.len() == 1 {
        out.remove(0)
    } else {
        Curve::Compound(out)
    }
}

fn arc_points(p: Vec<Coord>) -> Result<Curve, String> {
    if p.len() < 3 || p.len().is_multiple_of(2) {
        return Err(format!(
            "yay dizisinde {} nokta var; 3, 5, 7 … olmalı",
            p.len()
        ));
    }
    Ok(Curve::Arc(p))
}

/// A curve: a line string, a curve of segments, a composite curve, a ring.
fn curve(n: Node<'_, '_>, r: Reading, sys: &mut Systems, depth: usize) -> Result<Curve, String> {
    if depth > DEPTH {
        return Err("geometri çok derin iç içe".into());
    }
    let r = reading_of(n, r, sys);
    match local(n) {
        "LineString" | "LinearRing" => Ok(Curve::Line(positions(n, r, sys)?)),
        "Curve" => {
            let mut pieces = Vec::new();
            for segs in elements(n).filter(|c| local(*c) == "segments") {
                let sr = reading_of(segs, r, sys);
                for s in elements(segs) {
                    let p = positions(s, sr, sys)?;
                    pieces.push(match local(s) {
                        "LineStringSegment" => Curve::Line(p),
                        "ArcString" => arc_points(p)?,
                        "Arc" if p.len() == 3 => Curve::Arc(p),
                        "Arc" => return Err("Arc kesiminde 3 nokta olmalı".into()),
                        other => return Err(format!("{other} kesimi okunmuyor")),
                    });
                }
            }
            if pieces.is_empty() {
                return Err("Curve'ün kesimi yok".into());
            }
            Ok(join(pieces))
        }
        "CompositeCurve" | "Ring" => {
            let mut pieces = Vec::new();
            for m in elements(n).filter(|c| local(*c) == "curveMember") {
                let mr = reading_of(m, r, sys);
                for c in elements(m) {
                    pieces.push(curve(c, mr, sys, depth + 1)?);
                }
            }
            if pieces.is_empty() {
                return Err(format!("{} üyesiz", local(n)));
            }
            Ok(join(pieces))
        }
        other => Err(format!("{other} eğri olarak okunmuyor")),
    }
}

/// A polygon's rings, the exterior first (GML 3's `exterior`/`interior`, GML 2's boundaries).
fn rings(n: Node<'_, '_>, r: Reading, sys: &mut Systems, depth: usize) -> Result<Vec<Curve>, String> {
    let r = reading_of(n, r, sys);
    let mut out: Vec<Curve> = Vec::new();
    let mut exterior: Option<Curve> = None;
    for c in elements(n) {
        let cr = reading_of(c, r, sys);
        let outer = matches!(local(c), "exterior" | "outerBoundaryIs");
        let inner = matches!(local(c), "interior" | "innerBoundaryIs");
        if !outer && !inner {
            continue;
        }
        let Some(ring) = elements(c).next() else {
            continue;
        };
        let ring = curve(ring, cr, sys, depth + 1)?;
        if outer {
            exterior = Some(ring);
        } else {
            out.push(ring);
        }
    }
    match exterior {
        Some(e) => {
            out.insert(0, e);
            Ok(out)
        }
        None if out.is_empty() => Ok(out),
        None => Err("alanın dış halkası yok".into()),
    }
}

/// A surface's polygons: a polygon, or a surface's patches.
fn surface(
    n: Node<'_, '_>,
    r: Reading,
    sys: &mut Systems,
    depth: usize,
) -> Result<Vec<Vec<Curve>>, String> {
    if depth > DEPTH {
        return Err("geometri çok derin iç içe".into());
    }
    let r = reading_of(n, r, sys);
    match local(n) {
        "Polygon" | "PolygonPatch" | "Rectangle" | "Triangle" => Ok(vec![rings(n, r, sys, depth)?]),
        "Surface" | "CompositeSurface" => {
            let mut out = Vec::new();
            for c in elements(n) {
                let cr = reading_of(c, r, sys);
                match local(c) {
                    "patches" => {
                        for p in elements(c) {
                            out.extend(surface(p, cr, sys, depth + 1)?);
                        }
                    }
                    "surfaceMember" => {
                        for p in elements(c) {
                            out.extend(surface(p, cr, sys, depth + 1)?);
                        }
                    }
                    _ => {}
                }
            }
            if out.is_empty() {
                return Err(format!("{} yamasız", local(n)));
            }
            Ok(out)
        }
        other => Err(format!("{other} yüzey olarak okunmuyor")),
    }
}

fn members<'a, 'b: 'a>(
    n: Node<'a, 'b>,
    singular: &'a [&'a str],
    plural: &'a [&'a str],
) -> impl Iterator<Item = Node<'a, 'b>> {
    elements(n).flat_map(move |m| {
        let one = singular.contains(&local(m));
        let many = plural.contains(&local(m));
        let inner: Vec<Node<'a, 'b>> = if one || many { elements(m).collect() } else { Vec::new() };
        inner.into_iter()
    })
}

fn geometry(
    n: Node<'_, '_>,
    r: Reading,
    sys: &mut Systems,
    depth: usize,
) -> Result<Geometry, String> {
    if depth > DEPTH {
        return Err("geometri çok derin iç içe".into());
    }
    let r = reading_of(n, r, sys);
    Ok(match local(n) {
        "Point" => Geometry::Point(positions(n, r, sys)?.into_iter().next()),
        "LineString" | "LinearRing" | "Curve" | "CompositeCurve" | "Ring" => {
            Geometry::Curve(curve(n, r, sys, depth)?)
        }
        "Polygon" => Geometry::Polygon(rings(n, r, sys, depth)?),
        "Surface" | "CompositeSurface" => {
            let mut polys = surface(n, r, sys, depth)?;
            if polys.len() == 1 {
                Geometry::Polygon(polys.remove(0))
            } else {
                Geometry::MultiPolygon(polys)
            }
        }
        "MultiPoint" => {
            let mut pts = Vec::new();
            for p in members(n, &["pointMember"], &["pointMembers"]) {
                pts.extend(positions(p, r, sys)?.into_iter().take(1));
            }
            Geometry::MultiPoint(pts)
        }
        "MultiCurve" | "MultiLineString" => {
            let mut cs = Vec::new();
            for c in members(
                n,
                &["curveMember", "lineStringMember"],
                &["curveMembers"],
            ) {
                cs.push(curve(c, r, sys, depth + 1)?);
            }
            Geometry::MultiCurve(cs)
        }
        "MultiSurface" | "MultiPolygon" => {
            let mut ps = Vec::new();
            for s in members(
                n,
                &["surfaceMember", "polygonMember"],
                &["surfaceMembers"],
            ) {
                ps.extend(surface(s, r, sys, depth + 1)?);
            }
            Geometry::MultiPolygon(ps)
        }
        "MultiGeometry" => {
            let mut gs = Vec::new();
            for g in members(n, &["geometryMember"], &["geometryMembers"]) {
                gs.push(geometry(g, r, sys, depth + 1)?);
            }
            Geometry::Collection(gs)
        }
        other => return Err(format!("{other} geometrisi okunmuyor")),
    })
}

/// Whether the element is a geometry of GML.
fn is_geometry_element(n: Node<'_, '_>) -> bool {
    is_gml(n)
        && matches!(
            local(n),
            "Point"
                | "LineString"
                | "LinearRing"
                | "Curve"
                | "CompositeCurve"
                | "Ring"
                | "OrientableCurve"
                | "Polygon"
                | "Surface"
                | "CompositeSurface"
                | "OrientableSurface"
                | "MultiPoint"
                | "MultiCurve"
                | "MultiLineString"
                | "MultiSurface"
                | "MultiPolygon"
                | "MultiGeometry"
                | "Solid"
                | "MultiSolid"
                | "TriangulatedSurface"
                | "Tin"
                | "PolyhedralSurface"
        )
}

// ── Declared types ──────────────────────────────────────────────────────────

/// The types a schema or a `.gfs` declares: each feature type's properties in order.
#[derive(Default, Debug)]
struct Declared {
    types: BTreeMap<String, Vec<(String, ColumnKind)>>,
    /// Property names that hold geometry.
    geometries: BTreeSet<String>,
}

fn xsd_kind(base: &str) -> Option<ColumnKind> {
    let b = base.rsplit(':').next().unwrap_or(base);
    Some(match b {
        "integer" | "int" | "long" | "short" | "byte" | "nonNegativeInteger" | "positiveInteger"
        | "negativeInteger" | "nonPositiveInteger" | "unsignedInt" | "unsignedLong"
        | "unsignedShort" | "unsignedByte" => ColumnKind::Integer,
        "decimal" | "double" | "float" => ColumnKind::Real,
        "boolean" => ColumnKind::Boolean,
        "date" => ColumnKind::Date,
        "dateTime" => ColumnKind::DateTime,
        t if t.ends_with("PropertyType") || t.contains("Geometry") => return None,
        _ => ColumnKind::Text(None),
    })
}

/// An `xs:element`'s type: its `type`, or an inline restriction's base and `maxLength`.
fn element_kind(e: Node<'_, '_>) -> Option<ColumnKind> {
    if let Some(t) = attr(e, "type") {
        return xsd_kind(t);
    }
    let restriction = e
        .descendants()
        .find(|d| d.is_element() && local(*d) == "restriction")?;
    let mut kind = xsd_kind(attr(restriction, "base")?)?;
    for f in elements(restriction) {
        let n = attr(f, "value").and_then(|v| v.parse::<u32>().ok());
        if matches!(local(f), "maxLength" | "length")
            && matches!(kind, ColumnKind::Text(_))
            && let Some(n) = n.filter(|n| *n > 0)
        {
            kind = ColumnKind::Text(Some(n));
        }
    }
    Some(kind)
}

fn parse_schema(text: &str) -> Option<Declared> {
    let doc = xml::document(text).ok()?;
    let mut by_type: HashMap<String, Vec<(String, ColumnKind)>> = HashMap::new();
    let mut geometries = BTreeSet::new();
    for ct in doc
        .descendants()
        .filter(|d| d.is_element() && local(*d) == "complexType")
    {
        let Some(name) = attr(ct, "name") else {
            continue;
        };
        let mut fields = Vec::new();
        for e in ct.descendants().filter(|d| d.is_element() && local(*d) == "element") {
            let Some(pn) = attr(e, "name") else {
                continue;
            };
            // A property with a content model of its own (the collection's members) is no simple property.
            if elements(e).any(|c| local(c) == "complexType") {
                continue;
            }
            match element_kind(e) {
                Some(k) => fields.push((pn.to_owned(), k)),
                None => {
                    geometries.insert(pn.to_owned());
                }
            }
        }
        by_type.insert(name.to_owned(), fields);
    }
    let mut out = Declared {
        geometries,
        ..Declared::default()
    };
    for e in elements(doc.root_element()).filter(|d| local(*d) == "element") {
        let (Some(name), Some(t)) = (attr(e, "name"), attr(e, "type")) else {
            continue;
        };
        let t = t.rsplit(':').next().unwrap_or(t);
        if let Some(fields) = by_type.get(t) {
            out.types.insert(name.to_owned(), fields.clone());
        }
    }
    (!out.types.is_empty()).then_some(out)
}

fn parse_gfs(text: &str) -> Option<Declared> {
    let doc = xml::document(text).ok()?;
    let mut out = Declared::default();
    for class in doc
        .descendants()
        .filter(|d| d.is_element() && local(*d) == "GMLFeatureClass")
    {
        let child = |n: Node<'_, '_>, k: &str| {
            elements(n)
                .find(|c| local(*c) == k)
                .map(|c| text_of(c).trim().to_owned())
        };
        let Some(name) = child(class, "Name") else {
            continue;
        };
        let mut fields = Vec::new();
        for p in elements(class).filter(|c| local(*c) == "PropertyDefn") {
            let (Some(pn), Some(t)) = (child(p, "Name"), child(p, "Type")) else {
                continue;
            };
            let path = child(p, "ElementPath").unwrap_or_default();
            if path.starts_with("boundedBy|") || pn == "gml_id" {
                continue;
            }
            let width = child(p, "Width").and_then(|w| w.parse::<u32>().ok()).filter(|w| *w > 0);
            let kind = match (child(p, "Subtype").as_deref(), t.as_str()) {
                (Some("Boolean"), _) => ColumnKind::Boolean,
                (Some("Date"), _) => ColumnKind::Date,
                (Some("DateTime"), _) => ColumnKind::DateTime,
                (_, "Integer") => ColumnKind::Integer,
                (_, "Real") => ColumnKind::Real,
                (_, "String") => ColumnKind::Text(width),
                _ => ColumnKind::Text(None),
            };
            fields.push((pn, kind));
        }
        out.types.insert(name, fields);
    }
    (!out.types.is_empty()).then_some(out)
}

// ── Values ──────────────────────────────────────────────────────────────────

fn is_integer(t: &str) -> bool {
    let d = t.strip_prefix('-').unwrap_or(t);
    !d.is_empty()
        && d.bytes().all(|b| b.is_ascii_digit())
        && (d == "0" || !d.starts_with('0'))
        && t.parse::<i64>().is_ok()
}

fn is_real(t: &str) -> bool {
    let d = t.strip_prefix('-').unwrap_or(t);
    let (mantissa, exp) = match d.find(['e', 'E']) {
        Some(i) => (&d[..i], Some(&d[i + 1..])),
        None => (d, None),
    };
    let (int, frac) = match mantissa.split_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (mantissa, None),
    };
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    let exp_ok = exp.is_none_or(|e| {
        let e = e.strip_prefix(['+', '-']).unwrap_or(e);
        !e.is_empty() && digits(e)
    });
    digits(int)
        && frac.is_none_or(digits)
        && (!int.is_empty() || frac.is_some_and(|f| !f.is_empty()))
        && exp_ok
        // Digits only with a leading zero (an identifier "007") are no number.
        && !(frac.is_none() && exp.is_none() && int.len() > 1 && int.starts_with('0'))
        && t.parse::<f64>().is_ok_and(f64::is_finite)
}

fn is_date(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

fn is_datetime(t: &str) -> bool {
    let (Some(date), Some(rest)) = (t.get(..10), t.get(10..)) else {
        return false;
    };
    if !is_date(date) {
        return false;
    }
    let Some(time) = rest.strip_prefix('T') else {
        return false;
    };
    let b = time.as_bytes();
    if b.len() < 8 || b[2] != b':' || b[5] != b':' {
        return false;
    }
    let clock_ok = [0usize, 1, 3, 4, 6, 7].iter().all(|&i| b[i].is_ascii_digit());
    let tail = &time[8..];
    let frac = tail.strip_prefix('.').map_or(tail, |t| t.trim_start_matches(|c: char| c.is_ascii_digit()));
    let zone_ok = frac.is_empty()
        || frac == "Z"
        || (frac.len() == 6
            && (frac.starts_with('+') || frac.starts_with('-'))
            && frac.as_bytes()[3] == b':'
            && [1usize, 2, 4, 5].iter().all(|&i| frac.as_bytes()[i].is_ascii_digit()));
    clock_ok && zone_ok
}

fn is_boolean(t: &str) -> bool {
    matches!(t, "true" | "false")
}

/// Whether a value reads as a kind.
fn fits(kind: &ColumnKind, t: &str) -> bool {
    match kind {
        ColumnKind::Integer => t.trim().parse::<i64>().is_ok(),
        ColumnKind::Real => t.trim().parse::<f64>().is_ok_and(f64::is_finite),
        ColumnKind::Boolean => matches!(t.trim(), "true" | "false" | "1" | "0"),
        ColumnKind::Date => is_date(t.trim()),
        ColumnKind::DateTime => is_datetime(t.trim()),
        _ => true,
    }
}

/// The kind the values say: every one an integer, a number, a boolean, a date, a date and time, else text.
fn infer(values: &[&str]) -> ColumnKind {
    let vals: Vec<&str> = values.iter().copied().filter(|v| !v.is_empty()).collect();
    if vals.is_empty() {
        return ColumnKind::Text(None);
    }
    if vals.iter().all(|v| is_integer(v)) {
        ColumnKind::Integer
    } else if vals.iter().all(|v| is_real(v)) {
        ColumnKind::Real
    } else if vals.iter().all(|v| is_boolean(v)) {
        ColumnKind::Boolean
    } else if vals.iter().all(|v| is_date(v)) {
        ColumnKind::Date
    } else if vals.iter().all(|v| is_datetime(v)) {
        ColumnKind::DateTime
    } else {
        ColumnKind::Text(None)
    }
}

fn value_of(kind: &ColumnKind, t: &str) -> Value {
    let s = t.trim();
    match kind {
        ColumnKind::Integer => s.parse::<i64>().map_or(Value::Null, Value::Integer),
        ColumnKind::Real => s.parse::<f64>().map_or(Value::Null, Value::Real),
        ColumnKind::Boolean => Value::Integer(i64::from(matches!(s, "true" | "1"))),
        ColumnKind::Date | ColumnKind::DateTime => Value::Text(s.to_owned()),
        _ => Value::Text(t.to_owned()),
    }
}

// ── The document ────────────────────────────────────────────────────────────

struct RawRow {
    geometry: Option<Result<Geometry, String>>,
    props: Vec<(String, Option<String>)>,
}

/// A property's leaf values: nested simple elements flatten to `outer_inner`.
fn flatten(n: Node<'_, '_>, name: &str, out: &mut Vec<(String, Option<String>)>, depth: usize) {
    let kids: Vec<Node<'_, '_>> = elements(n).filter(|c| !is_gml(*c)).collect();
    if kids.is_empty() || depth > 8 {
        let nil = attr(n, "nil").is_some_and(|v| v == "true" || v == "1");
        out.push((name.to_owned(), if nil { None } else { Some(text_of(n)) }));
        return;
    }
    for k in kids {
        flatten(k, &format!("{name}_{}", local(k)), out, depth + 1);
    }
}

/// The features of a collection: its members' elements.
fn features<'a, 'b: 'a>(root: Node<'a, 'b>) -> Vec<Node<'a, 'b>> {
    let mut out = Vec::new();
    for c in elements(root) {
        match local(c) {
            "featureMember" | "member" => out.extend(elements(c).next()),
            "featureMembers" | "members" => out.extend(elements(c)),
            _ => {}
        }
    }
    out
}

/// The text read as GML with the schema and `.gfs` beside it, when there are.
pub fn parse(text: &str, schema: Option<&str>, gfs: Option<&str>) -> Result<Parsed, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let doc = xml::document(text).map_err(|e| match e {
        xml::Unparsed::NotXml => "XML belgesi değil ya da bozuk".to_owned(),
        xml::Unparsed::TooDeep => "XML öğeleri çok derin iç içe".to_owned(),
    })?;
    let root = doc.root_element();
    let mut parsed = Parsed::default();
    let mut sys = Systems::default();
    // The collection's own envelope names the system when no geometry does.
    let mut fallback = Reading { dim: 2, swap: false };
    if let Some(b) = elements(root).find(|c| local(*c) == "boundedBy")
        && let Some(name) = elements(b).filter_map(|e| attr(e, "srsName")).next()
    {
        fallback.swap = swaps(name);
    }
    let feats = features(root);
    // A collection with no member is a collection (nothing to take); a document that is neither is no GML.
    let has_members = elements(root).any(|c| {
        matches!(local(c), "featureMember" | "member" | "featureMembers" | "members")
    });
    if feats.is_empty() && !has_members && !local(root).ends_with("Collection") {
        return Err("GML özellik topluluğu değil (featureMember, featureMembers ya da member yok)".into());
    }
    let declared = schema
        .map(|s| (parse_schema(s), Typed::Schema))
        .filter(|(d, _)| d.is_some())
        .or_else(|| gfs.map(|g| (parse_gfs(g), Typed::Gfs)).filter(|(d, _)| d.is_some()));
    if schema.is_some() && !matches!(declared, Some((_, Typed::Schema))) {
        parsed.losses.insert(
            ".xsd şeması okunamadı ya da özellik türü tanımlamıyor: türler .gfs'den ya da değerlerden çıkarıldı".into(),
        );
    }
    let (declared, typed) = match declared {
        Some((Some(d), t)) => (d, t),
        _ => (Declared::default(), Typed::Values),
    };
    parsed.typed_by = typed;
    let mut order: Vec<String> = Vec::new();
    let mut by_name: HashMap<String, Vec<RawRow>> = HashMap::new();
    let (mut second, mut repeated) = (0u32, 0u32);
    for f in feats {
        let name = local(f).to_owned();
        let mut geometry: Option<Result<Geometry, String>> = None;
        let mut props: Vec<(String, Option<String>)> = Vec::new();
        for c in elements(f) {
            if is_gml(c) {
                continue;
            }
            let geom_child = elements(c).find(|e| is_geometry_element(*e));
            let named_geometry = declared.geometries.contains(local(c)) || local(c) == "geometryProperty";
            if geom_child.is_some() || (named_geometry && elements(c).next().is_none()) {
                if geometry.is_some() {
                    second += 1;
                    continue;
                }
                if let Some(g) = geom_child {
                    let r = reading_of(c, fallback, &mut sys);
                    geometry = Some(geometry_of(g, r, &mut sys));
                }
                continue;
            }
            let mut leaves = Vec::new();
            flatten(c, local(c), &mut leaves, 0);
            for (k, v) in leaves {
                if props.iter().any(|(p, _)| *p == k) {
                    repeated += 1;
                } else {
                    props.push((k, v));
                }
            }
        }
        if !by_name.contains_key(&name) {
            order.push(name.clone());
        }
        by_name.entry(name).or_default().push(RawRow { geometry, props });
    }
    if second > 0 {
        parsed
            .losses
            .insert(format!("{second} nesnenin ikinci geometri özelliği alınmadı"));
    }
    if repeated > 0 {
        parsed.losses.insert(format!(
            "{repeated} tekrarlanan özellik öğesinde ilk değer alındı"
        ));
    }
    for name in order {
        let raw = by_name.remove(&name).unwrap_or_default();
        parsed.tables.push(table(name, raw, &declared, &mut parsed.losses));
    }
    parsed.srid = sys.seen.first().copied();
    parsed.other_crs = sys.other;
    if sys.seen.len() > 1 {
        parsed.losses.insert(format!(
            "geometriler birden çok sistem bildiriyor ({}); ilki esas alındı",
            sys.seen
                .iter()
                .map(|s| format!("EPSG:{s}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if parsed.srid.is_none() && parsed.other_crs.is_none() {
        // The envelope may still name it.
        if let Some(name) = elements(root)
            .find(|c| local(*c) == "boundedBy")
            .and_then(|b| elements(b).filter_map(|e| attr(e, "srsName")).next())
        {
            match crs_name(name) {
                Some(c) => parsed.srid = Some(c.srid),
                None => parsed.other_crs = Some(name.to_owned()),
            }
        }
    }
    Ok(parsed)
}

fn geometry_of(n: Node<'_, '_>, r: Reading, sys: &mut Systems) -> Result<Geometry, String> {
    geometry(n, r, sys, 0)
}

fn table(
    name: String,
    raw: Vec<RawRow>,
    declared: &Declared,
    losses: &mut BTreeSet<String>,
) -> Table {
    // The columns: the declared properties in order, then the others as they come.
    let mut names: Vec<String> = Vec::new();
    let mut kinds: HashMap<String, ColumnKind> = HashMap::new();
    if let Some(fields) = declared.types.get(&name) {
        for (n, k) in fields {
            if !names.contains(n) {
                names.push(n.clone());
                kinds.insert(n.clone(), k.clone());
            }
        }
    }
    for r in &raw {
        for (k, _) in &r.props {
            if !names.contains(k) {
                names.push(k.clone());
            }
        }
    }
    let lookup: Vec<HashMap<&str, &Option<String>>> = raw
        .iter()
        .map(|r| r.props.iter().map(|(k, v)| (k.as_str(), v)).collect())
        .collect();
    let mut columns = Vec::new();
    for n in &names {
        let values: Vec<&str> = lookup
            .iter()
            .filter_map(|m| m.get(n.as_str()).and_then(|v| v.as_deref()))
            .collect();
        let kind = match kinds.get(n) {
            Some(k) => {
                let k = k.clone();
                if values.iter().filter(|v| !v.trim().is_empty()).all(|v| fits(&k, v)) {
                    k
                } else {
                    losses.insert(format!(
                        "“{n}” özelliğinin değerleri bildirilen türe uymuyor: metin alanı olarak alındı"
                    ));
                    ColumnKind::Text(None)
                }
            }
            None => infer(&values),
        };
        columns.push(Column {
            name: n.clone(),
            decl: String::new(),
            kind,
            not_null: false,
            default: None,
        });
    }
    let rows = raw
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let m: HashMap<&str, &Option<String>> =
                r.props.iter().map(|(k, v)| (k.as_str(), v)).collect();
            let values = columns
                .iter()
                .map(|c| match m.get(c.name.as_str()) {
                    Some(Some(t)) if matches!(c.kind, ColumnKind::Text(_)) => Value::Text(t.clone()),
                    Some(Some(t)) if t.trim().is_empty() => Value::Null,
                    Some(Some(t)) => value_of(&c.kind, t),
                    _ => Value::Null,
                })
                .collect();
            Row {
                fid: i as i64 + 1,
                geometry: r.geometry.map(|g| g.map_err(BlobError)),
                values,
            }
        })
        .collect();
    Table { name, columns, rows }
}
