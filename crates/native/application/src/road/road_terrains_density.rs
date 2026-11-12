//! `road.terrains.density` v1 (plan/road.md Y19.3, docs/adr/0920 §6; ROAD-SURVEY-08): a terrain surface's corners
//! counted in square cells of the user's size over a project boundary or the surface itself (`kentos_road::surface`'s
//! density), USGS's density raster. Reads only. The shared cases are `fixtures/commands/v1/road.terrains.density.json`.

use kentos_commands::{CommandResult, CommandWarning, MOST_DENSITY_CELLS, TerrainsDensity, TerrainsDensityRead, Vec2};
use kentos_domain::Document;
use kentos_road::surface::{Density, Region, Surface};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_POINT_DENSITY, message, path)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The input's numbers checked.
pub(crate) fn numbers(cell: f64, least: Option<u32>, share: Option<f64>) -> Result<(), Stop> {
    if !(cell.is_finite() && cell > 0.0) {
        return Err(invalid("Hücre boyu (cell) sıfırdan büyük olmalı.".into(), "cell"));
    }
    if least == Some(0) {
        return Err(invalid("En az nokta (least) 1 ya da daha çok olmalı.".into(), "least"));
    }
    if share.is_some_and(|s| !(s.is_finite() && s > 0.0 && s <= 1.0)) {
        return Err(invalid("İstenen pay (share) 0'dan büyük, 1'den büyük olmayan bir oran olmalı.".into(), "share"));
    }
    Ok(())
}

/// The surface, its name and the boundary read, the cell count checked: the density grid's reading.
pub(crate) fn grid(doc: &Document, surface: &str, boundary: Option<&str>, cell: f64, least: Option<u32>) -> Result<(String, String, Density), Stop> {
    terrains::terrain_project(doc)?;
    let id = crate::road_profiles::surface_id(doc, surface, "surface")?;
    let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
    let made = terrains::make(doc, &def);
    let built: &Surface = made.built()?;
    let region: Option<Region> = boundary.map(|uid| crate::road_terrains_coverage::boundary_of(doc, uid)).transpose()?;
    let cells = built.density_cells(region.as_ref(), cell);
    if cells > MOST_DENSITY_CELLS as usize {
        return Err(invalid(
            format!("{cell} m hücreyle ızgara {cells} hücre olur; en çok {MOST_DENSITY_CELLS} hücre okunur. Hücreyi büyütün."),
            "cell",
        ));
    }
    Ok((id, def.name.clone(), built.density(region.as_ref(), cell, least)))
}

/// The density grid now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &TerrainsDensity) -> Result<(TerrainsDensityRead, Vec<CommandWarning>), Stop> {
    numbers(input.cell, input.least, input.share)?;
    let (id, name, d) = grid(doc, &input.surface, input.boundary.as_deref(), input.cell, input.least)?;
    let share = (d.cells > 0).then(|| d.filled as f64 / d.cells as f64);
    let area = d.cells as f64 * input.cell * input.cell;
    let meets = input.share.map(|want| share.is_some_and(|s| s >= want));
    let mut warnings = Vec::new();
    if let (Some(false), Some(want)) = (meets, input.share) {
        warnings.push(CommandWarning {
            code: codes::TERRAIN_DENSITY_SHORT.into(),
            message: format!(
                "“{name}” yüzeyinin {} hücresinin {}'inde nokta var (%{:.1}); istenen %{:.1}.",
                d.cells,
                d.filled,
                100.0 * share.unwrap_or(0.0),
                100.0 * want
            ),
            path: Some("share".into()),
        });
    }
    Ok((
        TerrainsDensityRead {
            surface: id,
            cell: input.cell,
            origin: Vec2 { x: d.origin[0], y: d.origin[1] },
            columns: count(d.columns),
            rows: count(d.rows),
            cells: count(d.cells),
            filled: count(d.filled),
            empty: count(d.cells - d.filled),
            share,
            sparse: d.sparse.map(count),
            points: count(d.points),
            area,
            mean: (area > 0.0).then(|| d.points as f64 / area),
            most: d.most,
            meets,
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsDensity) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsDensity) -> CommandResult<TerrainsDensityRead> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsDensity) -> CommandResult<TerrainsDensityRead> {
    plan(cx, &input)
}

/// The density for a window, or why it does not read.
pub fn preview(doc: &Document, input: &TerrainsDensity) -> Result<(TerrainsDensityRead, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}

/// A density grid's picture: each cell's count (none for a cell that does not count), row by row from the lower
/// left, and the grid's place.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DensityPicture {
    pub origin: [f64; 2],
    pub cell: f64,
    pub columns: usize,
    pub rows: usize,
    pub counts: Vec<Option<u32>>,
    pub rings: Vec<Vec<[f64; 2]>>,
}

/// The picture of `input`'s density grid, or why it does not read.
pub fn picture(doc: &Document, input: &TerrainsDensity) -> Result<DensityPicture, String> {
    let read = || -> Result<DensityPicture, Stop> {
        numbers(input.cell, input.least, input.share)?;
        let (_, _, d) = grid(doc, &input.surface, input.boundary.as_deref(), input.cell, input.least)?;
        let region = input.boundary.as_deref().map(|uid| crate::road_terrains_coverage::boundary_of(doc, uid)).transpose()?;
        let rings = region.map_or_else(Vec::new, |r| std::iter::once(r.outer).chain(r.holes).collect());
        Ok(DensityPicture { origin: d.origin, cell: d.cell, columns: d.columns, rows: d.rows, counts: d.counts, rings })
    };
    read().map_err(crate::road_corridors::words)
}
