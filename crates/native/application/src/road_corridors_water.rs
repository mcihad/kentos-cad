//! `road.corridors.water` v1 (plan/road.md Y22, docs/adr/0923; ROAD-DEVER-09): water on a corridor's platform as built
//! now (`kentos_road::platform_water`): the flat lane and shoulder triangles, the zero crossfall stations and the low
//! points on the platform's edges. Reads only. The shared cases are `fixtures/commands/v1/road.corridors.water.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CorridorsWater, CorridorsWatered, CrossfallZero, PlatformFlat, PlatformLow,
};
use kentos_domain::Document;
use kentos_road::platform_water;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors as common;
use crate::road_terrains::fail;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The water now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &CorridorsWater) -> Result<(CorridorsWatered, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_corridors(), &input.corridor, "corridor")?;
    if !(input.least.is_finite() && input.least > 0.0) {
        return Err(fail(codes::INVALID_PLATFORM_WATER, "En küçük eğim (least) 0'dan büyük bir oran olmalı.".into(), "least"));
    }
    let made = common::make(doc, def, false)?;
    let w = platform_water::water(&made.built, input.least);
    let flat_area: f64 = w.flats.iter().map(|f| f.area).sum();
    let mut warnings = made.warnings;
    if !w.flats.is_empty() {
        warnings.push(CommandWarning {
            code: codes::CORRIDOR_FLAT.into(),
            message: format!(
                "“{}” koridorunun platformunda {} aralıkta {:.2} m² yer %{:.2}'den yatık; su orada durur.",
                def.name,
                w.flats.len(),
                flat_area,
                100.0 * input.least
            ),
            path: Some("least".into()),
        });
    }
    Ok((
        CorridorsWatered {
            corridor: def.id.clone(),
            least: input.least,
            flat_triangles: count(w.flat_triangles),
            flat_area,
            flats: w
                .flats
                .iter()
                .map(|f| PlatformFlat { region: count(f.region), from: f.from, to: f.to, area: f.area, least: f.least })
                .collect(),
            zeros: w.zeros.iter().map(|z| CrossfallZero { region: count(z.region), right: z.right, station: z.station }).collect(),
            lows: w
                .lows
                .iter()
                .map(|l| PlatformLow { code: l.code.clone(), region: count(l.region), station: l.at[0], x: l.at[1], y: l.at[2], z: l.at[3] })
                .collect(),
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &CorridorsWater) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &CorridorsWater) -> CommandResult<CorridorsWatered> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CorridorsWater) -> CommandResult<CorridorsWatered> {
    plan(cx, &input)
}

/// The water for a window, or why it does not read.
pub fn preview(doc: &Document, input: &CorridorsWater) -> Result<(CorridorsWatered, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}

/// A top surface triangle in plan, whether it is on the platform (lane or shoulder) and whether it is flatter than the
/// least.
pub type PlatformTriangle = ([[f64; 2]; 3], bool, bool);

/// A corridor's platform in plan for a window: its triangles, each with whether it is flatter than the least.
pub fn picture(doc: &Document, input: &CorridorsWater) -> Result<Vec<PlatformTriangle>, String> {
    let read = || -> Result<Vec<PlatformTriangle>, Stop> {
        common::road_project(doc)?;
        let (_, def) = common::find(doc.road_corridors(), &input.corridor, "corridor")?;
        let made = common::make(doc, def, false)?;
        Ok(made
            .built
            .triangles
            .iter()
            .zip(&made.built.roles)
            .map(|(t, role)| {
                let paved = matches!(role, kentos_road::corridor::Role::Lane | kentos_road::corridor::Role::Shoulder);
                (t.map(|p| [p[0], p[1]]), paved, paved && platform_water::slope(t) < input.least)
            })
            .collect())
    };
    read().map_err(crate::road_corridors::words)
}
