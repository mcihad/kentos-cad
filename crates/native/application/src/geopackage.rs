//! A GeoPackage's tables as a drawing's objects and the drawing's layers as GeoPackage tables (docs/adr/0502,
//! plan/gis.md CBS-03, CBS-05, CBS-06): the conversions `file.import.gpkg` and `file.export.gpkg` use, over the
//! `kentos-geopackage` crate. Nothing here writes the document.
//!
//! Geometry: a point and a multi point are a point object (its parts); a line string, a circular string and a compound
//! curve a polyline, each arc of three points its edge's bulge, tan(θ/4), exact; a multi line string and a multi curve
//! a polyline of parts; a polygon and a curve polygon an area with its holes (the closing vertex dropped); a multi
//! polygon and a multi surface an area of parts. Z goes to the vertices' elevations; M to a polyline's measures (an
//! area keeps none: said). A row without geometry, or with an empty one, is a record (docs/adr/0410). A collection is
//! left out, said.
//!
//! Values: NULL is no attribute, an empty text the empty text. Integer columns are integer fields unless a value is
//! past ±(2⁵³ − 1): then text, every digit kept, said. A DATETIME's UTC time of day goes into the local date-and-time
//! field without its zone and its fraction (said), and back out as UTC. A blob column is left out, said.
//!
//! Lasting ids: a row's id is the file's `kentos_uids` (written by KentOS), else UUIDv5 of the table and fid in a
//! namespace of this module: the same row of the same table gets the same id every time.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kentos_contracts::identity::uuid_v5;
use kentos_contracts::{Entity, LayerField, LayerFieldKind, MAX_INTEGER};
use kentos_domain::{Document, Uuid};
use kentos_geopackage as gpkg;
use kentos_geopackage::{ColumnKind, Coord, Curve, DataType, GeoPackage, Geometry, Query, Value};
use serde_json::{Value as J, json};

/// The namespace of the ids of rows KentOS did not write (a random UUID, fixed).
const NAMESPACE: [u8; 16] = [
    0x6b, 0x1e, 0x4f, 0x0a, 0x93, 0x27, 0x4c, 0x51, 0x8d, 0x36, 0x2f, 0x7a, 0x10, 0xc4, 0x5e, 0x92,
];

/// A database row's lasting id when the database keeps none (docs/adr/0505): of the database's name, the table
/// (`schema.table`) and the primary key.
pub fn database_row_uid(database: &str, table: &str, key: i64) -> Uuid {
    Uuid::from_bytes(uuid_v5(
        &NAMESPACE,
        format!("pg:{database}/{table}:{key}").as_bytes(),
    ))
}

/// A row's lasting id when the file keeps none.
pub fn row_uid(table: &str, fid: i64) -> Uuid {
    Uuid::from_bytes(uuid_v5(
        &NAMESPACE,
        format!("gpkg:{table}:{fid}").as_bytes(),
    ))
}

/// A table read.
#[derive(Clone, Debug)]
pub struct TableRead {
    pub name: String,
    /// `gpkg_contents`'s identifier, when it differs from the name.
    pub title: Option<String>,
    pub srs_id: Option<i32>,
    pub fields: Vec<LayerField>,
    /// The objects with their lasting ids; their layer is the table's name.
    pub objects: Vec<(Uuid, Entity)>,
    /// Each object's row, by fid, in the objects' order.
    pub fids: Vec<i64>,
    /// How many rows were read, and of them records.
    pub read: u32,
    pub records: u32,
    /// Rows left out: fid and why.
    pub skipped: Vec<(i64, String)>,
    /// What was not carried, once each.
    pub losses: BTreeSet<String>,
}

/// A file read.
#[derive(Clone, Debug)]
pub struct Read {
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
    pub tables: Vec<TableRead>,
    /// Each srs id the tables use, with its organization and id (`EPSG`, 5256), none for an undefined one.
    pub systems: BTreeMap<i32, Option<(String, i32)>>,
}

fn coord_json(c: &Coord) -> J {
    json!({ "x": c.x, "y": c.y })
}

/// The signed sweep of the arc from `a` through `m` to `b`, radians (counter-clockwise positive); none when they
/// are on one line or two are one point.
fn sweep(a: &Coord, m: &Coord, b: &Coord) -> Option<f64> {
    let (ax, ay, mx, my, bx, by) = (a.x, a.y, m.x, m.y, b.x, b.y);
    let d = 2.0 * (ax * (my - by) + mx * (by - ay) + bx * (ay - my));
    if d == 0.0 || !d.is_finite() {
        return None;
    }
    let a2 = ax * ax + ay * ay;
    let m2 = mx * mx + my * my;
    let b2 = bx * bx + by * by;
    let cx = (a2 * (my - by) + m2 * (by - ay) + b2 * (ay - my)) / d;
    let cy = (a2 * (bx - mx) + m2 * (ax - bx) + b2 * (mx - ax)) / d;
    let ang = |x: f64, y: f64| (y - cy).atan2(x - cx);
    let (t0, t1, t2) = (ang(ax, ay), ang(mx, my), ang(bx, by));
    let tau = std::f64::consts::TAU;
    let ccw = (mx - ax) * (by - ay) - (my - ay) * (bx - ax) > 0.0;
    let norm = |x: f64| x.rem_euclid(tau);
    Some(if ccw {
        // Counter-clockwise from a, through m, to b.
        let s = norm(t2 - t0);
        debug_assert!(norm(t1 - t0) <= s + 1e-12);
        s
    } else {
        -norm(t0 - t2)
    })
}

/// A curve as vertices, each with its edge's bulge, elevation and measure (the last vertex's bulge 0).
#[derive(Default)]
struct Walk {
    pts: Vec<Coord>,
    bulges: Vec<f64>,
}

impl Walk {
    fn push(&mut self, c: Coord, bulge_before: Option<f64>) {
        if let (Some(b), Some(last)) = (bulge_before, self.bulges.last_mut()) {
            *last = b;
        }
        if self.pts.last().is_some_and(|p| p.x == c.x && p.y == c.y) {
            return;
        }
        self.pts.push(c);
        self.bulges.push(0.0);
    }

    fn add(&mut self, c: &Curve) {
        match c {
            Curve::Line(p) => {
                for q in p {
                    self.push(*q, Some(0.0).filter(|_| !self.pts.is_empty()));
                }
            }
            Curve::Arc(p) => {
                if let Some(first) = p.first() {
                    self.push(*first, None);
                }
                let mut i = 0;
                while i + 2 < p.len() {
                    let b = sweep(&p[i], &p[i + 1], &p[i + 2]).map_or(0.0, |s| (s / 4.0).tan());
                    self.push(p[i + 2], Some(b));
                    i += 2;
                }
            }
            Curve::Compound(parts) => parts.iter().for_each(|c| self.add(c)),
        }
    }

    /// A closed ring: the closing vertex dropped, its bulge the last edge's.
    fn close(mut self) -> Walk {
        if self.pts.len() > 1
            && let (Some(f), Some(l)) = (self.pts.first(), self.pts.last())
            && f.x == l.x
            && f.y == l.y
        {
            // The edge into the dropped vertex is the closing edge: its bulge stays on the vertex before it.
            self.pts.pop();
            self.bulges.pop();
        }
        self
    }

    fn of(c: &Curve) -> Walk {
        let mut p = Walk::default();
        p.add(c);
        p
    }
}

fn path_fields(p: &Walk, measures: bool, out: &mut serde_json::Map<String, J>) {
    out.insert(
        "pts".into(),
        J::Array(p.pts.iter().map(coord_json).collect()),
    );
    if p.bulges.iter().any(|b| *b != 0.0) {
        out.insert("bulges".into(), json!(p.bulges));
    }
    if p.pts.iter().any(|c| c.z.is_some_and(f64::is_finite)) {
        out.insert(
            "zs".into(),
            J::Array(
                p.pts
                    .iter()
                    .map(|c| c.z.filter(|z| z.is_finite()).map_or(J::Null, |z| json!(z)))
                    .collect(),
            ),
        );
    }
    if measures && p.pts.iter().any(|c| c.m.is_some_and(f64::is_finite)) {
        out.insert(
            "ms".into(),
            J::Array(
                p.pts
                    .iter()
                    .map(|c| c.m.filter(|m| m.is_finite()).map_or(J::Null, |m| json!(m)))
                    .collect(),
            ),
        );
    }
}

fn ring_json(p: &Walk) -> J {
    let mut m = serde_json::Map::new();
    path_fields(p, false, &mut m);
    J::Object(m)
}

fn has_m(c: &Curve) -> bool {
    let p = Walk::of(c);
    p.pts.iter().any(|c| c.m.is_some_and(f64::is_finite))
}

/// One surface's outer ring and holes as JSON fields.
fn surface_fields(rings: &[Curve], out: &mut serde_json::Map<String, J>) {
    let mut it = rings.iter().map(|r| Walk::of(r).close());
    if let Some(outer) = it.next() {
        path_fields(&outer, false, out);
    }
    let holes: Vec<J> = it.map(|h| ring_json(&h)).collect();
    if !holes.is_empty() {
        out.insert("holes".into(), J::Array(holes));
    }
}

/// The geometry's fields of an object's JSON and its kind; none for a geometry the drawing has no object for.
fn geometry_json(
    g: &Geometry,
    losses: &mut BTreeSet<String>,
) -> Option<(&'static str, serde_json::Map<String, J>)> {
    let mut m = serde_json::Map::new();
    match g {
        Geometry::Point(Some(c)) => {
            m.insert("p".into(), coord_json(c));
            if let Some(z) = c.z.filter(|z| z.is_finite()) {
                m.insert("z".into(), json!(z));
            }
            if c.m.is_some() {
                losses.insert("noktanın ölçüsü (M) alınmadı".into());
            }
            Some(("point", m))
        }
        Geometry::MultiPoint(ps) if !ps.is_empty() => {
            let first = ps[0];
            m.insert("p".into(), coord_json(&first));
            if let Some(z) = first.z.filter(|z| z.is_finite()) {
                m.insert("z".into(), json!(z));
            }
            if ps.len() > 1 {
                let parts: Vec<J> = ps[1..]
                    .iter()
                    .map(|c| {
                        let mut q = serde_json::Map::new();
                        q.insert("p".into(), coord_json(c));
                        if let Some(z) = c.z.filter(|z| z.is_finite()) {
                            q.insert("z".into(), json!(z));
                        }
                        J::Object(q)
                    })
                    .collect();
                m.insert("parts".into(), J::Array(parts));
            }
            Some(("point", m))
        }
        Geometry::Curve(c) => {
            let p = Walk::of(c);
            if p.pts.len() < 2 {
                return None;
            }
            path_fields(&p, true, &mut m);
            Some(("polyline", m))
        }
        Geometry::MultiCurve(cs) if !cs.is_empty() => {
            let mut paths = cs.iter().map(Walk::of).filter(|p| p.pts.len() >= 2);
            let first = paths.next()?;
            path_fields(&first, true, &mut m);
            let parts: Vec<J> = paths
                .map(|p| {
                    let mut q = serde_json::Map::new();
                    path_fields(&p, true, &mut q);
                    J::Object(q)
                })
                .collect();
            if !parts.is_empty() {
                m.insert("parts".into(), J::Array(parts));
            }
            Some(("polyline", m))
        }
        Geometry::Polygon(rings) if !rings.is_empty() => {
            if rings.iter().any(has_m) {
                losses.insert("alanın köşelerinin ölçüsü (M) alınmadı: alan rota değildir".into());
            }
            surface_fields(rings, &mut m);
            Some(("polygon", m))
        }
        Geometry::MultiPolygon(polys) if !polys.is_empty() => {
            if polys.iter().flatten().any(has_m) {
                losses.insert("alanın köşelerinin ölçüsü (M) alınmadı: alan rota değildir".into());
            }
            let mut it = polys.iter().filter(|r| !r.is_empty());
            let first = it.next()?;
            surface_fields(first, &mut m);
            let parts: Vec<J> = it
                .map(|rings| {
                    let mut q = serde_json::Map::new();
                    surface_fields(rings, &mut q);
                    J::Object(q)
                })
                .collect();
            if !parts.is_empty() {
                m.insert("parts".into(), J::Array(parts));
            }
            Some(("polygon", m))
        }
        _ => None,
    }
}

/// A field of a column, and whether its values go as text.
fn field_of(c: &gpkg::Column, wide: bool) -> Option<LayerField> {
    let (kind, length) = match &c.kind {
        ColumnKind::Integer if wide => (LayerFieldKind::Text, None),
        ColumnKind::Integer => (LayerFieldKind::Integer, None),
        ColumnKind::Real => (LayerFieldKind::Decimal, None),
        ColumnKind::Boolean => (LayerFieldKind::Boolean, None),
        ColumnKind::Date => (LayerFieldKind::Date, None),
        ColumnKind::DateTime => (LayerFieldKind::DateTime, None),
        ColumnKind::Text(n) => (LayerFieldKind::Text, *n),
        ColumnKind::Blob(_) => return None,
        ColumnKind::Other(_) => (LayerFieldKind::Text, None),
    };
    let mut f = json!({ "name": c.name, "kind": kind });
    if let Some(n) = length {
        f["length"] = json!(n);
    }
    serde_json::from_value(f).ok()
}

/// Shortest text of a double that reads back to it, without an exponent.
fn decimal_text(r: f64) -> String {
    let s = format!("{r}");
    if s.contains('e') || s.contains('E') {
        format!("{r:.17}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    } else {
        s
    }
}

/// A cell as the attribute's text; none for NULL.
fn value_text(
    v: &Value,
    kind: &ColumnKind,
    wide: bool,
    losses: &mut BTreeSet<String>,
    col: &str,
) -> Option<String> {
    match (v, kind) {
        (Value::Null, _) => None,
        (Value::Integer(i), ColumnKind::Boolean) => {
            Some(if *i != 0 { "true" } else { "false" }.into())
        }
        (Value::Integer(i), _) if wide => Some(i.to_string()),
        (Value::Integer(i), ColumnKind::Real) => Some(decimal_text(*i as f64)),
        (Value::Integer(i), _) => Some(i.to_string()),
        (Value::Real(r), _) if r.is_finite() => Some(decimal_text(*r)),
        (Value::Real(_), _) => {
            losses.insert(format!("“{col}” sütununun sonlu olmayan sayıları alınmadı"));
            None
        }
        (Value::Text(t), ColumnKind::DateTime) => {
            // `YYYY-MM-DDTHH:MM:SS[.fff][Z|±hh:mm]`: the local field keeps the date and time of day.
            let base: String = t.chars().take(19).collect();
            if t.len() > 19 {
                let rest = &t[19..];
                if rest != "Z" && !rest.is_empty() {
                    losses.insert(format!(
                        "“{col}” sütununun saat dilimi ya da saniyenin kesri alınmadı: tarih ve saat alanı yerel saattir"
                    ));
                }
            }
            Some(base.replace(' ', "T"))
        }
        (Value::Text(t), _) => Some(t.clone()),
        (Value::Blob(_), _) => {
            losses.insert(format!("“{col}” sütununun ikili değerleri alınmadı"));
            None
        }
    }
}

/// Reads the tables `wanted` names (every feature and attribute table when none).
pub fn read(path: &Path, wanted: Option<&[String]>) -> Result<Read, String> {
    // The file's digest read a piece at a time: a large file is not held whole beside its rows (plan/gis.md CBS-42).
    let (size, sha256) = std::fs::File::open(path)
        .and_then(|f| kentos_contracts::encoding::sha256_reader_hex(std::io::BufReader::new(f)))
        .map_err(|e| format!("“{}” okunamadı: {e}", path.display()))?;
    let gp = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let layers = gp.layers().map_err(|e| e.0)?;
    if let Some(w) = wanted
        && let Some(missing) = w.iter().find(|t| !layers.iter().any(|l| &l.name == *t))
    {
        return Err(format!("“{missing}” tablosu dosyada yok"));
    }
    let mut tables = Vec::new();
    let mut systems = BTreeMap::new();
    for layer in layers {
        let chosen = match wanted {
            Some(w) => w.contains(&layer.name),
            None => matches!(layer.data_type, DataType::Features | DataType::Attributes),
        };
        if !chosen {
            continue;
        }
        if let Some(id) = layer.srs_id
            && !systems.contains_key(&id)
        {
            let org = gp
                .srs(id)
                .map_err(|e| e.0)?
                .filter(|s| s.definition != "undefined" && id > 0)
                .map(|s| (s.organization.to_ascii_uppercase(), s.organization_id));
            systems.insert(id, org);
        }
        let uids = gp.uids(&layer.name).map_err(|e| e.0)?;
        let rows = gp.rows(&layer, &Query::default()).map_err(|e| e.0)?;
        let name = layer.name.clone();
        tables.push(table_of_rows(&layer, &rows, &uids, &|fid| {
            row_uid(&name, fid)
        })?);
    }
    Ok(Read {
        file: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        bytes: size,
        sha256,
        tables,
        systems,
    })
}

/// A table's rows as the drawing's objects (docs/adr/0502 §3; a PostGIS table's too, docs/adr/0505): each row an
/// object or a record with its lasting id (`uids`, else `uid_of` its fid), the columns as fields, what does not come
/// over said.
pub fn table_of_rows(
    layer: &gpkg::Layer,
    rows: &[gpkg::Row],
    uids: &std::collections::HashMap<i64, String>,
    uid_of: &dyn Fn(i64) -> Uuid,
) -> Result<TableRead, String> {
    let mut losses = BTreeSet::new();
    // An integer column holding a value past the safe range goes as text.
    let wide: Vec<bool> = (0..layer.columns.len())
        .map(|i| {
            layer.columns[i].kind == ColumnKind::Integer
                && rows.iter().any(
                    |r| matches!(r.values[i], Value::Integer(v) if v.unsigned_abs() > MAX_INTEGER),
                )
        })
        .collect();
    let mut fields = Vec::new();
    for (c, w) in layer.columns.iter().zip(&wide) {
        match field_of(c, *w) {
            Some(f) => fields.push(f),
            None => {
                losses.insert(format!("“{}” ikili sütunu alınmadı", c.name));
            }
        }
        if *w {
            losses.insert(format!(
                "“{}” sütunu 2⁵³'ten büyük sayılar tutuyor: metin alanı olarak, her basamağıyla alındı",
                c.name
            ));
        }
    }
    let mut objects = Vec::with_capacity(rows.len());
    let mut fids = Vec::with_capacity(rows.len());
    let mut skipped = Vec::new();
    let mut records = 0;
    for row in rows {
        let mut attrs = serde_json::Map::new();
        for ((c, v), w) in layer.columns.iter().zip(&row.values).zip(&wide) {
            if let Some(t) = value_text(v, &c.kind, *w, &mut losses, &c.name) {
                attrs.insert(c.name.clone(), J::String(t));
            }
        }
        let mut e = serde_json::Map::new();
        e.insert("id".into(), json!(0));
        e.insert("layerId".into(), J::String(layer.name.clone()));
        e.insert("attrs".into(), J::Object(attrs));
        let kind = match &row.geometry {
            None => "record",
            Some(Err(why)) => {
                skipped.push((row.fid, format!("geometrisi okunamadı: {why}")));
                continue;
            }
            Some(Ok(g)) if g.is_empty() => "record",
            Some(Ok(g)) => match geometry_json(g, &mut losses) {
                Some((kind, fields)) => {
                    e.extend(fields);
                    kind
                }
                None => {
                    skipped.push((
                        row.fid,
                        format!("{} geometrisinin çizimde karşılığı yok", g.type_name()),
                    ));
                    continue;
                }
            },
        };
        e.insert("kind".into(), J::String(kind.into()));
        if kind == "record" {
            records += 1;
        }
        let entity: Entity = serde_json::from_value(J::Object(e)).map_err(|why| {
            format!(
                "“{}” tablosunun {} numaralı satırı: {why}",
                layer.name, row.fid
            )
        })?;
        let uid = uids
            .get(&row.fid)
            .and_then(|u| Uuid::parse_str(u).ok())
            .unwrap_or_else(|| uid_of(row.fid));
        objects.push((uid, entity));
        fids.push(row.fid);
    }
    Ok(TableRead {
        title: layer.identifier.clone().filter(|i| i != &layer.name),
        name: layer.name.clone(),
        srs_id: layer.srs_id,
        fields,
        objects,
        fids,
        read: rows.len() as u32,
        records,
        skipped,
        losses,
    })
}

// ── The drawing's layers as tables ──────────────────────────────────────────

/// A table written.
#[derive(Clone, Debug, PartialEq)]
pub struct TableWritten {
    pub table: String,
    pub layer: String,
    pub rows: u32,
    pub geometry: Option<String>,
}

/// What [`write`] wrote.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Written {
    pub tables: Vec<TableWritten>,
    /// Objects not written, by kind.
    pub left: BTreeMap<String, u32>,
    /// Curves written as chords.
    pub approximated: u32,
    /// Attributes written under another column name (a name the table had in another case): table, from, to.
    pub renamed: Vec<(String, String, String)>,
}

fn c2(p: &kentos_contracts::Vec2, z: Option<f64>, m: Option<f64>) -> Coord {
    Coord {
        x: p.x,
        y: p.y,
        z,
        m,
    }
}

fn at<T: Copy>(v: &Option<Vec<Option<T>>>, i: usize) -> Option<T> {
    v.as_ref().and_then(|v| v.get(i).copied().flatten())
}

/// A path's vertices and bulges as a curve: straight runs as lines, each bulged edge an arc through its middle.
fn curve_of(
    pts: &[kentos_contracts::Vec2],
    bulges: Option<&Vec<f64>>,
    zs: &Option<Vec<Option<f64>>>,
    ms: &Option<Vec<Option<f64>>>,
    closed: bool,
) -> Curve {
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    let coord = |i: usize| c2(&pts[i % n], at(zs, i % n), at(ms, i % n));
    let bulge = |i: usize| bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0);
    if (0..edges).all(|i| bulge(i) == 0.0) {
        let mut line: Vec<Coord> = (0..n).map(coord).collect();
        if closed && n > 0 {
            line.push(coord(0));
        }
        return Curve::Line(line);
    }
    let mut parts: Vec<Curve> = Vec::new();
    let mut run: Vec<Coord> = vec![coord(0)];
    for i in 0..edges {
        let (a, b) = (coord(i), coord(i + 1));
        let k = bulge(i);
        if k == 0.0 {
            run.push(b);
        } else {
            if run.len() > 1 {
                parts.push(Curve::Line(std::mem::take(&mut run)));
            }
            // The arc's middle: the chord's middle moved by its sagitta, to the right for a positive bulge.
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let mid = Coord {
                x: (a.x + b.x) / 2.0 + dy * k / 2.0,
                y: (a.y + b.y) / 2.0 - dx * k / 2.0,
                z: match (a.z, b.z) {
                    (Some(p), Some(q)) => Some((p + q) / 2.0),
                    _ => a.z.or(b.z),
                },
                m: match (a.m, b.m) {
                    (Some(p), Some(q)) => Some((p + q) / 2.0),
                    _ => a.m.or(b.m),
                },
            };
            parts.push(Curve::Arc(vec![a, mid, b]));
            run = vec![b];
        }
    }
    if run.len() > 1 {
        parts.push(Curve::Line(run));
    }
    if parts.len() == 1 {
        return parts.pop().unwrap_or(Curve::Line(Vec::new()));
    }
    Curve::Compound(parts)
}

fn rings_of(
    pts: &[kentos_contracts::Vec2],
    bulges: Option<&Vec<f64>>,
    zs: &Option<Vec<Option<f64>>>,
    holes: Option<&Vec<kentos_contracts::RingGeometry>>,
) -> Vec<Curve> {
    let mut out = vec![curve_of(pts, bulges, zs, &None, true)];
    for h in holes.into_iter().flatten() {
        out.push(curve_of(&h.pts, h.bulges.as_ref(), &h.zs, &None, true));
    }
    out
}

/// What an object becomes: a geometry, a row without one (a record), or nothing (its kind), and whether it was
/// written as chords.
enum Row {
    Geometry(Geometry, bool),
    Record,
    Left(&'static str),
}

fn row_of(e: &Entity) -> Row {
    use kentos_contracts::Entity as E;
    match e {
        E::Record(_) => Row::Record,
        E::Point(p) => {
            let first = c2(&p.p, p.z, None);
            match &p.parts {
                Some(parts) if !parts.is_empty() => {
                    let mut all = vec![first];
                    all.extend(parts.iter().map(|q| c2(&q.p, q.z, None)));
                    Row::Geometry(Geometry::MultiPoint(all), false)
                }
                _ => Row::Geometry(Geometry::Point(Some(first)), false),
            }
        }
        E::Line(l) => Row::Geometry(
            Geometry::Curve(Curve::Line(vec![
                c2(&l.a, l.za, None),
                c2(&l.b, l.zb, None),
            ])),
            false,
        ),
        E::Polyline(p) => {
            let first = curve_of(&p.pts, p.bulges.as_ref(), &p.zs, &p.ms, p.closed);
            match &p.parts {
                Some(parts) if !parts.is_empty() => {
                    let mut all = vec![first];
                    all.extend(
                        parts
                            .iter()
                            .map(|q| curve_of(&q.pts, q.bulges.as_ref(), &q.zs, &q.ms, false)),
                    );
                    Row::Geometry(Geometry::MultiCurve(all), false)
                }
                _ => Row::Geometry(Geometry::Curve(first), false),
            }
        }
        E::Polygon(p) => {
            let first = rings_of(&p.pts, p.bulges.as_ref(), &p.zs, p.holes.as_ref());
            match &p.parts {
                Some(parts) if !parts.is_empty() => {
                    let mut all = vec![first];
                    all.extend(
                        parts
                            .iter()
                            .map(|q| rings_of(&q.pts, q.bulges.as_ref(), &q.zs, q.holes.as_ref())),
                    );
                    Row::Geometry(Geometry::MultiPolygon(all), false)
                }
                _ => Row::Geometry(Geometry::Polygon(first), false),
            }
        }
        E::Circle(c) => {
            let p = |dx: f64, dy: f64| Coord::xy(c.c.x + dx, c.c.y + dy);
            let r = c.r;
            Row::Geometry(
                Geometry::Polygon(vec![Curve::Arc(vec![
                    p(r, 0.0),
                    p(0.0, r),
                    p(-r, 0.0),
                    p(0.0, -r),
                    p(r, 0.0),
                ])]),
                false,
            )
        }
        E::Arc(a) => {
            let mut sweep = (a.a1 - a.a0).rem_euclid(std::f64::consts::TAU);
            if sweep == 0.0 {
                sweep = std::f64::consts::TAU;
            }
            let q = |t: f64| Coord::xy(a.c.x + a.r * t.cos(), a.c.y + a.r * t.sin());
            Row::Geometry(
                Geometry::Curve(Curve::Arc(vec![
                    q(a.a0),
                    q(a.a0 + sweep / 2.0),
                    q(a.a0 + sweep),
                ])),
                false,
            )
        }
        E::Ellipse(_) | E::Spline(_) | E::Nurbs(_) => {
            let shape = crate::geometry::shape(e);
            let areas = kentos_geometry_core::ops::areas::areas_of_entity(&shape);
            if let Some(a) = areas.first() {
                let mut ring: Vec<Coord> =
                    a.outer.pts.iter().map(|p| Coord::xy(p.x, p.y)).collect();
                if let Some(f) = ring.first().copied() {
                    ring.push(f);
                }
                return Row::Geometry(Geometry::Polygon(vec![Curve::Line(ring)]), true);
            }
            let edges = kentos_geometry_core::ops::edges::entity_edges(&shape);
            let mut line: Vec<Coord> = Vec::new();
            for ed in &edges {
                for t in [0.0, 1.0] {
                    let p = kentos_geometry_core::geom::intersect::point_at(ed, t);
                    if line.last().is_none_or(|q| q.x != p.x || q.y != p.y) {
                        line.push(Coord::xy(p.x, p.y));
                    }
                }
            }
            if line.len() < 2 {
                return Row::Left("eğri");
            }
            Row::Geometry(Geometry::Curve(Curve::Line(line)), true)
        }
        other => Row::Left(match other {
            E::Text(_) => "yazı",
            E::Dimension(_) => "ölçü",
            E::Insert(_) => "blok",
            E::Hatch(_) => "tarama",
            E::Leader(_) => "kılavuz",
            E::Table(_) => "tablo",
            E::Image(_) => "resim",
            E::Raster(_) => "raster",
            E::PointCloud(_) => "nokta bulutu",
            E::Xline(_) | E::Ray(_) => "yardımcı çizgi",
            _ => "başka nesne",
        }),
    }
}

/// The family a geometry belongs to, for the column's type.
fn family(g: &Geometry) -> (u8, bool, bool) {
    // (0 point, 1 line, 2 surface, 3 other; multi; curved)
    match g {
        Geometry::Point(_) => (0, false, false),
        Geometry::MultiPoint(_) => (0, true, false),
        Geometry::Curve(c) => (1, false, c.is_curved()),
        Geometry::MultiCurve(cs) => (1, true, cs.iter().any(Curve::is_curved)),
        Geometry::Polygon(r) => (2, false, r.iter().any(Curve::is_curved)),
        Geometry::MultiPolygon(p) => (2, true, p.iter().flatten().any(Curve::is_curved)),
        Geometry::Collection(_) => (3, true, false),
    }
}

/// The column's type and the geometries fitted to it (singles made multi where the column is).
fn fit(geoms: &mut [Option<Geometry>]) -> Option<String> {
    let fams: Vec<(u8, bool, bool)> = geoms.iter().flatten().map(family).collect();
    let first = fams.first()?;
    if fams.iter().any(|f| f.0 != first.0) {
        return Some("GEOMETRY".into());
    }
    let multi = fams.iter().any(|f| f.1);
    let curved = fams.iter().any(|f| f.2);
    let name = match (first.0, multi, curved) {
        (0, false, _) => "POINT",
        (0, true, _) => "MULTIPOINT",
        (1, false, false) => "LINESTRING",
        (1, false, true) => "COMPOUNDCURVE",
        (1, true, false) => "MULTILINESTRING",
        (1, true, true) => "MULTICURVE",
        (2, false, false) => "POLYGON",
        (2, false, true) => "CURVEPOLYGON",
        (2, true, false) => "MULTIPOLYGON",
        (2, true, true) => "MULTISURFACE",
        _ => "GEOMETRY",
    };
    for g in geoms.iter_mut().flatten() {
        let next = match (name, std::mem::replace(g, Geometry::Point(None))) {
            ("MULTIPOINT", Geometry::Point(Some(p))) => Geometry::MultiPoint(vec![p]),
            ("MULTILINESTRING" | "MULTICURVE", Geometry::Curve(c)) => Geometry::MultiCurve(vec![c]),
            ("MULTIPOLYGON" | "MULTISURFACE", Geometry::Polygon(r)) => {
                Geometry::MultiPolygon(vec![r])
            }
            ("COMPOUNDCURVE", Geometry::Curve(c @ (Curve::Line(_) | Curve::Arc(_)))) => {
                Geometry::Curve(Curve::Compound(vec![c]))
            }
            (_, other) => other,
        };
        *g = next;
    }
    Some(name.into())
}

pub(crate) fn column_of(f: &LayerField) -> gpkg::Column {
    let kind = match f.kind {
        LayerFieldKind::Text => ColumnKind::Text(f.length),
        LayerFieldKind::Integer => ColumnKind::Integer,
        LayerFieldKind::Decimal => ColumnKind::Real,
        LayerFieldKind::Date => ColumnKind::Date,
        LayerFieldKind::DateTime => ColumnKind::DateTime,
        LayerFieldKind::Boolean => ColumnKind::Boolean,
        LayerFieldKind::Time => ColumnKind::Text(None),
    };
    gpkg::Column {
        name: f.name.clone(),
        decl: String::new(),
        kind,
        not_null: false,
        default: None,
    }
}

fn cell(text: Option<&String>, kind: &ColumnKind) -> Value {
    let Some(t) = text else {
        return Value::Null;
    };
    match kind {
        ColumnKind::Integer => t
            .parse::<i64>()
            .map_or_else(|_| Value::Text(t.clone()), Value::Integer),
        ColumnKind::Real => t
            .parse::<f64>()
            .map_or_else(|_| Value::Text(t.clone()), Value::Real),
        ColumnKind::Boolean => match t.as_str() {
            "true" => Value::Integer(1),
            "false" => Value::Integer(0),
            _ => Value::Text(t.clone()),
        },
        ColumnKind::DateTime if t.len() == 19 => Value::Text(format!("{t}Z")),
        _ => Value::Text(t.clone()),
    }
}

/// The project's coordinate system as a GeoPackage's (its EPSG code and WKT); the undefined cartesian one for a local
/// project.
fn srs_of(doc: &Document) -> gpkg::Srs {
    let srid = doc.settings().srid;
    let found = kentos_project::crs::system(srid).and_then(|s| {
        let core = s.transform_system()?;
        let wkt = kentos_geometry_core::crs::text::write_wkt(&s.name, &core, None)?;
        Some(gpkg::Srs {
            name: s.name.clone(),
            id: srid as i32,
            organization: "EPSG".into(),
            organization_id: srid as i32,
            definition: wkt,
            description: None,
        })
    });
    found.unwrap_or(gpkg::Srs {
        name: "Undefined cartesian SRS".into(),
        id: -1,
        organization: "NONE".into(),
        organization_id: -1,
        definition: "undefined".into(),
        description: None,
    })
}

/// Writes the layers (by id; every layer with objects when empty) to a new GeoPackage at `path`.
pub fn write(doc: &Document, layers: &[String], path: &Path) -> Result<Written, String> {
    let mut gp = GeoPackage::create(path).map_err(|e| e.0)?;
    let srs = srs_of(doc);
    gp.add_srs(&srs).map_err(|e| e.0)?;
    let ids: Vec<String> = if layers.is_empty() {
        doc.layers().leaves().iter().map(|n| n.id.clone()).collect()
    } else {
        layers.to_vec()
    };
    let mut out = Written::default();
    let mut names: BTreeSet<String> = BTreeSet::new();
    for id in ids {
        let Some(node) = doc.layers().get(&id) else {
            continue;
        };
        let objects: Vec<(Uuid, &Entity)> = doc.by_layer_with_uids(&id).collect();
        if objects.is_empty() {
            continue;
        }
        let mut geoms: Vec<Option<Geometry>> = Vec::with_capacity(objects.len());
        let mut kept: Vec<(Uuid, &Entity)> = Vec::with_capacity(objects.len());
        for (uid, e) in objects {
            match row_of(e) {
                Row::Geometry(g, chords) => {
                    if chords {
                        out.approximated += 1;
                    }
                    geoms.push(Some(g));
                    kept.push((uid, e));
                }
                Row::Record => {
                    geoms.push(None);
                    kept.push((uid, e));
                }
                Row::Left(kind) => *out.left.entry(kind.into()).or_default() += 1,
            }
        }
        if kept.is_empty() {
            continue;
        }
        let type_name = fit(&mut geoms);
        let (mut z, mut m) = (false, false);
        let (mut all_z, mut all_m) = (true, true);
        for g in geoms.iter().flatten() {
            let (gz, gm) = g.dims();
            z |= gz;
            m |= gm;
            all_z &= gz;
            all_m &= gm;
        }
        let flag = |any: bool, all: bool| {
            if !any {
                0
            } else if all {
                1
            } else {
                2
            }
        };
        // Each attribute's column: the field's, else a text column. SQLite's names are one whatever their case, so a
        // name the table has already (`Ada` beside `ada`, `fid`, `geom`) gets `_2`, `_3` …; `keys` is the attribute
        // each column reads.
        let mut columns: Vec<gpkg::Column> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        let mut taken: BTreeSet<String> = ["fid".to_owned(), "geom".to_owned()].into();
        let mut add = |name: &str,
                       kind: ColumnKind,
                       columns: &mut Vec<gpkg::Column>,
                       keys: &mut Vec<String>| {
            if keys.iter().any(|k| k == name) {
                return;
            }
            let mut col = name.to_owned();
            let mut n = 2;
            while !taken.insert(col.to_ascii_lowercase()) {
                col = format!("{name}_{n}");
                n += 1;
            }
            keys.push(name.to_owned());
            columns.push(gpkg::Column {
                name: col,
                decl: String::new(),
                kind,
                not_null: false,
                default: None,
            });
        };
        for f in &node.fields {
            add(&f.name, column_of(f).kind, &mut columns, &mut keys);
        }
        let mut extra: BTreeSet<String> = BTreeSet::new();
        for (_, e) in &kept {
            extra.extend(e.base().attrs.keys().cloned());
        }
        for k in extra {
            add(&k, ColumnKind::Text(None), &mut columns, &mut keys);
        }
        for (c, k) in columns.iter().zip(&keys) {
            if &c.name != k {
                out.renamed
                    .push((node.name.clone(), k.clone(), c.name.clone()));
            }
        }
        let mut table = node.name.trim().to_owned();
        if table.is_empty()
            || table.to_ascii_lowercase().starts_with("gpkg_")
            || table.starts_with("rtree_")
        {
            table = format!("katman_{table}");
        }
        let base = table.clone();
        let mut k = 2;
        while !names.insert(table.to_ascii_lowercase()) {
            table = format!("{base}_{k}");
            k += 1;
        }
        let rows: Vec<gpkg::NewRow> = kept
            .iter()
            .zip(geoms)
            .map(|((_, e), g)| gpkg::NewRow {
                fid: None,
                geometry: g,
                values: columns
                    .iter()
                    .zip(&keys)
                    .map(|(c, k)| cell(e.base().attrs.get(k), &c.kind))
                    .collect(),
            })
            .collect();
        let uids: Vec<String> = kept.iter().map(|(u, _)| u.to_string()).collect();
        let def = gpkg::TableDef {
            name: table.clone(),
            data_type: if type_name.is_some() {
                DataType::Features
            } else {
                DataType::Attributes
            },
            identifier: Some(table.clone()),
            description: None,
            srs_id: type_name.as_ref().map(|_| srs.id),
            geometry: type_name.clone().map(|t| gpkg::GeometryColumn {
                name: "geom".into(),
                type_name: t,
                z: flag(z, all_z),
                m: flag(m, all_m),
            }),
            columns,
        };
        gp.write_table(&def, &rows, Some(&uids)).map_err(|e| e.0)?;
        out.tables.push(TableWritten {
            table,
            layer: id.clone(),
            rows: rows.len() as u32,
            geometry: type_name,
        });
    }
    gp.close().map_err(|e| e.0)?;
    Ok(out)
}

// ── One object against a table (docs/adr/0503) ─────────────────────────────

/// What a layer and its table are compared on: an object's kind, geometry and attributes (not its look).
pub fn essence(e: &Entity) -> J {
    const KEYS: [&str; 11] = [
        "kind", "attrs", "p", "z", "parts", "pts", "bulges", "zs", "ms", "holes", "closed",
    ];
    let mut out = serde_json::Map::new();
    if let J::Object(m) = serde_json::to_value(e).unwrap_or(J::Null) {
        for (k, v) in m {
            if KEYS.contains(&k.as_str()) {
                out.insert(k, v);
            }
        }
    }
    J::Object(out)
}

/// `e` with `from`'s kind, geometry and attributes, its look kept (a row read again into a bound layer).
pub fn with_essence(e: &Entity, from: &Entity) -> Option<Entity> {
    let J::Object(mut old) = serde_json::to_value(e).ok()? else {
        return None;
    };
    let J::Object(new) = essence(from) else {
        return None;
    };
    for k in [
        "kind", "attrs", "p", "z", "parts", "pts", "bulges", "zs", "ms", "holes", "closed",
        "widths", "a", "b", "za", "zb", "c", "r", "a0", "a1",
    ] {
        old.remove(k);
    }
    old.extend(new);
    serde_json::from_value(J::Object(old)).ok()
}

/// A geometry fitted to a column of type `name`: a single made multi where the column is, an arc's curve a compound
/// curve; the reason when the column cannot hold it (another family, or an arc in a straight-edged column).
pub fn fit_one(g: Geometry, name: &str) -> Result<Geometry, String> {
    let name = name.to_ascii_uppercase();
    if name == "GEOMETRY" {
        return Ok(g);
    }
    let (fam, _, curved) = family(&g);
    let (want, linear) = match name.as_str() {
        "POINT" | "MULTIPOINT" => (0, true),
        "LINESTRING" | "MULTILINESTRING" => (1, true),
        "CIRCULARSTRING" | "COMPOUNDCURVE" | "CURVE" | "MULTICURVE" => (1, false),
        "POLYGON" | "MULTIPOLYGON" => (2, true),
        "CURVEPOLYGON" | "SURFACE" | "MULTISURFACE" => (2, false),
        _ => return Ok(g),
    };
    if fam != want {
        return Err(format!(
            "{} geometrisi {name} sütununa yazılamaz",
            g.type_name()
        ));
    }
    if curved && linear {
        return Err(format!(
            "yaylı {} {name} sütununa kesin yazılamaz",
            g.type_name()
        ));
    }
    let mut one = [Some(g)];
    let multi = name.starts_with("MULTI");
    if let Some(g) = one[0].take() {
        one[0] = Some(match (multi, g) {
            (true, Geometry::Point(Some(p))) => Geometry::MultiPoint(vec![p]),
            (true, Geometry::Curve(c)) => Geometry::MultiCurve(vec![c]),
            (true, Geometry::Polygon(r)) => Geometry::MultiPolygon(vec![r]),
            (false, Geometry::MultiPoint(mut p)) if p.len() == 1 => Geometry::Point(p.pop()),
            (false, Geometry::MultiCurve(mut c)) if c.len() == 1 => {
                Geometry::Curve(c.pop().unwrap_or(Curve::Line(Vec::new())))
            }
            (false, Geometry::MultiPolygon(mut p)) if p.len() == 1 => {
                Geometry::Polygon(p.pop().unwrap_or_default())
            }
            (
                false,
                g @ (Geometry::MultiPoint(_) | Geometry::MultiCurve(_) | Geometry::MultiPolygon(_)),
            ) => {
                return Err(format!(
                    "çok parçalı {} {name} sütununa yazılamaz",
                    g.type_name()
                ));
            }
            (_, Geometry::Curve(c @ (Curve::Line(_) | Curve::Arc(_))))
                if name == "COMPOUNDCURVE" =>
            {
                Geometry::Curve(Curve::Compound(vec![c]))
            }
            (_, g) => g,
        });
    }
    one[0].take().ok_or_else(|| "geometri yok".to_owned())
}

/// An object as a row of a table with `columns` and a geometry column of type `geometry`: its geometry fitted, its
/// attributes by column (the name, else the name in another case). `Ok(None)`: a kind the table has no geometry for.
pub fn row_for(
    e: &Entity,
    columns: &[gpkg::Column],
    geometry: Option<&str>,
) -> Result<Option<gpkg::NewRow>, String> {
    let g = match row_of(e) {
        Row::Left(_) => return Ok(None),
        Row::Record => None,
        Row::Geometry(g, _) => match geometry {
            Some(t) => Some(fit_one(g, t)?),
            None => return Err("öznitelik tablosuna geometrili nesne yazılamaz".into()),
        },
    };
    let attrs = &e.base().attrs;
    let values = columns
        .iter()
        .map(|c| {
            let v = attrs.get(&c.name).or_else(|| {
                attrs
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(&c.name))
                    .map(|(_, v)| v)
            });
            cell(v, &c.kind)
        })
        .collect();
    Ok(Some(gpkg::NewRow {
        fid: None,
        geometry: g,
        values,
    }))
}

/// A column's field kind, as the import reads it (none for a blob).
pub fn column_kind(c: &gpkg::Column) -> Option<LayerFieldKind> {
    field_of(c, false).map(|f| f.kind)
}
