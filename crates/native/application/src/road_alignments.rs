//! What KentOS Yol's alignment commands share (plan/road.md Y3, docs/adr/0901): an alignment built from its definition
//! by `kentos-road` (`kentos.alignment/1`), reported as the catalog says, its places with km, a parallel axis through
//! its source, and the alignments written back as one undo step. The commands are [`crate::road_alignments_create`],
//! [`crate::road_alignments_edit`] and [`crate::road_alignments_query`].

use kentos_commands::{
    AlignmentCurveRow, AlignmentDef, AlignmentElementRow, AlignmentJoinRow, AlignmentListed, AlignmentPlace,
    AlignmentReport, AlignmentStationing, AlignmentsWritten, CommandResult, RoadSettings, StationMapRow,
};
use kentos_domain::Document;
use kentos_geometry_core::crs::ground::{Grid, grid_factor};
use kentos_road::alignment::{
    self as core, Alignment, ChainError, Equation, Kind, PiInput, Region, Stationing, StationingError, km_text, offset,
};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::fail;

/// Only a road project has alignments.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(
            codes::NOT_A_ROAD_PROJECT,
            "Aks yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(),
            "",
        ))
    }
}

/// The alignment of an id, else of a name (case aside).
pub(crate) fn find<'a>(list: &'a [AlignmentDef], name: &str, path: &str) -> Result<(usize, &'a AlignmentDef), Stop> {
    list.iter()
        .position(|a| a.id == name)
        .or_else(|| list.iter().position(|a| a.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_ALIGNMENT, format!("“{name}” kimlikli ya da adlı aks projede yok."), path))
}

/// The alignments judged by the contract's rules; a refusal's path is the input's `field`'s for the one at `at`.
pub(crate) fn judged(list: &[AlignmentDef], at: usize, field: &str) -> Result<(), Stop> {
    let road = RoadSettings {
        alignments: list.to_vec(),
        ..RoadSettings::default()
    };
    if let Some((path, words)) = road.problem() {
        let prefix = format!("alignments[{at}].");
        let path = match path.strip_prefix(&prefix) {
            Some(rest) if field.is_empty() => rest.to_owned(),
            Some(rest) => format!("{field}.{rest}"),
            None => field.to_owned(),
        };
        return Err(fail(codes::INVALID_ALIGNMENT, format!("Aks kurallarına uymuyor: {words}."), &path));
    }
    Ok(())
}

/// An alignment as built: its core geometry (a parallel axis's source's), its km regions, and its offset.
pub struct Built {
    pub core: Alignment,
    pub regions: Vec<Region>,
    pub offset: f64,
}

fn chain_stop(e: ChainError, field: &str, pis: &[PiInput]) -> Stop {
    let at = |pi: &str| {
        let head = if field.is_empty() { "pis".to_owned() } else { format!("{field}.pis") };
        match pis.iter().position(|p| p.id == pi) {
            Some(i) if !pi.is_empty() => format!("{head}[{i}]"),
            _ => head,
        }
    };
    match e {
        ChainError::TooFewPoints => fail(codes::INVALID_ALIGNMENT, "Aksın en az iki noktası olur.".into(), &at("")),
        ChainError::DuplicateId { pi } => {
            fail(codes::INVALID_ALIGNMENT, format!("“{pi}” kimliği aksta iki kez var."), &at(&pi))
        }
        ChainError::NotFinite { pi } => {
            fail(codes::INVALID_ALIGNMENT, format!("{pi} noktasının sayıları sonlu değil."), &at(&pi))
        }
        ChainError::SamePlace { pi } => fail(
            codes::INVALID_ALIGNMENT,
            format!("{pi} noktası bir öncekiyle aynı yerde; PI'lar ayrı yerlerde olmalı."),
            &at(&pi),
        ),
        ChainError::BadCurve { pi } => fail(
            codes::INVALID_ALIGNMENT,
            format!(
                "{pi} noktasının kurpu olamaz: başlangıç ve bitişte kurp yok, klotoid yarıçapla verilir, değerler eksi olmaz."
            ),
            &at(&pi),
        ),
        ChainError::TurnTooSharp { pi } => fail(
            codes::INVALID_ALIGNMENT,
            format!("{pi} noktasında doğrultu geri dönüyor (180°); aks kendi üstüne dönemez."),
            &at(&pi),
        ),
        ChainError::SpiralsLong { pi, turn, delta } => fail(
            codes::ALIGNMENT_SPIRALS_LONG,
            format!(
                "{pi} noktasında klotoidler {:.4}° dönüyor, sapma {:.4}°; klotoidleri kısaltın ya da yarıçapı büyütün.",
                turn.to_degrees(),
                delta.to_degrees()
            ),
            &at(&pi),
        ),
        ChainError::CurvesOverlap { a, b, excess } => fail(
            codes::ALIGNMENT_CURVES_OVERLAP,
            format!(
                "{a}–{b} aliymanına kurplar sığmıyor: teğet boyları aliymandan {excess:.3} m uzun. Yarıçapı ya da klotoidleri küçültün, PI'ları uzaklaştırın."
            ),
            &at(&b),
        ),
    }
}

pub(crate) fn pis_of(def: &AlignmentDef) -> Vec<PiInput> {
    def.pis
        .iter()
        .map(|p| PiInput {
            id: p.id.clone(),
            x: p.x,
            y: p.y,
            radius: p.radius,
            spiral_in: p.spiral_in,
            spiral_out: p.spiral_out,
        })
        .collect()
}

pub fn stationing_of(st: Option<&AlignmentStationing>) -> Stationing {
    st.map_or_else(Stationing::default, |s| Stationing {
        start: s.start,
        equations: s
            .equations
            .iter()
            .map(|q| Equation {
                back: q.back,
                ahead: q.ahead,
                increasing: q.increasing,
            })
            .collect(),
    })
}

/// Builds `def` (from `list`, which holds its source); refusals' paths under `field`.
pub(crate) fn build(list: &[AlignmentDef], def: &AlignmentDef, field: &str) -> Result<Built, Stop> {
    let join = |rest: &str| if field.is_empty() { rest.to_owned() } else { format!("{field}.{rest}") };
    if let Some(o) = &def.offset_of {
        let source = list
            .iter()
            .find(|a| a.id == o.alignment)
            .ok_or_else(|| fail(codes::INVALID_ALIGNMENT, "Paralel aksın kaynağı projede yok.".into(), &join("offsetOf")))?;
        let mut built = build(list, source, &join("offsetOf"))?;
        if let Err(e) = offset::check(&built.core, o.offset) {
            return Err(cusp_stop(&def.name, e, &join("offsetOf.offset")));
        }
        built.offset = o.offset;
        return Ok(built);
    }
    let pis = pis_of(def);
    let core = core::build(&pis).map_err(|e| chain_stop(e, field, &pis))?;
    let regions = stationing_of(def.stationing.as_ref()).regions(core.length).map_err(|e| match e {
        StationingError::Outside { equation } => fail(
            codes::INVALID_STATIONING,
            format!(
                "{}. km eşitliğinin geri km'si aksın üstünde, bir önceki eşitlikten sonra değil (aks {:.3} m).",
                equation + 1,
                core.length
            ),
            &join(&format!("stationing.equations[{equation}]")),
        ),
        StationingError::NotFinite { .. } => {
            fail(codes::INVALID_STATIONING, "Km değerleri sonlu olmalı.".into(), &join("stationing"))
        }
    })?;
    Ok(Built {
        core,
        regions,
        offset: 0.0,
    })
}

pub(crate) fn cusp_stop(name: &str, e: offset::OffsetError, path: &str) -> Stop {
    let words = match e {
        offset::OffsetError::Cusp { from, to } => format!(
            "“{name}” paralel aksı km {:.3}–{:.3} arasında kıvrılıyor: orada kurpun yarıçapı ötelemeden küçük.",
            from, to
        ),
        offset::OffsetError::Crosses { a, b } => {
            format!("“{name}” paralel aksı km {a:.3} ile {b:.3} yakınında kendini kesiyor.")
        }
    };
    fail(codes::OFFSET_CUSP, format!("{words} Ötelemeyi küçültün ya da kurpları genişletin."), path)
}

impl Built {
    /// The km at a station.
    pub fn km(&self, station: f64) -> f64 {
        Stationing::km(&self.regions, station)
    }

    /// The place at a station (offset for a parallel axis).
    pub fn place(&self, station: f64) -> Option<AlignmentPlace> {
        let at = self.core.at(station)?;
        let d = self.offset;
        let (x, y) = (at.x + d * at.direction.sin(), at.y - d * at.direction.cos());
        let k = at.curvature;
        let km = self.km(station);
        Some(AlignmentPlace {
            station,
            km,
            km_text: km_text(km, 3),
            region: Stationing::region_of(&self.regions, station) as u32,
            x,
            y,
            direction: at.direction,
            curvature: if d == 0.0 { k } else { k / (1.0 + k * d) },
            element: self.core.elements[at.element].id.clone(),
        })
    }

    /// The length: the source's, or the parallel axis's own.
    pub fn length(&self) -> f64 {
        if self.offset == 0.0 { self.core.length } else { offset::length(&self.core, self.offset) }
    }
}

/// The road project's alignments as they build now, for the desktop's drawing and window (plan/road.md Y3): each
/// definition and its alignment, or why it does not build. None outside a road project.
pub fn made_alignments(doc: &Document) -> Vec<(AlignmentDef, Result<Built, String>)> {
    if road_project(doc).is_err() {
        return Vec::new();
    }
    let list = alignments(doc);
    list.iter()
        .map(|d| {
            let built = build(list, d, "").map_err(|stop| match stop {
                Stop::Failed(e) | Stop::Conflict(e) => e.message,
                Stop::Cancelled => String::new(),
            });
            (d.clone(), built)
        })
        .collect()
}

/// The report of a built alignment, as `road.alignments.query` gives it.
pub fn report_of(doc: &Document, def: &AlignmentDef, built: &Built) -> AlignmentReport {
    report(doc, def, built)
}

/// The most ticks `every` makes.
const MOST_TICKS: usize = 100_000;

/// The stations of every round `step` metres of km, region by region in the order they fall along the alignment (a
/// region's end belongs to the next region, the alignment's end to the last); refused (`invalid_places`, path `every`)
/// beyond [`MOST_TICKS`].
pub(crate) fn tick_stations(built: &Built, step: f64) -> Result<Vec<f64>, Stop> {
    let mut ticks = Vec::new();
    for r in &built.regions {
        let (lo, hi) = (r.at, r.until);
        let (ka, kb) = (r.ahead, r.ahead + r.dir * (hi - r.at));
        let (from, to) = (ka.min(kb), ka.max(kb));
        let mut k = (from / step).ceil() * step;
        let mut here = Vec::new();
        while k <= to + 1e-9 {
            let s = r.at + r.dir * (k - r.ahead);
            let last = std::ptr::eq(r, built.regions.last().unwrap_or(r));
            if s >= lo - 1e-9 && (s < hi - 1e-9 || (last && s <= hi + 1e-9)) {
                here.push(s.clamp(lo, hi));
            }
            if here.len() + ticks.len() > MOST_TICKS {
                return Err(fail(
                    codes::INVALID_PLACES,
                    format!("Bu aralıkla {MOST_TICKS}'den çok işaret çıkar; every'yi büyütün."),
                    "every",
                ));
            }
            k += step;
        }
        here.sort_by(f64::total_cmp);
        ticks.extend(here);
    }
    Ok(ticks)
}

/// The ground length (ROAD-NUM-05): each piece of at most 50 m (and 0.05 rad), its grid length over its chord's scale
/// and height factor (docs/adr/0171 §3); none without the project's mean height and a system with one scale at a point.
fn ground(doc: &Document, built: &Built) -> (Option<f64>, String) {
    let settings = doc.settings();
    let Some(height) = settings.ground_height() else {
        return (None, "Zemin boyu için projenin ortalama elipsoit yüksekliği yazılmalı (Proje ayarları › Ölçme).".into());
    };
    let Some(system) = kentos_project::systems::own(settings).and_then(|n| n.system) else {
        return (None, "Projenin koordinat sistemi yok: zemin boyu hesaplanamaz.".into());
    };
    let grid = Grid { system, height };
    let mut total = 0.0;
    for e in &built.core.elements {
        let turn = e.k0.abs().max(e.k1.abs()) * e.length;
        let n = ((e.length / 50.0).ceil().max((turn / 0.05).ceil()) as usize).max(1);
        for i in 0..n {
            let (s0, s1) = (e.length * i as f64 / n as f64, e.length * (i + 1) as f64 / n as f64);
            let (a, b) = (e.point(s0), e.point(s1));
            let place = |p: [f64; 2], s: f64| {
                let th = e.theta(s);
                kentos_geometry_core::Vec2 {
                    x: p[0] + built.offset * th.sin(),
                    y: p[1] - built.offset * th.cos(),
                }
            };
            let grid_piece = (s1 - s0) * (1.0 + 0.5 * (e.curvature(s0) + e.curvature(s1)) * built.offset);
            match grid_factor(&grid, place(a, s0), place(b, s1)) {
                Ok(f) => total += grid_piece / f.combined(),
                Err(why) => return (None, why),
            }
        }
    }
    (
        Some(total),
        format!(
            "Zemin boyu: her parçanın ızgara boyu, ölçek ve {height} m ortalama elipsoit yüksekliğinin çarpanına bölünerek."
        ),
    )
}

/// The report of `def` as built.
pub(crate) fn report(doc: &Document, def: &AlignmentDef, built: &Built) -> AlignmentReport {
    let (ground_length, ground) = ground(doc, built);
    let c = &built.core;
    AlignmentReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        offset_of: def.offset_of.clone(),
        length: built.length(),
        ground_length,
        ground,
        km_start: built.km(0.0),
        km_end: built.km(c.length),
        elements: c
            .elements
            .iter()
            .map(|e| AlignmentElementRow {
                id: e.id.clone(),
                kind: match e.kind {
                    Kind::Tangent => "tangent",
                    Kind::Arc => "arc",
                    Kind::Spiral => "spiral",
                }
                .into(),
                station: e.station,
                length: e.length,
                km: built.km(e.station),
                start: e.start,
                direction: e.theta0,
                k0: e.k0,
                k1: e.k1,
                radius: (e.kind == Kind::Arc).then(|| 1.0 / e.k0.abs()),
                a: e.a(),
            })
            .collect(),
        curves: c
            .curves
            .iter()
            .map(|r| AlignmentCurveRow {
                pi: r.pi.clone(),
                delta: r.delta,
                radius: r.radius,
                spiral_in: r.spiral_in,
                spiral_out: r.spiral_out,
                t_in: r.t_in,
                t_out: r.t_out,
                arc: r.arc,
                stations: r.stations,
                centre: r.centre,
            })
            .collect(),
        joins: c
            .joins
            .iter()
            .map(|j| AlignmentJoinRow {
                between: j.between.clone(),
                reverse: j.reverse,
            })
            .collect(),
        residual: c.residual.place,
        residual_direction: c.residual.direction,
        seams: c
            .seams()
            .into_iter()
            .map(|s| kentos_commands::AlignmentSeam {
                station: s.station,
                before: s.before,
                after: s.after,
                place: s.place,
                direction: s.direction,
                curvature: s.curvature,
            })
            .collect(),
        algorithm: core::ALGORITHM.into(),
    }
}

/// The project's alignments.
pub(crate) fn alignments(doc: &Document) -> &[AlignmentDef] {
    doc.road_alignments()
}

pub(crate) fn listed(list: &[AlignmentDef]) -> Vec<AlignmentListed> {
    list.iter()
        .map(|a| AlignmentListed {
            id: a.id.clone(),
            name: a.name.clone(),
            revision: a.revision,
            offset_of: a.offset_of.as_ref().map(|o| o.alignment.clone()),
        })
        .collect()
}

/// The parallel axes following `id`.
pub(crate) fn dependents(list: &[AlignmentDef], id: &str) -> Vec<String> {
    list.iter()
        .filter(|a| a.offset_of.as_ref().is_some_and(|o| o.alignment == id))
        .map(|a| a.id.clone())
        .collect()
}

/// Every parallel axis of `id` still holds on `list`; a refusal names the axis, at `path`.
pub(crate) fn dependents_hold(list: &[AlignmentDef], id: &str, path: &str) -> Result<(), Stop> {
    let Some(source) = list.iter().find(|a| a.id == id) else {
        return Ok(());
    };
    let built = build(list, source, "")?;
    for d in list {
        if let Some(o) = d.offset_of.as_ref().filter(|o| o.alignment == id)
            && let Err(e) = offset::check(&built.core, o.offset)
        {
            return Err(cusp_stop(&d.name, e, path));
        }
    }
    Ok(())
}

/// The mapping rows of `before` → `after`.
pub(crate) fn mapping_rows(before: Option<&Alignment>, after: Option<&Alignment>) -> Vec<StationMapRow> {
    let empty = Alignment {
        elements: vec![],
        length: 0.0,
        curves: vec![],
        joins: vec![],
        residual: core::Residual::default(),
    };
    core::mapping::mapping(before.unwrap_or(&empty), after.unwrap_or(&empty))
        .into_iter()
        .map(|r| StationMapRow {
            element: r.element,
            change: r.change.word().into(),
            before: r.before,
            after: r.after,
        })
        .collect()
}

/// Writes `next` as one undo step named `label`; the answer reports `target` (none: removed).
pub(crate) fn write(
    cx: &mut ExecutionContext<'_>,
    next: Vec<AlignmentDef>,
    label: &str,
    target: Option<&str>,
    mapping: Vec<StationMapRow>,
    warnings: Vec<kentos_commands::CommandWarning>,
) -> CommandResult<AlignmentsWritten> {
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_alignments(next.clone(), label) {
        return fail(codes::INVALID_ALIGNMENT, format!("Aks kurallarına uymuyor: {}.", r.0), "").into();
    }
    let alignment = match target.and_then(|id| next.iter().find(|a| a.id == id)) {
        Some(def) => match build(&next, def, "") {
            Ok(b) => Some(report(doc, def, &b)),
            Err(stop) => return stop.into(),
        },
        None => None,
    };
    CommandResult::Completed {
        output: AlignmentsWritten {
            dependents: target.map(|id| dependents(&next, id)).unwrap_or_default(),
            superelevations: Vec::new(),
            profiles: Vec::new(),
            alignment,
            mapping,
            alignments: listed(&next),
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}
