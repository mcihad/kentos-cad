//! A synthetic drawing made of the objects given as data (docs/adr/8057 §37, §47): a new GIS project, their layers by
//! name, their attributes as text. A built-in tool an extension's code asks for runs on one (calls.rs); an agent's
//! trial of an extension on the desktop and a tool's examples (examples.rs) run on one too.

use std::collections::BTreeMap;

use kentos_domain::{Document, NewLayer, Uuid};
use serde_json::{Value, json};

/// The coordinate system of a trial's and an example's drawing: TUREF / TM36 (EPSG:5256).
pub const TRIAL_SRID: u32 = 5256;

/// The synthetic drawing: the objects' persistent ids in the order given, the layers by the names given.
pub struct Synthetic {
    pub doc: Document,
    pub uids: Vec<Uuid>,
    pub layers: Vec<String>,
}

/// A drawing made of `features` (`{layer, geometry, attrs?}`, GeoJSON shapes) in coordinate system `srid`: a new GIS
/// project, each layer found by its name or id or added (its id the name when that is one), each object on it with its
/// attributes as text. Why not, with the object's place.
pub fn synthetic_on(features: &[Value], srid: u32) -> Result<Synthetic, String> {
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
