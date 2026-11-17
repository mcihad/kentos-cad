//! What an extension's İşlemler tool asks of KentOS while it runs (docs/adr/8057 §47, PY-TOOL-18): `processing.run`,
//! one of KentOS's built-in tools run on a drawing of its own made of the objects the tool gives, never the user's,
//! and what that tool made given back as objects. The desktop and the headless hosts answer alike.
//!
//! - **Data in, data out:** a tool's inputs came to it as data (PY-TOOL-09) and so does what it asks: a features
//!   parameter is given as objects (`{"features": [{geometry, attrs}]}`), each becomes a layer of the call's own
//!   drawing (in the user's drawing's coordinate system), and the call reaches no layer the tool was not given.
//! - **Built-in tools only:** another extension's tool is not run from a tool (no chain of processes, no way round its
//!   dependency); a tool's own code calls its own functions.
//! - **No questions:** a tool's code does not ask the user or open a panel ([`NO_QUESTIONS`]): what it would ask is its
//!   parameter. The same on the desktop and headless; a window is not what is missing.
//! - **Bounded:** at most [`MOST`] objects in and out; the made objects are read back in the order they were made.
//! - **A stack of its own:** the call is a whole command run inside another one's (the tool's), on whatever thread that
//!   one has; it runs on a thread of [`STACK`]. An agent's run goes as a job, whose thread has the default 2 MiB: the
//!   debug MCP aborted there with a stack overflow on its first call for a buffer (measured, 16 November 2026).

use std::collections::{BTreeMap, BTreeSet};

use kentos_domain::{Document, Uuid};
use serde_json::{Map, Value, json};

/// The most objects a call takes in all, and the most it gives back.
pub const MOST: usize = 50_000;

/// The stack a call runs on: what KentOS gives its heavy work elsewhere (a reservation; the pages are used as needed).
pub const STACK: usize = 64 << 20;

fn refused(code: &str, words: impl Into<String>) -> (String, String) {
    (code.to_owned(), words.into())
}

/// The call's answer (`{summary, warnings, layers: [{name, features}]}`), or why not (a code and the words): `method`
/// with `params` from a tool running on a drawing in coordinate system `srid`, on a thread of [`STACK`] the caller
/// waits for.
pub fn tool_call(method: &str, params: &Value, srid: u32) -> Result<Value, (String, String)> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("kentos-ext-cagri".into())
            .stack_size(STACK)
            .spawn_scoped(scope, || answer(method, params, srid))
            .map_err(|e| {
                refused(
                    "tool_failed",
                    format!("Araç için iş parçacığı açılamadı: {e}"),
                )
            })?
            .join()
            .unwrap_or_else(|_| {
                Err(refused(
                    "tool_failed",
                    "Araç çalışırken beklenmedik biçimde durdu.",
                ))
            })
    })
}

/// Why a tool does not ask the user while it runs, on any host (docs/adr/8057 §73): what it would ask is its
/// parameter, which every way of running it (its window, a batch, a model, an agent, a server's job) gives before the
/// run and the run's record keeps (PY-TOOL-21, 22). A window is not what is missing: the desktop refuses it alike.
pub const NO_QUESTIONS: &str = "Bir İşlemler aracı çalışırken kullanıcıya sormaz ve panel açmaz (ctx.dialog, \
                                ctx.confirm, ctx.open_panel): sorulacak olan aracın parametresidir; araç penceresi, \
                                toplu çalıştırma, model, ajan ve sunucu işi onu çalıştırmadan önce verir, çalıştırma \
                                kaydı da onu tutar. Kullanıcıyla konuşan iş bir komuttur (@ext.command).";

/// [`tool_call`]'s work, on its thread.
fn answer(method: &str, params: &Value, srid: u32) -> Result<Value, (String, String)> {
    if method == "dialog" || method.starts_with("panel.") {
        return Err(refused("capability", NO_QUESTIONS));
    }
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
    let made = crate::synthetic::synthetic_on(&features, srid)
        .map_err(|why| refused("invalid_input", why))?;
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
        kentos_processing::command::register();
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
        // A question or a panel: refused on every host with why (the tool's parameters), not for want of a window.
        for asked in ["dialog", "panel.open"] {
            let (code, words) = tool_call(asked, &json!({ "title": "?" }), 5256).unwrap_err();
            assert_eq!(
                (code.as_str(), words.as_str()),
                ("capability", NO_QUESTIONS)
            );
        }
        let none = call(json!({ "input": { "features": [] }, "distance": 2 })).expect("an answer");
        assert_eq!(none["layers"], json!([]));
        assert!(none["summary"].as_str().unwrap().contains("Girdi boş"));
    }
}
