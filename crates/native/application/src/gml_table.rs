//! A GML of objects (docs/adr/0545, plan/gis.md CBS-05): what `file.export.gml` writes and `file.import.gml` reads
//! (`gml_write`, `gml_read`). Each layer is a feature type, an object a feature: its geometry as the GeoPackage export
//! makes it (arcs exact), its attributes the properties. Reading goes through the GeoPackage import's rows
//! (`geopackage::table_of_rows`): the same objects, fields, lasting ids and losses as a GeoPackage's table.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    Entity, ExportGml, GmlExported, GmlFileWritten, GmlLayerWritten, ImportGml,
};
use kentos_domain::Document;
use kentos_geopackage as gpkg;
use kentos_geopackage::{ColumnKind, Geometry, Value};

use crate::geopackage::{self, Read};
use crate::gml_read;
use crate::gml_write::{self, Notes, OutColumn, OutType};

/// What `file.export.gml` makes before it writes: the files (name and bytes) and the answer.
#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    /// The schema first, the GML last: the order they are written in.
    pub files: Vec<(String, Vec<u8>)>,
    pub output: GmlExported,
}

fn unique(taken: &mut Vec<String>, want: String) -> String {
    let mut name = want.clone();
    let mut k = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
        name = format!("{want}_{k}");
        k += 1;
    }
    taken.push(name.clone());
    name
}

/// The layers' objects as GML 3.2, or why none can be written: (code, message, path).
pub fn export(doc: &Document, input: &ExportGml) -> Result<Made, (String, String, String)> {
    let ids = crate::shapefile::layer_ids(doc, &input.layers).map_err(|(i, id)| {
        (
            kentos_commands::codes::LAYER_NOT_FOUND.to_owned(),
            format!("“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."),
            format!("layers[{i}]"),
        )
    })?;
    let mut lost: BTreeSet<String> = BTreeSet::new();
    let mut left: BTreeMap<String, u32> = BTreeMap::new();
    let mut chords = 0u32;
    let mut types: Vec<OutType> = Vec::new();
    let mut written: Vec<GmlLayerWritten> = Vec::new();
    let mut type_names: Vec<String> = Vec::new();
    for id in &ids {
        let objects: Vec<&Entity> = doc.by_layer(id).collect();
        if objects.is_empty() {
            continue;
        }
        let node = doc.layers().get(id);
        // The properties: the layer's fields in order, then the attributes no field names, by name.
        let mut fields: Vec<(String, ColumnKind)> = Vec::new();
        if let Some(node) = node {
            for f in &node.fields {
                if !fields.iter().any(|(n, _)| n.eq_ignore_ascii_case(&f.name)) {
                    fields.push((f.name.clone(), geopackage::column_of(f).kind));
                }
            }
        }
        let mut others: BTreeSet<String> = BTreeSet::new();
        for e in &objects {
            for k in e.base().attrs.keys() {
                if !fields.iter().any(|(n, _)| n.eq_ignore_ascii_case(k)) {
                    others.insert(k.clone());
                }
            }
        }
        fields.extend(others.into_iter().map(|k| (k, ColumnKind::Text(None))));
        let layer_name = node.map_or_else(|| id.clone(), |n| n.name.clone());
        let name = unique(&mut type_names, gml_write::xml_name(&layer_name));
        if name != layer_name {
            lost.insert(format!(
                "“{layer_name}” katmanının öğe adı XML adı olarak “{name}” yazıldı"
            ));
        }
        let mut props: Vec<String> = vec!["geometryProperty".to_owned()];
        let mut columns: Vec<OutColumn> = Vec::new();
        let mut sources: Vec<(String, ColumnKind)> = Vec::new();
        for (field, kind) in &fields {
            let want = gml_write::xml_name(field);
            let written_name = unique(&mut props, want);
            if written_name != *field {
                lost.insert(format!(
                    "“{field}” alanı “{written_name}” özelliği olarak yazıldı (XML adı ya da başka bir özelliğin adı)"
                ));
            }
            columns.push(OutColumn {
                name: written_name,
                kind: kind.clone(),
            });
            sources.push((field.clone(), kind.clone()));
        }
        let mut features: Vec<(Option<Geometry>, Vec<Value>)> = Vec::new();
        for e in &objects {
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
            let values = sources
                .iter()
                .map(|(n, kind)| {
                    let v = attrs.get(n).or_else(|| {
                        attrs
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case(n))
                            .map(|(_, v)| v)
                    });
                    geopackage::cell(v, kind)
                })
                .collect();
            features.push((geometry.map(|(g, _)| g), values));
        }
        if features.is_empty() {
            continue;
        }
        written.push(GmlLayerWritten {
            name: name.clone(),
            layer: id.clone(),
            features: features.len() as u32,
            columns: columns.iter().map(|c| c.name.clone()).collect(),
            types: columns
                .iter()
                .map(|c| gml_write::type_word(&c.kind).to_owned())
                .collect(),
            geometry: {
                let geoms: Vec<&Geometry> = features.iter().filter_map(|(g, _)| g.as_ref()).collect();
                if geoms.is_empty() {
                    "None".to_owned()
                } else {
                    gml_write::property_type(&geoms).to_owned()
                }
            },
        });
        types.push(OutType {
            name,
            columns,
            features,
        });
    }
    if types.is_empty() {
        return Err((
            kentos_commands::codes::NOTHING_TO_EXPORT.to_owned(),
            "Katmanlarda GML'e yazılacak nesne yok.".into(),
            "layers".into(),
        ));
    }
    if chords > 0 {
        lost.insert(format!(
            "{chords} eğri (elips, eğri, NURBS) GML'de karşılığı olmadığından kirişlerle yazıldı"
        ));
    }
    if !left.is_empty() {
        lost.insert(format!(
            "GML'de karşılığı olmayan nesneler yazılmadı: {}",
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
    let stem = Path::new(&input.path)
        .file_stem()
        .map_or_else(|| "katman".to_owned(), |n| n.to_string_lossy().into_owned());
    let xsd_name = format!("{stem}.xsd");
    let mut notes = Notes::default();
    let gml = gml_write::write(&types, srid, input.schema.then_some(xsd_name.as_str()), &mut notes);
    if notes.zero_z > 0 {
        lost.insert(format!(
            "{} konumun kotu yok: kotlu geometrilerde GML kotsuz konum tutmadığından 0 yazıldı",
            notes.zero_z
        ));
    }
    if notes.with_m > 0 {
        lost.insert(format!(
            "{} geometrinin M değerleri GML'de karşılığı olmadığından yazılmadı",
            notes.with_m
        ));
    }
    if notes.control > 0 {
        lost.insert(format!(
            "{} denetim karakteri XML'de bulunamayacağından metinlerden atıldı",
            notes.control
        ));
    }
    if notes.non_finite > 0 {
        lost.insert(format!(
            "{} sonlu olmayan sayı boş (xsi:nil) yazıldı",
            notes.non_finite
        ));
    }
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    if input.schema {
        let geoms: Vec<String> = written.iter().map(|w| w.geometry.clone()).collect();
        let geoms: Vec<&str> = geoms.iter().map(|g| if g == "None" { "" } else { g.as_str() }).collect();
        files.push((xsd_name, gml_write::schema(&types, &geoms).into_bytes()));
    }
    let gml_name = Path::new(&input.path)
        .file_name()
        .map_or_else(|| "katman.gml".to_owned(), |n| n.to_string_lossy().into_owned());
    files.push((gml_name, gml.into_bytes()));
    let output = GmlExported {
        path: input.path.clone(),
        features: written.iter().map(|w| w.features).sum(),
        layers: written,
        files: files
            .iter()
            .rev()
            .map(|(name, bytes)| GmlFileWritten {
                name: name.clone(),
                bytes: bytes.len() as u64,
                sha256: sha256_hex(bytes),
            })
            .collect(),
        left,
        lost: lost.into_iter().collect(),
    };
    Ok(Made { files, output })
}

/// The file's feature types as the GeoPackage import reads tables; the schema and `.gfs` beside `path` are read when
/// they are there.
pub fn read(path: &Path, bytes: &[u8], input: &ImportGml) -> Result<Read, String> {
    let file = path.file_name().map_or_else(
        || "katman.gml".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    if bytes.len() > gml_read::MAX_BYTES {
        return Err(format!(
            "dosya {} MiB'tan büyük; GML bu sınırın altındaki dosyalar okunur",
            gml_read::MAX_BYTES >> 20
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "UTF-8 olarak okunamadı".to_owned())?;
    let side = |ext: &str| std::fs::read_to_string(path.with_extension(ext)).ok();
    let (xsd, gfs) = (side("xsd"), side("gfs"));
    let mut parsed = gml_read::parse(text, xsd.as_deref(), gfs.as_deref())?;
    let mut losses: Vec<String> = parsed.losses.iter().cloned().collect();
    if let Some(other) = &parsed.other_crs
        && parsed.srid.is_none()
    {
        losses.push(format!("dosyanın sistemi ({other}) bir EPSG kodu değil; tanınmadı"));
    }
    let srid = parsed.srid.or(input.srid);
    let wanted: Vec<&String> = input.tables.iter().collect();
    for t in &wanted {
        if !parsed.tables.iter().any(|p| &p.name == *t) {
            return Err(format!("“{t}” tablosu dosyada yok"));
        }
    }
    parsed
        .tables
        .retain(|t| wanted.is_empty() || wanted.iter().any(|w| **w == t.name));
    let mut tables = Vec::new();
    for t in parsed.tables {
        let has_geometry = t.rows.iter().any(|r| r.geometry.is_some());
        let layer = gpkg::Layer {
            name: t.name.clone(),
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
            columns: t.columns,
            indexed: false,
        };
        let key = format!("gml/{file}/{}", t.name);
        let mut read = geopackage::table_of_rows(&layer, &t.rows, &HashMap::new(), &|fid| {
            geopackage::row_uid(&key, fid)
        })?;
        read.losses.extend(losses.iter().cloned());
        tables.push(read);
    }
    let mut systems = BTreeMap::new();
    if let Some(s) = srid {
        systems.insert(s as i32, Some(("EPSG".to_owned(), s as i32)));
    }
    Ok(Read {
        file,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(bytes),
        tables,
        systems,
        verticals: BTreeMap::new(),
    })
}
