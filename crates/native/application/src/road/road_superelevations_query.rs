//! `road.superelevations.query` v1 (plan/road.md Y6, docs/adr/0904): every superelevation schedule's report, or one's
//! cross slopes and widening at stations. Reads only. The shared cases are
//! `fixtures/commands/v1/road.superelevations.query.json`.

use kentos_commands::{CommandResult, CommandWarning, SuperAt, SuperelevationsQueried, SuperelevationsQuery};
use kentos_road::alignment::km_text;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_superelevations as common;
use crate::road_terrains::fail;

fn answer(
    doc: &kentos_domain::Document,
    input: &SuperelevationsQuery,
) -> Result<(SuperelevationsQueried, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let own = doc.road_superelevations();
    let Some(name) = input.superelevation.as_deref() else {
        if !input.stations.is_empty() {
            return Err(fail(
                codes::INVALID_PLACES,
                "Eğim bir dever çizelgesinde sorulur: çizelgenin kimliğini ya da adını (superelevation) verin.".into(),
                "superelevation",
            ));
        }
        let warnings = own.iter().filter_map(|d| common::stale_warning(doc, d)).collect();
        return Ok((
            SuperelevationsQueried { superelevations: own.iter().map(|d| common::report(doc, d)).collect(), at: vec![] },
            warnings,
        ));
    };
    let (_, def) = common::find(own, name, "superelevation")?;
    let built = common::alignment_of(doc, def)?;
    let (schedule, _) = common::schedule(def, &built)?;
    let length = built.core.length;
    let mut at = Vec::new();
    for (i, s) in input.stations.iter().enumerate() {
        if !s.is_finite() || *s < -1e-9 || *s > length + 1e-9 {
            return Err(fail(
                codes::STATION_OUTSIDE,
                format!("{s} m aksın üstünde değil (aks {length:.3} m)."),
                &format!("stations[{i}]"),
            ));
        }
        let c = schedule.at(*s, common::normal(def));
        let (wl, wr) = schedule.widening(*s);
        let km = built.km(*s);
        at.push(SuperAt { station: *s, km, km_text: km_text(km, 3), left: c.left, right: c.right, left_widening: wl, right_widening: wr });
    }
    let warnings = common::stale_warning(doc, def).into_iter().collect();
    Ok((SuperelevationsQueried { superelevations: vec![common::report(doc, def)], at }, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &SuperelevationsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &SuperelevationsQuery) -> CommandResult<SuperelevationsQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SuperelevationsQuery) -> CommandResult<SuperelevationsQueried> {
    plan(cx, &input)
}
