//! A FlatGeobuf of objects (docs/adr/0544, plan/gis.md CBS-05): what `file.export.fgb` writes and `file.import.fgb`
//! reads (`kentos_geopackage::fgb`). An object is a feature: its geometry as the GeoPackage export makes it (arcs
//! exact), its attributes the properties. Reading goes through the GeoPackage import's rows
//! (`geopackage::table_of_rows`): the same objects, fields, lasting ids and losses as a GeoPackage's table.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{Entity, ExportFgb, FgbExported, ImportFgb};
use kentos_domain::Document;
use kentos_geopackage as gpkg;
use kentos_geopackage::fgb::{self, ColumnType, FgbColumn};
use kentos_geopackage::{ColumnKind, Geometry, Value};

use crate::geopackage::{self, Read};

/// What `file.export.fgb` makes before it writes: the file's bytes and the answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    pub bytes: Vec<u8>,
    pub output: FgbExported,
}

/// GDAL's name of a layer's geometry type, as `ogrinfo` says it.
fn type_word(gs: &[&Geometry]) -> String {
    let word = |g: &Geometry| -> &'static str {
        match g {
            Geometry::Point(_) => "Point",
            Geometry::MultiPoint(_) => "Multi Point",
            Geometry::Curve(gpkg::Curve::Line(_)) => "Line String",
            Geometry::Curve(gpkg::Curve::Arc(_)) => "Circular String",
            Geometry::Curve(gpkg::Curve::Compound(_)) => "Compound Curve",
            Geometry::Polygon(r) if r.iter().all(|c| matches!(c, gpkg::Curve::Line(_))) => "Polygon",
            Geometry::Polygon(_) => "Curve Polygon",
            Geometry::MultiCurve(c) if c.iter().all(|c| matches!(c, gpkg::Curve::Line(_))) => {
                "Multi Line String"
            }
            Geometry::MultiCurve(_) => "Multi Curve",
            Geometry::MultiPolygon(p)
                if p.iter().flatten().all(|c| matches!(c, gpkg::Curve::Line(_))) =>
            {
                "Multi Polygon"
            }
            Geometry::MultiPolygon(_) => "Multi Surface",
            Geometry::Collection(_) => "Geometry Collection",
        }
    };
    let words: BTreeSet<&str> = gs.iter().map(|g| word(g)).collect();
    match (words.len(), words.first()) {
        (1, Some(w)) => (*w).to_owned(),
        _ => "Unknown".into(),
    }
}

/// The layers' objects as a FlatGeobuf, or why none can be written: (code, message, path).
pub fn export(doc: &Document, input: &ExportFgb) -> Result<Made, (String, String, String)> {
    let ids = crate::shapefile::layer_ids(doc, &input.layers).map_err(|(i, id)| {
        (
            kentos_commands::codes::LAYER_NOT_FOUND.to_owned(),
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            format!("layers[{i}]"),
        )
    })?;
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
    // The columns: the layers' fields in order, then the attributes no field names, by name.
    let mut fields: Vec<(String, ColumnKind, Option<u32>)> = Vec::new();
    let mut lost: BTreeSet<String> = BTreeSet::new();
    for id in &used {
        if let Some(node) = doc.layers().get(id) {
            for f in &node.fields {
                let c = geopackage::column_of(f);
                match fields
                    .iter_mut()
                    .find(|(n, _, _)| n.eq_ignore_ascii_case(&f.name))
                {
                    Some(have) if ColumnType::for_kind(&have.1) != ColumnType::for_kind(&c.kind) => {
                        lost.insert(format!(
                            "“{}” alanı katmanlarda farklı türlerde: metin sütunu olarak yazıldı",
                            f.name
                        ));
                        have.1 = ColumnKind::Text(None);
                        have.2 = None;
                    }
                    Some(_) => {}
                    None => fields.push((f.name.clone(), c.kind, f.length)),
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
        fields.push((k, ColumnKind::Text(None), None));
    }
    let layered = used.len() > 1;
    let mut columns: Vec<FgbColumn> = Vec::new();
    if layered {
        columns.push(FgbColumn {
            name: "katman".into(),
            kind: ColumnType::String,
            width: None,
        });
    }
    let mut sources: Vec<(String, ColumnKind)> = Vec::new();
    for (name, kind, length) in &fields {
        let mut written = name.clone();
        while columns.iter().any(|c| c.name.eq_ignore_ascii_case(&written)) {
            lost.insert(format!(
                "“{written}” alanı “{written}_” sütunu olarak yazıldı (adı başka bir sütunun)"
            ));
            written.push('_');
        }
        let kind_out = ColumnType::for_kind(kind);
        columns.push(FgbColumn {
            name: written,
            kind: kind_out,
            width: if kind_out == ColumnType::String { *length } else { None },
        });
        sources.push((name.clone(), kind.clone()));
    }
    if fields.iter().any(|(_, k, _)| *k == ColumnKind::Date) {
        lost.insert("tarih alanları tarih-saat sütunu olarak yazıldı (değerleri yalnız gün)".into());
    }
    let mut features: Vec<(Option<Geometry>, Vec<Value>)> = Vec::new();
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
        if let Some((_, true)) = &geometry {
            chords += 1;
        }
        let attrs = &e.base().attrs;
        let mut values = Vec::with_capacity(columns.len());
        if layered {
            values.push(Value::Text(
                doc.layers()
                    .get(layer)
                    .map(|n| n.name.clone())
                    .unwrap_or_default(),
            ));
        }
        for (name, kind) in &sources {
            let v = attrs.get(name).or_else(|| {
                attrs
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(name))
                    .map(|(_, v)| v)
            });
            values.push(geopackage::cell(v, kind));
        }
        features.push((geometry.map(|(g, _)| g), values));
    }
    if features.is_empty() {
        return Err((
            kentos_commands::codes::NOTHING_TO_EXPORT.to_owned(),
            "Katmanlarda FlatGeobuf'a yazılacak nesne yok.".into(),
            "layers".into(),
        ));
    }
    if chords > 0 {
        lost.insert(format!(
            "{chords} eğri (elips, eğri, NURBS) FlatGeobuf'ta karşılığı olmadığından kirişlerle yazıldı"
        ));
    }
    if !left.is_empty() {
        lost.insert(format!(
            "FlatGeobuf'ta karşılığı olmayan nesneler yazılmadı: {}",
            left.iter()
                .map(|(k, n)| format!("{n} {k}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let srid = doc.settings().srid;
    let srid = (srid != 0).then_some(srid);
    if srid.is_none() {
        lost.insert("projenin kayıtlı bir koordinat sistemi yok: dosya sistemsiz yazıldı".into());
    }
    let name = std::path::Path::new(&input.path)
        .file_stem()
        .map_or_else(|| "katman".to_owned(), |n| n.to_string_lossy().into_owned());
    let bytes = fgb::write(&fgb::Layer {
        name: &name,
        srid,
        columns: &columns,
        features: &features,
    });
    let geometries: Vec<&Geometry> = features.iter().filter_map(|(g, _)| g.as_ref()).collect();
    let output = FgbExported {
        path: input.path.clone(),
        name,
        features: features.len() as u32,
        columns: columns.iter().map(|c| c.name.clone()).collect(),
        types: columns.iter().map(|c| c.kind.word().to_owned()).collect(),
        geometry: if geometries.is_empty() {
            "None".into()
        } else {
            type_word(&geometries)
        },
        indexed: features
            .iter()
            .all(|(g, _)| g.as_ref().and_then(fgb::bounds).is_some()),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        left,
        lost: lost.into_iter().collect(),
    };
    Ok(Made { bytes, output })
}

/// The file's layer as the GeoPackage import reads a table.
pub fn read(path: &std::path::Path, bytes: &[u8], input: &ImportFgb) -> Result<Read, String> {
    let file = path.file_name().map_or_else(
        || "katman.fgb".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let stem = path
        .file_stem()
        .map_or_else(|| "katman".to_owned(), |n| n.to_string_lossy().into_owned());
    let r = fgb::read(bytes)?;
    let mut losses: Vec<String> = Vec::new();
    if let Some(other) = &r.other_crs {
        losses.push(format!("dosyanın sistemi ({other}) bir EPSG kodu değil; tanınmadı"));
    }
    let srid = r.srid.or(input.srid);
    let has_geometry = r.features.iter().any(|(g, _)| g.is_some());
    let rows: Vec<gpkg::Row> = r
        .features
        .into_iter()
        .enumerate()
        .map(|(i, (geometry, values))| gpkg::Row {
            fid: i as i64 + 1,
            geometry,
            values,
        })
        .collect();
    let name = if r.name.trim().is_empty() { stem } else { r.name };
    let layer = gpkg::Layer {
        name,
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
        columns: r.columns,
        indexed: r.indexed,
    };
    let key = format!("fgb/{file}");
    let mut t = geopackage::table_of_rows(&layer, &rows, &HashMap::new(), &|fid| {
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
