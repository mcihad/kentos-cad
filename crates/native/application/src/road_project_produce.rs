//! `road.project.produce` v1 (plan/map-cad.md MAP-20, docs/adr/3240): Tariften üret. A recipe (İş grafiği's Tarifi
//! kaydet…, `road.project.graph`'s answer as JSON, docs/adr/1940) drawn in this project, in one undo step “Tariften
//! üret”: each of its drawings by its command (the version this KentOS draws, else refused), with its input as kept,
//! a source the recipe names by its id in the recipe's project given by its name (the commands find a source by id or
//! name; a layer stays by its id). A drawing this project has (the same command from the same main source on the same
//! layer) is left: `road.project.rebuild` draws it again. Named drawings are all drawn or none (a command's refusal,
//! the recipe's drawing named in its words); every drawing asked, one that does not draw here is left with the
//! command's words. Each drawing is kept as a drawing of this project; the progress goes a drawing at a time and a stop
//! is heard between them (docs/adr/2440). A drawing's layer (docs/adr/3640): the one `layers` gives for its recipe
//! layer; else, the recipe keeping the layer's definition, the layer of that name here (folded, as Katmanları çevir
//! finds a standard layer; two of them: which one is asked), or one added with the recipe's look (kept only when a
//! drawing went on it); a recipe from before, by the layer's id. The shared cases are
//! `fixtures/commands/v1/road.project.produce.json`.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use kentos_commands::{
    CommandResult, CommandWarning, RoadDrawingLeft, RoadDrawingProduced, RoadGraphKind as G,
    RoadLayerMatch, RoadProducedLayer, RoadProducedSource, RoadProjectGraph, RoadProjectProduce, RoadProjectProduced,
    RoadRecipeLayer, fold_name,
};
use kentos_domain::{Document, NewLayer, default_style};
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::road_drawings;
use crate::road_project_rebuild::{in_share, run_drawing};
use crate::trim::relay;

/// The input's field a layer goes by: it stays as the recipe keeps it.
const LAYER: &str = "layerId";

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A recipe's drawing to draw here: its id in the recipe, its command, its input with the recipe's sources by name; its
/// layer's id in the recipe and the layer's definition the recipe keeps (docs/adr/3640).
struct Wanted {
    from: String,
    command: String,
    input: BTreeMap<String, Value>,
    layer_from: Option<String>,
    layer: Option<RoadRecipeLayer>,
}

struct Asked {
    wanted: Vec<Wanted>,
    /// Drawings named: a command's refusal refuses them all; none named, the drawing is left with its words.
    named: bool,
    /// The layers given: a recipe layer's id → this project's (docs/adr/3640).
    given: BTreeMap<String, String>,
    warnings: Vec<CommandWarning>,
}

/// Where a recipe drawing's layer is here (docs/adr/3640).
enum Place {
    Here(String, RoadLayerMatch),
    /// No layer of its name: one is added with the recipe's look.
    Create(RoadRecipeLayer),
    /// More than one layer of its name: their ids.
    Ambiguous(String, Vec<String>),
}

/// Where `w`'s layer is here: given; by the recipe's definition, the layer of its name (folded), or to be added; a
/// recipe from before, its id. None: the drawing's input names no layer.
fn place(doc: &Document, w: &Wanted, given: &BTreeMap<String, String>) -> Option<Place> {
    let from = w.layer_from.as_deref()?;
    if let Some(to) = given.get(from) {
        return Some(Place::Here(to.clone(), RoadLayerMatch::Given));
    }
    let Some(def) = &w.layer else { return Some(Place::Here(from.to_owned(), RoadLayerMatch::Id)) };
    let want = fold_name(def.name.trim());
    let hits: Vec<String> =
        doc.layers().leaves().into_iter().filter(|n| fold_name(n.name.trim()) == want).map(|n| n.id.clone()).collect();
    Some(match hits.len() {
        0 => Place::Create(def.clone()),
        1 => Place::Here(hits[0].clone(), RoadLayerMatch::Name),
        _ => Place::Ambiguous(def.name.clone(), hits),
    })
}

/// `input` with its layer `id`.
fn on_layer(input: &BTreeMap<String, Value>, id: &str) -> BTreeMap<String, Value> {
    let mut out = input.clone();
    out.insert(LAYER.to_owned(), Value::String(id.to_owned()));
    out
}

/// The kinds of a recipe's nodes whose ids a drawing's input names its sources by.
fn source_kind(k: G) -> bool {
    matches!(
        k,
        G::Surface
            | G::Alignment
            | G::Profile
            | G::Sections
            | G::Superelevation
            | G::Junction
            | G::Template
            | G::Corridor
            | G::Guardrail
            | G::Culvert
            | G::Wall
            | G::Platform
    )
}

/// `value` with every text that is the id of a source of the recipe's project its name there (a layer kept).
fn by_name(value: Value, names: &HashMap<&str, &str>) -> Value {
    match value {
        Value::String(s) => Value::String(names.get(s.as_str()).map_or(s, |n| (*n).to_owned())),
        Value::Array(a) => Value::Array(a.into_iter().map(|v| by_name(v, names)).collect()),
        Value::Object(o) => {
            Value::Object(o.into_iter().map(|(k, v)| if k == LAYER { (k, v) } else { (k, by_name(v, names)) }).collect())
        }
        other => other,
    }
}

/// A file's name, as a refusal names it.
fn file_name(text: &str) -> String {
    Path::new(text).file_name().map_or_else(|| text.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// The recipe at `text`: İş grafiği's graph, refused at `path` as the contract says.
fn read(text: &str) -> Result<RoadProjectGraph, Stop> {
    let path = Path::new(text);
    if text.trim().is_empty() || !path.is_absolute() {
        return Err(refuse(codes::INVALID_FILE_PATH, "Tarifin yolu boş ya da göreli; mutlak bir .json yolu verin.".into(), "path"));
    }
    if !text.to_ascii_lowercase().ends_with(".json") {
        return Err(refuse(codes::INVALID_FILE_PATH, format!("“{text}” bir .json dosyası değil (İş grafiği'nin tarifi)."), "path"));
    }
    let name = file_name(text);
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(refuse(
                codes::FILE_NOT_FOUND,
                format!("“{text}” tarifi yok; İş grafiği'nin Tarifi kaydet… ile yazdığı .json dosyasını seçin."),
                "path",
            ));
        }
        Err(e) => return Err(refuse(codes::INVALID_PROJECT_RECIPE, format!("“{name}” okunamadı: {e}."), "path")),
    };
    serde_json::from_slice(&bytes).map_err(|e| {
        refuse(codes::INVALID_PROJECT_RECIPE, format!("“{name}” İş grafiği'nin tarifi değil ({e}); Tarifi kaydet… ile yazılan dosyayı seçin."), "path")
    })
}

/// A system in words: its name and code, or a local project.
fn system_words(name: Option<&str>, epsg: &str) -> String {
    match (name, epsg) {
        (_, "") => "yerel (koordinat sistemi yok)".to_owned(),
        (Some(n), code) => format!("{n} ({code})"),
        (None, code) => code.to_owned(),
    }
}

fn asked(doc: &Document, input: &RoadProjectProduce) -> Result<Asked, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    crate::road_project::road_project(doc)?;
    let graph = read(&input.path)?;
    let name = file_name(&input.path);
    let names: HashMap<&str, &str> =
        graph.nodes.iter().filter(|n| source_kind(n.kind)).map(|n| (n.id.as_str(), n.name.as_str())).collect();
    let recipes: Vec<(&str, &kentos_commands::RoadDrawingRecipe)> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == G::Drawing)
        .filter_map(|n| n.recipe.as_ref().map(|r| (n.id.as_str(), r)))
        .collect();
    if recipes.is_empty() {
        return Err(refuse(codes::INVALID_PROJECT_RECIPE, format!("“{name}” tarifinde üretilecek çizim yok."), "path"));
    }
    let chosen: Vec<(&str, &kentos_commands::RoadDrawingRecipe)> = match &input.drawings {
        Some(ids) => {
            if let Some(id) = ids.iter().find(|id| !recipes.iter().any(|(i, _)| i == id)) {
                return Err(refuse(codes::UNKNOWN_DRAWING, format!("Tarifte “{id}” kimlikli çizim yok."), "drawings"));
            }
            recipes.into_iter().filter(|(i, _)| ids.iter().any(|x| x == i)).collect()
        }
        None => recipes,
    };
    let mut wanted = Vec::new();
    for (id, r) in chosen {
        if road_drawings::main_of(&r.command).is_none() {
            return Err(refuse(
                codes::INVALID_PROJECT_RECIPE,
                format!("Tarifteki “{id}” çiziminin komutu ({}) çizen bir komut değil.", r.command),
                "path",
            ));
        }
        let here = road_drawings::version_of(&r.command);
        if r.version != here {
            return Err(refuse(
                codes::INVALID_PROJECT_RECIPE,
                format!("Tarifteki “{id}” çizimi {} v{} ile; bu KentOS v{here} ile çizer.", r.command, r.version),
                "path",
            ));
        }
        let input = r
            .input
            .iter()
            .map(|(k, v)| (k.clone(), if k == LAYER { v.clone() } else { by_name(v.clone(), &names) }))
            .collect();
        let layer_from = r.input.get(LAYER).and_then(Value::as_str).map(str::to_owned);
        if let Some(def) = &r.layer
            && layer_from.as_deref() != Some(def.id.as_str())
        {
            return Err(refuse(
                codes::INVALID_PROJECT_RECIPE,
                format!(
                    "Tarifteki “{id}” çiziminin katman tanımı ({}) girdisinin katmanı ({}) değil.",
                    def.id,
                    layer_from.as_deref().unwrap_or("yok")
                ),
                "path",
            ));
        }
        wanted.push(Wanted { from: id.to_owned(), command: r.command.clone(), input, layer_from, layer: r.layer.clone() });
    }
    // The layers given (docs/adr/3640): each a layer of the recipe's drawings, each a layer here that takes objects.
    let theirs: Vec<&str> = graph
        .nodes
        .iter()
        .filter_map(|n| n.recipe.as_ref())
        .filter_map(|r| r.input.get(LAYER).and_then(Value::as_str))
        .collect();
    for (from, to) in &input.layers {
        if !theirs.contains(&from.as_str()) {
            return Err(refuse(
                codes::UNKNOWN_RECIPE_LAYER,
                format!("Tarifte “{from}” kimlikli katmana çizilen çizim yok; layers'ın anahtarı tarifin bir katmanı olmalı."),
                "layers",
            ));
        }
        checks::layer_at(doc, to, &format!("layers.{from}"))?;
    }
    let mut warnings = Vec::new();
    let settings = doc.settings();
    let facts = kentos_project::facts::coordinate_facts(settings);
    let theirs = graph.epsg.as_deref().unwrap_or("");
    if theirs != facts.epsg {
        let ours = kentos_project::crs::system(settings.srid).map(|s| s.name.clone());
        warnings.push(CommandWarning {
            code: codes::RECIPE_OTHER_SYSTEM.into(),
            message: format!(
                "Tarif {} projesinde kaydedilmiş; bu proje {}. Girdiler aynen alınır.",
                system_words(graph.system.as_deref(), theirs),
                system_words(ours.as_deref(), &facts.epsg)
            ),
            path: Some("path".into()),
        });
    }
    Ok(Asked { wanted, named: input.drawings.is_some(), given: input.layers.clone(), warnings })
}

/// Whether the project has the drawing: the same command from the same main source on the same layer (`input`'s, the
/// layer found here).
fn exists(doc: &Document, command: &str, input: &BTreeMap<String, Value>, field: &str, main: &str) -> bool {
    let layer = input.get(LAYER);
    doc.road_drawings()
        .iter()
        .any(|d| d.command == command && d.input.get(field).and_then(Value::as_str) == Some(main) && d.input.get(LAYER) == layer)
}

/// A command's refusal with the recipe's drawing named in its words.
fn naming(result: CommandResult<()>, from: &str, words: &str) -> CommandResult<()> {
    match result {
        CommandResult::Failed { mut error } => {
            error.message = format!("Tarifin “{from}” çizimi ({words}): {}", error.message);
            CommandResult::Failed { error }
        }
        CommandResult::Conflict { mut error } => {
            error.message = format!("Tarifin “{from}” çizimi ({words}): {}", error.message);
            CommandResult::Conflict { error }
        }
        other => other,
    }
}

fn write(cx: &mut ExecutionContext<'_>, asked: Asked) -> CommandResult<RoadProjectProduced> {
    let mut produced = Vec::new();
    let mut left = Vec::new();
    let mut warnings = asked.warnings;
    let mut stepped = false;
    // The layers the drawings went on, and those added here with whether a drawing went on them (docs/adr/3640).
    let mut layers: Vec<RoadProducedLayer> = Vec::new();
    let mut added: Vec<(String, String, bool)> = Vec::new();
    let total = asked.wanted.len() as u64;
    for (k, w) in asked.wanted.iter().enumerate() {
        cx.progress(k as u64, total);
        if cx.stopped() {
            return CommandResult::Cancelled;
        }
        let Some(main) = road_drawings::main_of(&w.command) else { continue };
        let words = crate::road_project::drawing_words(&w.command);
        // The drawing's layer here: found, to be added, or asked (two of its name).
        let (input, found) = match place(cx.doc, w, &asked.given) {
            None => (w.input.clone(), None),
            Some(Place::Here(id, how)) => (on_layer(&w.input, &id), Some((id, how))),
            Some(Place::Create(def)) => {
                if !stepped {
                    cx.doc.name_step("Tariften üret");
                    stepped = true;
                }
                let mut new = NewLayer::layer(def.name.trim());
                new.style = kentos_commands::LayerStyle {
                    color: def.color.clone(),
                    line_type: def.line_type,
                    line_weight: def.line_weight,
                    ..default_style()
                };
                match cx.doc.add_layer_at(new, None, None, Some("Tariften üret"), false) {
                    Ok(id) => {
                        added.push((id.clone(), def.name.trim().to_owned(), false));
                        (on_layer(&w.input, &id), Some((id, RoadLayerMatch::Created)))
                    }
                    Err(r) => return Stop::failed(error(codes::LAYER_REFUSED, r.0, Some("layers".into()))).into(),
                }
            }
            Some(Place::Ambiguous(name, ids)) => {
                let message = format!(
                    "Projede “{name}” adlı {} katman var ({}); tarifin katmanının hangisi olduğunu layers ile verin.",
                    ids.len(),
                    ids.join(", ")
                );
                if asked.named {
                    return relay(naming(
                        Stop::failed(error(codes::AMBIGUOUS_RECIPE_LAYER, message, Some("layers".into()))).into(),
                        &w.from,
                        words,
                    ));
                }
                left.push(RoadDrawingLeft { drawing: w.from.clone(), why: "refused".into(), message });
                continue;
            }
        };
        let text = input.get(main.1).and_then(Value::as_str).unwrap_or_default();
        let here = road_drawings::resolve(cx.doc, main.0, text);
        if here.as_deref().is_some_and(|id| exists(cx.doc, &w.command, &input, main.1, id)) {
            left.push(RoadDrawingLeft {
                drawing: w.from.clone(),
                why: "exists".into(),
                message: "Projede bu çizim var (aynı komut, ana kaynak ve katman); Yeniden üret ile güncellenir.".into(),
            });
            continue;
        }
        if !stepped {
            cx.doc.name_step("Tariften üret");
            stepped = true;
        }
        match in_share(cx, k as u64, total, |cx| run_drawing(cx, &w.from, &w.command, &input, &[])) {
            Ok((uids, said)) => {
                if let Some((id, how)) = found {
                    if let Some(a) = added.iter_mut().find(|a| a.0 == id) {
                        a.2 = true;
                    }
                    let from = w.layer_from.clone().unwrap_or_default();
                    if !layers.iter().any(|l| l.from == from) {
                        // A recipe from before keeps no name: the layer's here.
                        let to_name = cx.doc.layers().get(&id).map(|n| n.name.clone()).unwrap_or_default();
                        let name = w.layer.as_ref().map_or_else(|| to_name.clone(), |d| d.name.clone());
                        layers.push(RoadProducedLayer { from, name, to: id, to_name, how });
                    }
                }
                let drawing = match road_drawings::keep(cx.doc, &w.command, &input, main, &uids, None) {
                    Ok(d) => d,
                    Err(stop) => return stop.into(),
                };
                let id = here.unwrap_or_default();
                let name = road_drawings::name_of(cx.doc, main.0, &id).unwrap_or_default();
                produced.push(RoadDrawingProduced {
                    from: w.from.clone(),
                    drawing,
                    command: w.command.clone(),
                    source: RoadProducedSource { kind: main.0, id, name },
                    uids,
                });
                warnings.extend(said);
            }
            // Every drawing asked: one whose check refuses here is left with the command's words (nothing of it was
            // written); a refusal after the check, or any of named drawings, refuses them all.
            Err((refused, written)) => match *refused {
                CommandResult::Failed { error } if !asked.named && !written => {
                    left.push(RoadDrawingLeft { drawing: w.from.clone(), why: "refused".into(), message: error.message });
                }
                other => return relay(naming(other, &w.from, words)),
            },
        }
    }
    // A layer added for drawings none of which went on it is taken away again; one kept is said (docs/adr/3640).
    for (id, name, used) in added {
        if used {
            warnings.push(CommandWarning {
                code: codes::RECIPE_LAYER_CREATED.into(),
                message: format!("Tarifin “{name}” katmanı bu projede yoktu; tarifteki tanımıyla (renk, çizgi türü, kalınlık) eklendi."),
                path: Some("layers".into()),
            });
        } else {
            let _ = cx.doc.remove_layer(&id);
        }
    }
    cx.progress(total, total);
    CommandResult::Completed {
        output: RoadProjectProduced { produced, left, layers, revision: cx.doc.revision().to_string() },
        warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoadProjectProduce) -> CommandResult<()> {
    match plan(cx, input) {
        CommandResult::Completed { warnings, .. } => CommandResult::Completed { output: (), warnings },
        other => relay(other),
    }
}

/// What execute would draw, on a reading copy; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoadProjectProduce) -> CommandResult<RoadProjectProduced> {
    let asked = match asked(cx.doc, input) {
        Ok(a) => a,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), asked);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.produced.iter_mut().for_each(|p| p.uids.clear());
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Draws the recipe's drawings: one undo step “Tariften üret”, all of them or none.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadProjectProduce) -> CommandResult<RoadProjectProduced> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_PROJECT_PRODUCE,
        kentos_commands::ROAD_PROJECT_PRODUCE_VERSION,
        input,
        |cx, input| match asked(cx.doc, &input) {
            Ok(a) => write(cx, a),
            Err(stop) => stop.into(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jobs::Progress;
    use kentos_commands::{CallOp, DocumentSnapshotV2};

    /// The shared cases' Yol project (road_terrain_zeroline_cases.py's) and the recipe of two zero lines.
    fn project() -> (Document, RoadProjectProduce) {
        let cases: Value =
            serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.project.produce.json")).expect("cases");
        let snapshot: DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("a snapshot");
        let doc = Document::from_snapshot_v2(snapshot).expect("opens");
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/recipes/v1/sifir-hatlari.json");
        (doc, RoadProjectProduce { path: path.display().to_string(), drawings: None, layers: Default::default(), expected_revision: None })
    }

    /// docs/adr/2440: a stop asked before the first drawing is heard: cancelled, nothing written.
    #[test]
    fn a_stopped_produce_writes_nothing() {
        let (mut doc, input) = project();
        let before = doc.to_snapshot_v2();
        let job = Progress::default();
        job.stop();
        let Ok(asked) = asked(&doc, &input) else { panic!("a recipe") };
        let mut cx = ExecutionContext::new(&mut doc);
        cx.job = Some(&job);
        assert!(matches!(write(&mut cx, asked), CommandResult::Cancelled));
        assert_eq!(doc.to_snapshot_v2(), before);
    }

    /// The progress goes a drawing at a time and ends whole.
    #[test]
    fn the_progress_is_the_drawings() {
        let (mut doc, input) = project();
        let job = Progress::default();
        let Ok(asked) = asked(&doc, &input) else { panic!("a recipe") };
        let mut cx = ExecutionContext::new(&mut doc);
        cx.job = Some(&job);
        let CommandResult::Completed { output, .. } = write(&mut cx, asked) else { panic!("produced") };
        assert_eq!(output.produced.iter().map(|p| p.drawing.as_str()).collect::<Vec<_>>(), ["cz-1", "cz-2"]);
        assert_eq!(job.fraction(), Some(1.0));
    }

    /// Through the registry, as Python and MCP call it: the drawings are the project's (current in its tree, in its
    /// graph with their recipes), and one undo step “Tariften üret” takes them back.
    #[test]
    fn the_drawings_are_the_project_s_and_one_step_undoes_them() {
        let (mut doc, input) = project();
        let before = doc.to_snapshot_v2();
        let r = crate::registry::run(
            &mut ExecutionContext::new(&mut doc),
            kentos_commands::ROAD_PROJECT_PRODUCE,
            CallOp::Execute,
            serde_json::to_value(&input).expect("json"),
        )
        .expect("runs");
        assert_eq!(r["status"], "completed", "{r}");
        let tree = crate::road_project_query::preview(&doc, false).expect("the tree");
        assert_eq!(tree.rebuildable, 0, "both drawings current");
        let graph = crate::road_project_graph::preview(&doc, false).expect("the graph");
        assert_eq!(graph.drawings, 2);
        let commands: Vec<&str> = graph.nodes.iter().filter_map(|n| n.recipe.as_ref()).map(|r| r.command.as_str()).collect();
        assert_eq!(commands, ["road.terrains.zeroline", "road.terrains.zeroline"]);
        assert_eq!(doc.undo().as_deref(), Some("Tariften üret"));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}
