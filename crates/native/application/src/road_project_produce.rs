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
//! drawing went on it); a recipe from before, by the layer's id. Docs/adr/4440: a layer added takes the recipe layer's
//! opacity and blending mode, its type and its Yazdır too; what a drawing writes that its layer's type does not hold
//! refuses the produce here, the plan too. The shared cases are `fixtures/commands/v1/road.project.produce.json`.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use kentos_commands::{
    CommandResult, CommandWarning, LayerNodeType, RoadDrawingLeft, RoadDrawingProduced, RoadGraphKind as G,
    RoadLayerMatch, RoadProducedLayer, RoadProducedSource, RoadProjectGraph, RoadProjectProduce, RoadProjectProduced,
    RoadRecipeLayer, fold_name, same_line_type_name,
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
    /// Docs/adr/5240: a layer found here takes the recipe's fields it lacks.
    add_fields: bool,
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
        if let Some(def) = &r.layer
            && let Some(why) = definition_problem(def)
        {
            return Err(refuse(
                codes::INVALID_PROJECT_RECIPE,
                format!("Tarifteki “{id}” çiziminin katman tanımı ({}) tutarsız: {why}", def.id),
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
    Ok(Asked { wanted, named: input.drawings.is_some(), given: input.layers.clone(), add_fields: input.add_fields, warnings })
}

/// What does not hold in a recipe layer's definition (docs/adr/4040): a group without a name; a style whose colour,
/// line type or weight is not the definition's; a line type definition its style does not name, or one that does not
/// hold as a project line type. Docs/adr/4440: a composition with a mask, one that says nothing, or an opacity outside
/// 0–1.
fn definition_problem(def: &RoadRecipeLayer) -> Option<String> {
    if def.group.iter().any(|g| g.trim().is_empty()) {
        return Some("grup adlarından biri boş.".to_owned());
    }
    if let Some(s) = &def.style
        && (s.color != def.color || s.line_type != def.line_type || s.line_weight != def.line_weight)
    {
        return Some("stilin rengi, çizgi türü ya da kalınlığı tanımınkiyle aynı değil.".to_owned());
    }
    if let Some(c) = &def.compose {
        if c.mask.is_some() {
            return Some(
                "katman çiziminde maske var; maske projesinin öbür katmanlarını kimlikleriyle keser, tarifte tutulmaz."
                    .to_owned(),
            );
        }
        if c.opacity.is_none() && c.blend.is_none() {
            return Some("katman çizimi boş: saydamlık da karışım kipi de yok.".to_owned());
        }
        if let Some(p) = c.problem() {
            return Some(format!("katman çiziminde {p}."));
        }
    }
    // Docs/adr/4860: its fields, the lists they link and its subtypes.
    if let Some(p) = crate::road_recipe_fields::problem(def) {
        return Some(p);
    }
    let d = def.line_type_def.as_ref()?;
    let named = def.style.as_ref().and_then(|s| s.line_pattern.as_deref());
    if !named.is_some_and(|n| same_line_type_name(n, &d.name)) {
        return Some(format!("“{}” çizgi türünün tanımı var ama stil onu adlamıyor.", d.name));
    }
    kentos_commands::line_types_problem(std::slice::from_ref(d)).map(|p| format!("{p}."))
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

/// The undo step's name, and the label of what it adds.
const STEP: &str = "Tariften üret";

/// A layer added for a recipe layer (docs/adr/3640, 4040): its id and the recipe layer's name, whether a drawing went
/// on it; the groups it went in (their names here), those added for it (id, name), the project line type added for it,
/// one its style named that neither this project nor the recipe defines (left out), and whether the recipe kept its
/// whole style; what it took from the recipe beyond its style, said (docs/adr/4440).
struct Added {
    id: String,
    name: String,
    used: bool,
    group: Vec<String>,
    groups_added: Vec<(String, String)>,
    line_type_added: Option<String>,
    dropped: Option<String>,
    styled: bool,
    look: Option<String>,
    /// Its fields, the lists they link and its subtypes as it took them (docs/adr/4860).
    given: crate::road_recipe_fields::Given,
}

/// What a layer added from `def` takes beyond its style (docs/adr/4440), in the words of the layer's panels: its
/// transparency (Saydamlık, 1 − opacity in percent), its blending mode, its type and that it is not printed —
/// “Tarifteki gibi saydamlığı %40, karışım kipi Çoğalt, türü Çizgi ve yazdırılmaz.”; none when it takes none of them.
pub fn look_said(def: &RoadRecipeLayer) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(c) = def.compose.as_ref().and_then(kentos_commands::LayerCompose::normalized) {
        if let Some(o) = c.opacity {
            parts.push(format!("saydamlığı %{}", crate::road_superelevations::percent(1.0 - o)));
        }
        if let Some(b) = c.blend {
            parts.push(format!("karışım kipi {}", b.label()));
        }
    }
    if let Some(g) = def.geometry {
        parts.push(format!("türü {}", g.label()));
    }
    if def.plot == Some(false) {
        parts.push("yazdırılmaz".to_owned());
    }
    let (last, rest) = parts.split_last()?;
    let said = if rest.is_empty() { last.clone() } else { format!("{} ve {last}", rest.join(", ")) };
    Some(format!("Tarifteki gibi {said}."))
}

/// The first object of `uids` its layer's type does not hold (docs/adr/0622 §5, 4440), said with what to do. The step's
/// integrity check would refuse it when the step is written; told here, the plan says it too.
fn misfit(doc: &Document, uids: &[String]) -> Option<String> {
    uids.iter().find_map(|u| {
        let e = kentos_domain::Uuid::parse_str(u).ok().and_then(|uid| doc.slot_of(uid)).and_then(|s| doc.get(s))?;
        let node = doc.layers().get(&e.base().layer_id)?;
        let holds = node.geometry?;
        (!holds.admits(e.kind())).then(|| {
            format!(
                "“{}” bir {}; {} bu katmana yazılmaz. Çizimin katmanını türüne uyan bir katman olarak layers ile verin ya da \
                 katmanın türünü Karışık yapın; hiçbir şey yazılmadı.",
                node.name,
                holds.layer_words(),
                kentos_commands::kind_words(e.kind())
            )
        })
    })
}

/// Adds the layer of `def` here (docs/adr/4040): in its groups, each found among its parent's groups by its name
/// (folded, the first of that name) or added last in it; with its whole style (a recipe from before: its colour, line
/// type and weight over the default), the project line type the style names taken from here by its name, else added
/// from the recipe's definition, else left out (its nearest built-in type draws); last among its group's nodes.
fn create(doc: &mut Document, def: &RoadRecipeLayer) -> Result<Added, Stop> {
    let refused = |r: kentos_domain::Refusal| Stop::failed(error(codes::LAYER_REFUSED, r.0, Some("layers".into())));
    let mut parent: Option<String> = None;
    let (mut group, mut groups_added) = (Vec::new(), Vec::new());
    for name in &def.group {
        let want = fold_name(name.trim());
        let layers = doc.layers();
        let siblings = match &parent {
            None => layers.nodes(),
            Some(p) => layers.get(p).map_or(&[][..], |n| n.children.as_slice()),
        };
        let found = siblings.iter().find(|n| n.kind == LayerNodeType::Group && fold_name(n.name.trim()) == want);
        let (id, here) = match found {
            Some(n) => (n.id.clone(), n.name.clone()),
            None => {
                let id = doc.add_layer_at(NewLayer::group(name.trim()), parent.as_deref(), None, Some(STEP), false).map_err(refused)?;
                groups_added.push((id.clone(), name.trim().to_owned()));
                (id, name.trim().to_owned())
            }
        };
        group.push(here);
        parent = Some(id);
    }
    let mut style = match &def.style {
        Some(s) => s.clone(),
        None => kentos_commands::LayerStyle {
            color: def.color.clone(),
            line_type: def.line_type,
            line_weight: def.line_weight,
            ..default_style()
        },
    };
    let (mut line_type_added, mut dropped) = (None, None);
    if let Some(name) = style.line_pattern.clone() {
        let settings = doc.settings();
        if let Some(t) = settings.line_types.iter().find(|t| same_line_type_name(&t.name, &name)) {
            style.line_pattern = Some(t.name.clone());
        } else if let Some(d) = def.line_type_def.as_ref() {
            let mut types = settings.line_types.clone();
            types.push(d.clone());
            let hatches = settings.hatch_patterns.clone();
            doc.set_pattern_tables(types, hatches, STEP);
            style.line_pattern = Some(d.name.clone());
            line_type_added = Some(d.name.clone());
        } else {
            style.line_pattern = None;
            dropped = Some(name);
        }
    }
    let mut new = NewLayer::layer(def.name.trim());
    new.style = style;
    // Docs/adr/4440: its composition, type and Yazdır as the recipe keeps them.
    new.compose = def.compose.as_ref().and_then(kentos_commands::LayerCompose::normalized);
    new.geometry = def.geometry;
    new.plot = def.plot.filter(|printed| !printed);
    let id = doc.add_layer_at(new, parent.as_deref(), None, Some(STEP), false).map_err(refused)?;
    // Docs/adr/4860: its fields, the lists they link (found here or added) and its subtypes.
    let given = crate::road_recipe_fields::give(doc, &id, def, STEP)?;
    Ok(Added {
        id,
        name: def.name.trim().to_owned(),
        used: false,
        group,
        groups_added,
        line_type_added,
        dropped,
        styled: def.style.is_some(),
        look: look_said(def),
        given,
    })
}

/// What adding `a` brought, said (docs/adr/3640, 4040): its look, its group, the groups and the line type added with it.
fn created_words(a: &Added) -> String {
    let look = if a.styled { "tarifteki stiliyle" } else { "tarifteki tanımıyla (renk, çizgi türü, kalınlık)" };
    let mut out = if a.group.is_empty() {
        format!("Tarifin “{}” katmanı bu projede yoktu; {look} eklendi.", a.name)
    } else {
        format!("Tarifin “{}” katmanı bu projede yoktu; {look} “{}” grubuna eklendi.", a.name, a.group.join(" › "))
    };
    match a.groups_added.as_slice() {
        [] => {}
        [(_, one)] => out.push_str(&format!(" “{one}” grubu da eklendi.")),
        many => {
            let names: Vec<String> = many.iter().map(|(_, n)| format!("“{n}”")).collect();
            out.push_str(&format!(" {} grupları da eklendi.", names.join(", ")));
        }
    }
    if let Some(t) = &a.line_type_added {
        out.push_str(&format!(" “{t}” çizgi türü de tarifteki tanımıyla eklendi."));
    }
    if let Some(t) = &a.dropped {
        out.push_str(&format!(" “{t}” çizgi türü bu projede de tarifte de tanımlı değil; katman yerleşik türüyle çizilir."));
    }
    if let Some(look) = &a.look {
        out.push(' ');
        out.push_str(look);
    }
    if let Some(fields) = crate::road_recipe_fields::said(&a.given) {
        out.push(' ');
        out.push_str(&fields);
    }
    out
}

fn write(cx: &mut ExecutionContext<'_>, asked: Asked) -> CommandResult<RoadProjectProduced> {
    let mut produced = Vec::new();
    let mut left = Vec::new();
    let mut warnings = asked.warnings;
    let mut stepped = false;
    // The layers the drawings went on, and those added here with whether a drawing went on them (docs/adr/3640).
    let mut layers: Vec<RoadProducedLayer> = Vec::new();
    let mut added: Vec<Added> = Vec::new();
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
                    cx.doc.name_step(STEP);
                    stepped = true;
                }
                match create(cx.doc, &def) {
                    Ok(a) => {
                        let id = a.id.clone();
                        added.push(a);
                        (on_layer(&w.input, &id), Some((id, RoadLayerMatch::Created)))
                    }
                    Err(stop) => return stop.into(),
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
            cx.doc.name_step(STEP);
            stepped = true;
        }
        match in_share(cx, k as u64, total, |cx| run_drawing(cx, &w.from, &w.command, &input, &[])) {
            Ok((uids, said)) => {
                // Docs/adr/4440: an object its layer's type does not hold refuses them all here, the plan too (written:
                // the step's integrity check would refuse it only when execute ends).
                if let Some(why) = misfit(cx.doc, &uids) {
                    return relay(naming(
                        Stop::failed(error(codes::LAYER_GEOMETRY_MISMATCH, why, Some("layers".into()))).into(),
                        &w.from,
                        words,
                    ));
                }
                if let Some((id, how)) = found {
                    let made = added.iter_mut().find(|a| a.id == id);
                    let (group, groups_added, line_type_added, mut domains_added, forms_added) = match made {
                        Some(a) => {
                            a.used = true;
                            (
                                a.group.clone(),
                                a.groups_added.iter().map(|g| g.1.clone()).collect(),
                                a.line_type_added.clone(),
                                a.given.added(),
                                a.given.forms.iter().map(|f| f.1.clone()).collect(),
                            )
                        }
                        None => (Vec::new(), Vec::new(), None, Vec::new(), Vec::new()),
                    };
                    let from = w.layer_from.clone().unwrap_or_default();
                    let mut fields_added = Vec::new();
                    if !layers.iter().any(|l| l.from == from) {
                        // Docs/adr/5240: asked, a layer found here takes the recipe's fields it lacks.
                        if how != RoadLayerMatch::Created
                            && asked.add_fields
                            && let Some(def) = &w.layer
                        {
                            match crate::road_recipe_fields::add_missing(cx.doc, &id, def, STEP) {
                                Ok((names, lists)) if !names.is_empty() => {
                                    let to_name = cx.doc.layers().get(&id).map(|n| n.name.clone()).unwrap_or_default();
                                    warnings.push(CommandWarning {
                                        code: codes::RECIPE_LAYER_FIELDS_ADDED.into(),
                                        message: crate::road_recipe_fields::added_said(&to_name, &names, &lists),
                                        path: Some("layers".into()),
                                    });
                                    domains_added = lists
                                        .iter()
                                        .filter_map(|l| match &l.went {
                                            crate::road_recipe_fields::Went::Linked => None,
                                            crate::road_recipe_fields::Went::Added => Some(l.name.clone()),
                                            crate::road_recipe_fields::Went::Renamed { here } => Some(here.clone()),
                                        })
                                        .collect();
                                    fields_added = names;
                                }
                                Ok(_) => {}
                                Err(stop) => return stop.into(),
                            }
                        }
                        // Docs/adr/4860: a layer found here keeps its own fields; the recipe's it lacks are said.
                        if how != RoadLayerMatch::Created
                            && let Some(def) = &w.layer
                            && let Some(said) = crate::road_recipe_fields::differences(cx.doc, &id, def)
                        {
                            warnings.push(CommandWarning {
                                code: codes::RECIPE_LAYER_FIELDS.into(),
                                message: said,
                                path: Some("layers".into()),
                            });
                        }
                        // A recipe from before keeps no name: the layer's here.
                        let to_name = cx.doc.layers().get(&id).map(|n| n.name.clone()).unwrap_or_default();
                        let name = w.layer.as_ref().map_or_else(|| to_name.clone(), |d| d.name.clone());
                        layers.push(RoadProducedLayer {
                            from,
                            name,
                            to: id,
                            to_name,
                            how,
                            group,
                            groups_added,
                            line_type_added,
                            domains_added,
                            forms_added,
                            fields_added,
                        });
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
    // A layer added for drawings none of which went on it is taken away again, with the groups added for it that hold
    // nothing then and the line type added for it that no layer names; one kept is said (docs/adr/3640, 4040).
    for a in &added {
        if a.used {
            warnings.push(CommandWarning {
                code: codes::RECIPE_LAYER_CREATED.into(),
                message: created_words(a),
                path: Some("layers".into()),
            });
            continue;
        }
        let _ = cx.doc.remove_layer(&a.id);
        // Docs/adr/5240: the forms added with it.
        let forms: Vec<String> = a.given.forms.iter().map(|f| f.0.clone()).collect();
        crate::road_recipe_fields::remove_forms(cx.doc, &forms);
        for (g, _) in a.groups_added.iter().rev() {
            if cx.doc.layers().get(g).is_some_and(|n| n.children.is_empty()) {
                let _ = cx.doc.remove_layer(g);
            }
        }
    }
    for t in added.iter().filter_map(|a| a.line_type_added.as_deref()) {
        let named = cx.doc.layers().leaves().iter().any(|l| l.style.line_pattern.as_deref().is_some_and(|p| same_line_type_name(p, t)));
        if !named {
            let settings = cx.doc.settings();
            let types = settings.line_types.iter().filter(|x| !same_line_type_name(&x.name, t)).cloned().collect();
            let hatches = settings.hatch_patterns.clone();
            cx.doc.set_pattern_tables(types, hatches, STEP);
        }
    }
    // Docs/adr/4860: the value lists added for layers taken away again that no field links now.
    let lists: Vec<String> = added
        .iter()
        .flat_map(|a| a.given.lists.iter().filter(|l| l.went != crate::road_recipe_fields::Went::Linked).map(|l| l.id.clone()))
        .collect();
    crate::road_recipe_fields::remove_unused(cx.doc, &lists, STEP);
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
        (doc, RoadProjectProduce { path: path.display().to_string(), drawings: None, layers: Default::default(), add_fields: false, expected_revision: None })
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
