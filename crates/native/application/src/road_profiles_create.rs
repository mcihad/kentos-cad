//! `road.profiles.create` v1 (plan/road.md Y4, docs/adr/0902): a vertical alignment from PVIs on an alignment, last in
//! the road project's profiles, holding the alignment's present revision; one undo step “Boykesit oluştur”. The
//! shared cases are `fixtures/commands/v1/road.profiles.create.json`.
//!
//! The checks, in the contract's order: a road project; the alignment and the surface; the definition with the
//! profiles; the PVIs' geometry; the expected revision.

use kentos_commands::{CommandResult, ProfileDef, ProfilesCreate, ProfilesWritten};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::road_alignments as alignments;
use crate::road_profiles as common;

pub(crate) const STEP: &str = "Boykesit oluştur";

struct Checked {
    list: Vec<ProfileDef>,
    id: String,
}

fn check(doc: &kentos_domain::Document, input: &ProfilesCreate) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let (_, alignment) = alignments::find(alignments::alignments(doc), &input.alignment, "alignment")?;
    let surface = match input.surface.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => Some(common::surface_id(doc, s, "surface")?),
        None => None,
    };
    let own = common::profiles(doc);
    let id = input.id.clone().unwrap_or_else(|| {
        (1..)
            .map(|n| format!("b-{n}"))
            .find(|id| own.iter().all(|p| &p.id != id))
            .unwrap_or_default()
    });
    let def = ProfileDef {
        id: id.clone(),
        name: input.name.clone(),
        alignment: alignment.id.clone(),
        alignment_revision: alignment.revision,
        revision: 1,
        pvis: input.pvis.clone(),
        surface,
    };
    let mut list = own.to_vec();
    list.push(def);
    let last = list.len() - 1;
    common::judged(doc, &list)?;
    common::build(doc, &list[last])?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, id })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: vec![],
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesCreate) -> CommandResult<ProfilesWritten> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = common::write(&mut ExecutionContext::new(&mut copy), c.list, STEP, Some(&c.id));
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Writes the profile: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesCreate) -> CommandResult<ProfilesWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_PROFILES_CREATE,
        kentos_commands::ROAD_PROFILES_CREATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: ProfilesCreate) -> CommandResult<ProfilesWritten> {
    match check(cx.doc, &input) {
        Ok(c) => common::write(cx, c.list, STEP, Some(&c.id)),
        Err(stop) => stop.into(),
    }
}
