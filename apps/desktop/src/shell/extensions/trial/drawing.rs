//! The synthetic drawing of an agent's trial (docs/adr/8057 §37): made of the objects the agent gives (a new GIS
//! project on TUREF / TM36, their layers by name, their attributes as text; `kentos_extension_tools::synthetic`, which
//! a tool's call for a built-in tool runs on too), and what a tool wrote on it read back.

use std::collections::{BTreeMap, BTreeSet};

use kentos_domain::{Document, Uuid};
pub(crate) use kentos_extension_tools::synthetic::Synthetic;
use serde_json::{Value, json};

/// The objects a tool wrote that a report shows of each layer; the rest are counted.
const SAMPLE: usize = 5;

/// A drawing made of `features` (`{layer, geometry, attrs?}`, GeoJSON shapes): a new GIS project on TUREF / TM36, each
/// layer found by its name or id or added (its id the name when that is one), each object on it with its attributes as
/// text. Why not, with the object's place.
pub(crate) fn synthetic(features: &[Value]) -> Result<Synthetic, String> {
    kentos_extension_tools::synthetic::synthetic_on(
        features,
        kentos_extension_tools::synthetic::TRIAL_SRID,
    )
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
