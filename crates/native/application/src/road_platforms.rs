//! A road project's graded pads (plan/road.md Y14, docs/adr/0927): found by id or name, graded now on their terrain
//! surface (`kentos_road::grading`): the slope lines and their side, the daylight ring, the open lines, the areas, and
//! cut and fill against the terrain. Nothing graded is kept: every read computes again.

use kentos_commands::{CommandWarning, PlatformDef, PlatformListed, PlatformReport, PlatformSide, PlatformSlopeLine};
use kentos_domain::Document;
use kentos_road::grading::{self, Graded, Platform, Side};
use kentos_road::surface::Surface;

use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::fail;

/// Only a road project has pads.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(codes::NOT_A_ROAD_PROJECT, "Platform yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""))
    }
}

/// The pad of `name`: by its id, else by its name (case aside).
pub(crate) fn find<'a>(list: &'a [PlatformDef], name: &str, path: &str) -> Result<(usize, &'a PlatformDef), Stop> {
    list.iter()
        .position(|c| c.id == name)
        .or_else(|| list.iter().position(|c| c.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_PLATFORM, format!("“{name}” kimlikli ya da adlı platform projede yok."), path))
}

pub(crate) fn listed(list: &[PlatformDef]) -> Vec<PlatformListed> {
    list.iter()
        .map(|p| PlatformListed { id: p.id.clone(), name: p.name.clone(), surface: p.surface.clone(), revision: p.revision })
        .collect()
}

/// The core's pad of a definition.
pub(crate) fn platform_of(def: &PlatformDef) -> Platform {
    Platform {
        corners: def.corners.clone(),
        base: def.base,
        grade: def.grade,
        direction: def.direction,
        cut: def.cut,
        fill: def.fill,
        step: def.step,
        fan: def.fan,
        reach: def.reach,
    }
}

/// The terrain a pad is graded to, built now.
pub(crate) fn terrain_of(doc: &Document, def: &PlatformDef) -> Result<Surface, Stop> {
    let Some(s) = doc.settings().road.as_ref().and_then(|r| r.surfaces.iter().find(|s| s.id == def.surface)) else {
        return Err(fail(codes::INVALID_PLATFORM, format!("“{}” platformunun arazi yüzeyi projede yok.", def.name), "surface"));
    };
    crate::road_terrains::make(doc, s).surface.map_err(|p| {
        fail(codes::INVALID_PLATFORM, format!("“{}” platformunun yüzeyi “{}” kurulamıyor: {}", def.name, s.name, p.message), "surface")
    })
}

/// A pad read: its report and the warnings it gives.
pub(crate) struct Read {
    pub report: PlatformReport,
    pub warnings: Vec<CommandWarning>,
}

pub(crate) fn report_of(def: &PlatformDef, g: &Graded) -> PlatformReport {
    let lines = g
        .lines
        .iter()
        .map(|l| PlatformSlopeLine {
            from: l.from,
            to: l.daylight,
            side: match l.side {
                Side::Cut => PlatformSide::Cut,
                Side::Fill => PlatformSide::Fill,
                Side::Open => PlatformSide::Open,
            },
            corner: u32::try_from(l.corner).unwrap_or(u32::MAX),
            at_corner: l.at_corner,
        })
        .collect();
    let daylight = if g.open == 0 { g.lines.iter().filter_map(|l| l.daylight).collect() } else { Vec::new() };
    PlatformReport {
        id: def.id.clone(),
        name: def.name.clone(),
        surface: def.surface.clone(),
        revision: def.revision,
        top: g.top.clone(),
        lines,
        daylight,
        open: u32::try_from(g.open).unwrap_or(u32::MAX),
        top_area: g.top_area,
        footprint: g.footprint,
        cut: g.volumes.map(|v| v.cut),
        fill: g.volumes.map(|v| v.fill),
        cut_area: g.volumes.map(|v| v.cut_area),
        fill_area: g.volumes.map(|v| v.fill_area),
        why: g.why.clone(),
    }
}

/// The open lines' warning.
pub(crate) fn open_warning(def: &PlatformDef, g: &Graded) -> Option<CommandWarning> {
    (g.open > 0).then(|| CommandWarning {
        code: codes::PLATFORM_OPEN.into(),
        message: format!(
            "“{}” platformunun {} şev çizgisi araziyle buluşmuyor (yüzeyin dışında ya da {} m eriminden uzakta); hacim ve günışığı çizgisi yok.",
            def.name, g.open, def.reach
        ),
        path: Some("platform".into()),
    })
}

/// The pad graded now on its terrain.
pub(crate) fn read(doc: &Document, def: &PlatformDef) -> Result<Read, Stop> {
    let terrain = terrain_of(doc, def)?;
    let g = grading::grade(&platform_of(def), &terrain, 0.0)
        .map_err(|e| fail(codes::INVALID_PLATFORM, format!("“{}” platformu tesviye edilemiyor: {e}", def.name), "corners"))?;
    Ok(Read { warnings: open_warning(def, &g).into_iter().collect(), report: report_of(def, &g) })
}

/// A pad that does not read, in a list: its names and nothing graded.
pub(crate) fn broken(def: &PlatformDef, why: String) -> PlatformReport {
    PlatformReport {
        id: def.id.clone(),
        name: def.name.clone(),
        surface: def.surface.clone(),
        revision: def.revision,
        top: Vec::new(),
        lines: Vec::new(),
        daylight: Vec::new(),
        open: 0,
        top_area: 0.0,
        footprint: None,
        cut: None,
        fill: None,
        cut_area: None,
        fill_area: None,
        why: Some(why),
    }
}
