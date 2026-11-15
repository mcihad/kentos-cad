//! What an extension's İşlemler tool asks of KentOS while it runs (docs/adr/8057 §47, PY-TOOL-18): `processing.run`,
//! one of KentOS's built-in tools run on a drawing of its own made of the objects the tool gives, never the user's,
//! and what that tool made given back as objects.
//!
//! - **Data in, data out:** a tool's inputs came to it as data (PY-TOOL-09) and so does what it asks: a features
//!   parameter is given as objects (`{"features": [{geometry, attrs}]}`), each becomes a layer of the call's own
//!   drawing (in the user's drawing's coordinate system), and the call reaches no layer the tool was not given.
//! - **Built-in tools only:** another extension's tool is not run from a tool (no chain of processes, no way round its
//!   dependency); a tool's own code calls its own functions.
//! - **Bounded:** at most [`MOST`] objects in and out; the made objects are read back in the order they were made.

use std::collections::{BTreeMap, BTreeSet};

use kentos_domain::{Document, Uuid};
use serde_json::{Map, Value, json};

/// The most objects a call takes in all, and the most it gives back.
pub(crate) const MOST: usize = 50_000;

fn refused(code: &str, words: impl Into<String>) -> (String, String) {
    (code.to_owned(), words.into())
}

/// The call's answer (`{summary, warnings, layers: [{name, features}]}`), or why not (a code and the words): `method`
/// with `params` from a tool running on a drawing in coordinate system `srid`.
pub(crate) fn tool_call(
    method: &str,
    params: &Value,
    srid: u32,
) -> Result<Value, (String, String)> {
    if method != "processing.run" {
        return Err(refused(
            "unknown_method",
            format!(
                "“{method}” bir aracın isteyebileceği bir şey değil: yalnız processing.run (ctx.run_tool)."
            ),
        ));
    }
    let tool = params["tool"]
        .as_str()
        .filter(|t| !t.trim().is_empty())
        .ok_or_else(|| {
            refused(
                "invalid_input",
                "tool: KentOS'un yerleşik aracının kimliği gerekli.",
            )
        })?;
    if tool.starts_with("ext.") {
        return Err(refused(
            "capability",
            format!(
                "“{tool}” bir eklentinin aracı: bir aracın içinden yalnız KentOS'un yerleşik araçları çalışır \
                 (geometry.buffer gibi)."
            ),
        ));
    }
    // Each features parameter becomes a layer of the call's own drawing; the rest go as given.
    let mut parameters: Map<String, Value> = params["parameters"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let mut features: Vec<Value> = Vec::new();
    let mut empty = Vec::new();
    for (name, value) in parameters.iter_mut() {
        let Some(list) = value.get("features").and_then(Value::as_array) else {
            continue;
        };
        let layer = format!("girdi-{}", name.to_lowercase());
        if list.is_empty() {
            empty.push(name.clone());
        }
        for f in list {
            features
                .push(json!({ "layer": layer, "geometry": f["geometry"], "attrs": f["attrs"] }));
        }
        *value = json!({ "scope": "layer", "layerId": layer });
    }
    if features.len() > MOST {
        return Err(refused(
            "too_large",
            format!(
                "Araca en çok {MOST} nesne verilir; {} nesne verildi.",
                features.len()
            ),
        ));
    }
    // Nothing to work on: nothing made, the tool not run (its input layer would not be there).
    if !empty.is_empty() {
        return Ok(json!({
            "summary": format!("Girdi boş ({}): araç çalıştırılmadı.", empty.join(", ")),
            "warnings": [],
            "layers": [],
        }));
    }
    let made =
        super::trial::synthetic_on(&features, srid).map_err(|why| refused("invalid_input", why))?;
    let mut doc = made.doc;
    let before: BTreeSet<Uuid> = made.uids.iter().copied().collect();
    let envelope = serde_json::from_value::<kentos_commands::CallEnvelope>(json!({
        "command": "cad.processing.run", "op": "execute",
        "input": { "tool": tool, "parameters": parameters }
    }))
    .map_err(|e| refused("invalid_input", format!("Aracın girdisi okunamadı: {e}")))?;
    let answer = serde_json::to_value(kentos_headless::call_envelope(&mut doc, None, envelope))
        .unwrap_or(Value::Null);
    if answer["status"] != "completed" {
        let error = &answer["error"];
        return Err(refused(
            error["code"].as_str().unwrap_or("tool_failed"),
            format!(
                "“{tool}”: {}",
                error["message"].as_str().unwrap_or("araç çalışmadı")
            ),
        ));
    }
    // The tool's own line ("2 nesnenin tamponu yazıldı"), not the call's account of the change.
    Ok(json!({
        "summary": answer["output"]["summary"].clone(),
        "warnings": answer.get("warnings").cloned().unwrap_or_else(|| json!([])),
        "layers": made_objects(&doc, &before),
    }))
}

/// What the call's tool made: each layer's new objects as a tool's features are given (`uid`, `layer`, `entity` with
/// its kind and attributes, `geometry` GeoJSON), at most [`MOST`] in all.
fn made_objects(doc: &Document, before: &BTreeSet<Uuid>) -> Vec<Value> {
    let mut by_layer: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut given = 0usize;
    for (uid, e) in doc.entities_with_uids() {
        if before.contains(&uid) || given >= MOST {
            continue;
        }
        let layer = doc
            .layers()
            .get(&e.base().layer_id)
            .map_or_else(|| e.base().layer_id.clone(), |l| l.name.clone());
        let kind = serde_json::to_value(e)
            .ok()
            .and_then(|v| v.get("kind").cloned())
            .unwrap_or(Value::Null);
        by_layer.entry(layer.clone()).or_default().push(json!({
            "uid": uid.to_string(),
            "layer": layer,
            "entity": { "kind": kind, "attrs": e.base().attrs },
            "geometry": kentos_processing::external::geojson(e),
        }));
        given += 1;
    }
    by_layer
        .into_iter()
        .map(|(name, features)| json!({ "name": name, "features": features }))
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn square(x: f64) -> serde_json::Value {
        json!({ "geometry": { "type": "Polygon", "coordinates": [[[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 10.0],
                                                                  [x, 10.0], [x, 0.0]]] },
                "attrs": { "no": format!("Y-{x}") } })
    }

    /// A built-in tool runs on the objects the call gives and its made objects come back as a tool's features; an
    /// extension's tool, another method and an empty input are said, the last without running anything.
    #[test]
    fn a_tools_call_runs_a_builtin_tool_on_its_own_objects() {
        kentos_processing::command::register_as("desktop");
        let call = |parameters: serde_json::Value| {
            tool_call(
                "processing.run",
                &json!({ "tool": "geometry.buffer", "parameters": parameters }),
                5256,
            )
        };
        let answer = call(
            json!({ "input": { "features": [square(0.0), square(100.0)] }, "distance": 2,
                                  "layer": { "newName": "Tampon" } }),
        )
        .expect("the tool ran");
        let layers = answer["layers"].as_array().expect("its layers");
        assert_eq!(layers.len(), 1, "{answer}");
        assert_eq!(layers[0]["name"], "Tampon");
        let made = layers[0]["features"].as_array().expect("its objects");
        assert_eq!(made.len(), 2, "a buffer for each square");
        assert_eq!(made[0]["geometry"]["type"], "Polygon");
        assert_eq!(made[0]["layer"], "Tampon");
        assert!(
            answer["summary"].as_str().is_some_and(|s| !s.is_empty()),
            "{answer}"
        );
        // Another extension's tool, another method: refused; an empty input: nothing run.
        let ext = tool_call(
            "processing.run",
            &json!({ "tool": "ext.tr.ornek.x.y", "parameters": {} }),
            5256,
        );
        assert_eq!(ext.unwrap_err().0, "capability");
        assert_eq!(
            tool_call("entities", &json!({}), 5256).unwrap_err().0,
            "unknown_method"
        );
        let none = call(json!({ "input": { "features": [] }, "distance": 2 })).expect("an answer");
        assert_eq!(none["layers"], json!([]));
        assert!(none["summary"].as_str().unwrap().contains("Girdi boş"));
    }
}
