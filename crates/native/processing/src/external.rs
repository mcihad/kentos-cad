//! Tools an extension adds to İşlemler (plan/eklenti.md ADIM 19; docs/adr/8057 §7): declared as data in the
//! extension's `tools/<id>.tool.json` (`kentos.processingTool` v1), their work done by its Python in a process of its
//! own.
//!
//! - **One contract:** a descriptor is read into the same [`Tool`] the built-in tools are, its parameters in İşlemler'
//!   own JSON (the web's `ParamDef`, as models keep theirs): the window, the command line, a model, a batch, Python and
//!   MCP list and run it under one id, `ext.<extension>.<tool>`, with one schema and one result (PY-TOOL-04, 21). The
//!   descriptor is read without the extension's code (PY-TOOL-02).
//! - **Its work elsewhere:** the host registers a [`Provider`] that runs the extension's code; this crate starts no
//!   process. The inputs go as data, as they were when the run began: each object with its persistent id, never a slot
//!   (PY-TOOL-05), a field the drawing hides from third parties cut as an agent reads it (docs/adr/0738); the drawing
//!   is not held while the code computes (PY-TOOL-09).
//! - **Typed outputs, checked whole:** what comes back is read against the descriptor: new objects for an output layer
//!   (a shape of their own or an input's), rows for a table, numbers and texts. Anything else, or a value that does not
//!   fit, refuses the run with nothing written (PY-TOOL-10); the runner writes the rest in one undo step (PY-TOOL-11,
//!   17), the layer fields checking every value as a built-in tool's (`writes::check`).
//! - **Versioned:** the tools are replaced as extensions are installed, turned on or off and removed; each change
//!   bumps [`generation`], what a list that was read compares to know it is stale.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

use kentos_commands::Entity;
use kentos_native_application::python_call::{HIDDEN, redacted_names};
use serde_json::{Map, Value, json};

use crate::parameters::is_visible;
use crate::text::js_number;
use crate::types::{
    ChangeSet, Feedback, OutputDef, OutputKind, ParamDef, ParamKind, Resolved, RunContext,
    RunResult, Target, Tool,
};
use crate::web_param::param_from_json;

/// The category extensions' tools are listed under.
pub const CATEGORY: &str = "extensions";

/// An extension's tool ids begin so: `ext.<extension>.<tool>`.
pub const PREFIX: &str = "ext.";

/// The most objects a run may write, rows it may give and characters an attribute value may hold: what a run gives
/// back is checked whole before anything is written, and a runaway answer is refused rather than written in part.
pub const MAX_OBJECTS: usize = 500_000;
pub const MAX_ROWS: usize = 100_000;
pub const MAX_TEXT: usize = 10_000;

/// The parameter kinds an extension's tool may declare (PY-TOOL-01): what its Python gets as data. A layer
/// parameter is the host's: where the tool's new objects go.
const KINDS: [&str; 8] = [
    "features", "number", "string", "boolean", "enum", "field", "point", "layer",
];

/// Python's words a parameter cannot be named, since the tool's function takes its parameters by their names.
const KEYWORDS: [&str; 36] = [
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import",
    "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while",
    "with", "yield", "ctx",
];

/// An output of an extension's tool: its name, kind and, for new objects, the layer parameter they go to.
#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    pub name: String,
    pub kind: OutputKind,
    pub layer: Option<String>,
}

/// An extension's tool: İşlemler' [`Tool`], and what running it needs.
#[derive(Clone)]
pub struct ExternalTool {
    pub tool: Tool,
    pub extension: String,
    /// Its id in the extension's manifest.
    pub local: String,
    pub outputs: Vec<Output>,
    /// How long a run may take before it is stopped as overdue, seconds.
    pub timeout: u64,
}

/// Why a run gave nothing.
#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    /// The user stopped it (Durdur): the runner ends it as cancelled.
    Stopped,
    /// It failed, or could not start: why, in a sentence the run ends with.
    Failed(String),
}

/// What runs an extension's code: the host's (the desktop's extension processes).
pub trait Provider: Send + Sync {
    /// Runs `tool` on `input` (`{ params, inputs }`), asking `feedback` whether the user stopped it and telling it how
    /// far it is and what the code says; the code's answer (`{ kind: "outputs", outputs, summary?, select? }`).
    fn run(
        &self,
        tool: &ExternalTool,
        input: Value,
        feedback: &mut dyn Feedback,
    ) -> Result<Value, Failure>;
}

struct Shelf {
    generation: u64,
    tools: Vec<ExternalTool>,
}

static SHELF: RwLock<Shelf> = RwLock::new(Shelf {
    generation: 0,
    tools: Vec::new(),
});

static PROVIDER: RwLock<Option<Arc<dyn Provider>>> = RwLock::new(None);

/// The extensions' expression functions (`ext.<eklenti>.<işlev>`), as İşlemler' expressions may call them.
static EXPRESSIONS: RwLock<Vec<kentos_expression::external::ExternalDef>> = RwLock::new(Vec::new());

/// What answers them: the host's (the desktop's extension processes).
pub type ExpressionCalls = Arc<dyn kentos_expression::external::ExternalCalls + Send + Sync>;

static EXPRESSION_CALLS: RwLock<Option<ExpressionCalls>> = RwLock::new(None);

/// The extensions' expression functions now, replaced whole as extensions change.
pub fn set_expressions(defs: Vec<kentos_expression::external::ExternalDef>) {
    if let Ok(mut slot) = EXPRESSIONS.write() {
        *slot = defs;
    }
}

/// The extensions' expression functions an İşlemler expression may call (the schema's `externals`).
pub fn expressions() -> Vec<kentos_expression::external::ExternalDef> {
    EXPRESSIONS.read().map(|d| d.clone()).unwrap_or_default()
}

/// Registers what answers the extensions' expression functions; a later registration replaces it.
pub fn register_expression_calls(calls: ExpressionCalls) {
    if let Ok(mut slot) = EXPRESSION_CALLS.write() {
        *slot = Some(calls);
    }
}

/// What answers them, where the host registered it.
pub fn expression_calls() -> Option<ExpressionCalls> {
    EXPRESSION_CALLS.read().ok().and_then(|c| c.clone())
}

/// Registers what runs the extensions' code; a later registration replaces it.
pub fn register_provider(provider: Arc<dyn Provider>) {
    if let Ok(mut slot) = PROVIDER.write() {
        *slot = Some(provider);
    }
}

fn provider() -> Option<Arc<dyn Provider>> {
    PROVIDER.read().ok().and_then(|p| p.clone())
}

/// The extensions' tools now: replaced whole as extensions change (installed, turned on or off, removed); the
/// generation goes up when the list differs.
pub fn set_tools(tools: Vec<ExternalTool>) {
    if let Ok(mut shelf) = SHELF.write() {
        let same = shelf.tools.len() == tools.len()
            && shelf
                .tools
                .iter()
                .zip(&tools)
                .all(|(a, b)| a.tool.id == b.tool.id && a.tool.label == b.tool.label);
        shelf.tools = tools;
        if !same {
            shelf.generation += 1;
        }
    }
}

/// The extensions' tools as İşlemler lists them.
pub fn tools() -> Vec<Tool> {
    SHELF
        .read()
        .map(|s| s.tools.iter().map(|t| t.tool.clone()).collect())
        .unwrap_or_default()
}

/// How many times the extensions' tools changed: a list read at an older generation is stale.
pub fn generation() -> u64 {
    SHELF.read().map_or(0, |s| s.generation)
}

/// An extension's tool by its id.
pub fn find(id: &str) -> Option<ExternalTool> {
    SHELF
        .read()
        .ok()?
        .tools
        .iter()
        .find(|t| t.tool.id == id)
        .cloned()
}

/// The extension and local id of `ext.<extension>.<tool>`: the local id is the last part (a manifest's local ids
/// have no dot).
pub fn split(id: &str) -> Option<(&str, &str)> {
    id.strip_prefix(PREFIX)?.rsplit_once('.')
}

fn text<'a>(o: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    o.get(key).and_then(Value::as_str)
}

fn is_python_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !KEYWORDS.contains(&name)
}

/// An extension's tool from its descriptor (`kentos.processingTool` v1), named `ext.<extension>.<local>`; why not,
/// in a sentence naming the place, when it is not one. `extension_name` and `version` go into its help: the user sees
/// whose code a tool runs, and that a separate process is not a sandbox (PY-RUN-19, D6).
pub fn from_descriptor(
    extension: &str,
    extension_name: &str,
    version: &str,
    local: &str,
    descriptor: &Value,
) -> Result<ExternalTool, String> {
    let o = descriptor
        .as_object()
        .ok_or("Araç tanımı bir JSON nesnesi değil.")?;
    if text(o, "format") != Some("kentos.processingTool")
        || o.get("schemaVersion").and_then(Value::as_u64) != Some(1)
    {
        return Err(
            "Araç tanımı “kentos.processingTool” sürüm 1 değil (format, schemaVersion).".into(),
        );
    }
    if text(o, "id") != Some(local) {
        return Err(format!(
            "Araç tanımının kimliği (id) manifestteki “{local}” ile aynı olmalı."
        ));
    }
    let label = text(o, "label").map(str::trim).unwrap_or_default();
    if label.is_empty() || label.chars().count() > 80 {
        return Err("label: aracın adı boş olmamalı, en çok 80 karakter.".into());
    }
    if let Some(runtimes) = runtimes(o)
        && !runtimes.iter().any(|r| r == "desktop-python")
    {
        return Err(
            "capabilities.runtimes: bu KentOS eklenti araçlarını yalnız masaüstünde, eklentinin Python'unda çalıştırır \
             (“desktop-python”)."
                .into(),
        );
    }
    let mut parameters: Vec<ParamDef> = Vec::new();
    for (i, p) in o
        .get("parameters")
        .and_then(Value::as_array)
        .ok_or("parameters: parametre listesi yok.")?
        .iter()
        .enumerate()
    {
        let at = format!("parameters[{i}]");
        let kind = p.get("type").and_then(Value::as_str).unwrap_or_default();
        if !KINDS.contains(&kind) {
            return Err(format!(
                "{at}: “{kind}” türü eklenti araçlarında desteklenmiyor ({}).",
                KINDS.join(", ")
            ));
        }
        let def =
            param_from_json(p).ok_or_else(|| format!("{at}: parametre okunamadı (name, type)."))?;
        if !is_python_name(&def.name) {
            return Err(format!(
                "{at}: “{}” Python'da bir ad olamaz: harfle ya da _ ile başlayan, harf, rakam ve _ içeren bir ad \
                 olmalı (ctx ve Python'un kendi sözcükleri olmaz).",
                def.name
            ));
        }
        if parameters.iter().any(|q| q.name == def.name) {
            return Err(format!("{at}: “{}” adlı iki parametre var.", def.name));
        }
        parameters.push(def);
    }
    // What a parameter names must be one of them: a field's source, a condition's parameter.
    let names: BTreeSet<&str> = parameters.iter().map(|p| p.name.as_str()).collect();
    for p in &parameters {
        if let ParamKind::Field { of, .. } = &p.kind
            && let Some(missing) = of.iter().find(|n| !names.contains(n.as_str()))
        {
            return Err(format!(
                "“{}”: alanların okunacağı “{missing}” parametresi yok.",
                p.name
            ));
        }
        if let Some(crate::types::ShownWhen::Equals { param, .. }) = &p.visible_when
            && !names.contains(param.as_str())
        {
            return Err(format!(
                "“{}”: visibleWhen'in baktığı “{param}” parametresi yok.",
                p.name
            ));
        }
    }
    let mut outputs: Vec<Output> = Vec::new();
    let mut output_defs: Vec<OutputDef> = Vec::new();
    for (i, out) in o
        .get("outputs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let at = format!("outputs[{i}]");
        let oo = out
            .as_object()
            .ok_or_else(|| format!("{at}: bir nesne değil."))?;
        let name = text(oo, "name").unwrap_or_default();
        if name.is_empty() || outputs.iter().any(|x| x.name == name) {
            return Err(format!("{at}: çıktının adı boş ya da tekrarlı."));
        }
        let kind = match text(oo, "type").unwrap_or_default() {
            "features" => OutputKind::Features,
            "table" => OutputKind::Table,
            "number" => OutputKind::Number,
            "string" => OutputKind::Text,
            other => {
                return Err(format!(
                    "{at}: “{other}” çıktı türü desteklenmiyor (features, table, number, string)."
                ));
            }
        };
        let layer = text(oo, "sinkParameter").map(str::to_owned);
        if kind == OutputKind::Features {
            let Some(param) = layer
                .as_deref()
                .and_then(|n| parameters.iter().find(|p| p.name == n))
            else {
                return Err(format!(
                    "{at}: yeni nesneler bir katman parametresine yazılır; sinkParameter onu adlandırmalı."
                ));
            };
            if !matches!(param.kind, ParamKind::Layer { .. }) {
                return Err(format!(
                    "{at}: sinkParameter “{}” bir katman (layer) parametresi değil.",
                    param.name
                ));
            }
        }
        output_defs.push(OutputDef::new(
            name,
            text(oo, "label").unwrap_or(name),
            kind,
        ));
        outputs.push(Output {
            name: name.to_owned(),
            kind,
            layer,
        });
    }
    let description = text(o, "description").unwrap_or_default().trim().to_owned();
    let own_help = text(o, "help").map(str::trim).filter(|h| !h.is_empty());
    let whose = format!(
        "{extension_name} eklentisinin aracı ({extension} {version}). Eklentinin kendi Python ortamında, ayrı bir \
         süreçte çalışır; ayrı süreç bir korumalı alan değildir: eklenti sizin haklarınızla çalışır, KentOS yalnız \
         çizimden ne verdiğini sınırlar. Girdiler çalıştırma anındaki hâlleriyle veri olarak gider; çıktılar \
         denetlenip tek geri alma adımında yazılır."
    );
    let mut keywords: Vec<String> = o
        .get("keywords")
        .and_then(Value::as_array)
        .map(|k| {
            k.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    keywords.push(extension_name.to_owned());
    keywords.push("eklenti".to_owned());
    let timeout = o
        .get("timeoutSeconds")
        .and_then(Value::as_u64)
        .unwrap_or(600)
        .clamp(1, 3600);
    Ok(ExternalTool {
        tool: Tool {
            id: format!("{PREFIX}{extension}.{local}"),
            label: label.to_owned(),
            category: CATEGORY.to_owned(),
            description,
            help: Some(match own_help {
                Some(h) => format!("{h}\n\n{whose}"),
                None => whose,
            }),
            keywords,
            aliases: Vec::new(),
            icon: Some("extensions".to_owned()),
            parameters,
            outputs: output_defs,
            // On a thread of its own, the window never waiting for the extension's process.
            targets: vec![Target::Worker],
            validate: None,
            preview: None,
            run: Some(run),
        },
        extension: extension.to_owned(),
        local: local.to_owned(),
        outputs,
        timeout,
    })
}

/// The runtimes a descriptor names (`capabilities.runtimes`); none when it names none.
fn runtimes(o: &Map<String, Value>) -> Option<Vec<String>> {
    let list = o.get("capabilities")?.get("runtimes")?.as_array()?;
    Some(
        list.iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
    )
}

/// An extension's tool's run (its [`Tool::run`]): its inputs to the provider as data, its answer read back against
/// its descriptor.
pub fn run(
    resolved: &Resolved<'_>,
    ctx: &RunContext<'_>,
    feedback: &mut dyn Feedback,
) -> RunResult {
    let Some(tool) = find(ctx.tool) else {
        return RunResult::refused(format!(
            "“{}” artık kurulu ve açık bir eklentinin aracı değil.",
            ctx.tool
        ));
    };
    let Some(provider) = provider() else {
        return RunResult::refused(
            "Eklenti araçları bu ortamda çalışmaz: KentOS masaüstünde, eklentinin Python ortamında çalışırlar.".into(),
        );
    };
    let input = match input_of(&tool, resolved, ctx) {
        Ok(input) => input,
        Err(why) => return RunResult::refused(why),
    };
    match provider.run(&tool, input, feedback) {
        Ok(answer) => result_of(&tool, &answer, resolved, ctx).unwrap_or_else(RunResult::refused),
        // The runner ends it as cancelled: the feedback was asked to stop.
        Err(Failure::Stopped) => RunResult::default(),
        Err(Failure::Failed(why)) => RunResult::refused(why),
    }
}

/// An object as a tool gets it: its persistent id, its layer's name and the object, its slot zeroed (a slot is not
/// an id outside this program) and the drawing's hidden fields cut.
fn feature(e: &Entity, ctx: &RunContext<'_>, hidden: &[String]) -> Option<Value> {
    let uid = ctx.doc.uid(kentos_domain::Slot(e.base().id))?.to_string();
    let mut entity = serde_json::to_value(e).ok()?;
    if let Some(o) = entity.as_object_mut() {
        o.insert("id".into(), json!(0));
        if !hidden.is_empty()
            && let Some(Value::Object(attrs)) = o.get_mut("attrs")
        {
            for (k, v) in attrs.iter_mut() {
                if hidden.contains(k) {
                    *v = json!(HIDDEN);
                }
            }
        }
    }
    Some(json!({
        "uid": uid,
        "layer": ctx.layer_name(&e.base().layer_id),
        "geometry": geojson(e),
        "entity": entity,
    }))
}

/// How far a chord of a tessellated arc may come from its arc, metres: a millimetre, below what a cadastral plan
/// draws. A parcel's curve of chord 30 m and bulge 0.3 (r = 27.25 m) is cut into 69 chords and its area comes 0.02 m²
/// short of the arc's 991.60 m²; the drawing's own 5° tessellation (`DEFAULT_STEP`) came 0.50 m² short (991.10 m²,
/// measured in the real window, 14 November 2026).
pub const CHORD_TOLERANCE: f64 = 1e-3;

/// A path's vertices with its arcs as chords within [`CHORD_TOLERANCE`] (`ops::derive::path_chords`, the shared
/// core's); `closed`: on round to its first vertex.
fn flattened(pts: &[kentos_commands::Vec2], bulges: Option<&[f64]>, closed: bool) -> Vec<[f64; 2]> {
    let core: Vec<_> = pts.iter().map(|p| crate::geometry::core(*p)).collect();
    let chords =
        kentos_geometry_core::ops::derive::path_chords(&core, bulges, closed, CHORD_TOLERANCE);
    let mut out: Vec<[f64; 2]> = chords.pts.iter().map(|p| [p.x, p.y]).collect();
    if closed && out.len() > 1 && out.first() != out.last() {
        let first = out[0];
        out.push(first);
    }
    out
}

/// An area's rings as GeoJSON writes them: its outline, then its holes, each closed.
fn rings(
    pts: &[kentos_commands::Vec2],
    bulges: Option<&[f64]>,
    holes: Option<&Vec<kentos_commands::RingGeometry>>,
) -> Vec<Vec<[f64; 2]>> {
    let mut out = vec![flattened(pts, bulges, true)];
    for h in holes.into_iter().flatten() {
        out.push(flattened(&h.pts, h.bulges.as_deref(), true));
    }
    out
}

/// An object's shape as GeoJSON (RFC 7946's geometry, in the project's plane, metres), its arcs tessellated: what
/// Python's libraries read alike (`shapely.geometry.shape`, GeoPandas, `__geo_interface__`). A point, a line, an area
/// and their multi-part kinds, a circle as an area; none for a kind GeoJSON has no shape for.
pub fn geojson(e: &Entity) -> Value {
    match e {
        Entity::Point(p) => match p.parts.as_deref() {
            Some(parts) if !parts.is_empty() => json!({
                "type": "MultiPoint",
                "coordinates": std::iter::once([p.p.x, p.p.y])
                    .chain(parts.iter().map(|q| [q.p.x, q.p.y]))
                    .collect::<Vec<_>>(),
            }),
            _ => json!({ "type": "Point", "coordinates": [p.p.x, p.p.y] }),
        },
        Entity::Polyline(path) => {
            let first = flattened(&path.pts, path.bulges.as_deref(), path.closed);
            match path.parts.as_deref() {
                Some(parts) if !parts.is_empty() => json!({
                    "type": "MultiLineString",
                    "coordinates": std::iter::once(first)
                        .chain(parts.iter().map(|q| flattened(&q.pts, q.bulges.as_deref(), false)))
                        .collect::<Vec<_>>(),
                }),
                _ => json!({ "type": "LineString", "coordinates": first }),
            }
        }
        Entity::Polygon(path) => {
            let first = rings(&path.pts, path.bulges.as_deref(), path.holes.as_ref());
            match path.parts.as_deref() {
                Some(parts) if !parts.is_empty() => json!({
                    "type": "MultiPolygon",
                    "coordinates": std::iter::once(first)
                        .chain(parts.iter().map(|q| rings(&q.pts, q.bulges.as_deref(), q.holes.as_ref())))
                        .collect::<Vec<_>>(),
                }),
                _ => json!({ "type": "Polygon", "coordinates": first }),
            }
        }
        Entity::Circle(c) => {
            let chords = kentos_geometry_core::ops::derive::arc_chords(
                crate::geometry::core(c.c),
                c.r,
                0.0,
                std::f64::consts::TAU,
                CHORD_TOLERANCE,
            );
            let mut ring: Vec<[f64; 2]> = chords.pts.iter().map(|p| [p.x, p.y]).collect();
            if ring.len() > 1 && ring.first() != ring.last() {
                let first = ring[0];
                ring.push(first);
            }
            json!({ "type": "Polygon", "coordinates": [ring] })
        }
        _ => Value::Null,
    }
}

/// The tool's inputs: its parameters' values (`params`, every parameter but a layer one, a hidden one as none) and
/// its features (`inputs`), as data.
fn input_of(
    tool: &ExternalTool,
    resolved: &Resolved<'_>,
    ctx: &RunContext<'_>,
) -> Result<Value, String> {
    let hidden = redacted_names(ctx.doc);
    let mut params = Map::new();
    let mut inputs = Map::new();
    for p in &tool.tool.parameters {
        let shown = is_visible(p, &resolved.values);
        match &p.kind {
            ParamKind::Layer { .. } => {}
            ParamKind::Features { .. } => {
                let set = resolved.features(&p.name);
                let features: Vec<Value> = if shown {
                    set.entities
                        .iter()
                        .filter_map(|e| feature(e, ctx, &hidden))
                        .collect()
                } else {
                    Vec::new()
                };
                inputs.insert(
                    p.name.clone(),
                    json!({ "description": set.description, "features": features }),
                );
            }
            _ => {
                let value = if shown {
                    resolved.values.get(&p.name).cloned().unwrap_or(Value::Null)
                } else {
                    Value::Null
                };
                // A field the drawing hides is not named to a third party's code (docs/adr/0743 §1).
                if let (ParamKind::Field { .. }, Some(named)) = (&p.kind, value.as_str())
                    && named
                        .split(',')
                        .map(str::trim)
                        .any(|n| hidden.iter().any(|h| h == n))
                {
                    return Err(format!(
                        "“{}”: seçilen alan çizimde üçüncü taraflardan gizli; eklentinin kodu onu okuyamaz.",
                        p.label
                    ));
                }
                params.insert(p.name.clone(), value);
            }
        }
    }
    Ok(json!({ "params": params, "inputs": inputs }))
}

/// A value of an attribute as KentOS keeps it (text): a number as JavaScript writes it, a yes or no as `true` or
/// `false`; none for an empty one; why not for a list or an object.
fn attribute(key: &str, v: &Value) -> Result<Option<String>, String> {
    let text = match v {
        Value::Null => return Ok(None),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.as_f64().map(js_number).unwrap_or_default(),
        Value::Bool(b) => b.to_string(),
        Value::Array(_) | Value::Object(_) => {
            return Err(format!(
                "“{key}” özniteliğinin değeri tek bir değer olmalı (sayı, metin, evet/hayır), liste ya da sözlük değil."
            ));
        }
    };
    if text.chars().count() > MAX_TEXT {
        return Err(format!(
            "“{key}” özniteliğinin değeri {MAX_TEXT} karakterden uzun."
        ));
    }
    Ok(Some(text))
}

fn attributes(at: &str, v: Option<&Value>) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    match v {
        None | Some(Value::Null) => {}
        Some(Value::Object(attrs)) => {
            for (k, v) in attrs {
                let key = k.trim();
                if key.is_empty() || key.chars().count() > 64 {
                    return Err(format!(
                        "{at}: öznitelik adı boş ya da 64 karakterden uzun."
                    ));
                }
                if let Some(text) = attribute(key, v).map_err(|e| format!("{at}: {e}"))? {
                    out.insert(key.to_owned(), text);
                }
            }
        }
        Some(_) => return Err(format!("{at}: attrs bir sözlük olmalı.")),
    }
    Ok(out)
}

/// A GeoJSON position as KentOS's point: its first two numbers (an elevation after them is left), finite.
fn position(at: &str, v: &Value) -> Result<Value, String> {
    let xy = v
        .as_array()
        .filter(|p| p.len() >= 2)
        .and_then(|p| Some((p[0].as_f64()?, p[1].as_f64()?)));
    match xy {
        Some((x, y)) if x.is_finite() && y.is_finite() && x.abs() < 1e10 && y.abs() < 1e10 => {
            Ok(json!({ "x": x, "y": y }))
        }
        _ => Err(format!(
            "{at}: bir konum iki sonlu sayı olmalı ([x, y], metre)."
        )),
    }
}

/// A line's or a ring's positions; a ring's closing position (its first again, as GeoJSON writes it) left out.
fn positions(at: &str, v: &Value, ring: bool) -> Result<Vec<Value>, String> {
    let list = v
        .as_array()
        .ok_or_else(|| format!("{at}: bir konum listesi olmalı."))?;
    let mut out = list
        .iter()
        .enumerate()
        .map(|(i, p)| position(&format!("{at}[{i}]"), p))
        .collect::<Result<Vec<_>, _>>()?;
    if ring && out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    let least = if ring { 3 } else { 2 };
    if out.len() < least {
        return Err(format!(
            "{at}: bir {} en az {least} köşe ister.",
            if ring { "alan halkası" } else { "çizgi" }
        ));
    }
    Ok(out)
}

/// A GeoJSON polygon's rings as KentOS's: its outline's points and its holes.
fn area(at: &str, v: &Value) -> Result<(Vec<Value>, Vec<Value>), String> {
    let rings = v
        .as_array()
        .filter(|r| !r.is_empty())
        .ok_or_else(|| format!("{at}: bir alanın en az bir halkası olmalı (boş şekil)."))?;
    let outline = positions(&format!("{at}[0]"), &rings[0], true)?;
    let holes = rings[1..]
        .iter()
        .enumerate()
        .map(|(i, r)| Ok(json!({ "pts": positions(&format!("{at}[{}]", i + 1), r, true)? })))
        .collect::<Result<Vec<_>, String>>()?;
    Ok((outline, holes))
}

/// A shape the tool made, GeoJSON (RFC 7946: what `ext.polygon`, shapely's and GeoPandas' `__geo_interface__` give),
/// as an object on `layer`: a point, a line or an area, each of one part or several, an area with its holes.
fn shape(
    at: &str,
    g: &Value,
    layer: &str,
    attrs: BTreeMap<String, String>,
) -> Result<Entity, String> {
    // A GeoJSON feature's shape is its geometry.
    let g = if g.get("type").and_then(Value::as_str) == Some("Feature") {
        g.get("geometry").unwrap_or(&Value::Null)
    } else {
        g
    };
    let kind = g.get("type").and_then(Value::as_str).unwrap_or_default();
    let coordinates = g.get("coordinates").unwrap_or(&Value::Null);
    let c = format!("{at}.coordinates");
    let parts = |list: &Value| -> Result<Vec<Value>, String> {
        list.as_array()
            .filter(|l| !l.is_empty())
            .cloned()
            .ok_or_else(|| format!("{c}: boş şekil."))
    };
    let mut object = json!({ "id": 0, "layerId": layer, "attrs": attrs });
    match kind {
        "Point" => {
            object["kind"] = json!("point");
            object["p"] = position(&c, coordinates)?;
        }
        "MultiPoint" => {
            let list = parts(coordinates)?;
            object["kind"] = json!("point");
            object["p"] = position(&format!("{c}[0]"), &list[0])?;
            if list.len() > 1 {
                object["parts"] = Value::Array(
                    list[1..]
                        .iter()
                        .enumerate()
                        .map(|(i, p)| Ok(json!({ "p": position(&format!("{c}[{}]", i + 1), p)? })))
                        .collect::<Result<_, String>>()?,
                );
            }
        }
        "LineString" => {
            object["kind"] = json!("polyline");
            object["pts"] = Value::Array(positions(&c, coordinates, false)?);
        }
        "MultiLineString" => {
            let list = parts(coordinates)?;
            object["kind"] = json!("polyline");
            object["pts"] = Value::Array(positions(&format!("{c}[0]"), &list[0], false)?);
            if list.len() > 1 {
                object["parts"] = Value::Array(
                    list[1..]
                        .iter()
                        .enumerate()
                        .map(|(i, l)| {
                            Ok(json!({ "pts": positions(&format!("{c}[{}]", i + 1), l, false)? }))
                        })
                        .collect::<Result<_, String>>()?,
                );
            }
        }
        "Polygon" => {
            let (outline, holes) = area(&c, coordinates)?;
            object["kind"] = json!("polygon");
            object["pts"] = Value::Array(outline);
            if !holes.is_empty() {
                object["holes"] = Value::Array(holes);
            }
        }
        "MultiPolygon" => {
            let list = parts(coordinates)?;
            let (outline, holes) = area(&format!("{c}[0]"), &list[0])?;
            object["kind"] = json!("polygon");
            object["pts"] = Value::Array(outline);
            if !holes.is_empty() {
                object["holes"] = Value::Array(holes);
            }
            if list.len() > 1 {
                object["parts"] = Value::Array(
                    list[1..]
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            let (pts, holes) = area(&format!("{c}[{}]", i + 1), p)?;
                            Ok(if holes.is_empty() {
                                json!({ "pts": pts })
                            } else {
                                json!({ "pts": pts, "holes": holes })
                            })
                        })
                        .collect::<Result<_, String>>()?,
                );
            }
        }
        other => {
            return Err(format!(
                "{at}: “{other}” şekli yazılamaz (Point, LineString, Polygon ve çok parçalıları; GeoJSON)."
            ));
        }
    }
    serde_json::from_value(object).map_err(|e| format!("{at}: şekil okunamadı ({e})."))
}

/// An input object's copy on `layer`, its look the layer's: the shape of what the tool was given.
fn copy(e: &Entity, layer: &str, attrs: BTreeMap<String, String>, label: Option<String>) -> Entity {
    let mut c = crate::builtin::layers::copy_of(e, layer, Some(attrs));
    let b = c.base_mut();
    b.color = None;
    b.symbol = None;
    b.line_weight = None;
    b.label = label;
    b.label_pins = Vec::new();
    b.attachments = Vec::new();
    b.look = Default::default();
    c
}

/// The objects of a features output, for `layer`: each a shape of its own or an input's (by its persistent id, only
/// one the tool was given).
fn new_objects(
    output: &str,
    list: &[Value],
    layer: &str,
    given: &BTreeMap<String, &Entity>,
) -> Result<Vec<Entity>, String> {
    let mut out = Vec::with_capacity(list.len());
    for (i, item) in list.iter().enumerate() {
        let at = format!("{output}[{i}]");
        let o = item
            .as_object()
            .ok_or_else(|| format!("{at}: bir nesne değil (ext.NewFeature)."))?;
        // A GeoJSON feature (GeoPandas' rows, a FeatureCollection's features): its properties are its attributes.
        let feature = o.get("type").and_then(Value::as_str) == Some("Feature");
        let mut attrs = attributes(&at, o.get(if feature { "properties" } else { "attrs" }))?;
        let label = o
            .get("label")
            .and_then(Value::as_str)
            .map(|l| l.chars().take(MAX_TEXT).collect());
        match (o.get("source").and_then(Value::as_str), o.get("geometry")) {
            (Some(uid), _) => {
                let e = given.get(uid).ok_or_else(|| {
                    format!("{at}: “{uid}” araca verilen nesnelerden biri değil; araç yalnız girdilerinin şeklini kopyalar.")
                })?;
                if o.get("keepAttrs").and_then(Value::as_bool) != Some(false) {
                    let mut kept = e.base().attrs.clone();
                    kept.append(&mut attrs);
                    attrs = kept;
                }
                out.push(copy(e, layer, attrs, label));
            }
            (None, Some(g)) => {
                let mut e = shape(&format!("{at}.geometry"), g, layer, attrs)?;
                e.base_mut().label = label;
                out.push(e);
            }
            (None, None) => {
                return Err(format!(
                    "{at}: yeni nesnenin şekli yok: geometry (GeoJSON, ext.polygon, bir shapely şekli …) ya da source \
                     (bir girdi) gerekli."
                ));
            }
        }
    }
    Ok(out)
}

/// A cell of a table as the window shows it: a number as JavaScript writes it, empty for none.
fn cell(v: &Value) -> Value {
    json!(match v {
        Value::Null => String::new(),
        Value::String(s) => s.chars().take(MAX_TEXT).collect(),
        Value::Number(n) => n.as_f64().map(js_number).unwrap_or_default(),
        other => other.to_string(),
    })
}

/// A table output: KentOS's own (`{ columns, rows }`, what the extension's process sends for rows, their columns in
/// the order the rows first give them: a JSON object's keys keep no order here), a list of rows (column → value) or
/// a FeatureCollection's properties (a GeoDataFrame given as a table); each cell as text.
fn table(output: &str, v: &Value) -> Result<Value, String> {
    if let Some(columns) = v.get("columns") {
        let columns: Vec<String> = columns
            .as_array()
            .and_then(|c| c.iter().map(|x| x.as_str().map(str::to_owned)).collect())
            .ok_or_else(|| format!("{output}: columns bir metin listesi olmalı."))?;
        let rows = v
            .get("rows")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{output}: rows bir satır listesi olmalı."))?;
        if rows.len() > MAX_ROWS {
            return Err(format!("{output}: {MAX_ROWS} satırdan fazla."));
        }
        let rows = rows
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let cells = r
                    .as_array()
                    .filter(|c| c.len() <= columns.len())
                    .ok_or_else(|| {
                        format!("{output}.rows[{i}]: sütun sayısı kadar hücre olmalı.")
                    })?;
                let mut out: Vec<Value> = cells.iter().map(cell).collect();
                out.resize(columns.len(), json!(""));
                Ok(Value::Array(out))
            })
            .collect::<Result<Vec<_>, String>>()?;
        return Ok(json!({ "columns": columns, "rows": rows }));
    }
    let properties: Option<Vec<Value>> = (v.get("type").and_then(Value::as_str)
        == Some("FeatureCollection"))
    .then(|| v.get("features").and_then(Value::as_array))
    .flatten()
    .map(|f| {
        f.iter()
            .map(|x| x.get("properties").cloned().unwrap_or(json!({})))
            .collect()
    });
    let rows = properties.as_ref().or(v.as_array()).ok_or_else(|| {
        format!("{output}: bir tablo satır listesi olmalı ({{sütun: değer}} sözlükleri).")
    })?;
    if rows.len() > MAX_ROWS {
        return Err(format!("{output}: {MAX_ROWS} satırdan fazla."));
    }
    let mut columns: Vec<String> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let o = row
            .as_object()
            .ok_or_else(|| format!("{output}[{i}]: bir satır {{sütun: değer}} sözlüğü olmalı."))?;
        for k in o.keys() {
            if !columns.contains(k) {
                columns.push(k.clone());
            }
        }
    }
    let cells: Vec<Value> = rows
        .iter()
        .map(|row| {
            Value::Array(
                columns
                    .iter()
                    .map(|c| cell(row.get(c).unwrap_or(&Value::Null)))
                    .collect(),
            )
        })
        .collect();
    Ok(json!({ "columns": columns, "rows": cells }))
}

/// The code's answer read against the descriptor (PY-TOOL-10): every output declared and of its kind, every object
/// and value checked; why not, naming the place, and nothing is written.
fn result_of(
    tool: &ExternalTool,
    answer: &Value,
    resolved: &Resolved<'_>,
    ctx: &RunContext<'_>,
) -> Result<RunResult, String> {
    let words = |why: String| format!("“{}” tipli sonucu geçersiz: {why}", tool.tool.label);
    let given: BTreeMap<String, &Entity> = resolved
        .features
        .values()
        .flat_map(|set| set.entities.iter().copied())
        .filter_map(|e| {
            Some((
                ctx.doc.uid(kentos_domain::Slot(e.base().id))?.to_string(),
                e,
            ))
        })
        .collect();
    let outputs = match answer.get("outputs") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(o)) => o.clone(),
        Some(_) => return Err(words("outputs bir sözlük değil.".into())),
    };
    let mut result = RunResult::default();
    let mut add: Vec<Entity> = Vec::new();
    for (name, value) in &outputs {
        let Some(output) = tool.outputs.iter().find(|o| &o.name == name) else {
            return Err(words(format!(
                "“{name}” araç tanımında bir çıktı değil ({}).",
                tool.outputs
                    .iter()
                    .map(|o| o.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        };
        match output.kind {
            OutputKind::Features => {
                // A list of objects, or a GeoJSON FeatureCollection (a GeoDataFrame's `__geo_interface__`).
                let list = value
                    .as_array()
                    .or_else(|| {
                        (value.get("type").and_then(Value::as_str) == Some("FeatureCollection"))
                            .then(|| value.get("features").and_then(Value::as_array))
                            .flatten()
                    })
                    .ok_or_else(|| {
                        words(format!(
                            "“{name}” bir nesne listesi ya da FeatureCollection olmalı."
                        ))
                    })?;
                let layer = output
                    .layer
                    .as_deref()
                    .map(|p| resolved.layer(p).id.clone())
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| words(format!("“{name}”: yazılacağı katman seçilmedi.")))?;
                if add.len() + list.len() > MAX_OBJECTS {
                    return Err(words(format!("{MAX_OBJECTS} nesneden fazla.")));
                }
                add.extend(new_objects(name, list, &layer, &given).map_err(words)?);
            }
            OutputKind::Table => {
                if !value.is_null() {
                    result
                        .outputs
                        .insert(name.clone(), table(name, value).map_err(words)?);
                }
            }
            OutputKind::Number => {
                let n = match value {
                    Value::Null => Value::Null,
                    Value::Number(_) => value.clone(),
                    _ => return Err(words(format!("“{name}” bir sayı olmalı."))),
                };
                result.outputs.insert(name.clone(), n);
            }
            OutputKind::Text => {
                let t = match value {
                    Value::Null => Value::Null,
                    Value::String(s) if s.chars().count() <= MAX_TEXT => value.clone(),
                    Value::String(_) => {
                        return Err(words(format!("“{name}” {MAX_TEXT} karakterden uzun.")));
                    }
                    _ => return Err(words(format!("“{name}” bir metin olmalı."))),
                };
                result.outputs.insert(name.clone(), t);
            }
        }
    }
    if let Some(select) = answer.get("select") {
        let uids = select
            .as_array()
            .ok_or_else(|| words("select bir kimlik listesi olmalı.".into()))?;
        let mut slots = Vec::new();
        for uid in uids {
            let uid = uid.as_str().unwrap_or_default();
            let e = given.get(uid).ok_or_else(|| {
                words(format!(
                    "seçilecek “{uid}” araca verilen nesnelerden biri değil."
                ))
            })?;
            slots.push(kentos_domain::Slot(e.base().id));
        }
        result.select = Some(slots);
    }
    result.summary = answer
        .get("summary")
        .and_then(Value::as_str)
        .map(|s| s.chars().take(500).collect());
    if !add.is_empty() {
        result.changes = Some(ChangeSet {
            add,
            ..ChangeSet::default()
        });
    }
    Ok(result)
}
