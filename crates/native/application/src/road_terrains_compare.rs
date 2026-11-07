//! `road.terrains.compare` v1 (plan/road.md Y18.3, docs/adr/0916 §3): the difference of two terrain surfaces over the
//! comparison's plan (Civil 3D's TIN volume surface), read now with `kentos_road::volume::difference`: cut, fill and
//! net volumes, their areas, the extremes and the ranges' areas. Nothing is written. The shared cases are
//! `fixtures/commands/v1/road.terrains.compare.json`.

use kentos_commands::{
    CommandResult, CommandWarning, TerrainDifferenceAt, TerrainDifferenceBand, TerrainsCompare, TerrainsCompared,
};
use kentos_domain::Document;
use kentos_road::surface::Surface;
use kentos_road::volume::{self, Triangle};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

fn invalid(message: &str, path: &str) -> Stop {
    fail(codes::INVALID_ANALYSIS, message.to_owned(), path)
}

fn triangles(s: &Surface) -> Vec<Triangle> {
    s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect()
}

/// The comparison now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &TerrainsCompare) -> Result<(TerrainsCompared, Vec<CommandWarning>), Stop> {
    terrains::road_project(doc)?;
    if let Some(b) = &input.ranges
        && (b.len() < 2 || b.iter().any(|v| !v.is_finite()) || b.windows(2).any(|w| w[0] >= w[1]))
    {
        return Err(invalid("Fark aralıklarının sınırları en az iki, sonlu ve artan sayılar olmalı.", "ranges"));
    }
    let base = crate::road_profiles::surface_id(doc, &input.base, "base")?;
    let comparison = crate::road_profiles::surface_id(doc, &input.comparison, "comparison")?;
    if base == comparison {
        return Err(invalid("Taban ile karşılaştırma aynı yüzey; iki ayrı yüzey verin.", "comparison"));
    }
    let def = |id: &str| terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled);
    let (b, c) = (def(&base)?, def(&comparison)?);
    // Docs/adr/1029: the two grounds' heights on one vertical system, or taken as one by the call.
    let mut warnings = Vec::new();
    crate::road_heights::proven(doc, &[(&b.name, Some(&b)), (&c.name, Some(&c))], input.assume_heights, "comparison", &mut warnings)?;
    let (made_b, made_c) = (terrains::make(doc, &b), terrains::make(doc, &c));
    let (sb, sc) = (made_b.built()?, made_c.built()?);
    let breaks = input.ranges.clone().unwrap_or_default();
    let d = volume::difference(&triangles(sc), &triangles(sb), &breaks);
    if d.volumes.uncovered > 0.0 {
        warnings.push(CommandWarning {
            code: codes::COMPARE_UNCOVERED.into(),
            message: format!(
                "Karşılaştırma yüzeyinin {:.2} m²'si tabanın dışında; o kısım hacme girmedi.",
                d.volumes.uncovered
            ),
            path: Some("comparison".into()),
        });
    }
    let at = |e: Option<(f64, [f64; 2])>| e.map(|(difference, at)| TerrainDifferenceAt { difference, at });
    let out = TerrainsCompared {
        base,
        comparison,
        cut: d.volumes.cut,
        fill: d.volumes.fill,
        net: d.volumes.fill - d.volumes.cut,
        cut_area: d.volumes.cut_area,
        fill_area: d.volumes.fill_area,
        covered: d.volumes.covered,
        uncovered: d.volumes.uncovered,
        least: at(d.least),
        largest: at(d.largest),
        bands: d.bands.iter().map(|b| TerrainDifferenceBand { from: b.0, to: b.1, area: b.2 }).collect(),
        outside_area: input.ranges.as_ref().map(|_| d.outside),
    };
    Ok((out, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsCompare) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsCompare) -> CommandResult<TerrainsCompared> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsCompare) -> CommandResult<TerrainsCompared> {
    plan(cx, &input)
}

/// The comparison for a window, or why it does not read.
pub fn preview(doc: &Document, input: &TerrainsCompare) -> Result<(TerrainsCompared, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
