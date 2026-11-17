//! An expression function's examples (docs/adr/8057 §78): each call run in the version's process as a label, a filter
//! or a style rule calls it (`kind: expression`, a row of arguments), its value checked against the one expected.

use kentos_extension_host::Registry;
use kentos_extensions::manifest::{ExpressionEntry, ExpressionExample};
use serde_json::{Value, json};

use super::rules::RunRule;

/// An expression function's examples, and how its host runs it.
pub struct Expression {
    /// `ext.<extension>.<local id>`: how an expression calls it.
    pub id: String,
    pub title: String,
    pub examples: Vec<ExpressionExample>,
    pub run: RunRule,
}

/// The expression functions of extension `ext` installed in `registry` that give examples.
pub fn expression_examples(registry: &Registry, ext: &str) -> Vec<ExpressionEntry> {
    registry
        .manifest(ext)
        .map(|m| {
            m.entrypoints
                .expressions
                .iter()
                .filter(|e| !e.examples.is_empty())
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// One call run and checked: `passed`, or `failed` with what it gave or why it gave nothing (its traceback, `detail`).
pub(super) fn one_expression(f: &Expression, example: &ExpressionExample) -> Value {
    let args: Vec<String> = example.args.iter().map(Value::to_string).collect();
    let title = example
        .title
        .clone()
        .unwrap_or_else(|| format!("{}({})", f.id, args.join(", ")));
    let row = |problems: Vec<String>, value: Value, detail: Option<&str>| {
        json!({
            "expression": f.id, "label": f.title, "title": title,
            "status": if problems.is_empty() { "passed" } else { "failed" },
            "problems": problems, "value": value, "detail": detail,
        })
    };
    let done = match (f.run)(json!({ "kind": "expression", "rows": [example.args] })) {
        Ok(done) => done,
        Err(why) => return row(vec![format!("işlev çalışmadı: {why}")], Value::Null, None),
    };
    if done["ok"] != true {
        let message = done["message"].as_str().unwrap_or("cevap vermedi");
        let words = match done["exception"].as_str() {
            Some(kind) => format!("{kind}: {message}"),
            None => message.to_owned(),
        };
        return row(
            vec![format!("işlev hata verdi: {words}")],
            Value::Null,
            done["error"].as_str(),
        );
    }
    let result = &done["result"];
    if result["failed"].as_u64().unwrap_or(0) > 0 {
        let why = result["error"].as_str().unwrap_or_default();
        return row(vec![format!("işlev hata verdi: {why}")], Value::Null, None);
    }
    let got = result["values"][0].clone();
    let problems = if super::same(&got, &example.expect) {
        Vec::new()
    } else {
        vec![format!("{} beklendi, {got} geldi", example.expect)]
    };
    row(problems, got, None)
}
