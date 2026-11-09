//! `road.project.graph` v1 (plan/map-cad.md MAP-20, docs/adr/1940): İş grafiği, read only. The project tree's rows
//! (`road_project::tree`, deep or not) with the layers its surfaces are made of before them and the sheets that show a
//! drawing after them:
//!
//! - a layer of a surface's points or breaklines `builds` it (each layer once, in the surfaces' order);
//! - a row is made on its parent in the tree (`basis`: a profile on its alignment, a pad on its surface …); a drawing's
//!   parent is its main source, said by its `source` edges instead;
//! - every source a drawing keeps is a `source` edge, its stamp then and now (`road_drawings::stamp_now`);
//! - a sheet's fixed-view map whose ground meets one of a drawing's objects `shows` it, as the print check reads it
//!   (docs/adr/1720 §5): an atlas map shows another ground on each page and is left out. A sheet that shows none is
//!   not in the graph; one that shows a drawing not up to date is stale, with the print check's reasons.
//!
//! Every drawing carries its recipe: the command, the version that draws it again, the input as kept, its objects
//! then and now and their stamp. The project's system, epoch and heights come as the field package says them.

use kentos_commands::{
    CommandResult, RoadDrawingDef, RoadDrawingRecipe, RoadEdgeKind as E, RoadGraphEdge, RoadGraphKind as G, RoadGraphNode,
    RoadNodeKind as N, RoadNodeState as S, RoadProjectGraph, RoadProjectGraphQuery, RoadSourceKind as K,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::road_drawings;

fn kind_of(n: N) -> G {
    match n {
        N::Surface => G::Surface,
        N::Alignment => G::Alignment,
        N::Profile => G::Profile,
        N::Sections => G::Sections,
        N::Superelevation => G::Superelevation,
        N::Junction => G::Junction,
        N::Template => G::Template,
        N::Corridor => G::Corridor,
        N::Volume => G::Volume,
        N::Haul => G::Haul,
        N::Guardrail => G::Guardrail,
        N::Culvert => G::Culvert,
        N::Wall => G::Wall,
        N::Platform => G::Platform,
        N::SheetSet => G::SheetSet,
        N::Pay => G::Pay,
        N::Network => G::Network,
        N::Drawing => G::Drawing,
    }
}

fn source_kind(k: K) -> G {
    match k {
        K::Surface => G::Surface,
        K::Alignment => G::Alignment,
        K::Profile => G::Profile,
        K::Template => G::Template,
        K::Sections => G::Sections,
        K::Superelevation => G::Superelevation,
        K::Corridor => G::Corridor,
        K::Guardrail => G::Guardrail,
        K::Culvert => G::Culvert,
        K::Wall => G::Wall,
        K::Junction => G::Junction,
        K::Platform => G::Platform,
        K::Network => G::Network,
    }
}

/// A fixed-view map's ground as a box (low, high corners); none for another item or an atlas map (docs/adr/1720 §5).
pub fn map_ground(item: &kentos_sheet::model::Item) -> Option<([f64; 2], [f64; 2])> {
    let corners = kentos_sheet::display::fixed_corners(item)?;
    Some(corners.iter().fold(([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]), |(lo, hi), c| {
        ([lo[0].min(c[0]), lo[1].min(c[1])], [hi[0].max(c[0]), hi[1].max(c[1])])
    }))
}

/// Whether one of a drawing's objects there now lies within the ground `lo`–`hi`: its bounds meet the box.
pub fn drawing_within(doc: &Document, d: &RoadDrawingDef, (lo, hi): ([f64; 2], [f64; 2])) -> bool {
    d.uids.iter().any(|u| {
        Uuid::parse_str(u).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s)).is_some_and(|e| {
            let b = kentos_geometry_core::entity::entity_bounds(&crate::geometry::shape(e));
            b.max_x >= lo[0] && b.min_x <= hi[0] && b.max_y >= lo[1] && b.min_y <= hi[1]
        })
    })
}

/// The definition of the layer drawing `d` went on (docs/adr/3640): its id, name and look; none when the layer is gone
/// or not a layer. Docs/adr/4040: the groups it is in (from the top), its whole style and the project line type the
/// style names, as this project defines it (none: it names none, or the project has no line type of that name).
/// Docs/adr/4440: its opacity and blending mode (a mask is not kept: it cuts into this project's layers by their ids),
/// its type and, when it is not printed, its Yazdır.
fn recipe_layer(doc: &Document, d: &RoadDrawingDef) -> Option<kentos_commands::RoadRecipeLayer> {
    let id = d.input.get("layerId")?.as_str()?;
    let layers = doc.layers();
    let node = layers.get(id)?;
    if node.kind != kentos_commands::LayerNodeType::Layer {
        return None;
    }
    let mut group = Vec::new();
    let mut at = layers.place_of(id).and_then(|(parent, _)| parent);
    while let Some(g) = at {
        group.push(layers.get(&g).map(|n| n.name.clone()).unwrap_or_default());
        at = layers.place_of(&g).and_then(|(parent, _)| parent);
    }
    group.reverse();
    let line_type_def = node.style.line_pattern.as_deref().and_then(|name| {
        doc.settings().line_types.iter().find(|t| kentos_commands::same_line_type_name(&t.name, name)).cloned()
    });
    Some(kentos_commands::RoadRecipeLayer {
        id: id.to_owned(),
        name: node.name.clone(),
        color: node.style.color.clone(),
        line_type: node.style.line_type,
        line_weight: node.style.line_weight,
        group,
        style: Some(node.style.clone()),
        line_type_def,
        compose: node.compose.as_ref().and_then(|c| {
            kentos_commands::LayerCompose { opacity: c.opacity, blend: c.blend, mask: None }.normalized()
        }),
        geometry: node.geometry,
        plot: node.plot.filter(|printed| !printed),
    })
}

fn recipe_of(doc: &Document, d: &RoadDrawingDef) -> RoadDrawingRecipe {
    let (_, present) = road_drawings::objects_stamp(doc, &d.uids);
    RoadDrawingRecipe {
        command: d.command.clone(),
        version: road_drawings::version_of(&d.command),
        input: d.input.clone(),
        written: u32::try_from(d.uids.len()).unwrap_or(u32::MAX),
        objects: u32::try_from(present).unwrap_or(u32::MAX),
        digest: d.digest.clone(),
        made: d.version,
        revision: d.revision.clone(),
        layer: recipe_layer(doc, d),
    }
}

fn edge(from: usize, to: usize, kind: E) -> RoadGraphEdge {
    RoadGraphEdge {
        from: u32::try_from(from).unwrap_or(u32::MAX),
        to: u32::try_from(to).unwrap_or(u32::MAX),
        kind,
        main: false,
        kept: None,
        now: None,
        map: None,
    }
}

/// The project's graph now.
pub(crate) fn graph(doc: &Document, deep: bool) -> Result<RoadProjectGraph, Stop> {
    let tree = crate::road_project::tree(doc, deep)?;
    let surfaces = crate::road_terrains::surfaces(doc);
    let layers_of = |id: &str| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Some(def) = surfaces.iter().find(|s| s.id == id) {
            for l in def.points.layers.iter().chain(&def.breaklines.layers) {
                if !out.contains(l) {
                    out.push(l.clone());
                }
            }
        }
        out
    };
    let mut nodes: Vec<RoadGraphNode> = Vec::new();
    // The layers the surfaces are made of, first.
    let mut layers: Vec<String> = Vec::new();
    for s in surfaces {
        for l in layers_of(&s.id) {
            if !layers.contains(&l) {
                layers.push(l);
            }
        }
    }
    for l in &layers {
        let (name, state, why) = match doc.layers().get(l) {
            Some(n) => (n.name.clone(), S::Current, Vec::new()),
            None => (l.clone(), S::Gone, vec![format!("“{l}” katmanı çizimde yok.")]),
        };
        nodes.push(RoadGraphNode { kind: G::Layer, id: l.clone(), name, state, why, recipe: None });
    }
    let base = nodes.len();
    let drawings = doc.road_drawings();
    let drawing_of = |id: &str| drawings.iter().find(|d| d.id == id);
    for n in &tree.nodes {
        let recipe = if n.kind == N::Drawing { drawing_of(&n.id).map(|d| recipe_of(doc, d)) } else { None };
        nodes.push(RoadGraphNode {
            kind: kind_of(n.kind),
            id: n.id.clone(),
            name: n.name.clone(),
            state: n.state,
            why: n.why.clone(),
            recipe,
        });
    }
    let mut edges: Vec<RoadGraphEdge> = Vec::new();
    for (k, n) in tree.nodes.iter().enumerate() {
        if n.kind == N::Surface {
            for l in layers_of(&n.id) {
                if let Some(from) = layers.iter().position(|x| *x == l) {
                    edges.push(edge(from, base + k, E::Builds));
                }
            }
        }
    }
    for (k, n) in tree.nodes.iter().enumerate() {
        if let Some(p) = n.parent
            && n.kind != N::Drawing
        {
            edges.push(edge(base + p as usize, base + k, E::Basis));
        }
    }
    for (k, n) in tree.nodes.iter().enumerate().filter(|(_, n)| n.kind == N::Drawing) {
        let Some(d) = drawing_of(&n.id) else { continue };
        for (i, s) in d.sources.iter().enumerate() {
            let Some(from) = nodes.iter().position(|x| x.kind == source_kind(s.kind) && x.id == s.id) else { continue };
            edges.push(RoadGraphEdge {
                main: i == 0,
                kept: Some(s.digest.clone()),
                now: road_drawings::stamp_now(doc, s.kind, &s.id),
                ..edge(from, base + k, E::Source)
            });
        }
    }
    // The sheets: a document holds only a book that reads (Document::set_sheets checks it).
    if let Some((book, _)) = doc.sheets().and_then(|s| kentos_sheet::project::read_project_sheets(s).ok()) {
        for sheet in &book.sheets {
            let Ok(scene) = kentos_sheet::scene::Scene::new(&book, &sheet.id) else { continue };
            let mut shows: Vec<(usize, String)> = Vec::new();
            for (item, _) in scene.all() {
                let Some(ground) = map_ground(item) else { continue };
                for (k, n) in tree.nodes.iter().enumerate().filter(|(_, n)| n.kind == N::Drawing) {
                    if drawing_of(&n.id).is_some_and(|d| drawing_within(doc, d, ground)) {
                        shows.push((base + k, item.name.clone()));
                    }
                }
            }
            if shows.is_empty() {
                continue;
            }
            let why: Vec<String> = shows
                .iter()
                .filter_map(|(k, map)| {
                    let n = &nodes[*k];
                    crate::road_project::outdated_word(n.state)
                        .map(|word| format!("“{map}” haritası {} çizimini gösteriyor ve çizim {word}.", n.name))
                })
                .collect();
            let at = nodes.len();
            nodes.push(RoadGraphNode {
                kind: G::Sheet,
                id: sheet.id.clone(),
                name: sheet.name.clone(),
                state: if why.is_empty() { S::Current } else { S::Stale },
                why,
                recipe: None,
            });
            for (from, map) in shows {
                edges.push(RoadGraphEdge { map: Some(map), ..edge(from, at, E::Shows) });
            }
        }
    }
    let count = |f: &dyn Fn(&RoadGraphNode) -> bool| u32::try_from(nodes.iter().filter(|n| f(n)).count()).unwrap_or(u32::MAX);
    let drawings = count(&|n| n.kind == G::Drawing);
    let outdated = count(&|n| n.kind == G::Drawing && crate::road_project::outdated_word(n.state).is_some());
    let sheets = count(&|n| n.kind == G::Sheet);
    let blocked = count(&|n| n.kind == G::Sheet && n.state == S::Stale);
    let settings = doc.settings();
    let facts = kentos_project::facts::coordinate_facts(settings);
    let system = (!facts.epsg.is_empty()).then(|| kentos_project::crs::system(settings.srid).map(|s| s.name.clone())).flatten();
    Ok(RoadProjectGraph {
        nodes,
        edges,
        drawings,
        outdated,
        sheets,
        blocked,
        epsg: (!facts.epsg.is_empty()).then_some(facts.epsg),
        system,
        epoch: facts.epoch,
        heights: (!facts.heights.is_empty()).then_some(facts.heights),
        deep,
        revision: doc.revision().to_string(),
    })
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &RoadProjectGraphQuery) -> CommandResult<()> {
    match graph(cx.doc, input.deep) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &RoadProjectGraphQuery) -> CommandResult<RoadProjectGraph> {
    match graph(cx.doc, input.deep) {
        Ok(output) => CommandResult::Completed { output, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RoadProjectGraphQuery) -> CommandResult<RoadProjectGraph> {
    plan(cx, &input)
}

/// The graph for a window, or why it does not read.
pub fn preview(doc: &Document, deep: bool) -> Result<RoadProjectGraph, String> {
    graph(doc, deep).map_err(crate::road_corridors::words)
}
