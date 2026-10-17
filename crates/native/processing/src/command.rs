//! İşlemler's tools for the processing commands (docs/adr/0508, plan/gis.md CBS-41): the application's
//! `cad.processing.tools` and `.run` reach the tools through [`register`]. A run is the window's run without a
//! screen: the values are the input's (a features input names its objects by persistent ids, or takes the input's
//! explicit `selection` and `view`), the drawing's defaults fill the rest, the runner checks and writes them as the
//! window does, one undo step named by the tool.

use std::collections::{BTreeMap, BTreeSet};

use kentos_contracts::{
    CommandError, ProcessingRan, ProcessingRun, ProcessingTool, ProcessingTools,
    ProcessingToolsListed, ToolNote, ToolParameter, codes,
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
    ) -> Result<ProcessingRan, Box<CommandError>> {
        let registry = Registry::builtin();
        let Ready {
            tool,
            values,
            selection,
            view,
        } = ready(&registry, doc, input)?;
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
                Ok(ProcessingRan {
                    tool: tool.id.clone(),
                    summary: record.summary,
                    added: uids(doc, &added),
                    changed: uids(doc, &changed),
                    selected: chosen.map(|c| uids(doc, &c)).unwrap_or_default(),
                    layers,
                    outputs,
                    notes,
                    step: edited.then(|| tool.label.clone()),
                    revision: doc.revision().to_string(),
                })
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
}
