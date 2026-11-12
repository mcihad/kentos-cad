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
//! Values: NULL is no attribute, an empty text the empty text. How a column reads (docs/adr/0629 §4, [`Reading`]): an
//! integer column holding a value past ±(2⁵³ − 1) is a big integer field; a DATETIME column holding an offset or a
//! fraction a time stamp field (a value without an offset is UTC), else a local date-and-time field without the `Z`;
//! a blob column a binary field (a value past 1 MiB left out, said); KentOS's document of the table
//! (`https://kentos.app/fields/1`) or a database's type (`uuid`, `json`, `jsonb`, `timestamptz`, `numeric`) says a
//! UUID, JSON, time stamp or exact decimal column when its every value is one. Written: a time stamp in UTC.
//!
//! Lasting ids: a row's id is the file's `kentos_uids` (written by KentOS), else UUIDv5 of the table and fid in a
//! namespace of this module: the same row of the same table gets the same id every time.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use kentos_commands::identity::uuid_v5;
use kentos_commands::{
    Entity, LayerField, LayerFieldKind, MAX_BINARY, MAX_INTEGER, binary_bytes, check_value,
    instant_moment, instant_utc,
};
use kentos_commands::encoding::base64_encode;
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
    /// The table's indexes as the layer's (docs/adr/0758 §5).
    pub indexes: Vec<kentos_commands::LayerIndex>,
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
    /// The heights' system of an srs id whose definition is compound (docs/adr/0631 §4).
    pub verticals: BTreeMap<i32, kentos_formats::vertical::VerticalCrs>,
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

/// KentOS's document of a table's fields (docs/adr/0629 §4): the kinds a GeoPackage's column types do not say,
/// `{"kinds": {"column": "kind"}}`, on the table.
pub const FIELDS_STANDARD: &str = "https://kentos.app/fields/1";

/// How a column's cells read into the drawing (docs/adr/0629 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reading {
    /// docs/adr/0502's rules.
    Plain,
    /// 64-bit whole numbers: a big integer field.
    Big,
    /// Times with their offsets (none: UTC): a time stamp field.
    Instant,
    Uuid,
    Json,
    /// Numbers written exactly (a database's `numeric`): a decimal field.
    Exact,
    /// Bytes: a binary field.
    Binary,
}

/// What a column's declared type says of its values (a database's type; a GeoPackage's are all plain).
fn hint_of_decl(decl: &str) -> Option<LayerFieldKind> {
    let d = decl.trim().to_ascii_lowercase();
    let base = d.split('(').next().unwrap_or_default().trim();
    match base {
        "uuid" => Some(LayerFieldKind::Uuid),
        "json" | "jsonb" => Some(LayerFieldKind::Json),
        "timestamptz" | "timestamp with time zone" => Some(LayerFieldKind::Instant),
        "numeric" | "decimal" => Some(LayerFieldKind::Decimal),
        _ => None,
    }
}

/// A text as the kind's canonical text, when it is one (a time stamp without an offset is UTC).
fn canonical_as(kind: LayerFieldKind, t: &str) -> Option<String> {
    let field = LayerField::new("", kind);
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    match check_value(&field, t) {
        Ok(v) if !v.is_empty() => Some(v),
        _ if kind == LayerFieldKind::Instant => check_value(&field, &format!("{t}Z"))
            .ok()
            .filter(|v| !v.is_empty()),
        _ => None,
    }
}

/// Whether a DATETIME text holds more than a local field keeps: an offset other than `Z`, or a fraction.
fn zoned(t: &str) -> bool {
    t.len() > 19 && &t[19..] != "Z"
}

/// The reading of column `i` with `rows`' values, a document's kind `hint` beside its declared type's.
fn reading_of(
    c: &gpkg::Column,
    i: usize,
    rows: &[gpkg::Row],
    hint: Option<LayerFieldKind>,
) -> Reading {
    let hint = hint.or_else(|| hint_of_decl(&c.decl));
    // A type that says its times are local (a Parquet timestamp not adjusted to UTC): a fraction makes no moment.
    let local = matches!(
        c.decl.trim().to_ascii_lowercase().as_str(),
        "timestamp" | "timestamp without time zone"
    );
    let texts = || {
        rows.iter().filter_map(move |r| match &r.values[i] {
            Value::Text(t) => Some(t.as_str()),
            _ => None,
        })
    };
    let all_fit = |kind| {
        rows.iter().all(|r| match &r.values[i] {
            Value::Null => true,
            Value::Text(t) => canonical_as(kind, t).is_some(),
            Value::Integer(_) => kind == LayerFieldKind::BigInt,
            _ => false,
        })
    };
    match (&c.kind, hint) {
        (ColumnKind::Blob(_), _) => Reading::Binary,
        (ColumnKind::Integer, Some(LayerFieldKind::BigInt)) => Reading::Big,
        (ColumnKind::Integer, _)
            if rows.iter().any(
                |r| matches!(r.values[i], Value::Integer(v) if v.unsigned_abs() > MAX_INTEGER),
            ) =>
        {
            Reading::Big
        }
        (ColumnKind::DateTime, Some(LayerFieldKind::Instant)) => Reading::Instant,
        (ColumnKind::DateTime, _) if !local && texts().any(zoned) => Reading::Instant,
        (ColumnKind::Text(_) | ColumnKind::Other(_), Some(kind)) if all_fit(kind) => match kind {
            LayerFieldKind::Uuid => Reading::Uuid,
            LayerFieldKind::Json => Reading::Json,
            LayerFieldKind::Instant => Reading::Instant,
            LayerFieldKind::BigInt => Reading::Big,
            LayerFieldKind::Decimal => Reading::Exact,
            _ => Reading::Plain,
        },
        _ => Reading::Plain,
    }
}

/// The kinds KentOS's document gives a table's columns (docs/adr/0629 §4); none without one.
pub(crate) fn field_hints(
    gp: &GeoPackage,
    table: &str,
) -> Result<BTreeMap<String, LayerFieldKind>, String> {
    let mut out = BTreeMap::new();
    for (standard, text) in gp.metadata(table).map_err(|e| e.0)? {
        if standard != FIELDS_STANDARD {
            continue;
        }
        let Ok(doc) = serde_json::from_str::<J>(&text) else {
            continue;
        };
        for (column, kind) in doc["kinds"].as_object().into_iter().flatten() {
            if let Some(k) = kind.as_str().and_then(LayerFieldKind::from_name) {
                out.insert(column.clone(), k);
            }
        }
    }
    Ok(out)
}

/// KentOS's document of the kinds of a table's columns its types do not say (docs/adr/0629 §4); none when every field
/// is of a kind a GeoPackage says itself.
pub(crate) fn field_document(fields: &[LayerField], columns: &[(String, String)]) -> Option<String> {
    let kinds: serde_json::Map<String, J> = fields
        .iter()
        .filter(|f| f.kind.is_wide())
        .filter_map(|f| {
            let column = columns.iter().find(|(k, _)| *k == f.name)?;
            Some((column.1.clone(), J::String(f.kind.name().to_owned())))
        })
        .collect();
    (!kinds.is_empty()).then(|| json!({ "kinds": kinds }).to_string())
}

/// A field of a column read so; none for a blob column read plainly (never: a blob is binary).
fn field_of(c: &gpkg::Column, reading: Reading) -> Option<LayerField> {
    let (kind, length) = match (reading, &c.kind) {
        (Reading::Big, _) => (LayerFieldKind::BigInt, None),
        (Reading::Instant, _) => (LayerFieldKind::Instant, None),
        (Reading::Uuid, _) => (LayerFieldKind::Uuid, None),
        (Reading::Json, _) => (LayerFieldKind::Json, None),
        (Reading::Exact, _) => (LayerFieldKind::Decimal, None),
        (Reading::Binary, _) => (LayerFieldKind::Binary, None),
        (Reading::Plain, ColumnKind::Integer) => (LayerFieldKind::Integer, None),
        (Reading::Plain, ColumnKind::Real) => (LayerFieldKind::Decimal, None),
        (Reading::Plain, ColumnKind::Boolean) => (LayerFieldKind::Boolean, None),
        (Reading::Plain, ColumnKind::Date) => (LayerFieldKind::Date, None),
        (Reading::Plain, ColumnKind::DateTime) => (LayerFieldKind::DateTime, None),
        (Reading::Plain, ColumnKind::Text(n)) => (LayerFieldKind::Text, *n),
        (Reading::Plain, ColumnKind::Blob(_)) => return None,
        (Reading::Plain, ColumnKind::Other(_)) => (LayerFieldKind::Text, None),
    };
    let mut f = json!({ "name": c.name, "kind": kind });
    if let Some(n) = length {
        f["length"] = json!(n);
    }
    // A database's numeric(p, s) keeps s decimals (docs/adr/0755 §3): the field takes no more, so a constraint judged
    // on its value judges what the table holds.
    if kind == LayerFieldKind::Decimal
        && let Some(s) = numeric_scale(&c.decl)
    {
        f["scale"] = json!(s);
    }
    // The table's rules (docs/adr/0750): NOT NULL without a default is a required field; a literal default is the
    // field's, a new object takes it; a default the database works out (`now()`, `uuidv7()`) it gives a new row itself.
    if c.not_null && c.default.is_none() {
        f["required"] = json!(true);
    }
    if let Some(d) = c
        .default
        .as_deref()
        .and_then(literal_default)
        .and_then(|t| default_as(kind, &t))
    {
        f["default"] = json!(d);
    }
    serde_json::from_value(f).ok()
}

/// The decimals a `numeric(p, s)` or `decimal(p, s)` type keeps, when they are a field's (at most
/// [`kentos_commands::MAX_SCALE`]).
fn numeric_scale(decl: &str) -> Option<u32> {
    let d = decl.trim().to_ascii_lowercase();
    let inner = d
        .strip_prefix("numeric")
        .or_else(|| d.strip_prefix("decimal"))?
        .trim()
        .strip_prefix('(')?
        .strip_suffix(')')?;
    let (_, s) = inner.split_once(',')?;
    s.trim()
        .parse::<u32>()
        .ok()
        .filter(|s| *s <= kentos_commands::MAX_SCALE)
}

/// A column's default as a value, when it is a literal (docs/adr/0750): a quoted text (`'İlçe'`,
/// `'İlçe'::character varying`), a number (`0`, `(-1)`, `2.5::numeric`), `true` or `false`, as PostgreSQL's
/// `pg_get_expr` and SQLite's `table_info` write it; none for NULL and for an expression the database works out
/// (`now()`, `uuidv7()`, `nextval(…)`, `CURRENT_TIMESTAMP`), which it gives a new row itself.
pub fn literal_default(sql: &str) -> Option<String> {
    let mut t = sql.trim();
    while t.len() >= 2 && t.starts_with('(') && t.ends_with(')') && closes_last(t) {
        t = t[1..t.len() - 1].trim();
    }
    let t = before_cast(t).trim();
    if let Some(rest) = t.strip_prefix('\'') {
        let mut out = String::new();
        let mut chars = rest.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\'' {
                out.push(c);
            } else if chars.peek() == Some(&'\'') {
                chars.next();
                out.push('\'');
            } else {
                return chars.next().is_none().then_some(out);
            }
        }
        return None;
    }
    match t.to_ascii_lowercase().as_str() {
        "true" => return Some("true".into()),
        "false" => return Some("false".into()),
        _ => {}
    }
    let n = t.trim_start_matches('(').trim_end_matches(')').trim();
    let numeric = !n.is_empty()
        && n.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E'))
        && n.parse::<f64>().is_ok_and(f64::is_finite);
    numeric.then(|| n.to_owned())
}

/// Whether the `(` that starts `t` closes at its last character (not `(a) + (b)`), quotes skipped.
fn closes_last(t: &str) -> bool {
    let mut depth = 0i32;
    let mut quoted = false;
    for (i, c) in t.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                depth -= 1;
                if depth == 0 {
                    return i + 1 == t.len();
                }
            }
            _ => {}
        }
    }
    false
}

/// `t` before its first cast (`::`) outside quotes and parentheses.
fn before_cast(t: &str) -> &str {
    let mut depth = 0i32;
    let mut quoted = false;
    let bytes = t.as_bytes();
    for (i, c) in t.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ':' if !quoted && depth == 0 && bytes.get(i + 1) == Some(&b':') => return &t[..i],
            _ => {}
        }
    }
    t
}

/// A literal default in the canonical text of the field's kind; none when it does not read as one. A boolean's
/// `0` and `1` are SQLite's false and true.
fn default_as(kind: LayerFieldKind, t: &str) -> Option<String> {
    let t = match (kind, t) {
        (LayerFieldKind::Boolean, "0") => "false",
        (LayerFieldKind::Boolean, "1") => "true",
        _ => t,
    };
    if t.is_empty() && kind == LayerFieldKind::Text {
        return None;
    }
    canonical_as(kind, t)
}

/// A cell as the attribute it reads as on a layer whose field for its column is of `field`'s kind (docs/adr/0750): the
/// values a table gave a new row's columns left to their defaults.
pub(crate) fn field_text(
    v: &Value,
    c: &gpkg::Column,
    field: Option<LayerFieldKind>,
) -> Option<String> {
    let mut quiet = BTreeSet::new();
    value_text(v, &c.kind, reading_for(field), &mut quiet, &c.name)
}

/// The reading a layer's field of `field`'s kind gives its column (a plain one when the layer has no such field).
fn reading_for(field: Option<LayerFieldKind>) -> Reading {
    match field {
        Some(LayerFieldKind::BigInt) => Reading::Big,
        Some(LayerFieldKind::Instant) => Reading::Instant,
        Some(LayerFieldKind::Uuid) => Reading::Uuid,
        Some(LayerFieldKind::Json) => Reading::Json,
        Some(LayerFieldKind::Binary) => Reading::Binary,
        // A blob column the layer has no field for is not read (docs/adr/0512): only the same with the same.
        _ => Reading::Plain,
    }
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
    reading: Reading,
    losses: &mut BTreeSet<String>,
    col: &str,
) -> Option<String> {
    let as_kind = |k: LayerFieldKind, t: &str| canonical_as(k, t).unwrap_or_else(|| t.to_owned());
    match (reading, v) {
        (_, Value::Null) => return None,
        (Reading::Big, Value::Integer(i)) => return Some(i.to_string()),
        (Reading::Big, Value::Text(t)) => return Some(as_kind(LayerFieldKind::BigInt, t)),
        (Reading::Instant, Value::Text(t)) => return Some(as_kind(LayerFieldKind::Instant, t)),
        (Reading::Uuid, Value::Text(t)) => return Some(as_kind(LayerFieldKind::Uuid, t)),
        (Reading::Json, Value::Text(t)) => return Some(as_kind(LayerFieldKind::Json, t)),
        (Reading::Exact, Value::Text(t)) => return Some(as_kind(LayerFieldKind::Decimal, t)),
        (Reading::Binary, Value::Blob(b)) if b.len() > MAX_BINARY => {
            losses.insert(format!(
                "“{col}” sütununun 1 MiB'ı aşan ikili değerleri alınmadı"
            ));
            return None;
        }
        (Reading::Binary, Value::Blob(b)) => return Some(base64_encode(b)),
        _ => {}
    }
    let wide = reading == Reading::Big;
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

/// A cell as the attribute's text by its column's declared type alone (docs/adr/0634 §1): a derived row taken back.
pub(crate) fn cell_text(v: &Value, kind: &ColumnKind) -> Option<String> {
    let mut losses = BTreeSet::new();
    value_text(v, kind, Reading::Plain, &mut losses, "")
}

/// Whether two cells of a column mean the same to the drawing (plan/gis.md CBS-06, docs/adr/0512): both read to the same
/// attribute text. An integer is compared by its digits (past 2⁵³ as well), a double by the text it reads back to, a
/// time by its date and time of day (a zone the drawing does not keep is not a change: a row is not written over for
/// it), NULL only with NULL.
pub(crate) fn same_cell(a: &Value, b: &Value, kind: &ColumnKind) -> bool {
    same_cell_as(a, b, kind, None)
}

/// [`same_cell`] for the layer's field of `field`'s kind (docs/adr/0629 §4): a time stamp is the same moment, a UUID the
/// same digits, JSON the same canonical text, binary the same bytes.
pub(crate) fn same_cell_as(
    a: &Value,
    b: &Value,
    kind: &ColumnKind,
    field: Option<LayerFieldKind>,
) -> bool {
    let reading = reading_for(field);
    if reading == Reading::Binary {
        return a == b;
    }
    let mut quiet = BTreeSet::new();
    let (ta, tb) = (
        value_text(a, kind, reading, &mut quiet, ""),
        value_text(b, kind, reading, &mut quiet, ""),
    );
    if reading == Reading::Instant
        && let (Some(x), Some(y)) = (&ta, &tb)
        && let (Some(mx), Some(my)) = (instant_moment(x), instant_moment(y))
    {
        return mx == my;
    }
    ta == tb
}

/// Reads the tables `wanted` names (every feature and attribute table when none).
pub fn read(path: &Path, wanted: Option<&[String]>) -> Result<Read, String> {
    read_in(path, wanted, None)
}

/// [`read`], only the rows whose box meets `area` (in the tables' own system; docs/adr/0510) when one is given.
pub fn read_in(
    path: &Path,
    wanted: Option<&[String]>,
    area: Option<[f64; 4]>,
) -> Result<Read, String> {
    // The file's digest read a piece at a time: a large file is not held whole beside its rows (plan/gis.md CBS-42),
    // and known again while the file is the same (docs/adr/0597).
    let (size, sha256) = crate::file_digest::sha256(path)
        .map_err(|e| format!("“{}” okunamadı: {e}", path.display()))?;
    let gp = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let layers = gp.layers().map_err(|e| e.0)?;
    if let Some(w) = wanted
        && let Some(missing) = w.iter().find(|t| !layers.iter().any(|l| &l.name == *t))
    {
        return Err(format!("“{missing}” tablosu dosyada yok"));
    }
    // QGIS's styles and the related tables' mappings are no data of their own: their content comes in as the layers'
    // looks and reference fields (docs/adr/0627 §6, §7); such a table is read only when it is named.
    let beside: BTreeSet<String> = match wanted {
        Some(_) => BTreeSet::new(),
        None => gp.side_tables().map_err(|e| e.0)?,
    };
    let mut tables = Vec::new();
    let mut systems = BTreeMap::new();
    let mut verticals = BTreeMap::new();
    for layer in layers {
        let chosen = match wanted {
            Some(w) => w.contains(&layer.name),
            None => {
                matches!(layer.data_type, DataType::Features | DataType::Attributes)
                    && !beside.contains(&layer.name)
            }
        };
        if !chosen {
            continue;
        }
        if let Some(id) = layer.srs_id
            && !systems.contains_key(&id)
        {
            let srs = gp
                .srs(id)
                .map_err(|e| e.0)?
                .filter(|s| s.definition != "undefined" && id > 0);
            let org = srs.as_ref().map(|s| {
                let org = (s.organization.to_ascii_uppercase(), s.organization_id);
                if org.0 == "EPSG" {
                    return org;
                }
                // A compound system GDAL keeps under its own id (organization NONE): its horizontal half's EPSG code
                // from its definition, as a .prj's is read (docs/adr/0631 §4).
                match kentos_formats::shp::prj::read(s.definition.as_bytes()).srid {
                    Some(code) if s.definition.to_ascii_uppercase().starts_with("COMPD_CS") => {
                        ("EPSG".to_owned(), i32::try_from(code).unwrap_or(0))
                    }
                    _ => org,
                }
            });
            if let Some(v) = srs
                .as_ref()
                .and_then(|s| kentos_formats::vertical::vertical_of(&s.definition))
            {
                verticals.insert(id, v);
            }
            systems.insert(id, org);
        }
        let rows = gp
            .rows(
                &layer,
                &Query {
                    bbox: area,
                    ..Query::default()
                },
            )
            .map_err(|e| e.0)?;
        // A part's rows take only their own lasting ids, not the table's every one (docs/adr/0597).
        let uids = match area {
            Some(_) => {
                let fids: Vec<i64> = rows.iter().map(|r| r.fid).collect();
                gp.uids_of(&layer.name, &fids)
            }
            None => gp.uids(&layer.name),
        }
        .map_err(|e| e.0)?;
        let name = layer.name.clone();
        let hints = field_hints(&gp, &layer.name)?;
        let mut table =
            table_of_rows_hinted(&layer, &rows, &uids, &|fid| row_uid(&name, fid), &hints)?;
        // The table's indexes (docs/adr/0758 §5); one that cannot be read is no index of the layer.
        let read = gp.indexes(&layer.name).unwrap_or_default();
        table.indexes = layer_indexes_of(&layer.name, &table.fields, &read);
        tables.push(table);
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
        verticals,
    })
}

/// Rows worth another thread when made into objects (docs/adr/0757 §2).
const ROWS_SPLIT: usize = 1 << 11;

/// A table's row made into an object: its lasting id, the object, its row's fid and whether it is a record; or its
/// fid and why it was left out. Nearly every row is an object, moved on as it is: boxing it would allocate once more
/// for each.
#[allow(clippy::large_enum_variant)]
enum Made {
    Object(Uuid, Entity, i64, bool),
    Skipped(i64, String),
}

/// A part of the rows made into objects, with the losses said on the way; or the first refusal.
type Part = Result<(Vec<Made>, BTreeSet<String>), String>;

/// A table's rows as the drawing's objects (docs/adr/0502 §3; a PostGIS table's too, docs/adr/0505): each row an
/// object or a record with its lasting id (`uids`, else `uid_of` its fid), the columns as fields, what does not come
/// over said.
pub fn table_of_rows(
    layer: &gpkg::Layer,
    rows: &[gpkg::Row],
    uids: &std::collections::HashMap<i64, String>,
    uid_of: &(dyn Fn(i64) -> Uuid + Sync),
) -> Result<TableRead, String> {
    table_of_rows_hinted(layer, rows, uids, uid_of, &BTreeMap::new())
}

/// [`table_of_rows`] with the kinds a document of the file gives its columns (`hints`, by column name; docs/adr/0629
/// §4).
pub fn table_of_rows_hinted(
    layer: &gpkg::Layer,
    rows: &[gpkg::Row],
    uids: &std::collections::HashMap<i64, String>,
    uid_of: &(dyn Fn(i64) -> Uuid + Sync),
    hints: &BTreeMap<String, LayerFieldKind>,
) -> Result<TableRead, String> {
    let mut losses = BTreeSet::new();
    let wide: Vec<Reading> = layer
        .columns
        .iter()
        .enumerate()
        .map(|(i, c)| reading_of(c, i, rows, hints.get(&c.name).copied()))
        .collect();
    let mut fields = Vec::new();
    for (c, w) in layer.columns.iter().zip(&wide) {
        match field_of(c, *w) {
            Some(f) => fields.push(f),
            None => {
                losses.insert(format!("“{}” ikili sütunu alınmadı", c.name));
            }
        }
    }
    // A row as an object, or why it is left out; the losses it had said into `losses`.
    let convert = |row: &gpkg::Row, losses: &mut BTreeSet<String>| -> Result<Made, String> {
        let mut attrs = serde_json::Map::new();
        for ((c, v), w) in layer.columns.iter().zip(&row.values).zip(&wide) {
            if let Some(t) = value_text(v, &c.kind, *w, losses, &c.name) {
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
                return Ok(Made::Skipped(
                    row.fid,
                    format!("geometrisi okunamadı: {why}"),
                ));
            }
            Some(Ok(g)) if g.is_empty() => "record",
            Some(Ok(g)) => match geometry_json(g, losses) {
                Some((kind, fields)) => {
                    e.extend(fields);
                    kind
                }
                None => {
                    return Ok(Made::Skipped(
                        row.fid,
                        format!("{} geometrisinin çizimde karşılığı yok", g.type_name()),
                    ));
                }
            },
        };
        e.insert("kind".into(), J::String(kind.into()));
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
        Ok(Made::Object(uid, entity, row.fid, kind == "record"))
    };
    // The rows split across the machine's threads (docs/adr/0757 §2: a bound table's 50 000 rows of 28 columns took
    // 0,66 s on one), each part's objects in the rows' order and the first refusal in that order.
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get().min(8));
    let parts: Vec<Part> = if threads < 2 || rows.len() < ROWS_SPLIT {
        let mut part_losses = BTreeSet::new();
        vec![
            rows.iter()
                .map(|row| convert(row, &mut part_losses))
                .collect::<Result<Vec<_>, _>>()
                .map(|made| (made, part_losses)),
        ]
    } else {
        let piece = rows.len().div_ceil(threads);
        let convert = &convert;
        std::thread::scope(|scope| {
            let handles: Vec<_> = rows
                .chunks(piece)
                .map(|part| {
                    scope.spawn(move || {
                        let mut part_losses = BTreeSet::new();
                        part.iter()
                            .map(|row| convert(row, &mut part_losses))
                            .collect::<Result<Vec<_>, _>>()
                            .map(|made| (made, part_losses))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
                })
                .collect()
        })
    };
    let mut objects = Vec::with_capacity(rows.len());
    let mut fids = Vec::with_capacity(rows.len());
    let mut skipped = Vec::new();
    let mut records = 0;
    for part in parts {
        let (made, part_losses) = part?;
        losses.extend(part_losses);
        for m in made {
            match m {
                Made::Object(uid, entity, fid, record) => {
                    records += u32::from(record);
                    objects.push((uid, entity));
                    fids.push(fid);
                }
                Made::Skipped(fid, why) => skipped.push((fid, why)),
            }
        }
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
        indexes: Vec::new(),
    })
}

/// A table's indexes as the layer's (docs/adr/0758 §5): an index over columns that are all its fields (not the fid,
/// not the geometry, none holding bytes), named without the table's name before it; a UNIQUE constraint's by its
/// columns. A name met again gets a number after it; the same columns twice are one index, unique when either was.
pub(crate) fn layer_indexes_of(
    table: &str,
    fields: &[LayerField],
    read: &[gpkg::indexes::Index],
) -> Vec<kentos_commands::LayerIndex> {
    let most = kentos_commands::MAX_NAME;
    let mut out: Vec<kentos_commands::LayerIndex> = Vec::new();
    for x in read {
        let Some(keys) = x
            .columns
            .iter()
            .map(|c| {
                fields
                    .iter()
                    .find(|f| f.name == *c && f.kind != LayerFieldKind::Binary)
                    .map(|f| f.key().to_owned())
            })
            .collect::<Option<Vec<String>>>()
        else {
            continue;
        };
        if keys.is_empty() || keys.len() > kentos_commands::MOST_INDEX_FIELDS {
            continue;
        }
        if let Some(same) = out.iter_mut().find(|o| o.fields == keys) {
            same.unique |= x.unique;
            continue;
        }
        let prefix = format!("{table}_");
        let given = if x.constraint {
            x.columns.join(", ")
        } else {
            x.name
                .get(..prefix.len())
                .filter(|p| p.eq_ignore_ascii_case(&prefix) && x.name.len() > prefix.len())
                .map_or(x.name.clone(), |_| x.name[prefix.len()..].to_owned())
        };
        let clean: String = given.chars().filter(|c| !c.is_control()).collect();
        let mut base: String = kentos_commands::js_trim(&clean)
            .chars()
            .take(most)
            .collect();
        base = kentos_commands::js_trim(&base).to_owned();
        if base.is_empty() {
            base = "dizin".into();
        }
        let mut name = base.clone();
        let mut n = 1;
        while out
            .iter()
            .any(|o| kentos_commands::fold(&o.name) == kentos_commands::fold(&name))
        {
            n += 1;
            let tail = format!(" {n}");
            let head: String = base.chars().take(most - tail.chars().count()).collect();
            name = format!("{}{tail}", kentos_commands::js_trim(&head));
        }
        out.push(kentos_commands::LayerIndex {
            name,
            fields: keys,
            unique: x.unique,
            nulls_equal: false,
        });
        if out.len() == kentos_commands::MOST_LAYER_INDEXES {
            break;
        }
    }
    out
}

// ── The drawing's layers as tables ──────────────────────────────────────────

/// A table written.
#[derive(Clone, Debug, PartialEq)]
pub struct TableWritten {
    pub table: String,
    /// The layer the rows are the objects of; none for a table output (docs/adr/0551 §3).
    pub layer: Option<String>,
    /// The tool's output a table holds; none for a layer's table.
    pub output: Option<String>,
    pub rows: u32,
    pub geometry: Option<String>,
    /// Whether the geometry column holds Z and M: 0 none, 1 every row, 2 some rows.
    pub z: u8,
    pub m: u8,
    /// Each column's attribute and the column it was written to (they differ when the name had to change).
    pub columns: Vec<(String, String)>,
    /// A table of the file's the write took out and wrote again (docs/adr/0549).
    pub replaced: bool,
    /// The layer's indexes written over it (docs/adr/0758 §5).
    pub indexes: u32,
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
    /// The batches the rows went in (docs/adr/0549).
    pub batches: u32,
    /// The relations and the QGIS styles written beside the tables (docs/adr/0627 §6, §7; [`write`] only).
    pub profile: crate::gpkg_profile::ProfileWritten,
    /// What of the layers' indexes the file could not hold as given (docs/adr/0758 §5).
    pub index_notes: Vec<String>,
}

fn c2(p: &kentos_commands::Vec2, z: Option<f64>, m: Option<f64>) -> Coord {
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
    pts: &[kentos_commands::Vec2],
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
    pts: &[kentos_commands::Vec2],
    bulges: Option<&Vec<f64>>,
    zs: &Option<Vec<Option<f64>>>,
    holes: Option<&Vec<kentos_commands::RingGeometry>>,
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
    use kentos_commands::Entity as E;
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

/// An object's geometry as a table's row holds it (a GeoPackage's, a CSV's WKT: docs/adr/0522): none for a record;
/// `Err` the kind's word for an object a table has no geometry for. The flag: curves written as chords.
pub fn geometry_of(e: &Entity) -> Result<Option<(Geometry, bool)>, &'static str> {
    match row_of(e) {
        Row::Geometry(g, chords) => Ok(Some((g, chords))),
        Row::Record => Ok(None),
        Row::Left(word) => Err(word),
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
pub(crate) fn fit(geoms: &mut [Option<Geometry>]) -> Option<String> {
    let mut fams = Families::default();
    for g in geoms.iter().flatten() {
        fams.add(family(g));
    }
    let name = fams.type_name()?;
    for g in geoms.iter_mut().flatten() {
        let old = std::mem::replace(g, Geometry::Point(None));
        *g = conform(old, &name);
    }
    Some(name)
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
        // docs/adr/0629 §4: a big integer is INTEGER (64 bits), a time stamp DATETIME in UTC, binary a BLOB; a UUID and
        // JSON are text, KentOS's document of the table says what they are.
        LayerFieldKind::BigInt => ColumnKind::Integer,
        LayerFieldKind::Instant => ColumnKind::DateTime,
        LayerFieldKind::Uuid => ColumnKind::Text(Some(36)),
        LayerFieldKind::Json => ColumnKind::Text(None),
        LayerFieldKind::Binary => ColumnKind::Blob(None),
    };
    gpkg::Column {
        name: f.name.clone(),
        decl: String::new(),
        kind,
        not_null: false,
        default: None,
    }
}

/// A time stamp's text in UTC (`…SS[.k…]Z`) with its fraction's digits filled to three, six or nine.
fn in_thousandths(t: &str) -> String {
    let Some((head, frac)) = t.strip_suffix('Z').and_then(|b| b.split_once('.')) else {
        return t.to_owned();
    };
    let width = frac.len().div_ceil(3) * 3;
    format!("{head}.{frac:0<width$}Z")
}

pub(crate) fn cell(text: Option<&String>, kind: &ColumnKind) -> Value {
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
        // A time stamp in UTC: the same moment (docs/adr/0629 §4), its fraction in milliseconds as the GeoPackage
        // standard writes them (finer digits in threes).
        ColumnKind::DateTime => Value::Text(instant_utc(t).map_or_else(|| t.clone(), |u| in_thousandths(&u))),
        ColumnKind::Blob(_) => binary_bytes(t).map_or_else(|| Value::Text(t.clone()), Value::Blob),
        _ => Value::Text(t.clone()),
    }
}

/// The project's coordinate system as a GeoPackage's (its EPSG code and WKT); the undefined cartesian one for a local
/// project.
pub(crate) fn srs_of(doc: &Document) -> gpkg::Srs {
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

/// How many rows a write holds at once (docs/adr/0549 §2): the objects become rows, the rows go into the file and
/// are let go, a batch at a time.
pub const BATCH_ROWS: usize = 5_000;

/// What a write does with a table the file already has (docs/adr/0549 §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clash {
    /// Refuse: nothing is written.
    Refuse,
    /// The feature or attribute table of that name is taken out of the file and written again; no other table is
    /// touched.
    Replace,
}

/// What a write asks of its host between batches: whether to stop, and where it has come to (rows done of rows).
pub struct Flow<'a> {
    pub stopped: &'a dyn Fn() -> bool,
    pub progress: &'a dyn Fn(u64, u64),
}

fn never() -> bool {
    false
}

fn ignore(_: u64, _: u64) {}

impl Flow<'static> {
    /// A write nothing stops and nothing watches.
    pub fn none() -> Flow<'static> {
        Flow {
            stopped: &never,
            progress: &ignore,
        }
    }
}

/// Why a write stopped without finishing: nothing of it is in the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteStop {
    /// The host asked to stop between two batches.
    Cancelled,
    /// A name the file has already: with `replaceable` a feature or attribute table (`Clash::Replace` takes it out),
    /// otherwise a table of another kind (a tile table, an extension's, an unregistered one) no write replaces.
    Taken { table: String, replaceable: bool },
    /// The file's spatial reference system of the id is another one than the drawing's.
    Srs(String),
    /// SQLite, or the file, said no.
    Failed(String),
}

impl From<gpkg::Error> for WriteStop {
    fn from(e: gpkg::Error) -> WriteStop {
        WriteStop::Failed(e.0)
    }
}

/// The geometry families a table's rows hold, enough to say its column's type.
#[derive(Default)]
struct Families {
    first: Option<u8>,
    mixed: bool,
    multi: bool,
    curved: bool,
}

impl Families {
    fn add(&mut self, f: (u8, bool, bool)) {
        match self.first {
            None => self.first = Some(f.0),
            Some(first) if first != f.0 => self.mixed = true,
            _ => {}
        }
        self.multi |= f.1;
        self.curved |= f.2;
    }

    /// The column's type: none for a table without geometries.
    fn type_name(&self) -> Option<String> {
        let first = self.first?;
        if self.mixed {
            return Some("GEOMETRY".into());
        }
        Some(
            match (first, self.multi, self.curved) {
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
            }
            .into(),
        )
    }
}

/// A geometry made to fit a column of type `name` (a single made multi where the column is, an arc's line a compound
/// curve): what [`fit`] does to each.
fn conform(g: Geometry, name: &str) -> Geometry {
    match (name, g) {
        ("MULTIPOINT", Geometry::Point(Some(p))) => Geometry::MultiPoint(vec![p]),
        ("MULTILINESTRING" | "MULTICURVE", Geometry::Curve(c)) => Geometry::MultiCurve(vec![c]),
        ("MULTIPOLYGON" | "MULTISURFACE", Geometry::Polygon(r)) => Geometry::MultiPolygon(vec![r]),
        ("COMPOUNDCURVE", Geometry::Curve(c @ (Curve::Line(_) | Curve::Arc(_)))) => {
            Geometry::Curve(Curve::Compound(vec![c]))
        }
        (_, other) => other,
    }
}

/// A table a tool made (a summary, a report: `{ columns, rows }`), to be written as an attribute table of text columns
/// (docs/adr/0551 §3): the cells are the text the tool shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextTable {
    /// The tool's output the table is.
    pub output: String,
    /// Its name; the table is called so (made one the file's names do not already hold).
    pub label: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// A text table's table, worked out before any of it is written.
struct TextPlan {
    output: String,
    table: String,
    columns: Vec<gpkg::Column>,
    /// The header each column reads.
    headers: Vec<String>,
    rows: usize,
    replaces: bool,
}

/// One layer's table, worked out before any of it is written.
struct Plan {
    layer: String,
    table: String,
    type_name: Option<String>,
    z: u8,
    m: u8,
    columns: Vec<gpkg::Column>,
    /// The attribute each column reads.
    keys: Vec<String>,
    rows: u32,
    replaces: bool,
}

/// The layer's indexes over the table just written (docs/adr/0758 §5): each over the columns its fields went to, how
/// many; what the file cannot hold as given said in `notes` (a unique one the rows break made plain, empty values taken
/// as the same, which SQLite does not do).
fn write_indexes(
    doc: &Document,
    p: &Plan,
    session: &gpkg::Session<'_>,
    notes: &mut Vec<String>,
) -> Result<u32, WriteStop> {
    let Some(node) = doc.layers().get(&p.layer) else {
        return Ok(0);
    };
    let mut made = 0;
    for x in &node.indexes {
        let columns: Option<Vec<String>> = x
            .fields
            .iter()
            .map(|k| {
                let f = node.fields.iter().find(|f| f.key() == k)?;
                let i = p.keys.iter().position(|a| *a == f.name)?;
                Some(p.columns[i].name.clone())
            })
            .collect();
        let Some(columns) = columns else {
            notes.push(format!(
                "“{}” tablosunun “{}” dizini yazılmadı: alanlarından biri tabloda sütun değil",
                p.table, x.name
            ));
            continue;
        };
        let written = session.create_index(&p.table, &x.name, &columns, x.unique)?;
        made += 1;
        if x.unique && !written.unique {
            notes.push(format!(
                "“{}” tablosunun “{}” dizini tekil yazılamadı: yazılan değerler dosyada yineleniyor (KentOS boş metni değer saymaz, SQLite sayar); düz dizin yazıldı",
                p.table, x.name
            ));
        } else if x.nulls_equal {
            notes.push(format!(
                "“{}” tablosunun “{}” dizini boşları aynı saymıyor: SQLite boş değerleri hep farklı sayar",
                p.table, x.name
            ));
        }
    }
    Ok(made)
}

/// Writes the layers (by id; every layer with objects when empty) to a new GeoPackage at `path`.
pub fn write(doc: &Document, layers: &[String], path: &Path) -> Result<Written, String> {
    let mut gp = GeoPackage::create(path).map_err(|e| e.0)?;
    let session = gp.session().map_err(|e| e.0)?;
    let out =
        write_in(doc, layers, &session, None, BATCH_ROWS, &Flow::none()).map_err(
            |stop| match stop {
                WriteStop::Failed(why) | WriteStop::Srs(why) => why,
                WriteStop::Cancelled => "yazma durduruldu".to_owned(),
                WriteStop::Taken { table, .. } => format!("“{table}” tablosu var"),
            },
        )?;
    session.commit().map_err(|e| e.0)?;
    let mut out = out;
    // The project's relations between the tables and the layers' looks as QGIS's default styles (docs/adr/0627).
    out.profile = crate::gpkg_profile::write_profile(
        doc,
        &out.tables,
        &mut gp,
        &crate::gis_derive::utc_now(),
    )?;
    gp.close().map_err(|e| e.0)?;
    Ok(out)
}

/// The layers' tables written in `session`, a batch of rows at a time (docs/adr/0549): its tables, rows, spatial
/// indexes and lasting ids, not yet committed. `clash` is how a table the file already has is met (none: the file is
/// new, and names that meet inside the write get `_2`, `_3` …); `batch` the rows held at once; `flow` is asked before
/// each batch and told how far it has come. Stopped or failed, the session is left for its owner to roll back.
pub fn write_in(
    doc: &Document,
    layers: &[String],
    session: &gpkg::Session<'_>,
    clash: Option<Clash>,
    batch: usize,
    flow: &Flow<'_>,
) -> Result<Written, WriteStop> {
    write_with_tables(doc, layers, &[], session, clash, batch, flow)
}

/// [`write_in`] with a tool's table outputs written too, each as an attribute table after the layers' (docs/adr/0551 §3).
pub fn write_with_tables(
    doc: &Document,
    layers: &[String],
    tables: &[TextTable],
    session: &gpkg::Session<'_>,
    clash: Option<Clash>,
    batch: usize,
    flow: &Flow<'_>,
) -> Result<Written, WriteStop> {
    let srs = srs_of(doc);
    if let Some(have) = session.srs(srs.id)?
        && (have.organization != srs.organization || have.organization_id != srs.organization_id)
    {
        return Err(WriteStop::Srs(format!(
            "Dosyada {} kimlikli koordinat sistemi {}:{}, çizimin sistemi {}:{}; farklı sistemin tablosu bu dosyaya eklenmez.",
            srs.id, have.organization, have.organization_id, srs.organization, srs.organization_id
        )));
    }
    session.add_srs(&srs)?;
    let mut out = Written::default();
    let (plans, text_plans) = plan(doc, layers, tables, session, clash, &mut out)?;
    let total: u64 = plans.iter().map(|p| u64::from(p.rows)).sum::<u64>()
        + text_plans.iter().map(|p| p.rows as u64).sum::<u64>();
    let mut done = 0u64;
    for p in &plans {
        if (flow.stopped)() {
            return Err(WriteStop::Cancelled);
        }
        if p.replaces {
            session.drop_table(&p.table)?;
        }
        let def = gpkg::TableDef {
            name: p.table.clone(),
            data_type: if p.type_name.is_some() {
                DataType::Features
            } else {
                DataType::Attributes
            },
            identifier: Some(p.table.clone()),
            description: None,
            srs_id: p.type_name.as_ref().map(|_| srs.id),
            geometry: p.type_name.clone().map(|t| gpkg::GeometryColumn {
                name: "geom".into(),
                type_name: t,
                z: p.z,
                m: p.m,
            }),
            columns: p.columns.clone(),
        };
        let mut w = session.start_table(&def)?;
        let mut rows: Vec<gpkg::NewRow> = Vec::with_capacity(batch.min(p.rows as usize));
        let mut uids: Vec<String> = Vec::with_capacity(rows.capacity());
        let mut objects = doc.by_layer_with_uids(&p.layer);
        loop {
            rows.clear();
            uids.clear();
            for (uid, e) in objects.by_ref() {
                let geometry = match row_of(e) {
                    Row::Geometry(g, _) => Some(match &p.type_name {
                        Some(name) => conform(g, name),
                        None => g,
                    }),
                    Row::Record => None,
                    Row::Left(_) => continue,
                };
                rows.push(gpkg::NewRow {
                    fid: None,
                    geometry,
                    values: p
                        .columns
                        .iter()
                        .zip(&p.keys)
                        .map(|(c, k)| cell(e.base().attrs.get(k), &c.kind))
                        .collect(),
                });
                uids.push(uid.to_string());
                if rows.len() >= batch.max(1) {
                    break;
                }
            }
            if rows.is_empty() {
                break;
            }
            if (flow.stopped)() {
                return Err(WriteStop::Cancelled);
            }
            session.append(&mut w, &rows, Some(&uids))?;
            out.batches += 1;
            done += rows.len() as u64;
            (flow.progress)(done, total);
        }
        session.finish_table(w)?;
        let indexes = write_indexes(doc, p, session, &mut out.index_notes)?;
        out.tables.push(TableWritten {
            table: p.table.clone(),
            layer: Some(p.layer.clone()),
            output: None,
            rows: p.rows,
            geometry: p.type_name.clone(),
            z: p.z,
            m: p.m,
            columns: p
                .keys
                .iter()
                .zip(&p.columns)
                .map(|(k, c)| (k.clone(), c.name.clone()))
                .collect(),
            replaced: p.replaces,
            indexes,
        });
    }
    for (p, t) in text_plans.iter().zip(tables) {
        if (flow.stopped)() {
            return Err(WriteStop::Cancelled);
        }
        if p.replaces {
            session.drop_table(&p.table)?;
        }
        let def = gpkg::TableDef {
            name: p.table.clone(),
            data_type: DataType::Attributes,
            identifier: Some(p.table.clone()),
            description: None,
            srs_id: None,
            geometry: None,
            columns: p.columns.clone(),
        };
        let mut w = session.start_table(&def)?;
        for chunk in t.rows.chunks(batch.max(1)) {
            if (flow.stopped)() {
                return Err(WriteStop::Cancelled);
            }
            let rows: Vec<gpkg::NewRow> = chunk
                .iter()
                .map(|r| gpkg::NewRow {
                    fid: None,
                    geometry: None,
                    values: (0..p.columns.len())
                        .map(|i| r.get(i).map_or(Value::Null, |c| Value::Text(c.clone())))
                        .collect(),
                })
                .collect();
            session.append(&mut w, &rows, None)?;
            out.batches += 1;
            done += rows.len() as u64;
            (flow.progress)(done, total);
        }
        session.finish_table(w)?;
        out.tables.push(TableWritten {
            table: p.table.clone(),
            layer: None,
            output: Some(p.output.clone()),
            rows: p.rows as u32,
            geometry: None,
            z: 0,
            m: 0,
            columns: p
                .headers
                .iter()
                .zip(&p.columns)
                .map(|(h, c)| (h.clone(), c.name.clone()))
                .collect(),
            replaced: p.replaces,
            indexes: 0,
        });
    }
    Ok(out)
}

/// The tables the layers make, worked out from one look at their objects: what is left out or written as chords,
/// each table's name, type, Z and M, its columns and the attribute each reads, how many rows; and what the file
/// already has under those names. Nothing is written.
fn plan(
    doc: &Document,
    layers: &[String],
    tables: &[TextTable],
    session: &gpkg::Session<'_>,
    clash: Option<Clash>,
    out: &mut Written,
) -> Result<(Vec<Plan>, Vec<TextPlan>), WriteStop> {
    let ids: Vec<String> = if layers.is_empty() {
        doc.layers().leaves().iter().map(|n| n.id.clone()).collect()
    } else {
        layers.to_vec()
    };
    // The table KentOS keeps lasting ids in is no table of a layer.
    let mut names: BTreeSet<String> = [gpkg::UIDS_EXTENSION.to_owned()].into();
    let mut plans = Vec::new();
    for id in ids {
        let Some(node) = doc.layers().get(&id) else {
            continue;
        };
        let mut fams = Families::default();
        let (mut z, mut m) = (false, false);
        let (mut all_z, mut all_m) = (true, true);
        let mut rows = 0u32;
        let mut extra: BTreeSet<String> = BTreeSet::new();
        for (_, e) in doc.by_layer_with_uids(&id) {
            match row_of(e) {
                Row::Geometry(g, chords) => {
                    if chords {
                        out.approximated += 1;
                    }
                    fams.add(family(&g));
                    let (gz, gm) = g.dims();
                    z |= gz;
                    m |= gm;
                    all_z &= gz;
                    all_m &= gm;
                }
                Row::Record => {}
                Row::Left(kind) => {
                    *out.left.entry(kind.into()).or_default() += 1;
                    continue;
                }
            }
            rows += 1;
            extra.extend(e.base().attrs.keys().cloned());
        }
        if rows == 0 {
            continue;
        }
        let flag = |any: bool, all: bool| -> u8 {
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
        for k in extra {
            add(&k, ColumnKind::Text(None), &mut columns, &mut keys);
        }
        for (c, k) in columns.iter().zip(&keys) {
            if &c.name != k {
                out.renamed
                    .push((node.name.clone(), k.clone(), c.name.clone()));
            }
        }
        let (table, replaces) = table_name(&node.name, "katman", &mut names, session, clash)?;
        plans.push(Plan {
            layer: id.clone(),
            table,
            type_name: fams.type_name(),
            z: flag(z, all_z),
            m: flag(m, all_m),
            columns,
            keys,
            rows,
            replaces,
        });
    }
    let mut text_plans = Vec::new();
    for t in tables {
        let (table, replaces) = table_name(&t.label, "tablo", &mut names, session, clash)?;
        // Each header's column: SQLite's names are one whatever their case, so a repeated header (or `fid`) gets
        // `_2`, `_3` …; an empty one is `sutun_N`.
        let mut taken: BTreeSet<String> = ["fid".to_owned()].into();
        let mut columns = Vec::with_capacity(t.columns.len());
        for (i, h) in t.columns.iter().enumerate() {
            let base = if h.trim().is_empty() {
                format!("sutun_{}", i + 1)
            } else {
                h.trim().to_owned()
            };
            let mut col = base.clone();
            let mut n = 2;
            while !taken.insert(col.to_ascii_lowercase()) {
                col = format!("{base}_{n}");
                n += 1;
            }
            columns.push(gpkg::Column {
                name: col,
                decl: String::new(),
                kind: ColumnKind::Text(None),
                not_null: false,
                default: None,
            });
        }
        text_plans.push(TextPlan {
            output: t.output.clone(),
            table,
            headers: t.columns.clone(),
            columns,
            rows: t.rows.len(),
            replaces,
        });
    }
    Ok((plans, text_plans))
}

/// A table's name: the wanted one (`prefix_` in front when it is empty or reserved), made one the write has not given
/// yet (`_2`, `_3` …), and how the file meets it: free, taken by a feature or attribute table the write replaces, or
/// refused (docs/adr/0549 §1).
fn table_name(
    wanted: &str,
    prefix: &str,
    names: &mut BTreeSet<String>,
    session: &gpkg::Session<'_>,
    clash: Option<Clash>,
) -> Result<(String, bool), WriteStop> {
    let mut table = wanted.trim().to_owned();
    if table.is_empty()
        || table.to_ascii_lowercase().starts_with("gpkg_")
        || table.starts_with("rtree_")
    {
        table = format!("{prefix}_{table}");
    }
    let base = table.clone();
    let mut k = 2;
    while !names.insert(table.to_ascii_lowercase()) {
        table = format!("{base}_{k}");
        k += 1;
    }
    let mut replaces = false;
    if let Some(clash) = clash {
        match session.taken(&table)? {
            gpkg::Taken::Free => {}
            gpkg::Taken::Data(_) if clash == Clash::Replace => replaces = true,
            gpkg::Taken::Data(_) => {
                return Err(WriteStop::Taken {
                    table,
                    replaceable: true,
                });
            }
            gpkg::Taken::Other => {
                return Err(WriteStop::Taken {
                    table,
                    replaceable: false,
                });
            }
        }
    }
    Ok((table, replaces))
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

/// Why a geometry cannot go into a geometry column whose Z and M are prohibited (0), mandatory (1) or optional (2)
/// (docs/adr/0584 §2): elevations or measures where the column takes none, none where it takes them always, or some
/// vertices with and some without (a WKB has one dimension: the rest would be made up or dropped). None when it fits;
/// nothing is dropped or made up on the way.
pub fn dims_problem(g: &Geometry, z: u8, m: u8) -> Option<String> {
    let [(any_z, all_z), (any_m, all_m)] = g.dims_of_all();
    if any_z && z == 0 {
        return Some("nesnenin kotları var, tablonun geometri sütunu Z almaz; kotları silin (Kot ver › Sıfırla) ya da Z'li bir tabloya yazın".into());
    }
    if any_z && !all_z {
        return Some("bazı köşelerin kotu var, bazılarının yok; tabloya ya her köşe kotuyla ya hiçbiri kotsuz yazılır (Kot ver)".into());
    }
    if !any_z && z == 1 {
        return Some("nesnenin kotu yok, tablonun geometri sütunu her köşede Z ister; köşelere kot verin (Kot ver)".into());
    }
    if any_m && m == 0 {
        return Some(
            "nesnenin ölçüleri (M) var, tablonun geometri sütunu M almaz; M'li bir tabloya yazın"
                .into(),
        );
    }
    if any_m && !all_m {
        return Some("bazı köşelerin ölçüsü (M) var, bazılarının yok; rotayı kalibre edin".into());
    }
    if !any_m && m == 1 {
        return Some("nesnenin ölçüsü (M) yok, tablonun geometri sütunu her köşede M ister; rotayı kalibre edin".into());
    }
    None
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

/// A column's field kind, as the import reads it plainly by its type (docs/adr/0629 §4: a blob's binary).
pub fn column_kind(c: &gpkg::Column) -> Option<LayerFieldKind> {
    let reading = if matches!(c.kind, ColumnKind::Blob(_)) {
        Reading::Binary
    } else {
        Reading::Plain
    };
    field_of(c, reading).map(|f| f.kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_columns_literal_default_is_read_as_postgresql_and_sqlite_write_it() {
        // PostgreSQL's pg_get_expr.
        assert_eq!(
            literal_default("'İlçe'::character varying").as_deref(),
            Some("İlçe")
        );
        assert_eq!(literal_default("'{}'::jsonb").as_deref(), Some("{}"));
        assert_eq!(literal_default("false").as_deref(), Some("false"));
        assert_eq!(literal_default("(-1)").as_deref(), Some("-1"));
        assert_eq!(
            literal_default("'2020-01-01'::date").as_deref(),
            Some("2020-01-01")
        );
        assert_eq!(literal_default("'a::b'::text").as_deref(), Some("a::b"));
        assert_eq!(literal_default("NULL::character varying"), None);
        for computed in [
            "now()",
            "uuidv7()",
            "gen_random_uuid()",
            "nextval('ilceler_id_seq'::regclass)",
            "CURRENT_TIMESTAMP",
            "('a' || 'b')",
            "(now() + '1 day'::interval)",
        ] {
            assert_eq!(literal_default(computed), None, "{computed}");
        }
        // SQLite's PRAGMA table_info.
        assert_eq!(literal_default("'Ali''nin'").as_deref(), Some("Ali'nin"));
        assert_eq!(literal_default("7").as_deref(), Some("7"));
        assert_eq!(literal_default("-1.5").as_deref(), Some("-1.5"));
        assert_eq!(literal_default("upper('ilce')"), None);
        assert_eq!(literal_default("NULL"), None);
        // A literal in its field's kind; one that does not read as the kind gives none.
        assert_eq!(
            default_as(LayerFieldKind::Boolean, "0").as_deref(),
            Some("false")
        );
        assert_eq!(
            default_as(LayerFieldKind::Boolean, "1").as_deref(),
            Some("true")
        );
        assert_eq!(
            default_as(LayerFieldKind::Integer, "3").as_deref(),
            Some("3")
        );
        assert_eq!(default_as(LayerFieldKind::Integer, "yok"), None);
        assert_eq!(default_as(LayerFieldKind::Text, ""), None);
    }

    #[test]
    fn a_not_null_column_without_a_default_is_a_required_field() {
        let column =
            |name: &str, kind: ColumnKind, not_null: bool, default: Option<&str>| gpkg::Column {
                name: name.into(),
                decl: String::new(),
                kind,
                not_null,
                default: default.map(str::to_owned),
            };
        let ad = field_of(
            &column("ad", ColumnKind::Text(None), true, None),
            Reading::Plain,
        )
        .expect("a field");
        assert!(ad.required && ad.default.is_none());
        let tur = field_of(
            &column("tur", ColumnKind::Text(None), true, Some("'İlçe'")),
            Reading::Plain,
        )
        .expect("a field");
        assert!(!tur.required);
        assert_eq!(tur.default.as_deref(), Some("İlçe"));
        let made = field_of(
            &column("zaman", ColumnKind::DateTime, true, Some("now()")),
            Reading::Plain,
        )
        .expect("a field");
        assert!(
            !made.required && made.default.is_none(),
            "the database gives it"
        );
        let free = field_of(
            &column("notu", ColumnKind::Text(None), false, None),
            Reading::Plain,
        )
        .expect("a field");
        assert!(!free.required && free.default.is_none());
    }

    fn column(name: &str, kind: ColumnKind, decl: &str) -> gpkg::Column {
        gpkg::Column {
            name: name.into(),
            decl: decl.into(),
            kind,
            not_null: false,
            default: None,
        }
    }

    fn rows(values: &[Value]) -> Vec<gpkg::Row> {
        values
            .iter()
            .enumerate()
            .map(|(i, v)| gpkg::Row {
                fid: i as i64 + 1,
                geometry: None,
                values: vec![v.clone()],
            })
            .collect()
    }

    /// docs/adr/0629 §4: what a column's values and type say of its field.
    #[test]
    fn a_columns_values_say_its_kind() {
        let at = |c: &gpkg::Column, vs: &[Value], hint| {
            let r = reading_of(c, 0, &rows(vs), hint);
            field_of(c, r).map(|f| f.kind)
        };
        let int = column("n", ColumnKind::Integer, "");
        assert_eq!(
            at(&int, &[Value::Integer(7)], None),
            Some(LayerFieldKind::Integer)
        );
        assert_eq!(
            at(&int, &[Value::Integer(1 << 53)], None),
            Some(LayerFieldKind::BigInt)
        );
        assert_eq!(
            at(&int, &[Value::Integer(7)], Some(LayerFieldKind::BigInt)),
            Some(LayerFieldKind::BigInt)
        );
        let when = column("t", ColumnKind::DateTime, "");
        let text = |t: &str| Value::Text(t.into());
        assert_eq!(
            at(&when, &[text("2024-03-01T10:20:30Z")], None),
            Some(LayerFieldKind::DateTime)
        );
        assert_eq!(
            at(&when, &[text("2024-03-01T10:20:30+03:00")], None),
            Some(LayerFieldKind::Instant)
        );
        assert_eq!(
            at(&when, &[text("2024-03-01T10:20:30.5Z")], None),
            Some(LayerFieldKind::Instant)
        );
        // A local time keeps being one: its fraction makes no moment.
        let local = column("t", ColumnKind::DateTime, "timestamp");
        assert_eq!(
            at(&local, &[text("2024-03-01T10:20:30.5")], None),
            Some(LayerFieldKind::DateTime)
        );
        assert_eq!(
            at(
                &column("b", ColumnKind::Blob(None), ""),
                &[Value::Blob(vec![1])],
                None
            ),
            Some(LayerFieldKind::Binary)
        );
        let id = column("u", ColumnKind::Other("uuid".into()), "uuid");
        assert_eq!(
            at(&id, &[text("0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0001")], None),
            Some(LayerFieldKind::Uuid)
        );
        // A value that does not keep the hint's kind leaves the column as text.
        assert_eq!(at(&id, &[text("yok")], None), Some(LayerFieldKind::Text));
        let json = column("j", ColumnKind::Other("jsonb".into()), "jsonb");
        assert_eq!(
            at(&json, &[text("{\"a\": 1}")], None),
            Some(LayerFieldKind::Json)
        );
    }

    /// The import's values of the kinds, and what a cell cannot keep said.
    #[test]
    fn what_a_cell_cannot_keep_is_said() {
        let mut losses = BTreeSet::new();
        let blob = ColumnKind::Blob(None);
        assert_eq!(
            value_text(
                &Value::Blob(b"ABC".to_vec()),
                &blob,
                Reading::Binary,
                &mut losses,
                "b"
            )
            .as_deref(),
            Some("QUJD")
        );
        assert!(losses.is_empty());
        let big = Value::Blob(vec![0; MAX_BINARY + 1]);
        assert_eq!(
            value_text(&big, &blob, Reading::Binary, &mut losses, "b"),
            None
        );
        assert!(
            losses.iter().any(|l| l.contains("1 MiB'ı aşan")),
            "{losses:?}"
        );
        assert_eq!(
            value_text(
                &Value::Real(f64::NAN),
                &ColumnKind::Real,
                Reading::Plain,
                &mut losses,
                "r"
            ),
            None
        );
        assert!(
            losses.iter().any(|l| l.contains("sonlu olmayan")),
            "{losses:?}"
        );
        let when = ColumnKind::DateTime;
        assert_eq!(
            value_text(
                &Value::Text("2024-03-01 10:20:30.500+03".into()),
                &when,
                Reading::Instant,
                &mut losses,
                "t"
            )
            .as_deref(),
            Some("2024-03-01T10:20:30.5+03:00")
        );
        assert_eq!(in_thousandths("2024-03-01T07:20:30.5Z"), "2024-03-01T07:20:30.500Z");
        assert_eq!(in_thousandths("2024-03-01T07:20:30.1234Z"), "2024-03-01T07:20:30.123400Z");
        assert_eq!(in_thousandths("2024-03-01T07:20:30Z"), "2024-03-01T07:20:30Z");
        // No offset in a time stamp column: UTC's.
        assert_eq!(
            value_text(
                &Value::Text("2024-03-01T10:20:30.25".into()),
                &when,
                Reading::Instant,
                &mut losses,
                "t"
            )
            .as_deref(),
            Some("2024-03-01T10:20:30.25Z")
        );
    }

    /// docs/adr/0629 §4: a cell the layer would write is the table's when it means the same to the layer's field.
    #[test]
    fn a_time_stamp_is_the_same_moment() {
        let when = ColumnKind::DateTime;
        let text = |t: &str| Value::Text(t.into());
        let (a, b) = (
            text("2024-03-01T07:20:30.5Z"),
            text("2024-03-01T10:20:30.500+03:00"),
        );
        assert!(same_cell_as(&a, &b, &when, Some(LayerFieldKind::Instant)));
        assert!(!same_cell_as(
            &a,
            &text("2024-03-01T07:20:31Z"),
            &when,
            Some(LayerFieldKind::Instant)
        ));
        let id = ColumnKind::Text(Some(36));
        assert!(same_cell_as(
            &text("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001"),
            &text("0192F5A0-7C3E-7D4A-9B1E-4C2F8A6D0001"),
            &id,
            Some(LayerFieldKind::Uuid)
        ));
        assert!(same_cell_as(
            &text("{\"a\":1}"),
            &text("{ \"a\" : 1 }"),
            &ColumnKind::Text(None),
            Some(LayerFieldKind::Json)
        ));
        let blob = ColumnKind::Blob(None);
        assert!(same_cell_as(
            &Value::Blob(vec![1, 2]),
            &Value::Blob(vec![1, 2]),
            &blob,
            Some(LayerFieldKind::Binary)
        ));
        assert!(!same_cell_as(
            &Value::Blob(vec![1, 2]),
            &Value::Blob(vec![1]),
            &blob,
            Some(LayerFieldKind::Binary)
        ));
    }

    /// docs/adr/0629 §4: KentOS's document says the kinds a GeoPackage's types do not.
    #[test]
    fn the_fields_document_names_the_wide_kinds_only() {
        let fields = vec![
            LayerField::new("Ad", LayerFieldKind::Text),
            LayerField::new("Cihaz", LayerFieldKind::Uuid),
            LayerField::new("Ek", LayerFieldKind::Json),
        ];
        let columns = vec![
            ("Ad".to_owned(), "ad".to_owned()),
            ("Cihaz".to_owned(), "cihaz".to_owned()),
            ("Ek".to_owned(), "ek".to_owned()),
        ];
        assert_eq!(
            field_document(&fields, &columns).as_deref(),
            Some(r#"{"kinds":{"cihaz":"uuid","ek":"json"}}"#)
        );
        assert_eq!(field_document(&fields[..1], &columns), None);
    }
}
