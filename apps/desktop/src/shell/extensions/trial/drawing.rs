//! The synthetic drawing of an agent's trial (docs/adr/8057 §37): made of the objects the agent gives (a new GIS
//! project on TUREF / TM36, their layers by name, their attributes as text), and what a tool wrote on it read back.

use std::collections::{BTreeMap, BTreeSet};

use kentos_domain::{Document, NewLayer, Uuid};
use serde_json::{Value, json};

/// The objects a tool wrote that a report shows of each layer; the rest are counted.
const SAMPLE: usize = 5;

/// The synthetic drawing: the objects' persistent ids in the agent's order, the layers by the names given.
pub(crate) struct Synthetic {
    pub(crate) doc: Document,
    pub(crate) uids: Vec<Uuid>,
    pub(crate) layers: Vec<String>,
}

/// A drawing made of `features` (`{layer, geometry, attrs?}`, GeoJSON shapes): a new GIS project on TUREF / TM36, each
/// layer found by its name or id or added (its id the name when that is one), each object on it with its attributes as
/// text. Why not, with the object's place.
pub(crate) fn synthetic(features: &[Value]) -> Result<Synthetic, String> {
    synthetic_on(features, 5256)
}

/// [`synthetic`] in coordinate system `srid` (an extension tool's call runs in the user's drawing's, §47).
pub(crate) fn synthetic_on(features: &[Value], srid: u32) -> Result<Synthetic, String> {
    let choices = kentos_headless::new_drawing(&json!({
        "name": "Deneme çizimi", "workspace": "gis", "srid": srid
    }))
    .map_err(|e| e.message)?;
    let snapshot = kentos_project::new_project::new_project(&choices)?;
    let mut doc = Document::from_snapshot(snapshot)?;
    let mut uids = Vec::with_capacity(features.len());
    let mut layers: Vec<String> = Vec::new();
    for (i, f) in features.iter().enumerate() {
        let at = format!("features[{i}]");
        let name = f
            .get("layer")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|n| !n.is_empty() && n.chars().count() <= 80)
            .ok_or_else(|| format!("{at}.layer: katmanın adı (en çok 80 karakter) gerekli."))?;
        let found = doc
            .layers()
            .leaves()
            .into_iter()
            .find(|l| l.id == name || l.name == name)
            .map(|l| l.id.clone());
        let layer = match found {
            Some(id) => id,
            None => {
                let simple = name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
                let new = NewLayer {
                    id: simple.then(|| name.to_owned()),
                    ..NewLayer::layer(name)
                };
                doc.add_layer(new, None, false)
                    .map_err(|e| format!("{at}.layer: {e}"))?
            }
        };
        if !layers.iter().any(|l| l == name) {
            layers.push(name.to_owned());
        }
        let attrs: BTreeMap<String, String> = f
            .get("attrs")
            .and_then(Value::as_object)
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| {
                        let text = match v {
                            Value::String(s) => s.clone(),
                            Value::Number(n) => n.to_string(),
                            Value::Bool(b) => b.to_string(),
                            _ => return None,
                        };
                        Some((k.clone(), text))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let geometry = f.get("geometry").unwrap_or(&Value::Null);
        let entity =
            kentos_processing::external::shape(&format!("{at}.geometry"), geometry, &layer, attrs)?;
        let slot = doc
            .add(entity)
            .map_err(|_| format!("{at}: çizime eklenemedi."))?;
        uids.push(
            doc.uid(slot)
                .ok_or_else(|| format!("{at}: kalıcı kimliği yok."))?,
        );
    }
    Ok(Synthetic { doc, uids, layers })
}

/// What a tool's run wrote on the synthetic drawing: each layer's new objects counted, the first few as GeoJSON with
/// their attributes.
pub(super) fn written(doc: &Document, before: &BTreeSet<Uuid>) -> Vec<Value> {
    let mut by_layer: BTreeMap<String, (usize, Vec<Value>)> = BTreeMap::new();
    for (uid, e) in doc.entities_with_uids() {
        if before.contains(&uid) {
            continue;
        }
        let (count, sample) = by_layer.entry(layer_name(doc, e)).or_default();
        *count += 1;
        if sample.len() < SAMPLE {
            sample.push(json!({
                "geometry": kentos_processing::external::geojson(e),
                "attrs": e.base().attrs,
            }));
        }
    }
    by_layer
        .into_iter()
        .map(|(layer, (objects, sample))| json!({ "layer": layer, "objects": objects, "sample": sample }))
        .collect()
}

/// The name of `e`'s layer (its id when the layer is not there).
pub(super) fn layer_name(doc: &Document, e: &kentos_commands::Entity) -> String {
    doc.layers()
        .get(&e.base().layer_id)
        .map_or_else(|| e.base().layer_id.clone(), |l| l.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::audits::MOST_OBJECTS;

    /// The synthetic drawing: a GIS project with the objects' layers by name (a simple name is the id too, a layer the
    /// project has is used), each object with its attributes as text, in the agent's order; a shape that is not one is
    /// said with its place.
    #[test]
    fn a_synthetic_drawing_is_made_of_the_agents_objects() {
        let made = synthetic(&[
            json!({ "layer": "mezar_yeri", "geometry": { "type": "Polygon",
                    "coordinates": [[[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0], [0.0, 0.0]]] },
                    "attrs": { "ada": "101", "no": 7, "dolu": true, "bos": null } }),
            json!({ "layer": "Yol ekseni", "geometry": { "type": "LineString", "coordinates": [[0.0, 0.0], [5.0, 5.0]] } }),
            json!({ "layer": "mezar_yeri", "geometry": { "type": "Point", "coordinates": [1.0, 1.0] } }),
        ])
        .expect("a drawing");
        assert_eq!(made.uids.len(), 3);
        assert_eq!(made.layers, ["mezar_yeri", "Yol ekseni"]);
        let first = made
            .doc
            .slot_of(made.uids[0])
            .and_then(|s| made.doc.get(s))
            .unwrap();
        assert_eq!(
            first.base().layer_id,
            "mezar_yeri",
            "a simple name is the id"
        );
        assert_eq!(first.base().attrs.get("no").map(String::as_str), Some("7"));
        assert_eq!(
            first.base().attrs.get("dolu").map(String::as_str),
            Some("true")
        );
        assert!(!first.base().attrs.contains_key("bos"));
        let facts = kentos_processing::external::audit_facts(&made.doc, MOST_OBJECTS);
        assert!(facts.to_string().contains("mezar_yeri"), "{facts}");
        let wrong = synthetic(&[json!({ "layer": "yapi", "geometry": { "type": "Circle" } })]);
        assert!(wrong.err().unwrap().starts_with("features[0].geometry"));
        assert!(
            synthetic(&[json!({ "geometry": { "type": "Point", "coordinates": [0, 0] } })])
                .is_err()
        );
    }
}
