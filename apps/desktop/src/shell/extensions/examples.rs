//! An extension tool's examples run (docs/adr/8057 §48, PY-TOOL-23): each on a synthetic drawing of its own objects
//! (the trial's, §37), never the user's, the tool run as İşlemler runs it, and its answer checked against what the
//! example expects: a number or text output's value, a features output's count and the attributes some written
//! object has, words of the run's summary. The user's Eklentiler (Örnekleri çalıştır) and an agent's trial
//! (`extension.workshop.trial` with `examples`) start it alike.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use kentos_domain::{Document, Uuid};
use kentos_processing::external::ExternalTool;
use kentos_ui::widget::Toast;
use serde_json::{Map, Value, json};

use super::audits::Refusal;
use super::trial::{TrialState, synthetic};
use super::{Event, msg};
use crate::app::{App, Message};

/// An extension's tools that give examples, as installed now.
pub(crate) fn with_examples(ext: &str) -> Vec<ExternalTool> {
    let prefix = format!("ext.{ext}.");
    kentos_processing::external::tools()
        .iter()
        .filter(|t| t.id.starts_with(&prefix))
        .filter_map(|t| kentos_processing::external::find(&t.id))
        .filter(|t| !t.examples.is_empty())
        .collect()
}

impl App {
    /// The examples of `ext`'s tools started in the background (the trial's place: one at a time); `by_user` says the
    /// user asked (a toast and the log say how it went). The task to run, or why not.
    pub(crate) fn ext_examples_start(
        &mut self,
        ext: &str,
        by_user: bool,
    ) -> Result<(Value, iced::Task<Message>), Refusal> {
        if self
            .extensions
            .trial
            .as_ref()
            .is_some_and(|t| t.running.is_some())
        {
            return Err((
                "busy",
                "Bir deneme koşusu sürüyor: bitince yeniden deneyin.".to_owned(),
            ));
        }
        if let Some(Some(why)) = self.extensions.python_reasons.get(ext) {
            return Err(("not_ready", format!("Eklentinin kodu çalışmıyor: {why}")));
        }
        let tools = with_examples(ext);
        if tools.is_empty() {
            return Err((
                "no_examples",
                format!("“{ext}” araçlarının tanımında örnek (examples) yok."),
            ));
        }
        let count: usize = tools.iter().map(|t| t.examples.len()).sum();
        let version = tools[0].version.clone();
        let stop = Arc::new(AtomicBool::new(false));
        self.extensions.trial = Some(TrialState {
            ext: ext.to_owned(),
            running: Some(Arc::clone(&stop)),
            report: None,
            by_user,
        });
        let head = json!({ "kind": "examples", "extension": { "id": ext, "version": version } });
        let task = crate::python_setup::in_background_deep(
            move || run(&tools, head, &stop),
            json!({ "status": "failed", "error": "Örnek koşusu cevapsız kaldı." }),
            |report| msg(Event::TrialDone(Box::new(report))),
        );
        Ok((
            json!({ "status": "running", "extension": ext, "examples": count }),
            task,
        ))
    }

    /// The user's run of the examples ended: how it went in a toast, each failed example in the extension's log.
    pub(crate) fn ext_examples_said(&mut self, report: &Value) {
        let ext = report["extension"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let name = self.extension_name(&ext);
        let said = report["summary"]
            .as_str()
            .unwrap_or("Örnekler çalıştı.")
            .to_owned();
        let failed: Vec<&Value> = report["examples"]
            .as_array()
            .map(|l| l.iter().filter(|e| e["status"] != "passed").collect())
            .unwrap_or_default();
        for e in &failed {
            let problems: Vec<&str> = e["problems"]
                .as_array()
                .map(|p| p.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let words = format!(
                "{} › {}: {}",
                e["label"].as_str().unwrap_or_default(),
                e["title"].as_str().unwrap_or_default(),
                problems.join("; ")
            );
            self.extension_failed(&ext, "examples", "example_failed", &words, None);
        }
        if failed.is_empty() {
            // A problem an earlier run left goes: the tools do what their examples say.
            self.extension_worked(&ext, "examples", "examples_passed", &said);
            self.toast(Toast::success(format!("{name}: araç örnekleri")).body(said));
        } else {
            self.toast(Toast::warning(format!("{name}: araç örnekleri")).body(said));
        }
    }
}

/// Each tool's examples in turn, the report; Durdur between examples.
fn run(tools: &[ExternalTool], head: Value, stop: &AtomicBool) -> Value {
    let mut out = Vec::new();
    for tool in tools {
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
    let passed = out.iter().filter(|e| e["status"] == "passed").count();
    let mut report = head;
    report["status"] = json!("done");
    report["summary"] = json!(said(out.len(), passed));
    report["passed"] = json!(passed);
    report["failed"] = json!(out.len() - passed);
    report["examples"] = json!(out);
    report
}

/// How many passed, in words that need no suffix after a number ("2'si", "3'ü" change with the number).
fn said(all: usize, passed: usize) -> String {
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
    let made = match synthetic(&features) {
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Numbers compare as numbers (text too, a comma for the point), text as written; an object holds an attribute
    /// set when every key matches.
    #[test]
    fn an_expected_value_is_met_as_a_number_or_as_text() {
        assert!(same(&json!(1), &json!(1.0)));
        assert!(same(&json!("0,4"), &json!(0.4)));
        assert!(!same(&json!(2), &json!(1)));
        assert!(
            same(&json!("aşıyor"), &json!("aşıyor")) && !same(&json!("uygun"), &json!("aşıyor"))
        );
        let attrs: Map<String, Value> =
            serde_json::from_value(json!({ "durum": "aşıyor", "taks": "0.5" })).unwrap();
        assert!(has(&attrs, &json!({ "durum": "aşıyor", "taks": 0.5 })));
        assert!(!has(&attrs, &json!({ "durum": "uygun" })));
        assert!(!has(&attrs, &json!({ "yok": 1 })));
        // How many passed, without a suffix after the number.
        assert_eq!(said(2, 2), "2 örneğin hepsi geçti.");
        assert_eq!(said(3, 2), "3 örnekten 2 tanesi geçti, 1 tanesi kaldı.");
        assert_eq!(said(1, 1), "Örnek geçti.");
        assert_eq!(said(2, 0), "2 örneğin hiçbiri geçmedi.");
    }
}
