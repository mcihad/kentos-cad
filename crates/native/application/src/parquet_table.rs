//! A GeoParquet read (docs/adr/0628 §2, §3): the `geo` key's primary geometry column as WKB, as GeoArrow, or in a file
//! without the key Parquet's GEOMETRY type; the system its PROJJSON's code; every other column with its meaning as
//! the GeoPackage import's columns (`geopackage::table_of_rows`): the same objects, fields, lasting ids (the file's
//! `kentos_uid` column, else one derived from the file and the row) and losses. What `file.export.parquet` writes is
//! `parquet_write`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{ImportParquet, LayerFieldKind};
use kentos_formats::parquet::record::{self, Assembled, Shape};
use kentos_formats::parquet::schema::Node;
use kentos_formats::parquet::{File, Logical, Physical, Repetition, TimeUnit, Values};
use kentos_geopackage as gpkg;
use kentos_geopackage::{BlobError, ColumnKind, Coord, Curve, Geometry, Value};
use serde_json::Value as Json;

use crate::geopackage::{self, Read};

/// The `geo` key's word for a geometry column: its encoding, its system's statement (missing: OGC:CRS84), its edges.
struct GeoColumn {
    encoding: String,
    crs: Option<Json>,
    spherical: bool,
}

/// The `geo` key: the primary column, every geometry column, and the covering columns (derived, not taken).
struct Geo {
    primary: String,
    columns: BTreeMap<String, GeoColumn>,
    covering: BTreeSet<String>,
}

fn geo_of(text: &str) -> Result<Geo, String> {
    let v: Json = serde_json::from_str(text)
        .map_err(|e| format!("GeoParquet'in geo anahtarı JSON değil: {e}"))?;
    let version = v.get("version").and_then(Json::as_str).unwrap_or("");
    if !(version.starts_with("1.") || version.starts_with("0.")) {
        return Err(format!(
            "GeoParquet sürümü “{version}” okunmaz; 1.0 ve 1.1 okunur"
        ));
    }
    let primary = v
        .get("primary_column")
        .and_then(Json::as_str)
        .ok_or("GeoParquet'in birincil geometri sütunu söylenmemiş")?
        .to_owned();
    let mut columns = BTreeMap::new();
    let mut covering = BTreeSet::new();
    for (name, c) in v
        .get("columns")
        .and_then(Json::as_object)
        .into_iter()
        .flatten()
    {
        let encoding = c
            .get("encoding")
            .and_then(Json::as_str)
            .unwrap_or("WKB")
            .to_owned();
        if let Some(cov) = c.get("covering").and_then(Json::as_object) {
            for spec in cov.values() {
                for path in spec.as_object().into_iter().flat_map(|o| o.values()) {
                    if let Some(top) = path
                        .as_array()
                        .and_then(|p| p.first())
                        .and_then(Json::as_str)
                    {
                        covering.insert(top.to_owned());
                    }
                }
            }
        }
        columns.insert(
            name.clone(),
            GeoColumn {
                encoding,
                crs: c.get("crs").cloned(),
                spherical: c.get("edges").and_then(Json::as_str) == Some("spherical"),
            },
        );
    }
    if !columns.contains_key(&primary) {
        return Err(format!(
            "GeoParquet'in birincil sütunu “{primary}” sütunlar arasında yok"
        ));
    }
    Ok(Geo {
        primary,
        columns,
        covering,
    })
}

/// How a column's values become cells.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Conv {
    Bool,
    Int,
    Unsigned,
    Real,
    Half,
    Date,
    Time(TimeUnit),
    Timestamp(TimeUnit, bool),
    Int96,
    /// A decimal of `scale`: a number when its digits fit a double, else text.
    Decimal(i32, bool),
    Text,
    Uuid,
}

/// What a top-level column of the file is to the import.
enum Plan {
    Cells {
        column: usize,
        conv: Conv,
    },
    /// A list of plain values, taken as its JSON text.
    List {
        node: usize,
    },
    /// A geometry column other than the primary one, taken as WKT text.
    Wkt {
        column: usize,
    },
}

fn conv_of(node: &Node) -> Option<(Conv, ColumnKind)> {
    let p = node.physical?;
    Some(match (&node.logical, p) {
        (_, Physical::Boolean) => (Conv::Bool, ColumnKind::Boolean),
        (Some(Logical::Date), Physical::Int32) => (Conv::Date, ColumnKind::Date),
        (Some(Logical::Time { unit, .. }), Physical::Int32 | Physical::Int64) => {
            (Conv::Time(*unit), ColumnKind::Other("TIME".into()))
        }
        (Some(Logical::Timestamp { unit, utc }), Physical::Int64) => {
            (Conv::Timestamp(*unit, *utc), ColumnKind::DateTime)
        }
        (Some(Logical::Decimal { scale, precision }), _) => {
            let fits = *precision <= 15;
            (
                Conv::Decimal(*scale, fits),
                if fits {
                    ColumnKind::Real
                } else {
                    ColumnKind::Text(None)
                },
            )
        }
        (
            Some(Logical::Integer {
                signed: false,
                bits: 64,
            }),
            Physical::Int64,
        ) => (Conv::Unsigned, ColumnKind::Integer),
        (Some(Logical::Integer { signed: false, .. }), Physical::Int32) => {
            (Conv::Unsigned, ColumnKind::Integer)
        }
        (None | Some(Logical::Integer { .. }), Physical::Int32 | Physical::Int64) => {
            (Conv::Int, ColumnKind::Integer)
        }
        (_, Physical::Int96) => (Conv::Int96, ColumnKind::DateTime),
        (_, Physical::Float | Physical::Double) => (Conv::Real, ColumnKind::Real),
        (Some(Logical::Float16), Physical::FixedLenByteArray) => (Conv::Half, ColumnKind::Real),
        (Some(Logical::Uuid), Physical::FixedLenByteArray) => (Conv::Uuid, ColumnKind::Text(None)),
        (
            Some(Logical::String | Logical::Enum | Logical::Json),
            Physical::ByteArray | Physical::FixedLenByteArray,
        ) => (Conv::Text, ColumnKind::Text(None)),
        _ => return None,
    })
}

/// Value `i` of a column as a cell.
fn cell(values: &Values, i: usize, conv: Conv, node: &Node) -> Value {
    match (conv, values) {
        (Conv::Bool, Values::Bool(v)) => Value::Integer(i64::from(v[i])),
        (Conv::Int, Values::I32(v)) => Value::Integer(i64::from(v[i])),
        (Conv::Int, Values::I64(v)) => Value::Integer(v[i]),
        (Conv::Unsigned, Values::I32(v)) => Value::Integer(i64::from(v[i] as u32)),
        (Conv::Unsigned, Values::I64(v)) => match i64::try_from(v[i] as u64) {
            Ok(x) => Value::Integer(x),
            Err(_) => Value::Text((v[i] as u64).to_string()),
        },
        (Conv::Real, Values::F32(v)) => Value::Real(f64::from(v[i])),
        (Conv::Real, Values::F64(v)) => Value::Real(v[i]),
        (Conv::Half, Values::Bytes(b)) => {
            let raw = b.get(i);
            if raw.len() == 2 {
                Value::Real(f64::from(record::half(u16::from_le_bytes([
                    raw[0], raw[1],
                ]))))
            } else {
                Value::Null
            }
        }
        (Conv::Date, Values::I32(v)) => Value::Text(record::date_text(i64::from(v[i]))),
        (Conv::Time(u), Values::I32(v)) => Value::Text(record::time_text(i64::from(v[i]), u)),
        (Conv::Time(u), Values::I64(v)) => Value::Text(record::time_text(v[i], u)),
        (Conv::Timestamp(u, utc), Values::I64(v)) => {
            Value::Text(record::timestamp_text(v[i], u, utc))
        }
        (Conv::Int96, Values::I96(v)) => Value::Text(record::int96_text(&v[i])),
        (Conv::Decimal(..), _) | (Conv::Text | Conv::Uuid, _) => {
            let text = match record::value(values, i, node) {
                Json::String(s) => s,
                other => other.to_string(),
            };
            match conv {
                Conv::Decimal(_, true) => {
                    text.parse::<f64>().map_or(Value::Text(text), Value::Real)
                }
                _ => Value::Text(text),
            }
        }
        _ => Value::Null,
    }
}

/// The GeoArrow coordinate of element `e`.
fn coord(a: &Assembled, axes: &[Option<usize>; 4], e: usize) -> Option<Coord> {
    let get = |c: Option<usize>| -> Option<f64> {
        let (leaf, n) = a.leaves.get(&c?)?;
        let v = n.values.get(e).copied().flatten()?;
        match &leaf.values {
            Values::F64(x) => Some(x[v]),
            Values::F32(x) => Some(f64::from(x[v])),
            _ => None,
        }
    };
    Some(Coord {
        x: get(axes[0])?,
        y: get(axes[1])?,
        z: get(axes[2]),
        m: get(axes[3]),
    })
}

/// The geometries of a GeoArrow column, record by record.
fn geoarrow(
    f: &File<Vec<u8>>,
    group: usize,
    top: usize,
    encoding: &str,
) -> Result<Vec<Option<Geometry>>, String> {
    let s = &f.schema;
    let a = record::assemble(f, group, top)?;
    let cols = s.columns_under(top);
    let axis = |name: &str| {
        cols.iter()
            .copied()
            .find(|&c| s.nodes[s.columns[c]].name == name)
    };
    let axes = [axis("x"), axis("y"), axis("z"), axis("m")];
    let x = axes[0].ok_or("GeoArrow sütununda x yok")?;
    axes[1].ok_or("GeoArrow sütununda y yok")?;
    let (_, n) = &a.leaves[&x];
    let levels = match encoding {
        "point" => 0,
        "linestring" | "multipoint" => 1,
        "polygon" | "multilinestring" => 2,
        "multipolygon" => 3,
        e => return Err(format!("GeoArrow kodlaması “{e}” tanınmıyor")),
    };
    if n.offsets.len() != levels {
        return Err(format!(
            "GeoArrow “{encoding}” sütununun iç içeliği beklenenden başka"
        ));
    }
    let pt = |e: usize| coord(&a, &axes, e).ok_or_else(|| "GeoArrow koordinatı eksik".to_owned());
    let line = |level: usize, i: usize| -> Result<Vec<Coord>, String> {
        n.range(level, i).map(pt).collect()
    };
    let mut out = Vec::with_capacity(a.records);
    for r in 0..a.records {
        if levels == 0 {
            out.push(match n.values.get(r).copied().flatten() {
                Some(_) => Some(Geometry::Point(Some(pt(r)?))),
                None => None,
            });
            continue;
        }
        if !n.present[0].get(r).copied().unwrap_or(false) {
            out.push(None);
            continue;
        }
        let g = match encoding {
            "linestring" => Geometry::Curve(Curve::Line(line(0, r)?)),
            "multipoint" => Geometry::MultiPoint(line(0, r)?),
            "polygon" => Geometry::Polygon(
                n.range(0, r)
                    .map(|ring| line(1, ring).map(Curve::Line))
                    .collect::<Result<_, _>>()?,
            ),
            "multilinestring" => Geometry::MultiCurve(
                n.range(0, r)
                    .map(|l| line(1, l).map(Curve::Line))
                    .collect::<Result<_, _>>()?,
            ),
            _ => Geometry::MultiPolygon(
                n.range(0, r)
                    .map(|p| {
                        n.range(1, p)
                            .map(|ring| line(2, ring).map(Curve::Line))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<_, _>>()?,
            ),
        };
        out.push(Some(g));
    }
    Ok(out)
}

/// The geometries of a WKB column, record by record.
fn wkb(
    f: &File<Vec<u8>>,
    group: usize,
    column: usize,
) -> Result<Vec<Option<Result<Geometry, BlobError>>>, String> {
    let a = record::assemble(f, group, f.schema.top(column))?;
    let (leaf, n) = &a.leaves[&column];
    let Values::Bytes(b) = &leaf.values else {
        return Err("WKB sütunu bayt dizisi değil".into());
    };
    Ok(n.values
        .iter()
        .map(|v| v.map(|i| gpkg::read_wkb(b.get(i))))
        .collect())
}

/// The file's system from a geometry column's statement, and what is said of it.
fn system_of(crs: &Option<Json>, losses: &mut Vec<String>) -> Option<u32> {
    match crs {
        None => Some(4326),
        Some(v) => match kentos_project::projjson::srid_of(v) {
            Ok(s) => s,
            Err(why) => {
                losses.push(format!("dosyanın sistemi tanınmadı: {why}"));
                None
            }
        },
    }
}

/// Parquet 2.11's GEOMETRY type's system: none said is OGC:CRS84; `srid:N`, a code, or a PROJJSON in the file's
/// key-value metadata (`projjson:key`).
fn logical_system(
    crs: &Option<String>,
    f: &File<Vec<u8>>,
    losses: &mut Vec<String>,
) -> Option<u32> {
    let Some(text) = crs else {
        return Some(4326);
    };
    if let Some(n) = text.strip_prefix("srid:") {
        return n.parse().ok();
    }
    let v = if let Some(key) = text.strip_prefix("projjson:") {
        f.meta
            .value(key)
            .and_then(|t| serde_json::from_str::<Json>(t).ok())
            .unwrap_or(Json::Null)
    } else {
        serde_json::from_str::<Json>(text).unwrap_or_else(|_| Json::String(text.clone()))
    };
    system_of(&Some(v), losses)
}

/// The file's table as the GeoPackage import reads one.
pub fn read(path: &std::path::Path, bytes: Vec<u8>, input: &ImportParquet) -> Result<Read, String> {
    let file = path.file_name().map_or_else(
        || "katman.parquet".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let stem = path
        .file_stem()
        .map_or_else(|| "katman".to_owned(), |n| n.to_string_lossy().into_owned());
    let size = bytes.len() as u64;
    let sha256 = sha256_hex(&bytes);
    let f = File::open(bytes)?;
    let s = &f.schema;
    let mut losses: Vec<String> = Vec::new();
    let tops: Vec<usize> = s.nodes[0].children.clone();
    let by_name = |name: &str| tops.iter().copied().find(|&t| s.nodes[t].name == name);
    // The primary geometry: the `geo` key's, else a column of Parquet's GEOMETRY type.
    let geo = match f.meta.value("geo") {
        Some(t) => Some(geo_of(t)?),
        None => None,
    };
    let mut geometry: Option<(usize, String)> = None;
    let mut srid = None;
    let mut skip: BTreeSet<usize> = BTreeSet::new();
    let mut others: BTreeSet<usize> = BTreeSet::new();
    if let Some(g) = &geo {
        let top = by_name(&g.primary)
            .ok_or_else(|| format!("birincil geometri sütunu “{}” dosyada yok", g.primary))?;
        let c = &g.columns[&g.primary];
        let encoding = c.encoding.to_ascii_lowercase();
        if encoding != "wkb"
            && !matches!(
                encoding.as_str(),
                "point"
                    | "linestring"
                    | "polygon"
                    | "multipoint"
                    | "multilinestring"
                    | "multipolygon"
            )
        {
            return Err(format!(
                "geometri kodlaması “{}” okunmaz; WKB ya da GeoArrow okunur",
                c.encoding
            ));
        }
        geometry = Some((top, encoding));
        srid = system_of(&c.crs, &mut losses);
        if c.spherical {
            losses.push(
                "geometrinin kenarları küresel (jeodezik); düzlemde düz kenarlar olarak alındı"
                    .into(),
            );
        }
        skip.insert(top);
        for name in &g.covering {
            if let Some(t) = by_name(name) {
                skip.insert(t);
            }
        }
        for (name, c) in &g.columns {
            if name != &g.primary
                && let Some(t) = by_name(name)
            {
                if c.encoding.eq_ignore_ascii_case("wkb") {
                    others.insert(t);
                } else {
                    skip.insert(t);
                    losses.push(format!(
                        "“{name}” geometri sütunu ({}) alınmadı",
                        c.encoding
                    ));
                }
            }
        }
    } else if let Some(&top) = tops.iter().find(|&&t| {
        matches!(
            s.nodes[t].logical,
            Some(Logical::Geometry { .. } | Logical::Geography { .. })
        ) && s.nodes[t].column.is_some()
    }) {
        geometry = Some((top, "wkb".into()));
        let crs = match &s.nodes[top].logical {
            Some(Logical::Geometry { crs } | Logical::Geography { crs }) => crs.clone(),
            _ => None,
        };
        srid = logical_system(&crs, &f, &mut losses);
        if matches!(s.nodes[top].logical, Some(Logical::Geography { .. })) {
            losses.push(
                "geometri sütunu GEOGRAPHY (küresel kenarlar); düzlemde düz kenarlar olarak alındı"
                    .into(),
            );
        }
        skip.insert(top);
        for &t in &tops {
            if t != top
                && matches!(
                    s.nodes[t].logical,
                    Some(Logical::Geometry { .. } | Logical::Geography { .. })
                )
            {
                others.insert(t);
            }
        }
    }
    let uid_top = by_name("kentos_uid").filter(|&t| s.nodes[t].column.is_some());
    if let Some(t) = uid_top {
        skip.insert(t);
    }
    // The other columns.
    let mut plans: Vec<(String, Plan)> = Vec::new();
    let mut columns: Vec<gpkg::Column> = Vec::new();
    let mut time_columns: Vec<String> = Vec::new();
    for &t in &tops {
        if skip.contains(&t) {
            continue;
        }
        let node = &s.nodes[t];
        let name = node.name.clone();
        let plan = if others.contains(&t) && node.column.is_some() {
            losses.push(format!("“{name}” geometri sütunu WKT metni olarak alındı"));
            Some((
                Plan::Wkt {
                    column: node.column.unwrap_or(0),
                },
                ColumnKind::Text(None),
            ))
        } else if let (Some(c), Repetition::Required | Repetition::Optional) =
            (node.column, node.repetition)
        {
            match conv_of(node) {
                Some((conv, kind)) => {
                    if matches!(conv, Conv::Time(_)) {
                        time_columns.push(name.clone());
                    }
                    Some((Plan::Cells { column: c, conv }, kind))
                }
                None => {
                    losses.push(format!("“{name}” ikili sütunu alınmadı"));
                    None
                }
            }
        } else {
            match record::shape(s, t)? {
                Shape::List { element, .. } if matches!(*element, Shape::Leaf { .. }) => {
                    Some((Plan::List { node: t }, ColumnKind::Text(None)))
                }
                _ => {
                    losses.push(format!(
                        "“{name}” sütunu iç içe (yapı, eşleme ya da liste listesi); alınmadı"
                    ));
                    None
                }
            }
        };
        if let Some((plan, kind)) = plan {
            columns.push(gpkg::Column {
                name: name.clone(),
                decl: String::new(),
                kind,
                not_null: false,
                default: None,
            });
            plans.push((name, plan));
        }
    }
    // Row group after row group, the rows.
    let mut rows: Vec<gpkg::Row> = Vec::with_capacity(f.meta.num_rows.max(0) as usize);
    let mut uids: HashMap<i64, String> = HashMap::new();
    let mut fid = 0i64;
    let mut wide_unsigned: BTreeSet<String> = BTreeSet::new();
    for group in 0..f.meta.row_groups.len() {
        let n = f.meta.row_groups[group].num_rows.max(0) as usize;
        let geoms: Vec<Option<Result<Geometry, BlobError>>> = match &geometry {
            None => vec![None; n],
            Some((top, enc)) if enc == "wkb" => {
                let c = s.nodes[*top].column.ok_or("WKB sütunu bir sütun değil")?;
                wkb(&f, group, c)?
            }
            Some((top, enc)) => geoarrow(&f, group, *top, enc)?
                .into_iter()
                .map(|g| g.map(Ok))
                .collect(),
        };
        if geoms.len() != n {
            return Err("geometri sütununun satırları satır grubunun satırlarıyla tutmuyor".into());
        }
        let mut cells: Vec<Vec<Value>> = Vec::with_capacity(plans.len());
        for (name, plan) in &plans {
            let col: Vec<Value> = match plan {
                Plan::Cells { column, conv } => {
                    let a = record::assemble(&f, group, s.top(*column))?;
                    let (leaf, nest) = &a.leaves[column];
                    let node = &s.nodes[s.columns[*column]];
                    nest.values
                        .iter()
                        .map(|v| match v {
                            Some(i) => {
                                let c = cell(&leaf.values, *i, *conv, node);
                                if matches!(c, Value::Text(_)) && *conv == Conv::Unsigned {
                                    wide_unsigned.insert(name.clone());
                                }
                                c
                            }
                            None => Value::Null,
                        })
                        .collect()
                }
                Plan::List { node } => record::records(&f, group, *node)?
                    .into_iter()
                    .map(|v| {
                        if v.is_null() {
                            Value::Null
                        } else {
                            Value::Text(v.to_string())
                        }
                    })
                    .collect(),
                Plan::Wkt { column } => wkb(&f, group, *column)?
                    .into_iter()
                    .map(|g| match g {
                        Some(Ok(g)) => Value::Text(gpkg::wkt::write_wkt(&g)),
                        _ => Value::Null,
                    })
                    .collect(),
            };
            if col.len() != n {
                return Err(format!(
                    "“{name}” sütununun satırları satır grubunun satırlarıyla tutmuyor"
                ));
            }
            cells.push(col);
        }
        if let Some(t) = uid_top {
            let c = s.nodes[t].column.unwrap_or(0);
            let a = record::assemble(&f, group, t)?;
            let (leaf, nest) = &a.leaves[&c];
            if let Values::Bytes(b) = &leaf.values {
                for (k, v) in nest.values.iter().enumerate() {
                    if let Some(i) = v {
                        uids.insert(
                            fid + k as i64 + 1,
                            String::from_utf8_lossy(b.get(*i)).into_owned(),
                        );
                    }
                }
            }
        }
        for (k, g) in geoms.into_iter().enumerate() {
            fid += 1;
            rows.push(gpkg::Row {
                fid,
                geometry: g,
                values: cells.iter().map(|c| c[k].clone()).collect(),
            });
        }
    }
    // An unsigned column holding a value past i64 goes as text, every value.
    for name in &wide_unsigned {
        if let Some(i) = columns.iter().position(|c| &c.name == name) {
            columns[i].kind = ColumnKind::Text(None);
            for r in &mut rows {
                if let Value::Integer(v) = r.values[i] {
                    r.values[i] = Value::Text(v.to_string());
                }
            }
            losses.push(format!(
                "“{name}” sütunu 2⁶³'ü aşan işaretsiz sayılar tutuyor: metin alanı olarak alındı"
            ));
        }
    }
    // A time's fraction of a second does not go into a time field.
    for name in &time_columns {
        if let Some(i) = columns.iter().position(|c| &c.name == name) {
            let mut cut = false;
            for r in &mut rows {
                if let Value::Text(t) = &mut r.values[i]
                    && t.len() > 8
                {
                    t.truncate(8);
                    cut = true;
                }
            }
            if cut {
                losses.push(format!(
                    "“{name}” sütununun saniyenin kesirleri alınmadı: saat alanı saniyeyle yazılır"
                ));
            }
        }
    }
    let srid = srid.or(input.srid);
    let has_geometry = rows.iter().any(|r| r.geometry.is_some());
    let layer = gpkg::Layer {
        name: stem,
        data_type: if has_geometry {
            gpkg::DataType::Features
        } else {
            gpkg::DataType::Attributes
        },
        identifier: None,
        description: None,
        bounds: None,
        srs_id: srid.map(|s| s as i32),
        geometry: None,
        fid: "fid".into(),
        columns,
        indexed: false,
    };
    let key = format!("parquet/{file}");
    let mut t =
        geopackage::table_of_rows(&layer, &rows, &uids, &|fid| geopackage::row_uid(&key, fid))?;
    for f in &mut t.fields {
        if time_columns.contains(&f.name) {
            f.kind = LayerFieldKind::Time;
        }
    }
    t.losses.extend(losses);
    let mut systems = BTreeMap::new();
    if let Some(s) = srid {
        systems.insert(s as i32, Some(("EPSG".to_owned(), s as i32)));
    }
    Ok(Read {
        file,
        bytes: size,
        sha256,
        tables: vec![t],
        systems,
    })
}
