//! `road.platforms.edit` v1 (plan/road.md Y14, docs/adr/0912): a graded pad written (new, or the given fields replacing
//! its) or removed; one undo step “Platform”. The shared cases are `fixtures/commands/v1/road.platforms.edit.json`.

use kentos_commands::{
    CommandResult, CommandWarning, PlatformDef, PlatformReport, PlatformsEdit, PlatformsWritten, RoadEditOperation,
};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_platforms as common;
use crate::road_sections::judged;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Platform";

struct Checked {
    list: Vec<PlatformDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &PlatformsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_platforms();
    let mut list = own.to_vec();
    let found = input.platform.as_deref().and_then(|t| common::find(own, t, "platform").ok());
    if input.operation == RoadEditOperation::Remove && found.is_none() {
        let name = input.platform.clone().unwrap_or_default();
        return Err(common::find(own, &name, "platform").err().unwrap_or(Stop::Cancelled));
    }
    let surface = match input.surface.as_deref() {
        Some(s) => Some(crate::road_profiles::surface_id(doc, s, "surface")?),
        None => None,
    };
    let target = match (input.operation, found) {
        (RoadEditOperation::Rebind, _) => {
            return Err(fail(codes::INVALID_PLATFORM, "Platformun yeniden bağlanacak bir kaynağı yok: yüzeyi değişince her okumada yeniden tesviye edilir.".into(), "operation"));
        }
        (RoadEditOperation::Remove, Some((at, _))) => {
            list.remove(at);
            None
        }
        (RoadEditOperation::Set, Some((at, old))) => {
            let given = input.name.is_some()
                || input.corners.is_some()
                || input.base.is_some()
                || input.grade.is_some()
                || input.direction.is_some()
                || input.cut.is_some()
                || input.fill.is_some()
                || input.step.is_some()
                || input.fan.is_some()
                || input.reach.is_some()
                || surface.is_some();
            if !given {
                return Err(fail(codes::INVALID_PLATFORM, "Değiştirilecek bir şey yok: platformun bir alanını verin.".into(), ""));
            }
            let mut d = old.clone();
            if let Some(v) = &input.name {
                d.name = v.clone();
            }
            if let Some(v) = &input.corners {
                d.corners = v.clone();
            }
            if let Some(v) = input.base {
                d.base = v;
            }
            if let Some(v) = input.grade {
                d.grade = v;
            }
            if let Some(v) = input.direction {
                d.direction = v;
            }
            if let Some(v) = input.cut {
                d.cut = v;
            }
            if let Some(v) = input.fill {
                d.fill = v;
            }
            if let Some(v) = input.step {
                d.step = v;
            }
            if let Some(v) = input.fan {
                d.fan = v;
            }
            if let Some(v) = input.reach {
                d.reach = v;
            }
            if let Some(v) = &surface {
                d.surface = v.clone();
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
                ("corners", input.corners.is_none()),
                ("base", input.base.is_none()),
                ("cut", input.cut.is_none()),
                ("fill", input.fill.is_none()),
                ("step", input.step.is_none()),
                ("fan", input.fan.is_none()),
                ("reach", input.reach.is_none()),
                ("surface", surface.is_none()),
            ]
            .into_iter()
            .find(|(_, m)| *m);
            if let Some((field, _)) = missing {
                return Err(fail(
                    codes::INVALID_PLATFORM,
                    "Yeni platform için name, corners, base, cut, fill, step, fan, reach ve surface verilmeli.".into(),
                    field,
                ));
            }
            let (Some(name), Some(corners), Some(base), Some(cut), Some(fill), Some(step), Some(fan), Some(reach), Some(s)) = (
                &input.name,
                &input.corners,
                input.base,
                input.cut,
                input.fill,
                input.step,
                input.fan,
                input.reach,
                &surface,
            ) else {
                return Err(Stop::Cancelled);
            };
            let id = input.platform.clone().unwrap_or_else(|| {
                (1..).map(|n| format!("p-{n}")).find(|id| own.iter().all(|x| &x.id != id)).unwrap_or_default()
            });
            list.push(PlatformDef {
                id: id.clone(),
                name: name.clone(),
                revision: 1,
                corners: corners.clone(),
                base,
                grade: input.grade.unwrap_or(0.0),
                direction: input.direction.unwrap_or(0.0),
                cut,
                fill,
                step,
                fan,
                reach,
                surface: s.clone(),
            });
            Some(id)
        }
        _ => return Err(Stop::Cancelled),
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.platforms = list.clone();
    judged(&road, codes::INVALID_PLATFORM, "Platform")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|c| &c.id == id)) {
        kentos_road::grading::ring_of(&common::platform_of(d))
            .map_err(|e| fail(codes::INVALID_PLATFORM, format!("Platform tesviye edilemiyor: {e}"), "corners"))?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<PlatformsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_platforms(c.list.clone(), STEP) {
        return fail(codes::INVALID_PLATFORM, format!("Platform kurallarına uymuyor: {}.", r.0), "").into();
    }
    let mut warnings = Vec::new();
    let platform = match c.target.as_ref().and_then(|id| c.list.iter().find(|x| &x.id == id)) {
        Some(d) => match common::read(doc, d) {
            Ok(r) => {
                warnings = r.warnings;
                Some(r.report)
            }
            Err(stop) => return stop.into(),
        },
        None => None,
    };
    CommandResult::Completed {
        output: PlatformsWritten { platform, platforms: common::listed(&c.list), revision: doc.revision().to_string() },
        warnings,
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &PlatformsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &PlatformsEdit) -> CommandResult<PlatformsWritten> {
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

/// Writes the pad: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PlatformsEdit) -> CommandResult<PlatformsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_PLATFORMS_EDIT,
        kentos_commands::ROAD_PLATFORMS_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}

/// The pad `input` would write, graded, for a window (no step written): its report and warnings, or why not.
pub fn preview(doc: &kentos_domain::Document, input: &PlatformsEdit) -> Result<(PlatformReport, Vec<CommandWarning>), String> {
    let c = check(doc, input).map_err(crate::road_corridors::words)?;
    let mut copy = doc.reading_copy();
    copy.set_road_platforms(c.list.clone(), STEP).map_err(|r| r.0)?;
    let def = c.target.as_ref().and_then(|id| c.list.iter().find(|x| &x.id == id)).ok_or("Platform yok.")?;
    common::read(&copy, def).map(|r| (r.report, r.warnings)).map_err(crate::road_corridors::words)
}
