//! Kapı 3's part that runs code (plan/eklenti.md ADIM 21; docs/adr/8057 §37): an agent's trial of an installed
//! extension. A synthetic drawing is made of the objects the agent gives (a new GIS drawing on TUREF / TM36, their
//! layers by name), never the user's; the extension's audit rules read its facts and its İşlemler tools run on it, one
//! after another in the background, in the extension's own process. The report gives each rule's findings by the
//! object's place in the agent's list, each tool's written layers and objects, or the extension's error with its
//! traceback.
//!
//! - **Only what the user installed runs:** the extension installed and its environment ready; a tool of another
//!   extension is refused. The user's own run (the ribbon's) is not needed: the user installed it.
//! - **One trial at a time;** Durdur (the agent's turn stopped) stops it between its steps and the running rule.
//! - **Nothing of the user's drawing:** the synthetic drawing lives in the background task and goes with it.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use kentos_domain::Uuid;
use serde_json::{Value, json};

use super::audits::{Refusal, findings_of};
use super::{Event, msg};
use crate::app::App;

mod drawing;

use drawing::written;
pub(crate) use drawing::{Synthetic, synthetic, synthetic_on};

/// How long a rule may take in a trial: as on the drawing.
const RULE_TIMEOUT: Duration = Duration::from_secs(120);

/// The last trial: while it runs, what stops it; once it ran, its report.
#[derive(Clone, Debug)]
pub(crate) struct TrialState {
    pub(crate) ext: String,
    pub(crate) running: Option<Arc<AtomicBool>>,
    pub(crate) report: Option<Value>,
    /// The user started it (the tool examples from Eklentiler, examples.rs): a toast and the log say how it went.
    pub(crate) by_user: bool,
}

/// A rule's findings as a trial reports them: each by the object's place in the agent's list (`feature`), its fix's
/// title and command when it has one.
fn findings_report(done: &Value, places: &BTreeMap<String, usize>) -> Value {
    match findings_of(done) {
        Ok((findings, dropped)) => {
            let list: Vec<Value> = findings
                .iter()
                .map(|f| {
                    json!({
                        "severity": f.severity.code(),
                        "message": f.message,
                        "feature": f.feature.as_ref().and_then(|u| places.get(u)),
                        "fix": f.fix.as_ref().map(|x| json!({ "title": x.title, "command": x.command })),
                    })
                })
                .collect();
            json!({ "status": "done", "findings": list, "dropped": dropped })
        }
        Err((error, traceback)) => {
            json!({ "status": "failed", "error": error, "traceback": traceback })
        }
    }
}

/// One of the trial's rules: its id, title and the run in its extension's process.
type Rule = (String, String, super::tools::PoolJob);

impl App {
    /// `trial` (an agent's `extension.workshop.trial`): the synthetic drawing made and the extension's rules and tools
    /// started on it in the background; its task waits in `Extensions::trial_job` for the runtime. Why not, at once.
    pub(crate) fn ext_trial_start(&mut self, params: &Value) -> Result<Value, Refusal> {
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
        let ext = params
            .get("id")
            .and_then(Value::as_str)
            .ok_or((
                "invalid_input",
                "id gerekli: eklentinin kimliği.".to_owned(),
            ))?
            .to_owned();
        let manifest = self
            .extensions
            .registry
            .as_ref()
            .and_then(|r| r.manifest(&ext))
            .cloned()
            .ok_or_else(|| {
                (
                    "unknown_extension",
                    format!(
                        "“{ext}” kurulu bir eklenti değil: önce extension.workshop.review ile kullanıcının kurmasına \
                         açın."
                    ),
                )
            })?;
        if let Some(Some(why)) = self.extensions.python_reasons.get(&ext) {
            return Err(("not_ready", format!("Eklentinin kodu çalışmıyor: {why}")));
        }
        // Its tools' own examples, on drawings of their own (docs/adr/8057 §48): no objects of the agent's.
        if params.get("examples") == Some(&Value::Bool(true)) {
            let (started, task) = self.ext_examples_start(&ext, false)?;
            self.extensions.trial_job = Some(task);
            return Ok(started);
        }
        let features = params
            .get("features")
            .and_then(Value::as_array)
            .filter(|f| {
                !f.is_empty() && f.len() <= kentos_headless::agent_tools::TRIAL_MOST_FEATURES
            })
            .ok_or((
                "invalid_input",
                format!(
                    "features 1 ile {} arasında nesne olmalı.",
                    kentos_headless::agent_tools::TRIAL_MOST_FEATURES
                ),
            ))?;
        let made = synthetic(features).map_err(|why| ("invalid_input", why))?;
        // The rules: the ones asked for, else all of the extension's.
        let asked: Option<Vec<&str>> = params
            .get("audits")
            .and_then(Value::as_array)
            .map(|l| l.iter().filter_map(Value::as_str).collect());
        let mut rules: Vec<Rule> = Vec::new();
        for audit in &manifest.entrypoints.audits {
            if asked
                .as_ref()
                .is_some_and(|a| !a.contains(&audit.id.as_str()))
            {
                continue;
            }
            let job = self
                .pool_job(&ext, &audit.id, &audit.handler)
                .map_err(|why| ("not_ready", why))?;
            rules.push((format!("ext.{ext}.{}", audit.id), audit.title.clone(), job));
        }
        if let Some(missing) = asked.as_ref().and_then(|a| {
            a.iter()
                .find(|id| !manifest.entrypoints.audits.iter().any(|x| x.id == **id))
        }) {
            return Err((
                "invalid_input",
                format!("“{missing}” bu eklentinin bir denetimi değil (entrypoints.audits)."),
            ));
        }
        // The tools: only this extension's.
        let prefix = format!("ext.{ext}.");
        let tools: Vec<Value> = params
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if tools.len() > kentos_headless::agent_tools::TRIAL_MOST_TOOLS {
            return Err((
                "invalid_input",
                format!(
                    "En çok {} araç denenir.",
                    kentos_headless::agent_tools::TRIAL_MOST_TOOLS
                ),
            ));
        }
        if let Some(other) = tools.iter().find(|t| {
            !t.get("tool")
                .and_then(Value::as_str)
                .is_some_and(|id| id.starts_with(&prefix))
        }) {
            return Err((
                "invalid_input",
                format!(
                    "Deneme yalnız bu eklentinin araçlarını çalıştırır ({prefix}…); verilen: {}.",
                    other.get("tool").unwrap_or(&Value::Null)
                ),
            ));
        }
        if rules.is_empty() && tools.is_empty() {
            return Err((
                "invalid_input",
                "Denenecek bir şey yok: eklentinin denetimi yok ve araç verilmedi.".to_owned(),
            ));
        }
        let stop = Arc::new(AtomicBool::new(false));
        self.extensions.trial = Some(TrialState {
            ext: ext.clone(),
            running: Some(Arc::clone(&stop)),
            report: None,
            by_user: false,
        });
        let head = json!({
            "extension": { "id": ext, "version": manifest.version.to_string() },
            "drawing": {
                "objects": made.uids.len(),
                "layers": made.layers,
                "note": "Sentetik çizim: kullanıcının çizimine dokunulmadı; deneme bitince silindi.",
            },
        });
        let task = crate::python_setup::in_background_deep(
            move || run(made, rules, tools, head, &stop),
            json!({ "status": "failed", "error": "Deneme koşusu cevapsız kaldı." }),
            |report| msg(Event::TrialDone(Box::new(report))),
        );
        self.extensions.trial_job = Some(task);
        Ok(json!({ "status": "running", "extension": ext }))
    }

    /// `trial_report`: the last trial as an agent reads it.
    pub(crate) fn ext_trial_report(&self) -> Value {
        match &self.extensions.trial {
            None => json!({ "status": "none" }),
            Some(t) if t.running.is_some() => json!({ "status": "running", "extension": t.ext }),
            Some(t) => t
                .report
                .clone()
                .unwrap_or_else(|| json!({ "status": "none" })),
        }
    }

    /// The trial ended: its report kept, the agent's call waiting for it answered.
    pub(crate) fn ext_trial_done(&mut self, report: Value) {
        let by_user = self.extensions.trial.as_ref().is_some_and(|t| t.by_user);
        if by_user && report["kind"] == "examples" {
            self.ext_examples_said(&report);
        }
        if let Some(t) = self.extensions.trial.as_mut() {
            t.running = None;
            t.report = Some(report);
        }
        self.assistant_trial_ended();
    }

    /// Durdur: the running trial stops after the step it is in (a rule's process asked to stop at once).
    pub(crate) fn ext_trial_stop(&mut self) {
        if let Some(stop) = self
            .extensions
            .trial
            .as_ref()
            .and_then(|t| t.running.as_ref())
        {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

/// The trial's steps, in the background: each rule on the drawing's facts, then each tool on the drawing; the report.
fn run(
    made: Synthetic,
    rules: Vec<Rule>,
    tools: Vec<Value>,
    head: Value,
    stop: &AtomicBool,
) -> Value {
    let Synthetic { mut doc, uids, .. } = made;
    let places: BTreeMap<String, usize> = uids
        .iter()
        .enumerate()
        .map(|(i, u)| (u.to_string(), i))
        .collect();
    let stopped = || stop.load(Ordering::Relaxed);
    // The rules' facts as the drawing's own audit and a cloud job give them (`audit_job::facts_of`, docs/adr/8057 §36,
    // §42): the layers' objects and what KentOS measured; a measure that breaks leaves the rules the layers.
    let facts = if rules.is_empty() {
        Value::Null
    } else {
        match kentos_processing::audit_job::facts_of(&doc, &stopped) {
            Some(facts) => facts,
            None => return json!({ "status": "stopped" }),
        }
    };
    let mut audits = Vec::new();
    for (rule, title, job) in rules {
        if stopped() {
            return json!({ "status": "stopped" });
        }
        let began = Instant::now();
        let mut out = match job.run(
            kentos_processing::audit_job::message(facts.clone()),
            RULE_TIMEOUT,
            &stopped,
        ) {
            Ok(done) => findings_report(&done, &places),
            Err(why) => json!({ "status": "failed", "error": why }),
        };
        out["rule"] = json!(rule);
        out["title"] = json!(title);
        out["ms"] = json!(began.elapsed().as_millis() as u64);
        audits.push(out);
    }
    let mut ran = Vec::new();
    for input in tools {
        if stopped() {
            return json!({ "status": "stopped" });
        }
        let tool = input.get("tool").cloned().unwrap_or(Value::Null);
        let before: BTreeSet<Uuid> = doc.entities_with_uids().map(|(u, _)| u).collect();
        let envelope = serde_json::from_value::<kentos_commands::CallEnvelope>(json!({
            "command": "cad.processing.run", "op": "execute", "input": input
        }));
        let out = match envelope {
            Err(e) => {
                json!({ "tool": tool, "status": "failed", "error": format!("Girdi okunamadı: {e}") })
            }
            Ok(envelope) => {
                let answer = kentos_headless::call_envelope(&mut doc, None, envelope);
                let value = serde_json::to_value(&answer).unwrap_or(Value::Null);
                if value["status"] == "completed" {
                    json!({
                        "tool": tool,
                        "status": "done",
                        "summary": value.get("summary"),
                        "warnings": value.get("warnings"),
                        "written": written(&doc, &before),
                    })
                } else {
                    json!({ "tool": tool, "status": "failed", "error": value.get("error").cloned().unwrap_or(value) })
                }
            }
        };
        ran.push(out);
    }
    let found: usize = audits
        .iter()
        .filter_map(|a| a["findings"].as_array().map(Vec::len))
        .sum();
    let failed = audits.iter().filter(|a| a["status"] == "failed").count()
        + ran.iter().filter(|t| t["status"] == "failed").count();
    let mut report = head;
    report["status"] = json!("done");
    report["summary"] = json!(format!(
        "{} kural, {found} bulgu; {} araç; {failed} adım hata verdi.",
        audits.len(),
        ran.len()
    ));
    report["audits"] = json!(audits);
    report["tools"] = json!(ran);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rule's answer reported by the objects' places: the finding's object is its place in the agent's list, its fix
    /// named; a rule that failed says why with its traceback.
    #[test]
    fn findings_are_reported_by_the_objects_places() {
        let places = BTreeMap::from([("u-1".to_owned(), 0), ("u-2".to_owned(), 1)]);
        let done = json!({ "ok": true, "result": { "kind": "findings", "findings": [
            { "severity": "error", "message": "Mezar numarası yok.", "featureId": "u-2",
              "fix": { "title": "Numara ver", "command": "cad.entities.set", "args": {} } },
            { "severity": "info", "message": "Genel not." }
        ] } });
        let report = findings_report(&done, &places);
        assert_eq!(report["status"], "done");
        assert_eq!(report["findings"][0]["feature"], 1);
        assert_eq!(report["findings"][0]["fix"]["command"], "cad.entities.set");
        assert_eq!(report["findings"][1]["feature"], Value::Null);
        let failed = json!({ "ok": false, "exception": "KeyError", "message": "'no'", "error": "Traceback …" });
        let report = findings_report(&failed, &places);
        assert_eq!(report["status"], "failed");
        assert_eq!(report["error"], "KeyError: 'no'");
        assert_eq!(report["traceback"], "Traceback …");
    }
}
