//! A CBS project's data layers read into objects of their own layers, for KentOS Bulut (docs/adr/9196 §2, plan/ayrim.md
//! AY-05). A cloud CBS project's database is its data store: its upload takes each data layer's rows from the source
//! they are in (a GeoPackage, a PostgreSQL schema) as objects with their values, in the project's system, and the
//! layer then holds them as any layer of a cloud project does. A row without geometry is a record. The project on the
//! computer is not changed: the upload's copy of it is.

use kentos_commands::{DocumentSnapshotV2, Entity, EntityId, LayerNode, LayerNodeType};
use kentos_data::{Cancel, Request};
use serde_json::{Map, Value as J, json};

use crate::sources::TableSystem;

/// What the data layers gave.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Taken {
    /// The data layers read, each now a layer of objects.
    pub layers: usize,
    /// The rows taken as objects.
    pub rows: u64,
    /// The rows left out, each with its layer and why: a geometry the drawing has no object for.
    pub left: Vec<String>,
}

/// Rows read at a time.
const CHUNK: usize = 2000;

/// Every data layer of `snapshot` (not a service's) read into objects of its layer, its rows' values their attributes,
/// and the layer no longer a data layer. Stops at the first source that cannot be read, nothing changed.
pub fn take_rows(snapshot: &mut DocumentSnapshotV2, cancel: &Cancel) -> Result<Taken, String> {
    let mut next = snapshot
        .entities
        .iter()
        .map(|e| e.base().id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let settings = snapshot.settings.clone();
    let default_height = crate::data_text::default_height(&settings);
    let mut taken = Taken::default();
    let mut made: Vec<Entity> = Vec::new();
    let mut layers = std::mem::take(&mut snapshot.layers);
    let walked = walk(&mut layers, &mut |node| {
        let Some(data) = node.data.clone() else {
            return Ok(());
        };
        let source = kentos_data::registry()
            .source(&data.address)
            .map_err(|e| format!("“{}” katmanının kaynağı açılamadı: {e}", node.name))?;
        let schema = source
            .schema(&data.table)
            .map_err(|e| format!("“{}” katmanının tablosu okunamadı: {e}", node.name))?;
        let columns: Vec<String> = schema.fields.iter().map(|f| f.name.clone()).collect();
        let srid = data.geometry.as_ref().and_then(|g| g.srid);
        let system =
            TableSystem::of(&settings, srid).map_err(|why| format!("“{}”: {why}", node.name))?;
        let request = Request {
            columns: Some(columns.clone()),
            geometry: data.geometry.is_some(),
            chunk: CHUNK,
            ..Request::default()
        };
        source
            .features(&data.table, &request, cancel, &mut |part| {
                for f in part {
                    let attrs: Map<String, J> = columns
                        .iter()
                        .zip(&f.values)
                        .filter_map(|(c, v)| {
                            crate::geopackage::attribute_text(v).map(|t| (c.clone(), J::String(t)))
                        })
                        .collect();
                    let shape = match &f.geometry {
                        None => None,
                        Some(Ok(g)) if g.is_empty() => None,
                        Some(Ok(g)) => match &system {
                            Some(s) => match s.moved(g) {
                                Some(m) => Some(m),
                                None => {
                                    taken.left.push(format!(
                                        "“{}” {}: projenin sistemine çevrilemedi",
                                        node.name, f.fid
                                    ));
                                    continue;
                                }
                            },
                            None => Some(g.clone()),
                        },
                        Some(Err(why)) => {
                            taken.left.push(format!(
                                "“{}” {}: geometrisi okunamadı ({why})",
                                node.name, f.fid
                            ));
                            continue;
                        }
                    };
                    let entity = match (&shape, &data.text) {
                        (None, _) => record(&node.id, next, attrs),
                        (Some(g), Some(text)) => crate::data_text::point_of(g).and_then(|at| {
                            crate::data_text::object(
                                text,
                                at,
                                &node.id,
                                next,
                                attrs,
                                default_height,
                            )
                        }),
                        (Some(g), None) => crate::geopackage::object_of(g, &node.id, next, attrs),
                    };
                    match entity {
                        Some(e) => {
                            made.push(e);
                            next = next.saturating_add(1);
                            taken.rows += 1;
                        }
                        None => taken.left.push(format!(
                            "“{}” {}: geometrisinin çizimde karşılığı yok",
                            node.name, f.fid
                        )),
                    }
                }
                !cancel.is_cancelled()
            })
            .map_err(|e| format!("“{}” katmanının satırları okunamadı: {e}", node.name))?;
        if cancel.is_cancelled() {
            return Err("Durduruldu.".to_owned());
        }
        taken.layers += 1;
        Ok(())
    });
    if let Err(why) = walked {
        snapshot.layers = layers;
        return Err(why);
    }
    plain(&mut layers);
    snapshot.layers = layers;
    snapshot.uids.extend(
        made.iter()
            .map(|_| EntityId(kentos_domain::Uuid::now_v7().into_bytes())),
    );
    snapshot.entities.extend(made);
    Ok(taken)
}

/// The tree as the cloud keeps it once the rows are taken: each data layer (not a service's) a layer of objects, its
/// name, look and fields kept. The upload's catalog entry takes the same tree as its file.
pub fn plain(nodes: &mut [LayerNode]) {
    for n in nodes {
        if n.kind == LayerNodeType::Group {
            plain(&mut n.children);
        } else if n.service.is_none() {
            n.data = None;
        }
    }
}

/// A row without geometry: a record of the layer with its values.
fn record(layer: &str, id: u32, attrs: Map<String, J>) -> Option<Entity> {
    serde_json::from_value(json!({ "kind": "record", "id": id, "layerId": layer, "attrs": attrs }))
        .ok()
}

/// Each data layer of the tree, not a service's, depth first.
fn walk(
    nodes: &mut [LayerNode],
    each: &mut dyn FnMut(&mut LayerNode) -> Result<(), String>,
) -> Result<(), String> {
    for n in nodes {
        if n.kind == LayerNodeType::Group {
            walk(&mut n.children, each)?;
        } else if n.data.is_some() && n.service.is_none() {
            each(n)?;
        }
    }
    Ok(())
}
