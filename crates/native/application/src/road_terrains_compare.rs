//! `road.terrains.compare` v1 (plan/road.md Y18.3, docs/adr/0916 §3): the difference of two terrain surfaces over the
//! comparison's plan (Civil 3D's TIN volume surface), read now with `kentos_road::volume::difference`: cut, fill and
//! net volumes, their areas, the extremes and the ranges' areas; with material zones (docs/adr/3540,
//! `kentos_road::volume_materials`) each zone's part, the rest and the adjusted volumes by material; by sections
//! (docs/adr/4340, `kentos_road::volume_sections`) each section's areas and the volumes between them. Nothing is
//! written. The shared cases are `fixtures/commands/v1/road.terrains.compare.json`, `….compare.zones.json` and
//! `….compare.sections.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CompareMethod, CompareZone, MOST_COMPARE_CELLS, MOST_COMPARE_SECTIONS, SectionFormula,
    TerrainAdjusted, TerrainDifferenceAt, TerrainDifferenceBand, TerrainSection, TerrainVolumes, TerrainZoneRest,
    TerrainZoneVolumes, TerrainsCompare, TerrainsCompared,
};
use kentos_domain::Document;
use kentos_road::surface::{Region, Surface};
use kentos_road::volume::{self, Triangle};
use kentos_road::volume_materials;
use kentos_road::volume_sections::{self, SectionsError};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

/// The section method's axis (docs/adr/4340).
const AZIMUTH: &str = "Eksenin semti 0 ile 360 derece arasında olmalı (kuzeyden saat yönünde; 360 hariç).";

fn invalid(message: &str, path: &str) -> Stop {
    fail(codes::INVALID_ANALYSIS, message.to_owned(), path)
}

fn triangles(s: &Surface) -> Vec<Triangle> {
    s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect()
}

/// The volumes adjusted by a cut and a fill factor.
fn adjust(cut: f64, fill: f64, cut_factor: f64, fill_factor: f64) -> TerrainAdjusted {
    TerrainAdjusted { cut_factor, fill_factor, cut: cut * cut_factor, fill: fill * fill_factor, net: fill * fill_factor - cut * cut_factor }
}

/// The zones' boundaries (docs/adr/3540), each a closed object of the drawing, no two sharing more than 1 mm².
fn zone_regions(doc: &Document, zones: &[CompareZone]) -> Result<Vec<Region>, Stop> {
    let mut out: Vec<Region> = Vec::with_capacity(zones.len());
    for (k, z) in zones.iter().enumerate() {
        let path = format!("zones[{k}].boundary");
        // The coverage's rule for a boundary, its refusal said at the zone (an analysis's input here).
        let region = crate::road_terrains_coverage::boundary_of(doc, &z.boundary).map_err(|stop| match stop {
            Stop::Failed(mut e) => {
                if e.code == codes::INVALID_COVERAGE {
                    e.code = codes::INVALID_ANALYSIS.into();
                }
                e.path = Some(path.clone());
                Stop::Failed(e)
            }
            other => other,
        })?;
        for (j, earlier) in out.iter().enumerate() {
            let shared = volume_materials::overlap(earlier, &region);
            if shared > volume_materials::OVERLAP {
                return Err(invalid(
                    &format!(
                        "“{}” ile “{}” bölgeleri üst üste biniyor ({shared:.2} m²): bir yer tek malzemenin olur; sınırları ayırın.",
                        zones[j].name.trim(),
                        z.name.trim()
                    ),
                    &path,
                ));
            }
        }
        out.push(region);
    }
    Ok(out)
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
        if !input.zones.is_empty() {
            return Err(invalid(
                "Malzeme bölgeleri TIN yöntemiyle ölçülür; ızgara yönteminde bölge istenmez.",
                "zones",
            ));
        }
    }
    // The section method (docs/adr/4340): its interval and the axis's azimuth; its ranges and zones are the exact TIN's.
    let sections = input.method == Some(CompareMethod::Sections);
    if sections {
        match input.interval {
            None => return Err(invalid("Kesit yönteminde kesitlerin aralığını (interval, m) verin.", "interval")),
            Some(v) if !(v.is_finite() && v > 0.0) => {
                return Err(invalid("Kesit aralığı 0'dan büyük bir uzunluk olmalı.", "interval"));
            }
            _ => {}
        }
        if input.azimuth.is_some_and(|a| !(a.is_finite() && (0.0..360.0).contains(&a))) {
            return Err(invalid(AZIMUTH, "azimuth"));
        }
        if input.ranges.is_some() {
            return Err(invalid("Fark aralıkları TIN yöntemiyle verilir; kesit yönteminde aralık istenmez.", "ranges"));
        }
        if !input.zones.is_empty() {
            return Err(invalid("Malzeme bölgeleri TIN yöntemiyle ölçülür; kesit yönteminde bölge istenmez.", "zones"));
        }
    } else if input.formula.is_some() {
        return Err(invalid(
            "Formül (ortalama alan ya da prizmoidal) kesit yönteminindir; yöntemi kesit seçin ya da formülü kaldırın.",
            "formula",
        ));
    }
    const FACTOR: &str = "Kazı ve dolgu katsayısı 0'dan büyük, en çok 10 olur (1: düzeltme yok).";
    let bad = |v: Option<f64>| v.is_some_and(|f| !(f.is_finite() && f > 0.0 && f <= 10.0));
    for (path, v) in [("cutFactor", input.cut_factor), ("fillFactor", input.fill_factor)] {
        if bad(v) {
            return Err(invalid(FACTOR, path));
        }
    }
    // Material zones (docs/adr/3540): each named, its factors in the comparison's range.
    for (k, z) in input.zones.iter().enumerate() {
        if z.name.trim().is_empty() {
            return Err(invalid("Malzeme bölgesinin adı boş olamaz; malzemeyi yazın (ör. Kaya).", &format!("zones[{k}].name")));
        }
        for (key, v) in [("cutFactor", z.cut_factor), ("fillFactor", z.fill_factor)] {
            if bad(v) {
                return Err(invalid(FACTOR, &format!("zones[{k}].{key}")));
            }
        }
    }
    let base = crate::road_profiles::surface_id(doc, &input.base, "base")?;
    let comparison = crate::road_profiles::surface_id(doc, &input.comparison, "comparison")?;
    if base == comparison {
        return Err(invalid("Taban ile karşılaştırma aynı yüzey; iki ayrı yüzey verin.", "comparison"));
    }
    let regions = zone_regions(doc, &input.zones)?;
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
    // The sections (docs/adr/4340), on the two surfaces' triangles.
    let azimuth = input.azimuth.unwrap_or(0.0);
    let formula = input.formula.unwrap_or_default();
    let by_sections = match (sections, input.interval) {
        (true, Some(interval)) => Some(
            volume_sections::measure(
                &triangles(sc),
                &triangles(sb),
                azimuth,
                interval,
                formula == SectionFormula::Prismoidal,
                MOST_COMPARE_SECTIONS,
            )
            .map_err(|e| match e {
                SectionsError::TooMany => {
                    invalid("Bu aralıkla 10 000'den çok kesit olur; kesit aralığını büyütün.", "interval")
                }
                SectionsError::Interval => invalid("Kesit aralığı 0'dan büyük bir uzunluk olmalı.", "interval"),
                SectionsError::Azimuth => invalid(AZIMUTH, "azimuth"),
            })?,
        ),
        _ => None,
    };
    let v = measured.as_ref().unwrap_or(&d.volumes);
    // The volumes of the method asked: the sections', the grid's, else the exact TIN's.
    let (main_cut, main_fill) = by_sections.as_ref().map_or((v.cut, v.fill), |s| (s.cut, s.fill));
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
    // The zones' parts (docs/adr/3540): each zone's own factors, else the comparison's, else 1; the rest is the exact
    // difference less the zones' (they do not overlap) with the comparison's factors.
    let (cf, ff) = (input.cut_factor.unwrap_or(1.0), input.fill_factor.unwrap_or(1.0));
    let measured_zones = volume_materials::measure(&triangles(sc), &triangles(sb), &regions);
    let zones: Vec<TerrainZoneVolumes> = input
        .zones
        .iter()
        .zip(&measured_zones)
        .map(|(z, m)| TerrainZoneVolumes {
            name: z.name.trim().to_owned(),
            boundary: z.boundary.clone(),
            area: m.area,
            covered: m.covered,
            cut: m.cut,
            fill: m.fill,
            net: m.fill - m.cut,
            cut_area: m.cut_area,
            fill_area: m.fill_area,
            adjusted: adjust(m.cut, m.fill, z.cut_factor.unwrap_or(cf), z.fill_factor.unwrap_or(ff)),
        })
        .collect();
    for (k, z) in zones.iter().enumerate() {
        let off = z.area - z.covered;
        if off > 1e-6 * z.area.max(1.0) {
            warnings.push(CommandWarning {
                code: codes::COMPARE_UNCOVERED.into(),
                message: format!("“{}” bölgesinin {off:.2} m²'si iki yüzeyin ortak alanının dışında; o kısım bölgenin hacmine girmedi.", z.name),
                path: Some(format!("zones[{k}].boundary")),
            });
        }
    }
    let rest = (!zones.is_empty()).then(|| {
        let sum = |f: fn(&TerrainZoneVolumes) -> f64| zones.iter().map(f).sum::<f64>();
        let (cut, fill) = (d.volumes.cut - sum(|z| z.cut), d.volumes.fill - sum(|z| z.fill));
        TerrainZoneRest {
            covered: d.volumes.covered - sum(|z| z.covered),
            cut,
            fill,
            net: fill - cut,
            cut_area: d.volumes.cut_area - sum(|z| z.cut_area),
            fill_area: d.volumes.fill_area - sum(|z| z.fill_area),
            adjusted: adjust(cut, fill, cf, ff),
        }
    });
    let zoned = rest.as_ref().map(|r| {
        let cut = zones.iter().map(|z| z.adjusted.cut).sum::<f64>() + r.adjusted.cut;
        let fill = zones.iter().map(|z| z.adjusted.fill).sum::<f64>() + r.adjusted.fill;
        TerrainVolumes { cut, fill, net: fill - cut }
    });
    let at = |e: Option<(f64, [f64; 2])>| e.map(|(difference, at)| TerrainDifferenceAt { difference, at });
    let out = TerrainsCompared {
        base,
        comparison,
        cut: main_cut,
        fill: main_fill,
        net: main_fill - main_cut,
        cut_area: v.cut_area,
        fill_area: v.fill_area,
        covered: v.covered,
        uncovered: v.uncovered,
        least: at(d.least),
        largest: at(d.largest),
        bands: d.bands.iter().map(|b| TerrainDifferenceBand { from: b.0, to: b.1, area: b.2 }).collect(),
        outside_area: input.ranges.as_ref().map(|_| d.outside),
        cell: measured.as_ref().and(input.cell),
        exact: (measured.is_some() || by_sections.is_some()).then(|| TerrainVolumes {
            cut: d.volumes.cut,
            fill: d.volumes.fill,
            net: d.volumes.fill - d.volumes.cut,
        }),
        adjusted: (input.cut_factor.is_some() || input.fill_factor.is_some()).then(|| adjust(main_cut, main_fill, cf, ff)),
        zones,
        rest,
        zoned,
        interval: by_sections.as_ref().and(input.interval),
        azimuth: by_sections.as_ref().map(|_| azimuth),
        formula: by_sections.as_ref().map(|_| formula),
        sections: by_sections
            .map(|s| s.rows.iter().map(|r| TerrainSection { station: r.station, cut_area: r.cut, fill_area: r.fill }).collect())
            .unwrap_or_default(),
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
