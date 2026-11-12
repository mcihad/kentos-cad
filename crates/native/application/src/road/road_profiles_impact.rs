//! `road.profiles.impact` v1 (plan/road.md Y56, docs/adr/0962; ROAD-PROFILE-09): what a profile's new PVIs would make
//! stale, before they are written (`kentos_road::profile::compare`): the stretches where the new grade line leaves the
//! saved one, the corridor regions and section stations over them, the volumes on those corridors and the haul plans
//! on those volumes; an approval among them is told. Reads only. The shared cases are
//! `fixtures/commands/v1/road.profiles.impact.json`.

use kentos_commands::{
    CommandResult, CommandWarning, ImpactCorridor, ImpactDerived, ImpactRegion, ImpactSections, ProfileChange, ProfileChangeKind,
    ProfileImpact, ProfilesImpact,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::profile::compare::{self, Kind};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_profiles as common;
use crate::road_terrains::fail;

/// The tolerance where none is asked (m).
pub const TOLERANCE: f64 = 1e-3;

fn answer(doc: &Document, input: &ProfilesImpact) -> Result<(ProfileImpact, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = common::profiles(doc);
    let (at, old) = common::find(own, &input.profile, "profile")?;
    let tolerance = input.tolerance.unwrap_or(TOLERANCE);
    if !(tolerance.is_finite() && tolerance > 0.0 && tolerance <= 1.0) {
        return Err(fail(codes::INVALID_PROFILE_IMPACT, "Etki toleransı 0'dan büyük, en çok 1 m olur.".into(), "tolerance"));
    }
    // The new PVIs judged and built as road.profiles.edit would.
    let mut list = own.to_vec();
    list[at].pvis = input.pvis.clone();
    common::judged(doc, &list)?;
    let new = common::build(doc, &list[at])?;
    let saved = common::build(doc, old)?;
    let text = |s: f64| km_text(saved.alignment.km(s), 3);
    let changes: Vec<ProfileChange> = compare::changes(&saved.vertical, &new.vertical, tolerance)
        .into_iter()
        .map(|c| ProfileChange {
            kind: match c.kind {
                Kind::Changed => ProfileChangeKind::Changed,
                Kind::Added => ProfileChangeKind::Added,
                Kind::Removed => ProfileChangeKind::Removed,
            },
            from: c.from,
            to: c.to,
            from_text: text(c.from),
            to_text: text(c.to),
            most: c.most,
            at: c.at,
        })
        .collect();
    let inside = |s: f64| changes.iter().any(|c| s >= c.from - 1e-9 && s <= c.to + 1e-9);
    // The corridors on this profile: each region's stretches that change.
    let mut corridors = Vec::new();
    for c in doc.road_corridors().iter().filter(|c| c.profile == old.id) {
        let mut regions = Vec::new();
        for (ri, r) in c.regions.iter().enumerate() {
            for ch in &changes {
                let (a, b) = (r.from.max(ch.from), r.to.min(ch.to));
                if b - a > 1e-9 {
                    regions.push(ImpactRegion { region: ri as u32, from: a, to: b, from_text: text(a), to_text: text(b) });
                }
            }
        }
        if !regions.is_empty() {
            corridors.push(ImpactCorridor { corridor: c.id.clone(), name: c.name.clone(), regions });
        }
    }
    // The section sets on this profile: their stations in a change.
    let mut sections = Vec::new();
    for d in doc.road_sections().iter().filter(|d| d.profile.as_deref() == Some(old.id.as_str())) {
        let Ok(built) = crate::road_sections::alignment_of(doc, d) else { continue };
        let Ok(stations) = crate::road_sections::stations_of(doc, d, &built) else { continue };
        let n = stations.iter().filter(|s| inside(**s)).count();
        if n > 0 {
            sections.push(ImpactSections { set: d.id.clone(), name: d.name.clone(), stations: n as u32, of: stations.len() as u32 });
        }
    }
    // The volumes on those corridors, the haul plans on those volumes.
    let volumes: Vec<ImpactDerived> = doc
        .road_volumes()
        .iter()
        .filter(|v| corridors.iter().any(|c| c.corridor == v.corridor))
        .map(|v| ImpactDerived { id: v.id.clone(), name: v.name.clone(), approved: v.approved.is_some() })
        .collect();
    let hauls: Vec<ImpactDerived> = doc
        .road_hauls()
        .iter()
        .filter(|h| volumes.iter().any(|v| v.id == h.volume))
        .map(|h| ImpactDerived { id: h.id.clone(), name: h.name.clone(), approved: h.approved.is_some() })
        .collect();
    let mut warnings = Vec::new();
    let approved: Vec<String> = volumes
        .iter()
        .filter(|v| v.approved)
        .map(|v| format!("“{}” kübajı", v.name))
        .chain(hauls.iter().filter(|h| h.approved).map(|h| format!("“{}” taşıma planı", h.name)))
        .collect();
    if !approved.is_empty() {
        warnings.push(CommandWarning {
            code: codes::PROFILE_IMPACT_APPROVED.into(),
            message: format!(
                "“{}” boykesitinin yeni PVI'ları onaylı hesapları değiştirir: {}. Yazılınca onayları eskir; yeni bir hesap onaylanmalı.",
                old.name,
                approved.join(", ")
            ),
            path: Some("pvis".into()),
        });
    }
    let out = ProfileImpact { profile: old.id.clone(), clear: changes.is_empty(), changes, corridors, sections, volumes, hauls };
    Ok((out, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesImpact) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesImpact) -> CommandResult<ProfileImpact> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesImpact) -> CommandResult<ProfileImpact> {
    plan(cx, &input)
}

/// The impact of the PVIs a window is typing, or why it does not read.
pub fn preview(doc: &Document, input: &ProfilesImpact) -> Result<(ProfileImpact, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
