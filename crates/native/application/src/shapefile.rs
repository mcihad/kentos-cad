//! The drawing's layers as Shapefiles (docs/adr/0514, plan/gis.md CBS-05): what `file.export.shp` hands the formats
//! core's writer (`kentos_formats::shp::write`): the layers (a group gives its layers), their fields and objects in
//! the drawing's order, the blocks the objects place, and the project's system as the .prj: a registry system's ESRI
//! WKT 1 as GDAL writes it (fixtures/crs/v1/esri-prj.json, `scripts/fixtures/esri_prj.py`), which ArcGIS and QGIS read
//! back to its EPSG code; a project's own definition as OGC WKT 1 (read as a custom system); none for a local project.

use kentos_contracts::{BlockDefinition, Entity, LayerNodeType, ShpEncoding};
use kentos_domain::Document;
use kentos_formats::shp::write::{self, DbfEncoding, Input, Layer, Written};

/// The layers to write, by id: `wanted` (a group gives its layers), every layer when empty; the first of a layer named
/// twice kept. An id that is no layer of the drawing is the first error.
pub fn layer_ids(doc: &Document, wanted: &[String]) -> Result<Vec<String>, (usize, String)> {
    if wanted.is_empty() {
        return Ok(doc.layers().leaves().iter().map(|n| n.id.clone()).collect());
    }
    let mut ids: Vec<String> = Vec::new();
    for (i, id) in wanted.iter().enumerate() {
        let Some(node) = doc.layers().get(id) else {
            return Err((i, id.clone()));
        };
        let found: Vec<String> = match node.kind {
            LayerNodeType::Layer => vec![id.clone()],
            LayerNodeType::Group => doc
                .layers()
                .leaves_of(id)
                .iter()
                .map(|n| n.id.clone())
                .collect(),
        };
        for f in found {
            if !ids.contains(&f) {
                ids.push(f);
            }
        }
    }
    Ok(ids)
}

/// How the .prj names the system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrjKind {
    /// A registry system, as ESRI's WKT 1: read back to its EPSG code.
    Esri,
    /// The project's own definition, as OGC WKT 1: read as a custom system.
    Definition,
}

/// The EPSG code a `.prj` (a Shapefile's, a CSV's) names: one of the registry's ESRI texts word for word, or an
/// `AUTHORITY["EPSG", n]` of the whole system (its last); none when it names none (docs/adr/0522).
pub fn srid_of_prj(text: &str) -> Option<u32> {
    let t = text.trim();
    if let Some(found) = esri_table().iter().find(|(_, v)| v.trim() == t) {
        return found.0.parse().ok();
    }
    let at = t.rfind("AUTHORITY[")?;
    let rest = &t[at + "AUTHORITY[".len()..];
    let (org, code) = rest.split_once(',')?;
    if org.trim().trim_matches('"') != "EPSG" {
        return None;
    }
    code.split(']').next()?.trim().trim_matches('"').parse().ok()
}

#[derive(serde::Deserialize)]
struct PrjFile {
    prj: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    wkt1: std::collections::BTreeMap<String, String>,
}

fn prj_file() -> &'static PrjFile {
    use std::sync::OnceLock;
    static FILE: OnceLock<PrjFile> = OnceLock::new();
    FILE.get_or_init(|| {
        serde_json::from_str::<PrjFile>(include_str!("../../../../fixtures/crs/v1/esri-prj.json"))
            .unwrap_or(PrjFile {
                prj: Default::default(),
                wkt1: Default::default(),
            })
    })
}

fn esri_table() -> &'static std::collections::BTreeMap<String, String> {
    &prj_file().prj
}

/// The project's system as GDAL's CSV driver writes a `.prj`, OGC WKT 1 with its EPSG authority (docs/adr/0522); the
/// project's own definition as OGC WKT 1.
pub fn wkt1_prj(doc: &Document) -> Option<String> {
    let settings = doc.settings();
    if let Some(text) = prj_file().wkt1.get(&settings.srid.to_string()) {
        return Some(text.clone());
    }
    prj(doc).map(|(t, _)| t)
}

/// The registry systems' ESRI WKT 1, by EPSG code.
fn esri(srid: u32) -> Option<String> {
    esri_table().get(&srid.to_string()).cloned()
}

/// The project's system as a .prj's WKT 1, and how it names it.
pub fn prj(doc: &Document) -> Option<(String, PrjKind)> {
    let settings = doc.settings();
    if let Some(text) = esri(settings.srid) {
        return Some((text, PrjKind::Esri));
    }
    let named = kentos_project::systems::own(settings)?;
    let core = named.system?;
    kentos_geometry_core::crs::text::write_wkt(&named.name, &core, Some("TUREF"))
        .map(|t| (t, PrjKind::Definition))
}

/// Writes the layers `ids` as Shapefiles (bytes in memory).
pub fn write(doc: &Document, ids: &[String], encoding: ShpEncoding) -> Result<Written, String> {
    let blocks: Vec<BlockDefinition> = doc.blocks().iter().map(|b| (**b).clone()).collect();
    let mut layers = Vec::new();
    for id in ids {
        let Some(node) = doc.layers().get(id) else {
            continue;
        };
        let objects: Vec<&Entity> = doc.by_layer_with_uids(id).map(|(_, e)| e).collect();
        if objects.is_empty() {
            continue;
        }
        layers.push(Layer {
            id,
            name: &node.name,
            fields: &node.fields,
            objects,
        });
    }
    write::write(&Input {
        layers,
        blocks: &blocks,
        prj: prj(doc).map(|(text, _)| text),
        encoding: match encoding {
            ShpEncoding::Utf8 => DbfEncoding::Utf8,
            ShpEncoding::Windows1254 => DbfEncoding::Windows1254,
        },
    })
}
