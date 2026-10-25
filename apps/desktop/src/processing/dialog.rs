//! The state of one processing tool's or model's window (the web's
//! `ToolDialog`): the values as entered, which fields the user touched,
//! whether Çalıştır was pressed, the live parts (problems, what the inputs
//! resolve to, expression previews) and how the last run ended. How it
//! looks is [`super::window`]'s.

use std::collections::{BTreeMap, BTreeSet};

use kentos_domain::Slot;
use kentos_processing::model_runner::MODEL_PREFIX;
use kentos_processing::parameters::{default_values, restore_values};
use kentos_processing::{
    Defaults, InputSummary, Issue, Outcome, OutputKind, ParamKind, Registry, Runner, Scene, Target,
    Tool, Values,
};
use serde_json::{Value, json};

use super::Event;
use super::plan::{ServerPlace, StatusLine, Targets, auto_target, status_line};

/// How the last run ended, for the footer.
#[derive(Clone, Debug, PartialEq)]
pub enum RunStatus {
    Idle,
    /// It runs: the share done and the tool's step label (empty: none);
    /// `background`: on another thread, the window waiting (plan.rs).
    Running {
        fraction: f64,
        label: String,
        background: bool,
    },
    /// It ran. `pick`: what Sonuçları seç selects; `selected`: the run chose
    /// the selection itself; `undo`: it changed the drawing; `table`: the
    /// run's table output, shown under the form (docs/adr/0200 §7).
    Ok {
        text: String,
        pick: Vec<Slot>,
        selected: bool,
        undo: bool,
        table: Option<ResultTable>,
    },
    /// It did not start (nothing selected …).
    Invalid(String),
    /// It started and failed, or was stopped.
    Error(String),
    /// It was asked of the server as a job (docs/adr/0559 §5): what was said of it; the job goes on there.
    Asked(String),
}

/// A table a run gives (Özet istatistik): its columns and rows, texts.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl ResultTable {
    /// The table an output holds (`{ columns, rows }`); none when it is not one.
    pub fn read(v: &Value) -> Option<Self> {
        let texts = |v: &Value| -> Option<Vec<String>> {
            v.as_array()?
                .iter()
                .map(|c| c.as_str().map(str::to_owned))
                .collect()
        };
        Some(Self {
            columns: texts(v.get("columns")?)?,
            rows: v
                .get("rows")?
                .as_array()?
                .iter()
                .map(texts)
                .collect::<Option<_>>()?,
        })
    }

    /// The columns' row, then the rows: what Panoya kopyala and CSV olarak kaydet write.
    pub fn lines(&self) -> Vec<Vec<String>> {
        std::iter::once(self.columns.clone())
            .chain(self.rows.iter().cloned())
            .collect()
    }
}

/// A file parameter's value from a chosen file (docs/adr/0200 §7, 0223 §1):
/// its name, where it is and a sheet's rows, read within İşlemler's limits;
/// a workbook's sheets with rows (`sheets`), the one asked for (`sheet`,
/// else the first) giving `rows`; a sheet past the limits left out and said
/// (`leftOut`); or why it cannot be read.
pub fn file_value(
    file: &str,
    path: &std::path::Path,
    bytes: &[u8],
    sheet: Option<&str>,
) -> Result<Value, String> {
    use kentos_formats::table_file::{Limits, PROCESSING_COLUMNS, PROCESSING_ROWS, read_with};
    let read = read_with(bytes, Limits::PROCESSING);
    if let Some(problem) = read.problem {
        return Err(format!("“{file}” okunamadı: {problem}"));
    }
    let mut sheets = Vec::new();
    let mut left_out = Vec::new();
    for s in read.sheets {
        let name = s.name.unwrap_or_default();
        let which = if name.is_empty() {
            format!("“{file}”")
        } else {
            format!("“{file}” dosyasının “{name}” sayfası")
        };
        if s.cut {
            let most = grouped(PROCESSING_ROWS);
            left_out.push(format!(
                "{which} {most} satırdan uzun; İşlemler en çok {most} satır alır."
            ));
        } else if s.rows.iter().any(|r| r.len() > PROCESSING_COLUMNS) {
            left_out.push(format!(
                "{which} {PROCESSING_COLUMNS} sütundan geniş; İşlemler en çok {PROCESSING_COLUMNS} sütun alır."
            ));
        } else if !s.rows.is_empty() {
            sheets.push((name, s.rows));
        }
    }
    if sheets.is_empty() {
        return Err(match left_out.first() {
            Some(why) => format!("{why} Tabloyu bölün ya da süzün."),
            None => format!("“{file}” boş: okunacak satır yok."),
        });
    }
    let at = sheet
        .and_then(|n| sheets.iter().position(|(name, _)| name == n))
        .unwrap_or(0);
    let mut value = json!({
        "name": file,
        "path": path.to_string_lossy(),
        "rows": sheets[at].1,
    });
    if !sheets[at].0.is_empty() {
        value["sheet"] = json!(sheets[at].0);
    }
    if sheets.len() > 1 {
        value["sheets"] = json!(
            sheets
                .iter()
                .map(|(name, rows)| json!({ "name": name, "rows": rows }))
                .collect::<Vec<_>>()
        );
    }
    if !left_out.is_empty() {
        value["leftOut"] = json!(left_out);
    }
    Ok(value)
}

/// A count with its thousands apart: 250 000.
fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

/// The value of a workbook's file parameter with the sheet `name` chosen: its rows.
pub fn with_sheet(value: &Value, name: &str) -> Option<Value> {
    let rows = value["sheets"]
        .as_array()?
        .iter()
        .find(|s| s["name"] == name)?["rows"]
        .clone();
    let mut out = value.clone();
    out["rows"] = rows;
    out["sheet"] = json!(name);
    Some(out)
}

/// One tool's or model's window.
pub struct ToolDialog {
    /// The tool, or the model as a tool (`model:` id).
    pub tool: Tool,
    pub values: Values,
    /// Number fields' text as typed, so “12.” stays while typing.
    pub(super) numbers: BTreeMap<String, String>,
    /// A raster's name typed before Ekle adds its row (docs/adr/0237 §9), by parameter.
    pub(super) drafts: BTreeMap<String, String>,
    touched: BTreeSet<String>,
    /// After Çalıştır every problem shows, not only those of touched fields.
    attempted: bool,
    pub(super) advanced_open: bool,
    /// On the server (docs/adr/0559 §5): the result goes to a GeoPackage the job keeps, not to the project.
    pub(super) server_sink: bool,
    /// Sonuç here: the drawing, a GeoPackage or a database, and the inputs read whole (sink.rs, docs/adr/0592 §3).
    pub(super) sink: super::sink::LocalSink,
    pub status: RunStatus,
    /// The table the run gives to be drawn (`chart`: DBSCAN's sweep's k-distance graph): Grafik takes it before the
    /// table under the form (docs/adr/0639 §3).
    pub(super) chart: Option<ResultTable>,
    /// The background run this window shows (background.rs), while it runs.
    pub(super) waiting: Option<u64>,
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
            drafts: BTreeMap::new(),
            touched: BTreeSet::new(),
            attempted: false,
            advanced_open,
            server_sink: false,
            sink: super::sink::LocalSink::default(),
            status: RunStatus::Idle,
            chart: None,
            waiting: None,
            issues: Vec::new(),
            run_issues: Vec::new(),
            inputs: BTreeMap::new(),
            previews: BTreeMap::new(),
        }
    }

    /// A file the last values name (its path kept, not its rows) read again
    /// from its place; one that cannot be read is left to be chosen again.
    pub fn reread_files(&mut self) {
        for p in &self.tool.parameters {
            if !matches!(p.kind, ParamKind::File { .. }) {
                continue;
            }
            let Some(v) = self.values.get(&p.name) else {
                continue;
            };
            let (Some(file), Some(path), None) = (
                v.get("name").and_then(Value::as_str),
                v.get("path").and_then(Value::as_str),
                v.get("rows"),
            ) else {
                continue;
            };
            let path = std::path::PathBuf::from(path);
            let sheet = v.get("sheet").and_then(Value::as_str);
            if let Some(value) = std::fs::read(&path)
                .ok()
                .and_then(|bytes| file_value(file, &path, &bytes, sheet).ok())
            {
                self.values.insert(p.name.clone(), value);
            }
        }
    }

    /// Varsayılanlar: every field back to its default.
    pub fn reset(&mut self, defaults: &Defaults) {
        // It waits while a run is going (the web's footer disables it).
        if self.running() {
            return;
        }
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
        // A finished run's line clears; a running one stays (the web's).
        if !matches!(self.status, RunStatus::Idle | RunStatus::Running { .. }) {
            self.status = RunStatus::Idle;
        }
    }

    /// Whether a run is going.
    pub fn running(&self) -> bool {
        matches!(self.status, RunStatus::Running { .. })
    }

    /// A run started on another thread: the window shows run `id` until it ends.
    pub fn started(&mut self, id: u64) {
        self.waiting = Some(id);
        self.status = RunStatus::Running {
            fraction: 0.0,
            label: String::new(),
            background: true,
        };
    }

    /// How far the running run is, and the step it says.
    pub fn progressed(&mut self, share: f64, step: String) {
        if let RunStatus::Running {
            fraction, label, ..
        } = &mut self.status
        {
            *fraction = share;
            *label = step;
        }
    }

    fn set(&mut self, name: String, value: Value) {
        self.touch(&name);
        self.values.insert(name, value);
    }

    /// An edit of the form; `active`: the active layer, a new layer scope's first.
    pub fn edit(&mut self, e: Event, active: &str) {
        match e {
            Event::Value(name, value) => {
                self.drafts.remove(&name);
                self.set(name, value)
            }
            Event::Draft(name, text) => {
                self.drafts.insert(name, text);
            }
            Event::Number(name, text) => {
                let read = text.trim().replace(',', ".");
                // An optional field left empty is no value (docs/adr/0205 §2: the project's height).
                let optional = self
                    .tool
                    .parameters
                    .iter()
                    .any(|p| p.name == name && p.optional);
                let value = if optional && read.is_empty() {
                    Value::Null
                } else {
                    read.parse::<f64>()
                        .ok()
                        .filter(|n| n.is_finite() && !read.is_empty())
                        .map_or_else(|| Value::String(text.clone()), |n| json!(n))
                };
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
        self.waiting = None;
        match outcome {
            Outcome::Ok {
                result,
                added,
                touched,
                edited,
                record,
                ..
            } => {
                let table = self
                    .tool
                    .outputs
                    .iter()
                    .find(|o| o.kind == OutputKind::Table)
                    .and_then(|o| result.outputs.get(&o.name))
                    .and_then(ResultTable::read);
                self.chart = result.outputs.get("chart").and_then(ResultTable::read);
                self.status = RunStatus::Ok {
                    text: record.summary,
                    pick: if added.is_empty() { touched } else { added },
                    selected: result.select.is_some(),
                    undo: edited,
                    table,
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

    /// Nerede çalışır for this window (plan.rs): the places the tool names
    /// (a model: every place one of its steps can go here, the web's), those
    /// this program has, and where Otomatik sends the inputs now.
    pub(super) fn targets(&self, registry: &Registry, server: &ServerPlace) -> Targets {
        let here = |t: &Target| matches!(t, Target::Client | Target::Worker);
        let model = self
            .tool
            .id
            .strip_prefix(MODEL_PREFIX)
            .and_then(|id| registry.model(id));
        let (declared, available) = match model {
            Some(model) => {
                let mut places: Vec<Target> = Vec::new();
                for step in &model.steps {
                    for t in registry
                        .tool(&step.tool)
                        .map(|t| t.targets)
                        .unwrap_or_default()
                    {
                        if here(&t) && !places.contains(&t) {
                            places.push(t);
                        }
                    }
                }
                (places.clone(), places)
            }
            None => {
                // The server's place is the open cloud project's, for every tool (docs/adr/0559 §5).
                let mut declared = self.tool.targets.clone();
                let mut available: Vec<Target> =
                    self.tool.targets.iter().copied().filter(here).collect();
                match server {
                    ServerPlace::Absent => {}
                    ServerPlace::Ready => {
                        declared.retain(|t| *t != Target::Server);
                        declared.push(Target::Server);
                        available.push(Target::Server);
                    }
                    ServerPlace::Off(_) => {
                        declared.retain(|t| *t != Target::Server);
                        declared.push(Target::Server);
                    }
                }
                (declared, available)
            }
        };
        let size = self.inputs.values().map(|s| s.count).sum();
        Targets {
            auto: auto_target(&available, size),
            declared,
            available,
            model: model.is_some(),
            server_off: match server {
                ServerPlace::Off(why) => Some(why.clone()),
                _ => None,
            },
        }
    }

    /// The line beside the buttons (the web's plan, plan.rs): how the run
    /// goes or ended, or what to fix; the run's refusal comes first.
    pub(super) fn line(&self) -> StatusLine {
        let issues: Vec<Issue> = self
            .run_issues
            .iter()
            .chain(&self.issues)
            .cloned()
            .collect();
        status_line(&self.status, self.attempted, &issues)
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
