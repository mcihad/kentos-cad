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
/// The plan now, and against its approval where the diagram has one (docs/adr/0954): the plan made again with the
/// approved objective, its figures and sources against the approved ones.
pub(crate) fn answer(doc: &Document, input: &HaulsPlan) -> Result<(HaulPlanned, Vec<CommandWarning>), Stop> {
    let (mut out, mut warnings) = raw(doc, input)?;
    let (_, def) = hauls::find(doc.road_hauls(), &input.haul, "haul")?;
    if let Some(a) = &def.approved {
        let objective = if a.cost { HaulObjective::Cost } else { HaulObjective::Moment };
        let today = if objective == out.objective {
            Some(out.clone())
        } else {
            raw(doc, &HaulsPlan { haul: def.id.clone(), objective: Some(objective) }).ok().map(|r| r.0)
        };
        let report = approval_against(doc, def, a, today.as_ref());
        if report.stale {
            warnings.push(CommandWarning {
                code: codes::HAUL_PLAN_STALE.into(),
                message: format!(
                    "“{}” Brüknerinin {}. onaylı taşıma planı ({}) eskidi: {}.",
                    def.name,
                    a.number,
                    a.at,
                    report.reasons.join("; ")
                ),
                path: Some("haul".into()),
            });
        }
        out.approval = Some(report);
    }
    Ok((out, warnings))
}

/// Within this share (of the larger, at least 1) two of a plan's figures are the same.
const SAME: f64 = 1e-6;

fn differ(a: f64, b: f64) -> bool {
    (a - b).abs() > SAME * a.abs().max(b.abs()).max(1.0)
}

/// The approval `a` of `def` against today's plan made with its objective (none: it does not compute).
pub(crate) fn approval_against(
    doc: &Document,
    def: &kentos_commands::HaulDef,
    a: &kentos_commands::HaulApprovalDef,
    today: Option<&HaulPlanned>,
) -> kentos_commands::HaulPlanApproval {
    use kentos_commands::HaulSiteChange;
    let mut reasons = Vec::new();
    if def.revision != a.revision {
        reasons.push(format!("Brüknerin tanımı değişti (sürüm {} → {})", a.revision, def.revision));
    }
    let volume = doc.road_volumes().iter().find(|v| v.id == def.volume);
    let (vrev, vnum) = volume.map_or((0, 0), |v| (v.revision, v.approved.as_ref().map_or(0, |x| x.number.max(1))));
    if vrev != a.volume_revision {
        reasons.push(format!("kübajın tanımı değişti (sürüm {} → {vrev})", a.volume_revision));
    }
    if vnum != a.volume_approval {
        let words = |n: u32| if n == 0 { "onaysız".to_owned() } else { format!("{n}. onay") };
        reasons.push(format!("kübajın onayı değişti ({} → {})", words(a.volume_approval), words(vnum)));
    }
    // The sites: capacities from the definition, uses from today's plan.
    let mut sites = Vec::new();
    for s in &a.sites {
        let now = def.sites.iter().find(|d| d.name == s.name);
        let used = today.and_then(|t| t.sites.iter().find(|x| x.name == s.name).map(|x| x.used));
        let capacity = now.map(|d| d.volume);
        let capacity_changed = capacity.is_none_or(|c| differ(c, s.capacity));
        if capacity_changed {
            reasons.push(match capacity {
                Some(c) => format!("{} alanının kapasitesi değişti ({:.1} → {:.1} m³)", s.name, s.capacity, c),
                None => format!("{} alanı kalktı", s.name),
            });
        }
        if capacity_changed || used.is_none_or(|u| differ(u, s.used)) {
            sites.push(HaulSiteChange {
                name: s.name.clone(),
                approved_capacity: Some(s.capacity),
                capacity,
                approved_used: Some(s.used),
                used,
            });
        }
    }
    for d in def.sites.iter().filter(|d| !a.sites.iter().any(|s| s.name == d.name)) {
        reasons.push(format!("{} alanı eklendi ({:.1} m³)", d.name, d.volume));
        sites.push(HaulSiteChange {
            name: d.name.clone(),
            approved_capacity: None,
            capacity: Some(d.volume),
            approved_used: None,
            used: today.and_then(|t| t.sites.iter().find(|x| x.name == d.name).map(|x| x.used)),
        });
    }
    let moment_change = today.map(|t| t.moment - a.moment);
    match today {
        None => reasons.push("plan bugün kurulamıyor".into()),
        Some(t) => {
            // What of the plan's own figures changed: its moment, the volume moved, its cost.
            let mut parts = Vec::new();
            if differ(t.moment, a.moment) {
                parts.push(format!("moment {:+.1} m³·m", t.moment - a.moment));
            }
            if differ(t.moved, a.moved) {
                parts.push(format!("taşınan {:+.1} m³", t.moved - a.moved));
            }
            match (t.cost.as_ref(), a.total) {
                (Some(c), Some(y)) if differ(c.total, y) => parts.push(format!("tutar {:+.0} {}", c.total - y, c.currency)),
                (None, Some(_)) => parts.push("tutar bugün hesaplanamıyor".into()),
                (Some(_), None) => parts.push("tutar onayda yoktu".into()),
                _ => {}
            }
            if !parts.is_empty() {
                reasons.push(format!("planın kendisi değişti ({})", parts.join(", ")));
            }
        }
    }
    kentos_commands::HaulPlanApproval { approved: a.clone(), stale: !reasons.is_empty(), reasons, moment_change, sites }
}

/// The plan now, without its approval.
fn raw(doc: &Document, input: &HaulsPlan) -> Result<(HaulPlanned, Vec<CommandWarning>), Stop> {
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
            approval: None,
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
            approval: None,
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

/// A number as a sheet writes it: three decimals, a dot.
fn decimal(v: f64) -> String {
    format!("{v:.3}")
}

/// The plan as a workbook's sheets (plan/road.md ROAD-HAUL-12, docs/adr/0953; docs/adr/0223's writer): Özet — the
/// diagram, its volume with the volume's revision and approval, the objective, the figures each with its unit and what
/// its volumes are (the volume's factored ones, cut loose and fill compacted); Taşımalar — each stretch's flow, its
/// direction and length; Alanlar — the borrow and waste sites; Engeller where there are barriers.
pub fn sheets(doc: &Document, planned: &HaulPlanned) -> Vec<kentos_commands::WorkbookSheet> {
    use kentos_commands::{CellKind, WorkbookColumn, WorkbookSheet};
    let column = |name: &str, kind: CellKind, decimals: Option<u32>| WorkbookColumn { name: name.into(), kind, decimals };
    let text = |name: &str| column(name, CellKind::Text, None);
    let n = |name: &str| column(name, CellKind::Number, Some(3));
    let haul = doc.road_hauls().iter().find(|h| h.id == planned.haul);
    let volume = haul.and_then(|h| doc.road_volumes().iter().find(|v| v.id == h.volume));
    let alignment = volume.and_then(|v| volumes::compute_on(doc, v).ok()).map(|(_, _, a)| a);
    let km = |s: f64| alignment.as_ref().map_or_else(|| decimal(s), |a| km_text(a.km(s), 3));
    let row = |k: &str, v: String, unit: &str| vec![k.to_owned(), v, unit.to_owned()];
    let mut summary = vec![
        row("Brükner", haul.map_or(planned.haul.clone(), |h| h.name.clone()), ""),
        row("Brükner sürümü", haul.map_or_else(String::new, |h| h.revision.to_string()), ""),
        row("Kübaj", volume.map_or_else(String::new, |v| v.name.clone()), ""),
        row("Kübaj sürümü", volume.map_or_else(String::new, |v| v.revision.to_string()), ""),
        row(
            "Kübaj onayı",
            volume.and_then(|v| v.approved.as_ref()).map_or_else(
                || "onaysız".into(),
                |a| if a.number > 0 { format!("{}. onay · {}", a.number, a.at) } else { format!("onaylı · {}", a.at) },
            ),
            "",
        ),
        row("Hacimler", "kübajın düzeltilmiş hacmi: yarma gevşek, dolgu sıkışık".into(), ""),
        row(
            "Amaç",
            match planned.objective {
                HaulObjective::Moment => "En küçük taşıma momenti",
                HaulObjective::Cost => "En düşük maliyet",
            }
            .into(),
            "",
        ),
        row("Durum", if planned.feasible { "Kurulur" } else { "Kurulamaz" }.into(), ""),
        row("Yarma fazlası", decimal(planned.cut), "m³"),
        row("Dolgu fazlası", decimal(planned.fill), "m³"),
        row("Taşınan", decimal(planned.moved), "m³"),
        row("Taşıma momenti", decimal(planned.moment), "m³·m"),
        row("Ortalama uzaklık", planned.average.map_or_else(String::new, decimal), "m"),
        row("Karşılanmayan dolgu", decimal(planned.shortage), "m³"),
        row("Yerleşmeyen yarma", decimal(planned.surplus), "m³"),
    ];
    if let Some(f) = planned.free {
        summary.push(row("Serbest taşıma", decimal(f), "m"));
    }
    if let Some(c) = &planned.cost {
        summary.push(row("Fazla taşıma momenti", decimal(c.over_moment), "m³·m"));
        for (k, v) in [("Fazla taşıma", c.overhaul), ("Döküm", c.waste), ("Ödünç", c.borrow), ("Toplam maliyet", c.total)] {
            summary.push(row(k, decimal(v), &c.currency));
        }
    }
    if let Some(u) = planned.unsuitable {
        summary.push(row("Uygunsuz yarma", decimal(u), "m³"));
        summary.push(row("Dökülemeyen uygunsuz", planned.stranded.map_or_else(String::new, decimal), "m³"));
        summary.push(row("Uygunsuzun momenti", planned.unsuitable_moment.map_or_else(String::new, decimal), "m³·m"));
    }
    // The plan's approval (docs/adr/0954): its number and day, what it approved, and whether it went stale and why.
    if let Some(a) = &planned.approval {
        let x = &a.approved;
        let note = x.note.as_ref().map_or_else(String::new, |n| format!(" · {n}"));
        summary.push(row("Taşıma planı onayı", format!("{}. onay · {}{note}", x.number, x.at), ""));
        summary.push(row("Onaylı amaç", if x.cost { "En düşük maliyet" } else { "En küçük moment" }.into(), ""));
        summary.push(row("Onaylı taşınan", decimal(x.moved), "m³"));
        summary.push(row("Onaylı taşıma momenti", decimal(x.moment), "m³·m"));
        if let Some(t) = x.total {
            summary.push(row("Onaylı toplam", decimal(t), planned.cost.as_ref().map_or("", |c| c.currency.as_str())));
        }
        summary.push(row("Onay", if a.stale { format!("eskidi: {}", a.reasons.join("; ")) } else { "güncel".into() }, ""));
    }
    let sorted = planned.unsuitable.is_some();
    // A stretch's haul length is its stations' difference plus the detours of the barriers on it ([from, to)): the
    // plan's moment is Σ (|suitable| + |unsuitable|) × that length; the net flow alone would undercount where the two
    // run opposite ways (docs/adr/0944).
    let detour = |s: &HaulSegment| -> f64 {
        // Folded from +0: an empty f64 sum is −0, which a sheet writes as “-0.0”.
        planned.barriers.iter().filter(|b| s.from <= b.station && b.station < s.to).filter_map(|b| b.detour).fold(0.0, |a, d| a + d)
    };
    let barriers = !planned.barriers.is_empty();
    let mut flow_columns = vec![text("Başlangıç km"), text("Bitiş km"), n("Uzunluk (m)")];
    if barriers {
        flow_columns.push(n("Dolaşma (m)"));
        flow_columns.push(n("Taşıma boyu (m)"));
    }
    if sorted {
        flow_columns.push(n("Uygun (m³, + ileri)"));
        flow_columns.push(n("Uygunsuz (m³, + ileri)"));
    } else {
        flow_columns.push(text("Yön"));
        flow_columns.push(n("Hacim (m³)"));
    }
    let flows = planned
        .segments
        .iter()
        .filter(|s| s.flow.abs() > 1e-9 || s.unsuitable_flow.is_some_and(|u| u.abs() > 1e-9))
        .map(|s| {
            let mut r = vec![km(s.from), km(s.to), decimal(s.to - s.from)];
            if barriers {
                r.push(decimal(detour(s)));
                r.push(decimal(s.to - s.from + detour(s)));
            }
            if sorted {
                let bad = s.unsuitable_flow.unwrap_or(0.0);
                r.push(decimal(s.flow - bad));
                r.push(decimal(bad));
            } else {
                r.push(if s.flow > 0.0 { "ileri" } else { "geri" }.to_owned());
                r.push(decimal(s.flow.abs()));
            }
            r
        })
        .collect();
    let mut site_columns = vec![text("Alan"), text("Tür"), text("Km"), n("Kapasite (m³)"), n("Kullanılan (m³)")];
    if sorted {
        site_columns.push(n("Uygunsuz (m³)"));
    }
    let sites = planned
        .sites
        .iter()
        .map(|s| {
            let mut r = vec![
                s.name.clone(),
                if s.kind == kentos_commands::HaulSiteKind::Borrow { "ödünç" } else { "döküm" }.to_owned(),
                km(s.station),
                decimal(s.capacity),
                decimal(s.used),
            ];
            if sorted {
                r.push(s.used_unsuitable.map_or_else(String::new, decimal));
            }
            r
        })
        .collect();
    let mut out = vec![
        WorkbookSheet { name: "Özet".into(), columns: vec![text("Kalem"), text("Değer"), text("Birim")], rows: summary },
        WorkbookSheet { name: "Taşımalar".into(), columns: flow_columns, rows: flows },
        WorkbookSheet { name: "Alanlar".into(), columns: site_columns, rows: sites },
    ];
    if !planned.barriers.is_empty() {
        out.push(WorkbookSheet {
            name: "Engeller".into(),
            columns: vec![text("Engel"), text("Km"), text("Dolaşma (m)"), n("Geçen (m³)")],
            rows: planned
                .barriers
                .iter()
                .map(|b| vec![b.name.clone(), km(b.station), b.detour.map_or_else(|| "kapalı".into(), decimal), decimal(b.crossed)])
                .collect(),
        });
    }
    out
}
