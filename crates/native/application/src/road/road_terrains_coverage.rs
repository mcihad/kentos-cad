//! `road.terrains.coverage` v1 (plan/road.md Y19.1, docs/adr/0920; ROAD-SURVEY-08, ROAD-UX-07): how much of a project
//! boundary a terrain surface covers, the no-data rest, and the part estimated across too wide gaps between shots
//! (`kentos_road::surface`'s coverage). Reads only. The shared cases are
//! `fixtures/commands/v1/road.terrains.coverage.json`.

use kentos_commands::{CommandResult, CommandWarning, Entity, TerrainsCoverage, TerrainsCovered};
use kentos_domain::Document;
use kentos_road::surface::Region;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_COVERAGE, message, path)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The drawing's object `uid` as a boundary: a closed polyline or an area, one part, straight edges.
pub(crate) fn boundary_of(doc: &Document, uid: &str) -> Result<Region, Stop> {
    let entity = doc
        .entities_with_uids()
        .find(|(u, _)| u.to_string() == uid)
        .map(|(_, e)| e.clone())
        .ok_or_else(|| fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), "boundary"))?;
    let not = || invalid(format!("“{uid}” sınır olamaz: düz kenarlı, tek parçalı kapalı bir çoklu çizgi ya da alan olmalı."), "boundary");
    let straight = |b: &Option<Vec<f64>>| b.as_ref().is_none_or(|b| b.iter().all(|v| *v == 0.0));
    let ring = |pts: &[kentos_commands::Vec2]| pts.iter().map(|p| [p.x, p.y]).collect::<Vec<[f64; 2]>>();
    let path = match &entity {
        Entity::Polygon(p) => p,
        Entity::Polyline(p) if p.closed => p,
        _ => return Err(not()),
    };
    if path.parts.as_ref().is_some_and(|q| !q.is_empty()) || path.pts.len() < 3 || !straight(&path.bulges) {
        return Err(not());
    }
    let mut holes = Vec::new();
    if let Entity::Polygon(p) = &entity {
        for h in p.holes.iter().flatten() {
            if h.pts.len() < 3 || !straight(&h.bulges) {
                return Err(not());
            }
            holes.push(ring(&h.pts));
        }
    }
    Ok(Region { outer: ring(&path.pts), holes })
}

/// The drawing's object as a boundary for a window's picture; none when it is not one.
pub fn boundary_region(doc: &Document, uid: &str) -> Option<Region> {
    boundary_of(doc, uid).ok()
}

/// The coverage now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &TerrainsCoverage) -> Result<(TerrainsCovered, Vec<CommandWarning>), Stop> {
    if input.longest.is_some_and(|v| !(v.is_finite() && v > 0.0)) {
        return Err(invalid("Tahmin sayılan kenar boyu (longest) sıfırdan büyük olmalı.".into(), "longest"));
    }
    terrains::terrain_project(doc)?;
    let id = crate::road_profiles::surface_id(doc, &input.surface, "surface")?;
    let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
    let made = terrains::make(doc, &def);
    let surface = made.built()?;
    let region = input.boundary.as_deref().map(|uid| boundary_of(doc, uid)).transpose()?;
    let c = surface.coverage(region.as_ref(), input.longest);
    let no_data = c.boundary_area.map(|b| (b - c.covered).max(0.0));
    let mut warnings = Vec::new();
    if let (Some(gap), Some(b)) = (no_data, c.boundary_area)
        && gap > 1e-6 * b.max(1.0)
    {
        warnings.push(CommandWarning {
            code: codes::TERRAIN_NO_DATA.into(),
            message: format!(
                "“{}” yüzeyi sınırın {:.1} m²'sini örtmüyor (%{:.1}); orada veri yok, arazisi bilinmiyor.",
                def.name,
                gap,
                100.0 * gap / b
            ),
            path: Some("boundary".into()),
        });
    }
    Ok((
        TerrainsCovered {
            surface: id,
            surface_area: c.surface_area,
            boundary_area: c.boundary_area,
            covered: c.covered,
            no_data,
            outside: c.outside,
            share: c.boundary_area.filter(|b| *b > 0.0).map(|b| c.covered / b),
            estimated: c.estimated,
            estimated_share: if c.covered > 0.0 { c.estimated / c.covered } else { 0.0 },
            estimated_triangles: count(c.estimated_triangles),
            longest_edge: c.longest_edge,
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsCoverage) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsCoverage) -> CommandResult<TerrainsCovered> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsCoverage) -> CommandResult<TerrainsCovered> {
    plan(cx, &input)
}

/// The coverage for a window, or why it does not read.
pub fn preview(doc: &Document, input: &TerrainsCoverage) -> Result<(TerrainsCovered, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}

/// The most triangles a window's picture draws.
pub const MOST_PICTURE_TRIANGLES: usize = 50_000;

/// A coverage's picture: the surface's triangles in plan (at most `MOST_PICTURE_TRIANGLES`), each with whether it is
/// estimated, and the boundary's rings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CoveragePicture {
    pub triangles: Vec<([[f64; 2]; 3], bool)>,
    pub rings: Vec<Vec<[f64; 2]>>,
}

/// The picture of `input`'s coverage, or why it does not read.
pub fn picture(doc: &Document, input: &TerrainsCoverage) -> Result<CoveragePicture, String> {
    let read = || -> Result<CoveragePicture, Stop> {
        let id = crate::road_profiles::surface_id(doc, &input.surface, "surface")?;
        let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
        let made = terrains::make(doc, &def);
        let surface = made.built()?;
        let region = input.boundary.as_deref().map(|uid| boundary_of(doc, uid)).transpose()?;
        let longest = input.longest.unwrap_or(f64::INFINITY);
        let triangles = surface
            .triangles
            .iter()
            .enumerate()
            .take(MOST_PICTURE_TRIANGLES)
            .map(|(i, t)| {
                let p = t.map(|k| {
                    let v = surface.vertices[k as usize];
                    [v[0], v[1]]
                });
                (p, surface.longest_edge(i) > longest)
            })
            .collect();
        let rings = region.map_or_else(Vec::new, |r| std::iter::once(r.outer).chain(r.holes).collect());
        Ok(CoveragePicture { triangles, rings })
    };
    read().map_err(crate::road_corridors::words)
}
