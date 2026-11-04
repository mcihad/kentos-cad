//! `road.hauls.plan` v1 (plan/road.md Y23, docs/adr/0924; ROAD-HAUL-02, ROAD-HAUL-07): a mass haul diagram's earthwork
//! moved at the least haul moment, its borrow and waste sites taking up to their capacity (`kentos_road::haul_plan`).
//! Reads only. The shared cases are `fixtures/commands/v1/road.hauls.plan.json`.

use kentos_commands::{
    CommandResult, CommandWarning, HaulObjective, HaulPlanCost, HaulPlanSite, HaulPlanned, HaulSegment, HaulSiteKind, HaulsPlan,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::haul_plan::{PlanSite, Prices, plan as solve, plan_priced};

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
                .map(|(s, u)| HaulPlanSite { name: s.name.clone(), kind: s.kind, station: s.station, capacity: s.volume, used: *u })
                .collect(),
            segments: p.segments.iter().map(|&(from, to, flow)| HaulSegment { from, to, flow }).collect(),
            shortage: p.shortage,
            surplus: p.surplus,
            objective,
            free: priced.as_ref().and_then(|x| x.0),
            cost: priced.and_then(|x| x.1),
        },
        warnings,
    ))
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
