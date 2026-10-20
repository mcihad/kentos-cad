//! İşlemler's tools for the processing commands (docs/adr/0508, plan/gis.md CBS-41): the application's
//! `cad.processing.tools` and `.run` reach the tools through [`register`]. A run is the window's run without a
//! screen: the values are the input's (a features input names its objects by persistent ids, or takes the input's
//! explicit `selection` and `view`), the drawing's defaults fill the rest, the runner checks and writes them as the
//! window does, one undo step named by the tool. With a sink (docs/adr/0508 §4) the run is made on a copy and the
//! objects it made go to a new GeoPackage; the drawing does not change.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use kentos_commands::{
    BatchOnError, BatchRowFailed, BatchRowRan, CommandError, CommandWarning, GpkgExported, GpkgTableExported, ProcessingBatch,
    ProcessingBatched, ProcessingRan, ProcessingRecord, ProcessingRun, ProcessingTool,
    ProcessingTools, ProcessingToolsListed, RecordFailure, RecordStep, RecordTotal, RunInput,
    RunSink, SinkColumn, SinkDimension, SinkMode, SinkReport, SinkTable, ToolNote, ToolParameter, codes,
};
use kentos_domain::{ChangeMark, Document, Slot, Uuid};
use kentos_geometry_core::geometry::Bounds;
use kentos_native_application::geopackage::{BATCH_ROWS, TextTable};
use kentos_native_application::processing_host::{
    BatchEnd, BatchFlow, RunEnd, ToolHost, register_tools,
};
use serde_json::{Value, json};

use crate::features::{Host, Scene};
use crate::model::Model;
use crate::model_runner::{MODEL_PREFIX, model_as_tool, run_model};
use crate::parameters::{default_values, stored_values};
use crate::registry::Registry;
use crate::runner::{Level, Outcome, RunRecord, Runner, Status};
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
    /// The open drawing's folder: a raster's relative path is found from it.
    folder: Option<PathBuf>,
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
    let folder = input
        .drawing
        .as_deref()
        .and_then(|d| Path::new(d).parent())
        .map(Path::to_path_buf);
    Ok(Ready {
        tool,
        model,
        values,
        selection,
        view,
        folder,
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
        flow: &dyn BatchFlow,
    ) -> Result<RunEnd, Box<CommandError>> {
        let registry = Registry::builtin();
        let ready = ready(&registry, doc, input)?;
        let stop = flow.stop_flag();
        let Some(sink) = &input.sink else {
            return match run_on(doc, ready, &registry, &mut Digests::new(), stop) {
                Ok((ran, _)) => Ok(RunEnd::Ran(ran, Vec::new())),
                Err(Halt::Cancelled) => Ok(RunEnd::Cancelled),
                Err(Halt::Refused(e)) => Err(e),
            };
        };
        let target = sink_target(sink)?;
        let (tool_id, tool_label) = (ready.tool.id.clone(), ready.tool.label.clone());
        let output_labels: BTreeMap<String, String> = ready
            .tool
            .outputs
            .iter()
            .map(|o| (o.name.clone(), o.label.clone()))
            .collect();
        // The run is made on a copy: the drawing does not change, the file takes the result.
        let mut copy = doc.reading_copy();
        let (mut ran, added) =
            match run_on(&mut copy, ready, &registry, &mut Digests::new(), stop) {
                Ok(made) => made,
                Err(Halt::Cancelled) => return Ok(RunEnd::Cancelled),
                Err(Halt::Refused(e)) => return Err(e),
            };
        // A stop that came while the tool computed and was not heard by it ends the run before the file is touched.
        if flow.stopped() {
            return Ok(RunEnd::Cancelled);
        }
        let texts = if sink.tables {
            text_tables(&ran.outputs, &output_labels)
        } else {
            Vec::new()
        };
        let lineage = Lineage {
            tool: (&tool_id, &tool_label),
            request: input,
            inputs: &ran.inputs,
            drawing: doc.revision().to_string(),
            run: &ran.record,
        };
        match write_sink(&mut copy, &added, &texts, &target, write, &lineage, flow)? {
            Sunk::Cancelled => Ok(RunEnd::Cancelled),
            Sunk::Done(done) => {
                let (exported, report, warnings) = *done;
                ran.sink = Some(exported);
                ran.sink_report = Some(report);
                ran.step = None;
                ran.revision = doc.revision().to_string();
                Ok(RunEnd::Ran(ran, warnings))
            }
        }
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
        let stop = flow.stop_flag();
        // One undo step for all the rows, taken back whole when one stops (docs/adr/0529 §1); with `onError: continue`
        // a row that stops is taken back alone, to the place the doc was at when the row began (docs/adr/0551 §1).
        let group = doc.begin_group(&format!("Toplu: {}", tool.label));
        let mut ran_rows: Vec<BatchRowRan> = Vec::with_capacity(input.rows.len());
        let mut failed: Vec<BatchRowFailed> = Vec::new();
        // A layer's digest is taken once for each state of the objects: rows that leave them alone share it.
        let mut digests = Digests::new();
        for (i, row) in input.rows.iter().enumerate() {
            if flow.stopped() {
                doc.cancel_group(group);
                return Ok(BatchEnd::Cancelled);
            }
            flow.progress(i as u64, total);
            let label = row_label(i, row);
            let at = doc.group_savepoint();
            let made = ready(&registry, doc, &row_run(input, row))
                .map_err(Halt::Refused)
                .and_then(|r| run_on(doc, r, &registry, &mut digests, stop.clone()))
                .map(|(ran, _)| ran);
            match made {
                Ok(ran) => ran_rows.push(BatchRowRan {
                    index: i as u32,
                    label,
                    ran,
                }),
                // The host's stop heard inside the row: the whole batch is taken back, not a row's failure.
                Err(Halt::Cancelled) => {
                    doc.cancel_group(group);
                    return Ok(BatchEnd::Cancelled);
                }
                Err(Halt::Refused(e)) if input.on_error == BatchOnError::Continue => {
                    if let Some(at) = at {
                        doc.rollback_to(at);
                    }
                    failed.push(BatchRowFailed {
                        index: i as u32,
                        label,
                        code: e.code.clone(),
                        message: e.message.clone(),
                    });
                }
                Err(Halt::Refused(e)) => {
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
            &failed,
            revision,
            doc.settings().srid,
            clock,
        );
        let added: usize = ran_rows.iter().map(|r| r.ran.added.len()).sum();
        let changed: usize = ran_rows.iter().map(|r| r.ran.changed.len()).sum();
        let mut summary = format!(
            "{} satır çalıştı: {added} nesne eklendi, {changed} nesne değişti.",
            ran_rows.len()
        );
        let mut warnings = Vec::new();
        if !failed.is_empty() {
            let names: Vec<&str> = failed.iter().map(|f| f.label.as_str()).collect();
            summary.push_str(&format!(
                " {} satır çalışmadı ve geri alındı ({}).",
                failed.len(),
                names.join(", ")
            ));
            let rest = if ran_rows.is_empty() {
                "hiçbir satır yazılmadı"
            } else {
                "öteki satırlar yazıldı"
            };
            warnings.push(CommandWarning {
                code: codes::BATCH_ROWS_FAILED.into(),
                message: format!(
                    "{} satır çalışıp durdu ve tek tek geri alındı ({}); {rest}.",
                    failed.len(),
                    names.join(", ")
                ),
                path: Some("rows".into()),
            });
        }
        Ok(BatchEnd::Ran(
            ProcessingBatched {
                tool: input.tool.clone(),
                summary,
                rows: ran_rows,
                failed,
                step: edited.then(|| format!("Toplu: {}", tool.label)),
                record,
                revision: doc.revision().to_string(),
            },
            warnings,
        ))
    }

    fn network(
        &self,
        doc: &Document,
        id: &str,
    ) -> Result<
        (
            kentos_commands::NetworkDef,
            kentos_geometry_core::ops::network::NetworkSession,
        ),
        String,
    > {
        let built = crate::network::from_document(doc, id)?;
        Ok((built.def, built.session))
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
        drawing: input.drawing.clone(),
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
        failed: Vec::new(),
    }
}

/// A batch's record (docs/adr/0529 §2): its rows are the steps, the layers they read are its inputs.
fn batch_record(
    registry: &Registry,
    input: &ProcessingBatch,
    tool: &Tool,
    rows: &[BatchRowRan],
    failed: &[BatchRowFailed],
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
        failed: failed
            .iter()
            .map(|f| RecordFailure {
                id: format!("rows[{}]", f.index),
                label: f.label.clone(),
                code: f.code.clone(),
                message: f.message.clone(),
            })
            .collect(),
    }
}

/// Why a run did not end: refused (or stopped by the tool itself), or stopped by the host while it ran (docs/adr/0551 §4).
enum Halt {
    Refused(Box<CommandError>),
    Cancelled,
}

impl From<Box<CommandError>> for Halt {
    fn from(e: Box<CommandError>) -> Self {
        Halt::Refused(e)
    }
}

/// Runs a made-ready tool or model on `doc` (the drawing or a copy): what it did, with its record, and the objects it
/// made. `stop` is the host's stop, read by the tool as it goes: a tool that polls its feedback stops inside the run,
/// the drawing as it was.
fn run_on(
    doc: &mut Document,
    ready: Ready,
    registry: &Registry,
    digests: &mut Digests,
    stop: Option<Arc<AtomicBool>>,
) -> Result<(ProcessingRan, Vec<Slot>), Halt> {
    let begun = Begun::of(doc, &ready, digests);
    let Ready {
        tool,
        model,
        values,
        selection,
        view,
        folder,
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
    let files = crate::local_files::LocalFiles::of(host.doc, folder);
    let mut runner = Runner::with_files(std::sync::Arc::new(files));
    if let Some(stop) = stop {
        runner = runner.with_stop(stop);
    }
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
                sink_report: None,
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
            )
            .into())
        }
        // The host's stop, heard by the tool or between a model's steps: the drawing is as it was.
        Outcome::Stopped {
            status: Status::Canceled,
            ..
        } => Err(Halt::Cancelled),
        Outcome::Stopped { message, .. } => {
            Err(refused(codes::TOOL_FAILED, message, Some("tool".into())).into())
        }
    }
}

/// The content digests taken so far, by the drawing's change mark and the layer (docs/adr/0529 §6, 0551 §5): hashing a
/// big layer costs more than the run that reads it, and a change mark is one state of the objects (it moves with every
/// object an edit, an undo or a taken-back row touches; the generation does not move inside a command's open step).
type Digests = BTreeMap<(ChangeMark, String), String>;

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
                    .entry((doc.change_mark(), layer.clone()))
                    .or_insert_with(|| {
                        kentos_native_application::sources::layer_digest(doc, &layer)
                    })
                    .clone(),
                layer,
            }
        })
        .collect()
}

/// Where a sink writes, and how: an absolute `.gpkg` path in a folder that is there; for a new file one that is there
/// only with `overwrite`; for `addTables` and `replaceTables` a file that is there (docs/adr/0549 §1).
struct Target {
    path: PathBuf,
    mode: SinkMode,
}

fn sink_target(sink: &RunSink) -> Result<Target, Box<CommandError>> {
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
    match sink.mode {
        SinkMode::New => {
            if exists && !sink.overwrite {
                return Err(refused(
                    codes::FILE_EXISTS,
                    format!("{path} var; üzerine yazmak için overwrite: true verin."),
                    at,
                ));
            }
        }
        SinkMode::AddTables | SinkMode::ReplaceTables => {
            if sink.overwrite {
                return Err(refused(
                    codes::INVALID_INPUT,
                    "overwrite yeni dosyanın üzerine yazmak içindir; addTables ve replaceTables var olan dosyaya dokunmadan tablo ekler ya da yenisiyle değiştirir."
                        .into(),
                    Some("sink.overwrite".to_owned()),
                ));
            }
            if !exists {
                return Err(refused(
                    codes::FILE_NOT_FOUND,
                    format!(
                        "{path} yok; tablo eklenecek GeoPackage'ın yolunu verin ya da yeni bir dosya için sink.mode vermeyin."
                    ),
                    at,
                ));
            }
        }
    }
    Ok(Target {
        path: p.to_path_buf(),
        mode: sink.mode,
    })
}

/// The tool's outputs that are tables (`{ columns, rows }`, every cell the text the tool shows), as text tables to write
/// (docs/adr/0551 §3), each named by its output's label (the name of the output when it has none).
fn text_tables(
    outputs: &BTreeMap<String, Value>,
    labels: &BTreeMap<String, String>,
) -> Vec<TextTable> {
    let text = |v: &Value| match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    outputs
        .iter()
        .filter_map(|(name, v)| {
            let columns: Vec<String> = v
                .get("columns")?
                .as_array()?
                .iter()
                .map(text)
                .collect();
            let rows: Vec<Vec<String>> = v
                .get("rows")?
                .as_array()?
                .iter()
                .map(|r| r.as_array().map(|r| r.iter().map(text).collect()))
                .collect::<Option<_>>()?;
            Some(TextTable {
                output: name.clone(),
                label: labels
                    .get(name)
                    .filter(|l| !l.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| name.clone()),
                columns,
                rows,
            })
        })
        .collect()
}

/// What a result came from (docs/adr/0511): the tool, the request as given, the layers it read, the drawing's revision.
struct Lineage<'a> {
    tool: (&'a str, &'a str),
    request: &'a ProcessingRun,
    inputs: &'a [RunInput],
    drawing: String,
    /// The run's own record (docs/adr/0551 §6): what it was made of and how it went, kept with each table it made.
    run: &'a ProcessingRecord,
}

/// The URI of KentOS's lineage document in `gpkg_metadata` (docs/adr/0511).
pub const LINEAGE_STANDARD: &str = "https://kentos.app/lineage/1";

impl Lineage<'_> {
    /// The table's lineage document: `kentos.lineage` 1, as JSON.
    fn document(&self, table: &str, layer: Option<&str>, output: Option<&str>, made: &str) -> String {
        let mut request = serde_json::to_value(self.request).unwrap_or(Value::Null);
        if let Some(o) = request.as_object_mut() {
            o.remove("sink");
        }
        json!({
            "format": "kentos.lineage",
            "version": 1,
            "table": table,
            "layer": layer,
            "output": output,
            "tool": { "id": self.tool.0, "label": self.tool.1 },
            "request": request,
            "inputs": self.inputs,
            "drawing": { "revision": self.drawing },
            "run": self.run,
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

/// How a sink's write ended.
enum Sunk {
    /// Written (or, for a plan, measured on a passing copy): what the file holds now, how it was made, what was lost.
    Done(Box<(GpkgExported, SinkReport, Vec<CommandWarning>)>),
    /// Stopped between two batches: the file is as it was.
    Cancelled,
}

fn unwritable(why: String) -> Box<CommandError> {
    refused(
        codes::FILE_UNWRITABLE,
        format!("GeoPackage yazılamadı: {why}. Eski dosya olduğu gibi kaldı."),
        Some("sink.path".to_owned()),
    )
}

/// The passing file a write is made in beside the target.
fn passing_file(path: &Path) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    path.with_extension(format!(
        "gpkg.kentos-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// The objects a run made on `copy` written to the sink's GeoPackage, each layer they are on a table; the other
/// objects of the copy are left out first (docs/adr/0508 §4, 0549).
///
/// - A new file is made beside the target under a passing name and, with `write`, put in its place in one step.
/// - Into a file that is there (`addTables`, `replaceTables`) the tables go in one SQLite transaction on the file
///   itself: nothing of it is there until it commits, and a refusal, a failure or a stop rolls it back. A plan does
///   the same on a passing copy of the file, and removes the copy.
/// - The rows go in batches of [`BATCH_ROWS`]; `flow` is asked before each.
fn write_sink(
    copy: &mut Document,
    added: &[Slot],
    texts: &[TextTable],
    target: &Target,
    write: bool,
    lineage: &Lineage<'_>,
    flow: &dyn BatchFlow,
) -> Result<Sunk, Box<CommandError>> {
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
    if layers.is_empty() && texts.is_empty() {
        return Err(refused(
            codes::NOTHING_TO_EXPORT,
            "Araç yeni nesne yapmadı; dosyaya yazılacak sonuç yok (değiştiren ya da seçen araçların sonucu çizimdedir; \
             aracın tablo çıktısını yazmak için sink.tables verin)."
                .into(),
            at,
        ));
    }
    let path = &target.path;
    let fresh = target.mode == SinkMode::New;
    // Where the tables are written: a new file in a passing one, a file that is there in itself (a plan: in a copy).
    let (work, in_place) = if fresh || !write {
        (passing_file(path), false)
    } else {
        (path.clone(), true)
    };
    if !in_place {
        let _ = std::fs::remove_file(&work);
    }
    if !fresh && !write {
        std::fs::copy(path, &work).map_err(|e| unwritable(e.to_string()))?;
    }
    let outcome = write_tables(copy, &layers, texts, &work, target.mode, lineage, flow);
    let finished = match outcome {
        Ok(f) => f,
        Err(e) => {
            if !in_place {
                let _ = std::fs::remove_file(&work);
            }
            return match e {
                Stop::Cancelled => Ok(Sunk::Cancelled),
                Stop::Refused(why) => Err(why),
            };
        }
    };
    if finished.written.tables.is_empty() {
        if !in_place {
            let _ = std::fs::remove_file(&work);
        }
        // Nothing was committed (the transaction found nothing to write), so the file is as it was.
        return Err(refused(
            codes::NOTHING_TO_EXPORT,
            "Aracın yaptığı nesnelerin GeoPackage'da geometrisi yok (yazı, ölçü, blok, tarama); dosyaya yazılacak sonuç yok."
                .into(),
            at,
        ));
    }
    let measured = std::fs::File::open(&work)
        .and_then(|f| kentos_commands::encoding::sha256_reader_hex(std::io::BufReader::new(f)));
    let (bytes, sha256) = match measured {
        Ok(m) => m,
        Err(e) => {
            if !in_place {
                let _ = std::fs::remove_file(&work);
            }
            return Err(unwritable(e.to_string()));
        }
    };
    if fresh && write {
        if let Err(e) = std::fs::rename(&work, path) {
            let _ = std::fs::remove_file(&work);
            return Err(unwritable(e.to_string()));
        }
    } else if !in_place {
        let _ = std::fs::remove_file(&work);
    }
    let written = finished.written;
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
    let dimension = |flag: u8| match flag {
        0 => SinkDimension::None,
        1 => SinkDimension::All,
        _ => SinkDimension::Some,
    };
    let report = SinkReport {
        mode: target.mode,
        tables: written
            .tables
            .iter()
            .map(|t| SinkTable {
                table: t.table.clone(),
                layer: t.layer.clone(),
                output: t.output.clone(),
                rows: t.rows,
                geometry: t.geometry.clone(),
                z: dimension(t.z),
                m: dimension(t.m),
                replaced: t.replaced,
                columns: t
                    .columns
                    .iter()
                    .map(|(a, c)| SinkColumn {
                        attribute: a.clone(),
                        column: c.clone(),
                    })
                    .collect(),
            })
            .collect(),
        untouched: finished.untouched,
        batches: written.batches,
        batch_rows: BATCH_ROWS as u32,
        ids: kentos_geopackage::UIDS_EXTENSION.to_owned(),
    };
    let exported = GpkgExported {
        path: path.display().to_string(),
        bytes,
        sha256,
        tables: written
            .tables
            .into_iter()
            .map(|t| GpkgTableExported {
                table: t.table,
                layer: t.layer.or(t.output).unwrap_or_default(),
                rows: t.rows,
                geometry: t.geometry,
            })
            .collect(),
        left: written.left,
        approximated: written.approximated,
    };
    Ok(Sunk::Done(Box::new((exported, report, warnings))))
}

/// Why the tables were not written.
enum Stop {
    Cancelled,
    Refused(Box<CommandError>),
}

/// What a committed write holds.
struct Finished {
    written: kentos_native_application::geopackage::Written,
    /// The tables the file had before that the write left alone.
    untouched: Vec<String>,
}

/// The tables written in `work` in one transaction: committed when every batch is in, rolled back (the session
/// dropped) when anything stops it.
fn write_tables(
    copy: &Document,
    layers: &[String],
    texts: &[TextTable],
    work: &Path,
    mode: SinkMode,
    lineage: &Lineage<'_>,
    flow: &dyn BatchFlow,
) -> Result<Finished, Stop> {
    use kentos_native_application::geopackage::{Clash, Flow, WriteStop, write_with_tables};
    let path_text = work.display().to_string();
    let fresh = mode == SinkMode::New;
    let mut gp = if fresh {
        kentos_geopackage::GeoPackage::create(work).map_err(|e| Stop::Refused(unwritable(e.0)))?
    } else {
        kentos_geopackage::GeoPackage::open(work, true).map_err(|e| {
            Stop::Refused(refused(
                codes::IMPORT_UNREADABLE,
                format!("{path_text} GeoPackage olarak açılamadı: {}. Dosyaya dokunulmadı.", e.0),
                Some("sink.path".to_owned()),
            ))
        })?
    };
    let before: Vec<String> = if fresh {
        Vec::new()
    } else {
        gp.layers()
            .map_err(|e| Stop::Refused(unwritable(e.0)))?
            .into_iter()
            .map(|l| l.name)
            .collect()
    };
    let session = gp
        .session()
        .map_err(|e| Stop::Refused(unwritable(e.0)))?;
    let stopped = || flow.stopped();
    let progress = |done: u64, total: u64| flow.progress(done, total);
    let clash = match mode {
        SinkMode::New => None,
        SinkMode::AddTables => Some(Clash::Refuse),
        SinkMode::ReplaceTables => Some(Clash::Replace),
    };
    let written = write_with_tables(
        copy,
        layers,
        texts,
        &session,
        clash,
        BATCH_ROWS,
        &Flow {
            stopped: &stopped,
            progress: &progress,
        },
    )
    .map_err(|stop| match stop {
        WriteStop::Cancelled => Stop::Cancelled,
        WriteStop::Taken { table, replaceable } => Stop::Refused(refused(
            codes::TABLE_EXISTS,
            if replaceable {
                format!(
                    "“{table}” tablosu {path_text} dosyasında var (tablonun adı katmanın ya da aracın tablo çıktısının adıdır); üzerine yazmak için sink.mode: replaceTables verin ya da başka bir dosyaya yazın. Dosyaya dokunulmadı."
                )
            } else {
                format!(
                    "“{table}” adı {path_text} dosyasında bir nesne ya da öznitelik tablosu değil (başka bir tür tablo ya da bir eklentinin tablosu); aynı adlı tablo değiştirilmez. Dosyaya dokunulmadı."
                )
            },
            Some("sink.path".to_owned()),
        )),
        WriteStop::Srs(why) => Stop::Refused(refused(codes::CRS_MISMATCH, why, Some("sink.path".to_owned()))),
        WriteStop::Failed(why) => Stop::Refused(unwritable(why)),
    })?;
    if written.tables.is_empty() {
        // Dropping the session rolls back what little it did (the spatial reference system).
        drop(session);
        return Ok(Finished {
            written,
            untouched: Vec::new(),
        });
    }
    // Each table says what it came from (docs/adr/0511): GeoPackage's metadata extension.
    let made = utc_now();
    for t in &written.tables {
        session
            .add_metadata(
                &t.table,
                LINEAGE_STANDARD,
                "application/json",
                &lineage.document(&t.table, t.layer.as_deref(), t.output.as_deref(), &made),
                &made,
            )
            .map_err(|e| Stop::Refused(unwritable(e.0)))?;
    }
    if flow.stopped() {
        return Err(Stop::Cancelled);
    }
    session
        .commit()
        .map_err(|e| Stop::Refused(unwritable(e.0)))?;
    gp.close().map_err(|e| Stop::Refused(unwritable(e.0)))?;
    let replaced: Vec<String> = written
        .tables
        .iter()
        .filter(|t| t.replaced)
        .map(|t| t.table.to_ascii_lowercase())
        .collect();
    let untouched = before
        .into_iter()
        .filter(|n| !replaced.contains(&n.to_ascii_lowercase()))
        .collect();
    Ok(Finished { written, untouched })
}
