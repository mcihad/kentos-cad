//! The state of one processing tool's or model's window (the web's
//! `ToolDialog`): the values as entered, which fields the user touched,
//! whether Çalıştır was pressed, the live parts (problems, what the inputs
//! resolve to, expression previews) and how the last run ended. How it
//! looks is [`super::window`]'s.

use std::collections::{BTreeMap, BTreeSet};

use kentos_domain::Slot;
use kentos_processing::parameters::{default_values, restore_values};
use kentos_processing::{
    Defaults, InputSummary, Issue, Outcome, ParamKind, Runner, Scene, Tool, Values,
};
use serde_json::{Value, json};

use super::Event;

/// How the last run ended, for the footer.
#[derive(Clone, Debug, PartialEq)]
pub enum RunStatus {
    Idle,
    /// It ran. `pick`: what Sonuçları seç selects; `selected`: the run chose
    /// the selection itself; `undo`: it changed the drawing.
    Ok {
        text: String,
        pick: Vec<Slot>,
        selected: bool,
        undo: bool,
    },
    /// It did not start (nothing selected …).
    Invalid(String),
    /// It started and failed, or was stopped.
    Error(String),
}

/// One tool's or model's window.
pub struct ToolDialog {
    /// The tool, or the model as a tool (`model:` id).
    pub tool: Tool,
    pub values: Values,
    /// Number fields' text as typed, so “12.” stays while typing.
    pub(super) numbers: BTreeMap<String, String>,
    touched: BTreeSet<String>,
    /// After Çalıştır every problem shows, not only those of touched fields.
    attempted: bool,
    pub(super) advanced_open: bool,
    pub status: RunStatus,
    /// Problems of the values (the runner's validation).
    pub(super) issues: Vec<Issue>,
    /// Problems a run found before starting (nothing selected), until the next edit.
    run_issues: Vec<Issue>,
    /// What each features parameter resolves to now.
    pub(super) inputs: BTreeMap<String, InputSummary>,
    /// Each expression parameter's line (“16 / 340 nesne koşulu sağlıyor.”).
    pub(super) previews: BTreeMap<String, String>,
}

impl ToolDialog {
    /// The window with stored values over the defaults. Gelişmiş ayarlar
    /// opens when values given on purpose (history's Yeniden aç) differ
    /// there from the defaults.
    pub fn new(tool: Tool, stored: Option<&Values>, explicit: bool, defaults: &Defaults) -> Self {
        let values = restore_values(&tool, stored, defaults);
        let base = default_values(&tool, defaults);
        let advanced_open = explicit
            && tool.parameters.iter().any(|p| {
                p.advanced
                    && stored
                        .and_then(|s| s.get(&p.name))
                        .is_some_and(|v| Some(v) != base.get(&p.name))
            });
        Self {
            tool,
            values,
            numbers: BTreeMap::new(),
            touched: BTreeSet::new(),
            attempted: false,
            advanced_open,
            status: RunStatus::Idle,
            issues: Vec::new(),
            run_issues: Vec::new(),
            inputs: BTreeMap::new(),
            previews: BTreeMap::new(),
        }
    }

    /// Varsayılanlar: every field back to its default.
    pub fn reset(&mut self, defaults: &Defaults) {
        self.values = default_values(&self.tool, defaults);
        self.numbers.clear();
        self.touched.clear();
        self.attempted = false;
        self.run_issues.clear();
        self.status = RunStatus::Idle;
    }

    /// The live parts again, from the drawing as it is.
    pub fn refresh(&mut self, runner: &Runner, scene: &dyn Scene) {
        self.issues = runner.validate(&self.tool, &self.values, scene.doc());
        self.inputs = runner.describe_inputs(&self.tool, &self.values, scene);
        self.previews = self
            .tool
            .parameters
            .iter()
            .filter(|p| matches!(p.kind, ParamKind::Expression { .. }))
            .filter_map(|p| {
                runner
                    .preview_expression(&self.tool, &self.values, &p.name, scene)
                    .map(|line| (p.name.clone(), line))
            })
            .collect();
    }

    /// A field the user changed: its problems show from now on.
    pub fn touch(&mut self, name: &str) {
        self.touched.insert(name.to_owned());
        self.run_issues.clear();
        if !matches!(self.status, RunStatus::Idle) {
            self.status = RunStatus::Idle;
        }
    }

    fn set(&mut self, name: String, value: Value) {
        self.touch(&name);
        self.values.insert(name, value);
    }

    /// An edit of the form; `active`: the active layer, a new layer scope's first.
    pub fn edit(&mut self, e: Event, active: &str) {
        match e {
            Event::Value(name, value) => self.set(name, value),
            Event::Number(name, text) => {
                let read = text.trim().replace(',', ".");
                let value = read
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite() && !read.is_empty())
                    .map_or_else(|| Value::String(text.clone()), |n| json!(n));
                self.numbers.insert(name.clone(), text);
                self.set(name, value);
            }
            Event::Text(name, text) => {
                let max = self.tool.parameters.iter().find_map(|p| match &p.kind {
                    ParamKind::Text { max_length, .. } if p.name == name => *max_length,
                    _ => None,
                });
                // The web's `maxLength`: a longer text is not taken.
                if max.is_some_and(|m| text.encode_utf16().count() > m) {
                    return;
                }
                self.set(name, Value::String(text));
            }
            Event::LayerName(name, text) => self.set(name, json!({ "newName": text })),
            Event::Advanced => self.advanced_open = !self.advanced_open,
            Event::Kind(name, kind) => self.toggle_kind(name, &kind),
            Event::Insert(name, text) => {
                let before = self
                    .values
                    .get(&name)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                let pad = if before.is_empty() || before.ends_with([' ', '(', ',']) {
                    ""
                } else {
                    " "
                };
                self.set(name, Value::String(format!("{before}{pad}{text}")));
            }
            Event::Scope(name, scope) => {
                let kinds = self.values.get(&name).and_then(|v| v.get("kinds")).cloned();
                let mut value = if scope == "layer" {
                    json!({ "scope": "layer", "layerId": active })
                } else {
                    json!({ "scope": scope })
                };
                if let (Some(kinds), Some(o)) = (kinds, value.as_object_mut()) {
                    o.insert("kinds".into(), kinds);
                }
                self.set(name, value);
            }
            _ => {}
        }
    }

    /// A kind chip pressed (the web's): the kind leaves or joins the run;
    /// when every kind in scope is taken again, the filter goes.
    fn toggle_kind(&mut self, name: String, kind: &str) {
        let present: Vec<String> = self
            .inputs
            .get(&name)
            .map(|s| s.by_kind.iter().map(|(k, _)| k.clone()).collect())
            .unwrap_or_default();
        let Some(mut value) = self.values.get(&name).cloned() else {
            return;
        };
        let chosen: Vec<String> = value
            .get("kinds")
            .and_then(Value::as_array)
            .map(|k| {
                k.iter()
                    .filter_map(|k| k.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_else(|| present.clone());
        let mut next: BTreeSet<String> = chosen.into_iter().collect();
        if !next.remove(kind) {
            next.insert(kind.to_owned());
        }
        let Some(o) = value.as_object_mut() else {
            return;
        };
        if present.iter().all(|k| next.contains(k)) {
            o.remove("kinds");
        } else {
            let kept: Vec<&String> = present.iter().filter(|k| next.contains(*k)).collect();
            o.insert("kinds".into(), json!(kept));
        }
        self.set(name, value);
    }

    /// Çalıştır pressed: every problem shows; true when the tool may run.
    pub fn attempt(&mut self) -> bool {
        self.attempted = true;
        self.run_issues.clear();
        if self.issues.is_empty() {
            return true;
        }
        let advanced = |i: &Issue| {
            self.tool
                .parameters
                .iter()
                .any(|p| p.advanced && Some(&p.name) == i.param.as_ref())
        };
        self.advanced_open |= self.issues.iter().any(advanced);
        false
    }

    /// How the run ended.
    pub fn ran(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Ok {
                result,
                added,
                touched,
                edited,
                record,
            } => {
                self.status = RunStatus::Ok {
                    text: record.summary,
                    pick: if added.is_empty() { touched } else { added },
                    selected: result.select.is_some(),
                    undo: edited,
                };
                self.attempted = false;
            }
            Outcome::Invalid { issues } => {
                self.status = RunStatus::Invalid(
                    issues
                        .first()
                        .map(|i| i.message.clone())
                        .unwrap_or_default(),
                );
                self.run_issues = issues;
            }
            Outcome::Stopped { message, .. } => self.status = RunStatus::Error(message),
        }
    }

    /// The problem shown under a field: live for touched fields, all of
    /// them after Çalıştır.
    pub(super) fn issue_of(&self, name: &str) -> Option<&Issue> {
        if !self.attempted && !self.touched.contains(name) {
            return None;
        }
        self.run_issues
            .iter()
            .chain(&self.issues)
            .find(|i| i.param.as_deref() == Some(name))
    }

    /// The footer's warning: the tool's own rule, or how many fields to fix.
    pub(super) fn warning(&self) -> Option<String> {
        if !self.attempted {
            return None;
        }
        let all = || self.run_issues.iter().chain(&self.issues);
        if let Some(own) = all().find(|i| i.param.is_none()) {
            return Some(own.message.clone());
        }
        let fields = all().filter(|i| i.param.is_some()).count();
        if fields > 0 {
            return Some(format!("Çalıştırmadan önce {fields} alanı düzeltin."));
        }
        match &self.status {
            RunStatus::Invalid(text) => Some(text.clone()),
            _ => None,
        }
    }

    /// The tool's preview line, or why there is none (the web's `renderPreview`).
    pub(super) fn preview(&self) -> Option<String> {
        let valid = !self.issues.iter().any(|i| i.param.is_some());
        if !valid {
            return Some("Önizleme için alanları düzeltin.".into());
        }
        self.tool.preview.and_then(|p| p(&self.values))
    }
}
