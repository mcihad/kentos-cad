//! `road.culverts.closure` v1 (plan/road.md Y42, docs/adr/0948; ROAD-DRAIN-08): a flood's hydrograph through a culvert
//! and over the road (`road.culverts.hydraulics`' model of it), and how long the road is under water, closed to each
//! vehicle class and at each hazard class, with when (`kentos-road`'s `culvert_closure`); with a range of the flow
//! factor and Cd, the shortest and longest of each, exact at the range's corners. Reads only. The shared cases are
//! `fixtures/commands/v1/road.culverts.closure.json`.

use kentos_commands::{
    ClosureCriterion, ClosureFlood, ClosureSpan, CommandResult, CommandWarning, CulvertClosureRead, CulvertsClosure,
};
use kentos_domain::Document;
use kentos_road::culvert_closure::{Span, VEHICLES, flood};
use kentos_road::culvert_hydraulics::hazard_words;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_culverts as common;
use crate::road_culverts_hydraulics::{Asked, bad, check, flow_read, model, within};

/// The vehicle classes' words, in [`VEHICLES`]' order.
const VEHICLE_WORDS: [&str; 3] = ["Küçük binek araç", "Büyük binek araç", "Büyük 4×4 araç"];

fn criterion(code: &str, words: &str, s: &Span) -> ClosureCriterion {
    ClosureCriterion {
        criterion: code.into(),
        words: words.into(),
        flow: s.flow,
        hours: s.hours,
        spans: s.spans.iter().map(|&(from, to)| ClosureSpan { from, to }).collect(),
    }
}

fn answer(doc: &Document, input: &CulvertsClosure) -> Result<(CulvertClosureRead, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_culverts(), &input.culvert, "culvert")?;
    let asked = Asked {
        tailwater: input.tailwater,
        inlet: input.inlet,
        entrance_loss: input.entrance_loss,
        roughness: input.roughness,
        weir: Some(&input.weir),
    };
    let n = check(def, &asked)?;
    let h = &input.hydrograph;
    if !(2..=1000).contains(&h.len()) {
        return Err(bad("hydrograph", "hidrograf 2 ile 1000 nokta olur"));
    }
    if let Some(k) = h.iter().position(|p| !within(p.time, 0.0, 8760.0) || !within(p.flow, 0.0, 10_000.0)) {
        return Err(bad(&format!("hydrograph.{k}"), "hidrografın saati 0 ile 8760, debisi 0 ile 10000 m³/s arası olur"));
    }
    if let Some(k) = h.windows(2).position(|w| w[1].time <= w[0].time) {
        return Err(bad(&format!("hydrograph.{}", k + 1), "hidrografın saatleri artarak gider"));
    }
    if h.iter().all(|p| p.flow <= 0.0) {
        return Err(bad("hydrograph", "hidrografta debi yok"));
    }
    if let Some(r) = &input.range {
        if !(r.flow_low > 0.0 && within(r.flow_low, 0.0, 1.0) && within(r.flow_high, 1.0, 10.0)) {
            return Err(bad("range", "debi çarpanının alt sınırı 0 ile 1, üst sınırı 1 ile 10 arası olur"));
        }
        if !(r.cd_low > 0.0 && r.cd_low <= input.weir.cd && within(r.cd_high, input.weir.cd, 3.0)) {
            return Err(bad("range", "Cd'nin alt sınırı 0 ile savağın Cd'si, üst sınırı savağın Cd'si ile 3 arası olur"));
        }
    }
    let m = model(doc, def, &asked, n)?;
    let tw = input.tailwater.unwrap_or(0.0);
    let points: Vec<(f64, f64)> = h.iter().map(|p| (p.time, p.flow)).collect();
    let mut asked_at = vec![("asked", 1.0, input.weir.cd)];
    if let Some(r) = &input.range {
        asked_at.push(("low", r.flow_low, r.cd_low));
        asked_at.push(("high", r.flow_high, r.cd_high));
    }
    let floods: Vec<ClosureFlood> = asked_at
        .into_iter()
        .map(|(case, factor, cd)| {
            let f = flood(&m.culvert, &m.weir, tw, &points, factor, cd);
            let weir = kentos_road::culvert_hydraulics::Weir { cd, ..m.weir };
            let mut criteria = vec![criterion("submerged", "Su altında", &f.submerged)];
            for (k, s) in f.vehicles.iter().enumerate() {
                criteria.push(criterion(VEHICLES[k].0, VEHICLE_WORDS[k], s));
            }
            for (k, s) in f.hazards.iter().enumerate() {
                let class = k as u8 + 2;
                criteria.push(criterion(&format!("H{class}"), hazard_words(class), s));
            }
            ClosureFlood {
                case: case.into(),
                factor,
                cd,
                peak: flow_read(&m.culvert, &weir, f.peak_flow, tw, m.inlet_invert),
                criteria,
            }
        })
        .collect();
    let mut warnings = m.warnings;
    let hours = |f: &ClosureFlood, k: usize| f.criteria[k].hours;
    let asked_flood = &floods[0];
    let range = |k: usize| {
        if floods.len() == 3 {
            format!(" (aralıkta {:.2}–{:.2} saat)", hours(&floods[1], k), hours(&floods[2], k))
        } else {
            String::new()
        }
    };
    let sub = &asked_flood.criteria[0];
    if sub.hours > 0.0 {
        let (a, b) = (sub.spans.first().map_or(0.0, |s| s.from), sub.spans.last().map_or(0.0, |s| s.to));
        warnings.push(CommandWarning {
            code: codes::CULVERT_OVERTOPPED.into(),
            message: format!(
                "“{}” menfezinin üstündeki yol taşkında {:.2} saat su altında: {a:.2}. saatte başlar, {b:.2}. saatte biter{}.",
                def.name,
                sub.hours,
                range(0)
            ),
            path: Some("hydrograph".into()),
        });
    }
    let small = &asked_flood.criteria[1];
    if small.hours > 0.0 {
        warnings.push(CommandWarning {
            code: codes::CULVERT_ROAD_CLOSED.into(),
            message: format!(
                "“{}” menfezinin üstündeki yol küçük binek araçlara {:.2} saat kapalı{}.",
                def.name,
                small.hours,
                range(1)
            ),
            path: Some("hydrograph".into()),
        });
    }
    Ok((
        CulvertClosureRead {
            culvert: def.id.clone(),
            inlet_invert: m.inlet_invert,
            crest: m.crest.unwrap_or(m.inlet_invert),
            floods,
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &CulvertsClosure) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &CulvertsClosure) -> CommandResult<CulvertClosureRead> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CulvertsClosure) -> CommandResult<CulvertClosureRead> {
    plan(cx, &input)
}

/// The closure for a window, or why it does not read.
pub fn preview(doc: &Document, input: &CulvertsClosure) -> Result<(CulvertClosureRead, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
