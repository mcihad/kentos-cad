//! A CSV table of objects (docs/adr/0522, plan/gis.md CBS-05): what `file.export.csv` writes and `file.import.csv`
//! reads. A row is an object: its geometry as ISO WKT (`kentos_geopackage::wkt`; arcs as circular strings, exact) or
//! as X and Y columns, its attributes in the other columns. Beside the CSV a GDAL `.csvt` names each column's type and a
//! `.prj` the system (as the Shapefile's, `shapefile::prj`). Reading goes through the GeoPackage import's rows
//! (`geopackage::table_of_rows`): the same objects, fields, lasting ids and losses as a GeoPackage's table.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    CsvExported, CsvGeometry, CsvGeometryIn, CsvSeparator, Entity, ExportCsv, ImportCsv,
    LayerFieldKind,
};
use kentos_domain::Document;
use kentos_formats::table_file::{Limits, read_with};
use kentos_geopackage as gpkg;
use kentos_geopackage::wkt::{read_wkt, write_wkt};
use kentos_geopackage::{BlobError, ColumnKind, Coord, Geometry, Value};

use crate::geopackage::{self, Read};

/// What `file.export.csv` makes before it writes: the files' bytes and the answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    pub csv: Vec<u8>,
    pub csvt: Vec<u8>,
    pub prj: Option<Vec<u8>>,
    pub output: CsvExported,
}

/// A column written: its name, its GDAL type, and where its values come from.
#[derive(Clone, Debug, PartialEq)]
enum Source {
    Wkt,
    X,
    Y,
    Z,
    Layer,
    Field(String, ColumnKind),
}

fn gdal_type(kind: &LayerFieldKind, length: Option<u32>) -> String {
    match kind {
        LayerFieldKind::Text => {
            length.map_or_else(|| "String".to_owned(), |n| format!("String({n})"))
        }
        LayerFieldKind::Integer => "Integer64".into(),
        LayerFieldKind::Decimal => "Real".into(),
        LayerFieldKind::Date => "Date".into(),
        LayerFieldKind::DateTime => "DateTime".into(),
        LayerFieldKind::Time => "Time".into(),
        LayerFieldKind::Boolean => "Integer(Boolean)".into(),
    }
}

/// A field's value as the CSV writes it.
fn text_of(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Integer(i) => i.to_string(),
        Value::Real(r) => {
            if *r == r.trunc() && r.abs() < 1e15 {
                format!("{}", *r as i64)
            } else {
                format!("{r}")
            }
        }
        Value::Text(t) => t.clone(),
        Value::Blob(_) => String::new(),
    }
}

/// RFC 4180: a field holding the separator, a quote, a line break or spaces at its ends is quoted.
fn quoted(t: &str, sep: char) -> String {
    if t.contains(sep)
        || t.contains('"')
        || t.contains('\n')
        || t.contains('\r')
        || t.starts_with(' ')
        || t.ends_with(' ')
    {
        format!("\"{}\"", t.replace('"', "\"\""))
    } else {
        t.to_owned()
    }
}

fn num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// The layers' objects as a CSV, or why none can be written.
pub fn export(doc: &Document, input: &ExportCsv) -> Result<Made, (String, String, String)> {
    let ids = crate::shapefile::layer_ids(doc, &input.layers).map_err(|(i, id)| {
        (
            kentos_commands::codes::LAYER_NOT_FOUND.to_owned(),
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            format!("layers[{i}]"),
        )
    })?;
    let sep = match input.separator {
        CsvSeparator::Comma => ',',
        CsvSeparator::Semicolon => ';',
        CsvSeparator::Tab => '\t',
    };
    // The objects, layer by layer in the tree's order.
    let mut objects: Vec<(&str, &Entity)> = Vec::new();
    for id in &ids {
        for e in doc.by_layer(id) {
            objects.push((id.as_str(), e));
        }
    }
    let used: Vec<&String> = ids
        .iter()
        .filter(|id| objects.iter().any(|(l, _)| *l == id.as_str()))
        .collect();
    // The fields of the layers, then the attributes no field names, by name.
    let mut fields: Vec<(String, ColumnKind, String)> = Vec::new();
    let mut lost: BTreeSet<String> = BTreeSet::new();
    for id in &used {
        if let Some(node) = doc.layers().get(id) {
            for f in &node.fields {
                let c = geopackage::column_of(f);
                let t = gdal_type(&f.kind, f.length);
                match fields
                    .iter_mut()
                    .find(|(n, _, _)| n.eq_ignore_ascii_case(&f.name))
                {
                    Some(have) if have.2 != t => {
                        lost.insert(format!(
                            "“{}” alanı katmanlarda farklı türlerde: metin sütunu olarak yazıldı",
                            f.name
                        ));
                        have.1 = ColumnKind::Text(None);
                        have.2 = "String".into();
                    }
                    Some(_) => {}
                    None => fields.push((f.name.clone(), c.kind, t)),
                }
            }
        }
    }
    let mut others: BTreeSet<String> = BTreeSet::new();
    for (_, e) in &objects {
        for k in e.base().attrs.keys() {
            if !fields.iter().any(|(n, _, _)| n.eq_ignore_ascii_case(k)) {
                others.insert(k.clone());
            }
        }
    }
    for k in others {
        fields.push((k, ColumnKind::Text(None), "String".into()));
    }
    let mut columns: Vec<(String, String, Source)> = Vec::new();
    let any_z = objects
        .iter()
        .any(|(_, e)| matches!(e, Entity::Point(p) if p.z.is_some()));
    match input.geometry {
        CsvGeometry::Wkt => columns.push(("WKT".into(), "WKT".into(), Source::Wkt)),
        CsvGeometry::Xy => {
            columns.push(("X".into(), "CoordX".into(), Source::X));
            columns.push(("Y".into(), "CoordY".into(), Source::Y));
            if any_z {
                columns.push(("Z".into(), "Real".into(), Source::Z));
            }
        }
        CsvGeometry::None => {}
    }
    if used.len() > 1 {
        columns.push(("katman".into(), "String".into(), Source::Layer));
    }
    for (name, kind, t) in fields {
        let mut name = name;
        // A field named as a geometry or layer column keeps its own name with a mark.
        while columns
            .iter()
            .any(|(n, _, _)| n.eq_ignore_ascii_case(&name))
        {
            lost.insert(format!(
                "“{name}” alanı “{name}_” sütunu olarak yazıldı (adı başka bir sütunun)"
            ));
            name.push('_');
        }
        let source = Source::Field(name.clone(), kind);
        columns.push((name, t, source));
    }
    let header: Vec<String> = columns.iter().map(|(n, _, _)| quoted(n, sep)).collect();
    let mut out = String::new();
    out.push_str(&header.join(&sep.to_string()));
    out.push_str("\r\n");
    let mut rows = 0u32;
    let mut left: BTreeMap<String, u32> = BTreeMap::new();
    let mut chords = 0u32;
    for (layer, e) in &objects {
        let geometry = match geopackage::geometry_of(e) {
            Ok(g) => g,
            Err(word) => {
                *left.entry(word.to_owned()).or_default() += 1;
                continue;
            }
        };
        let point: Option<Coord> = match (&input.geometry, &geometry) {
            (CsvGeometry::Xy, Some((Geometry::Point(Some(c)), _))) => Some(*c),
            (CsvGeometry::Xy, Some((g, _))) => {
                let word = match g {
                    Geometry::MultiPoint(_) => "çok noktalı nesne",
                    Geometry::Curve(_) | Geometry::MultiCurve(_) => "çizgi",
                    Geometry::Polygon(_) | Geometry::MultiPolygon(_) => "alan",
                    _ => "başka nesne",
                };
                *left.entry(word.to_owned()).or_default() += 1;
                continue;
            }
            _ => None,
        };
        if let Some((_, true)) = &geometry
            && input.geometry == CsvGeometry::Wkt
        {
            chords += 1;
        }
        let attrs = &e.base().attrs;
        let cells: Vec<String> = columns
            .iter()
            .map(|(_, _, s)| match s {
                Source::Wkt => geometry
                    .as_ref()
                    .map(|(g, _)| quoted(&write_wkt(g), sep))
                    .unwrap_or_default(),
                Source::X => point.map(|c| num(c.x)).unwrap_or_default(),
                Source::Y => point.map(|c| num(c.y)).unwrap_or_default(),
                Source::Z => point.and_then(|c| c.z).map(num).unwrap_or_default(),
                Source::Layer => quoted(
                    &doc.layers()
                        .get(layer)
                        .map(|n| n.name.clone())
                        .unwrap_or_default(),
                    sep,
                ),
                Source::Field(name, kind) => {
                    let v = attrs.get(name.trim_end_matches('_')).or_else(|| {
                        attrs
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case(name.trim_end_matches('_')))
                            .map(|(_, v)| v)
                    });
                    quoted(&text_of(&geopackage::cell(v, kind)), sep)
                }
            })
            .collect();
        out.push_str(&cells.join(&sep.to_string()));
        out.push_str("\r\n");
        rows += 1;
    }
    if rows == 0 {
        return Err((
            kentos_commands::codes::NOTHING_TO_EXPORT.to_owned(),
            "Katmanlarda CSV'ye yazılacak nesne yok.".into(),
            "layers".into(),
        ));
    }
    if chords > 0 {
        lost.insert(format!(
            "{chords} eğri (elips, eğri, NURBS) WKT'de karşılığı olmadığından kirişlerle yazıldı"
        ));
    }
    if !left.is_empty() {
        lost.insert(format!(
            "CSV'de karşılığı olmayan nesneler yazılmadı: {}",
            left.iter()
                .map(|(k, n)| format!("{n} {k}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let types: Vec<String> = columns.iter().map(|(_, t, _)| t.clone()).collect();
    let csvt = format!(
        "{}\r\n",
        types
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(",")
    )
    .into_bytes();
    let prj = crate::shapefile::wkt1_prj(doc).map(String::into_bytes);
    if prj.is_none() {
        lost.insert("projenin kayıtlı bir koordinat sistemi yok: .prj yazılmadı".into());
    }
    let csv = out.into_bytes();
    let output = CsvExported {
        path: input.path.clone(),
        rows,
        columns: columns.iter().map(|(n, _, _)| n.clone()).collect(),
        types,
        files: Vec::new(),
        left,
        lost: lost.into_iter().collect(),
    };
    Ok(Made {
        csv,
        csvt,
        prj,
        output,
    })
}

// ── Reading ─────────────────────────────────────────────────────────────

const WKT_NAMES: [&str; 5] = ["wkt", "geom", "geometry", "the_geom", "wkt_geom"];
const XY_NAMES: [(&str, &str); 5] = [
    ("x", "y"),
    ("lon", "lat"),
    ("longitude", "latitude"),
    ("easting", "northing"),
    ("doğu", "kuzey"),
];

/// A column's kind from a `.csvt` type.
fn kind_of_csvt(t: &str) -> Option<ColumnKind> {
    let up = t.trim().trim_matches('"').to_ascii_uppercase();
    let base = up.split('(').next().unwrap_or("").trim().to_owned();
    Some(match base.as_str() {
        "INTEGER" if up.contains("BOOLEAN") => ColumnKind::Boolean,
        "INTEGER" | "INTEGER64" => ColumnKind::Integer,
        "REAL" => ColumnKind::Real,
        "STRING" => ColumnKind::Text(
            up.split_once('(')
                .and_then(|(_, w)| {
                    w.trim_end_matches(')')
                        .split('.')
                        .next()?
                        .trim()
                        .parse()
                        .ok()
                })
                .filter(|w: &u32| *w > 0),
        ),
        "DATE" => ColumnKind::Date,
        "DATETIME" => ColumnKind::DateTime,
        "TIME" => ColumnKind::Text(None),
        _ => return None,
    })
}

fn is_date(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && ((b[4] == b'-' && b[7] == b'-') || (b[4] == b'/' && b[7] == b'/'))
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

fn is_datetime(t: &str) -> bool {
    t.len() >= 19
        && is_date(&t[..10])
        && matches!(t.as_bytes()[10], b'T' | b' ')
        && t[11..19].bytes().enumerate().all(|(i, c)| {
            if i == 2 || i == 5 {
                c == b':'
            } else {
                c.is_ascii_digit()
            }
        })
}

fn is_integer(t: &str) -> bool {
    let d = t.strip_prefix('-').unwrap_or(t);
    !d.is_empty()
        && d.bytes().all(|c| c.is_ascii_digit())
        && (d == "0" || !d.starts_with('0'))
        && t.parse::<i64>().is_ok()
}

fn is_real(t: &str) -> bool {
    // A code with a leading zero ("007", "01.5") stays text.
    let d = t.strip_prefix('-').unwrap_or(t);
    let leading_zero = d.len() > 1 && d.starts_with('0') && !d.starts_with("0.");
    !t.starts_with('+')
        && !d.starts_with('.')
        && !leading_zero
        && d.bytes()
            .all(|c| c.is_ascii_digit() || b".eE+-".contains(&c))
        && t.parse::<f64>().is_ok_and(f64::is_finite)
}

/// A column's kind from its values: integer, decimal, date, date and time, else text.
fn kind_of_values<'a>(values: impl Iterator<Item = &'a str> + Clone) -> ColumnKind {
    let mut filled = values.filter(|v| !v.is_empty()).peekable();
    if filled.peek().is_none() {
        return ColumnKind::Text(None);
    }
    let all = |f: fn(&str) -> bool| filled.clone().all(f);
    if all(is_integer) {
        ColumnKind::Integer
    } else if all(is_real) {
        ColumnKind::Real
    } else if all(is_date) {
        ColumnKind::Date
    } else if all(is_datetime) {
        ColumnKind::DateTime
    } else {
        ColumnKind::Text(None)
    }
}

/// A date as GDAL's CSV writes it (`2024/02/29`) in ISO's form (`2024-02-29`).
fn iso_date(t: &str) -> String {
    let b = t.as_bytes();
    if b.len() >= 10 && b[4] == b'/' && b[7] == b'/' {
        format!("{}-{}-{}{}", &t[..4], &t[5..7], &t[8..10], &t[10..])
    } else {
        t.to_owned()
    }
}

/// A date and time as GDAL's CSV writes it (`2024/03/01 10:20:30+00`) in ISO's form (`2024-03-01T10:20:30Z`): a zone
/// of `+00` is UTC's `Z`, another stays (and the import says it is not kept).
fn iso_datetime(t: &str) -> String {
    let d = iso_date(t);
    if d.len() < 19 {
        return d;
    }
    let (head, zone) = d.split_at(19);
    let head = head.replacen(' ', "T", 1);
    match zone {
        "+00" | "+00:00" | "+0000" => format!("{head}Z"),
        z if z.len() == 3 && (z.starts_with('+') || z.starts_with('-')) => format!("{head}{z}:00"),
        z => format!("{head}{z}"),
    }
}

fn value_of(t: &str, kind: &ColumnKind) -> Value {
    if t.is_empty() {
        return Value::Null;
    }
    match kind {
        ColumnKind::Date => Value::Text(iso_date(t)),
        ColumnKind::DateTime => Value::Text(iso_datetime(t)),
        ColumnKind::Integer => t
            .parse::<i64>()
            .map_or_else(|_| Value::Text(t.to_owned()), Value::Integer),
        ColumnKind::Real => t
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
            .map_or_else(|| Value::Text(t.to_owned()), Value::Real),
        ColumnKind::Boolean => match t.to_ascii_lowercase().as_str() {
            "1" | "true" | "t" | "yes" | "evet" => Value::Integer(1),
            "0" | "false" | "f" | "no" | "hayır" => Value::Integer(0),
            _ => Value::Text(t.to_owned()),
        },
        _ => Value::Text(t.to_owned()),
    }
}

/// The CSV's columns as the import names them and its rows (the header aside), for a window to offer; or why it
/// cannot be read.
pub fn columns(bytes: &[u8]) -> Result<(Vec<String>, usize), String> {
    let table = read_with(bytes, Limits::PROCESSING);
    if let Some(problem) = table.problem {
        return Err(problem);
    }
    let sheet = table
        .sheets
        .into_iter()
        .next()
        .ok_or_else(|| "dosyada tablo yok".to_owned())?;
    let mut rows = sheet.rows.into_iter();
    let header = rows.next().ok_or_else(|| "dosya boş".to_owned())?;
    Ok((column_names(&header), rows.count()))
}

/// The header's names: blank ones numbered, repeated ones marked.
fn column_names(header: &[String]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for (k, h) in header.iter().enumerate() {
        let mut n = h.trim().to_owned();
        if n.is_empty() {
            n = format!("alan_{}", k + 1);
        }
        while names.iter().any(|x| x.eq_ignore_ascii_case(&n)) {
            n.push('_');
        }
        names.push(n);
    }
    names
}

/// Where `Auto` finds the geometry among `names`: the WKT column, else X and Y (and Z), else none.
pub fn auto_geometry(names: &[String]) -> CsvGeometryIn {
    let find = |want: &str| names.iter().find(|n| n.eq_ignore_ascii_case(want)).cloned();
    if let Some(column) = WKT_NAMES.iter().find_map(|w| find(w)) {
        return CsvGeometryIn::Wkt { column };
    }
    match XY_NAMES
        .iter()
        .find_map(|(x, y)| Some((find(x)?, find(y)?)))
    {
        Some((x, y)) => CsvGeometryIn::Xy {
            x,
            y,
            z: find("z").or_else(|| find("kot")),
        },
        None => CsvGeometryIn::None,
    }
}

/// The CSV at `path` (its bytes `bytes`) as one table, read the way `input` says; why not, as a sentence.
pub fn read(path: &std::path::Path, bytes: &[u8], input: &ImportCsv) -> Result<Read, String> {
    let file = path.file_name().map_or_else(
        || "tablo.csv".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let stem = path
        .file_stem()
        .map_or_else(|| "tablo".to_owned(), |n| n.to_string_lossy().into_owned());
    let table = read_with(bytes, Limits::PROCESSING);
    if let Some(problem) = table.problem {
        return Err(problem);
    }
    let sheet = table
        .sheets
        .into_iter()
        .next()
        .ok_or_else(|| "dosyada tablo yok".to_owned())?;
    let mut losses: Vec<String> = Vec::new();
    if sheet.cut {
        losses.push(format!(
            "dosyanın {} satırından sonrası okunmadı",
            kentos_formats::table_file::PROCESSING_ROWS
        ));
    }
    let mut rows = sheet.rows.into_iter();
    let header: Vec<String> = rows.next().ok_or_else(|| "dosya boş".to_owned())?;
    let rows: Vec<Vec<String>> = rows.collect();
    let names = column_names(&header);
    let find = |want: &str| names.iter().position(|n| n.eq_ignore_ascii_case(want));
    let need = |want: &str| {
        find(want).ok_or_else(|| {
            format!(
                "“{want}” sütunu dosyada yok (sütunlar: {})",
                names.join(", ")
            )
        })
    };
    // Where the geometry is.
    enum Geom {
        Wkt(usize),
        Xy(usize, usize, Option<usize>),
        None,
    }
    let geom = match input.geometry.as_ref().unwrap_or(&CsvGeometryIn::Auto) {
        CsvGeometryIn::Wkt { column } => Geom::Wkt(need(column)?),
        CsvGeometryIn::Xy { x, y, z } => Geom::Xy(
            need(x)?,
            need(y)?,
            match z {
                Some(z) => Some(need(z)?),
                None => None,
            },
        ),
        CsvGeometryIn::None => Geom::None,
        CsvGeometryIn::Auto => match WKT_NAMES.iter().find_map(|w| find(w)) {
            Some(i) => Geom::Wkt(i),
            None => match XY_NAMES
                .iter()
                .find_map(|(x, y)| Some((find(x)?, find(y)?)))
            {
                Some((x, y)) => Geom::Xy(x, y, find("z").or_else(|| find("kot"))),
                None => Geom::None,
            },
        },
    };
    let geometry_cols: Vec<usize> = match geom {
        Geom::Wkt(i) => vec![i],
        Geom::Xy(x, y, z) => [Some(x), Some(y), z].into_iter().flatten().collect(),
        Geom::None => Vec::new(),
    };
    // The types: the .csvt's, else worked out from the values.
    let csvt: Option<Vec<String>> = std::fs::read(path.with_extension("csvt"))
        .ok()
        .and_then(|b| {
            let t = String::from_utf8_lossy(&b).into_owned();
            let line = t.lines().next()?.to_owned();
            Some(
                line.split(',')
                    .map(|s| s.trim().trim_matches('"').to_owned())
                    .collect(),
            )
        });
    let csvt = match csvt {
        Some(t) if t.len() == names.len() => Some(t),
        Some(t) => {
            losses.push(format!(
                ".csvt {} tür söylüyor, dosyada {} sütun var: türler değerlerden bulundu",
                t.len(),
                names.len()
            ));
            None
        }
        None => None,
    };
    let mut columns: Vec<(usize, gpkg::Column)> = Vec::new();
    for (k, name) in names.iter().enumerate() {
        if geometry_cols.contains(&k) {
            continue;
        }
        let kind = csvt
            .as_ref()
            .and_then(|t| kind_of_csvt(&t[k]))
            .unwrap_or_else(|| {
                kind_of_values(rows.iter().map(|r| r.get(k).map_or("", |s| s.as_str())))
            });
        columns.push((
            k,
            gpkg::Column {
                name: name.clone(),
                decl: String::new(),
                kind,
                not_null: false,
                default: None,
            },
        ));
    }
    // The system: the .prj's, else the input's.
    let srid = match std::fs::read(path.with_extension("prj")) {
        Ok(b) => match crate::shapefile::srid_of_prj(&String::from_utf8_lossy(&b)) {
            Some(s) => Some(s),
            None => {
                losses.push(".prj'nin sistemi tanınmadı".into());
                input.srid
            }
        },
        Err(_) => input.srid,
    };
    let cell = |r: &Vec<String>, k: usize| r.get(k).map_or(String::new(), |s| s.trim().to_owned());
    let mut out_rows = Vec::with_capacity(rows.len());
    for (n, r) in rows.iter().enumerate() {
        // A row's number is its line's (the header is line 1).
        let fid = n as i64 + 2;
        let geometry = match geom {
            Geom::Wkt(i) => {
                let t = cell(r, i);
                (!t.is_empty()).then(|| read_wkt(&t).map_err(BlobError))
            }
            Geom::Xy(x, y, z) => {
                let (tx, ty) = (cell(r, x), cell(r, y));
                if tx.is_empty() && ty.is_empty() {
                    None
                } else {
                    let p = |t: &str| {
                        t.replace(',', ".")
                            .parse::<f64>()
                            .ok()
                            .filter(|v| v.is_finite())
                    };
                    Some(match (p(&tx), p(&ty)) {
                        (Some(x), Some(y)) => Ok(Geometry::Point(Some(Coord {
                            x,
                            y,
                            z: z.and_then(|z| p(&cell(r, z))),
                            m: None,
                        }))),
                        _ => Err(BlobError(format!("X “{tx}”, Y “{ty}” sayı değil"))),
                    })
                }
            }
            Geom::None => None,
        };
        let values = columns
            .iter()
            .map(|(k, c)| value_of(&cell(r, *k), &c.kind))
            .collect();
        out_rows.push(gpkg::Row {
            fid,
            geometry,
            values,
        });
    }
    let layer = gpkg::Layer {
        name: stem.clone(),
        data_type: if geometry_cols.is_empty() {
            gpkg::DataType::Attributes
        } else {
            gpkg::DataType::Features
        },
        identifier: None,
        description: None,
        bounds: None,
        srs_id: srid.map(|s| s as i32),
        geometry: None,
        fid: "fid".into(),
        columns: columns.into_iter().map(|(_, c)| c).collect(),
        indexed: false,
    };
    let key = format!("csv/{file}");
    let mut t = geopackage::table_of_rows(&layer, &out_rows, &HashMap::new(), &|fid| {
        geopackage::row_uid(&key, fid)
    })?;
    t.losses.extend(losses);
    let mut systems = BTreeMap::new();
    if let Some(s) = srid {
        systems.insert(s as i32, Some(("EPSG".to_owned(), s as i32)));
    }
    Ok(Read {
        file,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(bytes),
        tables: vec![t],
        systems,
    })
}
