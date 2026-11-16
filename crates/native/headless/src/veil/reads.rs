//! What an İşlemler call reads by its tools' parameters (docs/adr/8057 §108): an expression parameter's text is read
//! as an expression, a field parameter's names (several with commas) as fields, wherever the parameter is named. A
//! tool's expression may stand under `value` (Hesapla's), which a call's walk takes as a written value
//! ([`super::WRITTEN`]); a model an agent writes gives every step's value as `{"kind": "value", "value": …}`.
//! Checked by kind: a run, a batch's rows and a written model's steps refused alike when they read a hidden field. A
//! saved model's steps are its owner's, not the agent's.

use kentos_processing::types::ParamKind;
use serde_json::Value;

use super::Veil;

/// What a parameter reads: an expression, field names.
#[derive(Clone, Copy)]
enum Reads {
    Expression,
    Fields,
}

/// The parameters of tool `id` that read, by name.
fn reading(id: &str) -> Vec<(String, Reads)> {
    let Some(tool) = kentos_processing::Registry::current().tool(id) else {
        return Vec::new();
    };
    tool.parameters
        .iter()
        .filter_map(|p| match p.kind {
            ParamKind::Expression { .. } => Some((p.name.clone(), Reads::Expression)),
            ParamKind::Field { .. } => Some((p.name.clone(), Reads::Fields)),
            _ => None,
        })
        .collect()
}

/// The hidden field `given` reads as `reads`.
fn read(veil: &Veil, given: &Value, reads: Reads) -> Option<String> {
    let text = given.as_str()?;
    match reads {
        Reads::Expression => veil.read_by(text),
        Reads::Fields => veil.among(text),
    }
}

/// Where `command`'s `input` reads a hidden field through a tool's parameter, and the field: its place in the input.
pub(crate) fn processing_read(
    veil: &Veil,
    command: &str,
    input: &Value,
) -> Option<(String, String)> {
    if !matches!(command, "cad.processing.run" | "cad.processing.batch") {
        return None;
    }
    let tool = input
        .get("tool")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = reading(tool);
    // The run's own values, and a batch's rows'.
    let mut given: Vec<(String, &Value, Reads)> = Vec::new();
    for (name, reads) in &params {
        if let Some(v) = input.get("parameters").and_then(|p| p.get(name)) {
            given.push((format!("parameters.{name}"), v, *reads));
        }
        let rows = input.get("rows").and_then(Value::as_array);
        for (i, row) in rows.into_iter().flatten().enumerate() {
            if let Some(v) = row.get("parameters").and_then(|p| p.get(name)) {
                given.push((format!("rows[{i}].parameters.{name}"), v, *reads));
            }
        }
    }
    for (at, v, reads) in given {
        if let Some(field) = read(veil, v, reads) {
            return Some((at, field));
        }
    }
    // A model written for the run: each step's given values by its tool's parameters.
    let steps = input.pointer("/model/steps").and_then(Value::as_array);
    for (i, step) in steps.into_iter().flatten().enumerate() {
        let tool = step.get("tool").and_then(Value::as_str).unwrap_or_default();
        for (name, reads) in reading(tool) {
            let Some(source) = step.get("values").and_then(|v| v.get(&name)) else {
                continue;
            };
            if source.get("kind").and_then(Value::as_str) != Some("value") {
                continue;
            }
            let given = source.get("value").unwrap_or(&Value::Null);
            if let Some(field) = read(veil, given, reads) {
                return Some((format!("model.steps[{i}].values.{name}.value"), field));
            }
        }
    }
    None
}
