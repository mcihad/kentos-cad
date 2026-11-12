//! `cad.profiles.names` v1 (docs/adr/5180; plan/map-cad.md MAP-17): every field of a data profile with its name in the
//! institution by a mapping and the mapping's form of names (or another form, to preview), where the name comes from
//! and what is wrong with it; nothing is written. The shared cases are `fixtures/commands/v1/cad.profiles.names.json`.
//!
//! Checks, in order: the profile (`unknown_profile`, `invalid_profile`), then the mapping (`unknown_mapping`,
//! `invalid_profile_mapping` for its shape: the names it gives are the answer's rows, not a refusal).

use kentos_commands::{
    CommandResult, DataProfile, ProfileMapping, ProfileNames, ProfilesNames, mapping_shape_problem,
    name_rows, profile_names,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::profile_files::Shelf;
use crate::profiles::{mapping_found, profile_only};

fn resolved(doc: &Document, input: &ProfilesNames) -> Result<(DataProfile, Option<ProfileMapping>), Stop> {
    let shelf = Shelf::of(doc, input.drawing.as_deref());
    let profile = profile_only(input.name.as_deref(), input.profile.as_ref(), shelf)?;
    let mapping = mapping_found(input.mapping.as_ref(), input.mapping_name.as_deref(), shelf)?;
    if let Some(m) = &mapping
        && let Some(why) = mapping_shape_problem(m, &profile)
    {
        return Err(Stop::failed(error(
            codes::INVALID_PROFILE_MAPPING,
            format!("Eşleme geçersiz: {why}"),
            Some("mapping".into()),
        )));
    }
    Ok((profile, mapping))
}

/// The answer for a profile and a mapping already found.
pub fn names(p: &DataProfile, m: Option<&ProfileMapping>, form: Option<kentos_commands::MappingNames>) -> ProfileNames {
    let form = form.or_else(|| m.and_then(|m| m.names));
    let rows = name_rows(p, m, form.as_ref());
    profile_names(p, m.map(|m| m.name.clone()), form, rows)
}

fn run(doc: &Document, input: &ProfilesNames) -> CommandResult<ProfileNames> {
    match resolved(doc, input) {
        Ok((p, m)) => CommandResult::Completed {
            output: names(&p, m.as_ref(), input.names),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesNames) -> CommandResult<()> {
    match resolved(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The names; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesNames) -> CommandResult<ProfileNames> {
    run(cx.doc, input)
}

/// The names; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesNames) -> CommandResult<ProfileNames> {
    run(cx.doc, &input)
}

/// The names for the open drawing (the desktop's window).
pub fn run_in(doc: &Document, input: &ProfilesNames) -> CommandResult<ProfileNames> {
    run(doc, input)
}
