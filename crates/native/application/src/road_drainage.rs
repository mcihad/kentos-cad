//! `road.drainage.query` v1 (plan/road.md Y16, docs/adr/0929 §2): a terrain surface's ponds, water drops and a discharge
//! point's catchment, read now with `kentos_road::drainage`; the catchment's Rational peak flow and a culvert's
//! full-flow capacity by Manning against it. Nothing is written. The shared cases are
//! `fixtures/commands/v1/road.drainage.query.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CulvertShapeDef, DrainageCapacity, DrainageCatchment, DrainageDrop, DrainageDropEnd,
    DrainagePond, DrainageQueried, DrainageQuery, MOST_CATCHMENT_CELLS, MOST_DRAINAGE_DROPS,
};
use kentos_domain::Document;
use kentos_road::drainage::{self, Channel, DropEnd, Tin};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

fn invalid(message: &str, path: &str) -> Stop {
    fail(codes::INVALID_DRAINAGE, message.to_owned(), path)
}

/// The query's own values judged, before anything is read.
fn judged(input: &DrainageQuery) -> Result<(), Stop> {
    if input.ponds.is_none() && input.drops.is_empty() && input.catchment.is_none() {
        return Err(invalid("Bir şey sorun: ponds, drops ya da catchment.", ""));
    }
    if input.drops.len() > MOST_DRAINAGE_DROPS {
        return Err(invalid(&format!("Bir soruda en çok {MOST_DRAINAGE_DROPS} su damlası okunur."), "drops"));
    }
    if let Some(k) = input.drops.iter().position(|p| !(p[0].is_finite() && p[1].is_finite())) {
        return Err(invalid("Su damlasının yeri sonlu iki sayı olmalı.", &format!("drops[{k}]")));
    }
    if let Some(p) = &input.ponds
        && (p.min_depth.is_nan() || p.min_depth < 0.0 || p.min_depth.is_infinite())
    {
        return Err(invalid("Göletin en az derinliği eksi olmayan bir sayı olmalı.", "ponds.minDepth"));
    }
    if let Some(c) = &input.catchment {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !positive(c.cell) {
            return Err(invalid("Havzanın ızgara aralığı sıfırdan büyük olmalı.", "catchment.cell"));
        }
        if !positive(c.radius) {
            return Err(invalid("Boşaltım noktasının erimi sıfırdan büyük olmalı.", "catchment.radius"));
        }
        if c.at.is_none() && c.culvert.is_none() {
            return Err(invalid("Boşaltım noktasını verin: at ya da bir menfez (culvert).", "catchment.at"));
        }
        if c.at.is_some_and(|p| !(p[0].is_finite() && p[1].is_finite())) {
            return Err(invalid("Boşaltım noktası sonlu iki sayı olmalı.", "catchment.at"));
        }
        if c.runoff.is_some() != c.intensity.is_some() {
            return Err(invalid("Akış katsayısı (runoff) ile yağış şiddeti (intensity) birlikte verilir.", "catchment.runoff"));
        }
        if c.runoff.is_some_and(|r| !(r.is_finite() && (0.0..=1.0).contains(&r))) {
            return Err(invalid("Akış katsayısı 0 ile 1 arası olmalı.", "catchment.runoff"));
        }
        if c.intensity.is_some_and(|i| !positive(i)) {
            return Err(invalid("Yağış şiddeti sıfırdan büyük olmalı.", "catchment.intensity"));
        }
    }
    Ok(())
}

fn end_of(e: DropEnd) -> DrainageDropEnd {
    match e {
        DropEnd::LowPoint => DrainageDropEnd::LowPoint,
        DropEnd::Flat => DrainageDropEnd::Flat,
        DropEnd::Edge => DrainageDropEnd::Edge,
        DropEnd::Outside => DrainageDropEnd::Outside,
        DropEnd::Steps => DrainageDropEnd::Steps,
    }
}

/// A culvert's barrels full: each pipe or cell by Manning at the culvert's slope; none without a roughness.
fn capacity(def: &kentos_commands::CulvertDef) -> Option<(f64, f64)> {
    let n = def.roughness?;
    let slope = def.slope / 100.0;
    let (channel, count) = match def.shape {
        CulvertShapeDef::Pipe { diameter, count, .. } => (Channel::Pipe { diameter }, count),
        CulvertShapeDef::Box { width, height, cells, .. } => (Channel::Box { width, height }, cells),
    };
    if slope <= 0.0 {
        return Some((0.0, 0.0));
    }
    let f = drainage::manning(channel, slope, n)?;
    Some((f.discharge * f64::from(count), f.velocity))
}

/// The query answered now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &DrainageQuery) -> Result<(DrainageQueried, Vec<CommandWarning>), Stop> {
    terrains::road_project(doc)?;
    judged(input)?;
    let id = crate::road_profiles::surface_id(doc, &input.surface, "surface")?;
    let culvert = match input.catchment.as_ref().and_then(|c| c.culvert.as_deref()) {
        Some(name) => Some(crate::road_culverts::find(doc.road_culverts(), name, "catchment.culvert")?.1.clone()),
        None => None,
    };
    let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
    let made = terrains::make(doc, &def);
    let surface = made.built()?;
    let tin = Tin::new(surface);
    let mut warnings = Vec::new();
    let ponds = match &input.ponds {
        Some(p) => drainage::ponds(&tin, p.min_depth)
            .into_iter()
            .map(|p| DrainagePond {
                bottom: p.bottom,
                level: p.level,
                outlet: p.outlet,
                depth: p.depth,
                area: p.area,
                volume: p.volume,
                corners: u32::try_from(p.corners).unwrap_or(u32::MAX),
            })
            .collect(),
        None => Vec::new(),
    };
    let drops = input
        .drops
        .iter()
        .map(|&from| {
            let d = drainage::water_drop(&tin, from);
            DrainageDrop { from, to: d.path.last().copied(), path: d.path, length: d.length, end: end_of(d.end) }
        })
        .collect();
    let catchment = match &input.catchment {
        None => None,
        Some(ask) => {
            let at = match (ask.at, &culvert) {
                (Some(p), _) => p,
                (None, Some(c)) => {
                    let read = crate::road_culverts::read(doc, c)?;
                    let r = &read.report;
                    let end = r.ends.iter().find(|e| e.side == r.inlet).or(r.ends.first()).ok_or(Stop::Cancelled)?;
                    [end.x, end.y]
                }
                (None, None) => return Err(invalid("Boşaltım noktasını verin.", "catchment.at")),
            };
            let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            for v in &surface.vertices {
                x0 = x0.min(v[0]);
                y0 = y0.min(v[1]);
                x1 = x1.max(v[0]);
                y1 = y1.max(v[1]);
            }
            let cells = (((x1 - x0) / ask.cell).floor() + 2.0) * (((y1 - y0) / ask.cell).floor() + 2.0);
            if !(cells.is_finite() && cells <= MOST_CATCHMENT_CELLS as f64) {
                return Err(invalid(
                    &format!("Havzanın ızgarası {MOST_CATCHMENT_CELLS} hücreyi aşar; aralığı büyütün."),
                    "catchment.cell",
                ));
            }
            match drainage::catchment(&tin, at, ask.cell, ask.radius) {
                None => {
                    warnings.push(CommandWarning {
                        code: codes::DRAINAGE_CATCHMENT_EMPTY.into(),
                        message: format!(
                            "Hiçbir hücrenin suyu ({}, {}) noktasının {} m eriminden geçmiyor; havza yok. Noktayı bir su damlasının yolu üstünde seçin ya da erimi büyütün.",
                            at[0], at[1], ask.radius
                        ),
                        path: Some("catchment.at".into()),
                    });
                    None
                }
                Some(c) => {
                    let peak = ask.runoff.zip(ask.intensity).map(|(r, i)| drainage::rational(r, i, c.area));
                    let capacity = match &culvert {
                        None => None,
                        Some(def) => match capacity(def) {
                            None => {
                                warnings.push(CommandWarning {
                                    code: codes::CULVERT_NO_ROUGHNESS.into(),
                                    message: format!("“{}” menfezinin pürüzlülüğü yok; kapasitesi verilmedi.", def.name),
                                    path: Some("catchment.culvert".into()),
                                });
                                None
                            }
                            Some((discharge, velocity)) => {
                                let ratio = peak.filter(|_| discharge > 0.0).map(|p| p / discharge);
                                if let (Some(p), true) = (peak, ratio.is_none_or(|r| r > 1.0)) {
                                    warnings.push(CommandWarning {
                                        code: codes::CULVERT_OVER_CAPACITY.into(),
                                        message: format!(
                                            "Havzanın en yüksek debisi {:.3} m³/s, “{}” menfezinin dolu akış kapasitesi {:.3} m³/s.",
                                            p, def.name, discharge
                                        ),
                                        path: Some("catchment.culvert".into()),
                                    });
                                }
                                Some(DrainageCapacity { culvert: def.id.clone(), discharge, velocity, ratio })
                            }
                        },
                    };
                    Some(DrainageCatchment {
                        at,
                        cells: u32::try_from(c.cells).unwrap_or(u32::MAX),
                        cell: ask.cell,
                        area: c.area,
                        outline: c.outline,
                        slope: if c.length > 0.0 { c.fall / c.length } else { 0.0 },
                        path: c.path,
                        length: c.length,
                        fall: c.fall,
                        peak,
                        capacity,
                    })
                }
            }
        }
    };
    Ok((DrainageQueried { surface: id, ponds, drops, catchment }, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &DrainageQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &DrainageQuery) -> CommandResult<DrainageQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer now; writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: DrainageQuery) -> CommandResult<DrainageQueried> {
    plan(cx, &input)
}

/// The answer for a window: the query's output and warnings, or why not.
pub fn preview(doc: &Document, input: &DrainageQuery) -> Result<(DrainageQueried, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
