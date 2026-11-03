//! A road project's earthwork volumes (plan/road.md Y8, docs/adr/0906): found by id or name, computed now from their
//! corridor built now (`road_corridors::make`), by sections (the average end area, or the prismoidal formula with a
//! section built in the middle of each pair) or between the corridor's top surface and its terrain (`kentos-road`'s
//! `volume::Ground`, strip by strip), with the cut and fill factors of the stretch's middle, and the running totals.
//! Nothing computed is kept: every read computes again (ROAD-MOD-06).

use kentos_commands::{CommandWarning, VolumeDef, VolumeListed, VolumeMethod, VolumeReport, VolumeRow};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::volume::{Ground, Triangle, prismoidal};

use crate::checks::Stop;
use crate::codes;
use crate::road_corridors as corridors;
use crate::road_terrains::fail;

/// Only a road project has earthwork volumes.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(codes::NOT_A_ROAD_PROJECT, "Kübaj yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""))
    }
}

/// The volume of `name`: by its id, else by its name (case aside).
pub(crate) fn find<'a>(list: &'a [VolumeDef], name: &str, path: &str) -> Result<(usize, &'a VolumeDef), Stop> {
    list.iter()
        .position(|v| v.id == name)
        .or_else(|| list.iter().position(|v| v.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_VOLUME, format!("“{name}” kimlikli ya da adlı kübaj hesabı projede yok."), path))
}

pub(crate) fn listed(list: &[VolumeDef]) -> Vec<VolumeListed> {
    list.iter()
        .map(|v| VolumeListed {
            id: v.id.clone(),
            name: v.name.clone(),
            corridor: v.corridor.clone(),
            method: v.method,
            revision: v.revision,
        })
        .collect()
}

fn warning(code: &str, message: String) -> CommandWarning {
    CommandWarning { code: code.into(), message, path: Some("volume".into()) }
}

/// The factors of the stretch whose middle is `m`: the row with `from ≤ m < to` (the last row's end included).
fn factors_at(def: &VolumeDef, m: f64) -> (Option<f64>, Option<f64>) {
    let n = def.factors.len();
    def.factors
        .iter()
        .enumerate()
        .find(|(i, f)| f.from <= m && (m < f.to || (*i + 1 == n && m <= f.to)))
        .map_or((None, None), |(_, f)| (f.cut, f.fill))
}

/// One stretch's raw volumes: none where it has none.
struct Raw {
    from: f64,
    to: f64,
    volumes: Option<(f64, f64)>,
}

/// The volume computed now, with its warnings; refused where its corridor does not build.
pub(crate) fn compute(doc: &Document, def: &VolumeDef) -> Result<(VolumeReport, Vec<CommandWarning>), Stop> {
    let cdef = doc.road_corridors().iter().find(|c| c.id == def.corridor).ok_or_else(|| {
        fail(codes::UNKNOWN_CORRIDOR, format!("“{}” kimlikli koridor projede yok.", def.corridor), "corridor")
    })?;
    let made = corridors::make(doc, cdef, false)
        .map_err(|stop| fail(codes::INVALID_VOLUME, format!("“{}” koridoru kurulamıyor: {}", cdef.name, corridors::words(stop)), "corridor"))?;
    let mut warnings: Vec<CommandWarning> =
        made.warnings.iter().filter(|w| w.code == codes::CORRIDOR_STALE).cloned().collect();
    let mut raws: Vec<Raw> = Vec::new();
    let (mut cut_area, mut fill_area, mut uncovered) = (0.0, 0.0, 0.0);
    match def.method {
        VolumeMethod::EndArea | VolumeMethod::Prismoidal => {
            for region in &made.regions {
                for w in region.crosses.windows(2) {
                    let (s0, s1) = (w[0].line.station, w[1].line.station);
                    let l = s1 - s0;
                    let volumes = match (&w[0].section, &w[1].section) {
                        (Some(a), Some(b)) => match def.method {
                            VolumeMethod::EndArea => Some((l * (a.cut + b.cut) / 2.0, l * (a.fill + b.fill) / 2.0)),
                            _ => made
                                .sampler
                                .cross(&region.template, (s0 + s1) / 2.0)
                                .and_then(|c| c.section)
                                .map(|m| (prismoidal(a.cut, m.cut, b.cut, l), prismoidal(a.fill, m.fill, b.fill, l))),
                        },
                        _ => None,
                    };
                    raws.push(Raw { from: s0, to: s1, volumes });
                }
            }
        }
        VolumeMethod::Surfaces => {
            let Some(surface) = &made.sampler.surface else {
                return Err(fail(
                    codes::INVALID_VOLUME,
                    format!("“{}” koridorunun yüzeyi kurulamıyor; yüzeyler arası hacim hesaplanamaz.", cdef.name),
                    "corridor",
                ));
            };
            let terrain: Vec<Triangle> = surface
                .triangles
                .iter()
                .map(|t| [surface.vertices[t[0] as usize], surface.vertices[t[1] as usize], surface.vertices[t[2] as usize]])
                .collect();
            let mut ground = Ground::new(&terrain);
            let strips = &made.built.strips;
            for (ri, region) in made.regions.iter().enumerate() {
                for w in region.crosses.windows(2) {
                    let (s0, s1) = (w[0].line.station, w[1].line.station);
                    let volumes = strips.iter().find(|s| s.0 == ri && s.1 == s0 && s.2 == s1).map(|s| {
                        let v = ground.volumes(&made.built.triangles[s.3.clone()]);
                        cut_area += v.cut_area;
                        fill_area += v.fill_area;
                        uncovered += v.uncovered;
                        (v.cut, v.fill)
                    });
                    raws.push(Raw { from: s0, to: s1, volumes });
                }
            }
        }
    }
    let text = |s: f64| km_text(made.sampler.alignment.km(s), 3);
    let (mut cut, mut fill, mut fcut, mut ffill) = (0.0, 0.0, 0.0, 0.0);
    let mut unfactored = 0usize;
    let mut rows = Vec::with_capacity(raws.len());
    for r in &raws {
        let (cf, ff) = factors_at(def, (r.from + r.to) / 2.0);
        let (c, f) = match r.volumes {
            Some(v) => (Some(v.0), Some(v.1)),
            None => (None, None),
        };
        if let (Some(c), Some(f)) = (c, f) {
            cut += c;
            fill += f;
            fcut += c * cf.unwrap_or(1.0);
            ffill += f * ff.unwrap_or(1.0);
            if cf.is_none() || ff.is_none() {
                unfactored += 1;
            }
        }
        rows.push(VolumeRow {
            from: r.from,
            to: r.to,
            from_text: text(r.from),
            to_text: text(r.to),
            cut: c,
            fill: f,
            cut_factor: cf,
            fill_factor: ff,
            total_cut: fcut,
            total_fill: ffill,
            mass: fcut - ffill,
        });
    }
    // The stretches without volume: the corridor's gaps between regions and the rows that have none, joined.
    let mut gaps: Vec<[f64; 2]> = made.report.gaps.clone();
    for r in raws.iter().filter(|r| r.volumes.is_none()) {
        gaps.push([r.from, r.to]);
    }
    gaps.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let mut joined: Vec<[f64; 2]> = Vec::new();
    for g in gaps {
        match joined.last_mut() {
            Some(last) if g[0] <= last[1] + 1e-9 => last[1] = last[1].max(g[1]),
            _ => joined.push(g),
        }
    }
    if !joined.is_empty() {
        let words: Vec<String> = joined.iter().take(8).map(|g| format!("{} – {}", text(g[0]), text(g[1]))).collect();
        let more = if joined.len() > 8 { format!(" ve {} aralık daha", joined.len() - 8) } else { String::new() };
        warnings.push(warning(
            codes::VOLUME_GAP,
            format!("“{}” kübajında hacmi olmayan aralıklar: {}{more}.", def.name, words.join(", ")),
        ));
    }
    if def.method == VolumeMethod::Surfaces && uncovered > 1e-6 {
        warnings.push(warning(
            codes::VOLUME_UNCOVERED,
            format!("“{}” kübajında tasarım yüzeyinin {uncovered:.3} m²'sini arazi örtmüyor; o alanın hacmi yok.", def.name),
        ));
    }
    if unfactored > 0 {
        warnings.push(warning(
            codes::VOLUME_UNFACTORED,
            format!(
                "“{}” kübajının {unfactored} aralığına kabarma ya da sıkışma katsayısı verilmedi; orada düzeltilmiş hacim ham hacimdir.",
                def.name
            ),
        ));
    }
    let surfaces = def.method == VolumeMethod::Surfaces;
    let report = VolumeReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        corridor: def.corridor.clone(),
        method: def.method,
        stale: made.report.stale,
        rows,
        cut,
        fill,
        factored_cut: fcut,
        factored_fill: ffill,
        cut_area: surfaces.then_some(cut_area),
        fill_area: surfaces.then_some(fill_area),
        uncovered: surfaces.then_some(uncovered),
        gaps: joined,
        problem: None,
    };
    Ok((report, warnings))
}

/// A report for a volume that does not compute: its sources and why.
pub(crate) fn broken(def: &VolumeDef, why: String) -> VolumeReport {
    VolumeReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        corridor: def.corridor.clone(),
        method: def.method,
        stale: false,
        rows: Vec::new(),
        cut: 0.0,
        fill: 0.0,
        factored_cut: 0.0,
        factored_fill: 0.0,
        cut_area: None,
        fill_area: None,
        uncovered: None,
        gaps: Vec::new(),
        problem: Some(why),
    }
}

/// The volume's report now, or why it does not compute.
pub(crate) fn report(doc: &Document, def: &VolumeDef) -> VolumeReport {
    match compute(doc, def) {
        Ok((r, _)) => r,
        Err(stop) => broken(def, corridors::words(stop)),
    }
}

/// An earthwork volume computed now: its report and its warnings, or why it does not compute.
pub type MadeVolume = Result<(VolumeReport, Vec<CommandWarning>), String>;

/// The road project's earthwork volumes computed now, for the desktop's Kübaj window (plan/road.md Y8).
pub fn made_volumes(doc: &Document) -> Vec<(VolumeDef, MadeVolume)> {
    if road_project(doc).is_err() {
        return Vec::new();
    }
    doc.road_volumes().iter().map(|v| (v.clone(), compute(doc, v).map_err(corridors::words))).collect()
}

/// A volume (perhaps not written) computed now, for the Kübaj window.
pub fn preview(doc: &Document, def: &VolumeDef) -> MadeVolume {
    compute(doc, def).map_err(corridors::words)
}
