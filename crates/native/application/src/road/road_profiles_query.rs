//! `road.profiles.query` v1 (plan/road.md Y4, docs/adr/0902 §4–§5): the road project's profiles as they build now, or
//! one's red and black elevations at stations and every round km, and the terrain profile along its alignment. Reads
//! only. The shared cases are `fixtures/commands/v1/road.profiles.query.json`.

use kentos_commands::{
    CommandResult, CommandWarning, MAX_ALIGNMENT_PLACES, ProfileAt, ProfileGroundRow, ProfilesQueried, ProfilesQuery,
};
use kentos_road::profile::terrain;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_profiles::{self as common, Built};
use crate::road_terrains::fail;

fn at(built: &Built, surface: Option<&kentos_road::surface::Surface>, s: f64, path: &str) -> Result<ProfileAt, Stop> {
    common::at(built, surface, s).ok_or_else(|| {
        fail(
            codes::STATION_OUTSIDE,
            format!("{s} m aksın üstünde değil (aks {:.3} m).", built.alignment.core.length),
            path,
        )
    })
}

fn stale_warning(report: &kentos_commands::ProfileReport) -> Option<CommandWarning> {
    report.stale.then(|| CommandWarning {
        code: codes::PROFILE_STALE.into(),
        message: format!(
            "“{}” boykesitinin aksı değişti (boykesit {}. revizyona göre kurulu, aks şimdi {}.); PVI'ları denetleyip aksa yeniden bağlayın.",
            report.name, report.alignment_revision, report.alignment_now
        ),
        path: Some("profile".into()),
    })
}

fn answer(doc: &kentos_domain::Document, input: &ProfilesQuery) -> Result<(ProfilesQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = common::profiles(doc);
    if !(input.stations.iter().all(|s| s.is_finite()) && input.every.is_none_or(|e| e.is_finite() && e > 0.0)) {
        return Err(fail(
            codes::INVALID_PLACES,
            "Km'ler sonlu sayılar, every 0'dan büyük olmalı.".into(),
            "stations",
        ));
    }
    if input.stations.len() > MAX_ALIGNMENT_PLACES {
        return Err(fail(
            codes::INVALID_PLACES,
            format!("Bir soruda en çok {MAX_ALIGNMENT_PLACES} yer olur."),
            "stations",
        ));
    }
    let asks = !input.stations.is_empty() || input.every.is_some() || input.terrain;
    let mut warnings = Vec::new();
    let Some(name) = input.profile.as_deref() else {
        if asks {
            return Err(fail(
                codes::INVALID_PLACES,
                "Kot bir boykesitte sorulur: boykesitin kimliğini ya da adını (profile) verin.".into(),
                "profile",
            ));
        }
        let mut profiles = Vec::new();
        for def in own {
            let built = common::build(doc, def)?;
            let report = common::report(doc, def, &built);
            warnings.extend(stale_warning(&report));
            profiles.push(report);
        }
        return Ok((
            ProfilesQueried {
                profiles,
                at: vec![],
                ticks: vec![],
                terrain: vec![],
            },
            warnings,
        ));
    };
    let (_, def) = common::find(own, name, "profile")?;
    let built = common::build(doc, def)?;
    let report = common::report(doc, def, &built);
    warnings.extend(stale_warning(&report));
    if input.terrain && def.surface.is_none() {
        return Err(fail(
            codes::NO_TERRAIN,
            format!("“{}” boykesitinin yüzeyi yok: arazi boykesiti için önce bir yüzey verin.", def.name),
            "terrain",
        ));
    }
    let made = common::surface(doc, def);
    let surface = match &made {
        Some(Ok(s)) => Some(s),
        Some(Err(w)) => {
            warnings.push(w.clone());
            None
        }
        None => None,
    };
    let at_list = input
        .stations
        .iter()
        .enumerate()
        .map(|(i, s)| at(&built, surface, *s, &format!("stations[{i}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut ticks = Vec::new();
    if let Some(step) = input.every {
        for s in alignments::tick_stations(&built.alignment, step)? {
            ticks.push(at(&built, surface, s, "every")?);
        }
    }
    let ground = match (input.terrain, surface) {
        (true, Some(s)) => {
            // The points it is read at: crossings, elements' ends, the ends; not the pieces made for drawing.
            let ends: Vec<f64> = std::iter::once(0.0)
                .chain(built.alignment.core.elements.iter().map(|e| e.station))
                .chain(std::iter::once(built.alignment.core.length))
                .collect();
            terrain(s, &built.alignment.core)
                .into_iter()
                .filter(|p| p.crossing || ends.contains(&p.station))
                .map(|p| ProfileGroundRow {
                    station: p.station,
                    z: p.z,
                    gap: p.gap,
                    crossing: p.crossing,
                })
                .collect()
        }
        _ => vec![],
    };
    Ok((
        ProfilesQueried {
            profiles: vec![report],
            at: at_list,
            ticks,
            terrain: ground,
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesQuery) -> CommandResult<ProfilesQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesQuery) -> CommandResult<ProfilesQueried> {
    plan(cx, &input)
}
