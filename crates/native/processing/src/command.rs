//! İşlemler's tools for the processing commands (docs/adr/0508, plan/gis.md CBS-41): the application's
//! `cad.processing.tools` and `.run` reach the tools through [`register`]. A run is the window's run without a
//! screen: the values are the input's (a features input names its objects by persistent ids, or takes the input's
//! explicit `selection` and `view`), the drawing's defaults fill the rest, the runner checks and writes them as the
//! window does, one undo step named by the tool. With a sink (docs/adr/0508 §4) the run is made on a copy and the
//! objects it made go to a new GeoPackage; the drawing does not change.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use kentos_contracts::{
    CommandError, CommandWarning, GpkgExported, GpkgTableExported, ProcessingRan, ProcessingRun,
    ProcessingTool, ProcessingTools, ProcessingToolsListed, RunInput, RunSink, ToolNote,
    ToolParameter, codes,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::geometry::Bounds;
use kentos_native_application::processing_host::{ToolHost, register_tools};
use serde_json::{Value, json};

use crate::features::{Host, Scene};
use crate::parameters::default_values;
use crate::registry::Registry;
use crate::runner::{Level, Outcome, Runner};
use crate::text::fold_turkish;
use crate::types::{Defaults, OutputKind, ParamKind, Tool, Values};

/// Registers İşlemler's built-in tools as the processing commands' (the first registration stays).
pub fn register() {
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
    let Some(tool) = registry.tool(&input.tool) else {
        return Err(refused(
            codes::UNKNOWN_TOOL,
            format!(
                "“{}” bir işlem aracı değil. Araçların listesi cad.processing.tools'ta.",
                input.tool
            ),
            Some("tool".into()),
        ));
    };
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

fn node_ids(nodes: &[kentos_contracts::LayerNode], out: &mut BTreeSet<String>) {
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
        Ok(ProcessingToolsListed {
            tools: found
                .into_iter()
                .map(|t| described(&registry, t, doc))
                .collect(),
        })
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
        let inputs = inputs_of(doc, &ready);
        let Some(sink) = &input.sink else {
            let (mut ran, _) = run_on(doc, ready)?;
            ran.inputs = inputs;
            return Ok((ran, Vec::new()));
        };
        let target = sink_target(sink)?;
        // The run is made on a copy: the drawing does not change, the file takes the result.
        let mut copy = doc.reading_copy();
        let (mut ran, added) = run_on(&mut copy, ready)?;
        let (exported, warnings) = write_sink(&mut copy, &added, &target, write)?;
        ran.inputs = inputs;
        ran.sink = Some(exported);
        ran.step = None;
        ran.revision = doc.revision().to_string();
        Ok((ran, warnings))
    }
}

/// Runs a made-ready tool on `doc` (the drawing or a copy): what it did and the objects it made.
fn run_on(
    doc: &mut Document,
    ready: Ready,
) -> Result<(ProcessingRan, Vec<Slot>), Box<CommandError>> {
    let Ready {
        tool,
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
    let mut runner = Runner::new();
    let outcome = runner.run(&mut host, &tool, &values, true, &mut log);
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
            let kinds: BTreeMap<&str, OutputKind> = tool
                .outputs
                .iter()
                .map(|o| (o.name.as_str(), o.kind))
                .collect();
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
            let ran = ProcessingRan {
                tool: tool.id.clone(),
                summary: record.summary,
                added: uids(doc, &added),
                changed: uids(doc, &changed),
                selected: chosen.map(|c| uids(doc, &c)).unwrap_or_default(),
                layers,
                outputs,
                notes,
                inputs: Vec::new(),
                sink: None,
                step: edited.then(|| tool.label.clone()),
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

/// The layers the features inputs read, in the inputs' order: a layer named, or the layers of the objects named or
/// chosen; a bound layer with its source and the revision it read. The visible layers (`all`) and the view are not
/// listed one by one.
fn inputs_of(doc: &Document, ready: &Ready) -> Vec<RunInput> {
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

/// The objects a run made on `copy` written to the sink's GeoPackage, each layer they are on a table; the other
/// objects of the copy are left out first. The file is made beside the target under a passing name and, with
/// `write`, put in its place in one step; a plan removes it. What it holds, and what it could not hold.
fn write_sink(
    copy: &mut Document,
    added: &[Slot],
    (path, _): &(PathBuf, bool),
    write: bool,
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
    let written = kentos_native_application::geopackage::write(copy, &layers, &tmp);
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
        sha256: kentos_contracts::encoding::sha256_hex(&bytes),
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
