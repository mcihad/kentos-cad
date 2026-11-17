//! A trial's command or map tool as it runs (docs/adr/8057 §52): what its code asks answered on the synthetic drawing
//! as KentOS answers it on the user's (runner.rs's broker), and what it did read back for the report.
//!
//! - **The same rules:** reading with `document.read`, writing with `document.write`; the first write opens the step's
//!   undo step (named as its code names it, else after the step), taken back when the code fails or is stopped or a
//!   group of it fails, so the next step sees the drawing without it; another extension's tool only for a declared
//!   dependency; undo, redo and files never.
//! - **Its dialogs answered by the agent:** checked as the window checks them (dialogs.rs `dialog_panel`), then the
//!   agent's next answer set in their fields as the code's `panel.set` sets them and checked as Tamam checks them. An
//!   answer not given, or one that does not fit, is İptal, and the report says which; a dialog with a file picker is
//!   İptal too: the agent does not choose the user's files.
//! - **Its panels:** opened, changed and closed as on the user's screen, by panel_session.rs's own checks, and kept for
//!   the next steps; the report gives each as the step left it.
//! - **Bounded:** at most [`MOST_REFUSED`] refused calls and [`KEPT_OUTPUT`] bytes of what it printed are kept.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use kentos_domain::{Document, Group, Uuid};
use kentos_extensions::ui_panel::Component;
use kentos_headless::HeadlessError;
use kentos_interaction::Selection;
use kentos_python_env::process::{Answer, Heard};
use serde_json::{Map, Value, json};

use super::super::panel_session::{self, no_panel, unknown_panel};
use super::super::panels::OpenPanel;
use super::super::runner;
use super::drawing::{layer_name, written};
use super::steps::{Rights, Spec};
use crate::exchange::words;

/// The most refused calls a step's report keeps.
const MOST_REFUSED: usize = 20;
/// What a step's report keeps of what it printed.
const KEPT_OUTPUT: usize = 8 * 1024;
/// The objects of each kind (changed, a table's rows) a report shows; the rest are counted.
const SAMPLE: usize = 5;

/// What runs a step's code: its invoke, whether it is given the drawing, the answers to its calls and what it says;
/// the process's `done`, or why there is none (tools.rs `PoolJob::run_step`).
pub(crate) type Call<'a> =
    dyn FnMut(Value, bool, &mut Answer<'_>, &mut dyn FnMut(Heard)) -> Result<Value, String> + 'a;

/// The drawing a step runs on and what the trial keeps between its steps.
pub(crate) struct Ground<'a> {
    pub(crate) doc: &'a mut Document,
    pub(crate) rights: &'a Rights,
    /// The panels open in the trial, by id.
    pub(crate) panels: &'a mut BTreeMap<String, OpenPanel>,
    /// The agent's objects: each persistent id's place in its list, and the ids in that order.
    pub(crate) places: &'a BTreeMap<String, usize>,
    pub(crate) uids: &'a [Uuid],
}

/// Runs `spec` on `ground`, its code started by `call`: the step's report.
pub(crate) fn run(ground: Ground<'_>, spec: &Spec, call: &mut Call<'_>) -> Value {
    let began = Instant::now();
    let Ground {
        doc,
        rights,
        panels,
        places,
        uids,
    } = ground;
    let mut report = json!({ "title": spec.title });
    report[if spec.map_tool { "mapTool" } else { "command" }] = json!(spec.id);
    if let Some(origin) = &spec.panel {
        report["from"] = json!({ "panel": origin.panel, "button": origin.button });
    }
    let srid = doc.settings().srid;
    // A panel's button: the agent's values set in the panel and checked as the button checks them.
    let mut params = spec.params.clone();
    if let Some(origin) = &spec.panel {
        let layers = layer_ids(doc);
        let filled = rights
            .panels
            .get(&origin.panel)
            .cloned()
            .flatten()
            .ok_or_else(|| format!("“{}” panelinin tanımı okunamadı.", origin.panel))
            .and_then(|definition| {
                let panel = panels.entry(origin.panel.clone()).or_insert_with(|| {
                    let mut panel = OpenPanel::new(&rights.ext, &origin.panel, definition);
                    panel.default_crs(Some(srid));
                    panel
                });
                let given = spec.params.as_object().cloned().unwrap_or_default();
                let values = fill(panel, &given, &layers)?;
                panel.said = None;
                panel.spoke = false;
                Ok(values)
            });
        match filled {
            Ok(values) => params = values,
            Err(why) => {
                report["status"] = json!("invalid");
                report["error"] = json!(format!("Panelin değerleri: {why}"));
                if let Some(panel) = panels.get(&origin.panel) {
                    report["panels"] = json!([state(panel)]);
                }
                return report;
            }
        }
    }
    let mut invoke = json!({
        "kind": if spec.map_tool { "mapTool" } else { "command" },
        "params": params,
        "settings": {},
    });
    if let Some(origin) = &spec.panel {
        invoke["panel"] = json!(origin.panel);
    }
    let before: BTreeMap<Uuid, String> = doc
        .entities_with_uids()
        .map(|(u, e)| (u, serde_json::to_string(e).unwrap_or_default()))
        .collect();
    let mut selection = Selection::new();
    selection.set(
        spec.selection
            .iter()
            .filter_map(|at| uids.get(*at).and_then(|u| doc.slot_of(*u))),
    );
    let mut broker = Broker {
        doc,
        rights,
        panels,
        origin: spec.panel.as_ref().map(|o| o.panel.as_str()),
        title: &spec.title,
        srid,
        selection,
        selected: false,
        dialogs: &spec.dialogs,
        asked: Vec::new(),
        refused: Vec::new(),
        touched: BTreeSet::new(),
        group: None,
        label: None,
        poisoned: false,
    };
    let mut output = String::new();
    let done = call(
        invoke,
        rights.read(),
        &mut |method: &str, params: &Value| broker.answer(method, params),
        &mut |heard: Heard| match heard {
            Heard::Out { text, .. } => keep(&mut output, &text),
            Heard::Log { level, text } => keep(&mut output, &format!("[{level}] {text}\n")),
            Heard::Progress { .. } => {}
        },
    );
    // Its undo step kept when its code answered and no group of it failed; else taken back, as on the user's drawing.
    let ok = done.as_ref().is_ok_and(|d| d["ok"] == true);
    let keep_it = ok && !broker.poisoned;
    let undone = match broker.group.take() {
        Some(group) => {
            let wrote = broker.doc.group_changes() > 0;
            if keep_it {
                broker.doc.end_group(group);
            } else {
                broker.doc.cancel_group(group);
            }
            wrote && !keep_it
        }
        None => false,
    };
    let Broker {
        doc,
        panels,
        asked,
        refused,
        touched,
        selection,
        selected,
        ..
    } = broker;
    let mut touched = touched;
    match &done {
        Err(why) => {
            report["status"] = json!("failed");
            report["error"] = json!(why);
        }
        Ok(d) if ok => {
            report["status"] = json!("done");
            let result = d.get("result").cloned().unwrap_or(Value::Null);
            outcome(&mut report, &result);
        }
        Ok(d) => {
            let said = d.get("message").and_then(Value::as_str).unwrap_or_default();
            let status = if d["stopped"] == true {
                "stopped"
            } else if d["unavailable"] == true {
                "unavailable"
            } else if d["refused"] == true {
                "refused"
            } else {
                "failed"
            };
            report["status"] = json!(status);
            if status == "failed" {
                let exception = d.get("exception").and_then(Value::as_str).unwrap_or("hata");
                report["error"] = json!(format!("{exception}: {said}"));
                report["traceback"] = d.get("error").cloned().unwrap_or(Value::Null);
            } else if !said.is_empty() {
                report["message"] = json!(said);
            }
        }
    }
    // A panel's button: its answer on the panel, as the user would see it (runner.rs `ext_done`, panels.rs).
    if let Some(origin) = &spec.panel
        && let Some(panel) = panels.get_mut(&origin.panel)
    {
        match &done {
            Ok(d) if ok => {
                let table = panel.table_of(origin.table.as_ref());
                panel.take_answer(table.as_ref(), d.get("result").unwrap_or(&Value::Null));
            }
            Ok(d) if d["stopped"] == true => {
                panel.said = Some((
                    words::Kind::Info,
                    "Durduruldu; çizime yazdıkları geri alındı.".to_owned(),
                ));
            }
            _ => {
                let said = report["error"]
                    .as_str()
                    .or(report["message"].as_str())
                    .unwrap_or_default();
                panel.said = Some((
                    words::Kind::Error,
                    format!("“{}” çalışmadı: {said}", spec.title),
                ));
            }
        }
        touched.insert(origin.panel.clone());
    }
    let keys: BTreeSet<Uuid> = before.keys().copied().collect();
    let made = written(doc, &keys);
    if !made.is_empty() {
        report["written"] = json!(made);
    }
    changes(doc, &before, places, &mut report);
    if undone {
        report["undone"] = json!(true);
    }
    if selected {
        let chosen: Vec<Value> = selection
            .ids()
            .iter()
            .filter_map(|slot| doc.uid(*slot))
            .map(|u| json!(places.get(&u.to_string())))
            .collect();
        report["selection"] = json!(chosen);
    }
    if !asked.is_empty() {
        report["dialogs"] = json!(asked);
    }
    if !refused.is_empty() {
        report["refusedCalls"] = json!(refused);
    }
    let shown: Vec<Value> = touched
        .iter()
        .filter_map(|p| panels.get(p))
        .map(state)
        .collect();
    if !shown.is_empty() {
        report["panels"] = json!(shown);
    }
    if !output.is_empty() {
        report["output"] = json!(output);
    }
    report["ms"] = json!(began.elapsed().as_millis() as u64);
    report
}

/// What the code answered, in the report: its words, its title and its rows (counted, the first few shown).
fn outcome(report: &mut Value, result: &Value) {
    for key in ["message", "title"] {
        if let Some(words) = result.get(key).and_then(Value::as_str) {
            report[key] = json!(words);
        }
    }
    if let Some(rows) = result.get("rows").and_then(Value::as_array) {
        report["rows"] = json!({
            "columns": result.get("columns"),
            "count": rows.len(),
            "sample": rows.iter().take(SAMPLE * 2).collect::<Vec<_>>(),
        });
    }
    if result.get("message").is_none() && result.get("rows").is_none() && !result.is_null() {
        report["result"] = result.clone();
    }
}

/// The objects a step changed or removed, by their place in the agent's list (none: one an earlier step wrote).
fn changes(
    doc: &Document,
    before: &BTreeMap<Uuid, String>,
    places: &BTreeMap<String, usize>,
    report: &mut Value,
) {
    let mut changed = 0usize;
    let mut sample = Vec::new();
    let mut kept: BTreeSet<Uuid> = BTreeSet::new();
    for (uid, e) in doc.entities_with_uids() {
        let Some(was) = before.get(&uid) else {
            continue;
        };
        kept.insert(uid);
        if serde_json::to_string(e).ok().as_deref() == Some(was.as_str()) {
            continue;
        }
        changed += 1;
        if sample.len() < SAMPLE {
            sample.push(json!({
                "feature": places.get(&uid.to_string()),
                "layer": layer_name(doc, e),
                "attrs": e.base().attrs,
                "geometry": kentos_processing::external::geojson(e),
            }));
        }
    }
    if changed > 0 {
        report["changed"] = json!({ "objects": changed, "sample": sample });
    }
    let removed: Vec<Value> = before
        .keys()
        .filter(|u| !kept.contains(u))
        .map(|u| json!(places.get(&u.to_string())))
        .collect();
    if !removed.is_empty() {
        report["removed"] = json!({ "objects": removed.len(), "features": removed });
    }
}

/// A panel as a step left it: its values, its words, its tables (counted, the first rows shown), what its code hid
/// or turned off.
fn state(p: &OpenPanel) -> Value {
    let tables: Map<String, Value> = p
        .tables
        .iter()
        .map(|(id, (columns, rows))| {
            let sample: Vec<&Value> = rows.iter().take(SAMPLE * 2).collect();
            (
                id.clone(),
                json!({ "columns": columns, "rows": rows.len(), "sample": sample }),
            )
        })
        .collect();
    let mut out = json!({
        "panel": p.local,
        "title": p.definition.title,
        "values": p.check_params().ok(),
        "said": p.said.as_ref().map(|(kind, text)| json!({ "kind": kind_code(*kind), "text": text })),
        "tables": tables,
    });
    if !p.hidden.is_empty() {
        out["hidden"] = json!(p.hidden);
    }
    if !p.disabled.is_empty() {
        out["disabled"] = json!(p.disabled);
    }
    out
}

fn kind_code(kind: words::Kind) -> &'static str {
    match kind {
        words::Kind::Info => "info",
        words::Kind::Ok => "success",
        words::Kind::Warn => "warning",
        words::Kind::Error => "error",
    }
}

/// The drawing's layer ids, for a layer picker's value.
fn layer_ids(doc: &Document) -> Vec<String> {
    doc.layers()
        .leaves()
        .into_iter()
        .map(|l| l.id.clone())
        .collect()
}

/// `values` (by field id) set in `panel` as the code's `panel.set` sets them, then checked as Tamam (or a button)
/// checks them: the values its code is given, or why not in words.
fn fill(
    panel: &mut OpenPanel,
    values: &Map<String, Value>,
    layers: &[String],
) -> Result<Value, String> {
    for (id, value) in values {
        panel_session::set(panel, &json!({ "id": id, "value": value }), Some(layers))
            .map_err(|e| e.message)?;
    }
    panel.recalculate();
    panel.check_params().map_err(|(_, why)| why)
}

/// `text` added to what a step printed, up to [`KEPT_OUTPUT`]; the rest is marked cut.
fn keep(output: &mut String, text: &str) {
    let room = KEPT_OUTPUT.saturating_sub(output.len());
    if room == 0 {
        return;
    }
    if text.len() <= room {
        output.push_str(text);
        return;
    }
    let mut cut = room;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    output.push_str(&text[..cut]);
    output.push_str("\n…");
}

/// What a step's code asks, answered on the synthetic drawing.
struct Broker<'a> {
    doc: &'a mut Document,
    rights: &'a Rights,
    panels: &'a mut BTreeMap<String, OpenPanel>,
    /// The panel it started from; none from the ribbon.
    origin: Option<&'a str>,
    title: &'a str,
    srid: u32,
    selection: Selection,
    /// Its code selected objects: the report says which.
    selected: bool,
    dialogs: &'a [Value],
    asked: Vec<Value>,
    refused: Vec<Value>,
    /// The panels it opened or changed.
    touched: BTreeSet<String>,
    group: Option<Group>,
    label: Option<String>,
    poisoned: bool,
}

impl Broker<'_> {
    fn answer(&mut self, method: &str, params: &Value) -> Result<Value, (String, String)> {
        let answer = self.answer_or_not(method, params);
        if let Err(e) = &answer
            && self.refused.len() < MOST_REFUSED
        {
            self.refused
                .push(json!({ "method": method, "code": e.code, "message": e.message }));
        }
        answer.map_err(|e| (e.code.to_owned(), e.message))
    }

    fn answer_or_not(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        match method {
            "begin_group" => {
                if self.group.is_none() && self.label.is_none() {
                    self.label = params
                        .get("label")
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                }
                return Ok(Value::Null);
            }
            "end_group" => return Ok(json!(true)),
            "cancel_group" => {
                self.poisoned = true;
                return Ok(json!(true));
            }
            "dialog" => return self.dialog(params),
            m if m.starts_with("panel.") => return self.panel(m, params),
            _ => {}
        }
        if let Some(owner) = runner::others_tool(&self.rights.ext, params)
            && !self.rights.depends.iter().any(|d| d == owner)
        {
            return Err(HeadlessError::new("capability", runner::undeclared(owner)));
        }
        if !runner::answered(method) {
            return Err(HeadlessError::new(
                "not_for_extensions",
                runner::NOT_FOR_EXTENSIONS,
            ));
        }
        let write = runner::writes(method, params);
        if write && !self.rights.write() {
            return Err(HeadlessError::new("capability", runner::NO_WRITE));
        }
        if !write && !self.rights.read() {
            return Err(HeadlessError::new("capability", runner::NO_READ));
        }
        if matches!(method, "select" | "selection") {
            self.selected |= method == "select";
            return crate::python::selection_answer(self.doc, &mut self.selection, method, params);
        }
        if write && self.group.is_none() {
            self.group = Some(
                self.doc
                    .begin_group(self.label.as_deref().unwrap_or(self.title)),
            );
        }
        kentos_headless::rpc::call_for_agent(self.doc, None, None, method, params)
    }

    /// `ctx.dialog` / `ctx.confirm`: checked as the window checks it, answered with the agent's next answer.
    fn dialog(&mut self, params: &Value) -> Result<Value, HeadlessError> {
        let capabilities = &self.rights.capabilities;
        let (mut panel, _, _) = super::super::dialogs::dialog_panel(
            &self.rights.ext,
            params,
            &|c| capabilities.contains(&c),
            Some(self.srid),
        )?;
        let fields: Vec<&str> = panel
            .definition
            .body
            .iter()
            .filter_map(Component::id)
            .collect();
        let mut said = json!({ "title": panel.definition.title, "fields": fields });
        let picker = panel
            .definition
            .body
            .iter()
            .any(|c| matches!(c, Component::FilePicker { .. }));
        let answer = match self.dialogs.get(self.asked.len()) {
            _ if picker => {
                said["note"] = json!(
                    "Pencerede dosya seçici var: denemede kullanıcının dosyası seçilmez; İptal sayıldı."
                );
                Value::Null
            }
            None => {
                said["note"] = json!("Cevabı verilmedi: İptal sayıldı.");
                Value::Null
            }
            Some(Value::Object(values)) => match fill(&mut panel, values, &layer_ids(self.doc)) {
                Ok(values) => values,
                Err(why) => {
                    said["error"] = json!(why);
                    said["note"] = json!("Cevap pencereye uymadı: İptal sayıldı.");
                    Value::Null
                }
            },
            Some(_) => Value::Null,
        };
        said["answer"] = answer.clone();
        self.asked.push(said);
        Ok(answer)
    }

    /// `ctx.panel` / `ctx.open_panel`: on the trial's panels, by panel_session.rs's checks.
    fn panel(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        let declared: Vec<String> = self.rights.panels.keys().cloned().collect();
        let local = match params.get("panel").and_then(Value::as_str) {
            Some(local) if declared.iter().any(|p| p == local) => local.to_owned(),
            Some(local) => return Err(unknown_panel(local, &declared)),
            None => self.origin.map(str::to_owned).ok_or_else(no_panel)?,
        };
        if method == "panel.open" {
            if !self.panels.contains_key(&local) {
                let definition = self.rights.panels.get(&local).cloned().flatten().ok_or_else(|| {
                    HeadlessError::new(
                        "panel_invalid",
                        format!("“{local}” paneli açılamadı: tanım dosyası okunamadı (eklentinin günlüğünde)."),
                    )
                })?;
                let mut panel = OpenPanel::new(&self.rights.ext, &local, definition);
                panel.default_crs(Some(self.srid));
                self.panels.insert(local.clone(), panel);
            }
            self.touched.insert(local);
            return Ok(json!({ "applied": true }));
        }
        let layers = layer_ids(self.doc);
        let Some(panel) = self.panels.get_mut(&local) else {
            return Ok(json!({ "applied": false, "closed": true }));
        };
        let answer = match method {
            "panel.set" => json!({ "applied": panel_session::set(panel, params, Some(&layers))? }),
            "panel.rows" => {
                panel_session::rows(panel, params)?;
                json!({ "applied": true })
            }
            "panel.say" => {
                panel_session::say(panel, params)?;
                json!({ "applied": true })
            }
            // No window: the cursor goes nowhere.
            "panel.focus" => json!({ "applied": false }),
            "panel.close" => {
                self.panels.remove(&local);
                json!({ "applied": true })
            }
            other => {
                return Err(HeadlessError::new(
                    "unknown_method",
                    format!(
                        "“{other}” bir panel isteği değil: panel.open, panel.set, panel.rows, panel.say, panel.focus \
                         ya da panel.close."
                    ),
                ));
            }
        };
        self.touched.insert(local);
        Ok(answer)
    }
}

#[cfg(test)]
#[path = "broker_tests.rs"]
mod tests;
