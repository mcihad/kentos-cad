//! A road project's corridors (plan/road.md Y7, docs/adr/0905): found by id or name, built now from their sources (the
//! alignment, the profile's elevations, the surface, the superelevation schedule, the regions' templates) by
//! `kentos-road`'s corridor, and reported: the stations, the stretches not solved, the feature lines, the top surface.
//! Nothing built is kept: every read builds again, so a changed source shows at once (ROAD-MOD-06).

use kentos_commands::{
    CommandWarning, CorridorDef, CorridorFeature, CorridorListed, CorridorReport, CorridorStation, CorridorStretch,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::corridor::{
    self as core, CorridorError, Cross, Frequency, RegionCrosses, Station, Unsolved, Why,
};
use kentos_road::section::template as tpl;
use kentos_road::section::{ground, sample_line};

use crate::checks::Stop;
use crate::codes;
use crate::road_alignments::{self as alignments, Built};
use crate::road_sections::{core_template, superelevated};
use crate::road_terrains::{self as terrains, fail};

/// Only a road project has corridors.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(codes::NOT_A_ROAD_PROJECT, "Koridor yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""))
    }
}

/// The corridor of `name`: by its id, else by its name (case aside).
pub(crate) fn find<'a>(list: &'a [CorridorDef], name: &str, path: &str) -> Result<(usize, &'a CorridorDef), Stop> {
    list.iter()
        .position(|c| c.id == name)
        .or_else(|| list.iter().position(|c| c.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_CORRIDOR, format!("“{name}” kimlikli ya da adlı koridor projede yok."), path))
}

pub(crate) fn stale(doc: &Document, def: &CorridorDef) -> bool {
    doc.road_alignments().iter().find(|a| a.id == def.alignment).is_some_and(|a| a.revision != def.alignment_revision)
}

pub(crate) fn listed(doc: &Document, list: &[CorridorDef]) -> Vec<CorridorListed> {
    list.iter()
        .map(|c| CorridorListed {
            id: c.id.clone(),
            name: c.name.clone(),
            alignment: c.alignment.clone(),
            revision: c.revision,
            stale: stale(doc, c),
        })
        .collect()
}

fn bad(path: &str, words: String) -> Stop {
    fail(codes::INVALID_ROAD_CORRIDOR, words, path)
}

fn why_word(w: Why) -> &'static str {
    match w {
        Why::Region => "region",
        Why::Geometry => "geometry",
        Why::Critical => "critical",
        Why::Profile => "profile",
        Why::User => "user",
        Why::Curve => "curve",
        Why::Interval => "interval",
    }
}

/// A corridor built now: its report, the core's result and the warnings it gives.
pub(crate) struct Made {
    pub report: CorridorReport,
    pub built: core::Built,
    pub warnings: Vec<CommandWarning>,
    /// Each region's crosses: the sample lines and the sections built there.
    pub regions: Vec<RegionCrosses>,
    /// What builds a section at any station of the corridor (a volume's middle sections).
    pub sampler: Sampler,
}

/// The corridor's sources built once: its alignment, its profile's elevations, its surface, its superelevation
/// schedule and its swath; a section at any station of it.
pub(crate) struct Sampler {
    pub alignment: Built,
    vertical: kentos_road::profile::vertical::Vertical,
    pub surface: Option<kentos_road::surface::Surface>,
    schedule: Option<(kentos_road::superelevation::Schedule, kentos_road::superelevation::Crossfall)>,
    left: f64,
    right: f64,
}

impl Sampler {
    /// The design elevation at station `s` (the profile's).
    pub fn elevation(&self, s: f64) -> Option<f64> {
        self.vertical.at(s).map(|r| r.0)
    }

    /// The cross at station `s` with `template`: its sample line and the section, none where the line does not lie on
    /// the alignment.
    pub fn cross(&self, template: &tpl::Template, s: f64) -> Option<Cross> {
        self.sample(template, s).map(|(cross, _)| cross)
    }

    /// The template as built at station `s`: with the superelevation schedule's cross slopes there, if any.
    pub fn template_at(&self, template: &tpl::Template, s: f64) -> tpl::Template {
        match &self.schedule {
            Some((sch, normal)) => superelevated(template, sch, *normal, s),
            None => template.clone(),
        }
    }

    /// The cross at station `s` with the terrain along its line (empty where there is no surface).
    pub fn sample(&self, template: &tpl::Template, s: f64) -> Option<(Cross, Vec<kentos_road::section::GroundPoint>)> {
        let line = sample_line(&self.alignment.core, s, self.left, self.right)?;
        let mut terrain = Vec::new();
        let section = match (&self.surface, self.vertical.at(s)) {
            (Some(surf), Some((z, _, _))) => {
                terrain = ground(surf, &line);
                let (tt, z) = match &self.schedule {
                    // About an edge the axis rises over the profile grade (docs/adr/0931 §1).
                    Some((sch, normal)) => {
                        let tt = superelevated(template, sch, *normal, s);
                        let lift = sch.lift(s, *normal, crate::road_sections::lane_widths(&tt));
                        (tt, z + lift)
                    }
                    None => (template.clone(), z),
                };
                tpl::build(&tt, z, &terrain).ok()
            }
            _ => None,
        };
        Some((Cross { line, section }, terrain))
    }
}

fn warning(code: &str, message: String) -> CommandWarning {
    CommandWarning { code: code.into(), message, path: Some("corridor".into()) }
}

/// The alignment a corridor lies on, built.
pub(crate) fn alignment_of(doc: &Document, def: &CorridorDef) -> Result<Built, Stop> {
    let list = alignments::alignments(doc);
    let a = list.iter().find(|a| a.id == def.alignment).ok_or_else(|| {
        fail(codes::UNKNOWN_ALIGNMENT, format!("“{}” kimlikli aks projede yok.", def.alignment), "alignment")
    })?;
    alignments::build(list, a, "")
}

/// The corridor built now with `templates` (the project's, or the ones a window is typing): refused where its
/// regions, its stations or its profile make no corridor.
pub(crate) fn make_with(
    doc: &Document,
    def: &CorridorDef,
    templates: &[kentos_commands::TemplateDef],
    points: bool,
) -> Result<Made, Stop> {
    let built = alignment_of(doc, def)?;
    let length = built.core.length;
    let ranges: Vec<[f64; 2]> = def.regions.iter().map(|r| [r.from, r.to]).collect();
    let gaps = core::check_regions(&ranges, length).map_err(|e| match e {
        CorridorError::EmptyRegion { index } => {
            bad(&format!("regions[{index}]"), format!("{}. bölgenin sonu başından en az 1 mm sonra olmalı.", index + 1))
        }
        CorridorError::Outside { index } => bad(
            &format!("regions[{index}]"),
            format!("{}. bölge aksın dışına taşıyor: aks 0 ile {length:.3} m arası.", index + 1),
        ),
        CorridorError::Overlap { first, second } => bad(
            &format!("regions[{second}]"),
            format!("{}. ve {}. bölge kesişiyor; bölgeler en çok uç uca gelir.", first + 1, second + 1),
        ),
        _ => bad("regions", "Bölgeler geçerli değil.".into()),
    })?;
    for (i, s) in def.stations.iter().enumerate() {
        if *s < -1e-9 || *s > length + 1e-9 {
            return Err(bad(&format!("stations[{i}]"), format!("{s} m aksın üstünde değil: aks 0 ile {length:.3} m arası.")));
        }
    }
    let profile = doc
        .road_profiles()
        .iter()
        .find(|p| p.id == def.profile)
        .ok_or_else(|| fail(codes::UNKNOWN_ROAD_PROFILE, format!("“{}” kimlikli boykesit projede yok.", def.profile), "profile"))?;
    let vertical = kentos_road::profile::vertical::build(&crate::road_profiles::pvis_of(profile))
        .map_err(|_| bad("profile", format!("“{}” boykesiti kurulamıyor; koridor ondan kot alamaz.", profile.name)))?;
    let mut warnings = Vec::new();
    // The schedule, its normal slopes and critical stations.
    let schedule = match def.superelevation.as_deref() {
        None => None,
        Some(id) => {
            let made = doc.road_superelevations().iter().find(|d| d.id == id).and_then(|sd| {
                let b = crate::road_superelevations::alignment_of(doc, sd).ok()?;
                let (s, crit) = crate::road_superelevations::schedule(sd, &b).ok()?;
                Some((s, crate::road_superelevations::normal(sd), crit))
            });
            if made.is_none() {
                warnings.push(warning(
                    codes::CORRIDOR_SUPERELEVATION_GONE,
                    format!("“{}” koridorunun dever çizelgesi kurulamıyor; şeritler tip kesitin eğimiyle verildi.", def.name),
                ));
            }
            made
        }
    };
    let mut events: Vec<Station> = def.stations.iter().map(|s| Station { station: *s, why: Why::User }).collect();
    if def.profile_points {
        events.extend(profile.pvis.iter().map(|p| Station { station: p.station, why: Why::Profile }));
        for c in &vertical.curves {
            events.push(Station { station: c.start[0], why: Why::Profile });
            events.push(Station { station: c.end[0], why: Why::Profile });
            if let Some(x) = c.extreme {
                events.push(Station { station: x[0], why: Why::Profile });
            }
        }
    }
    if def.critical
        && let Some((_, _, crit)) = &schedule
    {
        events.extend(crit.iter().map(|c| Station { station: c.station, why: Why::Critical }));
    }
    let surface = match terrains::surfaces(doc).iter().find(|s| s.id == def.surface) {
        None => None,
        Some(sd) => terrains::make(doc, sd).surface.ok(),
    };
    if surface.is_none() {
        warnings.push(warning(
            codes::CORRIDOR_SURFACE_GONE,
            format!("“{}” koridorunun yüzeyi ({}) projede yok ya da kurulamıyor; hiçbir kesit çözülmedi.", def.name, def.surface),
        ));
    }
    let sampler = Sampler {
        alignment: built,
        vertical,
        surface,
        schedule: schedule.map(|(s, n, _)| (s, n)),
        left: def.left,
        right: def.right,
    };
    let built = &sampler.alignment;
    let f = Frequency { tangent: def.tangent, curve: def.curve, sag: def.sag };
    let mut stations_out = Vec::new();
    let mut regions = Vec::new();
    for (ri, r) in def.regions.iter().enumerate() {
        let t = templates.iter().find(|t| t.id == r.template).ok_or_else(|| {
            fail(codes::UNKNOWN_ROAD_TEMPLATE, format!("“{}” kimlikli tip kesit projede yok.", r.template), &format!("regions[{ri}].template"))
        })?;
        let template = core_template(t);
        let list = core::stations(&built.core, [r.from, r.to], &f, &events).map_err(|e| match e {
            CorridorError::TooMany => bad(
                "tangent",
                format!("Bu aralıklarla {}'den çok km çıkar; tangent ya da curve'ü büyütün.", core::MAX_STATIONS),
            ),
            _ => bad("tangent", "Aralıklar ve sapma toleransı 0'dan büyük olmalı.".into()),
        })?;
        let mut crosses = Vec::with_capacity(list.len());
        for st in &list {
            let s = st.station;
            stations_out.push(CorridorStation { station: s, km_text: km_text(built.km(s), 3), why: why_word(st.why).into(), region: ri as u32 });
            if let Some(cross) = sampler.cross(&template, s) {
                crosses.push(cross);
            }
        }
        regions.push(RegionCrosses { template, crosses });
    }
    let made = core::assemble(&regions);
    let text = |s: f64| km_text(built.km(s), 3);
    let unsolved: Vec<CorridorStretch> = made
        .unsolved
        .iter()
        .map(|u| CorridorStretch {
            region: u.region as u32,
            side: if u.right { "right" } else { "left" }.into(),
            why: match u.why {
                Unsolved::NoSection => "noSection",
                Unsolved::Open => "open",
            }
            .into(),
            from: u.from,
            to: u.to,
            from_text: text(u.from),
            to_text: text(u.to),
        })
        .collect();
    let mut features: Vec<CorridorFeature> = made
        .features
        .iter()
        .map(|f| {
            let length = f.points.windows(2).map(|w| ((w[1][1] - w[0][1]).powi(2) + (w[1][2] - w[0][2]).powi(2) + (w[1][3] - w[0][3]).powi(2)).sqrt()).sum();
            CorridorFeature {
                code: f.code.clone(),
                region: f.region as u32,
                from: f.points[0][0],
                to: f.points[f.points.len() - 1][0],
                length,
                points: if points { f.points.clone() } else { Vec::new() },
            }
        })
        .collect();
    features.sort_by(|a, b| (a.region, &a.code).cmp(&(b.region, &b.code)).then(a.from.total_cmp(&b.from)));
    let area = made.triangles.iter().map(|t| 0.5 * ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1]))).sum();
    if stale(doc, def) {
        let now = doc.road_alignments().iter().find(|a| a.id == def.alignment).map_or(0, |a| a.revision);
        warnings.push(warning(
            codes::CORRIDOR_STALE,
            format!(
                "“{}” koridorunun aksı değişti (koridor {}. revizyona göre kurulu, aks şimdi {now}.); bölgeleri denetleyip aksa yeniden bağlayın.",
                def.name, def.alignment_revision
            ),
        ));
    }
    if !gaps.is_empty() {
        let words: Vec<String> = gaps.iter().map(|g| format!("{} – {}", text(g[0]), text(g[1]))).collect();
        warnings.push(warning(codes::CORRIDOR_GAP, format!("“{}” koridorunun bölgeleri arasında kesitsiz aralık var: {}.", def.name, words.join(", "))));
    }
    if !unsolved.is_empty() {
        let words: Vec<String> = unsolved
            .iter()
            .take(8)
            .map(|u| {
                let side = if u.side == "right" { "sağ" } else { "sol" };
                let why = if u.why == "open" { "şev buluşmuyor" } else { "kesit yok" };
                if u.from == u.to { format!("km {} {side} ({why})", u.from_text) } else { format!("km {} – {} {side} ({why})", u.from_text, u.to_text) }
            })
            .collect();
        let more = if unsolved.len() > 8 { format!(" ve {} aralık daha", unsolved.len() - 8) } else { String::new() };
        warnings.push(warning(
            codes::CORRIDOR_UNSOLVED,
            format!("“{}” koridorunun çözülmeyen aralıkları: {}{more}.", def.name, words.join(", ")),
        ));
    }
    if !made.folded.is_empty() {
        let words: Vec<String> = made.folded.iter().map(|f| format!("km {} – {}", text(f.1), text(f.2))).collect();
        warnings.push(warning(
            codes::CORRIDOR_FOLDED,
            format!("“{}” koridorunda ardışık kesitler planda kesişiyor: {}; orada üst yüzey kurulmadı.", def.name, words.join(", ")),
        ));
    }
    let report = CorridorReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        alignment: def.alignment.clone(),
        profile: def.profile.clone(),
        surface: def.surface.clone(),
        superelevation: def.superelevation.clone(),
        stale: stale(doc, def),
        stations: stations_out,
        gaps,
        unsolved,
        folded: made.folded.iter().map(|f| [f.1, f.2]).collect(),
        features,
        triangles: made.triangles.len() as u32,
        area,
        problem: None,
    };
    Ok(Made { report, built: made, warnings, regions, sampler })
}

/// The corridor built now with the project's templates.
pub(crate) fn make(doc: &Document, def: &CorridorDef, points: bool) -> Result<Made, Stop> {
    make_with(doc, def, doc.road_templates(), points)
}

/// A report for a corridor that does not build: its sources and why.
pub(crate) fn broken(doc: &Document, def: &CorridorDef, why: String) -> CorridorReport {
    CorridorReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        alignment: def.alignment.clone(),
        profile: def.profile.clone(),
        surface: def.surface.clone(),
        superelevation: def.superelevation.clone(),
        stale: stale(doc, def),
        stations: Vec::new(),
        gaps: Vec::new(),
        unsolved: Vec::new(),
        folded: Vec::new(),
        features: Vec::new(),
        triangles: 0,
        area: 0.0,
        problem: Some(why),
    }
}

pub(crate) fn words(stop: Stop) -> String {
    match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.message,
        Stop::Cancelled => String::new(),
    }
}

/// The corridor's report now, or why it does not build.
pub(crate) fn report(doc: &Document, def: &CorridorDef) -> CorridorReport {
    match make(doc, def, false) {
        Ok(m) => m.report,
        Err(stop) => broken(doc, def, words(stop)),
    }
}

/// A corridor built now: its report and the core's result, or why it does not build.
pub type MadeCorridor = Result<(CorridorReport, core::Built), String>;

/// A corridor built once for looking along it (plan/road.md Y13.4, docs/adr/0911 §5): its stations, and its section
/// at any station of a region with that region's template, as the section sets' rows are.
pub struct CorridorSections {
    made: Made,
    /// Each region's template's pavement (docs/adr/0930 §2), in the regions' order.
    pavements: Vec<Option<kentos_commands::PavementDef>>,
}

impl CorridorSections {
    /// The corridor `id` built now, or why it does not build.
    pub fn of(doc: &Document, id: &str) -> Result<Self, String> {
        let def = doc.road_corridors().iter().find(|c| c.id == id).ok_or_else(|| format!("({id}) koridoru projede yok."))?;
        let pavements = def
            .regions
            .iter()
            .map(|r| doc.road_templates().iter().find(|t| t.id == r.template).and_then(|t| t.pavement.clone()))
            .collect();
        make(doc, def, false).map(|made| Self { made, pavements }).map_err(words)
    }

    /// The stations the corridor is sampled at, in order.
    pub fn stations(&self) -> Vec<f64> {
        let mut out: Vec<f64> = self.made.regions.iter().flat_map(|r| r.crosses.iter().map(|c| c.line.station)).collect();
        out.sort_by(f64::total_cmp);
        out.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        out
    }

    /// The section at station `s`: the template of the region it is in (its sampled ends), none off every region or
    /// where the sample line does not lie on the alignment.
    pub fn row_at(&self, s: f64) -> Option<kentos_commands::SectionRow> {
        let ri = self.made.regions.iter().position(|r| {
            let (a, b) = (r.crosses.first().map(|c| c.line.station), r.crosses.last().map(|c| c.line.station));
            matches!((a, b), (Some(a), Some(b)) if s >= a - 1e-9 && s <= b + 1e-9)
        })?;
        let region = &self.made.regions[ri];
        let sampler = &self.made.sampler;
        let (cross, terrain) = sampler.sample(&region.template, s)?;
        let km = sampler.alignment.km(s);
        let paved = match (self.pavements.get(ri).and_then(Option::as_ref), &cross.section) {
            (Some(p), Some(sec)) => crate::road_sections::pavement_row(p, &sampler.template_at(&region.template, s), sec, &terrain),
            _ => None,
        };
        // The axis as the section has it: risen over the profile grade about an edge (docs/adr/0931 §1).
        let axis = cross.section.as_ref().map(|x| x.axis).or(sampler.elevation(s));
        Some(crate::road_sections::row_of(s, km, cross.line.ends, axis, &terrain, cross.section.as_ref(), paved))
    }
}

/// The road project's corridors built now, for the desktop's Koridor window and its plan (plan/road.md Y7).
pub fn made_corridors(doc: &Document) -> Vec<(CorridorDef, MadeCorridor)> {
    if road_project(doc).is_err() {
        return Vec::new();
    }
    doc.road_corridors()
        .iter()
        .map(|c| (c.clone(), make(doc, c, true).map(|m| (m.report, m.built)).map_err(words)))
        .collect()
}

/// A corridor (perhaps not written) built now with templates perhaps not written either, for the Koridor window: its
/// report with the feature lines' points, the core's result and its warnings, or why it does not build.
pub fn preview(
    doc: &Document,
    def: &CorridorDef,
    templates: &[kentos_commands::TemplateDef],
) -> Result<(CorridorReport, core::Built, Vec<CommandWarning>), String> {
    make_with(doc, def, templates, true).map(|m| (m.report, m.built, m.warnings)).map_err(words)
}
