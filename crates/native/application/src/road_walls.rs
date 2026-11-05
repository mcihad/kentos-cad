//! A road project's retaining walls (plan/road.md Y11.3, docs/adr/0909 §3): found by id or name, read now from their
//! corridor's crosses at their stations: at each the side's hinge (the road's edge) and the ground under it on the
//! sample line, the wall's top, footing and stem there (`kentos-road`'s `wall`), and along the face its length, face
//! area and concrete. Nothing read is kept: every read computes again.

use kentos_commands::{
    CommandWarning, RetainingWallDef, RetainingWallListed, RetainingWallReport, RetainingWallRow, WallSide,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::corridor::place;
use kentos_road::section::SampleLine;
use kentos_road::wall::{Shape, ground_at, slice, totals};

use crate::checks::Stop;
use crate::codes;
use crate::road_corridors::{self as corridors, words};
use crate::road_terrains::fail;

/// Only a road project has retaining walls.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(codes::NOT_A_ROAD_PROJECT, "İstinat duvarı yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""))
    }
}

/// The wall of `name`: by its id, else by its name (case aside).
pub(crate) fn find<'a>(list: &'a [RetainingWallDef], name: &str, path: &str) -> Result<(usize, &'a RetainingWallDef), Stop> {
    list.iter()
        .position(|c| c.id == name)
        .or_else(|| list.iter().position(|c| c.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_RETAINING_WALL, format!("“{name}” kimlikli ya da adlı istinat duvarı projede yok."), path))
}

pub(crate) fn listed(list: &[RetainingWallDef]) -> Vec<RetainingWallListed> {
    list.iter()
        .map(|w| RetainingWallListed {
            id: w.id.clone(),
            name: w.name.clone(),
            corridor: w.corridor.clone(),
            side: w.side,
            from: w.from,
            to: w.to,
            revision: w.revision,
        })
        .collect()
}

pub(crate) fn shape_of(def: &RetainingWallDef) -> Shape {
    let s = &def.shape;
    Shape { top: s.top, batter: s.batter, footing: s.footing, base: s.base, toe: s.toe, cover: s.cover, above: s.above }
}

fn num(v: f64) -> String {
    format!("{v}")
}

/// A wall as read: its report and warnings, and at each row its sample line (for drawing).
pub(crate) struct Read {
    pub(crate) report: RetainingWallReport,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) lines: Vec<SampleLine>,
}

/// The wall's footing edges' offsets at a row: the toe's (in front of the face) and the heel's. The face looks away
/// from what it holds: outward from the road over a fill, toward it in a cut.
pub(crate) fn footing_edges(def: &RetainingWallDef, offset: f64, fill: bool) -> [f64; 2] {
    let side = if def.side == WallSide::Right { 1.0 } else { -1.0 };
    let facing = if fill { side } else { -side };
    let toe = offset + facing * def.shape.toe;
    [toe, toe - facing * def.shape.footing]
}

/// The wall read now.
pub(crate) fn read(doc: &Document, def: &RetainingWallDef) -> Result<Read, Stop> {
    let (_, corridor) = corridors::find(doc.road_corridors(), &def.corridor, "corridor")?;
    let bad = |words: String, path: &str| fail(codes::INVALID_RETAINING_WALL, words, path);
    let first = corridor.regions.iter().map(|r| r.from).fold(f64::INFINITY, f64::min);
    let last = corridor.regions.iter().map(|r| r.to).fold(f64::NEG_INFINITY, f64::max);
    for (v, path) in [(def.from, "from"), (def.to, "to")] {
        if v < first - 1e-9 || v > last + 1e-9 {
            return Err(bad(
                format!(
                    "“{}” istinat duvarının km'si ({} m) koridorun üstünde değil: koridor {} ile {} m arası.",
                    def.name,
                    num(v),
                    num(first),
                    num(last)
                ),
                path,
            ));
        }
    }
    let made = corridors::make(doc, corridor, false).map_err(|stop| {
        let why = format!("“{}” koridoru kurulamıyor: {}", corridor.name, words(stop));
        bad(format!("“{}” istinat duvarının koridoru okunamıyor: {why}", def.name), "corridor")
    })?;
    let mut warnings = made.warnings.clone();
    let shape = shape_of(def);
    let text = |s: f64| km_text(made.sampler.alignment.km(s), 3);
    let (mut rows, mut lines, mut slices, mut places) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for s in crate::road_stakeout::stepped(def.from, def.to, def.step) {
        let none = || bad(format!("“{}” istinat duvarının km {}'sinde koridorun kesiti yok.", def.name, text(s)), "from");
        let ri = corridor.regions.iter().position(|r| s >= r.from - 1e-9 && s <= r.to + 1e-9).ok_or_else(none)?;
        let (cross, terrain) = made.sampler.sample(&made.template_at(ri, s), s).ok_or_else(none)?;
        let section = cross.section.as_ref().ok_or_else(none)?;
        let hinge = if def.side == WallSide::Right { section.right.hinge } else { section.left.hinge };
        let ground = ground_at(&terrain, hinge[0]).ok_or_else(|| {
            bad(format!("“{}” istinat duvarının km {}'sinde yüzün altında zemin yok.", def.name, text(s)), "side")
        })?;
        let sl = slice(hinge[1], ground, &shape);
        let p = place(&cross.line, hinge[0]);
        rows.push(RetainingWallRow {
            station: s,
            km_text: text(s),
            offset: hinge[0],
            x: p[0],
            y: p[1],
            road: hinge[1],
            ground,
            top: sl.top,
            footing: sl.footing,
            height: sl.height,
            area: sl.area,
            fill: sl.fill,
        });
        places.push(p);
        slices.push(sl);
        lines.push(cross.line);
    }
    let (length, face, concrete) = totals(&places, &slices);
    let (mut highest, mut highest_at) = (f64::NEG_INFINITY, def.from);
    for r in &rows {
        if r.height > highest + 1e-12 {
            highest = r.height;
            highest_at = r.station;
        }
    }
    if let Some(asked) = def.highest
        && highest > asked + 1e-9
    {
        warnings.push(CommandWarning {
            code: codes::RETAINING_WALL_HIGH.into(),
            message: format!(
                "“{}” istinat duvarının gövdesi km {}'de {highest:.3} m; istenen en çok {} m.",
                def.name,
                text(highest_at),
                num(asked)
            ),
            path: Some("highest".into()),
        });
    }
    let stale = corridor.revision != def.corridor_revision;
    if stale {
        warnings.push(CommandWarning {
            code: codes::RETAINING_WALL_STALE.into(),
            message: format!("“{}” istinat duvarının koridoru değişti; duvarı denetleyip koridora yeniden bağlayın.", def.name),
            path: Some("wall".into()),
        });
    }
    let report = RetainingWallReport {
        id: def.id.clone(),
        name: def.name.clone(),
        corridor: def.corridor.clone(),
        revision: def.revision,
        stale,
        side: def.side,
        from: def.from,
        to: def.to,
        rows,
        length,
        face,
        concrete,
        highest,
        highest_at,
    };
    Ok(Read { report, warnings, lines })
}

/// A wall that does not read: its names and nothing else.
pub(crate) fn broken(def: &RetainingWallDef) -> RetainingWallReport {
    RetainingWallReport {
        id: def.id.clone(),
        name: def.name.clone(),
        corridor: def.corridor.clone(),
        revision: def.revision,
        stale: false,
        side: def.side,
        from: def.from,
        to: def.to,
        rows: Vec::new(),
        length: 0.0,
        face: 0.0,
        concrete: 0.0,
        highest: 0.0,
        highest_at: def.from,
    }
}
