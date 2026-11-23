//! A data layer's examples in Etiketler (docs/adr/9186): its rows live in their source, not in the drawing, so the window
//! reads the table's first rows off the interface's thread, as the drawing reads them (the project's system, the
//! columns the layer declares), into a document of their own. The examples, the empty texts and Denetle's candidates
//! then read them as a drawing layer's objects; the label texts are the engine's own.

use kentos_commands::{LayerNode, ProjectSettings, ProjectStyles};
use kentos_data::{Cancel, Feature, Request};
use kentos_native_application::sources::TableSystem;

use super::candidates::SAMPLE_OBJECTS;

/// The rows read: as many as a drawing layer's examples read of its objects.
pub(super) const ROWS: usize = SAMPLE_OBJECTS;

/// The first [`ROWS`] rows of the data layer `node` as objects of a document of their own: the layer without its
/// binding, the project's settings and styles. Why not, in words: the source unreachable, the table gone, the system.
pub(super) fn read(
    node: &LayerNode,
    settings: &ProjectSettings,
    styles: &ProjectStyles,
) -> Result<kentos_domain::Document, String> {
    let data = node
        .data
        .as_ref()
        .ok_or_else(|| format!("“{}” bir veri katmanı değil.", node.name))?;
    let source = kentos_data::registry()
        .source(&data.address)
        .map_err(|e| format!("Kaynağa ulaşılamadı: {e}"))?;
    let system = TableSystem::of(settings, data.geometry.as_ref().and_then(|g| g.srid))?;
    let columns: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
    let request = Request {
        columns: Some(columns.clone()),
        geometry: data.geometry.is_some(),
        limit: Some(ROWS as u64),
        chunk: ROWS,
        ..Request::default()
    };
    let mut entities = Vec::new();
    let mut next_id = 1u32;
    source
        .features(&data.table, &request, &Cancel::new(), &mut |part: Vec<Feature>| {
            for f in part {
                let attrs: serde_json::Map<String, serde_json::Value> = columns
                    .iter()
                    .zip(&f.values)
                    .filter_map(|(c, v)| crate::data_view::attribute(v).map(|a| (c.clone(), a)))
                    .collect();
                // A row without a geometry is no object: a table without geometry has no labels on the map.
                let Some(g) = f.shape() else { continue };
                let g = match &system {
                    Some(s) => match s.moved(g) {
                        Some(m) => m,
                        None => continue,
                    },
                    None => g.clone(),
                };
                if let Some(e) = kentos_native_application::geopackage::object_of(&g, &node.id, next_id, attrs) {
                    next_id = next_id.wrapping_add(1);
                    entities.push(e);
                }
            }
            entities.len() < ROWS
        })
        .map_err(|e| format!("“{}” tablosu okunamadı: {e}", data.table))?;
    let mut layer = node.clone();
    layer.data = None;
    kentos_domain::Document::from_snapshot(kentos_commands::DocumentSnapshotV1 {
        format: kentos_commands::DOCUMENT_FORMAT.into(),
        version: kentos_commands::DOCUMENT_VERSION,
        name: String::new(),
        settings: settings.clone(),
        origin: kentos_commands::Vec2 { x: 0.0, y: 0.0 },
        home_view: None,
        layers: vec![layer],
        active_layer: node.id.clone(),
        entities,
        styles: styles.clone(),
        blocks: Vec::new(),
    })
}
