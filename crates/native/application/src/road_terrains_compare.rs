//! `road.terrains.compare` v1 (plan/road.md Y18.3, docs/adr/0916 §3): the difference of two terrain surfaces over the
//! comparison's plan (Civil 3D's TIN volume surface), read now with `kentos_road::volume::difference`: cut, fill and
//! net volumes, their areas, the extremes and the ranges' areas. Nothing is written. The shared cases are
//! `fixtures/commands/v1/road.terrains.compare.json`.

use kentos_commands::{
    CommandResult, CommandWarning, MOST_COMPARE_CELLS, TerrainAdjusted, TerrainDifferenceAt, TerrainDifferenceBand,
    CompareMethod, TerrainVolumes, TerrainsCompare, TerrainsCompared,
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
    terrains::terrain_project(doc)?;
    if let Some(b) = &input.ranges
        && (b.len() < 2 || b.iter().any(|v| !v.is_finite()) || b.windows(2).any(|w| w[0] >= w[1]))
    {
        return Err(invalid("Fark aralıklarının sınırları en az iki, sonlu ve artan sayılar olmalı.", "ranges"));
    }
    // The method (docs/adr/1600): a grid needs its cell; its ranges are the exact TIN's alone.
    let grid = input.method == Some(CompareMethod::Grid);
    if grid {
        match input.cell {
            None => return Err(invalid("Izgara yönteminde hücrenin kenarını (cell, m) verin.", "cell")),
            Some(c) if !(c.is_finite() && c > 0.0) => {
                return Err(invalid("Izgaranın hücresi 0'dan büyük bir uzunluk olmalı.", "cell"));
            }
            _ => {}
        }
        if input.ranges.is_some() {
            return Err(invalid(
                "Fark aralıkları TIN yöntemiyle verilir; ızgara yönteminde aralık istenmez.",
                "ranges",
            ));
        }
    }
    for (path, v) in [("cutFactor", input.cut_factor), ("fillFactor", input.fill_factor)] {
        if v.is_some_and(|f| !(f.is_finite() && f > 0.0 && f <= 10.0)) {
            return Err(invalid("Kazı ve dolgu katsayısı 0'dan büyük, en çok 10 olur (1: düzeltme yok).", path));
        }
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
    // The grid's cells over the comparison: at most MOST_COMPARE_CELLS, counted from its plan's extent.
    let measured = match (grid, input.cell) {
        (true, Some(cell)) => {
            let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
            for v in &sc.vertices {
                x0 = x0.min(v[0]);
                y0 = y0.min(v[1]);
                x1 = x1.max(v[0]);
                y1 = y1.max(v[1]);
            }
            if ((x1 - x0) / cell + 1.0) * ((y1 - y0) / cell + 1.0) > MOST_COMPARE_CELLS {
                return Err(invalid(
                    "Izgaranın hücresi bu alan için çok küçük: en çok 4 000 000 hücre ölçülür; hücreyi büyütün.",
                    "cell",
                ));
            }
            let ground = volume::Ground::new(&triangles(sb));
            Some(ground.grid_volumes(&triangles(sc), cell, &mut std::collections::HashSet::new()))
        }
        _ => None,
    };
    let v = measured.as_ref().unwrap_or(&d.volumes);
    if v.uncovered > 0.0 {
        warnings.push(CommandWarning {
            code: codes::COMPARE_UNCOVERED.into(),
            message: format!(
                "Karşılaştırma yüzeyinin {:.2} m²'si tabanın dışında; o kısım hacme girmedi.",
                v.uncovered
            ),
            path: Some("comparison".into()),
        });
    }
    let at = |e: Option<(f64, [f64; 2])>| e.map(|(difference, at)| TerrainDifferenceAt { difference, at });
    let out = TerrainsCompared {
        base,
        comparison,
        cut: v.cut,
        fill: v.fill,
        net: v.fill - v.cut,
        cut_area: v.cut_area,
        fill_area: v.fill_area,
        covered: v.covered,
        uncovered: v.uncovered,
        least: at(d.least),
        largest: at(d.largest),
        bands: d.bands.iter().map(|b| TerrainDifferenceBand { from: b.0, to: b.1, area: b.2 }).collect(),
        outside_area: input.ranges.as_ref().map(|_| d.outside),
        cell: measured.as_ref().and(input.cell),
        exact: measured.as_ref().map(|_| TerrainVolumes {
            cut: d.volumes.cut,
            fill: d.volumes.fill,
            net: d.volumes.fill - d.volumes.cut,
        }),
        adjusted: (input.cut_factor.is_some() || input.fill_factor.is_some()).then(|| {
            let (cf, ff) = (input.cut_factor.unwrap_or(1.0), input.fill_factor.unwrap_or(1.0));
            TerrainAdjusted { cut_factor: cf, fill_factor: ff, cut: v.cut * cf, fill: v.fill * ff, net: v.fill * ff - v.cut * cf }
        }),
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
