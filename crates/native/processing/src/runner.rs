//! Runs processing tools (the web's `processing/runner.ts`): validate →
//! resolve what depends on the host (objects to ids, layers to a target) →
//! run the tool over the document as it is → apply its change set as one
//! undo step → record history.

use std::collections::{BTreeMap, HashSet};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use kentos_contracts::Entity;
use kentos_domain::{Document, NewLayer, Slot, default_style};
use serde_json::{Value, json};

use crate::expression::{measures_of, preview_expression};
use crate::features::{Host, InputSummary, Scene, resolve_features, summarize_features};
use crate::geometry::RunGeometry;
use crate::parameters::{Issue, is_visible, validate_values};
use crate::text::{fold_turkish, js_trim};
use crate::types::{
    Defaults, FeatureSet, Feedback, NewLayerStyle, ParamKind, Resolved, RunContext, RunResult,
    Target, TargetLayer, Tool, Values,
};
use crate::values::{FeaturesValue, LayerValue, Scope};

/// Runs kept in history, newest first.
pub const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
}

/// A message from a tool (`feedback.info`, `warn`) or about skipped changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    pub level: Level,
    pub text: String,
}

impl LogLine {
    pub fn warn(text: String) -> Self {
        Self {
            level: Level::Warn,
            text,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Error,
    Canceled,
}

/// A run in history (the web's `RunRecord`).
#[derive(Clone, Debug, PartialEq)]
pub struct RunRecord {
    pub seq: u64,
    pub tool_id: String,
    pub label: String,
    /// The values as entered, for "run again".
    pub values: Values,
    /// When it started, ms since the Unix epoch.
    pub started: u64,
    pub ms: u64,
    pub status: Status,
    pub summary: String,
    /// Objects the run created.
    pub added: Vec<Slot>,
    /// Objects it changed or selected ("Sonuçları seç" when nothing was added).
    pub touched: Vec<Slot>,
    /// Where it ran; none when it did not start.
    pub target: Option<Target>,
}

/// How a run ended.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// It ran; `edited`: the drawing changed (there is something to undo).
    Ok {
        result: RunResult,
        added: Vec<Slot>,
        touched: Vec<Slot>,
        edited: bool,
        record: RunRecord,
    },
    /// It did not start: the values have problems.
    Invalid { issues: Vec<Issue> },
    /// It started and failed, or was stopped (`status`), and the drawing is as it was.
    Stopped {
        status: Status,
        message: String,
        record: RunRecord,
    },
}

/// Where a scope looked, as the refusal of an input whose objects are all on locked layers says it.
fn locked_where(scope: &Scope) -> &'static str {
    match scope {
        Scope::Selection => "seçili nesnelerin",
        Scope::Visible => "görünen alandaki nesnelerin",
        Scope::All => "görünen katmanlardaki nesnelerin",
        Scope::Layer(_) | Scope::Ids(_) => "bu katmandaki nesnelerin",
    }
}

fn empty_input_message(label: &str, scope: &Scope) -> String {
    match scope {
        Scope::Selection => format!(
            "“{label}”: seçili nesneler arasında uygun nesne yok. Önce nesneleri seçin ya da kapsamı değiştirin."
        ),
        Scope::Visible => format!(
            "“{label}”: görünen alanda uygun nesne yok. Görünümü kaydırın ya da kapsamı değiştirin."
        ),
        Scope::Layer(_) => {
            format!("“{label}”: bu katmanda uygun nesne yok. Başka bir katman seçin.")
        }
        Scope::All | Scope::Ids(_) => format!("“{label}”: uygun nesne yok."),
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Ids once each, in the order first given.
fn distinct(ids: impl IntoIterator<Item = Slot>) -> Vec<Slot> {
    let mut seen = HashSet::new();
    ids.into_iter().filter(|id| seen.insert(*id)).collect()
}

/// Messages and progress of one run.
struct Collect<'a> {
    log: &'a mut Vec<LogLine>,
    canceled: bool,
}

impl Feedback for Collect<'_> {
    fn progress(&mut self, _fraction: f64, _label: &str) {}

    fn info(&mut self, message: String) {
        self.log.push(LogLine {
            level: Level::Info,
            text: message,
        });
    }

    fn warn(&mut self, message: String) {
        self.log.push(LogLine::warn(message));
    }

    fn canceled(&self) -> bool {
        self.canceled
    }
}

/// A new-layer value reuses a layer that already has that name (running a
/// tool twice keeps writing to the same "Köşe noktaları"); otherwise it
/// gets an id now and is created on apply.
pub fn resolve_layer(doc: &Document, v: &LayerValue) -> TargetLayer {
    let layers = doc.layers();
    match v {
        LayerValue::Existing(id) => TargetLayer {
            id: id.clone(),
            name: layers
                .get(id)
                .map_or_else(|| id.clone(), |l| l.name.clone()),
            is_new: false,
        },
        LayerValue::New(name) => {
            let name = js_trim(name).to_owned();
            let folded = fold_turkish(&name);
            if let Some(same) = layers
                .leaves()
                .into_iter()
                .find(|l| fold_turkish(&l.name) == folded)
            {
                return TargetLayer {
                    id: same.id.clone(),
                    name: same.name.clone(),
                    is_new: false,
                };
            }
            let mut slug = String::new();
            let mut gap = false;
            for c in folded.to_lowercase().chars() {
                if c.is_ascii_lowercase() || c.is_ascii_digit() {
                    slug.push(c);
                    gap = false;
                } else if !gap {
                    slug.push('-');
                    gap = true;
                }
            }
            let mut id = format!("islem-{slug}");
            // As the web does: each try extends the last ("-2", then "-2-3").
            let mut k = 2;
            while layers.get(&id).is_some() {
                id = format!("{id}-{k}");
                k += 1;
            }
            TargetLayer {
                id,
                name,
                is_new: true,
            }
        }
    }
}

/// The processing runner: history, and the run itself.
#[derive(Debug, Default)]
pub struct Runner {
    history: Vec<RunRecord>,
    seq: u64,
    canceled: bool,
}

impl Runner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs so far, newest first.
    pub fn history(&self) -> &[RunRecord] {
        &self.history
    }

    pub fn validate(&self, tool: &Tool, values: &Values, doc: &Document) -> Vec<Issue> {
        validate_values(tool, values, doc.layers())
    }

    /// What each features parameter resolves to now ("12 kapalı alan; seçili nesneler").
    pub fn describe_inputs(
        &self,
        tool: &Tool,
        values: &Values,
        scene: &dyn Scene,
    ) -> BTreeMap<String, InputSummary> {
        let mut out = BTreeMap::new();
        for p in &tool.parameters {
            if let ParamKind::Features { kinds, .. } = &p.kind
                && let Some(fv) = values.get(&p.name).and_then(FeaturesValue::read)
            {
                out.insert(
                    p.name.clone(),
                    summarize_features(&fv, kinds.as_deref(), scene),
                );
            }
        }
        out
    }

    /// How an expression parameter works out on the objects it reads, for
    /// the dialog; none when it is empty or does not compile (the field says why).
    pub fn preview_expression(
        &self,
        tool: &Tool,
        values: &Values,
        name: &str,
        scene: &dyn Scene,
    ) -> Option<String> {
        let def = tool.parameters.iter().find(|p| p.name == name)?;
        let ParamKind::Expression { returns, of, .. } = &def.kind else {
            return None;
        };
        let src = js_trim(values.get(name)?.as_str()?);
        let expr = kentos_style_core::expr::compile(src).ok()?;
        let input = tool
            .parameters
            .iter()
            .find(|p| Some(&p.name) == of.as_ref())?;
        let ParamKind::Features { kinds, .. } = &input.kind else {
            return None;
        };
        let fv = values.get(&input.name).and_then(FeaturesValue::read)?;
        let set = resolve_features(&fv, kinds.as_deref(), scene);
        let doc = scene.doc();
        let layer_name = |id: &str| {
            doc.layers()
                .get(id)
                .map_or_else(|| id.to_owned(), |l| l.name.clone())
        };
        let mut measures = |list: &[&Entity]| match scene.store() {
            Some(store) => store.measures(
                &list
                    .iter()
                    .map(|e| f64::from(e.base().id))
                    .collect::<Vec<_>>(),
            ),
            None => measures_of(list),
        };
        (!src.is_empty())
            .then(|| preview_expression(&expr, &set.entities, *returns, &layer_name, &mut measures))
    }

    /// Objects the features inputs resolve to now.
    pub fn input_size(&self, tool: &Tool, values: &Values, scene: &dyn Scene) -> usize {
        self.describe_inputs(tool, values, scene)
            .values()
            .map(|s| s.count)
            .sum()
    }

    /// Asks the running tool to stop (models stop between steps).
    pub fn cancel(&mut self) {
        self.canceled = true;
    }

    pub fn canceling(&self) -> bool {
        self.canceled
    }

    /// Adds a history record (runs record themselves; a model adds one for all its steps).
    pub fn add_record(&mut self, mut r: RunRecord) -> RunRecord {
        self.seq += 1;
        r.seq = self.seq;
        self.history.insert(0, r.clone());
        self.history.truncate(HISTORY_LIMIT);
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        &mut self,
        silent: bool,
        tool: &Tool,
        values: &Values,
        started: (u64, Instant),
        status: Status,
        summary: String,
        added: Vec<Slot>,
        touched: Vec<Slot>,
        target: Option<Target>,
    ) -> RunRecord {
        let r = RunRecord {
            seq: 0,
            tool_id: tool.id.clone(),
            label: tool.label.clone(),
            values: values.clone(),
            started: started.0,
            ms: started.1.elapsed().as_millis() as u64,
            status,
            summary,
            added,
            touched,
            target,
        };
        if silent { r } else { self.add_record(r) }
    }

    /// Runs `tool` with `values` on the host's drawing. `silent`: no history
    /// record (a model records itself once, not each step). Messages go to `log`.
    pub fn run(
        &mut self,
        host: &mut dyn Host,
        tool: &Tool,
        values: &Values,
        silent: bool,
        log: &mut Vec<LogLine>,
    ) -> Outcome {
        let issues = self.validate(tool, values, host.doc());
        if !issues.is_empty() {
            return Outcome::Invalid { issues };
        }
        let started = (now_ms(), Instant::now());
        // Resolve what depends on the host: objects to ids, layers to a
        // target (new layers are made only on apply).
        let mut inputs: BTreeMap<String, (Vec<Slot>, String)> = BTreeMap::new();
        let mut layers: BTreeMap<String, TargetLayer> = BTreeMap::new();
        let mut new_layers: BTreeMap<String, (String, NewLayerStyle)> = BTreeMap::new();
        // What resolving the inputs left out, said once the run starts.
        let mut notes: Vec<String> = Vec::new();
        {
            let scene: &dyn Scene = host;
            let doc = scene.doc();
            for p in &tool.parameters {
                let v = values.get(&p.name).unwrap_or(&Value::Null);
                if !is_visible(p, values) || v.is_null() {
                    continue;
                }
                match &p.kind {
                    ParamKind::Features { kinds, writes, .. } => {
                        let Some(fv) = FeaturesValue::read(v) else {
                            continue;
                        };
                        let set = resolve_features(&fv, kinds.as_deref(), scene);
                        let mut entities = set.entities;
                        let is_locked = |e: &Entity| doc.layers().is_locked(&e.base().layer_id);
                        let is_ids = matches!(fv.scope, Scope::Ids(_));
                        // A tool that changes its input does not get objects it may not change.
                        let locked = if *writes {
                            entities.iter().filter(|e| is_locked(e)).count()
                        } else {
                            0
                        };
                        if locked > 0 {
                            entities.retain(|e| !is_locked(e));
                            if entities.is_empty() && !p.optional && !is_ids {
                                return Outcome::Invalid {
                                    issues: vec![Issue {
                                        param: Some(p.name.clone()),
                                        message: format!(
                                            "“{}”: {} hepsi kilitli katmanda. Kilidi Katmanlar panelinden açın.",
                                            p.label,
                                            locked_where(&fv.scope)
                                        ),
                                    }],
                                };
                            }
                            notes.push(format!(
                                "“{}”: {locked} nesne kilitli katmanda olduğu için işleme alınmadı.",
                                p.label
                            ));
                        }
                        // Running on nothing is a mistake worth stopping (usually:
                        // nothing selected); an empty output passed along a model is not.
                        if entities.is_empty() && !p.optional && !is_ids {
                            return Outcome::Invalid {
                                issues: vec![Issue {
                                    param: Some(p.name.clone()),
                                    message: empty_input_message(&p.label, &fv.scope),
                                }],
                            };
                        }
                        inputs.insert(
                            p.name.clone(),
                            (
                                entities.iter().map(|e| Slot(e.base().id)).collect(),
                                set.description,
                            ),
                        );
                    }
                    ParamKind::Layer { new_layer_style } => {
                        let Some(lv) = LayerValue::read(v) else {
                            continue;
                        };
                        let t = resolve_layer(doc, &lv);
                        if t.is_new {
                            new_layers
                                .insert(t.id.clone(), (t.name.clone(), new_layer_style.clone()));
                        }
                        layers.insert(p.name.clone(), t);
                    }
                    _ => {}
                }
            }
        }
        let Some(run) = tool.run.filter(|_| tool.targets.contains(&Target::Client)) else {
            let wanted: Vec<&str> = tool.targets.iter().map(|t| t.id()).collect();
            let message = format!(
                "“{}” bu ortamda çalıştırılamıyor ({} gerekli).",
                tool.label,
                wanted.join(", ")
            );
            let record = self.record(
                silent,
                tool,
                values,
                started,
                Status::Error,
                message.clone(),
                Vec::new(),
                Vec::new(),
                None,
            );
            return Outcome::Stopped {
                status: Status::Error,
                message,
                record,
            };
        };
        let target = Some(Target::Client);
        self.canceled = false;
        for note in notes {
            log.push(LogLine::warn(note));
        }
        let (result, canceled) = {
            let doc = host.doc();
            let units = Defaults::of(doc);
            let selection = host.selected();
            let mut resolved = Resolved::new(values.clone());
            let mut objects: Vec<&Entity> = Vec::new();
            let mut seen = HashSet::new();
            for p in &tool.parameters {
                match &p.kind {
                    ParamKind::Features { .. } => {
                        if let Some((ids, description)) = inputs.get(&p.name) {
                            let entities: Vec<&Entity> =
                                ids.iter().filter_map(|id| doc.get(*id)).collect();
                            objects.extend(entities.iter().filter(|e| seen.insert(e.base().id)));
                            resolved.features.insert(
                                p.name.clone(),
                                FeatureSet {
                                    entities,
                                    description: description.clone(),
                                },
                            );
                        }
                    }
                    ParamKind::Expression { .. } => {
                        let src =
                            js_trim(values.get(&p.name).and_then(Value::as_str).unwrap_or(""));
                        if let Ok(expr) = kentos_style_core::expr::compile(src)
                            && !src.is_empty()
                        {
                            resolved.exprs.insert(p.name.clone(), expr);
                        }
                    }
                    ParamKind::Field { .. } => {
                        if let Some(s) = values.get(&p.name).and_then(Value::as_str) {
                            resolved.values.insert(p.name.clone(), json!(js_trim(s)));
                        }
                    }
                    _ => {}
                }
            }
            resolved.layers = layers;
            let geometry = RunGeometry::of(objects);
            let ctx = RunContext {
                doc,
                units: &units,
                selection: &selection,
                geometry: &geometry,
                layer_names: doc
                    .layers()
                    .leaves()
                    .into_iter()
                    .map(|l| (l.id.clone(), l.name.clone()))
                    .collect(),
            };
            let mut feedback = Collect {
                log: &mut *log,
                canceled: self.canceled,
            };
            let result = run(&resolved, &ctx, &mut feedback);
            (result, feedback.canceled || self.canceled)
        };
        if canceled {
            let message = "İşlem iptal edildi; çizim değişmedi.".to_owned();
            let record = self.record(
                silent,
                tool,
                values,
                started,
                Status::Canceled,
                message.clone(),
                Vec::new(),
                Vec::new(),
                target,
            );
            return Outcome::Stopped {
                status: Status::Canceled,
                message,
                record,
            };
        }
        let added = match apply(host.doc_mut(), tool, &result, &new_layers, log) {
            Ok(added) => added,
            Err(why) => {
                let message = format!("“{}” çalışırken hata: {why}", tool.label);
                let record = self.record(
                    silent,
                    tool,
                    values,
                    started,
                    Status::Error,
                    message.clone(),
                    Vec::new(),
                    Vec::new(),
                    target,
                );
                return Outcome::Stopped {
                    status: Status::Error,
                    message,
                    record,
                };
            }
        };
        let selected = result.select.as_ref().map(|ids| {
            let doc = host.doc();
            distinct(ids.iter().copied())
                .into_iter()
                .filter(|id| doc.get(*id).is_some())
                .collect::<Vec<_>>()
        });
        if let Some(ids) = &selected {
            host.select(ids);
        }
        let changes = result.changes.clone().unwrap_or_default();
        let touched: Vec<Slot> = {
            let doc = host.doc();
            distinct(
                changes
                    .update
                    .iter()
                    .map(|u| u.id)
                    .chain(selected.iter().flatten().copied()),
            )
            .into_iter()
            .filter(|id| doc.get(*id).is_some())
            .collect()
        };
        let summary = result.summary.clone().unwrap_or_else(|| match &selected {
            Some(ids) => format!("{} nesne seçildi.", ids.len()),
            None => format!("{} nesne eklendi.", added.len()),
        });
        let edited = !added.is_empty() || !changes.update.is_empty() || !changes.remove.is_empty();
        let record = self.record(
            silent,
            tool,
            values,
            started,
            Status::Ok,
            summary,
            added.clone(),
            touched.clone(),
            target,
        );
        Outcome::Ok {
            result,
            added,
            touched,
            edited,
            record,
        }
    }
}

/// Applies the change set in one undo step named after the tool; locked
/// layers are left alone and counted. Returns the objects added.
fn apply(
    doc: &mut Document,
    tool: &Tool,
    result: &RunResult,
    new_layers: &BTreeMap<String, (String, NewLayerStyle)>,
    log: &mut Vec<LogLine>,
) -> Result<Vec<Slot>, String> {
    let Some(ch) = &result.changes else {
        return Ok(Vec::new());
    };
    let mut skipped = 0usize;
    // Removals, then updates, then additions, each as one change.
    let added = doc.transact(&tool.label, |doc| {
        // Layers the run writes to but that do not exist yet: in the tool's step, so undo takes them too.
        for e in &ch.add {
            let id = &e.base().layer_id;
            let Some((name, style)) = new_layers.get(id) else {
                continue;
            };
            if doc.layers().get(id).is_some() {
                continue;
            }
            let mut layer = NewLayer::layer(name.clone());
            layer.id = Some(id.clone());
            layer.style = style.over(default_style());
            doc.add_layer(layer, None, false).map_err(|r| r.0)?;
        }
        let locked = |doc: &Document, layer: &str| doc.layers().is_locked(layer);
        let mut gone = Vec::new();
        for id in &ch.remove {
            let Some(e) = doc.get(*id) else {
                continue;
            };
            if locked(doc, &e.base().layer_id) {
                skipped += 1;
            } else {
                gone.push(*id);
            }
        }
        doc.remove(&gone);
        let mut patches = Vec::new();
        for u in &ch.update {
            let Some(e) = doc.get(u.id) else {
                continue;
            };
            if locked(doc, &e.base().layer_id) {
                skipped += 1;
                continue;
            }
            let mut next = e.clone();
            if let Some(attrs) = &u.attrs {
                next.base_mut().attrs.clone_from(attrs);
            }
            if let Some(label) = &u.label {
                next.base_mut().label.clone_from(label);
            }
            patches.push((u.id, next));
        }
        doc.update_many(patches, &tool.label);
        let mut fresh = Vec::new();
        for n in &ch.add {
            let layer = &n.base().layer_id;
            if doc.layers().get(layer).is_none() || locked(doc, layer) {
                skipped += 1;
                continue;
            }
            fresh.push(n.clone());
        }
        doc.add_many(fresh, &tool.label).map_err(|e| e.to_string())
    })?;
    if skipped > 0 {
        log.push(LogLine::warn(format!(
            "{skipped} değişiklik kilitli ya da olmayan katmanda olduğu için atlandı."
        )));
    }
    Ok(added)
}
