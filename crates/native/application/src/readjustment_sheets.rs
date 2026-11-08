//! `cad.readjustment.sheets` v1 (plan/map-cad.md MAP-11, docs/adr/2680): Ayırma çapı paftaları. One sheet per parcel
//! partly inside the boundary (Arazi ve Arsa Düzenlemeleri Hakkında Yönetmelik m. 13: a separation sheet per parcel,
//! every piece outside on it) from KentOS's separation sheet (`sys:ayirma-capi`) or the given template: the numbers and
//! parts are `cad.readjustment.scope`'s, the parts' corners its numbered points. The points are written on a layer of
//! the parcel's own under the group “Ayırma çapı noktaları”, labelled with their numbers; the sheet's map (fitted to the
//! parcel at a standard scale, as `cad.sheets.fit` fits) freezes the other parcels' point layers, its coordinate list
//! reads the parcel's, its tables take the areas and the parts written with the policy's places and rounding. The
//! points, the layers and the sheets are one undo step “Ayırma çapı paftaları”. The shared cases are
//! `fixtures/commands/v1/cad.readjustment.sheets.json`.

use std::collections::{BTreeMap, BTreeSet};

use kentos_commands::{
    CommandResult, EntitiesCreate, EntityGeometry, LayerNode, LayerNodeType, NewObject, ReadjustmentScopeParcel,
    ReadjustmentSheets, ReadjustmentSheetsMade, ScopePart, ScopeRing, SeparationSheet, SheetPicture, SheetTemplateSource,
    SheetsEdit, Vec2,
};
use kentos_domain::{Document, NewLayer};
use kentos_geometry_core::numeric::{RoundingMode, parse_decimal, subtract_exact};
use kentos_sheet::model::VarValue;
use kentos_sheet::ops::{Op, SetItemProps};
use kentos_sheet::template::{InstanceOptions, Template, VariableValue};
use kentos_sheet::{ItemKind, MapView, create, expr, fit, template};

use crate::checks::{self, Stop, error};
use crate::{ExecutionContext, codes, readjustment_scope, sheets_edit, sheets_fit};

/// The template when none is given: KentOS's separation sheet.
pub const DEFAULT_TEMPLATE: &str = "sys:ayirma-capi";
/// The undo step's name.
pub const STEP: &str = "Ayırma çapı paftaları";
/// The group the parcels' point layers are in.
pub const GROUP: &str = "Ayırma çapı noktaları";
/// A sheet's name before the parcel's: “Ayırma çapı 101/10”.
pub const SHEET_WORD: &str = "Ayırma çapı";
/// The template's items the sheets fill, by their ids in the template.
const MAP: &str = "harita";
const COORDINATES: &str = "koordinatlar";
const AREAS: &str = "yuzolcumleri";
const PARTS: &str = "kisimlar";
/// The ground around the parcel on each side, a percent of its size (`cad.sheets.fit`'s default).
const MARGIN: u8 = 5;
/// The template's questions each parcel answers from its own attributes.
const OWN: [&str; 5] = ["il", "ilce", "mahalle", "ada", "parsel"];

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A stop of the scope's, its place under `scope.`.
fn under(stop: Stop) -> Stop {
    match stop {
        Stop::Failed(mut e) => {
            e.path = Some(e.path.take().map_or_else(|| "scope".to_owned(), |p| format!("scope.{p}")));
            Stop::Failed(e)
        }
        other => other,
    }
}

/// The parts' names on the sheet: Giren and Girmeyen, numbered on a side that has more than one (each piece outside
/// takes a parcel number of its own, Article 13(3)).
pub fn part_names(parts: &[ScopePart]) -> Vec<String> {
    let on = |side: &str| parts.iter().filter(|p| p.side == side).count();
    let (inside, outside) = (on("inside"), on("outside"));
    let (mut i, mut o) = (0, 0);
    parts
        .iter()
        .map(|p| {
            let (word, n, of) = if p.side == "inside" {
                i += 1;
                ("Giren", i, inside)
            } else {
                o += 1;
                ("Girmeyen", o, outside)
            };
            if of > 1 { format!("{word} {n}") } else { word.to_owned() }
        })
        .collect()
}

/// A ring as point numbers, closed by its first: “1-2-3-4-1”.
fn ring_text(r: &ScopeRing) -> String {
    let mut n: Vec<String> = r.numbers.iter().map(u32::to_string).collect();
    if let Some(first) = n.first().cloned() {
        n.push(first);
    }
    n.join("-")
}

/// A part's boundary as point numbers: its outer ring, then each hole's (“1-2-3-4-1 (delik 5-6-7-8-5)”).
pub fn boundary_text(part: &ScopePart) -> String {
    let mut text = part.rings.first().map(ring_text).unwrap_or_default();
    for hole in part.rings.iter().skip(1) {
        text.push_str(&format!(" (delik {})", ring_text(hole)));
    }
    text
}

/// The table of areas (Article 13(2)'s items, and what the part outside keeps by Article 31(2)), square metres: the
/// record as written (its comma a point when it reads as a decimal), the computed values at the policy's places by its
/// rounding, the difference of the written values exactly (computed − registered, with the record's places or the
/// policy's, the more), the scope's `outsideRecorded` (registered − the part inside); “—” for what is not there.
pub fn area_rows(p: &ReadjustmentScopeParcel, scale: u32, mode: RoundingMode) -> Vec<Vec<String>> {
    let none = || "—".to_owned();
    let w = |v: f64| readjustment_scope::computed(v, scale, mode);
    let record = p.recorded_text.as_deref().map(|t| t.replace(',', ".")).filter(|t| parse_decimal(t).is_ok());
    let registered = record.clone().or_else(|| p.recorded_text.clone()).unwrap_or_else(none);
    let calculated = w(p.calculated);
    let difference = calculated.as_deref().zip(record.as_deref()).and_then(|(c, r)| {
        subtract_exact(c, r).ok().and_then(|d| readjustment_scope::padded(&d, readjustment_scope::places(r).max(scale)))
    });
    vec![
        vec!["Tescilli yüzölçümü".to_owned(), registered],
        vec!["Hesaplanan yüzölçümü".to_owned(), calculated.unwrap_or_else(none)],
        vec!["Yüzölçümü farkı".to_owned(), difference.unwrap_or_else(none)],
        vec!["Yanılma sınırı".to_owned(), w(p.limit).unwrap_or_else(none)],
        vec!["Giren kısım".to_owned(), w(p.inside).unwrap_or_else(none)],
        vec!["Girmeyen kısım".to_owned(), w(p.outside).unwrap_or_else(none)],
        vec!["Girmeyen kısma kalan (tescilli − giren)".to_owned(), p.outside_recorded.clone().unwrap_or_else(none)],
    ]
}

/// The table of parts: each part's name, its boundary as point numbers and its area at the policy's places.
pub fn part_rows(p: &ReadjustmentScopeParcel, scale: u32, mode: RoundingMode) -> Vec<Vec<String>> {
    part_names(&p.parts)
        .into_iter()
        .zip(&p.parts)
        .map(|(name, part)| {
            vec![
                name,
                boundary_text(part),
                readjustment_scope::computed(part.area, scale, mode).unwrap_or_else(|| "—".to_owned()),
            ]
        })
        .collect()
}

/// The parcel's points as the drawing's: a point each, labelled with its number, its parcel and number as attributes.
fn point_objects(p: &ReadjustmentScopeParcel) -> Vec<NewObject> {
    p.points
        .iter()
        .enumerate()
        .map(|(i, c)| NewObject {
            geometry: EntityGeometry::Point { p: Vec2 { x: c[0], y: c[1] }, z: None, parts: None },
            color: None,
            line_weight: None,
            line_type: None,
            attrs: Some(BTreeMap::from([
                ("Parsel".to_owned(), p.parcel.clone()),
                ("Nokta".to_owned(), (i + 1).to_string()),
            ])),
            label: Some((i + 1).to_string()),
            symbol: None,
            label_of: None,
            label_scale: None,
            transparency: None,
            order: None,
        })
        .collect()
}

/// The group named “Ayırma çapı noktaları” at the top of the tree, if the drawing has one.
fn group_of(doc: &Document) -> Option<&LayerNode> {
    doc.layers().nodes().iter().find(|n| n.kind == LayerNodeType::Group && n.name == GROUP)
}

/// The template `source` names, or KentOS's separation sheet.
fn template_of(source: Option<&SheetTemplateSource>) -> Result<Template, Stop> {
    match source {
        None => template::system_template(DEFAULT_TEMPLATE).cloned().ok_or_else(|| {
            fail(codes::UNKNOWN_TEMPLATE, format!("“{DEFAULT_TEMPLATE}” sistem şablonu yok."), "source")
        }),
        Some(SheetTemplateSource::System { id }) => template::system_template(id).cloned().ok_or_else(|| {
            fail(
                codes::UNKNOWN_TEMPLATE,
                format!("“{id}” sistem şablonu yok; cad.sheets.set(details=true) ile şablonları listeleyin."),
                "source.id",
            )
        }),
        Some(SheetTemplateSource::Custom { template }) => template::read_template(&template.to_string())
            .map_err(|e| Stop::failed(error(&e.code, e.message, e.path.map(|p| format!("source.template.{p}"))))),
    }
}

fn core(e: kentos_sheet::SheetError) -> Stop {
    Stop::failed(error(&e.code, e.message, e.path))
}

/// A sheet to make: its parcel, its operations (the sheet, its map's ground, its tables' rows), its map and coordinate
/// list in the book, and the answer's row.
struct Planned {
    parcel: ReadjustmentScopeParcel,
    ops: Vec<Op>,
    map: String,
    coordinates: Option<String>,
    out: SeparationSheet,
}

struct Laid {
    planned: Vec<Planned>,
    pictures: Vec<SheetPicture>,
    group: Option<String>,
}

fn prepare(doc: &Document, input: &ReadjustmentSheets) -> Result<Laid, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let scope = readjustment_scope::run(doc, &input.scope).map_err(under)?;
    // The scope read the rounding; its places and rounding write the tables.
    let mode = RoundingMode::parse(input.scope.rounding.trim()).unwrap_or(RoundingMode::HalfEven);
    let partial: Vec<&ReadjustmentScopeParcel> = scope.parcels.iter().filter(|p| p.kind == "partial").collect();
    if partial.is_empty() {
        return Err(fail(
            codes::NO_PARTIAL_PARCELS,
            "Düzenleme sınırına kısmen giren parsel yok; ayırma çapı yalnız bir kısmı düzenleme sahasında kalan parsel için \
             düzenlenir (Yönetmelik m. 13/1)."
                .to_owned(),
            "scope",
        ));
    }
    for p in &partial {
        if p.no_identity {
            return Err(fail(
                codes::NO_IDENTITY,
                format!("“{}” parselinin Ada'sı ya da Parsel'i yok; ayırma çapı paftası parselin kimliğiyle adlandırılır.", p.uid),
                "scope",
            ));
        }
        if p.duplicate {
            return Err(fail(
                codes::DUPLICATE_PARCEL,
                format!("“{}” iki parselin kimliği; ayırma çapı paftası parselin kimliğiyle adlandırılır, önce kimliklerden birini düzeltin.", p.parcel),
                "scope",
            ));
        }
    }
    let template = template_of(input.source.as_ref())?;
    let options: InstanceOptions = serde_json::from_value(input.options.clone())
        .map_err(|e| fail(codes::INVALID_OPTIONS, format!("Şablon seçenekleri okunamadı: {e}"), "options"))?;
    if options.name.is_some() {
        return Err(fail(
            codes::INVALID_OPTIONS,
            format!("Ayırma çapı paftalarının adları parsellerinden gelir (“{SHEET_WORD} 101/10”); options.name verilmez."),
            "options.name",
        ));
    }
    if options.center.is_some() {
        return Err(fail(
            codes::INVALID_OPTIONS,
            "Her paftanın haritası kendi parselini ortalar; options.center verilmez.".to_owned(),
            "options.center",
        ));
    }
    let parcel_asks: Vec<String> = OWN.iter().map(|q| expr::fold(q)).collect();
    for (i, v) in options.values.iter().enumerate() {
        if parcel_asks.contains(&expr::fold(&v.name)) {
            return Err(fail(
                codes::INVALID_OPTIONS,
                format!("“{}” her paftada parselin kendi özniteliğinden gelir; cevap olarak verilmez.", v.name),
                &format!("options.values[{i}].name"),
            ));
        }
    }
    let items = &template.sheet.items;
    let map_at = items
        .iter()
        .position(|i| i.id == MAP && matches!(&i.kind, ItemKind::Map(m) if matches!(m.view, MapView::Fixed(_))))
        .ok_or_else(|| {
            fail(
                codes::SHEET_NOT_FIXED_MAP,
                "Şablonda “harita” kimlikli sabit görünüm penceresi yok; ayırma çapının haritası ona yerleşir.".to_owned(),
                "source",
            )
        })?;
    let at = |id: &str, fits: fn(&ItemKind) -> bool| items.iter().position(|i| i.id == id && fits(&i.kind));
    let coordinates_at = at(COORDINATES, |k| matches!(k, ItemKind::CoordinateList(_)));
    let areas_at = at(AREAS, |k| matches!(k, ItemKind::Table(_)));
    let parts_at = at(PARTS, |k| matches!(k, ItemKind::Table(_)));
    let asked: BTreeSet<String> = template.variables.iter().map(|v| expr::fold(&v.name)).collect();
    let book = match doc.sheets() {
        Some(s) => kentos_sheet::project::read_project_sheets(s)
            .map_err(|e| Stop::failed(error(codes::BOOK_UNREADABLE, e.message, None)))?
            .0,
        None => Default::default(),
    };
    let names: BTreeSet<&str> = book.sheets.iter().map(|s| s.name.as_str()).collect();
    let mut taken: BTreeSet<String> = book.sheets.iter().map(|s| s.id.clone()).collect();
    let group = group_of(doc);
    let layers: BTreeSet<&str> = group.map(|g| g.children.iter().map(|c| c.name.as_str()).collect()).unwrap_or_default();
    let mut laid = Laid { planned: Vec::new(), pictures: Vec::new(), group: group.map(|g| g.id.clone()) };
    for p in partial {
        let name = format!("{SHEET_WORD} {}", p.parcel);
        if names.contains(name.as_str()) {
            return Err(fail(
                codes::SEPARATION_SHEET_EXISTS,
                format!("“{name}” paftası zaten var; yeniden yapmak için önce o paftayı ve “{GROUP} › {}” katmanını silin.", p.parcel),
                "scope",
            ));
        }
        if layers.contains(p.parcel.as_str()) {
            return Err(fail(
                codes::SEPARATION_SHEET_EXISTS,
                format!("“{GROUP} › {}” katmanı zaten var; yeniden yapmak için önce o katmanı ve “{name}” paftasını silin.", p.parcel),
                "scope",
            ));
        }
        let sheet = (1..).map(|k| format!("ayirma-capi-{k}")).find(|id| !taken.contains(id)).unwrap_or_default();
        taken.insert(sheet.clone());
        let ids = create::ids_for(&template, &sheet);
        let mut own = options.clone();
        own.name = Some(name.clone());
        for ((q, folded), value) in OWN.iter().zip(&parcel_asks).zip([&p.il, &p.ilce, &p.mahalle, &p.ada, &p.parsel]) {
            if asked.contains(folded) {
                own.values.push(VariableValue { name: (*q).to_owned(), value: VarValue::Text(value.clone()) });
            }
        }
        let made = create::from_template(&template, &ids, &own, None).map_err(core)?;
        for picture in made.pictures {
            if !laid.pictures.iter().any(|q| q.sha256 == picture.meta.sha256) {
                let bytes = template::base64_decode(&picture.data).ok_or_else(|| {
                    fail("bad_asset", "Şablon resmi okunamadı.".to_owned(), "source.template.assets")
                })?;
                laid.pictures.push(SheetPicture { sha256: picture.meta.sha256, bytes });
            }
        }
        let item = template::instantiate(&template, &ids, &own).map_err(core)?.sheet.items.swap_remove(map_at);
        let bounds = sheets_fit::object_extent(doc, Some(std::slice::from_ref(&p.uid)))
            .map_err(|e| Stop::failed(*e))?
            .ok_or_else(|| fail(codes::ENTITY_NOT_FOUND, format!("“{}” parselinin kapsamı yok.", p.uid), "scope"))?;
        // The map's ground: the parcel's box fitted at a standard scale, or centred at the scale given.
        let centre = [bounds[0] + (bounds[2] - bounds[0]) / 2., bounds[1] + (bounds[3] - bounds[1]) / 2.];
        let (patch, scale) = match options.scale {
            Some(s) => (serde_json::json!({"kind": {"view": {"center": {"x": centre[0], "y": centre[1]}}}}), s),
            None => match fit::map(&item, bounds, MARGIN, true).map_err(core)? {
                Op::SetItemProps(SetItemProps { patch, .. }) => {
                    let s = patch["kind"]["view"]["scale"].as_u64().and_then(|s| u32::try_from(s).ok()).unwrap_or(0);
                    (patch, s)
                }
                _ => (serde_json::json!({"kind": {}}), 0),
            },
        };
        // The scale chosen stays: the map's view is locked against a slip of the wheel.
        let mut patch = patch;
        patch["kind"]["viewLocked"] = serde_json::Value::Bool(true);
        let mut ops = made.ops;
        ops.push(Op::SetItemProps(SetItemProps { id: item.id.clone(), patch }));
        let (areas, parts) = (area_rows(p, input.scope.scale, mode), part_rows(p, input.scope.scale, mode));
        for (at, rows) in [(areas_at, &areas), (parts_at, &parts)] {
            if let Some(i) = at {
                ops.push(Op::SetItemProps(SetItemProps {
                    id: ids.items[i].clone(),
                    patch: serde_json::json!({"kind": {"source": {"type": "fixed", "rows": rows}}}),
                }));
            }
        }
        laid.planned.push(Planned {
            parcel: p.clone(),
            ops,
            map: item.id.clone(),
            coordinates: coordinates_at.map(|i| ids.items[i].clone()),
            out: SeparationSheet {
                sheet: ids.sheet.clone(),
                name,
                uid: p.uid.clone(),
                parcel: p.parcel.clone(),
                layer: None,
                scale,
                center: centre,
                points: u32::try_from(p.points.len()).unwrap_or(u32::MAX),
                areas,
                parts,
            },
        });
    }
    Ok(laid)
}

/// The answer a plan gives: the sheets without their layers, nothing written.
fn planned(doc: &Document, laid: Laid) -> ReadjustmentSheetsMade {
    ReadjustmentSheetsMade {
        group: None,
        sheets: laid.planned.into_iter().map(|p| p.out).collect(),
        uids: Vec::new(),
        revision: doc.revision().to_string(),
    }
}

fn write(cx: &mut ExecutionContext<'_>, laid: Laid) -> CommandResult<ReadjustmentSheetsMade> {
    let Laid { planned, pictures, group } = laid;
    let step = cx.doc.begin_group(STEP);
    let group = match group {
        Some(g) => g,
        None => match cx.doc.add_layer(NewLayer::group(GROUP), None, false) {
            Ok(id) => id,
            Err(r) => {
                cx.doc.cancel_group(step);
                return fail(codes::INVALID_GROUP_NAME, r.to_string(), "scope").into();
            }
        },
    };
    let mut layers = Vec::new();
    let mut uids = Vec::new();
    for p in &planned {
        let layer = match cx.doc.add_layer(NewLayer::layer(p.parcel.parcel.clone()), Some(&group), false) {
            Ok(id) => id,
            Err(r) => {
                cx.doc.cancel_group(step);
                return fail(codes::LAYER_NOT_FOUND, r.to_string(), "scope").into();
            }
        };
        let create = EntitiesCreate {
            layer_id: layer.clone(),
            objects: point_objects(&p.parcel),
            operation: None,
            expected_revision: None,
        };
        match crate::create::write_in(cx, create) {
            CommandResult::Completed { output, .. } => uids.extend(output.created),
            other => {
                cx.doc.cancel_group(step);
                return crate::trim::relay(other);
            }
        }
        layers.push(layer);
    }
    // Every point layer of the group now, the ones made before too: each map shows its own parcel's only.
    let all: Vec<String> = cx
        .doc
        .layers()
        .get(&group)
        .map(|g| g.children.iter().map(|c| c.id.clone()).collect())
        .unwrap_or_default();
    let mut ops: Vec<serde_json::Value> = Vec::new();
    let mut sheets = Vec::new();
    for (p, layer) in planned.into_iter().zip(layers) {
        let mut own: Vec<Op> = p.ops;
        let frozen: Vec<serde_json::Value> =
            all.iter().filter(|c| **c != layer).map(|c| serde_json::json!({"layer": c, "frozen": true})).collect();
        if !frozen.is_empty() {
            own.push(Op::SetItemProps(SetItemProps {
                id: p.map.clone(),
                patch: serde_json::json!({"kind": {"layerOverrides": frozen}}),
            }));
        }
        if let Some(list) = &p.coordinates {
            own.push(Op::SetItemProps(SetItemProps {
                id: list.clone(),
                patch: serde_json::json!({"kind": {"source": {"type": "layer", "layer": layer}, "naming": {"type": "given"}}}),
            }));
        }
        ops.extend(own.iter().map(|op| serde_json::to_value(op).unwrap_or_default()));
        let mut out = p.out;
        out.layer = Some(layer);
        sheets.push(out);
    }
    let edit = SheetsEdit { ops, pictures, replace_pictures: false, expected_revision: None };
    match sheets_edit::write_execute(cx, edit) {
        CommandResult::Completed { .. } => {}
        other => {
            cx.doc.cancel_group(step);
            return crate::trim::relay(other);
        }
    }
    cx.doc.end_group(step);
    CommandResult::Completed {
        output: ReadjustmentSheetsMade { group: Some(group), sheets, uids, revision: cx.doc.revision().to_string() },
        warnings: Vec::new(),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ReadjustmentSheets) -> CommandResult<()> {
    match prepare(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The sheets execute would make now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ReadjustmentSheets) -> CommandResult<ReadjustmentSheetsMade> {
    match prepare(cx.doc, input) {
        Ok(laid) => CommandResult::Completed { output: planned(cx.doc, laid), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// Writes the points and makes the sheets: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ReadjustmentSheets) -> CommandResult<ReadjustmentSheetsMade> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_READJUSTMENT_SHEETS,
        kentos_commands::CAD_READJUSTMENT_SHEETS_VERSION,
        input,
        |cx, input| match prepare(cx.doc, &input) {
            Ok(laid) => write(cx, laid),
            Err(stop) => stop.into(),
        },
    )
}
