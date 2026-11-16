//! An extension's examples run (docs/adr/8057 §48, §75; PY-TOOL-23), each on a synthetic drawing of its own objects
//! (the trial's, §37), never the user's:
//!
//! - **A tool's** (its descriptor's `examples`): run as İşlemler runs it, its answer checked against what the example
//!   expects: a number or text output's value, a features output's count and the attributes some written object has,
//!   words of the run's summary.
//! - **An audit rule's** (`audits/<id>.examples.json`, `kentos_extensions::audit_examples`): the drawing's facts
//!   measured as for any drawing, the architecture the example gives put in them, the rule run in its version's
//!   process as the host runs it, and its findings checked: how many, and the ones it must have.
//!
//! The desktop's Eklentiler (Örnekleri çalıştır), an agent's trial (`extension.workshop.trial` with `examples`) and the
//! command line's `kentos kek test` (§74) run the same code.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use kentos_domain::{Document, Uuid};
use kentos_processing::external::{self, ExternalTool};
use serde_json::{Map, Value, json};

use crate::synthetic::{TRIAL_SRID, synthetic_on};

/// An extension's tools that give examples, as this process's İşlemler lists them now.
pub fn of(ext: &str) -> Vec<ExternalTool> {
    let prefix = format!("ext.{ext}.");
    external::tools()
        .iter()
        .filter(|t| t.id.starts_with(&prefix))
        .filter_map(|t| external::find(&t.id))
        .filter(|t| !t.examples.is_empty())
        .collect()
}

/// What an extension gives examples of: its İşlemler tools', its audit rules' and its expression functions', each with
/// how its host runs it.
#[derive(Default)]
pub struct Examples {
    pub tools: Vec<ExternalTool>,
    pub rules: Vec<Rule>,
    pub expressions: Vec<Expression>,
    /// The package's own tests (§87), when it has `tests/test_*.py`: run after the examples.
    pub tests: Option<PackageTests>,
}

/// Where a package's own tests are and the Python of its environment.
#[derive(Clone, Debug)]
pub struct PackageTests {
    pub folder: std::path::PathBuf,
    pub python: std::path::PathBuf,
}

impl PackageTests {
    /// The package folder's tests with `python`, when it has `tests/test_*.py`.
    pub fn of(folder: &std::path::Path, python: std::path::PathBuf) -> Option<PackageTests> {
        let any = std::fs::read_dir(folder.join("tests"))
            .ok()?
            .flatten()
            .any(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.starts_with("test_") && name.ends_with(".py")
            });
        any.then(|| PackageTests {
            folder: folder.to_path_buf(),
            python,
        })
    }
}

impl Examples {
    /// How many examples there are.
    pub fn count(&self) -> usize {
        self.tools.iter().map(|t| t.examples.len()).sum::<usize>()
            + self.rules.iter().map(|r| r.examples.len()).sum::<usize>()
            + self
                .expressions
                .iter()
                .map(|e| e.examples.len())
                .sum::<usize>()
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0 && self.tests.is_none()
    }
}

/// Each tool's examples in turn, then each rule's, then each expression function's: the report (`head` and `status`,
/// `summary`, `passed`, `failed`, `examples`: one row each); Durdur between examples. Deep: a tool's code may ask for
/// a built-in tool (§47), a command inside the command.
pub fn run(examples: &Examples, head: Value, stop: &AtomicBool) -> Value {
    let mut out = Vec::new();
    for tool in &examples.tools {
        for example in &tool.examples {
            if stop.load(Ordering::Relaxed) {
                return json!({ "status": "stopped" });
            }
            let began = Instant::now();
            let mut row = one(tool, example);
            row["ms"] = json!(began.elapsed().as_millis() as u64);
            out.push(row);
        }
    }
    for rule in &examples.rules {
        for example in &rule.examples {
            if stop.load(Ordering::Relaxed) {
                return json!({ "status": "stopped" });
            }
            let began = Instant::now();
            let mut row = rules::one_rule(rule, example);
            row["ms"] = json!(began.elapsed().as_millis() as u64);
            out.push(row);
        }
    }
    for function in &examples.expressions {
        for example in &function.examples {
            if stop.load(Ordering::Relaxed) {
                return json!({ "status": "stopped" });
            }
            let began = Instant::now();
            let mut row = expressions::one_expression(function, example);
            row["ms"] = json!(began.elapsed().as_millis() as u64);
            out.push(row);
        }
    }
    let passed = out.iter().filter(|e| e["status"] == "passed").count();
    let mut report = head;
    report["status"] = json!("done");
    let mut summary = if out.is_empty() {
        String::new()
    } else {
        said(out.len(), passed)
    };
    // The package's own tests (§87), after its examples.
    if let Some(t) = &examples.tests
        && !stop.load(Ordering::Relaxed)
        && let Some(tests) = package_tests::run(&t.folder, &t.python)
    {
        let words = tests["summary"].as_str().unwrap_or_default();
        summary = if summary.is_empty() {
            format!("Sınamalar: {words}")
        } else {
            format!("{summary} Sınamalar: {words}")
        };
        report["tests"] = tests;
    }
    report["summary"] = json!(summary);
    report["passed"] = json!(passed);
    report["failed"] = json!(out.len() - passed);
    report["examples"] = json!(out);
    report
}

/// Whether a run's own tests failed (§87).
pub fn tests_failed(report: &Value) -> bool {
    report.get("tests").is_some_and(|t| t["status"] != "passed")
}

/// How many passed, in words that need no suffix after a number ("2'si", "3'ü" change with the number).
pub fn said(all: usize, passed: usize) -> String {
    match (all, passed) {
        (1, 1) => "Örnek geçti.".to_owned(),
        (1, _) => "Örnek kaldı.".to_owned(),
        (n, p) if n == p => format!("{n} örneğin hepsi geçti."),
        (n, 0) => format!("{n} örneğin hiçbiri geçmedi."),
        (n, p) => format!("{n} örnekten {p} tanesi geçti, {} tanesi kaldı.", n - p),
    }
}

/// One example run and checked: `passed`, or `failed` with what was not as it expects.
fn one(tool: &ExternalTool, example: &Value) -> Value {
    let row = |problems: Vec<String>, summary: Value| {
        json!({
            "tool": tool.tool.id, "label": tool.tool.label, "title": example["title"],
            "status": if problems.is_empty() { "passed" } else { "failed" },
            "problems": problems, "summary": summary,
        })
    };
    let features = example["features"].as_array().cloned().unwrap_or_default();
    let made = match synthetic_on(&features, TRIAL_SRID) {
        Ok(made) => made,
        Err(why) => return row(vec![format!("örneğin nesneleri: {why}")], Value::Null),
    };
    let mut doc = made.doc;
    let before: BTreeSet<Uuid> = doc.entities_with_uids().map(|(u, _)| u).collect();
    let envelope = serde_json::from_value::<kentos_commands::CallEnvelope>(json!({
        "command": "cad.processing.run", "op": "execute",
        "input": { "tool": tool.tool.id, "parameters": example.get("params").cloned().unwrap_or_else(|| json!({})) }
    }));
    let answer = match envelope {
        Ok(envelope) => {
            serde_json::to_value(kentos_headless::call_envelope(&mut doc, None, envelope))
                .unwrap_or(Value::Null)
        }
        Err(e) => {
            return row(
                vec![format!("örneğin parametreleri okunamadı: {e}")],
                Value::Null,
            );
        }
    };
    let summary = answer["output"]["summary"].clone();
    row(check(tool, example, &answer, &doc, &before), summary)
}

/// What of `example`'s expectations `answer` (the run on `doc`, whose objects before it were `before`) did not meet.
fn check(
    tool: &ExternalTool,
    example: &Value,
    answer: &Value,
    doc: &Document,
    before: &BTreeSet<Uuid>,
) -> Vec<String> {
    if answer["status"] != "completed" {
        let why = answer["error"]["message"].as_str().unwrap_or("çalışmadı");
        return vec![format!("araç çalışmadı: {why}")];
    }
    let mut problems = Vec::new();
    let output = &answer["output"];
    let expect = &example["expect"];
    if let Some(words) = expect["summary"].as_str() {
        let summary = output["summary"].as_str().unwrap_or_default();
        if !summary.contains(words) {
            problems.push(format!("özet “{words}” demiyor: “{summary}”"));
        }
    }
    let Some(outputs) = expect["outputs"].as_object() else {
        return problems;
    };
    for (name, want) in outputs {
        let Some(def) = tool.outputs.iter().find(|o| o.name == *name) else {
            problems.push(format!("“{name}” aracın bir çıktısı değil"));
            continue;
        };
        if let Some(sink) = &def.layer {
            let objects = written_on(doc, before, &sink_layer(tool, example, sink));
            if let Some(n) = want["count"].as_u64()
                && objects.len() as u64 != n
            {
                problems.push(format!(
                    "“{name}”: {n} nesne beklendi, {} yazıldı",
                    objects.len()
                ));
            }
            for attrs in want["has"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
            {
                if !objects.iter().any(|o| has(o, attrs)) {
                    problems.push(format!("“{name}”: {attrs} öznitelikli nesne yazılmadı"));
                }
            }
        } else {
            let got = &output["outputs"][name];
            if !same(got, want) {
                problems.push(format!("“{name}”: {want} beklendi, {got} geldi"));
            }
        }
    }
    problems
}

/// The layer a features output writes to in the example's drawing: the sink parameter's value as the example gives it,
/// else the tool's default (a new layer by its name, or a layer by its id).
fn sink_layer(tool: &ExternalTool, example: &Value, sink: &str) -> String {
    let given = example["params"]
        .get(sink)
        .cloned()
        .or_else(|| {
            tool.tool
                .parameters
                .iter()
                .find(|p| p.name == sink)
                .and_then(|p| match &p.default {
                    Some(kentos_processing::types::DefaultValue::Value(v)) => Some(v.clone()),
                    _ => None,
                })
        })
        .unwrap_or(Value::Null);
    given["newName"]
        .as_str()
        .or_else(|| given["layerId"].as_str())
        .unwrap_or_default()
        .to_owned()
}

/// The attributes of the objects the run wrote on layer `layer` (its name or id).
fn written_on(doc: &Document, before: &BTreeSet<Uuid>, layer: &str) -> Vec<Map<String, Value>> {
    doc.entities_with_uids()
        .filter(|(uid, _)| !before.contains(uid))
        .filter(|(_, e)| {
            let id = &e.base().layer_id;
            id == layer || doc.layers().get(id).is_some_and(|l| l.name == layer)
        })
        .map(|(_, e)| {
            e.base()
                .attrs
                .iter()
                .map(|(k, v)| (k.clone(), json!(v)))
                .collect()
        })
        .collect()
}

/// Whether an object's attributes hold every one of `want`'s (a number compared as a number, the rest as text).
fn has(attrs: &Map<String, Value>, want: &Value) -> bool {
    want.as_object().is_some_and(|w| {
        w.iter()
            .all(|(k, v)| attrs.get(k).is_some_and(|got| same(got, v)))
    })
}

/// Whether a value KentOS gave is the one expected: numbers (also written as text) within a billionth, the rest as
/// written.
fn same(got: &Value, want: &Value) -> bool {
    let number = |v: &Value| {
        v.as_f64().or_else(|| {
            v.as_str()
                .and_then(|t| t.trim().replace(',', ".").parse().ok())
        })
    };
    match (number(got), want.as_f64()) {
        (Some(a), Some(b)) => (a - b).abs() <= 1e-9 * b.abs().max(1.0),
        _ => match (got, want) {
            (Value::String(a), Value::String(b)) => a == b,
            _ => got == want,
        },
    }
}

mod expressions;
pub mod package_tests;
mod rules;

pub use expressions::{Expression, expression_examples};
pub use rules::{Rule, RunRule, rule_examples};

#[cfg(test)]
mod tests;
