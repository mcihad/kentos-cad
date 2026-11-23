//! A layer's kind and what a CBS project's file may hold (docs/adr/9195 §5, 9196 §2, 9197 §2.4). The kind is the
//! shared module definition's (`kentos_launch::LayerKind`): a group, a service's, a data layer, a bound one, an
//! annotation layer, one of rasters or of point clouds alone, else a drawing layer. The layer menu, the save of every
//! client (the desktop, Python, MCP) and its refusal ask this one place.

use kentos_commands::{LayerGeometry, LayerNode, LayerNodeType, Workspace};
use kentos_domain::Document;
pub use kentos_launch::{LayerKind, Mobile, Storage};

/// What `node` is.
pub fn of(doc: &Document, node: &LayerNode) -> LayerKind {
    if node.kind == LayerNodeType::Group {
        return LayerKind::Group;
    }
    if node.service.is_some() {
        return LayerKind::Service;
    }
    if node.data.is_some() {
        return LayerKind::Data;
    }
    if doc.settings().sources.iter().any(|s| s.layer == node.id) {
        return LayerKind::Bound;
    }
    if node.geometry == Some(LayerGeometry::Annotation) {
        return LayerKind::Annotation;
    }
    match alone(doc, &node.id) {
        Some("raster") => LayerKind::Raster,
        Some("pointcloud") => LayerKind::PointCloud,
        _ => LayerKind::Drawing,
    }
}

/// Where the project keeps its data (docs/adr/9196 §2, CLAUDE.md §25 rule 5): a CBS project's layers in a database,
/// any other project whole in its file.
pub fn storage(doc: &Document) -> Storage {
    match doc.settings().project_type() {
        Some(Workspace::Gis) => Storage::Database,
        _ => Storage::File,
    }
}

/// What `node` may be on the mobile clients here (docs/adr/9198 §2.2): to edit a CBS project's data layer alone.
pub fn mobile_choices(doc: &Document, node: &LayerNode) -> &'static [Mobile] {
    of(doc, node).mobile_choices(storage(doc))
}

/// `raster` or `pointcloud` when `layer` holds those alone: its first object decides unless it is one of them, so a
/// large layer of lines costs one object.
pub fn alone(doc: &Document, layer: &str) -> Option<&'static str> {
    let mut objects = doc.by_layer(layer);
    let first = objects.next()?.kind();
    (matches!(first, "raster" | "pointcloud") && objects.all(|e| e.kind() == first)).then_some(first)
}

/// A layer whose objects a CBS project's file would hold: its id and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub id: String,
    pub name: String,
}

/// The layers of a CBS project whose objects would go into its project file (docs/adr/9196 §2, CLAUDE.md §25 rule 5):
/// a drawing layer's or a bound one's with objects, in the tree's order. The annotation, data and service layers,
/// rasters and point clouds (their pixels and points in files of their own) are the file's to keep; a CAD project's
/// file keeps everything.
pub fn held_in_file(doc: &Document) -> Vec<Held> {
    if doc.settings().project_type() != Some(Workspace::Gis) {
        return Vec::new();
    }
    doc.layers()
        .leaves()
        .into_iter()
        .filter(|n| matches!(of(doc, n), LayerKind::Drawing | LayerKind::Bound) && doc.count(&n.id) > 0)
        .map(|n| Held { id: n.id.clone(), name: n.name.clone() })
        .collect()
}

/// Why a CBS project is not written to its file: the layers named and the way out said.
pub fn held_words(held: &[Held]) -> String {
    let names: Vec<String> = held.iter().map(|h| format!("“{}”", h.name)).collect();
    format!(
        "Kaydedilmedi: bu bir CBS projesi, verisi veri kaynağının tablolarında durur ve proje dosyası yalnız yapıyı, \
         stilleri, ayarları ve açıklamaları tutar. Nesneleri proje dosyasında duran katman: {}. Katmanı veri kaynağına \
         taşıyın (cad.layers.migrate; masaüstünde Katmanlar panelinde sağ tık ▸ Veri kaynağına taşı…), yazı ve ölçüler \
         içinse katmanı Açıklama türüne çevirin; sonra kaydedin.",
        names.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::DocumentSnapshotV2;
    use serde_json::json;

    fn layer(id: &str, geometry: Option<&str>) -> serde_json::Value {
        let mut l = json!({
            "id": id, "name": id, "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": []
        });
        if let Some(g) = geometry {
            l["geometry"] = json!(g);
        }
        l
    }

    fn doc(workspace: &str) -> Document {
        let line = |id: u32, layer: &str| {
            json!({ "kind": "line", "id": id, "layerId": layer, "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 1.0, "y": 1.0} })
        };
        let snapshot: DocumentSnapshotV2 = serde_json::from_value(json!({
            "format": "kentos.document", "version": 2, "name": "Katman türleri",
            "settings": {
                "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000.0, "workspace": workspace,
                "sources": [{
                    "id": "kaynak-1", "kind": "geopackage", "path": "/yok/bagli.gpkg", "table": "bagli",
                    "layer": "bagli", "revision": "0".repeat(64), "digest": "0".repeat(64), "writable": true
                }]
            },
            "layers": [layer("taslak", None), layer("yazi", Some("annotation")), layer("bagli", None), layer("bos", None)],
            "activeLayer": "taslak",
            "entities": [
                line(1, "taslak"),
                line(2, "bagli"),
                { "kind": "text", "id": 3, "layerId": "yazi", "attrs": {}, "p": {"x": 0.0, "y": 0.0}, "text": "Not", "height": 2.5, "rotation": 0.0 }
            ],
            "uids": [
                "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e11", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e12",
                "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13"
            ],
            "origin": { "x": 0.0, "y": 0.0 },
            "styles": { "items": [], "categories": [] }
        }))
        .expect("a snapshot");
        Document::from_snapshot_v2(snapshot).expect("a document")
    }

    /// A CBS project's file holds its annotation, not a drawing's or a bound layer's objects; an empty layer is no data.
    #[test]
    fn a_cbs_files_data_is_held_by_its_drawing_and_bound_layers() {
        let gis = doc("gis");
        let held: Vec<String> = held_in_file(&gis).into_iter().map(|h| h.id).collect();
        assert_eq!(held, ["taslak", "bagli"]);
        let node = |id: &str| gis.layers().get(id).cloned().expect("a layer");
        assert_eq!(of(&gis, &node("yazi")), LayerKind::Annotation);
        assert_eq!(of(&gis, &node("bagli")), LayerKind::Bound);
        assert_eq!(of(&gis, &node("bos")), LayerKind::Drawing);
        assert!(held_words(&held_in_file(&gis)).contains("“taslak”, “bagli”"));
        // A CAD project keeps everything in its file.
        assert!(held_in_file(&doc("cad")).is_empty());
    }

    /// The phone edits none of these: the mobile app writes a CBS data table's rows alone (docs/adr/9198 §2.2); a CAD
    /// project is shown or left out.
    #[test]
    fn the_phone_shows_or_leaves_out_what_it_does_not_write() {
        let (gis, cad) = (doc("gis"), doc("cad"));
        assert_eq!((storage(&gis), storage(&cad)), (Storage::Database, Storage::File));
        for id in ["taslak", "yazi", "bagli"] {
            for d in [&gis, &cad] {
                let node = d.layers().get(id).cloned().expect("a layer");
                assert_eq!(mobile_choices(d, &node), [Mobile::Show, Mobile::Hide], "{id}");
            }
        }
    }
}
