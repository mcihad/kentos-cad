//! The containers of a derived dataset (docs/adr/0634 §3, plan/gis.md CBS-16): a GeoPackage (docs/adr/0530), a zip
//! of Shapefiles and a GeoJSON file, or a zip of GeoJSON files. What `file.export.gis` builds (`gis_derive::build`)
//! each writes; `cad.gis.derivation`, `cad.gis.takeback` and `cad.gis.detach` read them all as the GeoPackage's tables
//! and rows, so one rule checks and takes back whatever the GIS edited.
//!
//! - **GeoPackage**: each table with its lasting ids (`kentos_uids`) and its record in the metadata (ADR 0530 §3).
//! - **Shapefile** (`.zip`): a table is a set of each geometry family it has (`Parsel`, or `Çizim_alan`,
//!   `Çizim_cizgi`, `Çizim_nokta`; a record a row with a null shape), as `file.export.shp` writes them. The archive's
//!   `kentos-turev.json` (`kentos.derivation-set` 1) holds each set's record, its columns by their names in the
//!   table and in the .dbf (cut to ten bytes) with their types, and the rows written without an elevation in a set
//!   that has them (the format writes 0 there).
//! - **GeoJSON** (`.geojson`, one table; `.zip`, several): a FeatureCollection a table, its record, columns and
//!   geometry type its foreign member `kentos:table`; coordinates in the project's system (the `crs` member names
//!   it), Z where the row has it, no M (the format has none: a derived row's M is not compared). A zip holds the
//!   same files and `kentos-turev.json`.
//!
//! A table whose link was broken (`cad.gis.detach`) holds a note in place of its record and is not a derived table.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use kentos_commands::{DerivationRecord, Entity, ShapefileReadOptions};
use kentos_formats::zip;
use kentos_geopackage::{
    self as gpkg, ColumnKind, Coord, Curve, DataType, GeoPackage, Geometry, Value,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as J, json};

use crate::gis_derive::{self, Built};

/// A derivative's archive: its data deflated at the lighter level (a derived dataset's chords are tens of megabytes;
/// docs/adr/0223 §8's rule), the same entries the same bytes.
fn packed(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let refs: Vec<(&str, &[u8], zip::Pack)> = entries
        .iter()
        .map(|(n, d)| (n.as_str(), d.as_slice(), zip::Pack::Fast))
        .collect();
    zip::write_with(&refs)
}

/// The archive's index of its tables.
pub const MANIFEST: &str = "kentos-turev.json";
pub const MANIFEST_FORMAT: &str = "kentos.derivation-set";
/// A GeoJSON derived table's foreign member.
pub const MEMBER: &str = "kentos:table";

/// A refusal: its code and its words.
pub type Refusal = (&'static str, String);

/// Which container a derived dataset is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    GeoPackage,
    Shapefile,
    GeoJson,
    PostGis,
}

/// A table of a derived dataset as the GeoPackage's: its shape, its record, its rows and their lasting ids by fid.
pub struct SetTable {
    pub layer: gpkg::Layer,
    pub record: DerivationRecord,
    pub rows: Vec<gpkg::Row>,
    pub uids: HashMap<i64, String>,
}

/// A derived dataset read: its file's name, size and SHA-256, its derived tables in the file's order.
pub struct DerivedSet {
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
    pub container: Container,
    pub tables: Vec<SetTable>,
}

/// A column of a table in a Shapefile or GeoJSON: its name in the table, its name in the file, its type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InfoColumn {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub kind: String,
}

/// The geometry column's type and dimensions (0 none, 1 every row, 2 some).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InfoGeometry {
    #[serde(rename = "type")]
    pub type_name: String,
    pub z: u8,
    pub m: u8,
}

/// A table of a Shapefile or GeoJSON derivative: what makes it read as the GeoPackage's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableInfo {
    pub table: String,
    /// The file in the archive (`Parsel.shp`, `Parsel.geojson`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<DerivationRecord>,
    /// The note a broken link left (`cad.gis.detach`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detached: Option<J>,
    pub columns: Vec<InfoColumn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<InfoGeometry>,
}

/// The archive's index.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub container: String,
    pub tables: Vec<TableInfo>,
}

// ── Values ──────────────────────────────────────────────────────────────────

/// A column's type as text, read back by `ColumnKind::read`.
pub fn kind_text(kind: &ColumnKind) -> String {
    match kind {
        ColumnKind::Boolean => "BOOLEAN".into(),
        ColumnKind::Integer => "INTEGER".into(),
        ColumnKind::Real => "REAL".into(),
        ColumnKind::Text(None) => "TEXT".into(),
        ColumnKind::Text(Some(n)) => format!("TEXT({n})"),
        ColumnKind::Blob(_) => "BLOB".into(),
        ColumnKind::Date => "DATE".into(),
        ColumnKind::DateTime => "DATETIME".into(),
        ColumnKind::Other(s) => s.clone(),
    }
}

/// A cell as text: none for no value.
fn value_text(v: &Value) -> Option<String> {
    match v {
        Value::Null | Value::Blob(_) => None,
        Value::Integer(i) => Some(i.to_string()),
        Value::Real(r) => Some(format!("{r}")),
        Value::Text(t) => Some(t.clone()),
    }
}

/// A cell read from a file's text by its column's type.
fn text_value(t: Option<&str>, kind: &ColumnKind) -> Value {
    let Some(t) = t else {
        return Value::Null;
    };
    match kind {
        ColumnKind::Integer | ColumnKind::Boolean => match t.trim().parse::<i64>() {
            Ok(i) => Value::Integer(i),
            Err(_) => match t.trim().parse::<f64>() {
                Ok(r) if r.fract() == 0.0 && r.abs() < 9.0e15 => Value::Integer(r as i64),
                _ => match t.trim() {
                    "true" | "T" | "t" | "Y" | "y" => Value::Integer(1),
                    "false" | "F" | "f" | "N" | "n" => Value::Integer(0),
                    _ => Value::Text(t.to_owned()),
                },
            },
        },
        ColumnKind::Real => t
            .trim()
            .parse::<f64>()
            .map_or_else(|_| Value::Text(t.to_owned()), Value::Real),
        _ => Value::Text(t.to_owned()),
    }
}

fn from_json(v: &J, kind: &ColumnKind) -> Value {
    match v {
        J::Null => Value::Null,
        J::Bool(b) => Value::Integer(i64::from(*b)),
        J::Number(n) => match kind {
            ColumnKind::Integer | ColumnKind::Boolean => n
                .as_i64()
                .map(Value::Integer)
                .unwrap_or_else(|| n.as_f64().map_or(Value::Null, Value::Real)),
            ColumnKind::Real => n.as_f64().map_or(Value::Null, Value::Real),
            _ => Value::Text(n.to_string()),
        },
        J::String(s) => text_value(Some(s), kind),
        other => Value::Text(other.to_string()),
    }
}

// ── Geometry as GeoJSON ─────────────────────────────────────────────────────

fn coords_of(c: &Curve) -> Vec<Coord> {
    match c {
        Curve::Line(p) | Curve::Arc(p) => p.clone(),
        Curve::Compound(parts) => parts.iter().flat_map(coords_of).collect(),
    }
}

fn coord_of(v: &J) -> Option<Coord> {
    let a = v.as_array()?;
    Some(Coord {
        x: a.first()?.as_f64()?,
        y: a.get(1)?.as_f64()?,
        z: a.get(2).and_then(J::as_f64),
        m: None,
    })
}

fn line_of(v: &J) -> Option<Curve> {
    Some(Curve::Line(
        v.as_array()?.iter().map(coord_of).collect::<Option<_>>()?,
    ))
}

fn rings_of(v: &J) -> Option<Vec<Curve>> {
    v.as_array()?.iter().map(line_of).collect()
}

/// GeoJSON's geometry object as a geometry; none when it is not one.
pub fn geometry_from_json(v: &J) -> Option<Geometry> {
    let c = &v["coordinates"];
    Some(match v["type"].as_str()? {
        "Point" => Geometry::Point(Some(coord_of(c)?)),
        "MultiPoint" => {
            Geometry::MultiPoint(c.as_array()?.iter().map(coord_of).collect::<Option<_>>()?)
        }
        "LineString" => Geometry::Curve(line_of(c)?),
        "MultiLineString" => {
            Geometry::MultiCurve(c.as_array()?.iter().map(line_of).collect::<Option<_>>()?)
        }
        "Polygon" => Geometry::Polygon(rings_of(c)?),
        "MultiPolygon" => {
            Geometry::MultiPolygon(c.as_array()?.iter().map(rings_of).collect::<Option<_>>()?)
        }
        "GeometryCollection" => Geometry::Collection(
            v["geometries"]
                .as_array()?
                .iter()
                .map(geometry_from_json)
                .collect::<Option<_>>()?,
        ),
        _ => return None,
    })
}

// ── Writing ─────────────────────────────────────────────────────────────────

fn info_of(t: &gis_derive::BuiltTable) -> TableInfo {
    TableInfo {
        table: t.def.name.clone(),
        file: None,
        record: Some(t.record.clone()),
        detached: None,
        columns: t
            .def
            .columns
            .iter()
            .map(|c| InfoColumn {
                name: c.name.clone(),
                file: None,
                kind: kind_text(&c.kind),
            })
            .collect(),
        geometry: t.def.geometry.as_ref().map(|g| InfoGeometry {
            type_name: g.type_name.clone(),
            z: g.z,
            m: g.m,
        }),
    }
}

/// A table as a GeoJSON FeatureCollection's text, its record and shape in `kentos:table`: written as it goes, a number
/// as its shortest text that reads back the same (a derived dataset's chords are millions of positions).
fn feature_collection(t: &gis_derive::BuiltTable, srid: u32) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut info = info_of(t);
    // GeoJSON has no M.
    if let Some(g) = info.geometry.as_mut() {
        g.m = 0;
    }
    let text = |v: &str| serde_json::to_string(v).unwrap_or_else(|_| "\"\"".into());
    let mut out: Vec<u8> = Vec::with_capacity(t.rows.len() * 256);
    let e = |e: std::io::Error| e.to_string();
    write!(
        out,
        "{{\"type\":\"FeatureCollection\",\"name\":{}",
        text(&t.def.name)
    )
    .map_err(e)?;
    if srid > 0 {
        write!(
            out,
            ",\"crs\":{{\"type\":\"name\",\"properties\":{{\"name\":\"urn:ogc:def:crs:EPSG::{srid}\"}}}}"
        )
        .map_err(e)?;
    }
    write!(out, ",\"{MEMBER}\":").map_err(e)?;
    serde_json::to_writer(&mut out, &info).map_err(|err| err.to_string())?;
    out.extend_from_slice(b",\"features\":[");
    let names: Vec<String> = t.def.columns.iter().map(|c| text(&c.name)).collect();
    for (k, r) in t.rows.iter().enumerate() {
        if k > 0 {
            out.push(b',');
        }
        out.extend_from_slice(b"{\"type\":\"Feature\",\"properties\":{");
        for (n, (name, v)) in names.iter().zip(&r.values).enumerate() {
            if n > 0 {
                out.push(b',');
            }
            out.extend_from_slice(name.as_bytes());
            out.push(b':');
            match v {
                Value::Integer(i) => write!(out, "{i}").map_err(e)?,
                Value::Real(x) if x.is_finite() => write_number(&mut out, *x),
                Value::Text(s) => out.extend_from_slice(text(s).as_bytes()),
                _ => out.extend_from_slice(b"null"),
            }
        }
        out.extend_from_slice(b"},\"geometry\":");
        match &r.geometry {
            Some(g) => write_geometry(&mut out, g),
            None => out.extend_from_slice(b"null"),
        }
        out.push(b'}');
    }
    out.extend_from_slice(b"]}");
    Ok(out)
}

/// A number as JSON's shortest text that reads back the same (serde_json's ryu).
fn write_number(out: &mut Vec<u8>, x: f64) {
    if serde_json::to_writer(&mut *out, &x).is_err() {
        out.extend_from_slice(b"null");
    }
}

fn write_position(out: &mut Vec<u8>, c: &Coord) {
    out.push(b'[');
    write_number(out, c.x);
    out.push(b',');
    write_number(out, c.y);
    if let Some(z) = c.z.filter(|z| z.is_finite()) {
        out.push(b',');
        write_number(out, z);
    }
    out.push(b']');
}

fn write_positions(out: &mut Vec<u8>, pts: &[Coord]) {
    out.push(b'[');
    for (i, c) in pts.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        write_position(out, c);
    }
    out.push(b']');
}

fn write_rings(out: &mut Vec<u8>, rings: &[Curve]) {
    out.push(b'[');
    for (i, r) in rings.iter().enumerate() {
        if i > 0 {
            out.push(b',');
        }
        write_positions(out, &coords_of(r));
    }
    out.push(b']');
}

/// A geometry as GeoJSON's object's text.
fn write_geometry(out: &mut Vec<u8>, g: &Geometry) {
    fn kind(out: &mut Vec<u8>, name: &str) {
        out.extend_from_slice(b"{\"type\":\"");
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(b"\",\"coordinates\":");
    }
    match g {
        Geometry::Point(None) => {
            out.extend_from_slice(b"null");
            return;
        }
        Geometry::Point(Some(c)) => {
            kind(out, "Point");
            write_position(out, c);
        }
        Geometry::MultiPoint(p) => {
            kind(out, "MultiPoint");
            write_positions(out, p);
        }
        Geometry::Curve(c) => {
            kind(out, "LineString");
            write_positions(out, &coords_of(c));
        }
        Geometry::MultiCurve(c) => {
            kind(out, "MultiLineString");
            write_rings(out, c);
        }
        Geometry::Polygon(r) => {
            kind(out, "Polygon");
            write_rings(out, r);
        }
        Geometry::MultiPolygon(p) => {
            kind(out, "MultiPolygon");
            out.push(b'[');
            for (i, r) in p.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_rings(out, r);
            }
            out.push(b']');
        }
        Geometry::Collection(all) => {
            out.extend_from_slice(b"{\"type\":\"GeometryCollection\",\"geometries\":[");
            for (i, g) in all.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_geometry(out, g);
            }
            out.extend_from_slice(b"]}");
            return;
        }
    }
    out.push(b'}');
}

/// A file name of a table: the characters no file system takes as `_`.
fn file_stem(name: &str) -> String {
    let s: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    if s.is_empty() { "tablo".into() } else { s }
}

fn manifest_bytes(container: &str, tables: Vec<TableInfo>) -> Vec<u8> {
    let m = Manifest {
        format: MANIFEST_FORMAT.into(),
        version: 1,
        container: container.into(),
        tables,
    };
    serde_json::to_vec_pretty(&m).unwrap_or_default()
}

/// The built dataset as GeoJSON: one table a `.geojson` file's bytes (`zip` false), several a zip of them with the
/// index (`zip` true).
pub fn write_geojson(built: &Built, srid: u32) -> Result<(Vec<u8>, bool), String> {
    if let [one] = built.tables.as_slice() {
        return Ok((feature_collection(one, srid)?, false));
    }
    let mut names: BTreeSet<String> = BTreeSet::new();
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut infos = Vec::new();
    for t in &built.tables {
        let mut name = format!("{}.geojson", file_stem(&t.def.name));
        let mut k = 2;
        while !names.insert(name.to_lowercase()) {
            name = format!("{}_{k}.geojson", file_stem(&t.def.name));
            k += 1;
        }
        let fc = feature_collection(t, srid)?;
        let mut info = info_of(t);
        if let Some(g) = info.geometry.as_mut() {
            g.m = 0;
        }
        info.file = Some(name.clone());
        infos.push(info);
        entries.push((name, fc));
    }
    entries.push((MANIFEST.into(), manifest_bytes("geojson", infos)));
    Ok((packed(&entries)?, true))
}

/// A geometry family as the Shapefile writer sorts them: point, line, area.
fn family(g: &Geometry) -> u8 {
    match g {
        Geometry::Point(_) | Geometry::MultiPoint(_) => 0,
        Geometry::Curve(_) | Geometry::MultiCurve(_) => 1,
        _ => 2,
    }
}

/// A row as an object the Shapefile writer takes: its geometry (straight; the derivation made it so) and its cells as
/// attributes by column name, built as the contract's types (no JSON between: a derived dataset's chords are millions
/// of vertices).
fn entity_of(geometry: Option<&Geometry>, attrs: BTreeMap<String, String>) -> Option<Entity> {
    use kentos_commands::{
        AreaPart, EntityBase, PathEntity, PointEntity, PointPart, RecordEntity, RingGeometry, Vec2,
    };
    let base = EntityBase {
        attrs,
        ..serde_json::from_value(json!({ "id": 0, "layerId": "t", "attrs": {} })).ok()?
    };
    let xy = |c: &Coord| Vec2 { x: c.x, y: c.y };
    let fin = |v: Option<f64>| v.filter(|x| x.is_finite());
    let some = |v: Vec<Option<f64>>| v.iter().any(Option::is_some).then_some(v);
    let zs = |c: &[Coord]| some(c.iter().map(|c| fin(c.z)).collect());
    let ms = |c: &[Coord]| some(c.iter().map(|c| fin(c.m)).collect());
    let ring = |c: &Curve| {
        let mut p = coords_of(c);
        if p.len() > 1 && (p[0].x, p[0].y) == (p[p.len() - 1].x, p[p.len() - 1].y) {
            p.pop();
        }
        p
    };
    let holes = |r: &[Curve]| {
        let h: Vec<RingGeometry> = r[1.min(r.len())..]
            .iter()
            .map(|c| {
                let p = ring(c);
                RingGeometry {
                    pts: p.iter().map(xy).collect(),
                    bulges: None,
                    zs: zs(&p),
                }
            })
            .collect();
        (!h.is_empty()).then_some(h)
    };
    let part = |pts: &[Coord], holes: Option<Vec<RingGeometry>>, m: bool| AreaPart {
        pts: pts.iter().map(xy).collect(),
        bulges: None,
        holes,
        zs: zs(pts),
        ms: if m { ms(pts) } else { None },
    };
    let path = |first: AreaPart, rest: Vec<AreaPart>| PathEntity {
        base: base.clone(),
        pts: first.pts,
        bulges: None,
        holes: first.holes,
        zs: first.zs,
        ms: first.ms,
        parts: (!rest.is_empty()).then_some(rest),
        closed: false,
        widths: None,
    };
    let point = |all: &[Coord]| {
        let first = all.first()?;
        let parts: Vec<PointPart> = all[1..]
            .iter()
            .map(|c| PointPart {
                p: xy(c),
                z: fin(c.z),
            })
            .collect();
        Some(Entity::Point(PointEntity {
            base: base.clone(),
            p: xy(first),
            z: fin(first.z),
            parts: (!parts.is_empty()).then_some(parts),
        }))
    };
    Some(match geometry {
        None => Entity::Record(RecordEntity { base: base.clone() }),
        Some(Geometry::Point(Some(c))) => point(std::slice::from_ref(c))?,
        Some(Geometry::MultiPoint(all)) => point(all)?,
        Some(Geometry::Curve(c)) => {
            Entity::Polyline(path(part(&coords_of(c), None, true), Vec::new()))
        }
        Some(Geometry::MultiCurve(all)) if !all.is_empty() => {
            let rest = all[1..]
                .iter()
                .map(|c| part(&coords_of(c), None, true))
                .collect();
            Entity::Polyline(path(part(&coords_of(&all[0]), None, true), rest))
        }
        Some(Geometry::Polygon(r)) if !r.is_empty() => {
            Entity::Polygon(path(part(&ring(&r[0]), holes(r), false), Vec::new()))
        }
        Some(Geometry::MultiPolygon(all)) if !all.is_empty() && !all[0].is_empty() => {
            let rest = all[1..]
                .iter()
                .filter(|r| !r.is_empty())
                .map(|r| part(&ring(&r[0]), holes(r), false))
                .collect();
            Entity::Polygon(path(part(&ring(&all[0][0]), holes(&all[0]), false), rest))
        }
        _ => return None,
    })
}

/// A column's field for the Shapefile writer: its type.
fn field_of(c: &gpkg::Column) -> Option<kentos_commands::LayerField> {
    let kind = match c.kind {
        ColumnKind::Integer => "integer",
        ColumnKind::Real => "decimal",
        ColumnKind::Boolean => "boolean",
        ColumnKind::Date => "date",
        ColumnKind::DateTime => "datetime",
        _ => "text",
    };
    serde_json::from_value(json!({ "name": c.name, "kind": kind })).ok()
}

/// The built dataset as a zip of Shapefiles and the index: each table a set of each geometry family it has.
pub fn write_shapefile(built: &Built, prj: Option<String>) -> Result<Vec<u8>, String> {
    use kentos_formats::shp::write as shp;
    let mut objects: Vec<Vec<Entity>> = Vec::new();
    let mut fields: Vec<Vec<kentos_commands::LayerField>> = Vec::new();
    for t in &built.tables {
        let mut list = Vec::with_capacity(t.rows.len());
        for r in &t.rows {
            let attrs: BTreeMap<String, String> = t
                .def
                .columns
                .iter()
                .zip(&r.values)
                .filter_map(|(c, v)| value_text(v).map(|text| (c.name.clone(), text)))
                .collect();
            list.push(entity_of(r.geometry.as_ref(), attrs).ok_or_else(|| {
                format!(
                    "“{}” tablosunun bir satırı Shapefile'a yazılamadı",
                    t.def.name
                )
            })?);
        }
        objects.push(list);
        fields.push(t.def.columns.iter().filter_map(field_of).collect());
    }
    let layers: Vec<shp::Layer<'_>> = built
        .tables
        .iter()
        .enumerate()
        .map(|(i, t)| shp::Layer {
            id: &t.def.name,
            name: &t.def.name,
            fields: &fields[i],
            objects: objects[i].iter().collect(),
        })
        .collect();
    let written = shp::write(&shp::Input {
        layers,
        blocks: &[],
        prj,
        encoding: shp::DbfEncoding::Utf8,
    })?;
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut infos = Vec::new();
    for set in &written.sets {
        let Some(t) = built.tables.iter().find(|t| t.def.name == set.layer) else {
            continue;
        };
        // The set's rows: those of its family, a record in the first family's set (as the writer sorts them).
        let families: BTreeSet<u8> = t
            .rows
            .iter()
            .filter_map(|r| r.geometry.as_ref())
            .map(family)
            .collect();
        let first = families.iter().next().copied();
        // Whether a shape type is a family's (the writer's names: POINT, MULTIPOINT, POLYLINE, POLYGON with Z or M).
        let of = |f: u8, shape: &str| match f {
            0 => shape.starts_with("POINT") || shape.starts_with("MULTIPOINT"),
            1 => shape.starts_with("POLYLINE"),
            _ => shape.starts_with("POLYGON"),
        };
        let mine: Vec<usize> = (0..t.rows.len())
            .filter(|&i| match (&t.rows[i].geometry, set.shape) {
                (Some(g), Some(shape)) => of(family(g), shape),
                (None, Some(shape)) => first.is_some_and(|f| of(f, shape)),
                (None, None) => true,
                (Some(_), None) => false,
            })
            .collect();
        let set_z = set.shape.is_some_and(|s| s.ends_with('Z'));
        let renamed: HashMap<&str, &str> = written
            .renamed
            .iter()
            .filter(|r| r.set == set.name)
            .map(|r| (r.attribute.as_str(), r.column.as_str()))
            .collect();
        let mut record = t.record.clone();
        record.table = set.name.clone();
        record.rows = set.rows;
        let geometry = set.shape.map(|_| {
            let rows: Vec<&Geometry> = mine
                .iter()
                .filter_map(|&i| t.rows[i].geometry.as_ref())
                .collect();
            let type_name = match rows.first().map(|g| family(g)) {
                Some(0) if rows.iter().any(|g| matches!(g, Geometry::MultiPoint(_))) => {
                    "MULTIPOINT"
                }
                Some(0) => "POINT",
                Some(1) if rows.iter().any(|g| matches!(g, Geometry::MultiCurve(_))) => {
                    "MULTILINESTRING"
                }
                Some(1) => "LINESTRING",
                _ if rows.iter().any(|g| matches!(g, Geometry::MultiPolygon(_))) => "MULTIPOLYGON",
                _ => "POLYGON",
            };
            // A Z type holds measures too; a row without one is “no data” there.
            let measured = rows.iter().any(|g| g.dims().1);
            InfoGeometry {
                type_name: type_name.into(),
                // A vertex without an elevation is 0 in a Shapefile's Z type.
                z: if set_z { gis_derive::Z_AS_ZERO } else { 0 },
                m: if measured { 2 } else { 0 },
            }
        });
        infos.push(TableInfo {
            table: set.name.clone(),
            file: Some(format!(
                "{}.{}",
                set.name,
                if set.shape.is_some() { "shp" } else { "dbf" }
            )),
            record: Some(record),
            detached: None,
            columns: t
                .def
                .columns
                .iter()
                .map(|c| InfoColumn {
                    name: c.name.clone(),
                    file: renamed.get(c.name.as_str()).map(|f| (*f).to_owned()),
                    kind: kind_text(&c.kind),
                })
                .collect(),
            geometry,
        });
        for (ext, bytes) in &set.files {
            entries.push((format!("{}.{ext}", set.name), bytes.clone()));
        }
    }
    entries.push((MANIFEST.into(), manifest_bytes("shapefile", infos)));
    packed(&entries)
}

// ── Reading ─────────────────────────────────────────────────────────────────

fn unreadable(path: &Path, why: impl std::fmt::Display) -> Refusal {
    (
        "import_unreadable",
        format!("{} okunamadı: {why}", path.display()),
    )
}

fn not_derived(path: &Path) -> Refusal {
    (
        "not_a_derivation",
        format!(
            "{} dosyasında KentOS'un türetim kaydı yok; file.export.gis'in yazdığı bir dosya değil ya da bütün tablolarının bağı koparılmış.",
            path.display()
        ),
    )
}

/// The layer a table's info stands for.
fn layer_of(info: &TableInfo) -> gpkg::Layer {
    gpkg::Layer {
        name: info.table.clone(),
        data_type: if info.geometry.is_some() {
            DataType::Features
        } else {
            DataType::Attributes
        },
        identifier: Some(info.table.clone()),
        description: None,
        bounds: None,
        srs_id: None,
        geometry: info.geometry.as_ref().map(|g| gpkg::GeometryColumn {
            name: "geom".into(),
            type_name: g.type_name.clone(),
            z: g.z,
            m: g.m,
        }),
        fid: "fid".into(),
        columns: info
            .columns
            .iter()
            .map(|c| gpkg::Column {
                name: c.name.clone(),
                decl: c.kind.clone(),
                kind: ColumnKind::read(&c.kind),
                not_null: false,
                default: None,
            })
            .collect(),
        indexed: false,
    }
}

fn uids_of(layer: &gpkg::Layer, rows: &[gpkg::Row]) -> HashMap<i64, String> {
    let col = gis_derive::column_of(layer, "kentos_uid");
    rows.iter()
        .filter_map(|r| gis_derive::text_at(r, col).map(|u| (r.fid, u)))
        .collect()
}

/// A GeoJSON FeatureCollection's table, when it is a derived one.
fn geojson_table(fc: &J, info: Option<TableInfo>) -> Option<SetTable> {
    let info =
        info.or_else(|| serde_json::from_value::<TableInfo>(fc.get(MEMBER)?.clone()).ok())?;
    let record = info.record.clone().filter(|r| r.problem().is_none())?;
    let layer = layer_of(&info);
    let rows: Vec<gpkg::Row> = fc["features"]
        .as_array()
        .map(|all| {
            all.iter()
                .enumerate()
                .map(|(i, f)| gpkg::Row {
                    fid: i as i64 + 1,
                    geometry: geometry_from_json(&f["geometry"]).map(Ok),
                    values: layer
                        .columns
                        .iter()
                        .map(|c| from_json(&f["properties"][c.name.as_str()], &c.kind))
                        .collect(),
                })
                .collect()
        })
        .unwrap_or_default();
    let uids = uids_of(&layer, &rows);
    Some(SetTable {
        layer,
        record,
        rows,
        uids,
    })
}

/// A Shapefile set's table from the archive's entries.
fn shapefile_table(entries: &[zip::Entry], info: &TableInfo) -> Result<Option<SetTable>, String> {
    let Some(record) = info.record.clone().filter(|r| r.problem().is_none()) else {
        return Ok(None);
    };
    let stem = info
        .file
        .as_deref()
        .and_then(|f| f.rsplit_once('.').map(|(s, _)| s.to_owned()))
        .unwrap_or_else(|| info.table.clone());
    let part = |ext: &str| {
        entries
            .iter()
            .find(|e| {
                e.name
                    .rsplit_once('.')
                    .is_some_and(|(s, x)| s == stem && x.eq_ignore_ascii_case(ext))
            })
            .map(|e| e.data.as_slice())
    };
    let layer = layer_of(info);
    let file_name = |c: &InfoColumn| c.file.clone().unwrap_or_else(|| c.name.clone());
    let uid_file = info
        .columns
        .iter()
        .find(|c| c.name == "kentos_uid")
        .map(file_name)
        .unwrap_or_else(|| "kentos_uid".into());
    // The shapes by their rows' lasting ids: the reader leaves a null shape (a record) out.
    let mut shapes: HashMap<String, Geometry> = HashMap::new();
    if let Some(shp) = part("shp") {
        let files = kentos_formats::shp::Files {
            shp,
            shx: part("shx"),
            dbf: part("dbf"),
            prj: part("prj"),
            cpg: part("cpg"),
        };
        let read = kentos_formats::shp::read(
            &files,
            &ShapefileReadOptions {
                layer: info.table.clone(),
                max_entities: 0,
            },
        )?;
        for e in &read.entities {
            if let (Some(uid), Ok(Some((g, _)))) = (
                e.base().attrs.get(&uid_file),
                crate::geopackage::geometry_of(e),
            ) {
                shapes.insert(uid.clone(), g);
            }
        }
    }
    // The rows in the .dbf's order, every one.
    let Some(dbf) = part("dbf") else {
        return Err(format!("“{}” setinin .dbf dosyası arşivde yok", info.table));
    };
    let (enc, _) =
        kentos_formats::shp::dbf::encoding(part("cpg"), dbf.get(29).copied().unwrap_or(0));
    let table = kentos_formats::shp::dbf::Table::open(dbf, enc)?;
    let mut rows = Vec::new();
    for i in 0..table.count {
        let Some((deleted, attrs)) = table.record(i) else {
            break;
        };
        if deleted {
            continue;
        }
        let values: Vec<Value> = info
            .columns
            .iter()
            .zip(&layer.columns)
            .map(|(ic, c)| text_value(attrs.get(&file_name(ic)).map(String::as_str), &c.kind))
            .collect();
        let geometry = attrs.get(&uid_file).and_then(|u| shapes.remove(u));
        rows.push(gpkg::Row {
            fid: rows.len() as i64 + 1,
            geometry: geometry.map(Ok),
            values,
        });
    }
    let uids = uids_of(&layer, &rows);
    Ok(Some(SetTable {
        layer,
        record,
        rows,
        uids,
    }))
}

/// The archive's index and entries; none when it has no index.
fn archive(bytes: &[u8]) -> Result<Option<(Manifest, Vec<zip::Entry>)>, String> {
    let limits = zip::Limits {
        entries: 10_000,
        total: 4 * 1024 * 1024 * 1024,
        ratio: 1000,
    };
    let listed = zip::list(bytes, &limits)?;
    if !listed.iter().any(|l| l.file_name() == MANIFEST) {
        return Ok(None);
    }
    let entries = zip::unpack(bytes, &listed, &limits, |_| true)?;
    let Some(m) = entries
        .iter()
        .find(|e| e.name.rsplit('/').next() == Some(MANIFEST))
    else {
        return Ok(None);
    };
    let manifest: Manifest =
        serde_json::from_slice(&m.data).map_err(|e| format!("{MANIFEST}: {e}"))?;
    if manifest.format != MANIFEST_FORMAT || manifest.version != 1 {
        return Err(format!(
            "{MANIFEST}: “{}” {} sürümü okunmuyor",
            manifest.format, manifest.version
        ));
    }
    Ok(Some((manifest, entries)))
}

/// The derived dataset at `path`: a GeoPackage, a zip of Shapefiles or GeoJSON files, or a GeoJSON file. Refusals:
/// `import_unreadable`, `not_a_derivation` (no derived table: not KentOS's, or every link broken).
pub fn open(path: &Path) -> Result<DerivedSet, Refusal> {
    let bytes = std::fs::read(path).map_err(|e| unreadable(path, e))?;
    let sha256 = kentos_commands::encoding::sha256_hex(&bytes);
    let file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (container, tables) = if bytes.starts_with(b"SQLite format 3\0") {
        (Container::GeoPackage, geopackage_tables(path)?)
    } else if zip::sniff(&bytes) {
        let Some((manifest, entries)) = archive(&bytes).map_err(|e| unreadable(path, e))? else {
            return Err(not_derived(path));
        };
        let mut tables = Vec::new();
        let container = if manifest.container == "geojson" {
            Container::GeoJson
        } else {
            Container::Shapefile
        };
        for info in &manifest.tables {
            let table = if container == Container::GeoJson {
                let name = info.file.clone().unwrap_or_default();
                let Some(e) = entries.iter().find(|e| e.name == name) else {
                    return Err(unreadable(path, format!("“{name}” arşivde yok")));
                };
                let fc: J = serde_json::from_slice(&e.data).map_err(|err| unreadable(path, err))?;
                geojson_table(&fc, Some(info.clone()))
            } else {
                shapefile_table(&entries, info).map_err(|e| unreadable(path, e))?
            };
            tables.extend(table);
        }
        (container, tables)
    } else {
        let fc: J = serde_json::from_slice(&bytes)
            .map_err(|_| unreadable(path, "GeoPackage, zip ya da GeoJSON değil"))?;
        if fc["type"] != "FeatureCollection" {
            return Err(unreadable(path, "GeoJSON FeatureCollection değil"));
        }
        (
            Container::GeoJson,
            geojson_table(&fc, None).into_iter().collect(),
        )
    };
    if tables.is_empty() {
        return Err(not_derived(path));
    }
    Ok(DerivedSet {
        file,
        bytes: bytes.len() as u64,
        sha256,
        container,
        tables,
    })
}

fn geopackage_tables(path: &Path) -> Result<Vec<SetTable>, Refusal> {
    let gp = GeoPackage::open(path, false).map_err(|e| {
        (
            "import_unreadable",
            format!("{} GeoPackage olarak okunamadı: {}", path.display(), e.0),
        )
    })?;
    let layers = gp.layers().map_err(|e| unreadable(path, e.0))?;
    let mut out = Vec::new();
    for layer in layers {
        let Some(record) = gis_derive::record_of(&gp, &layer.name) else {
            continue;
        };
        let uids = gp.uids(&layer.name).map_err(|e| unreadable(path, e.0))?;
        let rows = gp
            .rows(&layer, &gpkg::Query::default())
            .map_err(|e| unreadable(path, e.0))?;
        out.push(SetTable {
            layer,
            record,
            rows,
            uids,
        });
    }
    Ok(out)
}

// ── Breaking the link ──────────────────────────────────────────────────────

/// Breaks the links of `notes`' tables (table → its note as JSON) in a zip or GeoJSON derivative at `path`: each
/// table's record becomes its note; the file is written whole in place of the old one, or not at all.
pub fn detach_file(path: &Path, notes: &BTreeMap<String, J>) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let swap = |info: &mut TableInfo| {
        if let Some(note) = notes.get(&info.table) {
            info.record = None;
            info.detached = Some(note.clone());
        }
    };
    let out = if zip::sniff(&bytes) {
        let (mut manifest, entries) = archive(&bytes)?.ok_or("arşivde türev dizini yok")?;
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        manifest.tables.iter_mut().for_each(swap);
        for e in entries {
            if e.name.rsplit('/').next() == Some(MANIFEST) {
                continue;
            }
            let mut data = e.data;
            if e.name.to_ascii_lowercase().ends_with(".geojson") {
                let mut fc: J = serde_json::from_slice(&data).map_err(|err| err.to_string())?;
                if let Ok(mut info) = serde_json::from_value::<TableInfo>(fc[MEMBER].clone()) {
                    swap(&mut info);
                    fc[MEMBER] = serde_json::to_value(&info).map_err(|err| err.to_string())?;
                    data = serde_json::to_vec(&fc).map_err(|err| err.to_string())?;
                }
            }
            files.push((e.name, data));
        }
        files.push((
            MANIFEST.into(),
            manifest_bytes(&manifest.container, std::mem::take(&mut manifest.tables)),
        ));
        packed(&files)?
    } else {
        let mut fc: J = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let mut info: TableInfo =
            serde_json::from_value(fc[MEMBER].clone()).map_err(|e| e.to_string())?;
        swap(&mut info);
        fc[MEMBER] = serde_json::to_value(&info).map_err(|e| e.to_string())?;
        serde_json::to_vec(&fc).map_err(|e| e.to_string())?
    };
    let tmp = path.with_extension("kentos-yaziliyor");
    std::fs::write(&tmp, &out).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

// ── A database ──────────────────────────────────────────────────────────────

/// The built dataset as new tables of `schema` (docs/adr/0634 §3): each its rows with their keys, their lasting ids
/// and its record as the table's comment (its `table` the table's `schema.table`).
pub fn postgis_tables(built: &Built, schema: &str) -> Vec<crate::provider::NewTable> {
    built
        .tables
        .iter()
        .map(|t| {
            let name = format!("{schema}.{}", t.def.name);
            let mut record = t.record.clone();
            record.table = name.clone();
            let rows: Vec<gpkg::NewRow> = t
                .rows
                .iter()
                .enumerate()
                .map(|(i, r)| gpkg::NewRow {
                    fid: Some(i as i64 + 1),
                    ..r.clone()
                })
                .collect();
            let uids = t
                .uids
                .iter()
                .enumerate()
                .filter_map(|(i, u)| {
                    kentos_domain::Uuid::parse_str(u)
                        .ok()
                        .map(|u| (i as i64 + 1, u))
                })
                .collect();
            crate::provider::NewTable {
                name,
                layer: gpkg::Layer {
                    name: t.def.name.clone(),
                    data_type: t.def.data_type.clone(),
                    identifier: t.def.identifier.clone(),
                    description: t.def.description.clone(),
                    bounds: None,
                    srs_id: t.def.srs_id,
                    geometry: t.def.geometry.clone(),
                    fid: "fid".into(),
                    columns: t.def.columns.clone(),
                    indexed: false,
                },
                srid: t.def.srs_id.and_then(|s| u32::try_from(s).ok()),
                rows,
                uids,
                comment: serde_json::to_string(&record).unwrap_or_default(),
            }
        })
        .collect()
}

/// The derived tables of the database at `address`: those whose comment is a derivation record. Refusals:
/// `source_unreachable`, `not_a_derivation`.
pub fn open_database(address: &str) -> Result<DerivedSet, Refusal> {
    let Some(db) = crate::provider::postgis() else {
        return Err((
            "source_unreachable",
            "Bu ortamda PostGIS sağlayıcısı yok; veritabanındaki türev okunamaz.".into(),
        ));
    };
    let unreachable = |e: crate::provider::ProviderError| {
        (
            "source_unreachable",
            format!("{address} veritabanı okunamadı: {e:?}"),
        )
    };
    let listed = db.tables(address).map_err(unreachable)?;
    let mut tables = Vec::new();
    let mut digest = String::new();
    for t in listed {
        let Some(record) = t
            .comment
            .as_deref()
            .and_then(|c| serde_json::from_str::<DerivationRecord>(c).ok())
            .filter(|r| r.problem().is_none())
        else {
            continue;
        };
        let read = db.read(address, &t.name, None).map_err(unreachable)?;
        digest.push_str(&format!("{} {}\n", t.name, read.revision));
        let mut layer = read.layer;
        layer.name = t.name.clone();
        tables.push(SetTable {
            layer,
            record,
            rows: read.rows,
            uids: read.uids,
        });
    }
    if tables.is_empty() {
        return Err((
            "not_a_derivation",
            format!("{address} veritabanında KentOS'un türetim kaydı olan tablo yok."),
        ));
    }
    Ok(DerivedSet {
        file: address.to_owned(),
        bytes: 0,
        sha256: kentos_commands::encoding::sha256_hex(digest.as_bytes()),
        container: Container::PostGis,
        tables,
    })
}

/// A derived dataset at `path`: a database's address or a file.
pub fn open_any(path: &str) -> Result<DerivedSet, Refusal> {
    if kentos_commands::is_database(path) {
        open_database(path)
    } else {
        open(Path::new(path))
    }
}
