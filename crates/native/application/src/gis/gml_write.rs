//! Writing GML 3.2 (docs/adr/0545, plan/gis.md CBS-05) as GDAL's GML driver writes it (`FORMAT=GML3.2`): a
//! `FeatureCollection` of `featureMember`s, each feature an element named for its feature type with a `gml:boundedBy`
//! envelope, the geometry in `geometryProperty` and the properties after it (a null as `xsi:nil`), and the application
//! schema (`.xsd`) of the feature types. Geometries: `Point`, `LineString`, `Polygon` (`exterior`, `interior`),
//! `MultiPoint`, `MultiCurve`, `MultiSurface`, `MultiGeometry`; arcs and compound curves as a `Curve` of an `ArcString`,
//! a `Ring` of `curveMember`s and a `CompositeCurve`, exact. The system is `urn:ogc:def:crs:EPSG::<code>` and the
//! coordinates go in the order EPSG gives its axes (north or latitude first for TUREF's TM zones and the geographic
//! systems).

use std::fmt::Write as _;

use kentos_geopackage::{ColumnKind, Coord, Curve, Geometry, Value};
use kentos_services::crs::{Axis, axis_order};

/// GDAL's default namespace for the feature types and the prefix it writes them with.
const NAMESPACE: &str = "http://ogr.maptools.org/";

/// A property written.
#[derive(Clone, Debug)]
pub struct OutColumn {
    pub name: String,
    pub kind: ColumnKind,
}

/// A feature type written: its element name, properties and features.
#[derive(Clone, Debug)]
pub struct OutType {
    pub name: String,
    pub columns: Vec<OutColumn>,
    pub features: Vec<(Option<Geometry>, Vec<Value>)>,
}

/// What a write noted it could not hold.
#[derive(Debug, Default)]
pub struct Notes {
    /// Positions of a geometry with elevations that had none, written with 0.
    pub zero_z: u32,
    /// Geometries with M values (dropped).
    pub with_m: u32,
    /// Control characters dropped from text.
    pub control: u32,
    /// Non-finite numbers written as nil.
    pub non_finite: u32,
}

/// The geometry property's type in the schema: `PolygonPropertyType` …
pub fn property_type(geoms: &[&Geometry]) -> &'static str {
    // 0 point, 1 multi point, 2 line, 3 curve, 4 polygon, 5 curve polygon, 6 multi curve, 7 multi surface
    let class = |g: &Geometry| -> u8 {
        match g {
            Geometry::Point(_) => 0,
            Geometry::MultiPoint(_) => 1,
            Geometry::Curve(Curve::Line(_)) => 2,
            Geometry::Curve(_) => 3,
            Geometry::Polygon(r) if r.iter().any(Curve::is_curved) => 5,
            Geometry::Polygon(_) => 4,
            Geometry::MultiCurve(_) => 6,
            Geometry::MultiPolygon(_) => 7,
            Geometry::Collection(_) => 8,
        }
    };
    let mut classes: Vec<u8> = geoms.iter().map(|g| class(g)).collect();
    classes.sort_unstable();
    classes.dedup();
    match classes.as_slice() {
        [0] => "PointPropertyType",
        [1] => "MultiPointPropertyType",
        [2] => "LineStringPropertyType",
        [3] | [2, 3] => "CurvePropertyType",
        [4] => "PolygonPropertyType",
        [5] | [4, 5] => "SurfacePropertyType",
        [6] => "MultiCurvePropertyType",
        [7] => "MultiSurfacePropertyType",
        _ => "GeometryPropertyType",
    }
}

fn num(v: f64) -> String {
    format!("{v}")
}

/// An XML element or attribute name from a text: letters, digits, `_`, `-` and `.`, a letter or `_` first.
pub fn xml_name(text: &str) -> String {
    let mut out: String = text
        .chars()
        .map(|c| if c.is_alphanumeric() || matches!(c, '_' | '-' | '.') { c } else { '_' })
        .collect();
    if !out.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_') {
        out.insert(0, '_');
    }
    out
}

fn escape(t: &str, notes: &mut Notes, out: &mut String) {
    for c in t.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            '\t' | '\n' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{fffe}' || c == '\u{ffff}' => notes.control += 1,
            c => out.push(c),
        }
    }
}

/// A file name as a schema location: the location list is split at white space, so a name with a space (or a mark that
/// is no part of a URI reference) is percent-encoded.
fn uri_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '%' => out.push_str("%25"),
            '"' => out.push_str("%22"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '&' => out.push_str("%26"),
            '\t' | '\n' | '\r' => out.push_str(&format!("%{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// The system and how its coordinates are written.
#[derive(Clone, Debug)]
struct Srs {
    name: Option<String>,
    swap: bool,
}

impl Srs {
    fn of(srid: Option<u32>) -> Srs {
        match srid {
            Some(s) => Srs {
                name: Some(format!("urn:ogc:def:crs:EPSG::{s}")),
                swap: matches!(axis_order(s), Axis::Ne | Axis::LatLon),
            },
            None => Srs {
                name: None,
                swap: false,
            },
        }
    }

    fn attr(&self) -> String {
        self.name
            .as_ref()
            .map_or_else(String::new, |n| format!(" srsName=\"{n}\""))
    }

    fn pair(&self, c: &Coord) -> (f64, f64) {
        if self.swap { (c.y, c.x) } else { (c.x, c.y) }
    }
}

fn walk(g: &Geometry, f: &mut dyn FnMut(&Coord)) {
    fn curve(c: &Curve, f: &mut dyn FnMut(&Coord)) {
        match c {
            Curve::Line(p) | Curve::Arc(p) => p.iter().for_each(&mut *f),
            Curve::Compound(parts) => parts.iter().for_each(|p| curve(p, f)),
        }
    }
    match g {
        Geometry::Point(p) => p.iter().for_each(&mut *f),
        Geometry::MultiPoint(p) => p.iter().for_each(&mut *f),
        Geometry::Curve(c) => curve(c, f),
        Geometry::Polygon(r) | Geometry::MultiCurve(r) => r.iter().for_each(|c| curve(c, f)),
        Geometry::MultiPolygon(p) => p.iter().flatten().for_each(|c| curve(c, f)),
        Geometry::Collection(gs) => gs.iter().for_each(|g| walk(g, f)),
    }
}

/// The box of a geometry, arcs by their true extent: (min x, min y, max x, max y) and the elevations' range.
fn extent(g: &Geometry) -> Option<([f64; 4], Option<[f64; 2]>)> {
    let mut b: Option<[f64; 4]> = None;
    let mut z: Option<[f64; 2]> = None;
    let mut take = |x: f64, y: f64| {
        b = Some(match b {
            None => [x, y, x, y],
            Some(v) => [v[0].min(x), v[1].min(y), v[2].max(x), v[3].max(y)],
        });
    };
    walk(g, &mut |c| {
        take(c.x, c.y);
        if let Some(v) = c.z {
            z = Some(match z {
                None => [v, v],
                Some(r) => [r[0].min(v), r[1].max(v)],
            });
        }
    });
    arcs(g, &mut |x, y| take(x, y));
    Some((b?, z))
}

/// The points of an arc's circle at the compass directions it passes, for the box.
fn arcs(g: &Geometry, f: &mut dyn FnMut(f64, f64)) {
    fn curve(c: &Curve, f: &mut dyn FnMut(f64, f64)) {
        match c {
            Curve::Line(_) => {}
            Curve::Arc(p) => {
                for w in p.windows(3).step_by(2) {
                    arc_extremes(&w[0], &w[1], &w[2], f);
                }
            }
            Curve::Compound(parts) => parts.iter().for_each(|p| curve(p, f)),
        }
    }
    match g {
        Geometry::Curve(c) => curve(c, f),
        Geometry::Polygon(r) | Geometry::MultiCurve(r) => r.iter().for_each(|c| curve(c, f)),
        Geometry::MultiPolygon(p) => p.iter().flatten().for_each(|c| curve(c, f)),
        Geometry::Collection(gs) => gs.iter().for_each(|g| arcs(g, f)),
        _ => {}
    }
}

fn arc_extremes(a: &Coord, m: &Coord, b: &Coord, f: &mut dyn FnMut(f64, f64)) {
    let d = 2.0 * (a.x * (m.y - b.y) + m.x * (b.y - a.y) + b.x * (a.y - m.y));
    if d == 0.0 || !d.is_finite() {
        return;
    }
    let sq = |c: &Coord| c.x * c.x + c.y * c.y;
    let cx = (sq(a) * (m.y - b.y) + sq(m) * (b.y - a.y) + sq(b) * (a.y - m.y)) / d;
    let cy = (sq(a) * (b.x - m.x) + sq(m) * (a.x - b.x) + sq(b) * (m.x - a.x)) / d;
    let r = (a.x - cx).hypot(a.y - cy);
    let tau = std::f64::consts::TAU;
    let ang = |c: &Coord| (c.y - cy).atan2(c.x - cx);
    let (t0, t2) = (ang(a), ang(b));
    let ccw = (m.x - a.x) * (b.y - a.y) - (m.y - a.y) * (b.x - a.x) > 0.0;
    // The sweep from a to b through m, counter-clockwise positive.
    let s = if ccw { (t2 - t0).rem_euclid(tau) } else { -(t0 - t2).rem_euclid(tau) };
    for k in 0..4 {
        let t = k as f64 * std::f64::consts::FRAC_PI_2;
        let along = if ccw { (t - t0).rem_euclid(tau) } else { -(t0 - t).rem_euclid(tau) };
        if along.abs() <= s.abs() {
            f(cx + r * t.cos(), cy + r * t.sin());
        }
    }
}

struct W<'a> {
    srs: &'a Srs,
    notes: &'a mut Notes,
}

impl W<'_> {
    /// The positions of a coordinate list as a `posList`'s text; with elevations when `z`.
    fn list(&mut self, pts: &[Coord], z: bool) -> String {
        let mut s = String::new();
        for (i, c) in pts.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(&self.pos(c, z));
        }
        s
    }

    fn pos(&mut self, c: &Coord, z: bool) -> String {
        let (a, b) = self.srs.pair(c);
        let mut s = format!("{} {}", num(a), num(b));
        if z {
            let v = c.z.unwrap_or_else(|| {
                self.notes.zero_z += 1;
                0.0
            });
            let _ = write!(s, " {}", num(v));
        }
        s
    }

    fn pos_list(&mut self, pts: &[Coord], z: bool) -> String {
        // The dimension always: a reader that took the collection's envelope for the default (GDAL) would read the
        // two-dimensional lists of a file with an elevation somewhere as three.
        let dim = if z { 3 } else { 2 };
        format!("<gml:posList srsDimension=\"{dim}\">{}</gml:posList>", self.list(pts, z))
    }

    fn attrs(&self, id: &str, top: bool) -> String {
        if top {
            format!("{} gml:id=\"{id}\"", self.srs.attr())
        } else {
            format!(" gml:id=\"{id}\"")
        }
    }

    fn line_or_arc(&mut self, c: &Curve, id: &str, top: bool, z: bool, out: &mut String) {
        let a = self.attrs(id, top);
        match c {
            Curve::Line(p) => {
                let _ = write!(out, "<gml:LineString{a}>{}</gml:LineString>", self.pos_list(p, z));
            }
            Curve::Arc(p) => {
                let _ = write!(
                    out,
                    "<gml:Curve{a}><gml:segments><gml:ArcString>{}</gml:ArcString></gml:segments></gml:Curve>",
                    self.pos_list(p, z)
                );
            }
            Curve::Compound(parts) => {
                let _ = write!(out, "<gml:CompositeCurve{a}>");
                for (i, p) in flat(parts).into_iter().enumerate() {
                    out.push_str("<gml:curveMember>");
                    self.line_or_arc(p, &format!("{id}.{i}"), false, z, out);
                    out.push_str("</gml:curveMember>");
                }
                out.push_str("</gml:CompositeCurve>");
            }
        }
    }

    fn ring(&mut self, c: &Curve, id: &str, z: bool, out: &mut String) {
        if let Curve::Line(p) = c {
            let _ = write!(
                out,
                "<gml:LinearRing>{}</gml:LinearRing>",
                self.pos_list(p, z)
            );
            return;
        }
        out.push_str("<gml:Ring>");
        for (i, p) in flat(std::slice::from_ref(c)).into_iter().enumerate() {
            out.push_str("<gml:curveMember>");
            self.line_or_arc(p, &format!("{id}.{i}"), false, z, out);
            out.push_str("</gml:curveMember>");
        }
        out.push_str("</gml:Ring>");
    }

    fn polygon(&mut self, rings: &[Curve], id: &str, top: bool, z: bool, out: &mut String) {
        let a = self.attrs(id, top);
        let _ = write!(out, "<gml:Polygon{a}>");
        for (i, r) in rings.iter().enumerate() {
            let (tag, rid) = if i == 0 {
                ("exterior", format!("{id}.exterior"))
            } else {
                ("interior", format!("{id}.interior{i}"))
            };
            let _ = write!(out, "<gml:{tag}>");
            self.ring(r, &rid, z, out);
            let _ = write!(out, "</gml:{tag}>");
        }
        out.push_str("</gml:Polygon>");
    }

    fn geometry(&mut self, g: &Geometry, id: &str, top: bool, z: bool, out: &mut String) {
        let a = self.attrs(id, top);
        match g {
            Geometry::Point(Some(c)) => {
                let _ = write!(out, "<gml:Point{a}><gml:pos>{}</gml:pos></gml:Point>", self.pos(c, z));
            }
            Geometry::Point(None) => {}
            Geometry::Curve(c) => self.line_or_arc(c, id, top, z, out),
            Geometry::Polygon(r) => self.polygon(r, id, top, z, out),
            Geometry::MultiPoint(p) => {
                let _ = write!(out, "<gml:MultiPoint{a}>");
                for (i, c) in p.iter().enumerate() {
                    let _ = write!(
                        out,
                        "<gml:pointMember><gml:Point gml:id=\"{id}.{i}\"><gml:pos>{}</gml:pos></gml:Point></gml:pointMember>",
                        self.pos(c, z)
                    );
                }
                out.push_str("</gml:MultiPoint>");
            }
            Geometry::MultiCurve(cs) => {
                let _ = write!(out, "<gml:MultiCurve{a}>");
                for (i, c) in cs.iter().enumerate() {
                    out.push_str("<gml:curveMember>");
                    self.line_or_arc(c, &format!("{id}.{i}"), false, z, out);
                    out.push_str("</gml:curveMember>");
                }
                out.push_str("</gml:MultiCurve>");
            }
            Geometry::MultiPolygon(ps) => {
                let _ = write!(out, "<gml:MultiSurface{a}>");
                for (i, p) in ps.iter().enumerate() {
                    out.push_str("<gml:surfaceMember>");
                    self.polygon(p, &format!("{id}.{i}"), false, z, out);
                    out.push_str("</gml:surfaceMember>");
                }
                out.push_str("</gml:MultiSurface>");
            }
            Geometry::Collection(gs) => {
                let _ = write!(out, "<gml:MultiGeometry{a}>");
                for (i, m) in gs.iter().enumerate() {
                    out.push_str("<gml:geometryMember>");
                    self.geometry(m, &format!("{id}.{i}"), false, z, out);
                    out.push_str("</gml:geometryMember>");
                }
                out.push_str("</gml:MultiGeometry>");
            }
        }
    }

    fn envelope(&mut self, b: [f64; 4], z: Option<[f64; 2]>, out: &mut String) {
        let (lo, hi) = if self.srs.swap {
            ((b[1], b[0]), (b[3], b[2]))
        } else {
            ((b[0], b[1]), (b[2], b[3]))
        };
        let dim = if z.is_some() { " srsDimension=\"3\"" } else { "" };
        let (zl, zh) = match z {
            Some(r) => (format!(" {}", num(r[0])), format!(" {}", num(r[1]))),
            None => (String::new(), String::new()),
        };
        let _ = write!(
            out,
            "<gml:boundedBy><gml:Envelope{dim}{}><gml:lowerCorner>{} {}{zl}</gml:lowerCorner><gml:upperCorner>{} {}{zh}</gml:upperCorner></gml:Envelope></gml:boundedBy>",
            self.srs.attr(),
            num(lo.0),
            num(lo.1),
            num(hi.0),
            num(hi.1)
        );
    }
}

/// The straight and arc pieces of curves, compounds opened.
fn flat(cs: &[Curve]) -> Vec<&Curve> {
    let mut out = Vec::new();
    for c in cs {
        match c {
            Curve::Compound(parts) => out.extend(flat(parts)),
            c => out.push(c),
        }
    }
    out
}

fn value(v: &Value, kind: &ColumnKind, notes: &mut Notes, out: &mut String) -> bool {
    match (v, kind) {
        (Value::Null, _) => false,
        // Binary as base64 (docs/adr/0629 §4: `xs:base64Binary`).
        (Value::Blob(b), _) => {
            out.push_str(&kentos_commands::encoding::base64_encode(b));
            true
        }
        (Value::Integer(i), ColumnKind::Boolean) => {
            out.push_str(if *i != 0 { "true" } else { "false" });
            true
        }
        (Value::Integer(i), _) => {
            let _ = write!(out, "{i}");
            true
        }
        (Value::Real(r), _) if r.is_finite() => {
            out.push_str(&num(*r));
            true
        }
        (Value::Real(_), _) => {
            notes.non_finite += 1;
            false
        }
        (Value::Text(t), _) => {
            escape(t, notes, out);
            true
        }
    }
}

fn xsd_type(kind: &ColumnKind) -> String {
    match kind {
        ColumnKind::Integer => "<xs:simpleType><xs:restriction base=\"xs:long\"></xs:restriction></xs:simpleType>".into(),
        ColumnKind::Real => "<xs:simpleType><xs:restriction base=\"xs:double\"></xs:restriction></xs:simpleType>".into(),
        ColumnKind::Boolean => "<xs:simpleType><xs:restriction base=\"xs:boolean\"></xs:restriction></xs:simpleType>".into(),
        ColumnKind::Date => String::new(),
        ColumnKind::DateTime => String::new(),
        ColumnKind::Blob(_) => String::new(),
        ColumnKind::Text(Some(n)) => format!(
            "<xs:simpleType><xs:restriction base=\"xs:string\"><xs:maxLength value=\"{n}\"/></xs:restriction></xs:simpleType>"
        ),
        _ => "<xs:simpleType><xs:restriction base=\"xs:string\"></xs:restriction></xs:simpleType>".into(),
    }
}

/// The schema's word for a property's type.
pub fn type_word(kind: &ColumnKind) -> &'static str {
    match kind {
        ColumnKind::Integer => "xs:long",
        ColumnKind::Real => "xs:double",
        ColumnKind::Boolean => "xs:boolean",
        ColumnKind::Date => "xs:date",
        ColumnKind::DateTime => "xs:dateTime",
        ColumnKind::Blob(_) => "xs:base64Binary",
        _ => "xs:string",
    }
}

/// The GML of the feature types, with `schema` the file name of the `.xsd` it points to.
pub fn write(types: &[OutType], srid: Option<u32>, schema: Option<&str>, notes: &mut Notes) -> String {
    let srs = Srs::of(srid);
    let mut w = W { srs: &srs, notes };
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\" ?>\n");
    out.push_str("<ogr:FeatureCollection\n     gml:id=\"aFeatureCollection\"\n     xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n");
    if let Some(s) = schema {
        let _ = writeln!(out, "     xsi:schemaLocation=\"{NAMESPACE} {}\"", uri_name(s));
    }
    let _ = writeln!(out, "     xmlns:ogr=\"{NAMESPACE}\"");
    out.push_str("     xmlns:gml=\"http://www.opengis.net/gml/3.2\">\n");
    // The collection's envelope.
    let mut all: Option<([f64; 4], Option<[f64; 2]>)> = None;
    for t in types {
        for (g, _) in &t.features {
            if let Some((b, z)) = g.as_ref().and_then(extent) {
                all = Some(match all {
                    None => (b, z),
                    Some((a, az)) => (
                        [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
                        match (az, z) {
                            (Some(p), Some(q)) => Some([p[0].min(q[0]), p[1].max(q[1])]),
                            (p, q) => p.or(q),
                        },
                    ),
                });
            }
        }
    }
    out.push_str("  ");
    match all {
        Some((b, z)) => w.envelope(b, z, &mut out),
        None => out.push_str("<gml:boundedBy><gml:Null /></gml:boundedBy>"),
    }
    out.push('\n');
    for t in types {
        for (n, (g, values)) in t.features.iter().enumerate() {
            let fid = format!("{}.{n}", t.name);
            let _ = write!(out, "  <ogr:featureMember>\n    <ogr:{} gml:id=\"{fid}\">\n", t.name);
            if let Some(g) = g.as_ref().filter(|g| !g.is_empty()) {
                let (z, m) = {
                    let (mut z, mut m) = (false, false);
                    walk(g, &mut |c| {
                        z |= c.z.is_some();
                        m |= c.m.is_some();
                    });
                    (z, m)
                };
                if m {
                    w.notes.with_m += 1;
                }
                if let Some((b, zr)) = extent(g) {
                    out.push_str("      ");
                    w.envelope(b, zr, &mut out);
                    out.push('\n');
                }
                out.push_str("      <ogr:geometryProperty>");
                w.geometry(g, &format!("{}.geom.{n}", t.name), true, z, &mut out);
                out.push_str("</ogr:geometryProperty>\n");
            }
            for (c, v) in t.columns.iter().zip(values) {
                let mut text = String::new();
                if value(v, &c.kind, w.notes, &mut text) {
                    let _ = writeln!(out, "      <ogr:{n}>{text}</ogr:{n}>", n = c.name);
                } else {
                    let _ = writeln!(out, "      <ogr:{} xsi:nil=\"true\"/>", c.name);
                }
            }
            let _ = write!(out, "    </ogr:{}>\n  </ogr:featureMember>\n", t.name);
        }
    }
    out.push_str("</ogr:FeatureCollection>\n");
    out
}

/// The application schema of the feature types: each property's type, `geometries` the geometry property's type of
/// each.
pub fn schema(types: &[OutType], geometries: &[&str]) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<xs:schema \n");
    let _ = writeln!(out, "    targetNamespace=\"{NAMESPACE}\"");
    let _ = writeln!(out, "    xmlns:ogr=\"{NAMESPACE}\"");
    out.push_str("    xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"\n    xmlns:gml=\"http://www.opengis.net/gml/3.2\"\n    xmlns:gmlsf=\"http://www.opengis.net/gmlsf/2.0\"\n    elementFormDefault=\"qualified\"\n    version=\"1.0\">\n");
    out.push_str("<xs:annotation>\n  <xs:appinfo source=\"http://schemas.opengis.net/gmlsfProfile/2.0/gmlsfLevels.xsd\">\n    <gmlsf:ComplianceLevel>0</gmlsf:ComplianceLevel>\n  </xs:appinfo>\n</xs:annotation>\n");
    out.push_str("<xs:import namespace=\"http://www.opengis.net/gml/3.2\" schemaLocation=\"http://schemas.opengis.net/gml/3.2.1/gml.xsd\"/>\n");
    out.push_str("<xs:import namespace=\"http://www.opengis.net/gmlsf/2.0\" schemaLocation=\"http://schemas.opengis.net/gmlsfProfile/2.0/gmlsfLevels.xsd\"/>\n");
    out.push_str("<xs:element name=\"FeatureCollection\" type=\"ogr:FeatureCollectionType\" substitutionGroup=\"gml:AbstractFeature\"/>\n");
    out.push_str("<xs:complexType name=\"FeatureCollectionType\">\n  <xs:complexContent>\n    <xs:extension base=\"gml:AbstractFeatureType\">\n      <xs:sequence minOccurs=\"0\" maxOccurs=\"unbounded\">\n        <xs:element name=\"featureMember\">\n          <xs:complexType>\n            <xs:complexContent>\n              <xs:extension base=\"gml:AbstractFeatureMemberType\">\n                <xs:sequence>\n                  <xs:element ref=\"gml:AbstractFeature\"/>\n                </xs:sequence>\n              </xs:extension>\n            </xs:complexContent>\n          </xs:complexType>\n        </xs:element>\n      </xs:sequence>\n    </xs:extension>\n  </xs:complexContent>\n</xs:complexType>\n");
    for (t, geometry) in types.iter().zip(geometries) {
        let n = &t.name;
        let _ = writeln!(
            out,
            "<xs:element name=\"{n}\" type=\"ogr:{n}_Type\" substitutionGroup=\"gml:AbstractFeature\"/>"
        );
        let _ = writeln!(out, "<xs:complexType name=\"{n}_Type\">");
        out.push_str("  <xs:complexContent>\n    <xs:extension base=\"gml:AbstractFeatureType\">\n      <xs:sequence>\n");
        if !geometry.is_empty() {
            let _ = writeln!(
                out,
                "        <xs:element name=\"geometryProperty\" type=\"gml:{geometry}\" nillable=\"true\" minOccurs=\"0\" maxOccurs=\"1\"/>"
            );
        }
        for c in &t.columns {
            let inline = xsd_type(&c.kind);
            if inline.is_empty() {
                let _ = writeln!(
                    out,
                    "        <xs:element name=\"{}\" nillable=\"true\" minOccurs=\"0\" maxOccurs=\"1\" type=\"{}\"/>",
                    c.name,
                    type_word(&c.kind)
                );
            } else {
                let _ = writeln!(
                    out,
                    "        <xs:element name=\"{}\" nillable=\"true\" minOccurs=\"0\" maxOccurs=\"1\">{inline}</xs:element>",
                    c.name
                );
            }
        }
        out.push_str("      </xs:sequence>\n    </xs:extension>\n  </xs:complexContent>\n</xs:complexType>\n");
    }
    out.push_str("</xs:schema>\n");
    out
}
