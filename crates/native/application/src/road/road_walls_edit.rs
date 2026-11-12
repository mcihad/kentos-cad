//! `road.walls.edit` v1 (plan/road.md Y11.3, docs/adr/0909 §3): a retaining wall written (new, or the given fields
//! replacing its), rebound to its corridor's present revision, or removed; one undo step “İstinat duvarı”. The shared
//! cases are `fixtures/commands/v1/road.walls.edit.json`.

use kentos_commands::{
    CommandResult, CommandWarning, RetainingWallDef, RetainingWallReport, RetainingWallsEdit, RetainingWallsWritten,
    RoadEditOperation,
};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_corridors as corridors;
use crate::road_sections::judged;
use crate::road_terrains::fail;
use crate::road_walls as common;

pub(crate) const STEP: &str = "İstinat duvarı";

struct Checked {
    list: Vec<RetainingWallDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &RetainingWallsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_walls();
    let mut list = own.to_vec();
    let found = input.wall.as_deref().and_then(|t| common::find(own, t, "wall").ok());
    if input.operation != RoadEditOperation::Set && found.is_none() {
        let name = input.wall.clone().unwrap_or_default();
        return Err(common::find(own, &name, "wall").err().unwrap_or(Stop::Cancelled));
    }
    let corridor = match input.corridor.as_deref() {
        Some(c) => Some(corridors::find(doc.road_corridors(), c, "corridor")?.1.clone()),
        None => None,
    };
    let target = match (input.operation, found) {
        (RoadEditOperation::Remove, Some((at, _))) => {
            list.remove(at);
            None
        }
        (RoadEditOperation::Rebind, Some((at, old))) => {
            let now = doc.road_corridors().iter().find(|c| c.id == old.corridor).map_or(0, |c| c.revision);
            let mut d = old.clone();
            d.corridor_revision = now;
            if d != *old {
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, Some((at, old))) => {
            let given = input.name.is_some()
                || corridor.is_some()
                || input.side.is_some()
                || input.from.is_some()
                || input.to.is_some()
                || input.step.is_some()
                || input.shape.is_some()
                || input.highest.is_some()
                || input.material.is_some();
            if !given {
                return Err(fail(codes::INVALID_RETAINING_WALL, "Değiştirilecek bir şey yok: duvarın bir alanını verin.".into(), ""));
            }
            let mut d = old.clone();
            if let Some(n) = &input.name {
                d.name = n.clone();
            }
            if let Some(c) = &corridor {
                d.corridor = c.id.clone();
                d.corridor_revision = c.revision;
            }
            if let Some(v) = input.side {
                d.side = v;
            }
            if let Some(v) = input.from {
                d.from = v;
            }
            if let Some(v) = input.to {
                d.to = v;
            }
            if let Some(v) = input.step {
                d.step = v;
            }
            if let Some(v) = &input.shape {
                d.shape = v.clone();
            }
            if let Some(v) = input.highest {
                d.highest = v;
            }
            if let Some(v) = &input.material {
                d.material = v.clone();
            }
            if d != *old {
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, None) => {
            let missing = [
                ("name", input.name.is_none()),
                ("corridor", corridor.is_none()),
                ("side", input.side.is_none()),
                ("from", input.from.is_none()),
                ("to", input.to.is_none()),
                ("step", input.step.is_none()),
                ("shape", input.shape.is_none()),
            ]
            .into_iter()
            .find(|(_, m)| *m);
            if let Some((field, _)) = missing {
                return Err(fail(
                    codes::INVALID_RETAINING_WALL,
                    "Yeni istinat duvarı için name, corridor, side, from, to, step ve shape verilmeli.".into(),
                    field,
                ));
            }
            let (Some(name), Some(c), Some(side), Some(from), Some(to), Some(step), Some(shape)) =
                (&input.name, &corridor, input.side, input.from, input.to, input.step, &input.shape)
            else {
                return Err(Stop::Cancelled);
            };
            let id = input.wall.clone().unwrap_or_else(|| {
                (1..).map(|n| format!("d-{n}")).find(|id| own.iter().all(|x| &x.id != id)).unwrap_or_default()
            });
            list.push(RetainingWallDef {
                id: id.clone(),
                name: name.clone(),
                corridor: c.id.clone(),
                corridor_revision: c.revision,
                revision: 1,
                side,
                from,
                to,
                step,
                shape: shape.clone(),
                highest: input.highest.flatten(),
                material: input.material.clone().flatten(),
            });
            Some(id)
        }
        _ => return Err(Stop::Cancelled),
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.walls = list.clone();
    judged(&road, codes::INVALID_RETAINING_WALL, "İstinat duvarı")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|c| &c.id == id)) {
        common::read(doc, d)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<RetainingWallsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_walls(c.list.clone(), STEP) {
        return fail(codes::INVALID_RETAINING_WALL, format!("İstinat duvarı kurallarına uymuyor: {}.", r.0), "").into();
    }
    let wall = match c.target.as_ref().and_then(|id| c.list.iter().find(|x| &x.id == id)) {
        Some(d) => match common::read(doc, d) {
            Ok(r) => Some(r.report),
            Err(stop) => return stop.into(),
        },
        None => None,
    };
    CommandResult::Completed {
        output: RetainingWallsWritten { wall, walls: common::listed(&c.list), revision: doc.revision().to_string() },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RetainingWallsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RetainingWallsEdit) -> CommandResult<RetainingWallsWritten> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Writes the wall: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: RetainingWallsEdit) -> CommandResult<RetainingWallsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_WALLS_EDIT,
        kentos_commands::ROAD_WALLS_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}

/// The wall `input` would write, read, for a window (no step written): its report and warnings, or why not.
pub fn preview(
    doc: &kentos_domain::Document,
    input: &RetainingWallsEdit,
) -> Result<(RetainingWallReport, Vec<CommandWarning>), String> {
    let c = check(doc, input).map_err(crate::road_corridors::words)?;
    let mut copy = doc.reading_copy();
    copy.set_road_walls(c.list.clone(), STEP).map_err(|r| r.0)?;
    let def = c.target.as_ref().and_then(|id| c.list.iter().find(|x| &x.id == id)).ok_or("İstinat duvarı yok.")?;
    common::read(&copy, def).map(|r| (r.report, r.warnings)).map_err(crate::road_corridors::words)
}
