//! `cad.navigator.read` v1 (docs/adr/0778; Revit's Project Browser, Archicad's Navigator): the project's levels, named
//! views, sheets and schedules and how they link — a level's objects, the views whose recipe draws it and the maps that
//! draw it; a view's linked maps; a sheet's maps with their view, level and scale; a schedule's objects — and, given one
//! object, where it shows. The maps' answers are `cad.sheets.set`'s (`set_of`, its `shows`); a view draws an object when
//! the view's area's box meets the object's and its recipe takes it (`recipe_draws`). Read-only. The shared cases are
//! `fixtures/commands/v1/cad.navigator.read.json`.

use kentos_commands::{
    AffectReason, CommandResult, Entity, NamedView, NavLevel, NavMap, NavObject, NavSchedule, NavSheet,
    NavSheetMap, NavView, NavigatorRead, NavigatorReadResult, SheetsSet, TableSource, view_key,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::arrange::object_bounds;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::geometry::{drawing_font, shape};
use crate::sheets_set::{recipe_draws, set_of};

/// The box of a named view's area: its four corners turned by its twist.
fn view_box(v: &NamedView) -> [f64; 4] {
    let (s, c) = v.twist.sin_cos();
    let (hw, hh) = (v.width / 2.0, v.height / 2.0);
    let mut b = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    for (x, y) in [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)] {
        let (px, py) = (v.center.x + x * c - y * s, v.center.y + x * s + y * c);
        b = [b[0].min(px), b[1].min(py), b[2].max(px), b[3].max(py)];
    }
    b
}

fn meets(a: &Bounds, b: &[f64; 4]) -> bool {
    a.min_x <= b[2] && b[0] <= a.max_x && a.min_y <= b[3] && b[1] <= a.max_y
}

fn uid_text(doc: &Document, e: &Entity) -> String {
    doc.uid(kentos_domain::Slot(e.base().id))
        .map(|u| u.to_string())
        .unwrap_or_default()
}

pub(crate) fn read(doc: &Document, input: &NavigatorRead) -> Result<NavigatorReadResult, Stop> {
    let object = match &input.uid {
        None => None,
        Some(uid) => {
            let found = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s));
            let Some(e) = found else {
                return Err(Stop::failed(error(
                    codes::ENTITY_NOT_FOUND,
                    format!(
                        "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                    ),
                    Some("uid".into()),
                )));
            };
            Some((uid.clone(), e))
        }
    };
    let set = set_of(
        doc,
        &SheetsSet {
            uids: object.as_ref().map(|(u, _)| vec![u.clone()]),
            ..Default::default()
        },
    )?;
    let by_uid = |u: &str| Uuid::parse_str(u).ok().and_then(|u| doc.slot_of(u)).and_then(|s| doc.get(s));
    let levels_of = doc.settings().arch.as_ref().map(|a| a.levels.clone()).unwrap_or_default();
    let known = |id: &str| levels_of.iter().any(|l| l.id == id);
    let views = &doc.settings().named_views;

    // The objects on each level, and those on none (or on a level the project no longer has).
    let mut counts = vec![0u32; levels_of.len()];
    let mut unleveled = 0u32;
    for e in doc.entities() {
        match e.level(by_uid).and_then(|id| levels_of.iter().position(|l| l.id == id)) {
            Some(i) => counts[i] += 1,
            None => unleveled += 1,
        }
    }

    let sheet_maps = |pick: &dyn Fn(&kentos_commands::SetFrame) -> bool| -> Vec<NavMap> {
        set.sheets
            .iter()
            .flat_map(|s| {
                s.frames.iter().filter(|f| pick(f)).map(|f| NavMap {
                    sheet: s.id.clone(),
                    item: f.item.clone(),
                })
            })
            .collect()
    };

    let levels = levels_of
        .iter()
        .zip(&counts)
        .map(|(l, n)| NavLevel {
            id: l.id.clone(),
            name: l.name.clone(),
            elevation: l.elevation,
            objects: *n,
            views: views
                .iter()
                .filter(|v| {
                    v.recipe
                        .as_ref()
                        .and_then(|r| r.level.as_ref())
                        .is_some_and(|x| x.id == l.id)
                })
                .map(view_key)
                .collect(),
            maps: sheet_maps(&|f| f.level.as_deref() == Some(l.id.as_str())),
        })
        .collect();

    let nav_views = views
        .iter()
        .map(|v| {
            let key = view_key(v);
            NavView {
                level: v.recipe.as_ref().and_then(|r| r.level.as_ref()).map(|l| l.id.clone()),
                recipe: v.recipe.as_ref().map(|r| r.version),
                scale: v.recipe.as_ref().and_then(|r| r.scale),
                maps: sheet_maps(&|f| f.view.as_deref() == Some(key.as_str())),
                id: key,
                name: v.name.clone(),
            }
        })
        .collect();

    let sheets = set
        .sheets
        .iter()
        .map(|s| NavSheet {
            id: s.id.clone(),
            name: s.name.clone(),
            series: s.series.clone(),
            maps: s
                .frames
                .iter()
                .map(|f| NavSheetMap {
                    item: f.item.clone(),
                    name: f.name.clone(),
                    view: f.view.clone(),
                    missing: f.missing,
                    level: f.level.clone(),
                    scale: f.scale,
                })
                .collect(),
        })
        .collect();

    // The schedules: the tables reading the model, with their listed objects.
    let mut schedules = Vec::new();
    let mut listing: Vec<String> = Vec::new();
    for e in doc.entities() {
        let Entity::Table(t) = e else { continue };
        let Some(TableSource::Schedule { of, members, .. }) = &t.source else { continue };
        let mut seen: Vec<&String> = members.iter().flatten().collect();
        seen.sort();
        seen.dedup();
        let uid = uid_text(doc, e);
        if let Some((target, _)) = &object
            && seen.binary_search(&target).is_ok()
        {
            listing.push(uid.clone());
        }
        schedules.push(NavSchedule {
            uid,
            of: *of,
            objects: seen.len() as u32,
        });
    }

    let object = object.map(|(uid, e)| {
        let level = e.level(by_uid).filter(|l| known(l)).map(str::to_owned);
        let blocks = crate::blocks::core_blocks(doc.blocks());
        let font = drawing_font(doc.settings().drawing_font);
        let bounds = object_bounds(&shape(e), &blocks, font);
        let views = views
            .iter()
            .filter(|v| {
                let r = v.recipe.clone().unwrap_or_default();
                let draws = recipe_draws(doc, r.level.map(|l| l.id), r.phase, r.hidden);
                meets(&bounds, &view_box(v)) && draws(e)
            })
            .map(view_key)
            .collect();
        let maps = set
            .affected
            .iter()
            .flatten()
            .filter(|f| f.reasons.contains(&AffectReason::Shows))
            .map(|f| NavMap {
                sheet: f.sheet.clone(),
                item: f.item.clone(),
            })
            .collect();
        let tags = doc
            .entities()
            .filter_map(|x| match x {
                Entity::Tag(t) if t.tag.of == uid => Some(uid_text(doc, x)),
                _ => None,
            })
            .collect();
        NavObject {
            kind: e.kind().to_owned(),
            level,
            views,
            maps,
            schedules: listing,
            tags,
            uid,
        }
    });

    Ok(NavigatorReadResult {
        levels,
        unleveled,
        views: nav_views,
        sheets,
        schedules,
        object,
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &NavigatorRead) -> CommandResult<()> {
    match read(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &NavigatorRead) -> CommandResult<NavigatorReadResult> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document (the desktop's Gezgin panel).
pub fn plan_in(doc: &Document, input: &NavigatorRead) -> CommandResult<NavigatorReadResult> {
    match read(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(cx: &mut ExecutionContext<'_>, input: NavigatorRead) -> CommandResult<NavigatorReadResult> {
    plan(cx, &input)
}
