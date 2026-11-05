//! A road project's earthwork volumes (plan/road.md Y8, docs/adr/0906): found by id or name, computed now from their
//! corridor built now (`road_corridors::make`), by sections (the average end area, or the prismoidal formula with a
//! section built in the middle of each pair) or between the corridor's top surface and its terrain (`kentos-road`'s
//! `volume::Ground`, strip by strip), with the cut and fill factors of the stretch's middle, and the running totals.
//! Nothing computed is kept: every read computes again (ROAD-MOD-06).

use kentos_commands::{CommandWarning, PavementEdge, VolumeDef, VolumeLayer, VolumeListed, VolumeMethod, VolumeReport, VolumeRow};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::section::pavement::{self as pav, Edge, Pavement};
use kentos_road::section::template as tpl;
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
            approved: v.approved.is_some(),
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
        .map_or((None, None), |(_, f)| f.effective())
}

/// The topsoil depth at station `s`: the strip row with `from ≤ s < to` (the last row's end included); 0 off them.
fn depth_at(def: &VolumeDef, s: f64) -> f64 {
    let n = def.strip.len();
    def.strip
        .iter()
        .enumerate()
        .find(|(i, f)| f.from <= s && (s < f.to || (*i + 1 == n && s <= f.to)))
        .map_or(0.0, |(_, f)| f.depth)
}

/// One stretch's raw volumes: none where it has none; the topsoil and the benches where the volume has them.
struct Raw {
    from: f64,
    to: f64,
    volumes: Option<(f64, f64)>,
    /// The volumes without the curve correction, where it applied (docs/adr/0934).
    plain: Option<(f64, f64)>,
    extra: Option<(f64, f64)>,
    /// Each report layer's volume over the stretch (docs/adr/0930 §2); empty without a pavement.
    layers: Vec<Option<f64>>,
}

/// A section's earthwork as a volume's rule reads it (docs/adr/0930 §2): the cut and fill between its two lines, the
/// topsoil stripped, the benches, and each report layer's area (none: the template has no such layer).
#[derive(Clone)]
struct Work {
    cut: f64,
    fill: f64,
    strip: f64,
    bench: f64,
    layers: Vec<Option<f64>>,
    /// For the curve correction (docs/adr/0934): the road's cut and fill centroids' offsets (right positive; 0 without
    /// area), the curvature there and the section's farthest offsets left and right.
    centroid: (f64, f64),
    curvature: f64,
    reach: (f64, f64),
}

impl Work {
    /// The cut and fill as they sweep along the curve, `A(1 + κ·e)`; none where the section reaches past the centre.
    fn curved(&self) -> Option<(f64, f64)> {
        // The core's rule (kentos_road::volume::effective, tested against Pappus): the moment Q = e·A.
        let section = kentos_road::volume::CurvedSection {
            station: 0.0,
            areas: Some((self.cut, self.fill)),
            moments: (self.cut * self.centroid.0, self.fill * self.centroid.1),
            curvature: self.curvature,
            reach: self.reach,
        };
        kentos_road::volume::effective(&section).flatten()
    }
}

/// The core's pavement of a template's definition.
pub fn core_pavement(def: &kentos_commands::PavementDef) -> Pavement {
    Pavement {
        layers: def.layers.iter().map(|l| l.thickness).collect(),
        edge: match def.edge {
            PavementEdge::Vertical => Edge::Vertical,
            PavementEdge::Slope { h } => Edge::Slope(h),
        },
    }
}

/// The report's layers, by name in the order met over the regions' templates (the first one's material), and for each
/// region the index of each of its layers among them.
fn layers_of(regions: &[Option<&kentos_commands::PavementDef>]) -> (Vec<VolumeLayer>, Vec<Vec<usize>>) {
    let mut out: Vec<VolumeLayer> = Vec::new();
    let mut at = Vec::new();
    for r in regions {
        let mut idx = Vec::new();
        for l in r.map(|p| p.layers.as_slice()).unwrap_or_default() {
            let i = out.iter().position(|o| o.name == l.name).unwrap_or_else(|| {
                out.push(VolumeLayer { name: l.name.clone(), material: l.material.clone(), volume: 0.0 });
                out.len() - 1
            });
            idx.push(i);
        }
        at.push(idx);
    }
    (out, at)
}

/// The volume computed now, with its warnings; refused where its corridor does not build.
pub(crate) fn compute(doc: &Document, def: &VolumeDef) -> Result<(VolumeReport, Vec<CommandWarning>), Stop> {
    compute_on(doc, def).map(|(r, w, _)| (r, w))
}

/// The volume computed now with its warnings and the alignment its corridor lies on (a mass haul diagram's km).
pub(crate) fn compute_on(
    doc: &Document,
    def: &VolumeDef,
) -> Result<(VolumeReport, Vec<CommandWarning>, crate::road_alignments::Built), Stop> {
    let cdef = doc.road_corridors().iter().find(|c| c.id == def.corridor).ok_or_else(|| {
        fail(codes::UNKNOWN_CORRIDOR, format!("“{}” kimlikli koridor projede yok.", def.corridor), "corridor")
    })?;
    let made = corridors::make(doc, cdef, false)
        .map_err(|stop| fail(codes::INVALID_VOLUME, format!("“{}” koridoru kurulamıyor: {}", cdef.name, corridors::words(stop)), "corridor"))?;
    let mut warnings: Vec<CommandWarning> =
        made.warnings.iter().filter(|w| w.code == codes::CORRIDOR_STALE).cloned().collect();
    let mut raws: Vec<Raw> = Vec::new();
    let (mut cut_area, mut fill_area, mut uncovered) = (0.0, 0.0, 0.0);
    let bench = def.bench.as_ref().map(|b| tpl::Bench { slope: b.slope, height: b.height });
    // The rule (docs/adr/0930 §2): without one, the road against the ground, stripped where it strips (as before).
    let formation = def.rule.is_some_and(|r| r.formation());
    let topsoil = def.rule.map_or(!def.strip.is_empty(), |r| r.topsoil());
    let pavements: Vec<Option<&kentos_commands::PavementDef>> = cdef
        .regions
        .iter()
        .map(|r| doc.road_templates().iter().find(|t| t.id == r.template).and_then(|t| t.pavement.as_ref()))
        .collect();
    if let Some(rule) = def.rule.filter(|r| r.formation()) {
        for (r, p) in cdef.regions.iter().zip(&pavements) {
            if p.is_none() {
                let name = doc.road_templates().iter().find(|t| t.id == r.template).map_or(r.template.as_str(), |t| t.name.as_str());
                return Err(fail(
                    codes::INVALID_VOLUME,
                    format!("{} kuralı formasyonu ister: “{name}” tip kesitinde üstyapı yok. Tip kesite üstyapı tabakalarını yazın.", rule.words()),
                    "rule",
                ));
            }
        }
    }
    let (mut layer_list, layer_at) = layers_of(&pavements);
    let paved = !layer_list.is_empty();
    let curved = def.curve && def.method == VolumeMethod::EndArea;
    let worked = !def.strip.is_empty() || bench.is_some() || paved || def.rule.is_some() || curved;
    let mut past = 0u32;
    let unbuilt: std::cell::RefCell<Vec<f64>> = std::cell::RefCell::new(Vec::new());
    match def.method {
        // With stripping, benching, a pavement or a rule, each section's earthwork as the rule reads it.
        VolumeMethod::EndArea | VolumeMethod::Prismoidal if worked => {
            for (ri, region) in made.regions.iter().enumerate() {
                let pavement = pavements[ri].map(core_pavement);
                let work = |s: f64| -> Option<Work> {
                    let built = made.template_at(ri, s);
                    let (cross, terrain) = made.sampler.sample(&built, s)?;
                    let sec = cross.section?;
                    let depth = depth_at(def, s);
                    let used = if topsoil { depth } else { 0.0 };
                    let e = tpl::earthwork(&sec, &terrain, used, bench);
                    let strip = if used == depth { e.strip } else { tpl::earthwork(&sec, &terrain, depth, None).strip };
                    let (qc, qf) = if curved { tpl::moments(&sec, &terrain, used) } else { (0.0, 0.0) };
                    let centroid = (if e.cut > 0.0 { qc / e.cut } else { 0.0 }, if e.fill > 0.0 { qf / e.fill } else { 0.0 });
                    let curvature = if curved { made.sampler.alignment.core.at(s).map_or(0.0, |p| p.curvature) } else { 0.0 };
                    let reach = (
                        sec.left.points.last().map_or(0.0, |p| p[0]),
                        sec.right.points.last().map_or(0.0, |p| p[0]),
                    );
                    let mut w = Work {
                        cut: e.cut,
                        fill: e.fill,
                        strip,
                        bench: e.bench,
                        layers: vec![None; layer_list.len()],
                        centroid,
                        curvature,
                        reach,
                    };
                    if let Some(p) = &pavement {
                        let lowered: Vec<_> =
                            terrain.iter().map(|g| kentos_road::section::GroundPoint { z: g.z.map(|z| z - used), ..*g }).collect();
                        match pav::build(&made.sampler.template_at(&built, s), &sec, p, &lowered) {
                            Ok(built) => {
                                for (k, a) in built.layers.iter().enumerate() {
                                    w.layers[layer_at[ri][k]] = Some(*a);
                                }
                                if formation {
                                    w.cut += built.below;
                                    w.fill -= built.above;
                                }
                            }
                            Err(_) => {
                                unbuilt.borrow_mut().push(s);
                                if formation {
                                    return None;
                                }
                            }
                        }
                    }
                    Some(w)
                };
                for w in region.crosses.windows(2) {
                    let (s0, s1) = (w[0].line.station, w[1].line.station);
                    let l = s1 - s0;
                    let ends = match (&w[0].section, &w[1].section) {
                        (Some(_), Some(_)) => work(s0).zip(work(s1)),
                        _ => None,
                    };
                    let mut plain = None;
                    let both: Option<Work> = ends.and_then(|(a, b)| match def.method {
                        VolumeMethod::EndArea => {
                            let avg = |x: f64, y: f64| l * (x + y) / 2.0;
                            // On a curve the effective areas (docs/adr/0934); a section past the centre keeps the
                            // stretch plain, and says so.
                            let (cut, fill) = match (curved, a.curved(), b.curved()) {
                                (true, Some(x), Some(y)) => {
                                    plain = Some((avg(a.cut, b.cut), avg(a.fill, b.fill)));
                                    (avg(x.0, y.0), avg(x.1, y.1))
                                }
                                (true, _, _) => {
                                    past += 1;
                                    (avg(a.cut, b.cut), avg(a.fill, b.fill))
                                }
                                _ => (avg(a.cut, b.cut), avg(a.fill, b.fill)),
                            };
                            Some(Work {
                                cut,
                                fill,
                                strip: avg(a.strip, b.strip),
                                bench: avg(a.bench, b.bench),
                                layers: a.layers.iter().zip(&b.layers).map(|(x, y)| x.zip(*y).map(|(x, y)| avg(x, y))).collect(),
                                centroid: (0.0, 0.0),
                                curvature: 0.0,
                                reach: (0.0, 0.0),
                            })
                        }
                        _ => work((s0 + s1) / 2.0).map(|m| {
                            let p = |x: f64, y: f64, z: f64| prismoidal(x, y, z, l);
                            Work {
                                cut: p(a.cut, m.cut, b.cut),
                                fill: p(a.fill, m.fill, b.fill),
                                strip: p(a.strip, m.strip, b.strip),
                                bench: p(a.bench, m.bench, b.bench),
                                centroid: (0.0, 0.0),
                                curvature: 0.0,
                                reach: (0.0, 0.0),
                                layers: (0..a.layers.len())
                                    .map(|k| match (a.layers[k], m.layers[k], b.layers[k]) {
                                        (Some(x), Some(y), Some(z)) => Some(p(x, y, z)),
                                        _ => None,
                                    })
                                    .collect(),
                            }
                        }),
                    });
                    raws.push(Raw {
                        from: s0,
                        to: s1,
                        volumes: both.as_ref().map(|v| (v.cut, v.fill)),
                        plain,
                        extra: both.as_ref().map(|v| (v.strip, v.bench)),
                        layers: if paved { both.map_or_else(|| vec![None; layer_list.len()], |v| v.layers) } else { Vec::new() },
                    });
                }
            }
        }
        VolumeMethod::EndArea | VolumeMethod::Prismoidal => {
            for (ri, region) in made.regions.iter().enumerate() {
                for w in region.crosses.windows(2) {
                    let (s0, s1) = (w[0].line.station, w[1].line.station);
                    let l = s1 - s0;
                    let volumes = match (&w[0].section, &w[1].section) {
                        (Some(a), Some(b)) => match def.method {
                            VolumeMethod::EndArea => Some((l * (a.cut + b.cut) / 2.0, l * (a.fill + b.fill) / 2.0)),
                            _ => made
                                .sampler
                                .cross(&made.template_at(ri, (s0 + s1) / 2.0), (s0 + s1) / 2.0)
                                .and_then(|c| c.section)
                                .map(|m| (prismoidal(a.cut, m.cut, b.cut, l), prismoidal(a.fill, m.fill, b.fill, l))),
                        },
                        _ => None,
                    };
                    raws.push(Raw { from: s0, to: s1, volumes, plain: None, extra: None, layers: Vec::new() });
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
                    raws.push(Raw { from: s0, to: s1, volumes, plain: None, extra: None, layers: Vec::new() });
                }
            }
        }
    }
    let text = |s: f64| km_text(made.sampler.alignment.km(s), 3);
    let (mut cut, mut fill, mut fcut, mut ffill) = (0.0, 0.0, 0.0, 0.0);
    let (mut strip, mut benched) = (0.0, 0.0);
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
        if let Some((a, b)) = r.extra {
            strip += a;
            benched += b;
        }
        for (l, v) in layer_list.iter_mut().zip(&r.layers) {
            l.volume += v.unwrap_or(0.0);
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
            strip: (!def.strip.is_empty()).then_some(r.extra.map(|e| e.0)).flatten(),
            bench: def.bench.as_ref().and(r.extra.map(|e| e.1)),
            layers: r.layers.clone(),
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
    let mut unbuilt = unbuilt.into_inner();
    unbuilt.sort_by(f64::total_cmp);
    unbuilt.dedup();
    if !unbuilt.is_empty() {
        let words: Vec<String> = unbuilt.iter().take(8).map(|s| text(*s)).collect();
        let more = if unbuilt.len() > 8 { format!(" ve {} kesit daha", unbuilt.len() - 8) } else { String::new() };
        warnings.push(warning(
            codes::PAVEMENT_UNBUILT,
            format!(
                "“{}” kübajında şu kesitlerde üstyapı kurulamadı (kenar şevi son bileşenden yatık ya da bileşensiz yan): {}{more}.",
                def.name,
                words.join(", ")
            ),
        ));
    }
    if past > 0 {
        warnings.push(warning(
            codes::VOLUME_CURVE_PAST_CENTRE,
            format!(
                "“{}” kübajında {past} aralığın bir kesiti kurbun merkezini geçiyor; o aralıklar kurp düzeltmesiz sayıldı. Kesit genişliğini kurbun yarıçapından küçük tutun.",
                def.name
            ),
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
        strip: (!def.strip.is_empty()).then_some(strip),
        bench: def.bench.as_ref().map(|_| benched),
        gaps: joined,
        rule: def.rule,
        layers: layer_list,
        approval: None,
        problem: None,
        curve: curved.then(|| {
            let delta = |k: usize| {
                raws.iter()
                    .filter_map(|r| r.volumes.zip(r.plain))
                    .map(|(v, p)| if k == 0 { v.0 - p.0 } else { v.1 - p.1 })
                    .sum::<f64>()
            };
            kentos_commands::VolumeCurve { cut: delta(0), fill: delta(1), plain: past }
        }),
    };
    let mut report = report;
    if let Some(a) = &def.approved {
        let approval = compare(a, &report);
        if approval.changed {
            warnings.push(warning(
                codes::VOLUME_CHANGED,
                format!(
                    "“{}” kübajı {} günü onaylanan hâlinden farklı: {} aralık değişti; fark raporda.",
                    def.name,
                    a.at,
                    approval.rows.len()
                ),
            ));
        }
        report.approval = Some(approval);
    }
    Ok((report, warnings, made.sampler.alignment))
}

/// Within this (m³) two figures of a volume are the same; within this (m) two stations.
const SAME_VOLUME: f64 = 1e-6;

/// A volume's figures: its four totals and its stretches [from, to, cut, fill].
pub(crate) struct Figures<'a> {
    pub(crate) totals: [f64; 4],
    pub(crate) rows: std::borrow::Cow<'a, [[f64; 4]]>,
}

impl<'a> Figures<'a> {
    pub(crate) fn of_approval(a: &'a kentos_commands::VolumeApprovalDef) -> Self {
        Figures { totals: [a.cut, a.fill, a.factored_cut, a.factored_fill], rows: std::borrow::Cow::Borrowed(&a.rows) }
    }

    pub(crate) fn of_report(r: &VolumeReport) -> Figures<'static> {
        Figures {
            totals: [r.cut, r.fill, r.factored_cut, r.factored_fill],
            rows: std::borrow::Cow::Owned(r.rows.iter().filter_map(|x| Some([x.from, x.to, x.cut?, x.fill?])).collect()),
        }
    }
}

/// `after` against `before` (docs/adr/0950): the totals' changes and the stretches that changed (beyond 1e-6 m³), are
/// new or gone; stations the same within 1e-6 m.
pub(crate) fn change(before: &Figures<'_>, after: &Figures<'_>) -> kentos_commands::VolumeChange {
    use kentos_commands::VolumeRowChange;
    let today = &after.rows;
    let same = |x: f64, y: f64| (x - y).abs() <= SAME_VOLUME;
    let mut rows = Vec::new();
    let mut used = vec![false; today.len()];
    for p in before.rows.iter() {
        match today.iter().position(|t| same(t[0], p[0]) && same(t[1], p[1])) {
            Some(i) => {
                used[i] = true;
                let t = today[i];
                if !same(t[2], p[2]) || !same(t[3], p[3]) {
                    rows.push(VolumeRowChange { from: p[0], to: p[1], approved_cut: Some(p[2]), approved_fill: Some(p[3]), cut: Some(t[2]), fill: Some(t[3]) });
                }
            }
            None => rows.push(VolumeRowChange { from: p[0], to: p[1], approved_cut: Some(p[2]), approved_fill: Some(p[3]), cut: None, fill: None }),
        }
    }
    for (t, _) in today.iter().zip(&used).filter(|(_, u)| !**u) {
        rows.push(VolumeRowChange { from: t[0], to: t[1], approved_cut: None, approved_fill: None, cut: Some(t[2]), fill: Some(t[3]) });
    }
    rows.sort_by(|x, y| x.from.total_cmp(&y.from).then(x.to.total_cmp(&y.to)));
    let c: Vec<f64> = (0..4).map(|k| after.totals[k] - before.totals[k]).collect();
    kentos_commands::VolumeChange {
        changed: !rows.is_empty() || c.iter().any(|x| x.abs() > SAME_VOLUME),
        cut_change: c[0],
        fill_change: c[1],
        factored_cut_change: c[2],
        factored_fill_change: c[3],
        rows,
    }
}

/// The approved figures against today's: the totals' changes and the stretches that changed, are new or gone.
pub(crate) fn compare(a: &kentos_commands::VolumeApprovalDef, now: &VolumeReport) -> kentos_commands::VolumeApprovalReport {
    let c = change(&Figures::of_approval(a), &Figures::of_report(now));
    kentos_commands::VolumeApprovalReport {
        approved: a.clone(),
        changed: c.changed,
        cut_change: c.cut_change,
        fill_change: c.fill_change,
        factored_cut_change: c.factored_cut_change,
        factored_fill_change: c.factored_fill_change,
        rows: c.rows,
    }
}

/// The figures to approve: the volume as computed now, numbered after its approvals so far (docs/adr/0950).
pub(crate) fn approval_of(def: &VolumeDef, now: &VolumeReport, at: String, note: Option<String>) -> kentos_commands::VolumeApprovalDef {
    let last = def.history.iter().chain(&def.approved).map(|a| a.number).max().unwrap_or(0);
    // An approval from before the numbers counts as one: the next is its successor, not the first.
    let before = def.history.len() + usize::from(def.approved.is_some());
    kentos_commands::VolumeApprovalDef {
        number: last.max(before as u32) + 1,
        ended: None,
        revision: def.revision,
        at,
        note,
        cut: now.cut,
        fill: now.fill,
        factored_cut: now.factored_cut,
        factored_fill: now.factored_fill,
        strip: now.strip,
        bench: now.bench,
        rows: now.rows.iter().filter_map(|r| Some([r.from, r.to, r.cut?, r.fill?])).collect(),
    }
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
        strip: None,
        bench: None,
        gaps: Vec::new(),
        rule: def.rule,
        layers: Vec::new(),
        approval: None,
        problem: Some(why),
        curve: None,
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

fn decimal(v: f64) -> String {
    format!("{v:.3}")
}

/// The volume sheet as a workbook's sheet (docs/adr/0906 §5, docs/adr/0223's writer): a row between each two
/// consecutive sections (its km, length, cut and fill, the topsoil and the benches where the volume has them, the
/// factors, the factored cut and fill, the running totals and the mass ordinate; empty cells where a stretch has no
/// volume or no factor), then the totals.
pub fn sheet(report: &VolumeReport) -> kentos_commands::WorkbookSheet {
    use kentos_commands::{CellKind, WorkbookColumn};
    let (strips, benches) = (report.strip.is_some(), report.bench.is_some());
    let column = |name: &str, kind: CellKind, decimals: Option<u32>| WorkbookColumn { name: name.into(), kind, decimals };
    let n = |name: &str| column(name, CellKind::Number, Some(3));
    let mut columns = vec![
        column("Başlangıç km", CellKind::Text, None),
        column("Bitiş km", CellKind::Text, None),
        n("Uzunluk (m)"),
        n("Kazı (m³)"),
        n("Dolgu (m³)"),
    ];
    if strips {
        columns.push(n("Bitkisel toprak (m³)"));
    }
    if benches {
        columns.push(n("Diş kazısı (m³)"));
    }
    columns.extend([
        n("Kabarma"),
        n("Sıkışma"),
        n("Düzeltilmiş kazı (m³)"),
        n("Düzeltilmiş dolgu (m³)"),
        n("Birikimli kazı (m³)"),
        n("Birikimli dolgu (m³)"),
        n("Kütle (m³)"),
    ]);
    let opt = |v: Option<f64>| v.map(decimal).unwrap_or_default();
    let mut rows: Vec<Vec<String>> = report
        .rows
        .iter()
        .map(|r| {
            let mut row = vec![r.from_text.clone(), r.to_text.clone(), decimal(r.to - r.from), opt(r.cut), opt(r.fill)];
            if strips {
                row.push(opt(r.strip));
            }
            if benches {
                row.push(opt(r.bench));
            }
            row.extend([
                opt(r.cut_factor),
                opt(r.fill_factor),
                opt(r.cut.map(|c| c * r.cut_factor.unwrap_or(1.0))),
                opt(r.fill.map(|f| f * r.fill_factor.unwrap_or(1.0))),
                decimal(r.total_cut),
                decimal(r.total_fill),
                decimal(r.mass),
            ]);
            row
        })
        .collect();
    let length: f64 = report.rows.iter().filter(|r| r.cut.is_some()).map(|r| r.to - r.from).sum();
    let mut total = vec!["Toplam".into(), String::new(), decimal(length), decimal(report.cut), decimal(report.fill)];
    if let Some(v) = report.strip {
        total.push(decimal(v));
    }
    if let Some(v) = report.bench {
        total.push(decimal(v));
    }
    total.extend([
        String::new(),
        String::new(),
        decimal(report.factored_cut),
        decimal(report.factored_fill),
        decimal(report.factored_cut),
        decimal(report.factored_fill),
        decimal(report.factored_cut - report.factored_fill),
    ]);
    rows.push(total);
    let name: String = report.name.chars().filter(|c| !"[]:*?/\\".contains(*c)).take(31).collect();
    kentos_commands::WorkbookSheet { name: if name.is_empty() { "Kübaj".into() } else { name }, columns, rows }
}
