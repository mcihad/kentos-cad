//! `road.junctions.edit` v1 (plan/road.md Y12.2, docs/adr/0910 §2): a junction written (new, or the given fields
//! replacing its), rebound to its legs' alignments' present revisions, or removed; one undo step “Kavşak”. The shared
//! cases are `fixtures/commands/v1/road.junctions.edit.json`.

use kentos_commands::{
    CommandResult, CommandWarning, JunctionDef, JunctionKind, JunctionLegDef, JunctionReport, JunctionsEdit,
    JunctionsWritten, RoadEditOperation,
};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_junctions as common;
use crate::road_sections::judged;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Kavşak";

struct Checked {
    list: Vec<JunctionDef>,
    target: Option<String>,
}

/// The legs given, each alignment by its id taken at its present revision.
fn legs_of(doc: &kentos_domain::Document, given: &[JunctionLegDef]) -> Result<Vec<JunctionLegDef>, Stop> {
    let list = alignments::alignments(doc);
    given
        .iter()
        .enumerate()
        .map(|(k, leg)| {
            let (_, a) = alignments::find(list, &leg.alignment, &format!("legs[{k}].alignment"))?;
            Ok(JunctionLegDef { alignment: a.id.clone(), alignment_revision: a.revision, ..leg.clone() })
        })
        .collect()
}

fn check(doc: &kentos_domain::Document, input: &JunctionsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_junctions();
    let mut list = own.to_vec();
    let found = input.junction.as_deref().and_then(|t| common::find(own, t, "junction").ok());
    if input.operation != RoadEditOperation::Set && found.is_none() {
        let name = input.junction.clone().unwrap_or_default();
        return Err(common::find(own, &name, "junction").err().unwrap_or(Stop::Cancelled));
    }
    let legs = match &input.legs {
        Some(l) => Some(legs_of(doc, l)?),
        None => None,
    };
    let separated = |d: &mut JunctionDef| {
        if d.kind == JunctionKind::Separated {
            d.radius = 0.0;
            d.radii = None;
        }
    };
    let target = match (input.operation, found) {
        (RoadEditOperation::Remove, Some((at, _))) => {
            list.remove(at);
            None
        }
        (RoadEditOperation::Rebind, Some((at, old))) => {
            let mut d = old.clone();
            for leg in &mut d.legs {
                leg.alignment_revision =
                    alignments::alignments(doc).iter().find(|a| a.id == leg.alignment).map_or(0, |a| a.revision);
            }
            if d != *old {
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, Some((at, old))) => {
            let given = input.name.is_some()
                || input.kind.is_some()
                || legs.is_some()
                || input.at.is_some()
                || input.radius.is_some()
                || input.radii.is_some();
            if !given {
                return Err(fail(codes::INVALID_JUNCTION, "Değiştirilecek bir şey yok: kavşağın bir alanını verin.".into(), ""));
            }
            let mut d = old.clone();
            if let Some(v) = &input.name {
                d.name = v.clone();
            }
            if let Some(v) = input.kind {
                d.kind = v;
            }
            if let Some(v) = &legs {
                d.legs = v.clone();
            }
            if let Some(v) = input.at {
                d.at = v;
            }
            if let Some(v) = input.radius {
                d.radius = v;
            }
            if let Some(v) = &input.radii {
                d.radii = v.clone();
            }
            separated(&mut d);
            if d != *old {
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, None) => {
            let missing =
                [("name", input.name.is_none()), ("kind", input.kind.is_none()), ("legs", legs.is_none())].into_iter().find(|(_, m)| *m);
            if let Some((field, _)) = missing {
                return Err(fail(codes::INVALID_JUNCTION, "Yeni kavşak için name, kind ve legs verilmeli.".into(), field));
            }
            let (Some(name), Some(kind), Some(legs)) = (&input.name, input.kind, legs) else {
                return Err(Stop::Cancelled);
            };
            let id = input.junction.clone().unwrap_or_else(|| {
                (1..).map(|n| format!("j-{n}")).find(|id| own.iter().all(|x| &x.id != id)).unwrap_or_default()
            });
            let mut d = JunctionDef {
                id: id.clone(),
                name: name.clone(),
                revision: 1,
                kind,
                legs,
                at: input.at.unwrap_or(0),
                radius: input.radius.unwrap_or(0.0),
                radii: input.radii.clone().flatten(),
            };
            separated(&mut d);
            list.push(d);
            Some(id)
        }
        _ => return Err(Stop::Cancelled),
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.junctions = list.clone();
    judged(&road, codes::INVALID_JUNCTION, "Kavşak")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|c| &c.id == id)) {
        common::read(doc, d)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<JunctionsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_junctions(c.list.clone(), STEP) {
        return fail(codes::INVALID_JUNCTION, format!("Kavşak kurallarına uymuyor: {}.", r.0), "").into();
    }
    let junction = match c.target.as_ref().and_then(|id| c.list.iter().find(|x| &x.id == id)) {
        Some(d) => match common::read(doc, d) {
            Ok(r) => Some(r.0),
            Err(stop) => return stop.into(),
        },
        None => None,
    };
    CommandResult::Completed {
        output: JunctionsWritten { junction, junctions: common::listed(&c.list), revision: doc.revision().to_string() },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &JunctionsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &JunctionsEdit) -> CommandResult<JunctionsWritten> {
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

/// Writes the junction: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: JunctionsEdit) -> CommandResult<JunctionsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_JUNCTIONS_EDIT,
        kentos_commands::ROAD_JUNCTIONS_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}

/// The junction `input` would write, read, for a window (no step written): its report and warnings, or why not.
pub fn preview(doc: &kentos_domain::Document, input: &JunctionsEdit) -> Result<(JunctionReport, Vec<CommandWarning>), String> {
    let c = check(doc, input).map_err(crate::road_corridors::words)?;
    let mut copy = doc.reading_copy();
    copy.set_road_junctions(c.list.clone(), STEP).map_err(|r| r.0)?;
    let def = c.target.as_ref().and_then(|id| c.list.iter().find(|x| &x.id == id)).ok_or("Kavşak yok.")?;
    common::read(&copy, def).map_err(crate::road_corridors::words)
}
