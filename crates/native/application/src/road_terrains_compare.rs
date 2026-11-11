//! `road.terrains.compare` v1 (plan/road.md Y18.3, docs/adr/0916 §3): the difference of two terrain surfaces over the
//! comparison's plan (Civil 3D's TIN volume surface), read now with `kentos_road::volume::difference`: cut, fill and
//! net volumes, their areas, the extremes and the ranges' areas; with material zones (docs/adr/3540,
//! `kentos_road::volume_materials`) each zone's part, the rest and the adjusted volumes by material; by sections
//! (docs/adr/4340, `kentos_road::volume_sections`) each section's areas and the volumes between them, the sections
//! parallel or along a drawn axis (docs/adr/4760). Nothing is written. The shared cases are `fixtures/commands/v1/road.terrains.compare.json`, `….compare.zones.json` and
//! `….compare.sections.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CompareMethod, CompareZone, MOST_COMPARE_CELLS, MOST_COMPARE_SECTIONS, SectionFormula,
    TerrainAdjusted, TerrainDifferenceAt, TerrainDifferenceBand, TerrainSection, TerrainVolumes, TerrainZoneRest,
    TerrainZoneVolumes, TerrainsCompare, TerrainsCompared,
};
use kentos_commands::Entity;
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

/// The factors' range (docs/adr/0974, 3540).
const FACTOR_RANGE: &str = "Kazı ve dolgu katsayısı 0'dan büyük, en çok 10 olur (1: düzeltme yok).";

/// Material zones' names and factors (docs/adr/3540), refused at `field[k]`.
pub(crate) fn check_zones(zones: &[CompareZone], field: &str) -> Result<(), Stop> {
    let bad = |v: Option<f64>| v.is_some_and(|f| !(f.is_finite() && f > 0.0 && f <= 10.0));
    for (k, z) in zones.iter().enumerate() {
        if z.name.trim().is_empty() {
            return Err(invalid("Malzeme bölgesinin adı boş olamaz; malzemeyi yazın (ör. Kaya).", &format!("{field}[{k}].name")));
        }
        for (key, v) in [("cutFactor", z.cut_factor), ("fillFactor", z.fill_factor)] {
            if bad(v) {
                return Err(invalid(FACTOR_RANGE, &format!("{field}[{k}].{key}")));
            }
        }
    }
    Ok(())
}

/// Each material zone's part of the difference (docs/adr/3540), its factors its own else `cf`/`ff`, and the warning of
/// a zone partly off the two surfaces at `field[k].boundary`.
pub(crate) fn zone_volumes(
    comparison: &[Triangle],
    base: &[Triangle],
    zones: &[CompareZone],
    regions: &[Region],
    (cf, ff): (f64, f64),
    field: &str,
    warnings: &mut Vec<CommandWarning>,
) -> Vec<TerrainZoneVolumes> {
    let measured = volume_materials::measure(comparison, base, regions);
    let out: Vec<TerrainZoneVolumes> = zones
        .iter()
        .zip(&measured)
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
    for (k, z) in out.iter().enumerate() {
        let off = z.area - z.covered;
        if off > 1e-6 * z.area.max(1.0) {
            warnings.push(CommandWarning {
                code: codes::COMPARE_UNCOVERED.into(),
                message: format!("“{}” bölgesinin {off:.2} m²'si iki yüzeyin ortak alanının dışında; o kısım bölgenin hacmine girmedi.", z.name),
                path: Some(format!("{field}[{k}].boundary")),
            });
        }
    }
    out
}

/// The sections' widths' refusal (docs/adr/4760).
pub(crate) const WIDTHS: &str = "Kesitin sol ve sağ genişliği 0 ya da daha büyük bir uzunluk olmalı; ikisi birden 0 olamaz.";

/// A drawn axis (docs/adr/4760): a line or a single-part open polyline by its persistent id, each segment's bulge kept
/// (an arc's). Refused at `path`: the object missing, or not such a line.
/// The core's formula for the command's (docs/adr/4340, 5140).
pub(crate) fn section_formula(f: SectionFormula) -> volume_sections::Formula {
    match f {
        SectionFormula::EndArea => volume_sections::Formula::EndArea,
        SectionFormula::Prismoidal => volume_sections::Formula::Prismoidal,
        SectionFormula::Frustum => volume_sections::Formula::Frustum,
    }
}

pub(crate) fn axis_of(doc: &Document, uid: &str, path: &str) -> Result<volume_sections::Axis, Stop> {
    let entity = doc
        .entities_with_uids()
        .find(|(u, _)| u.to_string() == uid)
        .map(|(_, e)| e.clone())
        .ok_or_else(|| fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), path))?;
    let not = || invalid(&format!("“{uid}” kesitlerin ekseni olamaz: bir çizgi ya da tek parçalı açık bir çoklu çizgi olmalı."), path);
    let axis = match &entity {
        Entity::Line(l) => volume_sections::Axis { pts: vec![[l.a.x, l.a.y], [l.b.x, l.b.y]], bulges: Vec::new() },
        Entity::Polyline(p) if !p.closed && p.parts.as_ref().is_none_or(|q| q.is_empty()) && p.pts.len() >= 2 => {
            volume_sections::Axis {
                pts: p.pts.iter().map(|v| [v.x, v.y]).collect(),
                bulges: (0..p.pts.len() - 1)
                    .map(|i| kentos_geometry_core::geom::bulge::bulge_at(p.bulges.as_deref(), i))
                    .collect(),
            }
        }
        _ => return Err(not()),
    };
    Ok(axis)
}

/// The refusal of a section measurement (docs/adr/4340, 4760), its paths after `prefix` (`sections.` in a cut and fill
/// drawing).
pub(crate) fn sections_refused(e: SectionsError, prefix: &str) -> Stop {
    let at = |f: &str| format!("{prefix}{f}");
    match e {
        SectionsError::TooMany => invalid("Bu aralıkla 10 000'den çok kesit olur; kesit aralığını büyütün.", &at("interval")),
        SectionsError::Interval => invalid("Kesit aralığı 0'dan büyük bir uzunluk olmalı.", &at("interval")),
        SectionsError::Azimuth => invalid(AZIMUTH, &at("azimuth")),
        SectionsError::Width => invalid(WIDTHS, &at("left")),
        SectionsError::Axis => invalid("Kesitlerin ekseninin uzunluğu yok ya da yayı okunamıyor.", &at("axis")),
    }
}

fn triangles(s: &Surface) -> Vec<Triangle> {
    s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect()
}

/// The volumes adjusted by a cut and a fill factor.
pub(crate) fn adjust(cut: f64, fill: f64, cut_factor: f64, fill_factor: f64) -> TerrainAdjusted {
    TerrainAdjusted { cut_factor, fill_factor, cut: cut * cut_factor, fill: fill * fill_factor, net: fill * fill_factor - cut * cut_factor }
}

/// The zones' boundaries (docs/adr/3540), each a closed object of the drawing, no two sharing more than 1 mm².
fn zone_regions(doc: &Document, zones: &[CompareZone]) -> Result<Vec<Region>, Stop> {
    zone_regions_at(doc, zones, "zones")
}

/// The zones' boundaries as `zone_regions` reads them, their refusals at `field[k].boundary` (docs/adr/4760: a cut and
/// fill drawing's `materials`).
pub(crate) fn zone_regions_at(doc: &Document, zones: &[CompareZone], field: &str) -> Result<Vec<Region>, Stop> {
    let mut out: Vec<Region> = Vec::with_capacity(zones.len());
    for (k, z) in zones.iter().enumerate() {
        let path = format!("{field}[{k}].boundary");
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
        // Docs/adr/4760: a drawn axis with its widths, or an azimuth.
        if input.axis.is_some() && input.azimuth.is_some() {
            return Err(invalid(
                "Kesitlerin ekseni ya semtle ya da çizimdeki bir eksenle verilir; ikisinden birini kaldırın.",
                "azimuth",
            ));
        }
        if input.axis.is_some() {
            for (path, v) in [("left", input.left), ("right", input.right)] {
                match v {
                    None => return Err(invalid("Eksenle kesitlerin sol ve sağ genişliğini (left, right, m) verin.", path)),
                    Some(w) if !(w.is_finite() && w >= 0.0) => return Err(invalid(WIDTHS, path)),
                    _ => {}
                }
            }
            if input.left.unwrap_or(0.0) + input.right.unwrap_or(0.0) <= 0.0 {
                return Err(invalid(WIDTHS, "left"));
            }
        }
    } else if input.axis.is_some() {
        return Err(invalid("Çizimdeki eksen kesit yönteminindir; yöntemi kesit seçin ya da ekseni kaldırın.", "axis"));
    } else if input.formula.is_some() {
        return Err(invalid(
            "Formül (ortalama alan ya da prizmoidal) kesit yönteminindir; yöntemi kesit seçin ya da formülü kaldırın.",
            "formula",
        ));
    }
    if input.axis.is_none() {
        for (path, given) in [("left", input.left.is_some()), ("right", input.right.is_some()), ("corners", input.corners.is_some())] {
            if given {
                return Err(invalid("Kesitlerin genişlikleri ve köşe kesitleri çizimdeki eksenle verilir; ekseni (axis) verin.", path));
            }
        }
    }
    const FACTOR: &str = "Kazı ve dolgu katsayısı 0'dan büyük, en çok 10 olur (1: düzeltme yok).";
    let bad = |v: Option<f64>| v.is_some_and(|f| !(f.is_finite() && f > 0.0 && f <= 10.0));
    for (path, v) in [("cutFactor", input.cut_factor), ("fillFactor", input.fill_factor)] {
        if bad(v) {
            return Err(invalid(FACTOR, path));
        }
    }
    // Material zones (docs/adr/3540): each named, its factors in the comparison's range.
    check_zones(&input.zones, "zones")?;
    let base = crate::road_profiles::surface_id(doc, &input.base, "base")?;
    let comparison = crate::road_profiles::surface_id(doc, &input.comparison, "comparison")?;
    if base == comparison {
        return Err(invalid("Taban ile karşılaştırma aynı yüzey; iki ayrı yüzey verin.", "comparison"));
    }
    // Docs/adr/4760: the drawn axis the sections stand along.
    let axis = match (sections, input.axis.as_deref()) {
        (true, Some(uid)) => Some(axis_of(doc, uid, "axis")?),
        _ => None,
    };
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
    let prismoidal = section_formula(formula);
    // Each section's row, and the axis's length along a drawn axis (docs/adr/4760).
    let by_sections: Option<(Vec<TerrainSection>, f64, f64, Option<f64>)> = match (sections, input.interval, &axis) {
        (true, Some(interval), Some(a)) => {
            let s = volume_sections::along_axis(
                &triangles(sc),
                &triangles(sb),
                a,
                interval,
                input.left.unwrap_or(0.0),
                input.right.unwrap_or(0.0),
                input.corners.unwrap_or(false),
                prismoidal,
                MOST_COMPARE_SECTIONS,
            )
            .map_err(|e| sections_refused(e, ""))?;
            let rows = s
                .rows
                .iter()
                .map(|r| TerrainSection {
                    station: r.station,
                    cut_area: r.cut,
                    fill_area: r.fill,
                    at: Some(r.at),
                    ends: Some([r.left, r.right]),
                })
                .collect();
            Some((rows, s.cut, s.fill, Some(s.length)))
        }
        (true, Some(interval), None) => {
            let s = volume_sections::measure(&triangles(sc), &triangles(sb), azimuth, interval, prismoidal, MOST_COMPARE_SECTIONS)
                .map_err(|e| sections_refused(e, ""))?;
            let rows = s
                .rows
                .iter()
                .map(|r| TerrainSection { station: r.station, cut_area: r.cut, fill_area: r.fill, at: None, ends: None })
                .collect();
            Some((rows, s.cut, s.fill, None))
        }
        _ => None,
    };
    let v = measured.as_ref().unwrap_or(&d.volumes);
    // The volumes of the method asked: the sections', the grid's, else the exact TIN's.
    let (main_cut, main_fill) = by_sections.as_ref().map_or((v.cut, v.fill), |s| (s.1, s.2));
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
    let zones = if input.zones.is_empty() {
        Vec::new()
    } else {
        zone_volumes(&triangles(sc), &triangles(sb), &input.zones, &regions, (cf, ff), "zones", &mut warnings)
    };
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
        azimuth: by_sections.as_ref().filter(|_| axis.is_none()).map(|_| azimuth),
        formula: by_sections.as_ref().map(|_| formula),
        axis: by_sections.as_ref().and(input.axis.clone()).filter(|_| axis.is_some()),
        length: by_sections.as_ref().and_then(|s| s.3),
        sections: by_sections.map(|s| s.0).unwrap_or_default(),
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
