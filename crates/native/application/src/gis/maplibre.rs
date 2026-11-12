//! A layer's look, or the visible layers', as a MapLibre GL style (docs/adr/0560, plan/gis.md CBS-17): what
//! `file.export.maplibre` writes and `file.import.maplibre` reads. The writing and reading are the formats core's
//! (`kentos_maplibre`); here, as for SLD and QML (`crate::sld`, `crate::qml`), the drawing gives each
//! layer's renderer (or its simple look as a single symbol), the project library's symbols for the renderer's
//! references, the geometry its objects have, its labels and its opacity; and a style's looks go to the layers
//! through `cad.layers.renderer`'s write, all of them one undo step “Katman stili”. The GeoJSON beside a style is
//! the host's (`kentos_headless::maplibre_export`).

use kentos_commands::{
    CommandError, CommandResult, CommandWarning, Entity, ImportMaplibre, LayerNode, LayerNodeType,
    LayersRenderer, MaplibreImported, MaplibreLayerImported,
};
use kentos_domain::Document;
use kentos_maplibre::{self as maplibre, LayerIn, StyleLayer, Written};
use serde_json::{Map, Value};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::sld::{kinds_of, library_symbol, simple_renderer};

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A layer the style is written for: its id and name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picked {
    pub id: String,
    pub name: String,
}

/// The layers to write and the visible ones left out because they hold no objects to draw.
pub struct Selection {
    pub picked: Vec<Picked>,
    /// The names of the visible layers with no point, line or area (all-layers export only).
    pub empty: Vec<String>,
}

/// Whether a layer holds a point, a line or an area (a record, a text or a dimension does not draw as MapLibre's
/// layers do).
fn has_geometry(doc: &Document, layer: &str) -> bool {
    doc.entities().any(|e| {
        e.base().layer_id == layer
            && matches!(
                e,
                Entity::Point(_)
                    | Entity::Polygon(_)
                    | Entity::Circle(_)
                    | Entity::Ellipse(_)
                    | Entity::Hatch(_)
                    | Entity::Line(_)
                    | Entity::Polyline(_)
                    | Entity::Arc(_)
                    | Entity::Spline(_)
                    | Entity::Nurbs(_)
            )
    })
}

/// The layers to write, the bottom one first: the layer given, or the visible layers that hold something to draw
/// (the layer tree lists the top one first; MapLibre draws the first one first).
pub fn pick(doc: &Document, layer: Option<&str>) -> Result<Selection, Box<CommandError>> {
    let layers = doc.layers();
    let is_layer = |n: &LayerNode| n.kind == LayerNodeType::Layer;
    match layer {
        Some(id) => {
            let Some(node) = layers.get(id) else {
                return Err(Box::new(error(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    Some("layer".into()),
                )));
            };
            if !is_layer(node) {
                return Err(Box::new(error(
                    codes::NOT_A_LAYER,
                    format!(
                        "“{}” bir katman grubu; görünüş katmanındır. Grubun bir katmanını verin ya da layer'ı boş bırakıp görünen katmanları yazdırın.",
                        node.name
                    ),
                    Some("layer".into()),
                )));
            }
            if node.service.is_some() {
                return Err(Box::new(error(
                    codes::SERVICE_LAYER,
                    format!(
                        "“{}” servisten çizilir ve nesne tutmaz; MapLibre stiline nesneleri olan katman yazılır.",
                        node.name
                    ),
                    Some("layer".into()),
                )));
            }
            Ok(Selection {
                picked: vec![Picked {
                    id: node.id.clone(),
                    name: node.name.clone(),
                }],
                empty: Vec::new(),
            })
        }
        None => {
            let visible: Vec<&LayerNode> = layers
                .leaves()
                .into_iter()
                .filter(|n| is_layer(n) && n.service.is_none() && layers.is_on(&n.id))
                .collect();
            let (full, empty): (Vec<&LayerNode>, Vec<&LayerNode>) =
                visible.into_iter().partition(|n| has_geometry(doc, &n.id));
            let mut picked: Vec<Picked> = full
                .into_iter()
                .map(|n| Picked {
                    id: n.id.clone(),
                    name: n.name.clone(),
                })
                .collect();
            picked.reverse();
            Ok(Selection {
                picked,
                empty: empty.into_iter().map(|n| n.name.clone()).collect(),
            })
        }
    }
}

/// Whether the layer has objects the GeoJSON writes as lines that the layer's fill would colour: circles, ellipses
/// and hatches.
pub fn curve_areas(doc: &Document, layer: &str) -> bool {
    doc.entities().any(|e| {
        e.base().layer_id == layer
            && matches!(e, Entity::Circle(_) | Entity::Ellipse(_) | Entity::Hatch(_))
    })
}

/// The style of `picked` over `sources`; `bind` gives each layer's source id and vector tile layer.
pub fn export(
    doc: &Document,
    picked: &[Picked],
    name: &str,
    sources: &Map<String, Value>,
    bind: &dyn Fn(&Picked) -> (String, Option<String>),
    glyphs: Option<&str>,
) -> Result<Written, Box<CommandError>> {
    let nodes: Vec<(&Picked, &LayerNode)> = picked
        .iter()
        .filter_map(|p| doc.layers().get(&p.id).map(|n| (p, n)))
        .collect();
    let renderers: Vec<Value> = nodes
        .iter()
        .map(|(_, n)| {
            n.style
                .renderer
                .clone()
                .unwrap_or_else(|| simple_renderer(&n.style))
        })
        .collect();
    let labels: Vec<(Option<Value>, Option<Value>)> = nodes
        .iter()
        .map(|(_, n)| {
            (
                n.style
                    .label
                    .as_ref()
                    .and_then(|l| serde_json::to_value(l).ok()),
                n.style
                    .labels
                    .as_ref()
                    .and_then(|l| serde_json::to_value(l).ok()),
            )
        })
        .collect();
    let binds: Vec<(String, Option<String>)> = nodes.iter().map(|(p, _)| bind(p)).collect();
    let layers: Vec<LayerIn<'_>> = nodes
        .iter()
        .enumerate()
        .map(|(i, (p, n))| LayerIn {
            id: &p.id,
            name: &p.name,
            renderer: &renderers[i],
            kinds: kinds_of(doc, &p.id),
            opacity: n
                .compose
                .as_ref()
                .and_then(|c| c.opacity)
                .map_or(1.0, |o| o.clamp(0.0, 1.0)),
            source: &binds[i].0,
            source_layer: binds[i].1.as_deref(),
            label: labels[i].0.as_ref(),
            labels: labels[i].1.as_ref(),
        })
        .collect();
    let mut written = maplibre::write(name, &layers, sources, glyphs, &|id| {
        library_symbol(doc, id)
    });
    let many = nodes.len() > 1;
    for (p, n) in &nodes {
        let who = if many {
            format!("“{}” katmanı: ", p.name)
        } else {
            String::new()
        };
        if let Some(c) = &n.compose
            && (c
                .blend
                .is_some_and(|b| b != kentos_commands::BlendMode::Normal)
                || c.mask.is_some())
        {
            written.lost.push(format!(
                "{who}Katman çiziminin karışım kipi ve maskesi MapLibre stiline yazılmadı (saydamlığı simgelere katıldı)"
            ));
        }
        if written.empty.contains(&p.id) && many {
            written.lost.push(format!(
                "{who}görünüşünde MapLibre'nin katmanlarıyla çizilebilecek bir sınıf ya da kural yok; katman yazılmadı"
            ));
        }
    }
    if written.layers == 0 {
        return Err(Box::new(error(
            codes::NOTHING_TO_EXPORT,
            format!(
                "Görünüşte MapLibre'nin katmanlarıyla çizilebilecek bir sınıf ya da kural yok: {}",
                if written.lost.is_empty() {
                    "yazılacak katman kalmadı".to_owned()
                } else {
                    written.lost.join("; ")
                }
            ),
            Some("layer".into()),
        )));
    }
    Ok(written)
}

// ─── Import ───────────────────────────────────────────────────────────────────────────────────────────

fn text_of(input: &ImportMaplibre) -> Result<String, Stop> {
    match (&input.path, &input.text) {
        (Some(_), Some(_)) | (None, None) => Err(refuse(
            codes::INVALID_INPUT,
            "Stilin ya yolunu (path) ya da kendisini (text) verin, ikisini birden değil.".into(),
            "path",
        )),
        (None, Some(t)) => Ok(t.clone()),
        (Some(p), None) => std::fs::read(p)
            .map_err(|e| {
                refuse(
                    codes::FILE_UNREADABLE,
                    format!("“{p}” okunamadı: {e}. Dosyanın yerinde olduğunu denetleyin."),
                    "path",
                )
            })
            .map(|b| String::from_utf8_lossy(&b).into_owned()),
    }
}

/// The style layers a KentOS layer is the target of.
fn matching<'a>(
    style: &'a maplibre::Style,
    node: &LayerNode,
    from: Option<&str>,
) -> Vec<&'a StyleLayer> {
    let names = |l: &StyleLayer, x: &str| {
        l.source_layer.as_deref() == Some(x)
            || l.id == x
            || l.id.strip_prefix(x).is_some_and(|r| r.starts_with('/'))
    };
    style
        .layers
        .iter()
        .filter(|l| match from {
            Some(f) => names(l, f),
            None => match l.kentos_layer() {
                Some(k) => k == node.id,
                None => names(l, &node.id) || names(l, &node.name),
            },
        })
        .collect()
}

/// What `run` reads: the layers' looks and what each lost.
struct Taken {
    layer: String,
    name: String,
    from: Vec<String>,
    read: kentos_formats::sld::Read,
}

fn read(doc: &Document, input: &ImportMaplibre) -> Result<(Vec<Taken>, Vec<String>), Stop> {
    let text = text_of(input)?;
    let style =
        maplibre::parse(&text).map_err(|why| refuse(codes::INVALID_MAPLIBRE, why, "path"))?;
    let layers = doc.layers();
    let targets: Vec<&LayerNode> = match &input.layer {
        Some(id) => {
            let Some(node) = layers.get(id) else {
                return Err(refuse(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    "layer",
                ));
            };
            if node.kind != LayerNodeType::Layer {
                return Err(refuse(
                    codes::NOT_A_LAYER,
                    format!(
                        "“{}” bir katman grubu; görünüş katmanındır. Grubun bir katmanını verin.",
                        node.name
                    ),
                    "layer",
                ));
            }
            if node.service.is_some() {
                return Err(refuse(
                    codes::SERVICE_LAYER,
                    format!(
                        "“{}” servisten çizilir ve nesne tutmaz; görünüş nesneleri olan katmanın olur.",
                        node.name
                    ),
                    "layer",
                ));
            }
            vec![node]
        }
        None => layers
            .leaves()
            .into_iter()
            .filter(|n| n.kind == LayerNodeType::Layer && n.service.is_none())
            .collect(),
    };
    let many = input.layer.is_none();
    let mut taken = Vec::new();
    let mut used: Vec<&str> = Vec::new();
    let mut lost: Vec<String> = Vec::new();
    for node in targets {
        let matched = matching(&style, node, input.from.as_deref());
        if matched.is_empty() {
            continue;
        }
        used.extend(matched.iter().map(|l| l.id.as_str()));
        let read = maplibre::renderer_of_at(&matched, doc.settings().plot_scale).map_err(|why| {
            refuse(
                codes::INVALID_MAPLIBRE,
                format!("“{}” katmanının stili okunamadı: {why}", node.name),
                "path",
            )
        })?;
        if !read.renderer.is_null()
            && let Some(why) = crate::layers_renderer::renderer_problem(&read.renderer)
        {
            return Err(refuse(
                codes::INVALID_MAPLIBRE,
                format!(
                    "“{}” katmanının stili KentOS'un görünüşü olamadı: {why}",
                    node.name
                ),
                "path",
            ));
        }
        taken.push(Taken {
            layer: node.id.clone(),
            name: node.name.clone(),
            from: matched.iter().map(|l| l.id.clone()).collect(),
            read,
        });
    }
    if taken.is_empty() {
        return Err(refuse(
            codes::STYLE_LAYER_NOT_FOUND,
            match (&input.layer, &input.from) {
                (_, Some(f)) => format!(
                    "Stilde “{f}” adlı (source-layer ya da id) bir katman yok. Stilin katmanlarının adlarına bakın."
                ),
                (Some(id), None) => format!(
                    "Stilde “{id}” katmanı için bir stil katmanı yok: metadata kentos:layer, source-layer ya da id katmanın kimliği ya da adı olmalı; başka bir ad için from verin."
                ),
                (None, None) => "Stilde çizimin hiçbir katmanı için bir stil katmanı yok: metadata kentos:layer, source-layer ya da id katmanların kimliği ya da adı olmalı.".into(),
            },
            "path",
        ));
    }
    if many {
        let left: Vec<&str> = style
            .layers
            .iter()
            .map(|l| l.id.as_str())
            .filter(|id| !used.contains(id))
            .collect();
        if !left.is_empty() {
            lost.push(format!(
                "{} stil katmanı çizimin hiçbir katmanına denk gelmedi, alınmadı: {}",
                left.len(),
                left.join(", ")
            ));
        }
    }
    Ok((taken, lost))
}

fn warnings_of(lost: &[String]) -> Vec<CommandWarning> {
    if lost.is_empty() {
        return Vec::new();
    }
    vec![CommandWarning {
        code: codes::IMPORT_LOSSES.into(),
        message: format!("Stilin bazı parçaları alınamadı: {}.", lost.join("; ")),
        path: Some("path".into()),
    }]
}

fn changed_of<T>(r: CommandResult<T>, changed: impl Fn(&T) -> bool) -> Result<bool, Stop> {
    match r {
        CommandResult::Completed { output, .. } => Ok(changed(&output)),
        CommandResult::Failed { error } => Err(Stop::failed(error)),
        CommandResult::Conflict { error } => Err(Stop::conflict(error)),
        _ => Err(Stop::Cancelled),
    }
}

fn run(
    cx: &mut ExecutionContext<'_>,
    input: &ImportMaplibre,
    write: bool,
) -> Result<(MaplibreImported, Vec<CommandWarning>), Stop> {
    let (taken, mut lost) = read(cx.doc, input)?;
    checks::revision(cx.doc, input.expected_revision.as_deref())?;
    let many = taken.len() > 1 || input.layer.is_none();
    let mut layers = Vec::new();
    let mut any = false;
    for t in taken {
        let labelled = match &t.read.labels {
            Some(l) => Some(crate::sld::labelling_in(l).map_err(|why| {
                refuse(
                    codes::INVALID_MAPLIBRE,
                    format!(
                        "“{}” katmanının etiketleri KentOS'un etiketlemesi olamadı: {why}",
                        t.name
                    ),
                    "path",
                )
            })?),
            None => None,
        };
        let mut changed = false;
        if !t.read.renderer.is_null() {
            let renderer_input = LayersRenderer {
                layer: t.layer.clone(),
                renderer: Some(t.read.renderer.clone()),
                expected_revision: None,
            };
            changed |= if write {
                changed_of(crate::layers_renderer::execute(cx, renderer_input), |o| {
                    o.changed
                })?
            } else {
                changed_of(crate::layers_renderer::plan(cx, &renderer_input), |o| {
                    o.changed
                })?
            };
        }
        if let Some((label, labels)) = labelled.clone() {
            changed |= crate::sld::apply_labelling(cx, &t.layer, label, labels, write)?;
        }
        let (label, labels) = labelled.map_or((None, None), |(a, b)| (a, Some(b)));
        any |= changed;
        lost.extend(t.read.lost.iter().map(|l| {
            if many {
                format!("“{}” katmanı: {l}", t.name)
            } else {
                l.clone()
            }
        }));
        layers.push(MaplibreLayerImported {
            kind: t.read.renderer["type"].as_str().unwrap_or("").to_owned(),
            rules: t.read.rules as u32,
            renderer: t.read.renderer,
            labels,
            label,
            layer: t.layer,
            from: t.from,
            changed,
        });
    }
    Ok((
        MaplibreImported {
            layers,
            changed: any,
            lost: lost.clone(),
            revision: cx.doc.revision().to_string(),
        },
        warnings_of(&lost),
    ))
}

/// Checks `input` against the drawing, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ImportMaplibre) -> CommandResult<()> {
    let mut copy = cx.doc.clone();
    let mut cx = ExecutionContext::new(&mut copy);
    match run(&mut cx, input, false) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would make the layers' looks; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ImportMaplibre) -> CommandResult<MaplibreImported> {
    let mut copy = cx.doc.clone();
    let mut cx = ExecutionContext::new(&mut copy);
    match run(&mut cx, input, false) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// Makes the style's looks the layers', one undo step “Katman stili” when any differs.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ImportMaplibre,
) -> CommandResult<MaplibreImported> {
    // One step of the drawing and its own row in the journal (docs/adr/0253 §3, 0548 §4).
    crate::call::recorded(
        cx,
        kentos_commands::FILE_IMPORT_MAPLIBRE,
        kentos_commands::FILE_IMPORT_MAPLIBRE_VERSION,
        input,
        written_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step and journals it.
fn written_execute(
    cx: &mut ExecutionContext<'_>,
    input: ImportMaplibre,
) -> CommandResult<MaplibreImported> {
    match run(cx, &input, true) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}
