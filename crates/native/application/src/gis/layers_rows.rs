//! `cad.layers.rows` v1 (docs/adr/9102 §4): a data layer's rows read from its source through its driver
//! (`kentos_data::registry`, registered by the host). The box, the keys and the search are the source's; so are the
//! order (a text in the Turkish alphabet's order, docs/adr/9102 §3), the page and the count, while there is no filter.
//! A filter is the language's own: every row the source keeps is read (its key, the fields the filter names, its
//! geometry when the filter measures it) and the filter asked of it, in the order; the page and the count are then of
//! the rows it held for. Nothing is written.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, LAYER_ROWS_PAGE_MOST, LayerFilter, LayerRow, LayerRowsAnswer, LayersRows, RowsChecked,
};
use kentos_data::{Cancel, Condition, Feature, Order, Request};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geopackage::{attribute_text, object_of};
use crate::layer_filter::{CompiledFilter, compile_filter};
use crate::sources::TableSystem;

/// Rows asked of the source at a time.
const CHUNK: usize = 2048;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

fn unreachable(e: impl std::fmt::Display) -> Stop {
    Stop::failed(error(codes::SOURCE_UNREACHABLE, format!("Kaynak okunamadı: {e}"), None))
}

/// What the question asks, checked against the document.
struct Asked {
    data: kentos_commands::DataLayer,
    layer: String,
    fields: Vec<String>,
    all_fields: Vec<String>,
    order: Vec<Order>,
    filter: Option<CompiledFilter>,
    limit: u64,
}

/// The input's own checks: the layer named, the page's size, the box.
fn checked(input: &LayersRows) -> Result<(), Stop> {
    let layer = input.layer.trim();
    if layer.is_empty() {
        return Err(fail(codes::NO_LAYER, "Satırları okunacak veri katmanını (layer) verin.".into(), "layer"));
    }
    let limit = input.limit.unwrap_or(100);
    if limit > LAYER_ROWS_PAGE_MOST {
        return Err(fail(
            codes::INVALID_COUNT,
            format!("Bir sayfa en çok {LAYER_ROWS_PAGE_MOST} satır tutar; {limit} istendi."),
            "limit",
        ));
    }
    if let Some(b) = input.bbox
        && !(b.iter().all(|v| v.is_finite()) && b[0] <= b[2] && b[1] <= b[3])
    {
        return Err(fail(
            codes::INVALID_EXTENT,
            "Kutu batı, güney, doğu, kuzey sırasıyla sonlu dört sayı olmalı; batı doğuyu, güney kuzeyi geçemez.".into(),
            "bbox",
        ));
    }
    Ok(())
}

/// The layer the input names, after its own checks.
fn found<'d>(doc: &'d Document, input: &LayersRows) -> Result<&'d kentos_commands::LayerNode, Stop> {
    checked(input)?;
    let layer = input.layer.trim();
    doc.layers().get(layer).ok_or_else(|| {
        fail(
            codes::LAYER_NOT_FOUND,
            format!("“{layer}” kimlikli katman çizimde yok."),
            "layer",
        )
    })
}

/// What the input asks of `node`: its table, the fields, the order, the filter compiled.
fn asked(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
    input: &LayersRows,
) -> Result<Asked, Stop> {
    let limit = input.limit.unwrap_or(100);
    let Some(data) = node.data.clone() else {
        return Err(fail(
            codes::NOT_A_DATA_LAYER,
            format!("“{}” bir veri katmanı değil; satırları kaynaktan okunmaz.", node.name),
            "layer",
        ));
    };
    let all_fields: Vec<String> = node.fields.iter().map(|f| f.name.clone()).collect();
    let known = |name: &str, path: &str| -> Result<(), Stop> {
        if all_fields.iter().any(|f| f == name) {
            Ok(())
        } else {
            Err(fail(
                codes::FIELD_NOT_FOUND,
                format!("“{name}” alanı “{}” katmanında yok.", node.name),
                path,
            ))
        }
    };
    let fields = match &input.fields {
        None => all_fields.clone(),
        Some(fs) => {
            for (i, f) in fs.iter().enumerate() {
                known(f, &format!("fields[{i}]"))?;
            }
            fs.clone()
        }
    };
    let mut order = Vec::new();
    for (i, o) in input.order.iter().flatten().enumerate() {
        known(&o.field, &format!("order[{i}].field"))?;
        order.push(Order {
            field: o.field.clone(),
            descending: o.descending,
        });
    }
    let filter = match input.filter.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(text) => {
            let unit = crate::expression_unit::reading_unit(settings);
            let f = compile_filter(
                &LayerFilter {
                    expression: Some(text.to_owned()),
                    ..LayerFilter::default()
                },
                unit,
            )
            .map_err(|why| fail(codes::INVALID_EXPRESSION, format!("Süzgeç: {why}"), "filter"))?;
            if f.expr().is_some_and(|e| !e.world.is_empty()) {
                return Err(fail(
                    codes::INVALID_EXPRESSION,
                    "Süzgeç başka nesnelere bakamaz: bir veri katmanının satırı yalnız kendi alanlarıyla sorulur.".into(),
                    "filter",
                ));
            }
            Some(f)
        }
    };
    Ok(Asked {
        data,
        layer: node.id.clone(),
        fields,
        all_fields,
        order,
        filter,
        limit: u64::from(limit),
    })
}

/// A row as the answer gives it: its key, the fields asked for, its geometry in the project's system.
fn row_of(f: &Feature, names: &[String], system: Option<&TableSystem>, wkt: bool) -> LayerRow {
    let attrs: BTreeMap<String, String> = names
        .iter()
        .zip(&f.values)
        .filter_map(|(n, v)| attribute_text(v).map(|t| (n.clone(), t)))
        .collect();
    let wkt = wkt.then(|| f.shape()).flatten().and_then(|g| match system {
        Some(s) => s.moved(g),
        None => Some(g.clone()),
    });
    LayerRow {
        key: f.fid,
        attrs,
        wkt: wkt.map(|g| kentos_data::wkt::write_wkt(&g)),
    }
}

fn run(doc: &Document, input: &LayersRows, stopped: &dyn Fn() -> bool) -> Result<LayerRowsAnswer, Stop> {
    let node = found(doc, input)?;
    rows_of(node, doc.settings(), input, stopped)
}

/// The command's answer for a layer and the project's settings a host holds (the attribute table reads its pages with
/// it off the interface's thread, without a document): the same checks after the layer's lookup, the same answer.
pub fn read(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
    input: &LayersRows,
    stopped: &dyn Fn() -> bool,
) -> Result<LayerRowsAnswer, Box<kentos_commands::CommandError>> {
    let stop = match checked(input).and_then(|()| rows_of(node, settings, input, stopped)) {
        Ok(answer) => return Ok(answer),
        Err(stop) => stop,
    };
    match CommandResult::<LayerRowsAnswer>::from(stop) {
        CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
            Err(Box::new(error))
        }
        _ => Err(Box::new(error(codes::SOURCE_UNREACHABLE, "Kaynak okunamadı.".into(), None))),
    }
}

fn rows_of(
    node: &kentos_commands::LayerNode,
    settings: &kentos_commands::ProjectSettings,
    input: &LayersRows,
    stopped: &dyn Fn() -> bool,
) -> Result<LayerRowsAnswer, Stop> {
    let asked = asked(node, settings, input)?;
    let data = &asked.data;
    let source = kentos_data::registry().source(&data.address).map_err(unreachable)?;
    let srid = data.geometry.as_ref().and_then(|g| g.srid);
    let system = TableSystem::of(settings, srid).map_err(unreachable)?;
    let bbox = match (input.bbox, &system, &data.geometry) {
        (None, ..) | (_, _, None) => None,
        (Some(b), None, Some(_)) => Some(b),
        (Some(b), Some(s), Some(_)) => Some(
            s.to_table(b)
                .ok_or_else(|| fail(codes::INVALID_EXTENT, "Kutu tablonun sistemine çevrilemedi.".into(), "bbox"))?,
        ),
    };
    let mut parts = Vec::new();
    if let Some(keys) = &input.keys {
        parts.push(Condition::Fids(keys.clone()));
    }
    if let Some(words) = input.search.as_deref().map(str::trim).filter(|w| !w.is_empty()) {
        parts.push(Condition::Contains {
            fields: asked.all_fields.clone(),
            text: words.to_owned(),
        });
    }
    let condition = (!parts.is_empty()).then(|| Condition::and(parts));
    let base = Request {
        bbox,
        condition: condition.clone(),
        order: asked.order.clone(),
        chunk: CHUNK,
        ..Request::default()
    };
    let offset = input.offset.unwrap_or(0);
    let wkt = input.geometry == Some(true);
    let cancel = Cancel::new();
    let read_page = |request: &Request| -> Result<Vec<LayerRow>, Stop> {
        let mut rows = Vec::new();
        source
            .features(&data.table, request, &cancel, &mut |part: Vec<Feature>| {
                rows.extend(part.iter().map(|f| row_of(f, &asked.fields, system.as_ref(), wkt)));
                !stopped()
            })
            .map_err(unreachable)?;
        if stopped() {
            return Err(Stop::failed(error(codes::CANCELLED, "İş durduruldu.".into(), None)));
        }
        Ok(rows)
    };
    let page = |condition: Option<Condition>, offset: u64| Request {
        condition,
        columns: Some(asked.fields.clone()),
        geometry: wkt,
        limit: Some(asked.limit),
        offset,
        ..base.clone()
    };
    let Some(filter) = &asked.filter else {
        let total = match input.count {
            Some(true) => Some(source.count(&data.table, &base, false).map_err(unreachable)?),
            _ => None,
        };
        let keys = match input.all_keys {
            Some(true) => Some(keys_of(&*source, &data.table, &base, stopped)?),
            _ => None,
        };
        let rows = if asked.limit == 0 {
            Vec::new()
        } else {
            read_page(&page(condition.clone(), offset))?
        };
        return Ok(LayerRowsAnswer {
            fields: asked.fields.clone(),
            rows,
            offset,
            total,
            keys,
            checked: RowsChecked::Source,
        });
    };
    // The filter asked of every row the source keeps, in the order: the rows it holds for, by key.
    let expr_fields: Vec<String> = filter
        .expr()
        .map(|e| e.fields.iter().filter(|f| asked.all_fields.contains(f)).cloned().collect())
        .unwrap_or_default();
    let measured = filter
        .expr()
        .is_some_and(|e| e.needs.measured || e.needs.vertices || e.needs.kind || e.needs.centroid);
    let scan = Request {
        columns: Some(expr_fields.clone()),
        geometry: measured,
        ..base.clone()
    };
    let mut kept: Vec<i64> = Vec::new();
    let mut failure = None;
    source
        .features(&data.table, &scan, &cancel, &mut |part: Vec<Feature>| {
            let objects: Vec<(i64, kentos_commands::Entity)> = part
                .iter()
                .filter_map(|f| {
                    let attrs: serde_json::Map<String, serde_json::Value> = expr_fields
                        .iter()
                        .zip(&f.values)
                        .filter_map(|(n, v)| attribute_text(v).map(|t| (n.clone(), serde_json::Value::String(t))))
                        .collect();
                    let g = match (measured, f.shape()) {
                        (true, Some(g)) => match &system {
                            Some(s) => s.moved(g)?,
                            None => g.clone(),
                        },
                        // A filter that does not measure is asked of the values alone, on a point.
                        _ => kentos_data::Geometry::Point(Some(kentos_data::Coord::xy(0.0, 0.0))),
                    };
                    object_of(&g, &asked.layer, 1, attrs).map(|e| (f.fid, e))
                })
                .collect();
            let refs: Vec<&kentos_commands::Entity> = objects.iter().map(|(_, e)| e).collect();
            let shapes: Vec<_> = if measured {
                refs.iter().map(|e| crate::geometry::shape(e)).collect()
            } else {
                Vec::new()
            };
            let held = filter.condition(&refs, &|_| String::new(), |i| shapes.get(i));
            kept.extend(objects.iter().zip(held).filter(|(_, h)| *h).map(|((k, _), _)| *k));
            if stopped() {
                failure = Some(Stop::failed(error(codes::CANCELLED, "İş durduruldu.".into(), None)));
                return false;
            }
            true
        })
        .map_err(unreachable)?;
    if let Some(stop) = failure {
        return Err(stop);
    }
    let from = (offset as usize).min(kept.len());
    let to = (from + asked.limit as usize).min(kept.len());
    let rows = if from == to {
        Vec::new()
    } else {
        let keys = Condition::Fids(kept[from..to].to_vec());
        let condition = Some(match &condition {
            Some(c) => Condition::and(vec![c.clone(), keys]),
            None => keys,
        });
        read_page(&page(condition, 0))?
    };
    Ok(LayerRowsAnswer {
        fields: asked.fields.clone(),
        rows,
        offset,
        total: (input.count == Some(true)).then_some(kept.len() as u64),
        keys: (input.all_keys == Some(true)).then_some(kept),
        checked: RowsChecked::Here,
    })
}

/// Every key the request keeps, in its order.
fn keys_of(
    source: &dyn kentos_data::Source,
    table: &str,
    request: &Request,
    stopped: &dyn Fn() -> bool,
) -> Result<Vec<i64>, Stop> {
    let mut keys = Vec::new();
    let request = Request {
        columns: Some(Vec::new()),
        geometry: false,
        ..request.clone()
    };
    source
        .features(table, &request, &Cancel::new(), &mut |part: Vec<Feature>| {
            keys.extend(part.iter().map(|f| f.fid));
            !stopped()
        })
        .map_err(unreachable)?;
    Ok(keys)
}

/// Checks `input` against the document, reading no source.
pub fn validate(cx: &ExecutionContext<'_>, input: &LayersRows) -> CommandResult<()> {
    match found(cx.doc, input).and_then(|node| asked(node, cx.doc.settings(), input)) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The answer, as execute gives it: the command writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &LayersRows) -> CommandResult<LayerRowsAnswer> {
    answer(cx, input)
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: LayersRows) -> CommandResult<LayerRowsAnswer> {
    answer(cx, &input)
}

fn answer(cx: &ExecutionContext<'_>, input: &LayersRows) -> CommandResult<LayerRowsAnswer> {
    match run(cx.doc, input, &|| cx.stopped()) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
