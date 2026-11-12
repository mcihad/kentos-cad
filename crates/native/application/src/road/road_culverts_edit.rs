//! `road.culverts.edit` v1 (plan/road.md Y11.2, docs/adr/0909 §2): a culvert written (new, or the given fields replacing
//! its), rebound to its corridor's present revision, or removed; one undo step “Menfez”. The shared cases are
//! `fixtures/commands/v1/road.culverts.edit.json`.

use kentos_commands::{CommandResult, CommandWarning, CulvertDef, CulvertReport, CulvertsEdit, CulvertsWritten, RoadEditOperation};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_corridors as corridors;
use crate::road_culverts as common;
use crate::road_sections::judged;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Menfez";

struct Checked {
    list: Vec<CulvertDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &CulvertsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_culverts();
    let mut list = own.to_vec();
    let found = input.culvert.as_deref().and_then(|t| common::find(own, t, "culvert").ok());
    if input.operation != RoadEditOperation::Set && found.is_none() {
        let name = input.culvert.clone().unwrap_or_default();
        return Err(common::find(own, &name, "culvert").err().unwrap_or(Stop::Cancelled));
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
                || input.station.is_some()
                || input.skew.is_some()
                || input.shape.is_some()
                || input.invert.is_some()
                || input.slope.is_some()
                || input.flow.is_some()
                || input.extend.is_some()
                || input.cover.is_some()
                || input.material.is_some()
                || input.roughness.is_some();
            if !given {
                return Err(fail(codes::INVALID_CULVERT, "Değiştirilecek bir şey yok: menfezin bir alanını verin.".into(), ""));
            }
            let mut d = old.clone();
            if let Some(n) = &input.name {
                d.name = n.clone();
            }
            if let Some(c) = &corridor {
                d.corridor = c.id.clone();
                d.corridor_revision = c.revision;
            }
            if let Some(v) = input.station {
                d.station = v;
            }
            if let Some(v) = input.skew {
                d.skew = v;
            }
            if let Some(v) = &input.shape {
                d.shape = v.clone();
            }
            if let Some(v) = input.invert {
                d.invert = v;
            }
            if let Some(v) = input.slope {
                d.slope = v;
            }
            if let Some(v) = input.flow {
                d.flow = v;
            }
            if let Some(v) = input.extend {
                d.extend = v;
            }
            if let Some(v) = input.cover {
                d.cover = v;
            }
            if let Some(v) = &input.material {
                d.material = v.clone();
            }
            if let Some(v) = input.roughness {
                d.roughness = v;
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
                ("station", input.station.is_none()),
                ("shape", input.shape.is_none()),
                ("invert", input.invert.is_none()),
                ("slope", input.slope.is_none()),
                ("flow", input.flow.is_none()),
            ]
            .into_iter()
            .find(|(_, m)| *m);
            if let Some((field, _)) = missing {
                return Err(fail(
                    codes::INVALID_CULVERT,
                    "Yeni menfez için name, corridor, station, shape, invert, slope ve flow verilmeli.".into(),
                    field,
                ));
            }
            let (Some(name), Some(c), Some(station), Some(shape), Some(invert), Some(slope), Some(flow)) =
                (&input.name, &corridor, input.station, &input.shape, input.invert, input.slope, input.flow)
            else {
                return Err(Stop::Cancelled);
            };
            let id = input.culvert.clone().unwrap_or_else(|| {
                (1..).map(|n| format!("m-{n}")).find(|id| own.iter().all(|x| &x.id != id)).unwrap_or_default()
            });
            list.push(CulvertDef {
                id: id.clone(),
                name: name.clone(),
                corridor: c.id.clone(),
                corridor_revision: c.revision,
                revision: 1,
                station,
                skew: input.skew.unwrap_or(0.0),
                shape: shape.clone(),
                invert,
                slope,
                flow,
                extend: input.extend.unwrap_or(0.0),
                cover: input.cover.flatten(),
                material: input.material.clone().flatten(),
                roughness: input.roughness.flatten(),
            });
            Some(id)
        }
        _ => return Err(Stop::Cancelled),
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.culverts = list.clone();
    judged(&road, codes::INVALID_CULVERT, "Menfez")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|c| &c.id == id)) {
        common::read(doc, d)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<CulvertsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_culverts(c.list.clone(), STEP) {
        return fail(codes::INVALID_CULVERT, format!("Menfez kurallarına uymuyor: {}.", r.0), "").into();
    }
    let culvert = match c.target.as_ref().and_then(|id| c.list.iter().find(|x| &x.id == id)) {
        Some(d) => match common::read(doc, d) {
            Ok(r) => Some(r.report),
            Err(stop) => return stop.into(),
        },
        None => None,
    };
    CommandResult::Completed {
        output: CulvertsWritten { culvert, culverts: common::listed(&c.list), revision: doc.revision().to_string() },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &CulvertsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &CulvertsEdit) -> CommandResult<CulvertsWritten> {
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

/// Writes the culvert: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CulvertsEdit) -> CommandResult<CulvertsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_CULVERTS_EDIT,
        kentos_commands::ROAD_CULVERTS_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}

/// The culvert `input` would write, read, for a window (no step written): its report and warnings, or why not.
pub fn preview(doc: &kentos_domain::Document, input: &CulvertsEdit) -> Result<(CulvertReport, Vec<CommandWarning>), String> {
    let c = check(doc, input).map_err(crate::road_corridors::words)?;
    let mut copy = doc.reading_copy();
    copy.set_road_culverts(c.list.clone(), STEP).map_err(|r| r.0)?;
    let def = c.target.as_ref().and_then(|id| c.list.iter().find(|x| &x.id == id)).ok_or("Menfez yok.")?;
    common::read(&copy, def).map(|r| (r.report, r.warnings)).map_err(crate::road_corridors::words)
}
