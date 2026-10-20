//! `file.export.maplibre` (docs/adr/0560): a layer's look, or the visible layers', as a MapLibre GL style, with each
//! layer's objects as a WGS 84 GeoJSON beside it (or a source URL given instead). The bytes are made first (`plan`
//! stops there); `execute` writes the GeoJSON files and then the style, each whole or not at all
//! (`files::write_bytes`), so a style is never there without its data.

use std::path::{Path, PathBuf};

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    CommandWarning, Entity, ExportMaplibre, GeoJsonLayer, GeoJsonWriteInput, MaplibreData,
    MaplibreExported, MaplibreSourceKind, codes,
};
use kentos_domain::Document;
use kentos_native_application::maplibre::{self, Picked};
use serde_json::{Map, json};

use crate::file_commands::{Refused, refuse, target};

/// MapLibre's own demo glyph server, for a style with labels and no glyph URL given.
pub const DEMO_GLYPHS: &str = "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf";

/// The files made: the GeoJSON first, the style last.
pub struct Made {
    pub files: Vec<(PathBuf, Vec<u8>)>,
    pub output: MaplibreExported,
    pub warnings: Vec<CommandWarning>,
}

/// How a layer is bound to its source: the source's id and, for vector tiles, the tile layer.
type Bind = Box<dyn Fn(&Picked) -> (String, Option<String>)>;

/// A layer id as a file name part: letters, digits, `-` and `_`; the rest `_`.
fn safe(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn vector_url(url: &str, kind: Option<MaplibreSourceKind>) -> bool {
    match kind {
        Some(MaplibreSourceKind::Vector) => true,
        Some(MaplibreSourceKind::Geojson) => false,
        None => url.contains("{z}") || url.ends_with(".pbf") || url.ends_with(".mvt"),
    }
}

pub fn make(doc: &Document, input: &ExportMaplibre) -> Result<Made, Refused> {
    let (path, _) = target(&input.path, input.overwrite)?;
    let selection = maplibre::pick(doc, input.layer.as_deref())?;
    let picked = selection.picked;
    if picked.is_empty() {
        return Err(refuse(
            codes::NOTHING_TO_EXPORT,
            "Görünen bir katman yok; yazılacak bir şey kalmadı. Bir katmanı görünür yapın ya da layer verin.",
            "layer",
        ));
    }
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "stil".to_owned());
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let name = if picked.len() == 1 {
        picked[0].name.clone()
    } else {
        doc.name().trim_end_matches(".kcad").to_owned()
    };
    let name = if name.is_empty() { stem.clone() } else { name };
    let mut sources = Map::new();
    let mut files: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    let mut data: Vec<MaplibreData> = Vec::new();
    let mut data_lost: Vec<String> = Vec::new();
    if !selection.empty.is_empty() {
        data_lost.push(format!(
            "{} görünen katmanda çizilecek nokta, çizgi ya da alan yok, yazılmadı: {}",
            selection.empty.len(),
            selection.empty.join(", ")
        ));
    }
    let bind: Bind;
    match &input.source_url {
        Some(url) => {
            let vector = vector_url(url, input.source_kind);
            if !vector && picked.len() > 1 {
                return Err(refuse(
                    codes::INVALID_INPUT,
                    "Bir GeoJSON adresi tek katmanın nesneleridir; birden çok katman için vektör karo adresi (sourceKind vector) verin ya da layer ile bir katmanı seçin.",
                    "sourceUrl",
                ));
            }
            if vector {
                let source = if url.contains("{z}") {
                    json!({ "type": "vector", "tiles": [url] })
                } else {
                    json!({ "type": "vector", "url": url })
                };
                sources.insert("kentos".into(), source);
                bind = Box::new(|p: &Picked| ("kentos".to_owned(), Some(p.name.clone())));
            } else {
                let id = picked[0].id.clone();
                sources.insert(id.clone(), json!({ "type": "geojson", "data": url }));
                bind = Box::new(move |_: &Picked| (id.clone(), None));
            }
        }
        None => {
            let settings = doc.settings();
            let from = if settings.srid == 4326 {
                None
            } else {
                let system = kentos_project::systems::own(settings).and_then(|n| n.system);
                match system {
                    Some(s) => Some((s, kentos_project::systems::choices(settings))),
                    None => {
                        return Err(refuse(
                            codes::NO_SYSTEM,
                            "Projenin koordinat sistemi yok (ya da WGS 84'e yolu yok); nesneler GeoJSON'a enlem ve boylam olarak yazılamaz. Projeye bir koordinat sistemi verin ya da sourceUrl ile kaynağı adresle bildirin.",
                            "sourceUrl",
                        ));
                    }
                }
            };
            let blocks: Vec<_> = doc.blocks().iter().map(|b| (**b).clone()).collect();
            for p in &picked {
                let Some(node) = doc.layers().get(&p.id) else {
                    continue;
                };
                let entities: Vec<Entity> = doc
                    .entities()
                    .filter(|e| e.base().layer_id == p.id)
                    .cloned()
                    .collect();
                let write_input = GeoJsonWriteInput {
                    entities,
                    layers: vec![GeoJsonLayer {
                        id: p.id.clone(),
                        name: p.name.clone(),
                        fields: node.fields.clone(),
                    }],
                    srid: settings.srid,
                    name: p.name.clone(),
                    blocks: blocks.clone(),
                };
                let (bytes, report) = match &from {
                    Some((system, choices)) => {
                        kentos_formats::geojson::write_wgs84(&write_input, system, choices)
                    }
                    None => kentos_formats::geojson::write(&write_input),
                };
                let file = if picked.len() == 1 {
                    format!("{stem}.geojson")
                } else {
                    format!("{stem}.{}.geojson", safe(&p.id))
                };
                let file_path = dir.join(&file);
                if file_path.exists() && !input.overwrite {
                    return Err(refuse(
                        codes::FILE_EXISTS,
                        format!(
                            "{} var; üzerine yazmak için overwrite: true verin.",
                            file_path.display()
                        ),
                        "path",
                    ));
                }
                for item in &report.skipped {
                    data_lost.push(format!(
                        "“{}” katmanı: {} ({} nesne) GeoJSON'a yazılmadı: {}",
                        p.name, item.what, item.count, item.reason
                    ));
                }
                if maplibre::curve_areas(doc, &p.id) {
                    data_lost.push(format!(
                        "“{}” katmanı: daire, elips ve taramalar GeoJSON'a çizgi olarak yazılır; alan dolgusu onları boyamaz, kenarları çizilir",
                        p.name
                    ));
                }
                sources.insert(p.id.clone(), json!({ "type": "geojson", "data": file }));
                data.push(MaplibreData {
                    layer: p.id.clone(),
                    path: file_path.display().to_string(),
                    features: report.counts.values().sum(),
                    bytes: bytes.len() as u64,
                    sha256: sha256_hex(&bytes),
                });
                files.push((file_path, bytes));
            }
            bind = Box::new(|p: &Picked| (p.id.clone(), None));
        }
    }
    let glyphs = input.glyphs.as_deref().unwrap_or(DEMO_GLYPHS);
    let mut written = maplibre::export(doc, &picked, &name, &sources, &*bind, Some(glyphs))?;
    written.lost.extend(data_lost);
    let bytes = written.json.clone().into_bytes();
    let warnings = if written.lost.is_empty() {
        Vec::new()
    } else {
        vec![CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: format!(
                "MapLibre stilinin söyleyemediği: {}.",
                written.lost.join("; ")
            ),
            path: Some("layer".into()),
        }]
    };
    let output = MaplibreExported {
        path: path.display().to_string(),
        layers: picked.iter().map(|p| p.id.clone()).collect(),
        style_layers: written.layers as u32,
        rules: written.rules as u32,
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
        data,
        lost: written.lost,
    };
    files.push((path, bytes));
    Ok(Made {
        files,
        output,
        warnings,
    })
}

/// Writes the files made, the GeoJSON first.
pub fn write(made: &Made) -> Result<(), Refused> {
    for (path, bytes) in &made.files {
        crate::file_commands::write(path, bytes)?;
    }
    Ok(())
}
