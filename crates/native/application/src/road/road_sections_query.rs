//! `road.sections.query` v1 (plan/road.md Y5, docs/adr/0903 §4–§5): every sample line set's report, or one's sections:
//! the terrain along each sample line, the template's platform on the profile's elevation, its slopes tied to the
//! terrain and the cut and fill areas. Reads only. The shared cases are `fixtures/commands/v1/road.sections.query.json`.

use kentos_commands::{CommandResult, CommandWarning, SectionsQueried, SectionsQuery};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_sections as common;
use crate::road_terrains::fail;

fn answer(doc: &kentos_domain::Document, input: &SectionsQuery) -> Result<(SectionsQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = doc.road_sections();
    let Some(name) = input.set.as_deref() else {
        if !input.stations.is_empty() {
            return Err(fail(
                codes::INVALID_PLACES,
                "Km bir kesit takımında sorulur: takımın kimliğini ya da adını (set) verin.".into(),
                "set",
            ));
        }
        let warnings = own.iter().filter_map(|d| common::stale_warning(doc, d)).collect();
        return Ok((SectionsQueried { sets: common::reports(doc, own), sections: vec![], sources: None }, warnings));
    };
    let (_, def) = common::find_set(own, name, "set")?;
    let built = common::alignment_of(doc, def)?;
    let stations = common::stations_of(doc, def, &built)?;
    let asked = if input.stations.is_empty() {
        stations.clone()
    } else {
        let mut out = Vec::new();
        for (i, s) in input.stations.iter().enumerate() {
            match stations.iter().find(|k| (*k - s).abs() < common::SAME_STATION) {
                Some(k) => out.push(*k),
                None => {
                    return Err(fail(
                        codes::INVALID_PLACES,
                        format!("{s} m “{}” kesit takımının km'lerinden biri değil.", def.name),
                        &format!("stations[{i}]"),
                    ));
                }
            }
        }
        out
    };
    let mut warnings: Vec<CommandWarning> = common::stale_warning(doc, def).into_iter().collect();
    let (rows, more) = common::sections(doc, def, &built, &asked);
    warnings.extend(more);
    Ok((
        SectionsQueried { sets: vec![common::report(doc, def, stations)], sections: rows, sources: Some(sources_of(doc, def)) },
        warnings,
    ))
}

/// What a set's sections are computed from, each as it is now (docs/adr/1039): an id the project does not have reads
/// revision 0.
pub fn sources_of(doc: &kentos_domain::Document, def: &kentos_commands::SectionSetDef) -> kentos_commands::SectionSources {
    use kentos_commands::RoadSource;
    let alignment = doc.road_alignments().iter().find(|a| a.id == def.alignment).map_or(0, |a| a.revision);
    kentos_commands::SectionSources {
        set: RoadSource { id: def.id.clone(), revision: def.revision },
        alignment: RoadSource { id: def.alignment.clone(), revision: alignment },
        profile: def.profile.as_ref().map(|id| RoadSource {
            id: id.clone(),
            revision: doc.road_profiles().iter().find(|p| p.id == *id).map_or(0, |p| p.revision),
        }),
        template: def.template.as_ref().map(|id| RoadSource {
            id: id.clone(),
            revision: doc.road_templates().iter().find(|t| t.id == *id).map_or(0, |t| t.revision),
        }),
        superelevation: def.superelevation.as_ref().map(|id| RoadSource {
            id: id.clone(),
            revision: doc.road_superelevations().iter().find(|s| s.id == *id).map_or(0, |s| s.revision),
        }),
        surface: def.surface.as_ref().map(|id| crate::road_sources::surface(doc, id)),
    }
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &SectionsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &SectionsQuery) -> CommandResult<SectionsQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SectionsQuery) -> CommandResult<SectionsQueried> {
    plan(cx, &input)
}
