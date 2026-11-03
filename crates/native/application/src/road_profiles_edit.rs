//! `road.profiles.edit` v1 (plan/road.md Y4, docs/adr/0902 §6): a profile's name, PVIs or surface set, the profile
//! rebound to its alignment's present revision, or removed; one undo step “Boykesiti düzenle”. The shared cases are
//! `fixtures/commands/v1/road.profiles.edit.json`.

use kentos_commands::{CommandResult, ProfileDef, ProfilesEdit, ProfilesEditOperation, ProfilesWritten};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_profiles as common;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Boykesiti düzenle";

struct Checked {
    list: Vec<ProfileDef>,
    target: Option<String>,
}

fn check(doc: &kentos_domain::Document, input: &ProfilesEdit) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = common::profiles(doc);
    let (at, old) = common::find(own, &input.profile, "profile")?;
    let mut list = own.to_vec();
    let target = match input.operation {
        ProfilesEditOperation::Set => {
            if input.name.is_none() && input.pvis.is_none() && input.surface.is_none() {
                return Err(fail(
                    codes::INVALID_ROAD_PROFILE,
                    "Değiştirilecek bir şey yok: name, pvis ya da surface verin.".into(),
                    "",
                ));
            }
            let mut def = old.clone();
            if let Some(s) = input.surface.as_deref() {
                def.surface = if s.is_empty() { None } else { Some(common::surface_id(doc, s, "surface")?) };
            }
            if let Some(n) = &input.name {
                def.name = n.clone();
            }
            if let Some(p) = &input.pvis {
                def.pvis = p.clone();
            }
            if def != *old {
                def.revision = old.revision.saturating_add(1);
            }
            list[at] = def;
            common::judged(doc, &list)?;
            common::build(doc, &list[at])?;
            Some(list[at].id.clone())
        }
        ProfilesEditOperation::Rebind => {
            let (stale, now) = common::stale(doc, old);
            let mut def = old.clone();
            if stale {
                def.alignment_revision = now;
                def.revision = old.revision.saturating_add(1);
            }
            list[at] = def;
            common::build(doc, &list[at])?;
            Some(list[at].id.clone())
        }
        ProfilesEditOperation::Remove => {
            let sets: Vec<&str> =
                doc.road_sections().iter().filter(|s| s.profile.as_deref() == Some(&old.id)).map(|s| s.name.as_str()).collect();
            if !sets.is_empty() {
                return Err(fail(
                    codes::PROFILE_IN_USE,
                    format!("“{}” boykesitini şu kesit takımları kullanıyor: {}. Önce takımlardan kaldırın.", old.name, sets.join(", ")),
                    "profile",
                ));
            }
            let corridors: Vec<&str> =
                doc.road_corridors().iter().filter(|c| c.profile == old.id).map(|c| c.name.as_str()).collect();
            if !corridors.is_empty() {
                return Err(fail(
                    codes::PROFILE_IN_USE,
                    format!("“{}” boykesitini şu koridorlar kullanıyor: {}. Önce koridorlardan kaldırın.", old.name, corridors.join(", ")),
                    "profile",
                ));
            }
            list.remove(at);
            None
        }
    };
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, target })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesEdit) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: vec![],
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesEdit) -> CommandResult<ProfilesWritten> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = common::write(&mut ExecutionContext::new(&mut copy), c.list, STEP, c.target.as_deref());
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Writes the edit: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesEdit) -> CommandResult<ProfilesWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_PROFILES_EDIT,
        kentos_commands::ROAD_PROFILES_EDIT_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ProfilesEdit) -> CommandResult<ProfilesWritten> {
    match check(cx.doc, &input) {
        Ok(c) => common::write(cx, c.list, STEP, c.target.as_deref()),
        Err(stop) => stop.into(),
    }
}
