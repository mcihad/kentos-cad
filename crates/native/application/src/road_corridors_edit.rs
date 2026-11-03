//! `road.corridors.edit` v1 (plan/road.md Y7, docs/adr/0905 §1): a corridor written (new, or the given fields
//! replacing its), rebound to its alignment's present revision, or removed; one undo step “Koridor”. The shared cases
//! are `fixtures/commands/v1/road.corridors.edit.json`.

use kentos_commands::{CommandResult, CorridorDef, CorridorsEdit, CorridorsWritten, RoadEditOperation};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_corridors as common;
use crate::road_profiles as profiles;
use crate::road_sections::{find_template, judged};
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Koridor";

struct Checked {
    list: Vec<CorridorDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &CorridorsEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = doc.road_corridors();
    let mut list = own.to_vec();
    let found = input.corridor.as_deref().and_then(|t| common::find(own, t, "corridor").ok());
    if matches!(input.operation, RoadEditOperation::Remove | RoadEditOperation::Rebind) && found.is_none() {
        let name = input.corridor.clone().unwrap_or_default();
        return Err(common::find(own, &name, "corridor").err().unwrap_or(Stop::Cancelled));
    }
    let alignment = match input.alignment.as_deref() {
        Some(a) => Some(alignments::find(doc.road_alignments(), a, "alignment")?.1.clone()),
        None => None,
    };
    let profile = match input.profile.as_deref() {
        Some(p) => Some(profiles::find(doc.road_profiles(), p, "profile")?.1.id.clone()),
        None => None,
    };
    let surface = match input.surface.as_deref() {
        Some(s) => Some(profiles::surface_id(doc, s, "surface")?),
        None => None,
    };
    let superelevation = match input.superelevation.as_deref() {
        None => None,
        Some("") => Some(None),
        Some(e) => Some(Some(
            crate::road_superelevations::find(doc.road_superelevations(), e, "superelevation")?.1.id.clone(),
        )),
    };
    let regions = match &input.regions {
        None => None,
        Some(rs) => {
            let mut out = rs.clone();
            for (i, r) in out.iter_mut().enumerate() {
                r.template = find_template(doc.road_templates(), &r.template, &format!("regions[{i}].template"))?.1.id.clone();
            }
            Some(out)
        }
    };
    let sag = input.sag.map(|t| (t != 0.0).then_some(t));
    let target = match (input.operation, found) {
        (RoadEditOperation::Remove | RoadEditOperation::Rebind, None) => return Err(Stop::Cancelled),
        (RoadEditOperation::Remove, Some((at, _))) => {
            list.remove(at);
            None
        }
        (RoadEditOperation::Rebind, Some((at, old))) => {
            let now = doc.road_alignments().iter().find(|a| a.id == old.alignment).map_or(0, |a| a.revision);
            let mut d = old.clone();
            if d.alignment_revision != now {
                d.alignment_revision = now;
                d.revision = old.revision.saturating_add(1);
            }
            list[at] = d;
            Some(list[at].id.clone())
        }
        (RoadEditOperation::Set, Some((at, old))) => {
            let given = input.name.is_some()
                || alignment.is_some()
                || profile.is_some()
                || surface.is_some()
                || superelevation.is_some()
                || input.left.is_some()
                || input.right.is_some()
                || input.tangent.is_some()
                || input.curve.is_some()
                || sag.is_some()
                || input.profile_points.is_some()
                || input.critical.is_some()
                || input.stations.is_some()
                || regions.is_some();
            if !given {
                return Err(fail(
                    codes::INVALID_ROAD_CORRIDOR,
                    "Değiştirilecek bir şey yok: name, alignment, profile, surface, superelevation, left, right, tangent, curve, sag, profilePoints, critical, stations ya da regions verin.".into(),
                    "",
                ));
            }
            let mut d = old.clone();
            if let Some(n) = &input.name {
                d.name = n.clone();
            }
            if let Some(a) = &alignment {
                d.alignment = a.id.clone();
                d.alignment_revision = a.revision;
            }
            if let Some(p) = profile {
                d.profile = p;
            }
            if let Some(s) = surface {
                d.surface = s;
            }
            if let Some(e) = superelevation {
                d.superelevation = e;
            }
            if let Some(v) = input.left {
                d.left = v;
            }
            if let Some(v) = input.right {
                d.right = v;
            }
            if let Some(v) = input.tangent {
                d.tangent = v;
            }
            if let Some(v) = input.curve {
                d.curve = v;
            }
            if let Some(v) = sag {
                d.sag = v;
            }
            if let Some(v) = input.profile_points {
                d.profile_points = v;
            }
            if let Some(v) = input.critical {
                d.critical = v;
            }
            if let Some(v) = &input.stations {
                d.stations = v.clone();
            }
            if let Some(v) = regions {
                d.regions = v;
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
                ("alignment", alignment.is_none()),
                ("profile", profile.is_none()),
                ("surface", surface.is_none()),
                ("left", input.left.is_none()),
                ("right", input.right.is_none()),
                ("tangent", input.tangent.is_none()),
                ("curve", input.curve.is_none()),
                ("regions", regions.is_none()),
            ]
            .into_iter()
            .find(|(_, m)| *m);
            let (Some(name), Some(a), Some(p), Some(s), Some(l), Some(r), Some(t), Some(c), Some(rs), None) = (
                &input.name,
                alignment,
                profile,
                surface,
                input.left,
                input.right,
                input.tangent,
                input.curve,
                regions,
                missing,
            ) else {
                return Err(fail(
                    codes::INVALID_ROAD_CORRIDOR,
                    "Yeni koridor için name, alignment, profile, surface, left, right, tangent, curve ve regions verilmeli.".into(),
                    missing.map_or("", |(f, _)| f),
                ));
            };
            let id = input.corridor.clone().unwrap_or_else(|| {
                (1..).map(|n| format!("c-{n}")).find(|id| own.iter().all(|s| &s.id != id)).unwrap_or_default()
            });
            list.push(CorridorDef {
                id: id.clone(),
                name: name.clone(),
                alignment: a.id.clone(),
                alignment_revision: a.revision,
                revision: 1,
                profile: p,
                surface: s,
                superelevation: superelevation.flatten(),
                left: l,
                right: r,
                tangent: t,
                curve: c,
                sag: sag.flatten(),
                profile_points: input.profile_points.unwrap_or(false),
                critical: input.critical.unwrap_or(false),
                stations: input.stations.clone().unwrap_or_default(),
                regions: rs,
            });
            Some(id)
        }
    };
    let mut road = doc.settings().road.clone().unwrap_or_default();
    road.corridors = list.clone();
    judged(&road, codes::INVALID_ROAD_CORRIDOR, "Koridor")?;
    if let Some(d) = target.as_ref().and_then(|id| list.iter().find(|s| &s.id == id)) {
        common::make(doc, d, false)?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<CorridorsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_corridors(c.list.clone(), STEP) {
        return fail(codes::INVALID_ROAD_CORRIDOR, format!("Koridor kurallarına uymuyor: {}.", r.0), "").into();
    }
    let corridor = c.target.as_ref().and_then(|id| c.list.iter().find(|s| &s.id == id)).map(|d| common::report(doc, d));
    CommandResult::Completed {
        output: CorridorsWritten {
            corridor,
            corridors: common::listed(doc, &c.list),
            revision: doc.revision().to_string(),
        },
        warnings: vec![],
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &CorridorsEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed { output: (), warnings: vec![] },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &CorridorsEdit) -> CommandResult<CorridorsWritten> {
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

/// Writes the corridor: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CorridorsEdit) -> CommandResult<CorridorsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_CORRIDORS_EDIT,
        kentos_commands::ROAD_CORRIDORS_EDIT_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        },
    )
}
