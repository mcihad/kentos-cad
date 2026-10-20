//! İşlemler's tools for the processing commands (docs/adr/0508, plan/gis.md CBS-41): the application's
//! `cad.processing.tools` and `.run` reach the tools through [`register`]. A run is the window's run without a
//! screen: the values are the input's (a features input names its objects by persistent ids, or takes the input's
//! explicit `selection` and `view`), the drawing's defaults fill the rest, the runner checks and writes them as the
//! window does, one undo step named by the tool. With a sink (docs/adr/0508 §4) the run is made on a copy and the
//! objects it made go to a new GeoPackage; the drawing does not change.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use std::sync::OnceLock;
use std::time::Instant;

use kentos_commands::{
    BatchRowRan, CommandError, CommandWarning, GpkgExported, GpkgTableExported, ProcessingBatch,
    ProcessingBatched, ProcessingRan, ProcessingRecord, ProcessingRun, ProcessingTool,
    ProcessingTools, ProcessingToolsListed, RecordStep, RecordTotal, RunInput, RunSink, ToolNote,
    ToolParameter, codes,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::geometry::Bounds;
use kentos_native_application::processing_host::{BatchEnd, BatchFlow, ToolHost, register_tools};
use serde_json::{Value, json};

use crate::features::{Host, Scene};
use crate::model::Model;
use crate::model_runner::{MODEL_PREFIX, model_as_tool, run_model};
use crate::parameters::{default_values, stored_values};
use crate::registry::Registry;
use crate::runner::{Level, Outcome, RunRecord, Runner};
use crate::text::fold_turkish;
use crate::types::{Defaults, OutputKind, ParamKind, Tool, Values};

/// What a run's record says about where it ran (docs/adr/0529 §2).
static HOST: OnceLock<&'static str> = OnceLock::new();

/// Registers İşlemler's built-in tools as the processing commands' (the first registration stays); the record says
/// `headless`: Python, MCP, the agent's connection and the shared cases' runner.
pub fn register() {
    register_as("headless");
}

/// [`register`] for a host that says what it is in the runs' records (`desktop`).
pub fn register_as(host: &'static str) {
    let _ = HOST.set(host);
    register_tools(Box::new(Tools));
}

struct Tools;

fn refused(code: &str, message: String, path: Option<String>) -> Box<CommandError> {
    Box::new(CommandError {
        code: code.to_owned(),
        message,
        path,
        ..CommandError::default()
    })
}

/// A parameter kind's name in the command's words.
fn kind_name(kind: &ParamKind) -> &'static str {
    match kind {
        ParamKind::Features { .. } => "features",
        ParamKind::Number { .. } => "number",
        ParamKind::Text { .. } => "text",
        ParamKind::Boolean => "boolean",
        ParamKind::Choice { .. } => "choice",
        ParamKind::Layer { .. } => "layer",
        ParamKind::Point => "point",
        ParamKind::Expression { .. } => "expression",
        ParamKind::Field { .. } => "field",
        ParamKind::File { .. } => "file",
        ParamKind::SaveFile { .. } => "saveFile",
        ParamKind::Network { .. } => "network",
        ParamKind::AddressSource => "addressSource",
        ParamKind::RasterValues { .. } => "rasterValues",
        ParamKind::RasterPairs { .. } => "rasterPairs",
        ParamKind::FieldMap { .. } => "fieldMap",
        ParamKind::Layers { .. } => "layers",
    }
}

/// Whether `tool` is in `category` or one of its sub-categories.
fn in_category(registry: &Registry, tool: &Tool, category: &str) -> bool {
    let mut at = registry.category(&tool.category);
    while let Some(c) = at {
        if c.id == category {
            return true;
        }
        at = c.parent.and_then(|p| registry.category(p));
    }
    false
}

fn described(registry: &Registry, tool: &Tool, doc: &Document) -> ProcessingTool {
    let defaults = default_values(tool, &Defaults::of(doc));
    ProcessingTool {
        id: tool.id.clone(),
        label: tool.label.clone(),
        category: registry.category_path(&tool.category),
        description: tool.description.clone(),
        aliases: tool.aliases.clone(),
        parameters: tool
            .parameters
            .iter()
            .map(|p| ToolParameter {
                name: p.name.clone(),
                label: p.label.clone(),
                kind: kind_name(&p.kind).to_owned(),
                description: p.description.clone(),
                optional: p.optional,
                advanced: p.advanced,
                default: defaults.get(&p.name).filter(|v| !v.is_null()).cloned(),
                options: match &p.kind {
                    ParamKind::Choice { options, .. } => {
                        options.iter().map(|o| o.value.clone()).collect()
                    }
                    _ => Vec::new(),
                },
            })
            .collect(),
    }
}

/// The objects `uids` name, in their order.
fn slots_of(doc: &Document, uids: &[String], path: &str) -> Result<Vec<Slot>, Box<CommandError>> {
    uids.iter()
        .enumerate()
        .map(|(i, text)| {
            let at = Some(format!("{path}[{i}]"));
            let uid = Uuid::parse_str(text).map_err(|_| {
                refused(
                    codes::INVALID_UID,
                    format!("“{text}” bir nesne kimliği değil (UUID olmalı)."),
                    at.clone(),
                )
            })?;
            doc.slot_of(uid).ok_or_else(|| {
                refused(
                    codes::ENTITY_NOT_FOUND,
                    format!("{text} kimlikli nesne çizimde yok."),
                    at,
                )
            })
        })
        .collect()
}

/// A run made ready: the tool, its values with the drawing's defaults, the selection and the box it reads.
struct Ready {
    tool: Tool,
    /// The model, when the tool is one (`model:…`): it runs its steps.
    model: Option<Model>,
    values: Values,
    selection: Vec<Slot>,
    view: Option<Bounds>,
}

fn invalid(param: &str, message: String) -> Box<CommandError> {
    refused(
        codes::INVALID_PARAMETERS,
        message,
        Some(format!("parameters.{param}")),
    )
}

/// A features value in the runner's words: `uids` become the objects' slots.
fn features_value(
    doc: &Document,
    name: &str,
    v: &Value,
    has_view: bool,
) -> Result<Value, Box<CommandError>> {
    let Some(o) = v.as_object() else {
        return Err(invalid(
            name,
            format!("“{name}” nesneleri {{ \"scope\": … }} biçiminde verilir."),
        ));
    };
    let mut out = o.clone();
    match o.get("scope").and_then(Value::as_str) {
        Some("uids") => {
            let uids: Vec<String> = o
                .get("uids")
                .and_then(Value::as_array)
                .map(|l| {
                    l.iter()
                        .filter_map(|u| u.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let slots = slots_of(doc, &uids, &format!("parameters.{name}.uids"))?;
            out.remove("uids");
            out.insert("scope".into(), json!("ids"));
            out.insert(
                "ids".into(),
                json!(slots.iter().map(|s| s.0).collect::<Vec<_>>()),
            );
        }
        Some("ids") => {
            return Err(invalid(
                name,
                format!(
                    "“{name}”: nesneler kalıcı kimlikleriyle verilir ({{ \"scope\": \"uids\", \"uids\": [ … ] }})."
                ),
            ));
        }
        Some("visible") if !has_view => {
            return Err(invalid(
                name,
                format!("“{name}”: görünen alandakiler için girdide view (kutu) verilmeli."),
            ));
        }
        _ => {}
    }
    Ok(Value::Object(out))
}

/// A file value: `{ "path": … }` read as the dialog reads a chosen table file, or the rows given.
fn file_value(name: &str, v: &Value) -> Result<Value, Box<CommandError>> {
    let Some(path) = v.get("path").and_then(Value::as_str) else {
        return Ok(v.clone());
    };
    let bytes = std::fs::read(path)
        .map_err(|e| invalid(name, format!("“{name}”: {path} okunamadı: {e}.")))?;
    let read = kentos_formats::table_file::read_with(
        &bytes,
        kentos_formats::table_file::Limits::PROCESSING,
    );
    let rows = read
        .sheets
        .first()
        .map(|s| s.rows.clone())
        .ok_or_else(|| invalid(name, format!("“{name}”: {path} içinde tablo yok.")))?;
    let file = std::path::Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
    Ok(json!({ "name": file, "rows": rows }))
}

fn ready(
    registry: &Registry,
    doc: &Document,
    input: &ProcessingRun,
) -> Result<Ready, Box<CommandError>> {
    let Some(tool) = registry.get(&input.tool) else {
        return Err(refused(
            codes::UNKNOWN_TOOL,
            format!(
                "“{}” bir işlem aracı değil. Araçların listesi cad.processing.tools'ta.",
                input.tool
            ),
            Some("tool".into()),
        ));
    };
    let model = input
        .tool
        .strip_prefix(MODEL_PREFIX)
        .and_then(|id| registry.model(id))
        .cloned();
    let selection = slots_of(doc, &input.selection, "selection")?;
    let view = input.view.map(|[min_x, min_y, max_x, max_y]| Bounds {
        min_x,
        min_y,
        max_x,
        max_y,
    });
    let mut values = default_values(&tool, &Defaults::of(doc));
    for (name, v) in &input.parameters {
        let Some(p) = tool.parameters.iter().find(|p| &p.name == name) else {
            let known: Vec<&str> = tool.parameters.iter().map(|p| p.name.as_str()).collect();
            return Err(invalid(
                name,
                format!(
                    "“{}” aracında “{name}” parametresi yok; parametreleri: {}.",
                    tool.label,
                    known.join(", ")
                ),
            ));
        };
        let v = match &p.kind {
            ParamKind::Features { .. } => features_value(doc, name, v, view.is_some())?,
            ParamKind::File { .. } => file_value(name, v)?,
            _ => v.clone(),
        };
        values.insert(name.clone(), v);
    }
    // A features input left at its default reads the screen's scope it names: the input's own selection and box.
    for p in &tool.parameters {
        if let (ParamKind::Features { .. }, Some(v)) = (&p.kind, values.get(&p.name))
            && v.get("scope").and_then(Value::as_str) == Some("visible")
            && view.is_none()
        {
            return Err(invalid(
                &p.name,
                format!(
                    "“{}”: görünen alandakiler için girdide view (kutu) verilmeli.",
                    p.name
                ),
            ));
        }
    }
    let runner = Runner::new();
    if let Some(issue) = runner.validate(&tool, &values, doc).into_iter().next() {
        return Err(refused(
            codes::INVALID_PARAMETERS,
            issue.message,
            issue.param.map(|p| format!("parameters.{p}")),
        ));
    }
    Ok(Ready {
        tool,
        model,
        values,
        selection,
        view,
    })
}

/// The drawing as a run sees it: no screen, the input's selection and box.
struct Headless<'d> {
    doc: &'d mut Document,
    selection: Vec<Slot>,
    view: Option<Bounds>,
    chosen: Option<Vec<Slot>>,
}

impl Scene for Headless<'_> {
    fn doc(&self) -> &Document {
        self.doc
    }

    fn selected(&self) -> Vec<Slot> {
        self.selection.clone()
    }

    fn visible_bounds(&self) -> Option<Bounds> {
        self.view
    }
}

impl Host for Headless<'_> {
    fn doc_mut(&mut self) -> &mut Document {
        self.doc
    }

    fn select(&mut self, ids: &[Slot]) {
        self.chosen = Some(ids.to_vec());
    }
}

fn node_ids(nodes: &[kentos_commands::LayerNode], out: &mut BTreeSet<String>) {
    for n in nodes {
        out.insert(n.id.clone());
        node_ids(&n.children, out);
    }
}

fn layer_ids(doc: &Document) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    node_ids(doc.layers().nodes(), &mut out);
    out
}

fn uids(doc: &Document, slots: &[Slot]) -> Vec<String> {
    slots
        .iter()
        .filter_map(|s| doc.uid(*s))
        .map(|u| u.to_string())
        .collect()
}

/// An output in the command's words: a features output's slots as persistent ids.
fn output_value(doc: &Document, kind: Option<OutputKind>, v: &Value) -> Value {
    if kind != Some(OutputKind::Features) {
        return v.clone();
    }
    let slots: Option<Vec<Slot>> = match v {
        Value::Array(list) => list
            .iter()
            .map(|n| n.as_u64().and_then(|n| u32::try_from(n).ok()).map(Slot))
            .collect(),
        Value::Object(o) => o.get("ids").and_then(Value::as_array).and_then(|list| {
            list.iter()
                .map(|n| n.as_u64().and_then(|n| u32::try_from(n).ok()).map(Slot))
                .collect()
        }),
        _ => None,
    };
    slots.map_or_else(|| v.clone(), |s| json!(uids(doc, &s)))
}

impl ToolHost for Tools {
    fn list(
        &self,
        doc: &Document,
        input: &ProcessingTools,
    ) -> Result<ProcessingToolsListed, Box<CommandError>> {
        let registry = Registry::builtin();
        if let Some(c) = input.category.as_deref()
            && registry.category(c).is_none()
        {
            return Err(refused(
                codes::INVALID_INPUT,
                format!("“{c}” bir işlem kategorisi değil."),
                Some("category".into()),
            ));
        }
        let query = input.query.as_deref().unwrap_or("");
        let folded = fold_turkish(query);
        let mut found: Vec<&Tool> = registry.search(query);
        // An alias or id typed whole finds its tool too.
        for t in registry.tools() {
            let named = t.id == query || t.aliases.iter().any(|a| fold_turkish(a) == folded);
            if named && !found.iter().any(|f| f.id == t.id) {
                found.push(t);
            }
        }
        found.retain(|t| {
            input
                .category
                .as_deref()
                .is_none_or(|c| in_category(&registry, t, c))
        });
        found.sort_by(|a, b| a.id.cmp(&b.id));
        let mut tools: Vec<ProcessingTool> = found
            .into_iter()
            .map(|t| described(&registry, t, doc))
            .collect();
        // The models, when asked (docs/adr/0529): a model's inputs are its parameters.
        if input.models {
            let words: Vec<&str> = folded.split_whitespace().collect();
            let lookup = |id: &str| registry.tool(id);
            for m in registry.models() {
                let as_tool = model_as_tool(m, &lookup);
                let hay = fold_turkish(&format!(
                    "{} {} {}",
                    as_tool.id, as_tool.label, as_tool.description
                ));
                let found = words.iter().all(|w| hay.contains(w)) || as_tool.id == query;
                let in_cat = input
                    .category
                    .as_deref()
                    .is_none_or(|c| in_category(&registry, &as_tool, c));
                if found && in_cat {
                    tools.push(described(&registry, &as_tool, doc));
                }
            }
        }
        Ok(ProcessingToolsListed { tools })
    }

    fn check(&self, doc: &Document, input: &ProcessingRun) -> Result<(), Box<CommandError>> {
        ready(&Registry::builtin(), doc, input).map(|_| ())
    }

    fn run(
        &self,
        doc: &mut Document,
        input: &ProcessingRun,
        write: bool,
    ) -> Result<(ProcessingRan, Vec<CommandWarning>), Box<CommandError>> {
        let registry = Registry::builtin();
        let ready = ready(&registry, doc, input)?;
        let Some(sink) = &input.sink else {
            let (ran, _) = run_on(doc, ready, &registry, &mut Digests::new())?;
            return Ok((ran, Vec::new()));
        };
        let target = sink_target(sink)?;
        let (tool_id, tool_label) = (ready.tool.id.clone(), ready.tool.label.clone());
        // The run is made on a copy: the drawing does not change, the file takes the result.
        let mut copy = doc.reading_copy();
        let (mut ran, added) = run_on(&mut copy, ready, &registry, &mut Digests::new())?;
        let lineage = Lineage {
            tool: (&tool_id, &tool_label),
            request: input,
            inputs: &ran.inputs,
            drawing: doc.revision().to_string(),
        };
        let (exported, warnings) = write_sink(&mut copy, &added, &target, write, &lineage)?;
        ran.sink = Some(exported);
        ran.step = None;
        ran.revision = doc.revision().to_string();
        Ok((ran, warnings))
    }

    fn check_batch(
        &self,
        doc: &Document,
        input: &ProcessingBatch,
    ) -> Result<(), Box<CommandError>> {
        let registry = Registry::builtin();
        batch_size(input)?;
        for (i, row) in input.rows.iter().enumerate() {
            ready(&registry, doc, &row_run(input, row)).map_err(|e| in_row(e, i, row))?;
        }
        Ok(())
    }

    fn batch(
        &self,
        doc: &mut Document,
        input: &ProcessingBatch,
        flow: &dyn BatchFlow,
    ) -> Result<BatchEnd, Box<CommandError>> {
        let registry = Registry::builtin();
        batch_size(input)?;
        let clock = (now_ms(), Instant::now());
        let revision = doc.revision().to_string();
        let Some(tool) = registry.get(&input.tool) else {
            return Err(refused(
                codes::UNKNOWN_TOOL,
                format!(
                    "“{}” bir işlem aracı değil. Araçların listesi cad.processing.tools'ta.",
                    input.tool
                ),
                Some("tool".into()),
            ));
        };
        let total = input.rows.len() as u64;
        // One undo step for all the rows, taken back whole when one stops (docs/adr/0529 §1).
        let group = doc.begin_group(&format!("Toplu: {}", tool.label));
        let mut ran_rows: Vec<BatchRowRan> = Vec::with_capacity(input.rows.len());
        // A layer's digest is taken once for each state of the drawing: rows that leave it alone share it.
        let mut digests = Digests::new();
        for (i, row) in input.rows.iter().enumerate() {
            if flow.stopped() {
                doc.cancel_group(group);
                return Ok(BatchEnd::Cancelled);
            }
            flow.progress(i as u64, total);
            let label = row_label(i, row);
            let made = ready(&registry, doc, &row_run(input, row))
                .and_then(|r| run_on(doc, r, &registry, &mut digests))
                .map(|(ran, _)| ran);
            match made {
                Ok(ran) => ran_rows.push(BatchRowRan {
                    index: i as u32,
                    label,
                    ran,
                }),
                Err(e) => {
                    doc.cancel_group(group);
                    return Err(Box::new(CommandError {
                        code: codes::BATCH_ROW_FAILED.into(),
                        message: format!(
                            "{label} çalışmadı: {} Toplu işin hiçbir sonucu yazılmadı.",
                            e.message
                        ),
                        path: Some(format!("rows[{i}]")),
                        ..CommandError::default()
                    }));
                }
            }
        }
        flow.progress(total, total);
        doc.end_group(group);
        let edited = ran_rows.iter().any(|r| r.ran.step.is_some());
        let record = batch_record(
            &registry,
            input,
            &tool,
            &ran_rows,
            revision,
            doc.settings().srid,
            clock,
        );
        let added: usize = ran_rows.iter().map(|r| r.ran.added.len()).sum();
        let changed: usize = ran_rows.iter().map(|r| r.ran.changed.len()).sum();
        Ok(BatchEnd::Ran(ProcessingBatched {
            tool: input.tool.clone(),
            summary: format!(
                "{} satır çalıştı: {added} nesne eklendi, {changed} nesne değişti.",
                ran_rows.len()
            ),
            rows: ran_rows,
            step: edited.then(|| format!("Toplu: {}", tool.label)),
            record,
            revision: doc.revision().to_string(),
        }))
    }
}

/// The most rows one batch takes (docs/adr/0529 §1).
pub const MAX_BATCH_ROWS: usize = 500;

/// A batch's refusals of its size: no rows, too many.
fn batch_size(input: &ProcessingBatch) -> Result<(), Box<CommandError>> {
    if input.rows.is_empty() {
        return Err(refused(
            codes::EMPTY_BATCH,
            "Toplu çalıştırmanın satırı yok; en az bir satır verin.".into(),
            Some("rows".into()),
        ));
    }
    if input.rows.len() > MAX_BATCH_ROWS {
        return Err(refused(
            codes::TOO_MANY_ROWS,
            format!(
                "{} satır verildi; toplu çalıştırma en çok {MAX_BATCH_ROWS} satır alır. Satırları bölüp ayrı çağırın.",
                input.rows.len()
            ),
            Some("rows".into()),
        ));
    }
    Ok(())
}

/// A row's name in the answer: the label it gave, else “Satır N” (N from 1).
fn row_label(i: usize, row: &kentos_commands::BatchRow) -> String {
    row.label
        .as_deref()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map_or_else(|| format!("Satır {}", i + 1), str::to_owned)
}

/// A row as the run it is: the batch's values with the row's own over them, the batch's selection and view.
fn row_run(input: &ProcessingBatch, row: &kentos_commands::BatchRow) -> ProcessingRun {
    let mut parameters = input.parameters.clone();
    for (k, v) in &row.parameters {
        parameters.insert(k.clone(), v.clone());
    }
    ProcessingRun {
        tool: input.tool.clone(),
        parameters,
        selection: input.selection.clone(),
        view: input.view,
        sink: None,
    }
}

/// A row's refusal with its parameter's path taken into the row (`rows[2].parameters.distance`) when the row gave
/// that parameter its own value; a value every row shares keeps `parameters.…`.
fn in_row(
    mut e: Box<CommandError>,
    i: usize,
    row: &kentos_commands::BatchRow,
) -> Box<CommandError> {
    if let Some(path) = e.path.take() {
        let own = path
            .strip_prefix("parameters.")
            .and_then(|rest| rest.split(['.', '[']).next())
            .is_some_and(|name| row.parameters.contains_key(name));
        e.path = Some(if own {
            format!("rows[{i}].{path}")
        } else {
            path
        });
    }
    e
}

/// What a run began with, taken before it changes the drawing (docs/adr/0529 §2).
struct Begun {
    inputs: Vec<RunInput>,
    revision: String,
    parameters: BTreeMap<String, Value>,
    selection: Vec<String>,
    view: Option<[f64; 4]>,
    srid: Option<u32>,
    seed: Option<u64>,
    definition: Option<String>,
}

impl Begun {
    fn of(doc: &Document, ready: &Ready, digests: &mut Digests) -> Self {
        let srid = doc.settings().srid;
        Begun {
            inputs: inputs_of(doc, ready, digests),
            revision: doc.revision().to_string(),
            parameters: restated(doc, &ready.tool, &ready.values),
            selection: uids(doc, &ready.selection),
            view: ready.view.map(|b| [b.min_x, b.min_y, b.max_x, b.max_y]),
            srid: (srid != 0).then_some(srid),
            seed: ready
                .values
                .get("seed")
                .and_then(Value::as_f64)
                .filter(|n| *n >= 0.0 && n.fract() == 0.0 && *n < 9e15)
                .map(|n| n as u64),
            definition: ready.model.as_ref().map(definition_digest),
        }
    }
}

/// A window's values in the words the processing commands take (and a record keeps): a file as its name and path,
/// objects as persistent ids (docs/adr/0529 §4).
pub fn command_values(doc: &Document, tool: &Tool, values: &Values) -> BTreeMap<String, Value> {
    restated(doc, tool, values)
}

/// The values a run took in the words a record keeps: a file as its name and path, objects as persistent ids.
fn restated(doc: &Document, tool: &Tool, values: &Values) -> BTreeMap<String, Value> {
    stored_values(tool, values)
        .into_iter()
        .map(|(k, v)| {
            let slots: Option<Vec<Slot>> = (v.get("scope").and_then(Value::as_str) == Some("ids"))
                .then(|| v.get("ids").and_then(Value::as_array))
                .flatten()
                .and_then(|list| {
                    list.iter()
                        .map(|n| n.as_u64().and_then(|n| u32::try_from(n).ok()).map(Slot))
                        .collect()
                });
            let v = match (slots, v.as_object()) {
                (Some(slots), Some(o)) => {
                    let mut out = o.clone();
                    out.remove("ids");
                    out.insert("scope".into(), json!("uids"));
                    out.insert("uids".into(), json!(uids(doc, &slots)));
                    Value::Object(out)
                }
                _ => v,
            };
            (k, v)
        })
        .collect()
}

/// A model's definition as a digest: its steps, their wiring and fixed values, its inputs and outputs; where its boxes
/// sit on the designer's canvas is not part of what it computes.
fn definition_digest(model: &Model) -> String {
    let mut v = model.to_json();
    if let Some(o) = v.as_object_mut() {
        o.remove("inputPositions");
        if let Some(steps) = o.get_mut("steps").and_then(Value::as_array_mut) {
            for s in steps {
                if let Some(s) = s.as_object_mut() {
                    s.remove("position");
                    s.remove("caption");
                }
            }
        }
    }
    kentos_commands::encoding::sha256_hex(v.to_string().as_bytes())
}

/// The tool's version: bumped when what it computes from the same values changes (docs/adr/0529 §2). Every tool is at
/// 1 today; a model is of the tool-and-model contract's 1.
fn tool_version(_id: &str) -> u32 {
    1
}

const RECORD_FORMAT: &str = "kentos.run-record";

fn host_name() -> &'static str {
    HOST.get().copied().unwrap_or("headless")
}

fn engine() -> String {
    format!("KentOS {}", env!("CARGO_PKG_VERSION"))
}

/// The record of a tool's or a model's run (docs/adr/0529 §2), from what it began with and the history record the
/// runner made of it.
fn record_of(
    begun: Begun,
    tool: &Tool,
    model: Option<&Model>,
    ran: &RunRecord,
    grids: Vec<kentos_commands::RecordGrid>,
) -> ProcessingRecord {
    let steps = if model.is_some() {
        ran.steps
            .iter()
            .map(|s| RecordStep {
                id: s.id.clone(),
                tool: s.tool_id.clone(),
                label: s.label.clone(),
                records_in: s.records_in,
                records_out: s.records_out,
                ms: s.ms,
                summary: s.summary.clone(),
            })
            .collect()
    } else {
        vec![RecordStep {
            id: tool.id.clone(),
            tool: tool.id.clone(),
            label: tool.label.clone(),
            records_in: ran.records_in,
            records_out: ran.records_out,
            ms: ran.ms,
            summary: ran.summary.clone(),
        }]
    };
    ProcessingRecord {
        format: RECORD_FORMAT.into(),
        version: 1,
        kind: if model.is_some() { "model" } else { "tool" }.into(),
        tool: tool.id.clone(),
        label: tool.label.clone(),
        tool_version: tool_version(&tool.id),
        definition: begun.definition,
        parameters: begun.parameters,
        selection: begun.selection,
        view: begun.view,
        inputs: begun.inputs,
        document_revision: begun.revision,
        srid: begun.srid,
        seed: begun.seed,
        engine: engine(),
        host: host_name().into(),
        started_at: utc_at(ran.started),
        ms: ran.ms,
        grids,
        steps,
        total: RecordTotal {
            records_in: ran.records_in,
            records_out: ran.records_out,
        },
    }
}

/// A batch's record (docs/adr/0529 §2): its rows are the steps, the layers they read are its inputs.
fn batch_record(
    registry: &Registry,
    input: &ProcessingBatch,
    tool: &Tool,
    rows: &[BatchRowRan],
    revision: String,
    srid: u32,
    clock: (u64, Instant),
) -> ProcessingRecord {
    let mut inputs: Vec<RunInput> = Vec::new();
    let mut touched: BTreeSet<&str> = BTreeSet::new();
    for r in rows {
        for i in &r.ran.inputs {
            if !inputs.iter().any(|x| x.layer == i.layer) {
                inputs.push(i.clone());
            }
        }
        touched.extend(
            r.ran
                .added
                .iter()
                .chain(&r.ran.changed)
                .chain(&r.ran.selected)
                .map(String::as_str),
        );
    }
    let records_in: u64 = rows.iter().map(|r| r.ran.record.total.records_in).sum();
    ProcessingRecord {
        format: RECORD_FORMAT.into(),
        version: 1,
        kind: "batch".into(),
        tool: tool.id.clone(),
        label: tool.label.clone(),
        tool_version: tool_version(&tool.id),
        definition: input
            .tool
            .strip_prefix(MODEL_PREFIX)
            .and_then(|id| registry.model(id))
            .map(definition_digest),
        parameters: input.parameters.clone(),
        selection: input.selection.clone(),
        view: input.view,
        inputs,
        document_revision: revision,
        srid: (srid != 0).then_some(srid),
        seed: None,
        engine: engine(),
        host: host_name().into(),
        started_at: utc_at(clock.0),
        ms: clock.1.elapsed().as_millis() as u64,
        grids: rows
            .iter()
            .flat_map(|r| r.ran.record.grids.iter().cloned())
            .collect(),
        steps: rows
            .iter()
            .map(|r| RecordStep {
                id: format!("rows[{}]", r.index),
                tool: tool.id.clone(),
                label: r.label.clone(),
                records_in: r.ran.record.total.records_in,
                records_out: r.ran.record.total.records_out,
                ms: r.ran.record.ms,
                summary: r.ran.summary.clone(),
            })
            .collect(),
        total: RecordTotal {
            records_in,
            records_out: touched.len() as u64,
        },
    }
}

/// Runs a made-ready tool or model on `doc` (the drawing or a copy): what it did, with its record, and the objects it
/// made.
fn run_on(
    doc: &mut Document,
    ready: Ready,
    registry: &Registry,
    digests: &mut Digests,
) -> Result<(ProcessingRan, Vec<Slot>), Box<CommandError>> {
    let begun = Begun::of(doc, &ready, digests);
    let Ready {
        tool,
        model,
        values,
        selection,
        view,
    } = ready;
    let before = layer_ids(doc);
    let mut host = Headless {
        doc,
        selection,
        view,
        chosen: None,
    };
    let mut log = Vec::new();
    // The raster tools read and write files where there is no window too (docs/adr/0515).
    let files = crate::local_files::LocalFiles::of(host.doc, None);
    let mut runner = Runner::with_files(std::sync::Arc::new(files));
    let outcome = match &model {
        // A model's steps one after another, one undo step, taken back whole when one stops.
        Some(m) => {
            let lookup = |id: &str| registry.tool(id);
            run_model(m, &values, &mut runner, &mut host, &lookup, &mut log)
        }
        None => runner.run(&mut host, &tool, &values, true, &mut log),
    };
    let Headless { doc, chosen, .. } = host;
    let notes: Vec<ToolNote> = log
        .iter()
        .map(|l| ToolNote {
            level: match l.level {
                Level::Info => "info",
                Level::Warn => "warn",
            }
            .to_owned(),
            text: l.text.clone(),
        })
        .collect();
    match outcome {
        Outcome::Ok {
            result,
            added,
            updated,
            edited,
            record,
            ..
        } => {
            let kinds: BTreeMap<String, OutputKind> = match &model {
                None => tool
                    .outputs
                    .iter()
                    .map(|o| (o.name.clone(), o.kind))
                    .collect(),
                Some(m) => m
                    .outputs
                    .iter()
                    .filter_map(|o| {
                        let step = m.steps.iter().find(|s| s.id == o.step)?;
                        let t = registry.tool(&step.tool)?;
                        let kind = t.outputs.iter().find(|x| x.name == o.output)?.kind;
                        Some((o.name.clone(), kind))
                    })
                    .collect(),
            };
            let outputs = result
                .outputs
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        output_value(doc, kinds.get(k.as_str()).copied(), v),
                    )
                })
                .collect();
            let added_set: BTreeSet<Slot> = added.iter().copied().collect();
            let changed: Vec<Slot> = updated
                .into_iter()
                .filter(|s| !added_set.contains(s))
                .collect();
            let layers = layer_ids(doc).difference(&before).cloned().collect();
            let inputs = begun.inputs.clone();
            let ran = ProcessingRan {
                tool: tool.id.clone(),
                summary: record.summary.clone(),
                added: uids(doc, &added),
                changed: uids(doc, &changed),
                selected: chosen.map(|c| uids(doc, &c)).unwrap_or_default(),
                layers,
                outputs,
                notes,
                inputs,
                sink: None,
                step: edited.then(|| tool.label.clone()),
                record: record_of(begun, &tool, model.as_ref(), &record, result.grids.clone()),
                revision: doc.revision().to_string(),
            };
            Ok((ran, added))
        }
        Outcome::Invalid { issues } => {
            let first = issues.into_iter().next();
            Err(refused(
                codes::INVALID_PARAMETERS,
                first
                    .as_ref()
                    .map_or_else(|| "Değerler uymuyor.".to_owned(), |i| i.message.clone()),
                first
                    .and_then(|i| i.param)
                    .map(|p| format!("parameters.{p}")),
            ))
        }
        Outcome::Stopped { message, .. } => {
            Err(refused(codes::TOOL_FAILED, message, Some("tool".into())))
        }
    }
}

/// The content digests taken so far, by the drawing's generation and the layer (docs/adr/0529 §6): hashing a big layer
/// costs more than the run that reads it, and a generation is one state of the drawing.
type Digests = BTreeMap<(u64, String), String>;

/// The layers the features inputs read, in the inputs' order: a layer named, or the layers of the objects named or
/// chosen; a bound layer with its source and the revision it read. The visible layers (`all`) and the view are not
/// listed one by one.
fn inputs_of(doc: &Document, ready: &Ready, digests: &mut Digests) -> Vec<RunInput> {
    let mut layers: Vec<String> = Vec::new();
    let mut add = |id: &str| {
        if !layers.iter().any(|l| l == id) {
            layers.push(id.to_owned());
        }
    };
    let layer_of = |s: Slot| doc.get(s).map(|e| e.base().layer_id.clone());
    for p in &ready.tool.parameters {
        let (ParamKind::Features { .. }, Some(v)) = (&p.kind, ready.values.get(&p.name)) else {
            continue;
        };
        match v.get("scope").and_then(Value::as_str) {
            Some("layer") => {
                if let Some(id) = v.get("layerId").and_then(Value::as_str) {
                    add(id);
                }
            }
            Some("ids") => {
                for n in v.get("ids").and_then(Value::as_array).into_iter().flatten() {
                    if let Some(id) = n
                        .as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .and_then(|n| layer_of(Slot(n)))
                    {
                        add(&id);
                    }
                }
            }
            Some("selection") => {
                for s in &ready.selection {
                    if let Some(id) = layer_of(*s) {
                        add(&id);
                    }
                }
            }
            _ => {}
        }
    }
    let sources = &doc.settings().sources;
    layers
        .into_iter()
        .map(|layer| {
            let link = sources.iter().find(|s| s.layer == layer);
            RunInput {
                source: link.map(|s| s.id.clone()),
                revision: link.map(|s| s.revision.clone()),
                digest: digests
                    .entry((doc.generation(), layer.clone()))
                    .or_insert_with(|| {
                        kentos_native_application::sources::layer_digest(doc, &layer)
                    })
                    .clone(),
                layer,
            }
        })
        .collect()
}

/// Where a sink writes, or why not there: an absolute `.gpkg` path in a folder that is there; a file that is there
/// only with `overwrite`.
fn sink_target(sink: &RunSink) -> Result<(PathBuf, bool), Box<CommandError>> {
    let at = Some("sink.path".to_owned());
    let path = sink.path.trim();
    let p = Path::new(path);
    if path.is_empty() || !p.is_absolute() {
        return Err(refused(
            codes::INVALID_FILE_PATH,
            format!(
                "“{path}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/sonuç.gpkg)."
            ),
            at,
        ));
    }
    if !p
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gpkg"))
    {
        return Err(refused(
            codes::INVALID_FILE_PATH,
            format!("{path} bir GeoPackage adı değil; sonucun dosyası .gpkg ile biter."),
            at,
        ));
    }
    if p.is_dir() || !p.parent().is_some_and(Path::is_dir) {
        return Err(refused(
            codes::INVALID_FILE_PATH,
            format!("{path} dosyasının klasörü yok; önce klasörü açın ya da başka bir yol verin."),
            at,
        ));
    }
    let exists = p.exists();
    if exists && !sink.overwrite {
        return Err(refused(
            codes::FILE_EXISTS,
            format!("{path} var; üzerine yazmak için overwrite: true verin."),
            at,
        ));
    }
    Ok((p.to_path_buf(), exists))
}

/// What a result came from (docs/adr/0511): the tool, the request as given, the layers it read, the drawing's revision.
struct Lineage<'a> {
    tool: (&'a str, &'a str),
    request: &'a ProcessingRun,
    inputs: &'a [RunInput],
    drawing: String,
}

/// The URI of KentOS's lineage document in `gpkg_metadata` (docs/adr/0511).
pub const LINEAGE_STANDARD: &str = "https://kentos.app/lineage/1";

impl Lineage<'_> {
    /// The table's lineage document: `kentos.lineage` 1, as JSON.
    fn document(&self, table: &str, layer: &str, made: &str) -> String {
        let mut request = serde_json::to_value(self.request).unwrap_or(Value::Null);
        if let Some(o) = request.as_object_mut() {
            o.remove("sink");
        }
        json!({
            "format": "kentos.lineage",
            "version": 1,
            "table": table,
            "layer": layer,
            "tool": { "id": self.tool.0, "label": self.tool.1 },
            "request": request,
            "inputs": self.inputs,
            "drawing": { "revision": self.drawing },
            "engine": format!("KentOS {}", env!("CARGO_PKG_VERSION")),
            "made": made,
        })
        .to_string()
    }
}

/// Milliseconds since the Unix epoch now.
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// The time now, `YYYY-MM-DDTHH:MM:SS.SSSZ`.
fn utc_now() -> String {
    utc_at(now_ms())
}

/// A time as `YYYY-MM-DDTHH:MM:SS.SSSZ`, from milliseconds since the Unix epoch.
fn utc_at(ms: u64) -> String {
    let text = kentos_geometry_core::time::write(ms as f64, false);
    // A whole second is written without its milliseconds, and midnight as the day alone.
    let text = if text.len() == 10 {
        format!("{text}T00:00:00")
    } else {
        text
    };
    let text = if text.len() == 19 {
        format!("{text}.000")
    } else {
        text
    };
    format!("{text}Z")
}

/// The objects a run made on `copy` written to the sink's GeoPackage, each layer they are on a table; the other
/// objects of the copy are left out first. The file is made beside the target under a passing name and, with
/// `write`, put in its place in one step; a plan removes it. What it holds, and what it could not hold.
fn write_sink(
    copy: &mut Document,
    added: &[Slot],
    (path, _): &(PathBuf, bool),
    write: bool,
    lineage: &Lineage<'_>,
) -> Result<(GpkgExported, Vec<CommandWarning>), Box<CommandError>> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let at = Some("sink.path".to_owned());
    let keep: HashSet<Slot> = added.iter().copied().collect();
    let others: Vec<Slot> = copy
        .entities()
        .map(|e| Slot(e.base().id))
        .filter(|s| !keep.contains(s))
        .collect();
    copy.remove(&others);
    let mut layers: Vec<String> = Vec::new();
    for s in added {
        if let Some(e) = copy.get(*s)
            && !layers.contains(&e.base().layer_id)
        {
            layers.push(e.base().layer_id.clone());
        }
    }
    if layers.is_empty() {
        return Err(refused(
            codes::NOTHING_TO_EXPORT,
            "Araç yeni nesne yapmadı; dosyaya yazılacak sonuç yok (değiştiren ya da seçen araçların sonucu çizimdedir)."
                .into(),
            at,
        ));
    }
    let tmp = path.with_extension(format!(
        "gpkg.kentos-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&tmp);
    let written = kentos_native_application::geopackage::write(copy, &layers, &tmp).and_then(|w| {
        // Each table says what it came from (docs/adr/0511): GeoPackage's metadata extension.
        let gp = kentos_geopackage::GeoPackage::open(&tmp, true).map_err(|e| e.0)?;
        let made = utc_now();
        for t in &w.tables {
            gp.add_metadata(
                &t.table,
                LINEAGE_STANDARD,
                "application/json",
                &lineage.document(&t.table, &t.layer, &made),
                &made,
            )
            .map_err(|e| e.0)?;
        }
        gp.close().map_err(|e| e.0)?;
        Ok(w)
    });
    let bytes = std::fs::read(&tmp);
    let unwritable = |why: String| {
        refused(
            codes::FILE_UNWRITABLE,
            format!("GeoPackage yazılamadı: {why}. Eski dosya olduğu gibi kaldı."),
            Some("sink.path".to_owned()),
        )
    };
    let (written, bytes) = match (written, bytes) {
        (Ok(w), Ok(b)) => (w, b),
        (Err(why), _) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(unwritable(why));
        }
        (_, Err(e)) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(unwritable(e.to_string()));
        }
    };
    if written.tables.is_empty() {
        let _ = std::fs::remove_file(&tmp);
        return Err(refused(
            codes::NOTHING_TO_EXPORT,
            "Aracın yaptığı nesnelerin GeoPackage'da geometrisi yok (yazı, ölçü, blok, tarama); dosyaya yazılacak sonuç yok."
                .into(),
            at,
        ));
    }
    if write {
        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(unwritable(e.to_string()));
        }
    } else {
        let _ = std::fs::remove_file(&tmp);
    }
    let mut warnings = Vec::new();
    let mut said: Vec<String> = Vec::new();
    if !written.left.is_empty() {
        let parts: Vec<String> = written
            .left
            .iter()
            .map(|(k, n)| format!("{n} {k}"))
            .collect();
        said.push(format!(
            "GeoPackage'da karşılığı olmayan nesneler yazılmadı: {}.",
            parts.join(", ")
        ));
    }
    if written.approximated > 0 {
        said.push(format!(
            "{} eğri (elips, eğri, NURBS) 0,1 mm'lik kirişlerle yazıldı.",
            written.approximated
        ));
    }
    for (table, from, to) in &written.renamed {
        said.push(format!(
            "“{table}” tablosunda “{from}” özniteliği “{to}” sütununa yazıldı: aynı adda (büyük küçük harf ayrı) bir sütun vardı."
        ));
    }
    if !said.is_empty() {
        warnings.push(CommandWarning {
            code: codes::EXPORT_LOSSES.into(),
            message: said.join(" "),
            path: Some("sink".into()),
        });
    }
    let exported = GpkgExported {
        path: path.display().to_string(),
        bytes: bytes.len() as u64,
        sha256: kentos_commands::encoding::sha256_hex(&bytes),
        tables: written
            .tables
            .into_iter()
            .map(|t| GpkgTableExported {
                table: t.table,
                layer: t.layer,
                rows: t.rows,
                geometry: t.geometry,
            })
            .collect(),
        left: written.left,
        approximated: written.approximated,
    };
    Ok((exported, warnings))
}
