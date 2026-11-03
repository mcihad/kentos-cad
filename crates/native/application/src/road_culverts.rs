//! A road project's culverts (plan/road.md Y11.2, docs/adr/0909 §2): found by id or name, read now from their corridor's
//! cross at their station (`road_stakeout::table`'s points there): both sides in fill, the barrel between the toes
//! turned by the skew (`kentos-road`'s `culvert::barrel`), its ends placed on the alignment, the least cover under the
//! road's top between the hinges (`culvert::cover`) and the quantities. Nothing read is kept: every read computes again.

use kentos_commands::{
    CommandWarning, CulvertDef, CulvertEnd, CulvertFlow, CulvertListed, CulvertReport, CulvertShapeDef, StakeoutDistances,
    StakeoutQuery,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::culvert::{Shape, barrel, cover};

use crate::checks::Stop;
use crate::codes;
use crate::road_corridors::{self as corridors, words};
use crate::road_terrains::fail;

/// Only a road project has culverts.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(codes::NOT_A_ROAD_PROJECT, "Menfez yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""))
    }
}

/// The culvert of `name`: by its id, else by its name (case aside).
pub(crate) fn find<'a>(list: &'a [CulvertDef], name: &str, path: &str) -> Result<(usize, &'a CulvertDef), Stop> {
    list.iter()
        .position(|c| c.id == name)
        .or_else(|| list.iter().position(|c| c.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_CULVERT, format!("“{name}” kimlikli ya da adlı menfez projede yok."), path))
}

pub(crate) fn listed(list: &[CulvertDef]) -> Vec<CulvertListed> {
    list.iter()
        .map(|c| CulvertListed {
            id: c.id.clone(),
            name: c.name.clone(),
            corridor: c.corridor.clone(),
            station: c.station,
            revision: c.revision,
        })
        .collect()
}

pub(crate) fn shape_of(s: &CulvertShapeDef) -> Shape {
    match *s {
        CulvertShapeDef::Pipe { diameter, thickness, count, spacing } => Shape::Pipe { diameter, thickness, count, spacing },
        CulvertShapeDef::Box { width, height, thickness, slab, cells } => Shape::Box { width, height, thickness, slab, cells },
    }
}

fn num(v: f64) -> String {
    format!("{v}")
}

/// A culvert as read: its report and warnings, its barrel's direction (radians) and its axis point.
pub(crate) struct Read {
    pub(crate) report: CulvertReport,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) angle: f64,
    pub(crate) at: [f64; 2],
}

/// The culvert read now.
pub(crate) fn read(doc: &Document, def: &CulvertDef) -> Result<Read, Stop> {
    let (_, corridor) = corridors::find(doc.road_corridors(), &def.corridor, "corridor")?;
    let bad = |words: String| fail(codes::INVALID_CULVERT, words, "station");
    let first = corridor.regions.iter().map(|r| r.from).fold(f64::INFINITY, f64::min);
    let last = corridor.regions.iter().map(|r| r.to).fold(f64::NEG_INFINITY, f64::max);
    let s = def.station;
    if s < first - 1e-9 || s > last + 1e-9 {
        return Err(bad(format!(
            "“{}” menfezinin km'si ({} m) koridorun üstünde değil: koridor {} ile {} m arası.",
            def.name,
            num(s),
            num(first),
            num(last)
        )));
    }
    let query = StakeoutQuery {
        corridor: corridor.id.clone(),
        from: Some(s),
        to: Some(s),
        step: Some(1.0),
        codes: None,
        distances: StakeoutDistances::Grid,
    };
    let (table, mut warnings, alignment) = crate::road_stakeout::table(doc, &query).map_err(|stop| {
        fail(codes::INVALID_CULVERT, format!("“{}” menfezinin koridoru okunamıyor: {}", def.name, words(stop)), "corridor")
    })?;
    warnings.retain(|w| w.code != codes::STAKEOUT_UNSOLVED);
    let Some(region) = table.rows.first().map(|r| r.region) else {
        return Err(bad(format!("“{}” menfezinin km'sinde koridorun kesiti yok.", def.name)));
    };
    let rows: Vec<_> = table.rows.iter().filter(|r| r.region == region).collect();
    let point = |code: &str| rows.iter().find(|r| r.code == code);
    let mut toes = [0.0; 2];
    let mut hinges = [0.0; 2];
    for (k, (word, side)) in [("left", "sol"), ("right", "sağ")].into_iter().enumerate() {
        let fill = point(&format!("{word}.hinge")).zip(point(&format!("{word}.daylight"))).filter(|(h, d)| h.z - d.z > 1e-9);
        let Some((h, d)) = fill else {
            return Err(bad(format!(
                "“{}” menfezinin km'sinde {side} yan dolguda değil; menfez dolgunun altından geçer.",
                def.name
            )));
        };
        toes[k] = d.offset;
        hinges[k] = h.offset;
    }
    let shape = shape_of(&def.shape);
    let right = def.flow == CulvertFlow::Right;
    let b = barrel(toes, def.skew, def.extend, def.invert, def.slope, right);
    let top: Vec<[f64; 2]> = rows
        .iter()
        .filter(|r| r.offset >= hinges[0] - 1e-9 && r.offset <= hinges[1] + 1e-9)
        .map(|r| [r.offset, r.z])
        .collect();
    let (least, at) = cover(&top, def.skew, def.invert, def.slope, right, &shape).unwrap_or((0.0, 0.0));
    let place = alignment.core.at(s).ok_or_else(|| bad(format!("“{}” menfezinin km'si aksın üstünde değil.", def.name)))?;
    let angle = place.direction - std::f64::consts::FRAC_PI_2 + def.skew.to_radians();
    let c = def.skew.to_radians().cos();
    let ends = ["left", "right"]
        .into_iter()
        .enumerate()
        .map(|(k, side)| CulvertEnd {
            side: side.into(),
            distance: b.ends[k],
            offset: b.ends[k] * c,
            x: place.x + angle.cos() * b.ends[k],
            y: place.y + angle.sin() * b.ends[k],
            invert: b.inverts[k],
        })
        .collect();
    if let Some(asked) = def.cover
        && least < asked - 1e-9
    {
        warnings.push(CommandWarning {
            code: codes::CULVERT_COVER.into(),
            message: format!(
                "“{}” menfezinin üstündeki örtü en az {least:.3} m ({at:.3} m ötelemede); istenen {} m.",
                def.name,
                num(asked)
            ),
            path: Some("cover".into()),
        });
    }
    let now = corridor.revision;
    let stale = now != def.corridor_revision;
    if stale {
        warnings.push(CommandWarning {
            code: codes::CULVERT_STALE.into(),
            message: format!("“{}” menfezinin koridoru değişti; menfezi denetleyip koridora yeniden bağlayın.", def.name),
            path: Some("culvert".into()),
        });
    }
    let report = CulvertReport {
        id: def.id.clone(),
        name: def.name.clone(),
        corridor: def.corridor.clone(),
        revision: def.revision,
        stale,
        station: s,
        km_text: km_text(alignment.km(s), 3),
        length: b.length,
        width: shape.width(),
        ends,
        inlet: if right { "left" } else { "right" }.into(),
        cover: least,
        cover_offset: at,
        pipes: f64::from(shape.pipes()) * b.length,
        concrete: shape.concrete() * b.length,
    };
    Ok(Read { report, warnings, angle, at: [place.x, place.y] })
}

/// A culvert that does not read: its names and nothing else.
pub(crate) fn broken(def: &CulvertDef) -> CulvertReport {
    CulvertReport {
        id: def.id.clone(),
        name: def.name.clone(),
        corridor: def.corridor.clone(),
        revision: def.revision,
        stale: false,
        station: def.station,
        km_text: String::new(),
        length: 0.0,
        width: shape_of(&def.shape).width(),
        ends: Vec::new(),
        inlet: if def.flow == CulvertFlow::Right { "left" } else { "right" }.into(),
        cover: 0.0,
        cover_offset: 0.0,
        pipes: 0.0,
        concrete: 0.0,
    }
}
