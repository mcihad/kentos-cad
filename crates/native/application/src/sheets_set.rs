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
                Some(SetFrame {
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
                    let showing: Vec<&String> = boxes
                        .iter()
                        .filter(|(_, x)| meets(x, b))
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
