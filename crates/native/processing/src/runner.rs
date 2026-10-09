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
use crate::features::{
    Host, InputSummary, Scene, resolve_features, summarize_features, summarize_file,
};
use crate::geometry::RunGeometry;
use crate::parameters::{Issue, file_table, is_visible, validate_with};
use crate::text::{fold_turkish, js_trim};
use crate::types::{
    ChangeSet, Defaults, FeatureSet, Feedback, NewLayerStyle, ParamKind, Resolved, RunContext,
    RunFn, RunResult, Target, TargetLayer, Tool, Values,
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

/// How a run ended. A run ends once and `Ok` is its usual end: boxing the
/// result would only add an allocation.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
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

/// Input size (objects) from which a job goes to the background (the web's `WORKER_THRESHOLD`).
pub const WORKER_THRESHOLD: usize = 2000;

/// A run resolved against the host and ready to compute (the web's
/// `RunJob`): the tool, its values, each features input as the ids it
/// resolved to, the layers it writes to and the selection, all owned, so
/// the computation can go to another thread ([`Runner::compute`]).
#[derive(Clone)]
pub struct Job {
    pub tool: Tool,
    /// Where it runs: here (as prepared), or in the background; the history records it.
    pub target: Target,
    values: Values,
    silent: bool,
    started: (u64, Instant),
    inputs: BTreeMap<String, (Vec<Slot>, String)>,
    layers: BTreeMap<String, TargetLayer>,
    /// Layers the run writes to that do not exist yet: their names, looks and
    /// the layer each goes right above (none: last).
    new_layers: BTreeMap<String, NewLayerPlan>,
    selection: Vec<Slot>,
    run: RunFn,
}

/// A layer the run creates on apply: its name, its look and the layer it goes right above or below.
#[derive(Clone, Debug)]
struct NewLayerPlan {
    name: String,
    style: NewLayerStyle,
    above: Option<String>,
    below: Option<String>,
}

impl Job {
    /// Objects the features inputs resolved to: the web's input size, from
    /// which [`WORKER_THRESHOLD`] sends a job to the background.
    pub fn size(&self) -> usize {
        self.inputs.values().map(|(ids, _)| ids.len()).sum()
    }
}

/// What preparing a run gave.
pub enum Prepared {
    /// Ready to compute.
    Ready(Job),
    /// It cannot start: invalid values, nothing to run on, nowhere to run here.
    Done(Outcome),
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
        validate_with(tool, values, doc.layers(), &doc.settings().networks)
    }

    /// What each features parameter resolves to now ("12 kapalı alan; seçili
    /// nesneler"), and each chosen file's table (its columns as the fields a
    /// field parameter offers, docs/adr/0200 §6).
    pub fn describe_inputs(
        &self,
        tool: &Tool,
        values: &Values,
        scene: &dyn Scene,
    ) -> BTreeMap<String, InputSummary> {
        let mut out = BTreeMap::new();
        for p in &tool.parameters {
            match &p.kind {
                ParamKind::Features { kinds, .. } => {
                    if let Some(fv) = values.get(&p.name).and_then(FeaturesValue::read) {
                        out.insert(
                            p.name.clone(),
                            summarize_features(&fv, kinds.as_deref(), scene),
                        );
                    }
                }
                ParamKind::File { .. } => {
                    if let Some((header, rows)) = values.get(&p.name).and_then(file_table) {
                        out.insert(p.name.clone(), summarize_file(&header, &rows));
                    }
                }
                _ => {}
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
        // A raster's expression runs cell by cell in the raster core (docs/adr/0233 §3): no object to preview it on.
        if only_rasters(&set.entities) {
            return None;
        }
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
    /// record (a model records itself once, not each step). Messages go to
    /// `log`. The three steps at once: [`Runner::prepare`], [`Runner::compute`]
    /// on the host's drawing, [`Runner::finish`].
    pub fn run(
        &mut self,
        host: &mut dyn Host,
        tool: &Tool,
        values: &Values,
        silent: bool,
        log: &mut Vec<LogLine>,
    ) -> Outcome {
        match self.prepare(host, tool, values, silent, log) {
            Prepared::Ready(job) => self.complete(host, job, log),
            Prepared::Done(outcome) => outcome,
        }
    }

    /// Computes a prepared job on the host's drawing and finishes it, here
    /// and now: [`Runner::compute`], then [`Runner::finish`].
    pub fn complete(&mut self, host: &mut dyn Host, job: Job, log: &mut Vec<LogLine>) -> Outcome {
        let (result, canceled) = {
            let mut feedback = Collect {
                log: &mut *log,
                canceled: self.canceled,
            };
            let result = Self::compute(&job, host.doc(), &mut feedback);
            (result, feedback.canceled || self.canceled)
        };
        self.finish(host, job, result, canceled, log)
    }

    /// Validates the values and resolves what depends on the host (the
    /// objects of each features input, the layers written to, the
    /// selection) into a job; or how the run ended when it cannot start.
    /// Notes on what resolving left out go to `log`.
    pub fn prepare(
        &mut self,
        host: &dyn Scene,
        tool: &Tool,
        values: &Values,
        silent: bool,
        log: &mut Vec<LogLine>,
    ) -> Prepared {
        let issues = self.validate(tool, values, host.doc());
        if !issues.is_empty() {
            return Prepared::Done(Outcome::Invalid { issues });
        }
        let started = (now_ms(), Instant::now());
        // Resolve what depends on the host: objects to ids, layers to a
        // target (new layers are made only on apply).
        let mut inputs: BTreeMap<String, (Vec<Slot>, String)> = BTreeMap::new();
        let mut layers: BTreeMap<String, TargetLayer> = BTreeMap::new();
        let mut new_layers: BTreeMap<String, NewLayerPlan> = BTreeMap::new();
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
                                return Prepared::Done(Outcome::Invalid {
                                    issues: vec![Issue {
                                        param: Some(p.name.clone()),
                                        message: format!(
                                            "“{}”: {} hepsi kilitli katmanda. Kilidi Katmanlar panelinden açın.",
                                            p.label,
                                            locked_where(&fv.scope)
                                        ),
                                    }],
                                });
                            }
                            notes.push(format!(
                                "“{}”: {locked} nesne kilitli katmanda olduğu için işleme alınmadı.",
                                p.label
                            ));
                        }
                        // Running on nothing is a mistake worth stopping (usually:
                        // nothing selected); an empty output passed along a model is not.
                        if entities.is_empty() && !p.optional && !is_ids {
                            return Prepared::Done(Outcome::Invalid {
                                issues: vec![Issue {
                                    param: Some(p.name.clone()),
                                    message: empty_input_message(&p.label, &fv.scope),
                                }],
                            });
                        }
                        inputs.insert(
                            p.name.clone(),
                            (
                                entities.iter().map(|e| Slot(e.base().id)).collect(),
                                set.description,
                            ),
                        );
                    }
                    ParamKind::Layer {
                        new_layer_style,
                        above,
                        below,
                    } => {
                        let Some(lv) = LayerValue::read(v) else {
                            continue;
                        };
                        let t = resolve_layer(doc, &lv);
                        if t.is_new {
                            new_layers.insert(
                                t.id.clone(),
                                NewLayerPlan {
                                    name: t.name.clone(),
                                    style: new_layer_style.clone(),
                                    // The features parameter's name for now; its first object's layer below.
                                    above: above.clone(),
                                    below: below.clone(),
                                },
                            );
                        }
                        layers.insert(p.name.clone(), t);
                    }
                    _ => {}
                }
            }
        }
        // This program runs a tool here or on its own thread (the web's worker);
        // the server and PostGIS are elsewhere.
        let Some(run) = tool.run.filter(|_| {
            tool.targets
                .iter()
                .any(|t| matches!(t, Target::Client | Target::Worker))
        }) else {
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
            return Prepared::Done(Outcome::Stopped {
                status: Status::Error,
                message,
                record,
            });
        };
        // A new layer named to go above or below a features input: the layer of that input's first object.
        let layer_of = |param: String| {
            let first = *inputs.get(&param)?.0.first()?;
            Some(host.doc().get(first)?.base().layer_id.clone())
        };
        for plan in new_layers.values_mut() {
            plan.above = plan.above.take().and_then(layer_of);
            plan.below = plan.below.take().and_then(layer_of);
        }
        self.canceled = false;
        for note in notes {
            log.push(LogLine::warn(note));
        }
        Prepared::Ready(Job {
            tool: tool.clone(),
            target: Target::Client,
            values: values.clone(),
            silent,
            started,
            inputs,
            layers,
            new_layers,
            selection: host.selected(),
            run,
        })
    }

    /// The tool's work (the web's executors run this part): what it would
    /// change, computed on `doc`, the drawing the job was prepared on or a
    /// copy of it, on any thread. It reads and never writes; the runner
    /// applies the result on the host ([`Runner::finish`]).
    pub fn compute(job: &Job, doc: &Document, feedback: &mut dyn Feedback) -> RunResult {
        let units = Defaults::of(doc);
        let mut resolved = Resolved::new(job.values.clone());
        let mut objects: Vec<&Entity> = Vec::new();
        let mut seen = HashSet::new();
        for p in &job.tool.parameters {
            match &p.kind {
                ParamKind::Features { .. } => {
                    if let Some((ids, description)) = job.inputs.get(&p.name) {
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
                    let src = js_trim(
                        job.values
                            .get(&p.name)
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                    );
                    if let Ok(expr) = kentos_style_core::expr::compile(src)
                        && !src.is_empty()
                    {
                        resolved.exprs.insert(p.name.clone(), expr);
                    }
                }
                ParamKind::Field { .. } => {
                    if let Some(s) = job.values.get(&p.name).and_then(Value::as_str) {
                        resolved.values.insert(p.name.clone(), json!(js_trim(s)));
                    }
                }
                _ => {}
            }
        }
        resolved.layers = job.layers.clone();
        let geometry = RunGeometry::of(objects);
        let ctx = RunContext {
            doc,
            units: &units,
            selection: &job.selection,
            geometry: &geometry,
            layer_names: doc
                .layers()
                .leaves()
                .into_iter()
                .map(|l| (l.id.clone(), l.name.clone()))
                .collect(),
        };
        (job.run)(&resolved, &ctx, feedback)
    }

    /// Ends a job that was stopped (Durdur), or whose drawing is gone:
    /// nothing changes; the run is recorded as canceled.
    pub fn stop(&mut self, job: Job) -> Outcome {
        let message = "İşlem iptal edildi; çizim değişmedi.".to_owned();
        self.end(job, Status::Canceled, message)
    }

    /// Ends a job whose computation broke off (`why`): nothing changes, the
    /// run is recorded as an error in the web's words.
    pub fn failed(&mut self, job: Job, why: &str) -> Outcome {
        let message = format!("“{}” çalışırken hata: {why}", job.tool.label);
        self.end(job, Status::Error, message)
    }

    /// A job ended without changing anything.
    fn end(&mut self, job: Job, status: Status, message: String) -> Outcome {
        let record = self.record(
            job.silent,
            &job.tool,
            &job.values,
            job.started,
            status,
            message.clone(),
            Vec::new(),
            Vec::new(),
            Some(job.target),
        );
        Outcome::Stopped {
            status,
            message,
            record,
        }
    }

    /// Ends a job with what [`Runner::compute`] gave: a stopped run changes
    /// nothing; otherwise the change set is applied to the host's drawing in
    /// one undo step (objects gone or on locked layers are left out and
    /// counted, as on the web), the selection set, the run recorded.
    pub fn finish(
        &mut self,
        host: &mut dyn Host,
        job: Job,
        mut result: RunResult,
        canceled: bool,
        log: &mut Vec<LogLine>,
    ) -> Outcome {
        if canceled {
            return self.stop(job);
        }
        let Job {
            tool,
            target,
            values,
            silent,
            started,
            new_layers,
            ..
        } = job;
        let (tool, values) = (&tool, &values);
        let target = Some(target);
        // The change set is spent here: its new objects move into the
        // drawing, not copied (a run may add hundreds of thousands).
        let mut changes = result.changes.take().unwrap_or_default();
        // A tool that refuses, or a value a layer's field does not take, ends the run with nothing changed.
        let refused = result
            .refused
            .clone()
            .or_else(|| crate::writes::check(host.doc(), &mut changes).err());
        if let Some(message) = refused {
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
        let updated: Vec<Slot> = changes.update.iter().map(|u| u.id).collect();
        let removed = !changes.remove.is_empty();
        let added = match apply(
            host.doc_mut(),
            tool,
            changes,
            (&new_layers, result.above.as_deref()),
            log,
        ) {
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
        let touched: Vec<Slot> = {
            let doc = host.doc();
            distinct(
                updated
                    .iter()
                    .copied()
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
        let edited = !added.is_empty() || !updated.is_empty() || removed;
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
    ch: ChangeSet,
    (new_layers, above): (&BTreeMap<String, NewLayerPlan>, Option<&str>),
    log: &mut Vec<LogLine>,
) -> Result<Vec<Slot>, String> {
    let mut skipped = 0usize;
    // Removals, then updates, then additions, each as one change.
    let added = doc.transact(&tool.label, |doc| {
        // Layers the run writes to but that do not exist yet: in the tool's step, so undo takes them too.
        // Each run of one layer is asked once: a run adds hundreds of thousands of objects to a few layers.
        let mut asked: Option<&str> = None;
        // New layers below one layer keep the run's order: each below the one before (Kriging's
        // prediction right under its points, its error under that).
        let mut under_count: BTreeMap<&str, usize> = BTreeMap::new();
        for e in &ch.add {
            let id = &e.base().layer_id;
            if asked == Some(id.as_str()) {
                continue;
            }
            asked = Some(id.as_str());
            let Some(plan) = new_layers.get(id) else {
                continue;
            };
            if doc.layers().get(id).is_some() {
                continue;
            }
            let mut layer = NewLayer::layer(plan.name.clone());
            layer.id = Some(id.clone());
            layer.style = plan.style.over(default_style());
            // Right above or below the layer its plan names (its group, its place), else last;
            // the tool may name it.
            let at = match (&plan.above, &plan.below) {
                (Some(over), _) => doc.layers().place_of(above.unwrap_or(over)),
                (None, Some(under)) => {
                    let n = under_count.entry(under.as_str()).or_insert(0);
                    *n += 1;
                    doc.layers()
                        .place_of(under)
                        .map(|(group, index)| (group, index + *n))
                }
                (None, None) => None,
            };
            match at {
                Some((group, index)) => doc
                    .add_layer_at(layer, group.as_deref(), Some(index), None, false)
                    .map_err(|r| r.0)?,
                None => doc.add_layer(layer, None, false).map_err(|r| r.0)?,
            };
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
            if let Some(zs) = &u.zs {
                kentos_native_application::elevation::assign(&mut next, zs);
            }
            patches.push((u.id, next));
        }
        doc.update_many(patches, &tool.label);
        // Filtered in place: nothing moves unless something is left out (an object is large).
        let mut fresh = ch.add;
        let mut verdict: Option<(String, bool)> = None;
        fresh.retain(|n| {
            let layer = &n.base().layer_id;
            let takes = match &verdict {
                Some((id, takes)) if id == layer => *takes,
                _ => {
                    let takes = doc.layers().get(layer).is_some() && !locked(doc, layer);
                    verdict = Some((layer.clone(), takes));
                    takes
                }
            };
            if !takes {
                skipped += 1;
            }
            takes
        });
        doc.add_many(fresh, &tool.label).map_err(|e| e.to_string())
    })?;
    if skipped > 0 {
        log.push(LogLine::warn(format!(
            "{skipped} değişiklik kilitli ya da olmayan katmanda olduğu için atlandı."
        )));
    }
    Ok(added)
}

/// Rasters only: an expression on them names their bands (docs/adr/0233 §3), not attributes.
pub fn only_rasters(list: &[&Entity]) -> bool {
    !list.is_empty() && list.iter().all(|e| matches!(e, Entity::Raster(_)))
}
