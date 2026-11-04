//! `road.hauls.plan` v1 (plan/road.md Y23, docs/adr/0924; ROAD-HAUL-02, ROAD-HAUL-07): a mass haul diagram's earthwork
//! moved at the least haul moment, its borrow and waste sites taking up to their capacity (`kentos_road::haul_plan`).
//! Reads only. The shared cases are `fixtures/commands/v1/road.hauls.plan.json`.

use kentos_commands::{CommandResult, CommandWarning, HaulPlanSite, HaulPlanned, HaulSegment, HaulSiteKind, HaulsPlan};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::haul_plan::{PlanSite, plan as solve};

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
    let p = solve(&curve, &sites);
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
