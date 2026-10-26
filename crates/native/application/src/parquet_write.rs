//! `file.export.parquet`'s file (docs/adr/0628 §3, §4): the layers' objects as one GeoParquet 1.1. The geometry is ISO
//! WKB of the GeoPackage export's geometry (`geopackage::geometry_of`) with its arcs as chords of `kentos.chords/1`
//! within 0.1 mm and M left out, outer rings counter-clockwise and holes clockwise; the layers' fields typed columns,
//! other attributes text, `kentos_uid` the lasting ids; the `bbox` covering column; the `geo` key with the types the
//! rows hold, their box and the system's PROJJSON. Written by `kentos_formats::parquet::write`.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    Entity, ExportParquet, LayerFieldKind, ParquetCompression, ParquetExported, binary_bytes,
    instant_moment,
};
use kentos_domain::Document;
use kentos_formats::compress::Codec;
use kentos_formats::parquet::write::{Column, Options, write};
use kentos_formats::parquet::{Bytes, Logical, Physical, TimeUnit, Values};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::ops::derive::arc_chords;
use kentos_geopackage::{ColumnKind, Coord, Curve, Geometry, Value};
use serde_json::json;

use crate::geopackage;

/// What `file.export.parquet` makes before it writes: the file's bytes and the answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    pub bytes: Vec<u8>,
    pub output: ParquetExported,
    /// The objects the export's time left out (docs/adr/0646 §2).
    pub time_left_out: u32,
}

/// The farthest a chord of an arc comes from it (m).
const CHORD: f64 = 1e-4;

/// The circle through three points: its centre and radius; none when they are on one line.
fn circle(a: &Coord, m: &Coord, b: &Coord) -> Option<(f64, f64, f64)> {
    let d = 2.0 * (a.x * (m.y - b.y) + m.x * (b.y - a.y) + b.x * (a.y - m.y));
    if d == 0.0 || !d.is_finite() {
        return None;
    }
    let (a2, m2, b2) = (
        a.x * a.x + a.y * a.y,
        m.x * m.x + m.y * m.y,
        b.x * b.x + b.y * b.y,
    );
    let cx = (a2 * (m.y - b.y) + m2 * (b.y - a.y) + b2 * (a.y - m.y)) / d;
    let cy = (a2 * (b.x - m.x) + m2 * (a.x - b.x) + b2 * (m.x - a.x)) / d;
    Some((cx, cy, (a.x - cx).hypot(a.y - cy)))
}

/// An arc from `a` through `m` to `b` as chords, `a` and `b` as they are; Z taken along by the turn.
fn arc(a: &Coord, m: &Coord, b: &Coord, out: &mut Vec<Coord>) {
    let Some((cx, cy, r)) = circle(a, m, b) else {
        out.extend([*m, *b]);
        return;
    };
    let ang = |p: &Coord| (p.y - cy).atan2(p.x - cx);
    let (a0, am, a1) = (ang(a), ang(m), ang(b));
    let tau = std::f64::consts::TAU;
    let ccw = |from: f64, to: f64| (to - from).rem_euclid(tau);
    // Counter-clockwise when the middle comes before the end going that way.
    let sweep = if ccw(a0, am) <= ccw(a0, a1) {
        ccw(a0, a1)
    } else {
        -(tau - ccw(a0, a1))
    };
    let run = arc_chords(Vec2::new(cx, cy), r, a0, sweep, CHORD);
    let n = run.pts.len().saturating_sub(1).max(1);
    for (k, p) in run.pts.iter().enumerate().skip(1) {
        let f = k as f64 / n as f64;
        let z = match (a.z, b.z) {
            (Some(za), Some(zb)) => Some(za + (zb - za) * f),
            _ => None,
        };
        out.push(if k == n {
            Coord { m: None, ..*b }
        } else {
            Coord {
                x: p.x,
                y: p.y,
                z,
                m: None,
            }
        });
    }
}

/// A curve as straight segments; whether an arc became chords.
fn linear(c: &Curve, chorded: &mut bool) -> Vec<Coord> {
    match c {
        Curve::Line(p) => p.clone(),
        Curve::Arc(p) => {
            *chorded = true;
            let mut out = vec![p[0]];
            let mut k = 0;
            while k + 2 < p.len() {
                arc(&p[k], &p[k + 1], &p[k + 2], &mut out);
                k += 2;
            }
            out
        }
        Curve::Compound(parts) => {
            let mut out: Vec<Coord> = Vec::new();
            for part in parts {
                let pts = linear(part, chorded);
                let skip = usize::from(!out.is_empty());
                out.extend(pts.into_iter().skip(skip));
            }
            out
        }
    }
}

fn twice_area(ring: &[Coord]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (&ring[i], &ring[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum()
}

/// A polygon's rings: the outer one counter-clockwise, the holes clockwise.
fn rings(rs: &[Curve], chorded: &mut bool) -> Vec<Curve> {
    rs.iter()
        .enumerate()
        .map(|(i, r)| {
            let mut pts = linear(r, chorded);
            let ccw = twice_area(&pts) > 0.0;
            if ccw != (i == 0) {
                pts.reverse();
            }
            Curve::Line(pts)
        })
        .collect()
}

/// The geometry with straight segments only, M left out; whether arcs became chords and whether M was there.
fn plain(g: &Geometry, chorded: &mut bool, had_m: &mut bool) -> Geometry {
    let strip = |c: &Coord, had_m: &mut bool| {
        *had_m |= c.m.is_some();
        Coord { m: None, ..*c }
    };
    let mut lines = |c: &Curve, had_m: &mut bool| -> Curve {
        Curve::Line(linear(c, chorded).iter().map(|p| strip(p, had_m)).collect())
    };
    match g {
        Geometry::Point(p) => Geometry::Point(p.as_ref().map(|c| strip(c, had_m))),
        Geometry::MultiPoint(ps) => {
            Geometry::MultiPoint(ps.iter().map(|c| strip(c, had_m)).collect())
        }
        Geometry::Curve(c) => Geometry::Curve(lines(c, had_m)),
        Geometry::MultiCurve(cs) => {
            Geometry::MultiCurve(cs.iter().map(|c| lines(c, had_m)).collect())
        }
        Geometry::Polygon(rs) => Geometry::Polygon(strip_rings(rings(rs, chorded), had_m)),
        Geometry::MultiPolygon(ps) => Geometry::MultiPolygon(
            ps.iter()
                .map(|rs| strip_rings(rings(rs, chorded), had_m))
                .collect(),
        ),
        Geometry::Collection(gs) => {
            Geometry::Collection(gs.iter().map(|g| plain(g, chorded, had_m)).collect())
        }
    }
}

fn strip_rings(rs: Vec<Curve>, had_m: &mut bool) -> Vec<Curve> {
    rs.into_iter()
        .map(|c| match c {
            Curve::Line(p) => Curve::Line(
                p.into_iter()
                    .map(|c| {
                        *had_m |= c.m.is_some();
                        Coord { m: None, ..c }
                    })
                    .collect(),
            ),
            other => other,
        })
        .collect()
}

fn coords<'a>(g: &'a Geometry, out: &mut Vec<&'a Coord>) {
    let curve = |c: &'a Curve, out: &mut Vec<&'a Coord>| {
        if let Curve::Line(p) = c {
            out.extend(p.iter());
        }
    };
    match g {
        Geometry::Point(p) => out.extend(p.iter()),
        Geometry::MultiPoint(ps) => out.extend(ps.iter()),
        Geometry::Curve(c) => curve(c, out),
        Geometry::MultiCurve(cs) | Geometry::Polygon(cs) => cs.iter().for_each(|c| curve(c, out)),
        Geometry::MultiPolygon(ps) => ps.iter().flatten().for_each(|c| curve(c, out)),
        Geometry::Collection(gs) => gs.iter().for_each(|g| coords(g, out)),
    }
}

/// GeoParquet's name of a geometry's type, ` Z` when it has heights.
fn type_name(g: &Geometry) -> String {
    let base = match g {
        Geometry::Point(_) => "Point",
        Geometry::MultiPoint(_) => "MultiPoint",
        Geometry::Curve(_) => "LineString",
        Geometry::MultiCurve(_) => "MultiLineString",
        Geometry::Polygon(_) => "Polygon",
        Geometry::MultiPolygon(_) => "MultiPolygon",
        Geometry::Collection(_) => "GeometryCollection",
    };
    let mut c = Vec::new();
    coords(g, &mut c);
    if c.iter().any(|p| p.z.is_some()) {
        format!("{base} Z")
    } else {
        base.to_owned()
    }
}

/// Days from 1970-01-01 of a `YYYY-MM-DD` (Howard Hinnant's `days_from_civil`).
fn days(text: &str) -> Option<i64> {
    let mut it = text.get(..10)?.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let d: i64 = it.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// Microseconds of a `SS:DD:SS[.ffffff]`.
fn micros_of_day(text: &str) -> Option<i64> {
    let (hms, frac) = text.split_once('.').unwrap_or((text, ""));
    let mut it = hms.split(':');
    let h: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let s: i64 = it.next().unwrap_or("0").parse().ok()?;
    if h > 23 || m > 59 || s > 60 {
        return None;
    }
    let digits: String = frac
        .chars()
        .take_while(char::is_ascii_digit)
        .take(6)
        .collect();
    let f: i64 = if digits.is_empty() {
        0
    } else {
        format!("{digits:0<6}").parse().ok()?
    };
    Some(((h * 60 + m) * 60 + s) * 1_000_000 + f)
}

/// A column of the file being made: its name, type and each row's value.
struct Building {
    name: String,
    physical: Physical,
    logical: Option<Logical>,
    kind: LayerFieldKind,
    present: Vec<bool>,
    values: Values,
    bad: u32,
    /// Time stamps written without their digits finer than a microsecond (docs/adr/0629 §4).
    fine: u32,
}

impl Building {
    fn new(name: String, kind: LayerFieldKind) -> Building {
        let (physical, logical) = match kind {
            LayerFieldKind::Text => (Physical::ByteArray, Some(Logical::String)),
            LayerFieldKind::Integer => (Physical::Int64, None),
            LayerFieldKind::Decimal => (Physical::Double, None),
            LayerFieldKind::Boolean => (Physical::Boolean, None),
            LayerFieldKind::Date => (Physical::Int32, Some(Logical::Date)),
            LayerFieldKind::DateTime => (
                Physical::Int64,
                Some(Logical::Timestamp {
                    utc: false,
                    unit: TimeUnit::Micros,
                }),
            ),
            LayerFieldKind::Time => (
                Physical::Int64,
                Some(Logical::Time {
                    utc: false,
                    unit: TimeUnit::Micros,
                }),
            ),
            // docs/adr/0629 §4: a 64-bit integer, a UTC time stamp, a UUID's 16 bytes, JSON and bytes.
            LayerFieldKind::BigInt => (Physical::Int64, None),
            LayerFieldKind::Instant => (
                Physical::Int64,
                Some(Logical::Timestamp {
                    utc: true,
                    unit: TimeUnit::Micros,
                }),
            ),
            LayerFieldKind::Uuid => (Physical::FixedLenByteArray, Some(Logical::Uuid)),
            LayerFieldKind::Json => (Physical::ByteArray, Some(Logical::Json)),
            LayerFieldKind::Binary => (Physical::ByteArray, None),
        };
        Building {
            name,
            physical,
            logical,
            kind,
            present: Vec::new(),
            values: kentos_formats::parquet::Values::empty(physical),
            bad: 0,
            fine: 0,
        }
    }

    fn push(&mut self, text: Option<&String>) {
        let t = text.map(|t| t.trim()).filter(|t| !t.is_empty());
        let ok = match (t, &mut self.values) {
            (None, _) => false,
            (Some(t), Values::Bytes(b)) => match self.kind {
                LayerFieldKind::Uuid => {
                    let hex: String = t.chars().filter(|c| *c != '-').collect();
                    let bytes: Option<Vec<u8>> = (hex.len() == 32)
                        .then(|| {
                            (0..16)
                                .map(|i| u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok())
                                .collect()
                        })
                        .flatten();
                    bytes.map(|x| b.push(&x)).is_some()
                }
                LayerFieldKind::Binary => binary_bytes(t).map(|x| b.push(&x)).is_some(),
                _ => {
                    b.push(t.as_bytes());
                    true
                }
            },
            (Some(t), Values::I64(v)) => {
                let x = match self.kind {
                    LayerFieldKind::Instant => instant_moment(t).and_then(|(sec, nanos)| {
                        if nanos % 1000 != 0 {
                            self.fine += 1;
                        }
                        sec.checked_mul(1_000_000)?
                            .checked_add(i64::from(nanos / 1000))
                    }),
                    LayerFieldKind::DateTime => days(t).and_then(|d| {
                        let rest = t.get(11..).unwrap_or("00:00:00");
                        micros_of_day(rest).map(|m| d * 86_400_000_000 + m)
                    }),
                    LayerFieldKind::Time => micros_of_day(t),
                    _ => t.parse::<i64>().ok(),
                };
                x.map(|x| v.push(x)).is_some()
            }
            (Some(t), Values::I32(v)) => days(t)
                .and_then(|d| i32::try_from(d).ok())
                .map(|d| v.push(d))
                .is_some(),
            (Some(t), Values::F64(v)) => t
                .parse::<f64>()
                .ok()
                .filter(|x| x.is_finite())
                .map(|x| v.push(x))
                .is_some(),
            (Some(t), Values::Bool(v)) => {
                match geopackage::cell(Some(&t.to_owned()), &ColumnKind::Boolean) {
                    Value::Integer(i) => {
                        v.push(i != 0);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        };
        if t.is_some() && !ok {
            self.bad += 1;
        }
        self.present.push(ok);
    }

    fn column(self) -> Column {
        Column {
            name: self.name,
            group: None,
            physical: self.physical,
            logical: self.logical,
            optional: true,
            present: self.present,
            values: self.values,
        }
    }
}

fn type_word(c: &Column) -> String {
    let logical = match &c.logical {
        Some(Logical::String) => " STRING",
        Some(Logical::Date) => " DATE",
        Some(Logical::Timestamp { utc: true, .. }) => " TIMESTAMP(MICROS, UTC)",
        Some(Logical::Timestamp { .. }) => " TIMESTAMP(MICROS)",
        Some(Logical::Uuid) => " UUID",
        Some(Logical::Json) => " JSON",
        Some(Logical::Time { .. }) => " TIME(MICROS)",
        _ => "",
    };
    format!("{}{logical}", c.physical.name())
}

/// The export of `input`, of the objects its time shows (docs/adr/0646 §2).
pub fn export(doc: &Document, input: &ExportParquet) -> Result<Made, (String, String, String)> {
    let (doc, left) = crate::time_export::filtered(doc, &input.layers, input.time.as_ref())?;
    let mut made = export_all(&doc, input)?;
    made.time_left_out = left;
    Ok(made)
}

/// The layers' objects as a GeoParquet, or why none can be written: (code, message, path).
fn export_all(doc: &Document, input: &ExportParquet) -> Result<Made, (String, String, String)> {
    let ids = crate::shapefile::layer_ids(doc, &input.layers).map_err(|(i, id)| {
        (
            kentos_commands::codes::LAYER_NOT_FOUND.to_owned(),
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            format!("layers[{i}]"),
        )
    })?;
    let mut objects: Vec<(&str, &Entity, kentos_domain::Uuid)> = Vec::new();
    for id in &ids {
        for (uid, e) in doc.by_layer_with_uids(id) {
            objects.push((id.as_str(), e, uid));
        }
    }
    let used: Vec<&String> = ids
        .iter()
        .filter(|id| objects.iter().any(|(l, _, _)| *l == id.as_str()))
        .collect();
    // The columns: the layers' fields in order, then the attributes no field names, by name.
    let mut fields: Vec<(String, LayerFieldKind)> = Vec::new();
    let mut lost: BTreeSet<String> = BTreeSet::new();
    for id in &used {
        if let Some(node) = doc.layers().get(id) {
            for f in &node.fields {
                match fields
                    .iter_mut()
                    .find(|(n, _)| n.eq_ignore_ascii_case(&f.name))
                {
                    Some(have) if have.1 != f.kind => {
                        lost.insert(format!(
                            "“{}” alanı katmanlarda farklı türlerde: metin sütunu olarak yazıldı",
                            f.name
                        ));
                        have.1 = LayerFieldKind::Text;
                    }
                    Some(_) => {}
                    None => fields.push((f.name.clone(), f.kind)),
                }
            }
        }
    }
    let mut others: BTreeSet<String> = BTreeSet::new();
    for (_, e, _) in &objects {
        for k in e.base().attrs.keys() {
            if !fields.iter().any(|(n, _)| n.eq_ignore_ascii_case(k)) {
                others.insert(k.clone());
            }
        }
    }
    fields.extend(others.into_iter().map(|k| (k, LayerFieldKind::Text)));
    let reserved = ["geometry", "bbox", "kentos_uid", "katman"];
    let layered = used.len() > 1;
    let mut built: Vec<Building> = Vec::new();
    for (name, kind) in &fields {
        let mut written = name.clone();
        while reserved.iter().any(|r| r.eq_ignore_ascii_case(&written))
            || built.iter().any(|b| b.name.eq_ignore_ascii_case(&written))
        {
            lost.insert(format!(
                "“{written}” alanı “{written}_” sütunu olarak yazıldı (adı başka bir sütunun)"
            ));
            written.push('_');
        }
        built.push(Building::new(written, *kind));
    }
    let mut geometry = Bytes::default();
    let mut geometry_present = Vec::new();
    let mut boxes: [Vec<f64>; 4] = Default::default();
    let mut uids = Bytes::default();
    let mut layers = Bytes::default();
    let mut types: BTreeSet<String> = BTreeSet::new();
    let mut extent: Option<[f64; 4]> = None;
    let mut left: BTreeMap<String, u32> = BTreeMap::new();
    let (mut arcs, mut curves, mut measures) = (0u32, 0u32, false);
    let mut rows = 0usize;
    for (layer, e, uid) in &objects {
        let g = match geopackage::geometry_of(e) {
            Ok(g) => g,
            Err(word) => {
                *left.entry(word.to_owned()).or_default() += 1;
                continue;
            }
        };
        rows += 1;
        match g {
            Some((g, chords)) => {
                curves += u32::from(chords);
                let mut chorded = false;
                let g = plain(&g, &mut chorded, &mut measures);
                arcs += u32::from(chorded);
                let mut c = Vec::new();
                coords(&g, &mut c);
                if c.is_empty() {
                    geometry_present.push(false);
                } else {
                    let mut b = [
                        f64::INFINITY,
                        f64::INFINITY,
                        f64::NEG_INFINITY,
                        f64::NEG_INFINITY,
                    ];
                    for p in &c {
                        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
                    }
                    for (k, v) in b.iter().enumerate() {
                        boxes[k].push(*v);
                    }
                    extent = Some(extent.map_or(b, |x| {
                        [
                            x[0].min(b[0]),
                            x[1].min(b[1]),
                            x[2].max(b[2]),
                            x[3].max(b[3]),
                        ]
                    }));
                    types.insert(type_name(&g));
                    geometry.push(&kentos_geopackage::write_wkb(&g));
                    geometry_present.push(true);
                }
            }
            None => geometry_present.push(false),
        }
        uids.push(uid.to_string().as_bytes());
        if layered {
            let name = doc
                .layers()
                .get(layer)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            layers.push(name.as_bytes());
        }
        let attrs = &e.base().attrs;
        for (b, (name, _)) in built.iter_mut().zip(&fields) {
            let v = attrs.get(name).or_else(|| {
                attrs
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(name))
                    .map(|(_, v)| v)
            });
            b.push(v);
        }
    }
    if rows == 0 {
        return Err((
            kentos_commands::codes::NOTHING_TO_EXPORT.to_owned(),
            "Katmanlarda GeoParquet'e yazılacak nesne yok.".into(),
            "layers".into(),
        ));
    }
    if arcs > 0 {
        lost.insert(format!(
            "{arcs} nesnenin yayları GeoParquet'te karşılığı olmadığından 0,1 mm kirişlerle yazıldı"
        ));
    }
    if curves > 0 {
        lost.insert(format!(
            "{curves} eğri (elips, eğri, NURBS) kirişlerle yazıldı"
        ));
    }
    if measures {
        lost.insert("köşelerin M ölçüleri yazılmadı: GeoParquet 1.1 yalnız Z taşır".into());
    }
    for b in &built {
        if b.bad > 0 {
            lost.insert(format!(
                "“{}” alanının türüne uymayan {} değeri boş yazıldı",
                b.name, b.bad
            ));
        }
        if b.fine > 0 {
            lost.insert(format!(
                "“{}” alanının {} zaman damgasının mikrosaniyeden ince kesri yazılmadı",
                b.name, b.fine
            ));
        }
    }
    // The kinds Parquet's types do not say (docs/adr/0629 §4): `{"kinds": {"column": "kind"}}`.
    let kinds: serde_json::Map<String, serde_json::Value> = built
        .iter()
        .filter(|b| b.kind.is_wide())
        .map(|b| (b.name.clone(), json!(b.kind.name())))
        .collect();
    if !left.is_empty() {
        lost.insert(format!(
            "GeoParquet'te karşılığı olmayan nesneler yazılmadı: {}",
            left.iter()
                .map(|(k, n)| format!("{n} {k}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let srid = doc.settings().srid;
    let system = (srid != 0)
        .then(|| kentos_project::crs::system(srid))
        .flatten();
    let crs = system.and_then(kentos_project::projjson::projjson);
    if srid == 0 {
        lost.insert(
            "projenin koordinat sistemi yok: dosyanın sistemi null (bilinmiyor) yazıldı".into(),
        );
    } else if crs.is_none() {
        lost.insert(format!(
            "EPSG:{srid} sisteminin tanımı yazılamadı: dosyanın sistemi null yazıldı"
        ));
    }
    // The columns in order: the geometry, the layer, the fields, the id, the covering box.
    let mut columns = vec![Column {
        name: "geometry".into(),
        group: None,
        physical: Physical::ByteArray,
        logical: None,
        optional: true,
        present: geometry_present.clone(),
        values: Values::Bytes(geometry),
    }];
    if layered {
        columns.push(Column {
            name: "katman".into(),
            group: None,
            physical: Physical::ByteArray,
            logical: Some(Logical::String),
            optional: false,
            present: Vec::new(),
            values: Values::Bytes(layers),
        });
    }
    columns.extend(built.into_iter().map(Building::column));
    columns.push(Column {
        name: "kentos_uid".into(),
        group: None,
        physical: Physical::ByteArray,
        logical: Some(Logical::String),
        optional: false,
        present: Vec::new(),
        values: Values::Bytes(uids),
    });
    if input.covering {
        for (k, name) in ["xmin", "ymin", "xmax", "ymax"].iter().enumerate() {
            columns.push(Column {
                name: (*name).into(),
                group: Some("bbox".into()),
                physical: Physical::Double,
                logical: None,
                optional: true,
                present: geometry_present.clone(),
                values: Values::F64(std::mem::take(&mut boxes[k])),
            });
        }
    }
    let mut geo_column = json!({
        "encoding": "WKB",
        "geometry_types": types.iter().collect::<Vec<_>>(),
        "crs": crs.clone().unwrap_or(serde_json::Value::Null),
        "orientation": "counterclockwise",
        "edges": "planar",
    });
    if let Some(b) = extent {
        geo_column["bbox"] = json!(b);
    }
    if input.covering {
        geo_column["covering"] = json!({ "bbox": {
            "xmin": ["bbox", "xmin"], "ymin": ["bbox", "ymin"], "xmax": ["bbox", "xmax"], "ymax": ["bbox", "ymax"]
        }});
    }
    let geo = json!({ "version": "1.1.0", "primary_column": "geometry", "columns": { "geometry": geo_column } });
    let o = Options {
        codec: match input.compression {
            ParquetCompression::Snappy => Codec::Snappy,
            ParquetCompression::None => Codec::None,
        },
        key_value: std::iter::once(("geo".into(), geo.to_string()))
            .chain((!kinds.is_empty()).then(|| ("kentos".into(), json!({ "kinds": kinds }).to_string())))
            .collect(),
        created_by: "KentOS CAD".into(),
        ..Options::default()
    };
    let bytes = write(&columns, rows, &o).map_err(|why| {
        (
            kentos_commands::codes::FILE_UNWRITABLE.to_owned(),
            format!("GeoParquet yazılamadı: {why}"),
            "path".into(),
        )
    })?;
    let output = ParquetExported {
        path: input.path.clone(),
        rows: rows as u32,
        row_groups: rows.div_ceil(o.row_group) as u32,
        columns: columns
            .iter()
            .map(|c| {
                c.group
                    .as_ref()
                    .map_or_else(|| c.name.clone(), |g| format!("{g}.{}", c.name))
            })
            .collect(),
        types: columns.iter().map(type_word).collect(),
        geometry_types: types.into_iter().collect(),
        srid: (srid != 0).then_some(srid),
        bbox: extent,
        compression: input.compression,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        left,
        lost: lost.into_iter().collect(),
    };
    Ok(Made { bytes, output, time_left_out: 0 })
}
