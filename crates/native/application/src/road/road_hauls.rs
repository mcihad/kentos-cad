//! A road project's mass haul diagrams (plan/road.md Y9, docs/adr/0907): found by id or name, read now from their
//! earthwork volume computed now (`road_volumes::compute_on`): the volume sheet's mass ordinate as a polyline (flat
//! across a stretch without volume), each borrow or waste site a step in it, read against the balance line by
//! `kentos-road`'s `haul::diagram`. Nothing read is kept: every read computes again (ROAD-MOD-06).

use kentos_commands::{CommandWarning, HaulDef, HaulListed, HaulLoop, HaulReport, HaulSiteKind, VolumeReport};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::haul::diagram;

use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::fail;
use crate::road_volumes as volumes;

/// Only a road project has mass haul diagrams.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(codes::NOT_A_ROAD_PROJECT, "Brükner yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""))
    }
}

/// The diagram of `name`: by its id, else by its name (case aside).
pub(crate) fn find<'a>(list: &'a [HaulDef], name: &str, path: &str) -> Result<(usize, &'a HaulDef), Stop> {
    list.iter()
        .position(|h| h.id == name)
        .or_else(|| list.iter().position(|h| h.name.to_lowercase() == name.to_lowercase()))
        .map(|i| (i, &list[i]))
        .ok_or_else(|| fail(codes::UNKNOWN_HAUL, format!("“{name}” kimlikli ya da adlı Brükner projede yok."), path))
}

pub(crate) fn listed(list: &[HaulDef]) -> Vec<HaulListed> {
    list.iter()
        .map(|h| HaulListed { id: h.id.clone(), name: h.name.clone(), volume: h.volume.clone(), revision: h.revision })
        .collect()
}

/// The volume sheet's mass ordinate as a polyline: from the first stretch's start at 0, each stretch's end at its
/// mass, flat across a gap between stretches.
pub(crate) fn ordinate(report: &VolumeReport) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::new();
    for r in &report.rows {
        match out.last().copied() {
            None => out.push([r.from, 0.0]),
            Some(last) if last[0] != r.from => out.push([r.from, last[1]]),
            _ => {}
        }
        out.push([r.to, r.mass]);
    }
    out
}

/// The ordinate with a step of `delta` at station `s`: after any point at `s`, everything beyond moved by `delta`.
fn step(points: &[[f64; 2]], s: f64, delta: f64) -> Vec<[f64; 2]> {
    let i = points.partition_point(|p| p[0] <= s);
    let v = if i > 0 && points[i - 1][0] == s {
        points[i - 1][1]
    } else {
        let (p, q) = (points[i - 1], points[i]);
        p[1] + (q[1] - p[1]) * (s - p[0]) / (q[0] - p[0])
    };
    let mut out: Vec<[f64; 2]> = points[..i].to_vec();
    if out.last().is_none_or(|l| l[0] != s) {
        out.push([s, v]);
    }
    out.push([s, v + delta]);
    out.extend(points[i..].iter().map(|p| [p[0], p[1] + delta]));
    out
}

/// How the haul is assigned: the mass diagram read layer by layer, not an optimisation (docs/adr/0907 §5).
const LAYERS: &str = "layers";

/// The diagram's cost by its prices: the overhaul moment in m³·km times its price, the waste and the borrow times
/// theirs, the total of those priced; the trips everything moved over the payload, rounded up.
pub(crate) fn cost(c: &kentos_commands::HaulCostDef, d: &kentos_road::haul::Diagram) -> kentos_commands::HaulCostReport {
    let overhaul = c.overhaul.map(|p| d.over_moment / 1000.0 * p);
    let waste = c.waste.map(|p| d.waste * p);
    let borrow = c.borrow.map(|p| d.borrow * p);
    kentos_commands::HaulCostReport {
        currency: c.currency.clone(),
        overhaul,
        waste,
        borrow,
        total: [overhaul, waste, borrow].into_iter().flatten().sum(),
        trips: c.payload.map(|p| ((d.volume + d.waste + d.borrow) / p - 1e-9).ceil().max(0.0)),
    }
}

/// The diagram read now with its warnings; refused where its volume does not compute or a site is off its stations.
pub(crate) fn read(doc: &Document, def: &HaulDef, points: bool) -> Result<(HaulReport, Vec<CommandWarning>), Stop> {
    let vdef = doc.road_volumes().iter().find(|v| v.id == def.volume).ok_or_else(|| {
        fail(codes::UNKNOWN_VOLUME, format!("“{}” kimlikli kübaj hesabı projede yok.", def.volume), "volume")
    })?;
    let (report, vwarnings, alignment) = volumes::compute_on(doc, vdef)
        .map_err(|stop| fail(codes::INVALID_HAUL, format!("“{}” kübajı hesaplanamıyor: {}", vdef.name, crate::road_corridors::words(stop)), "volume"))?;
    let mut warnings: Vec<CommandWarning> = vwarnings
        .into_iter()
        .filter(|w| w.code == codes::CORRIDOR_STALE || w.code == codes::VOLUME_GAP)
        .collect();
    let mut curve = ordinate(&report);
    if curve.len() < 2 {
        return Err(fail(codes::INVALID_HAUL, format!("“{}” kübajının hacimli aralığı yok; Brükner çizilemez.", vdef.name), "volume"));
    }
    let (first, last) = (curve[0][0], curve[curve.len() - 1][0]);
    // A site off the road stands where its access meets the alignment; one the network does not reach is left out
    // (docs/adr/0955).
    let placed = crate::road_hauls_network::placed(doc, def, &alignment)?;
    warnings.extend(placed.warnings.iter().cloned());
    let mut sites: Vec<(usize, kentos_commands::HaulSiteDef)> = def
        .sites
        .iter()
        .zip(&placed.sites)
        .zip(&placed.unconnected)
        .enumerate()
        .filter(|(_, (_, off))| !**off)
        .map(|(j, ((s, p), _))| (j, kentos_commands::HaulSiteDef { station: p.station, ..s.clone() }))
        .collect();
    sites.sort_by(|a, b| a.1.station.total_cmp(&b.1.station).then(a.0.cmp(&b.0)));
    for (j, site) in sites {
        if site.station < first || site.station > last {
            return Err(fail(
                codes::INVALID_HAUL,
                format!(
                    "“{}” alanı kübajın km'leri dışında: kübaj {} – {} arası.",
                    site.name,
                    km_text(alignment.km(first), 3),
                    km_text(alignment.km(last), 3)
                ),
                &format!("sites[{j}]"),
            ));
        }
        let delta = match site.kind {
            HaulSiteKind::Borrow => site.volume,
            HaulSiteKind::Waste => -site.volume,
        };
        curve = step(&curve, site.station, delta);
    }
    let d = diagram(&curve, def.balance, def.free.unwrap_or(0.0));
    let text = |s: f64| km_text(alignment.km(s), 3);
    let loops = d
        .loops
        .iter()
        .map(|l| HaulLoop {
            from: l.from,
            to: l.to,
            from_text: text(l.from),
            to_text: text(l.to),
            forward: l.forward,
            volume: l.volume,
            moment: l.moment,
            average: (l.volume > 0.0).then(|| l.moment / l.volume),
            over_volume: l.over_volume,
            over_moment: l.over_moment,
            unbalanced: l.unbalanced,
        })
        .collect();
    if d.waste > 1e-6 || d.borrow > 1e-6 {
        warnings.push(CommandWarning {
            code: codes::HAUL_UNBALANCED.into(),
            message: format!("“{}” Brüknerinde dengelenmeyen hacim var: döküm {:.3} m³, ödünç {:.3} m³.", def.name, d.waste, d.borrow),
            path: Some("haul".into()),
        });
    }
    let priced = def.cost.as_ref().map(|c| cost(c, &d));
    let out = HaulReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        volume: def.volume.clone(),
        balance: def.balance,
        free: def.free,
        stale: report.stale,
        points: if points { curve } else { Vec::new() },
        balance_points: d.balance,
        loops,
        hauled: d.volume,
        moment: d.moment,
        average: (d.volume > 0.0).then(|| d.moment / d.volume),
        over_volume: d.over_volume,
        over_moment: d.over_moment,
        waste: d.waste,
        borrow: d.borrow,
        method: LAYERS.into(),
        cost: priced,
        problem: None,
    };
    Ok((out, warnings))
}

/// A report for a diagram that does not read: its sources and why.
pub(crate) fn broken(def: &HaulDef, why: String) -> HaulReport {
    HaulReport {
        id: def.id.clone(),
        name: def.name.clone(),
        revision: def.revision,
        volume: def.volume.clone(),
        balance: def.balance,
        free: def.free,
        stale: false,
        points: Vec::new(),
        balance_points: Vec::new(),
        loops: Vec::new(),
        hauled: 0.0,
        moment: 0.0,
        average: None,
        over_volume: 0.0,
        over_moment: 0.0,
        waste: 0.0,
        borrow: 0.0,
        method: LAYERS.into(),
        cost: None,
        problem: Some(why),
    }
}

/// The diagram's report now, or why it does not read.
pub(crate) fn report(doc: &Document, def: &HaulDef) -> HaulReport {
    match read(doc, def, false) {
        Ok((r, _)) => r,
        Err(stop) => broken(def, crate::road_corridors::words(stop)),
    }
}

/// A diagram read now: its report with the ordinate's points and its warnings, or why it does not read.
pub type ReadHaul = Result<(HaulReport, Vec<CommandWarning>), String>;

/// The road project's mass haul diagrams read now with their points, for the desktop's Brükner window (Y9).
pub fn read_hauls(doc: &Document) -> Vec<(HaulDef, ReadHaul)> {
    if road_project(doc).is_err() {
        return Vec::new();
    }
    doc.road_hauls().iter().map(|h| (h.clone(), read(doc, h, true).map_err(crate::road_corridors::words))).collect()
}

/// A diagram (perhaps not written) read now with its points, for the Brükner window.
pub fn preview(doc: &Document, def: &HaulDef) -> ReadHaul {
    read(doc, def, true).map_err(crate::road_corridors::words)
}

/// The alignment a diagram's volume's corridor lies on, built (docs/adr/0955: its haul road network joins it).
pub(crate) fn alignment_of(doc: &Document, def: &HaulDef) -> Result<crate::road_alignments::Built, Stop> {
    let bad = |words: String, path: &str| crate::road_terrains::fail(codes::INVALID_HAUL, words, path);
    let v = doc.road_volumes().iter().find(|v| v.id == def.volume).ok_or_else(|| bad(format!("“{}” kimlikli kübaj hesabı projede yok.", def.volume), "volume"))?;
    let c = doc.road_corridors().iter().find(|c| c.id == v.corridor).ok_or_else(|| bad(format!("“{}” kimlikli koridor projede yok.", v.corridor), "volume"))?;
    let list = crate::road_alignments::alignments(doc);
    let a = list.iter().find(|a| a.id == c.alignment).ok_or_else(|| bad(format!("“{}” kimlikli aks projede yok.", c.alignment), "volume"))?;
    crate::road_alignments::build(list, a, "").map_err(|stop| bad(format!("“{}” aksı kurulamıyor: {}", a.name, crate::road_corridors::words(stop)), "volume"))
}
