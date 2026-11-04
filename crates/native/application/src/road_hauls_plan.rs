//! `road.hauls.plan` v1 (plan/road.md Y23, docs/adr/0924; ROAD-HAUL-02, ROAD-HAUL-07): a mass haul diagram's earthwork
//! moved at the least haul moment, its borrow and waste sites taking up to their capacity (`kentos_road::haul_plan`).
//! Reads only. The shared cases are `fixtures/commands/v1/road.hauls.plan.json`.

use kentos_commands::{
    CommandResult, CommandWarning, HaulObjective, HaulPlanCost, HaulPlanSite, HaulPlanned, HaulSegment, HaulSiteKind, HaulsPlan,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::haul_plan::{Barrier, PlanSite, Prices, SortedStretch, plan as solve, plan_priced, plan_sorted};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_hauls as hauls;
use crate::road_terrains::fail;
use crate::road_volumes as volumes;

/// The plan now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &HaulsPlan) -> Result<(HaulPlanned, Vec<CommandWarning>), Stop> {
    hauls::road_project(doc)?;
    let (_, def) = hauls::find(doc.road_hauls(), &input.haul, "haul")?;
    let vdef = doc.road_volumes().iter().find(|v| v.id == def.volume).ok_or_else(|| {
        fail(codes::UNKNOWN_VOLUME, format!("“{}” kimlikli kübaj hesabı projede yok.", def.volume), "volume")
    })?;
    let (report, vwarnings, alignment) = volumes::compute_on(doc, vdef).map_err(|stop| {
        fail(codes::INVALID_HAUL, format!("“{}” kübajı hesaplanamıyor: {}", vdef.name, crate::road_corridors::words(stop)), "volume")
    })?;
    let mut warnings: Vec<CommandWarning> =
        vwarnings.into_iter().filter(|w| w.code == codes::CORRIDOR_STALE || w.code == codes::VOLUME_GAP).collect();
    let curve = hauls::ordinate(&report);
    if curve.len() < 2 {
        return Err(fail(codes::INVALID_HAUL, format!("“{}” kübajının hacimli aralığı yok; taşınacak bir şey yok.", vdef.name), "volume"));
    }
    let (first, last) = (curve[0][0], curve[curve.len() - 1][0]);
    let mut order: Vec<(usize, &kentos_commands::HaulSiteDef)> = def.sites.iter().enumerate().collect();
    order.sort_by(|a, b| a.1.station.total_cmp(&b.1.station).then(a.0.cmp(&b.0)));
    for (j, site) in order {
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
    }
    let sites: Vec<PlanSite> = def
        .sites
        .iter()
        .map(|s| PlanSite { station: s.station, capacity: s.volume, borrow: s.kind == HaulSiteKind::Borrow })
        .collect();
    let objective = input.objective.unwrap_or(HaulObjective::Moment);
    if !def.unsuitable.is_empty() || !def.barriers.is_empty() {
        if objective == HaulObjective::Cost {
            return Err(fail(
                codes::INVALID_HAUL,
                format!("“{}” Brüknerinde uygunsuz malzeme ya da engel var; ekonomik plan onlarla henüz çözülmez, momentle planlayın.", def.name),
                "objective",
            ));
        }
        return Ok(sorted(def, &report, &sites, warnings));
    }
    let (p, priced) = match objective {
        HaulObjective::Moment => (solve(&curve, &sites), None),
        HaulObjective::Cost => {
            let c = def.cost.as_ref().ok_or_else(|| {
                fail(codes::INVALID_HAUL, format!("“{}” Brüknerinin fiyatları yok; ekonomik plan fazla taşıma birim fiyatı ister.", def.name), "objective")
            })?;
            let overhaul = c.overhaul.filter(|o| *o > 0.0).ok_or_else(|| {
                fail(codes::INVALID_HAUL, format!("“{}” Brüknerinin fazla taşıma birim fiyatı yok ya da 0; ekonomik plan onu ister.", def.name), "objective")
            })?;
            for (kind, price, words) in
                [(HaulSiteKind::Waste, c.waste, "döküm alanı var ama döküm birim fiyatı yok"), (HaulSiteKind::Borrow, c.borrow, "ödünç alanı var ama ödünç birim fiyatı yok")]
            {
                if price.is_none() && def.sites.iter().any(|s| s.kind == kind) {
                    return Err(fail(codes::INVALID_HAUL, format!("“{}” Brüknerinin {words}.", def.name), "objective"));
                }
            }
            let prices = Prices {
                free: def.free.unwrap_or(0.0),
                overhaul: overhaul / 1000.0,
                waste: c.waste.unwrap_or(0.0),
                borrow: c.borrow.unwrap_or(0.0),
            };
            let costed = plan_priced(&curve, &sites, prices);
            let economic = (c.waste.is_some() || c.borrow.is_some()).then(|| prices.free + (prices.waste + prices.borrow) / prices.overhaul);
            let cost = costed.plan.feasible.then(|| HaulPlanCost {
                currency: c.currency.clone(),
                over_moment: costed.over_moment,
                overhaul: costed.overhaul,
                waste: costed.waste,
                borrow: costed.borrow,
                total: costed.total,
                economic,
            });
            (costed.plan, Some((def.free, cost)))
        }
    };
    if !p.feasible {
        warnings.push(CommandWarning {
            code: codes::HAUL_PLAN_INFEASIBLE.into(),
            message: format!(
                "“{}” taşıma planı her şeyi yerleştiremiyor: karşılanmayan dolgu {:.3} m³, yerleşmeyen yarma {:.3} m³; ödünç ya da döküm alanı ekleyin ya da büyütün.",
                def.name, p.shortage, p.surplus
            ),
            path: Some("haul".into()),
        });
    }
    Ok((
        HaulPlanned {
            haul: def.id.clone(),
            feasible: p.feasible,
            cut: p.cut,
            fill: p.fill,
            moved: p.moved,
            moment: p.moment,
            average: (p.moved > 0.0 && p.feasible).then(|| p.moment / p.moved),
            sites: def
                .sites
                .iter()
                .zip(&p.used)
                .map(|(s, u)| HaulPlanSite {
                    name: s.name.clone(),
                    kind: s.kind,
                    station: s.station,
                    capacity: s.volume,
                    used: *u,
                    used_unsuitable: None,
                })
                .collect(),
            segments: p.segments.iter().map(|&(from, to, flow)| HaulSegment { from, to, flow, unsuitable_flow: None }).collect(),
            shortage: p.shortage,
            surplus: p.surplus,
            objective,
            free: priced.as_ref().and_then(|x| x.0),
            cost: priced.and_then(|x| x.1),
            unsuitable: None,
            stranded: None,
            unsuitable_moment: None,
            barriers: Vec::new(),
        },
        warnings,
    ))
}

/// The share (0 to 1) of the cut unsuitable at station `s`: the row it falls in, none outside them.
fn share_at(def: &kentos_commands::HaulDef, s: f64) -> f64 {
    def.unsuitable.iter().find(|u| u.from <= s && s < u.to).map_or(0.0, |u| u.share / 100.0)
}

/// The plan of a diagram with unsuitable cut or barriers (docs/adr/0944): each stretch's factored cut and fill, the cut's
/// unsuitable share at its middle, by `kentos_road::haul_plan::plan_sorted`.
fn sorted(
    def: &kentos_commands::HaulDef,
    report: &kentos_commands::VolumeReport,
    sites: &[PlanSite],
    mut warnings: Vec<CommandWarning>,
) -> (HaulPlanned, Vec<CommandWarning>) {
    let stretches: Vec<SortedStretch> = report
        .rows
        .iter()
        .filter_map(|r| {
            Some(SortedStretch {
                from: r.from,
                to: r.to,
                cut: r.cut? * r.cut_factor.unwrap_or(1.0),
                fill: r.fill? * r.fill_factor.unwrap_or(1.0),
                unsuitable: share_at(def, 0.5 * (r.from + r.to)),
            })
        })
        .collect();
    let barriers: Vec<Barrier> = def.barriers.iter().map(|b| Barrier { station: b.station, detour: b.detour }).collect();
    let p = plan_sorted(&stretches, sites, &barriers);
    // As the plain plan says them: the cut more than the fill and the fill more than the cut over the stretches.
    let cut: f64 = stretches.iter().map(|r| (r.cut - r.fill).max(0.0)).sum();
    let fill: f64 = stretches.iter().map(|r| (r.fill - r.cut).max(0.0)).sum();
    // What leaves its stretch: the suitable cut beyond its own fill and the unsuitable cut, placed; and the borrow.
    let leaves: f64 = stretches.iter().map(|r| (r.cut * (1.0 - r.unsuitable) - r.fill).max(0.0) + r.cut * r.unsuitable).sum();
    let borrowed: f64 = sites.iter().zip(&p.used).filter(|(s, _)| s.borrow).map(|(_, u)| u).sum();
    let moved = (leaves - p.surplus - p.stranded).max(0.0) + borrowed;
    if !p.feasible {
        warnings.push(CommandWarning {
            code: codes::HAUL_PLAN_INFEASIBLE.into(),
            message: format!(
                "“{}” taşıma planı her şeyi yerleştiremiyor: karşılanmayan dolgu {:.3} m³, yerleşmeyen uygun yarma {:.3} m³, dökülemeyen uygunsuz yarma {:.3} m³; ödünç ya da döküm alanı ekleyin, büyütün ya da kapalı engeli açın.",
                def.name, p.shortage, p.surplus, p.stranded
            ),
            path: Some("haul".into()),
        });
    }
    (
        HaulPlanned {
            haul: def.id.clone(),
            feasible: p.feasible,
            cut,
            fill,
            moved,
            moment: p.moment,
            average: (moved > 0.0 && p.feasible).then(|| p.moment / moved),
            sites: def
                .sites
                .iter()
                .enumerate()
                .map(|(j, s)| HaulPlanSite {
                    name: s.name.clone(),
                    kind: s.kind,
                    station: s.station,
                    capacity: s.volume,
                    used: p.used[j],
                    used_unsuitable: (s.kind == HaulSiteKind::Waste).then_some(p.used_unsuitable[j]),
                })
                .collect(),
            segments: p
                .segments
                .iter()
                .map(|&(from, to, good, bad)| HaulSegment { from, to, flow: good + bad, unsuitable_flow: Some(bad) })
                .collect(),
            shortage: p.shortage,
            surplus: p.surplus,
            objective: HaulObjective::Moment,
            free: None,
            cost: None,
            unsuitable: Some(p.unsuitable),
            stranded: Some(p.stranded),
            unsuitable_moment: Some(p.unsuitable_moment),
            barriers: def
                .barriers
                .iter()
                .zip(&p.crossed)
                .map(|(b, c)| kentos_commands::HaulPlanBarrier { name: b.name.clone(), station: b.station, detour: b.detour, crossed: *c })
                .collect(),
        },
        warnings,
    )
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &HaulsPlan) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &HaulsPlan) -> CommandResult<HaulPlanned> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: HaulsPlan) -> CommandResult<HaulPlanned> {
    plan(cx, &input)
}

/// The plan for a window, or why it does not read.
pub fn preview(doc: &Document, input: &HaulsPlan) -> Result<(HaulPlanned, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
