//! `cad.sheets.set` v1 (docs/adr/0347 §1; plan/cad.md `CAD-ARCH-43`): the project's sheet book as a drawing set — the
//! sheets by series from their names' numbers, each map frame's ground (the sheet core's fixed view, `fixed_corners`) and
//! the view marks pointing at it — and, given objects, the frames their change affects: an object's box meeting the frame's
//! ground box (`shows`), a view mark among them pointing at it (`marked`), a frame whose ground is not known (`unknown`).
//! Read-only. The shared cases are `fixtures/commands/v1/cad.sheets.set.json`.

use kentos_commands::{
    AffectReason, AffectedFrame, CommandResult, Entity, OTHER_SERIES, SetFrame, SetSheet,
    SheetSeries, SheetsSet, SheetsSetResult, Vec2, ViewTarget, series_of, series_title,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::arrange::object_bounds;
use kentos_sheet::kinds::{ItemKind, MapView};
use kentos_sheet::project::read_project_sheets;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::geometry::{drawing_font, shape};

fn corners_box(c: &[Vec2]) -> Option<[f64; 4]> {
    let first = c.first()?;
    let mut b = [first.x, first.y, first.x, first.y];
    for p in c {
        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
    }
    Some(b)
}

/// Two closed boxes share a point.
fn meets(a: &Bounds, b: &[f64; 4]) -> bool {
    a.min_x <= b[2] && b[0] <= a.max_x && a.min_y <= b[3] && b[1] <= a.max_y
}

/// Whether the map `item` of sheet `sheet` draws the object of persistent id `uid` (docs/adr/0774 §4): on the map's
/// level or on none (an opening on its wall's, a tag on its object's: Kat görünümü's rule), its category and phase not
/// left out by its linked view's recipe; when that recipe crops (docs/adr/0781 §2), its box also meets the view's
/// area's. The map frame's box test is the caller's.
fn drawn_by<'a>(
    doc: &'a Document,
    sheet: &str,
    item: &str,
    book: Option<&'a kentos_sheet::model::SheetBook>,
) -> impl Fn(&String, &Bounds) -> bool + 'a {
    let map = book
        .and_then(|b| b.sheets.iter().find(|s| s.id == sheet))
        .and_then(|s| s.items.iter().find(|i| i.id == item))
        .and_then(|i| match &i.kind {
            ItemKind::Map(m) => Some(m.clone()),
            _ => None,
        });
    let views = &doc.settings().named_views;
    let recipe = map
        .as_ref()
        .and_then(|m| kentos_sheet::viewport::linked_recipe(m, views))
        .cloned();
    let level = recipe
        .as_ref()
        .and_then(|r| r.level.as_ref().map(|l| l.id.clone()))
        .or_else(|| map.as_ref().and_then(|m| m.level.as_ref().map(|l| l.id.clone())));
    let phase = recipe.as_ref().and_then(|r| r.phase);
    // The cropping view's area as a box: a turned view's is wider, so more may be listed, never fewer.
    let crop = recipe
        .as_ref()
        .filter(|r| r.crop)
        .and(map.as_ref())
        .and_then(|m| kentos_sheet::viewport::linked_view(m, views))
        .map(|v| {
            kentos_sheet::viewport::view_ring(v).iter().fold(
                [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY],
                |b, p| [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])],
            )
        });
    let hidden = recipe.map(|r| r.hidden).unwrap_or_default();
    let draws = recipe_draws(doc, level, phase, hidden);
    move |uid: &String, bounds: &Bounds| {
        if crop.is_some_and(|c| !meets(bounds, &c)) {
            return false;
        }
        let Some(slot) = kentos_domain::Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)) else {
            return true;
        };
        doc.get(slot).is_none_or(&draws)
    }
}

/// Whether a view drawing `level` (none: every level), `phase` and leaving out `hidden` draws an object (docs/adr/0774
/// §4): on that level or on none or on an unknown one (Kat görünümü's rule, docs/adr/0715), its phase not hidden, its
/// category not left out.
pub(crate) fn recipe_draws<'a>(
    doc: &'a Document,
    level: Option<String>,
    phase: Option<kentos_commands::PhaseView>,
    hidden: Vec<kentos_commands::ViewCategory>,
) -> impl Fn(&Entity) -> bool + 'a {
    let known = |id: &str| {
        doc.settings()
            .arch
            .as_ref()
            .is_some_and(|a| a.levels.iter().any(|l| l.id == id))
    };
    move |e: &Entity| {
        let by_uid = |u: &str| {
            kentos_domain::Uuid::parse_str(u)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|s| doc.get(s))
        };
        let on_level = match (&level, e.level(by_uid)) {
            (Some(map), Some(own)) => own == map.as_str() || !known(own),
            _ => true,
        };
        let phased = phase.is_none_or(|v| v.look(e.phase(by_uid)) != kentos_commands::PhaseLook::Hidden);
        on_level
            && phased
            && !kentos_commands::ViewCategory::of_kind(e.kind()).is_some_and(|c| hidden.contains(&c))
    }
}

/// The set, and the frames `uids`' objects affect.
pub(crate) fn set_of(doc: &Document, input: &SheetsSet) -> Result<SheetsSetResult, Stop> {
    let named = match &input.uids {
        Some(uids) => Some(checks::named(doc, uids)?),
        None => None,
    };
    let book = match doc.sheets() {
    None => None,
    Some(kept) => Some(
        read_project_sheets(kept)
            .map_err(|e| {
                Stop::failed(error(
                    codes::BOOK_UNREADABLE,
                    format!(
                        "Projenin pafta kitabı okunamadı ({e}); kitap olduğu gibi duruyor. Pafta tasarımcısında açıp kaydedin."
                    ),
                    None,
                ))
            })?
            .0,
    ),
};
    // The marks and their targets, in the drawing's order.
    let marks: Vec<(String, Option<ViewTarget>)> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::ViewMark(m) => Some((
                doc.uid(Slot(m.base.id))
                    .map(|u| u.to_string())
                    .unwrap_or_default(),
                m.mark.target.clone(),
            )),
            _ => None,
        })
        .collect();
    let mut sheets = Vec::new();
    for s in book.iter().flat_map(|b| b.sheets.iter()) {
        let frames = s
            .items
            .iter()
            .filter_map(|item| {
                let ItemKind::Map(m) = &item.kind else {
                    return None;
                };
                let camera = kentos_sheet::model_scope::Viewport::of_with_views(
                    item,
                    &doc.settings().named_views,
                )
                .ok();
                let corners: Vec<Vec2> = camera
                    .as_ref()
                    .map(|v| {
                        v.transform()
                            .ground_corners(&v.content)
                            .map(|p| Vec2 { x: p[0], y: p[1] })
                            .to_vec()
                    })
                    .unwrap_or_default();
                let target = ViewTarget::sheet(&s.id, &item.id);
                let views = &doc.settings().named_views;
                let linked = match &m.source {
                    kentos_sheet::viewport::Source::ViewLink { view } => Some(kentos_commands::view_key(view)),
                    _ => None,
                };
                let current = linked
                    .as_ref()
                    .and_then(|k| views.iter().find(|v| kentos_commands::view_key(v) == *k));
                let recipe = current.and_then(|v| v.recipe.as_ref());
                let level = recipe
                    .and_then(|r| r.level.as_ref().map(|l| l.id.clone()))
                    .or_else(|| m.level.as_ref().map(|l| l.id.clone()));
                Some(SetFrame {
                    missing: linked.is_some() && current.is_none(),
                    view: linked,
                    level,
                    recipe: recipe.map(|r| r.version),
                    item: item.id.clone(),
                    name: item.name.clone(),
                    scale: match &m.view {
                        MapView::Fixed(_) => kentos_sheet::viewport::effective_fixed_view(
                            item,
                            &doc.settings().named_views,
                        )
                        .ok()
                        .map(|v| v.denominator()),
                        MapView::Atlas(_) => None,
                    },
                    bounds: corners_box(&corners),
                    corners,
                    marks: marks
                        .iter()
                        .filter(|(_, t)| t.as_ref() == Some(&target))
                        .map(|(u, _)| u.clone())
                        .collect(),
                })
            })
            .collect();
        sheets.push(SetSheet {
            id: s.id.clone(),
            name: s.name.clone(),
            series: series_of(&s.name),
            frames,
        });
    }
    // The series: by their letters and digit, Diğer last.
    let mut series: Vec<SheetSeries> = Vec::new();
    for s in &sheets {
        match series.iter_mut().find(|x| x.series == s.series) {
            Some(x) => x.sheets.push(s.id.clone()),
            None => series.push(SheetSeries {
                series: s.series.clone(),
                title: series_title(&s.series).to_owned(),
                sheets: vec![s.id.clone()],
            }),
        }
    }
    series.sort_by(|a, b| {
        (a.series == OTHER_SERIES, &a.series).cmp(&(b.series == OTHER_SERIES, &b.series))
    });
    let affected = named.map(|named| {
        let blocks = crate::blocks::core_blocks(doc.blocks());
        let font = drawing_font(doc.settings().drawing_font);
        let boxes: Vec<(&String, Bounds)> = named
            .iter()
            .map(|(_, _, e, uid)| (*uid, object_bounds(&shape(e), &blocks, font)))
            .collect();
        let pointing: Vec<(&String, &ViewTarget)> = named
            .iter()
            .filter_map(|(_, _, e, uid)| match e {
                Entity::ViewMark(m) => m.mark.target.as_ref().map(|t| (*uid, t)),
                _ => None,
            })
            .collect();
        let mut out = Vec::new();
        for s in &sheets {
            for f in &s.frames {
                let target = ViewTarget::sheet(&s.id, &f.item);
                let mut reasons = Vec::new();
                let mut uids: Vec<String> = Vec::new();
                let mut add = |u: &String| {
                    if !uids.contains(u) {
                        uids.push(u.clone());
                    }
                };
                if let Some(b) = &f.bounds {
                    let draws = drawn_by(doc, s.id.as_str(), &f.item, book.as_ref());
                    let showing: Vec<&String> = boxes
                        .iter()
                        .filter(|(u, x)| meets(x, b) && draws(u, x))
                        .map(|(u, _)| *u)
                        .collect();
                    if !showing.is_empty() {
                        reasons.push(AffectReason::Shows);
                        showing.into_iter().for_each(&mut add);
                    }
                }
                let marking: Vec<&String> = pointing
                    .iter()
                    .filter(|(_, t)| **t == target)
                    .map(|(u, _)| *u)
                    .collect();
                if !marking.is_empty() {
                    reasons.push(AffectReason::Marked);
                    marking.into_iter().for_each(&mut add);
                }
                if f.bounds.is_none() && !named.is_empty() {
                    reasons.push(AffectReason::Unknown);
                }
                if !reasons.is_empty() {
                    // The objects in the input's order.
                    let order = |u: &String| named.iter().position(|(_, _, _, x)| *x == u);
                    uids.sort_by_key(|u| order(u));
                    out.push(AffectedFrame {
                        sheet: s.id.clone(),
                        item: f.item.clone(),
                        reasons,
                        uids,
                    });
                }
            }
        }
        out
    });
    Ok(SheetsSetResult {
        book: input.details.then(|| {
            serde_json::to_value(book.as_ref().cloned().unwrap_or_default()).unwrap_or_default()
        }),
        templates: input.details.then(|| {
            kentos_sheet::template::system_templates()
                .iter()
                .map(|t| serde_json::to_value(t).unwrap_or_default())
                .collect()
        }),
        pictures: input.pictures.then(|| cx_pictures(doc)),
        standard_ratios: input
            .details
            .then(|| kentos_sheet::viewport::standard_ratios().to_vec()),
        named_views: input.details.then(|| doc.settings().named_views.clone()),
        annotation_scales:input.details.then(||doc.annotation_scales()),
        series,
        sheets,
        affected,
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SheetsSet) -> CommandResult<()> {
    match set_of(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &SheetsSet) -> CommandResult<SheetsSetResult> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document (the desktop's Pafta seti window).
pub fn plan_in(doc: &Document, input: &SheetsSet) -> CommandResult<SheetsSetResult> {
    match set_of(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SheetsSet) -> CommandResult<SheetsSetResult> {
    plan(cx, &input)
}

fn cx_pictures(doc: &Document) -> Vec<kentos_commands::SheetPicture> {
    doc.sheets().map(|s| s.pictures.clone()).unwrap_or_default()
}
