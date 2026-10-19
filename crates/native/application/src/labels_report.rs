//! `cad.labels.report` v1 (docs/adr/0533 §6): how the label engine places a window's labels at a scale, counted:
//! placed, left without a place, hidden by hand, pinned, forced over another, boxes that really cross, texts that
//! repeat. It asks the same engine the drawing area, a sheet's map and its PDF ask (`Store::place_labels`), over a
//! store of the document it makes itself (the labels' layers, texts and pins as the desktop's `Spatial` sends them),
//! and writes nothing. A bound layer that holds a part of its table (docs/adr/0510) says so: its labels and its
//! obstacles are those of the part. The shared cases are `fixtures/commands/v1/cad.labels.report.json`
//! (`scripts/fixtures/labels_report_cases.py`).

use std::collections::HashMap;

use kentos_contracts::{
    CommandResult, LabelCounts, LabelsReport, LabelsReported, LayerLabelReport, LayerNode,
    LayerNodeType,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::label_audit::{AuditedLabel, audit};
use kentos_geometry_core::store::placing::{ObjectLabels, Pin, PlaceOptions};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::label_texts::{label_defaults_json, label_layers_json, layer_texts, object_texts};

/// A CSS pixel of the paper, millimetres: the unit a sheet's map lays its labels out in (`kentos_plot::maps::PX_MM`).
const PAPER_PX_MM: f64 = 25.4 / 96.0;
/// The most pixels a window may have: the engine places in a page this size.
const MOST_PIXELS: f64 = 1.0e8;
/// The largest paper scale a report takes.
const MOST_SCALE: u32 = 1_000_000;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The scale as pixels per metre and the window, checked in the contract's order; the layers asked for.
struct Asked {
    window: Option<Bounds>,
    scale: f64,
}

fn check(doc: &Document, input: &LabelsReport) -> Result<Asked, Stop> {
    let scale = match (input.px_per_metre, input.paper_scale) {
        (Some(px), None) if px.is_finite() && px > 0.0 => px,
        (None, Some(n)) if (1..=MOST_SCALE).contains(&n) => 1000.0 / (PAPER_PX_MM * f64::from(n)),
        (Some(_), None) => {
            return Err(fail(
                codes::INVALID_SCALE,
                "Piksel bölü metre sıfırdan büyük, sonlu bir sayı olmalı.".into(),
                "pxPerMetre",
            ));
        }
        (None, Some(_)) => {
            return Err(fail(
                codes::INVALID_SCALE,
                format!(
                    "Pafta ölçeği 1 ile {MOST_SCALE} arasında bir tam sayı olmalı (1:N'nin N'si)."
                ),
                "paperScale",
            ));
        }
        _ => {
            return Err(fail(
                codes::INVALID_SCALE,
                "Ölçek pxPerMetre ya da paperScale'den tam biri verilmeli.".into(),
                "pxPerMetre",
            ));
        }
    };
    let window = match input.window {
        Some(w) => {
            if let Some(why) = kentos_contracts::extent_problem(&w) {
                return Err(fail(
                    codes::INVALID_WINDOW,
                    format!("Pencerenin {why}."),
                    "window",
                ));
            }
            if (w[2] - w[0]) * scale * (w[3] - w[1]) * scale > MOST_PIXELS {
                return Err(fail(
                    codes::INVALID_WINDOW,
                    "Pencere bu ölçekte 100 milyon pikselden büyük; daha küçük bir pencere ya da ölçek verin."
                        .into(),
                    "window",
                ));
            }
            Some(Bounds {
                min_x: w[0],
                min_y: w[1],
                max_x: w[2],
                max_y: w[3],
            })
        }
        None => None,
    };
    for (i, id) in input.layers.iter().flatten().enumerate() {
        let path = format!("layers[{i}]");
        let Some(node) = doc.layers().get(id) else {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{id}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                &path,
            ));
        };
        if node.kind != LayerNodeType::Layer {
            return Err(fail(
                codes::NOT_A_LAYER,
                format!(
                    "“{}” bir katman grubu; grubun bir katmanını verin.",
                    node.name
                ),
                &path,
            ));
        }
        if node.service.is_some() {
            return Err(fail(
                codes::SERVICE_LAYER,
                format!(
                    "“{}” servisten çizilir ve nesne tutmaz; etiketi olmaz.",
                    node.name
                ),
                &path,
            ));
        }
    }
    Ok(Asked { window, scale })
}

/// The store of `doc` with what the label engine reads: the layers' labelling, the objects' texts and pins.
fn label_store(doc: &Document) -> Store {
    let mut store = crate::geometry_store::store_of(doc);
    // The kinds' own label styles hold where a layer has none.
    let _ = store.set_label_defaults_json(&label_defaults_json());
    let leaves: Vec<&LayerNode> = doc
        .layers()
        .leaves()
        .into_iter()
        .filter(|l| l.service.is_none())
        .collect();
    let _ = store.set_label_layers_json(&label_layers_json(&leaves));
    let ids: Vec<f64> = doc
        .text_labelled()
        .iter()
        .map(|slot| f64::from(slot.0))
        .collect();
    store.set_text_labelled(&ids);
    let unit = crate::expression_unit::reading_unit(doc.settings());
    let names = |id: &str| {
        doc.layers()
            .get(id)
            .map_or_else(|| id.to_owned(), |n| n.name.clone())
    };
    for l in &leaves {
        let list: Vec<&kentos_contracts::Entity> = doc
            .layer_slots(&l.id)
            .filter_map(|(_, s)| doc.get(s))
            .collect();
        let t = layer_texts(Some(l), unit);
        let texts = object_texts(&list, &t, &names, &|i| {
            store.get(f64::from(list[i].base().id)).map(|it| &it.shape)
        });
        let given: Vec<(f64, ObjectLabels)> = list
            .iter()
            .zip(texts)
            .map(|(e, (texts, z))| (f64::from(e.base().id), ObjectLabels { texts, z }))
            .collect();
        for (id, labels) in given {
            store.set_object_labels(id, labels);
        }
    }
    for e in doc.entities() {
        let pins = &e.base().label_pins;
        if !pins.is_empty() {
            store.set_label_pins(
                f64::from(e.base().id),
                pins.iter()
                    .map(|p| Pin {
                        class: p.class.clone(),
                        at: p.at.map(|a| Vec2::new(a.x, a.y)),
                        rotation: p.rotation.unwrap_or(0.0),
                        hidden: p.hidden == Some(true),
                    })
                    .collect(),
            );
        }
    }
    store
}

fn counts(labels: &[&AuditedLabel]) -> LabelCounts {
    let n = |f: &dyn Fn(&AuditedLabel) -> bool| labels.iter().filter(|l| f(l)).count() as u32;
    LabelCounts {
        placed: n(&|l| l.placed()),
        unplaced: n(&|l| l.unplaced()),
        hidden: n(&|l| l.hidden()),
        pinned: n(&|l| l.placed() && l.pinned()),
        forced: n(&|l| l.placed() && l.forced()),
        collisions: n(&|l| l.collides),
        repeats: n(&|l| l.repeats),
    }
}

/// The report; the same answer as [`execute`].
fn report(doc: &Document, input: &LabelsReport) -> Result<LabelsReported, Stop> {
    let asked = check(doc, input)?;
    let store = label_store(doc);
    let window = asked.window.or_else(|| store.extent(None));
    let Some(window) = window else {
        return Ok(LabelsReported {
            window: [0.0; 4],
            px_per_metre: asked.scale,
            totals: LabelCounts::default(),
            layers: Vec::new(),
            revision: doc.revision().to_string(),
        });
    };
    // The drawing's extent of one point (or a line along an axis) is no window: it takes a little room.
    let window = if window.max_x > window.min_x && window.max_y > window.min_y {
        window
    } else {
        Bounds {
            min_x: window.min_x - 1.0,
            min_y: window.min_y - 1.0,
            max_x: window.max_x + 1.0,
            max_y: window.max_y + 1.0,
        }
    };
    let shown = store.place_labels(
        &window,
        asked.scale,
        &[],
        PlaceOptions {
            unplaced: true,
            hidden: true,
        },
    );
    // The layers by their place in the tree, once.
    let leaves: Vec<&LayerNode> = doc.layers().leaves();
    let place: HashMap<&str, u32> = leaves
        .iter()
        .enumerate()
        .map(|(i, l)| (l.id.as_str(), i as u32))
        .collect();
    let layer_index = move |id: f64| -> u32 {
        doc.get(Slot(id as u32))
            .and_then(|e| place.get(e.base().layer_id.as_str()).copied())
            .unwrap_or(u32::MAX)
    };
    let audited = audit(&shown, asked.scale, &layer_index);
    let wanted: Option<std::collections::HashSet<&str>> = input
        .layers
        .as_ref()
        .map(|l| l.iter().map(String::as_str).collect());
    let mut by_layer: Vec<(usize, Vec<&AuditedLabel>)> = Vec::new();
    for l in &audited {
        // The window's labels: the engine places a margin round it too, to keep the labels at the edge steady.
        if l.at[0] < window.min_x
            || l.at[0] > window.max_x
            || l.at[1] < window.min_y
            || l.at[1] > window.max_y
        {
            continue;
        }
        let Some(e) = doc.get(Slot(l.id as u32)) else {
            continue;
        };
        let id = e.base().layer_id.as_str();
        if wanted.as_ref().is_some_and(|w| !w.contains(id)) {
            continue;
        }
        let Some(at) = leaves.iter().position(|n| n.id == id) else {
            continue;
        };
        match by_layer.iter_mut().find(|(i, _)| *i == at) {
            Some((_, list)) => list.push(l),
            None => by_layer.push((at, vec![l])),
        }
    }
    by_layer.sort_by_key(|(i, _)| *i);
    let every: Vec<&AuditedLabel> = by_layer
        .iter()
        .flat_map(|(_, l)| l.iter().copied())
        .collect();
    let layers = by_layer
        .into_iter()
        .map(|(i, list)| {
            let node = leaves[i];
            let link = doc.settings().sources.iter().find(|s| s.layer == node.id);
            let part = link.and_then(|s| s.extent);
            LayerLabelReport {
                layer: node.id.clone(),
                counts: counts(&list),
                source: link.map(|s| s.id.clone()),
                part,
                partial: part.is_some_and(|e| {
                    window.min_x < e[0]
                        || window.min_y < e[1]
                        || window.max_x > e[2]
                        || window.max_y > e[3]
                }),
            }
        })
        .collect();
    Ok(LabelsReported {
        window: [window.min_x, window.min_y, window.max_x, window.max_y],
        px_per_metre: asked.scale,
        totals: counts(&every),
        layers,
        revision: doc.revision().to_string(),
    })
}

/// Checks `input`.
pub fn validate(cx: &ExecutionContext<'_>, input: &LabelsReport) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The counts; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LabelsReport) -> CommandResult<LabelsReported> {
    answer(cx.doc, input)
}

/// The counts. Writes nothing.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: LabelsReport,
) -> CommandResult<LabelsReported> {
    answer(cx.doc, &input)
}

fn answer(doc: &Document, input: &LabelsReport) -> CommandResult<LabelsReported> {
    match report(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
