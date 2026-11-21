//! `cad.layers.classes` v1 (docs/adr/9105 §2): a layer's style classes as the layer tree lists them, with `count` how
//! many objects each takes by the drawing's own rule (`kentos_scene_query::style_classes`). A layer's objects are its
//! drawing's, those its filter shows; a data layer's are its table's rows, read from the source once a part at a time,
//! only the fields the classes read (and the geometry when one reads it). Nothing is written.
//!
//! [`data_counts`] and [`data_class_keys`] are the desktop's too: the tree's counts and Bu sınıfın nesnelerini seç,
//! off the interface's thread.

use kentos_commands::{CommandResult, LayerClass, LayerClassesAnswer, LayersClasses};
use kentos_data::{Cancel, Feature, Request};
use kentos_domain::Document;
use kentos_scene_query::style_classes::{ClassReader, classes_kind, style_classes};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geopackage::{attribute_text, object_of};
use crate::sources::TableSystem;

/// Rows asked of the source at a time.
const CHUNK: usize = 2048;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn unreachable(e: impl std::fmt::Display) -> Stop {
    Stop::failed(error(codes::SOURCE_UNREACHABLE, format!("Kaynak okunamadı: {e}"), None))
}

/// The layer the input names, checked.
fn found<'d>(doc: &'d Document, input: &LayersClasses) -> Result<&'d kentos_commands::LayerNode, Stop> {
    let layer = input.layer.trim();
    if layer.is_empty() {
        return Err(fail(codes::NO_LAYER, "Sınıfları okunacak katmanı (layer) verin.".into(), "layer"));
    }
    let node = doc
        .layers()
        .get(layer)
        .ok_or_else(|| fail(codes::LAYER_NOT_FOUND, format!("“{layer}” kimlikli katman çizimde yok."), "layer"))?;
    if node.kind == kentos_commands::LayerNodeType::Group {
        return Err(fail(
            codes::NOT_A_LAYER,
            format!("“{}” bir katman grubu; sınıflar katmanın stilinindir. Grubun bir katmanını verin.", node.name),
            "layer",
        ));
    }
    if node.service.is_some() {
        return Err(fail(
            codes::SERVICE_LAYER,
            format!("“{}” bir harita servisinden çizilir; sınıfları yoktur.", node.name),
            "layer",
        ));
    }
    Ok(node)
}

/// How many objects each class takes and how many the layer has: a data layer's rows read from its source; the
/// message of why they could not be read.
pub fn data_counts(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
    stopped: &dyn Fn() -> bool,
) -> Result<(Vec<Result<u64, String>>, u64), String> {
    counted(node, settings, stopped).map_err(message)
}

/// Why a stop stopped, in words.
fn message(stop: Stop) -> String {
    match CommandResult::<()>::from(stop) {
        CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
            error.message
        }
        _ => "Kaynak okunamadı.".to_owned(),
    }
}

fn counted(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
    stopped: &dyn Fn() -> bool,
) -> Result<(Vec<Result<u64, String>>, u64), Stop> {
    let unit = crate::expression_unit::reading_unit(settings);
    let Some((renderer, reader)) = node
        .style
        .renderer
        .as_ref()
        .and_then(|r| ClassReader::new(r, unit).map(|reader| (r, reader)))
    else {
        // Without classes only the rows are counted, by the source.
        return Ok((Vec::new(), rows_counted(node)?));
    };
    let mut counts: Vec<Result<u64, String>> = Vec::new();
    let mut total = 0u64;
    scan(node, settings, &reader, stopped, &mut |objects| {
        let refs: Vec<&kentos_commands::Entity> = objects.iter().map(|(_, e)| e).collect();
        let shapes = shapes_of(&reader, &refs);
        let part = reader.counts(&refs, &|_| node.name.clone(), &|i| shapes.get(i));
        total += refs.len() as u64;
        if counts.is_empty() {
            counts = part;
        } else {
            for (sum, add) in counts.iter_mut().zip(part) {
                *sum = match (sum.clone(), add) {
                    (Ok(a), Ok(b)) => Ok(a + b),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                };
            }
        }
    })?;
    if counts.is_empty() {
        counts = vec![Ok(0); style_classes(renderer).len()];
    }
    Ok((counts, total))
}

/// The keys of a data layer's rows the class at `at` ([`style_classes`]' order) takes, in the table's order, at most
/// `most` of them, and whether there were more; the message of why they could not be read.
pub fn data_class_keys(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
    at: usize,
    most: usize,
    stopped: &dyn Fn() -> bool,
) -> Result<(Vec<i64>, bool), String> {
    class_keys(node, settings, at, most, stopped).map_err(message)
}

fn class_keys(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
    at: usize,
    most: usize,
    stopped: &dyn Fn() -> bool,
) -> Result<(Vec<i64>, bool), Stop> {
    let Some(reader) = node
        .style
        .renderer
        .as_ref()
        .and_then(|r| ClassReader::new(r, crate::expression_unit::reading_unit(settings)))
    else {
        return Ok((Vec::new(), false));
    };
    let mut keys = Vec::new();
    // Past `most` the scan stops: the rows after are not read.
    let more = std::cell::Cell::new(false);
    let mut failure = None;
    scan(node, settings, &reader, &|| stopped() || more.get(), &mut |objects| {
        let refs: Vec<&kentos_commands::Entity> = objects.iter().map(|(_, e)| e).collect();
        let shapes = shapes_of(&reader, &refs);
        match reader.members(&refs, &|_| node.name.clone(), &|i| shapes.get(i), at) {
            Ok(m) => {
                for ((key, _), taken) in objects.iter().zip(m) {
                    if taken {
                        if keys.len() == most {
                            more.set(true);
                            break;
                        }
                        keys.push(*key);
                    }
                }
            }
            Err(e) => failure = Some(e),
        }
    })
    .or_else(|stop| if more.get() { Ok(()) } else { Err(stop) })?;
    if let Some(e) = failure {
        return Err(fail(codes::INVALID_EXPRESSION, format!("Sınıfın koşulu okunamadı: {e}"), "layer"));
    }
    Ok((keys, more.get()))
}

/// How many rows a data layer's table has, counted by the source.
fn rows_counted(node: &kentos_commands::LayerNode) -> Result<u64, Stop> {
    let Some(data) = &node.data else {
        return Ok(0);
    };
    let source = kentos_data::registry().source(&data.address).map_err(unreachable)?;
    source.count(&data.table, &Request::default(), false).map_err(unreachable)
}

/// The shapes of the objects when a class reads geometry (none otherwise).
fn shapes_of(reader: &ClassReader, refs: &[&kentos_commands::Entity]) -> Vec<kentos_geometry_core::entity::Shape> {
    if reader.reads_geometry() {
        refs.iter().map(|e| crate::geometry::shape(e)).collect()
    } else {
        Vec::new()
    }
}

/// Every row of a data layer's table as an object of the layer with the fields the classes read (its geometry in the
/// project's system when a class reads it, a point otherwise), a part at a time.
fn scan(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
    reader: &ClassReader,
    stopped: &dyn Fn() -> bool,
    each: &mut dyn FnMut(Vec<(i64, kentos_commands::Entity)>),
) -> Result<(), Stop> {
    let Some(data) = &node.data else {
        return Ok(());
    };
    let source = kentos_data::registry().source(&data.address).map_err(unreachable)?;
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    let system = TableSystem::of(settings, srid).map_err(unreachable)?;
    let known: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
    let fields: Vec<String> = reader.fields().iter().filter(|f| known.contains(f)).cloned().collect();
    let measured = reader.reads_geometry();
    let request = Request {
        columns: Some(fields.clone()),
        geometry: measured,
        chunk: CHUNK,
        ..Request::default()
    };
    let cancel = Cancel::new();
    source
        .features(&data.table, &request, &cancel, &mut |part: Vec<Feature>| {
            let objects: Vec<(i64, kentos_commands::Entity)> = part
                .iter()
                .filter_map(|f| {
                    let attrs: serde_json::Map<String, serde_json::Value> = fields
                        .iter()
                        .zip(&f.values)
                        .filter_map(|(n, v)| attribute_text(v).map(|t| (n.clone(), serde_json::Value::String(t))))
                        .collect();
                    let g = match (measured, f.shape()) {
                        (true, Some(g)) => match &system {
                            Some(s) => s.moved(g)?,
                            None => g.clone(),
                        },
                        // Classes that do not read the geometry are asked of the values alone, on a point.
                        _ => kentos_data::Geometry::Point(Some(kentos_data::Coord::xy(0.0, 0.0))),
                    };
                    object_of(&g, &node.id, 1, attrs).map(|e| (f.fid, e))
                })
                .collect();
            each(objects);
            !stopped()
        })
        .map_err(unreachable)?;
    if stopped() {
        return Err(Stop::failed(error(codes::CANCELLED, "İş durduruldu.".into(), None)));
    }
    Ok(())
}

/// How many of a drawing layer's shown objects each class takes, and how many it shows (those the style engine draws).
pub fn drawing_counts(doc: &Document, node: &kentos_commands::LayerNode) -> (Vec<Result<u64, String>>, u64) {
    let hidden = node.filter.as_ref().map(|_| kentos_scene_query::layer_filter::left_out(doc));
    let refs: Vec<&kentos_commands::Entity> = doc
        .by_layer(&node.id)
        .filter(|e| hidden.as_ref().is_none_or(|h| !h.contains(&kentos_domain::Slot(e.base().id))))
        .collect();
    let total = refs.iter().filter(|e| kentos_scene_query::style_classes::drawn_by_style(e)).count() as u64;
    let Some(reader) = node
        .style
        .renderer
        .as_ref()
        .and_then(|r| ClassReader::new(r, crate::expression_unit::reading_unit(doc.settings())))
    else {
        return (Vec::new(), total);
    };
    let shapes = shapes_of(&reader, &refs);
    let names = |id: &str| doc.layers().get(id).map(|n| n.name.clone()).unwrap_or_default();
    (reader.counts(&refs, &names, &|i| shapes.get(i)), total)
}

/// The answer for the layer the input names.
fn read(doc: &Document, input: &LayersClasses, stopped: &dyn Fn() -> bool) -> Result<LayerClassesAnswer, Stop> {
    let node = found(doc, input)?;
    let renderer = node.style.renderer.as_ref();
    let classes = renderer.map(style_classes).unwrap_or_default();
    let (counts, total) = match input.count {
        Some(true) if node.data.is_some() => {
            let (c, t) = counted(node, doc.settings(), stopped)?;
            (Some(c), Some(t))
        }
        Some(true) => {
            let (c, t) = drawing_counts(doc, node);
            (Some(c), Some(t))
        }
        _ => (None, None),
    };
    let classes = classes
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let counted = counts.as_ref().and_then(|c| c.get(i)).cloned();
            LayerClass {
                id: c.id,
                label: c.label,
                enabled: c.enabled,
                depth: u32::try_from(c.depth).unwrap_or(u32::MAX),
                expression: c.expression,
                count: counted.as_ref().and_then(|r| r.as_ref().ok()).copied(),
                count_error: counted.and_then(|r| r.err()),
            }
        })
        .collect();
    Ok(LayerClassesAnswer {
        layer: node.id.clone(),
        kind: renderer.and_then(classes_kind).map(str::to_owned),
        classes,
        total,
    })
}

fn answer(cx: &ExecutionContext<'_>, input: &LayersClasses) -> CommandResult<LayerClassesAnswer> {
    match read(cx.doc, input, &|| false) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &LayersClasses) -> CommandResult<()> {
    match found(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &LayersClasses) -> CommandResult<LayerClassesAnswer> {
    answer(cx, input)
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersClasses) -> CommandResult<LayerClassesAnswer> {
    answer(cx, &input)
}
