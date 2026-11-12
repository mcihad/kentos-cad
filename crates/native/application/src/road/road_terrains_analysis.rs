//! `road.terrains.analysis` v1 (plan/road.md Y18.2, docs/adr/0916 §2): a terrain surface's elevation, slope and aspect
//! ranges, roughness and a raster's slope against its triangles', read now with `kentos_road::surface`. Nothing is
//! written. The shared cases are `fixtures/commands/v1/road.terrains.analysis.json`.

use kentos_commands::{
    CommandResult, CommandWarning, TerrainBand, TerrainBands, TerrainGridSlope, TerrainRoughness, TerrainsAnalysed,
    TerrainsAnalysis,
};
use kentos_domain::Document;
use kentos_road::surface::{AnalysisError, Measure, Surface};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

fn invalid(message: &str, path: &str) -> Stop {
    fail(codes::INVALID_ANALYSIS, message.to_owned(), path)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The query's own values judged, before anything is read.
fn judged(input: &TerrainsAnalysis) -> Result<(), Stop> {
    if input.elevations.is_none() && input.slopes.is_none() && input.aspects.is_none() && !input.roughness && input.grid.is_none() {
        return Err(invalid("Bir şey sorun: elevations, slopes, aspects, roughness ya da grid.", ""));
    }
    for (breaks, path) in [(&input.elevations, "elevations"), (&input.slopes, "slopes"), (&input.aspects, "aspects")] {
        if let Some(b) = breaks
            && (b.len() < 2 || b.iter().any(|v| !v.is_finite()) || b.windows(2).any(|w| w[0] >= w[1]))
        {
            return Err(invalid("Aralık sınırları en az iki, sonlu ve artan sayılar olmalı.", path));
        }
    }
    if input.grid.is_some_and(|c| !(c.is_finite() && c > 0.0)) {
        return Err(invalid("Izgaranın hücresi sıfırdan büyük olmalı.", "grid"));
    }
    Ok(())
}

fn banded(surface: &Surface, measure: Measure, breaks: &[f64], pieces: bool) -> Result<TerrainBands, Stop> {
    let (b, parts) = surface.bands(measure, breaks, pieces).map_err(|_| invalid("Aralık sınırları geçerli değil.", ""))?;
    let mut parts = parts.into_iter();
    Ok(TerrainBands {
        bands: b
            .bands
            .into_iter()
            .map(|x| TerrainBand {
                from: x.from,
                to: x.to,
                area: x.area,
                surface_area: x.surface_area,
                triangles: count(x.triangles),
                parts: parts.next().unwrap_or_default(),
            })
            .collect(),
        outside_area: b.outside_area,
        outside_triangles: count(b.outside_triangles),
        flat_area: b.flat_area,
        flat_triangles: count(b.flat_triangles),
    })
}

/// The analysis now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &TerrainsAnalysis) -> Result<(TerrainsAnalysed, Vec<CommandWarning>), Stop> {
    terrains::terrain_project(doc)?;
    judged(input)?;
    let id = crate::road_profiles::surface_id(doc, &input.surface, "surface")?;
    let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
    let made = terrains::make(doc, &def);
    let surface = made.built()?;
    let (area, surface_area) = surface.areas();
    let elevations = input.elevations.as_deref().map(|b| banded(surface, Measure::Elevation, b, input.pieces)).transpose()?;
    let slopes = input.slopes.as_deref().map(|b| banded(surface, Measure::Slope, b, input.pieces)).transpose()?;
    let aspects = input.aspects.as_deref().map(|b| banded(surface, Measure::Aspect, b, input.pieces)).transpose()?;
    let roughness = if input.roughness {
        surface.roughness().map(|r| TerrainRoughness { mean: r.mean, max: r.max, at: r.at })
    } else {
        None
    };
    let grid = match input.grid {
        Some(cell) => match surface.grid_slope(cell) {
            Ok(g) => Some(TerrainGridSlope {
                cell,
                cells: count(g.cells),
                mean: g.mean,
                mean_absolute: g.mean_absolute,
                max_absolute: g.max_absolute,
                at: g.at,
            }),
            Err(AnalysisError::TooMany) => {
                return Err(invalid(
                    &format!("Izgara bu hücreyle {} hücreden çok tutuyor; hücreyi büyütün.", kentos_road::surface::MAX_GRID_CELLS),
                    "grid",
                ));
            }
            Err(_) => return Err(invalid("Izgaranın hücresi sıfırdan büyük olmalı.", "grid")),
        },
        None => None,
    };
    Ok((TerrainsAnalysed { surface: id, area, surface_area, elevations, slopes, aspects, roughness, grid }, Vec::new()))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsAnalysis) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsAnalysis) -> CommandResult<TerrainsAnalysed> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsAnalysis) -> CommandResult<TerrainsAnalysed> {
    plan(cx, &input)
}

/// The analysis for a window, or why it does not read.
pub fn preview(doc: &Document, input: &TerrainsAnalysis) -> Result<(TerrainsAnalysed, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
