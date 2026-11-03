//! `road.junctions.clearance` v1 (plan/road.md Y17.4, docs/adr/0917): a separated junction's vertical clearance, the
//! upper road's profile less its deck's depth over the lower road's profile, where the roads overlap in plan
//! (`kentos-road`'s `junction::overlap`). Reads only. The shared cases are
//! `fixtures/commands/v1/road.junctions.clearance.json`.

use kentos_commands::{
    CommandResult, CommandWarning, JunctionClearanceAt, JunctionKind, JunctionsCleared, JunctionsClearance, MOST_CLEARANCE_SAMPLES,
    ProfileDef,
};
use kentos_domain::Document;
use kentos_road::junction::{Deck, Overlap, overlap, overlap_grid};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_junctions as common;
use crate::road_profiles as profiles;
use crate::road_terrains::fail;

fn bad(words: String, path: &str) -> Stop {
    fail(codes::INVALID_CLEARANCE, words, path)
}

fn answer(doc: &Document, input: &JunctionsClearance) -> Result<(JunctionsCleared, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_junctions(), &input.junction, "junction")?;
    let (report, mut warnings) = common::read(doc, def)?;
    if def.kind == JunctionKind::AtGrade {
        return Err(fail(
            codes::INVALID_JUNCTION,
            format!("“{}” hemzemin kavşak; düşey açıklık yalnız ayrı seviyeli kavşakta olur.", def.name),
            "junction",
        ));
    }
    let own = profiles::profiles(doc);
    // The legs' profiles: asked, else each leg's alignment's only one.
    let mut chosen: Vec<&ProfileDef> = Vec::new();
    match &input.profiles {
        Some(names) => {
            for (k, name) in names.iter().enumerate() {
                chosen.push(profiles::find(own, name, &format!("profiles[{k}]"))?.1);
            }
        }
        None => {
            for leg in &report.legs {
                let on: Vec<&ProfileDef> = own.iter().filter(|p| p.alignment == leg.alignment).collect();
                let name = alignments::alignments(doc).iter().find(|a| a.id == leg.alignment).map_or("", |a| a.name.as_str());
                match on.as_slice() {
                    [one] => chosen.push(one),
                    [] => return Err(bad(format!("“{name}” aksının boykesiti yok; profiles ile verin."), "profiles")),
                    _ => return Err(bad(format!("“{name}” aksının {} boykesiti var; hangisi, profiles ile verin.", on.len()), "profiles")),
                }
            }
        }
    }
    let mut built = Vec::new();
    for p in &chosen {
        built.push(profiles::build(doc, p)?);
    }
    if chosen.len() != 2 {
        return Err(bad(format!("Kolların sırasıyla iki boykesit verin; {} verildi.", chosen.len()), "profiles"));
    }
    for (k, (p, leg)) in chosen.iter().zip(&report.legs).enumerate() {
        if p.alignment != leg.alignment {
            return Err(bad(format!("“{}” boykesiti kavşağın {}. kolunun aksında değil.", p.name, k + 1), &format!("profiles[{k}]")));
        }
    }
    if !(input.depth.is_finite() && input.depth >= 0.0) {
        return Err(bad("Tabliye derinliği (depth) 0 ya da daha büyük olmalı.".into(), "depth"));
    }
    if !(input.step.is_finite() && input.step > 0.0) {
        return Err(bad("Örnekleme aralığı (step) 0'dan büyük olmalı.".into(), "step"));
    }
    if input.minimum.is_some_and(|m| !(m.is_finite() && m >= 0.0)) {
        return Err(bad("Gerekli açıklık (minimum) 0 ya da daha büyük olmalı.".into(), "minimum"));
    }
    for p in &chosen {
        let (stale, now) = profiles::stale(doc, p);
        if stale {
            warnings.push(CommandWarning {
                code: codes::PROFILE_STALE.into(),
                message: format!(
                    "“{}” boykesitinin aksı değişti (boykesit {}. revizyona göre kurulu, aks şimdi {}.); PVI'ları denetleyip aksa yeniden bağlayın.",
                    p.name, p.alignment_revision, now
                ),
                path: Some("profiles".into()),
            });
        }
    }
    // The upper road: the higher at the crossing.
    let height = |k: usize, s: f64| built[k].vertical.at(s).map(|r| r.0);
    let (s0, s1) = (report.legs[0].station, report.legs[1].station);
    let (Some(h0), Some(h1)) = (height(0, s0), height(1, s1)) else {
        return Err(bad("Bir boykesit aksların kesişimini örtmüyor.".into(), "profiles"));
    };
    if (h0 - h1).abs() < 1e-9 {
        return Err(bad(format!("İki kol kesişimde aynı kotta ({h0:.3} m); biri üstten geçmeli."), "profiles"));
    }
    let upper = usize::from(h1 > h0);
    let lower = 1 - upper;
    let deck = |k: usize| Deck { alignment: &built[k].alignment.core, left: def.legs[k].left, right: def.legs[k].right };
    let sine = (report.angle.to_radians()).sin();
    let crossing = report.legs[lower].station;
    let (n, m, _, _) = overlap_grid(deck(upper), deck(lower), crossing, sine, input.step);
    if n.saturating_mul(m) > MOST_CLEARANCE_SAMPLES {
        return Err(bad(format!("Aralık çok küçük: {n} × {m} ızgara noktası; en çok {MOST_CLEARANCE_SAMPLES}."), "step"));
    }
    let places = overlap(deck(upper), deck(lower), crossing, sine, input.step);
    if places.is_empty() {
        return Err(bad("Yollar planda örtüşmüyor.".into(), "junction"));
    }
    let at = |p: &Overlap| -> Option<JunctionClearanceAt> {
        let (hu, hl) = (height(upper, p.upper_station)?, height(lower, p.lower_station)?);
        Some(JunctionClearanceAt {
            x: p.x,
            y: p.y,
            upper_station: p.upper_station,
            upper_offset: p.upper_offset,
            lower_station: p.lower_station,
            lower_offset: p.lower_offset,
            upper_elevation: hu,
            lower_elevation: hl,
            clearance: hu - input.depth - hl,
        })
    };
    let mut read = Vec::with_capacity(places.len());
    for p in &places {
        let Some(c) = at(p) else {
            return Err(bad(
                format!("Boykesitler örtüşme bölgesini örtmüyor ({:.3} m km'sinde).", p.lower_station),
                "profiles",
            ));
        };
        read.push(c);
    }
    // The first of equal ones, in the samples' order.
    let pick = |better: fn(f64, f64) -> bool| {
        read.iter().fold(None::<&JunctionClearanceAt>, |b, c| match b {
            Some(b) if !better(c.clearance, b.clearance) => Some(b),
            _ => Some(c),
        })
    };
    let (Some(least), Some(largest)) = (pick(|a, b| a < b).cloned(), pick(|a, b| a > b).cloned()) else {
        return Err(bad("Yollar planda örtüşmüyor.".into(), "junction"));
    };
    let (pu, pl) = (&report.legs[upper], &report.legs[lower]);
    let Some(crossing_at) = at(&Overlap {
        x: report.x,
        y: report.y,
        upper_station: pu.station,
        upper_offset: 0.0,
        lower_station: pl.station,
        lower_offset: 0.0,
    }) else {
        return Err(bad("Bir boykesit aksların kesişimini örtmüyor.".into(), "profiles"));
    };
    let short = input.minimum.is_some_and(|m| least.clearance < m);
    if short {
        warnings.push(CommandWarning {
            code: codes::JUNCTION_CLEARANCE_SHORT.into(),
            message: format!(
                "“{}” kavşağında en küçük düşey açıklık {:.3} m, istenen {:.3} m'den az.",
                def.name,
                least.clearance,
                input.minimum.unwrap_or(0.0)
            ),
            path: Some("minimum".into()),
        });
    }
    Ok((
        JunctionsCleared {
            junction: def.id.clone(),
            upper: upper as u32,
            upper_profile: chosen[upper].id.clone(),
            lower_profile: chosen[lower].id.clone(),
            depth: input.depth,
            step: input.step,
            samples: read.len() as u32,
            least,
            largest,
            crossing: crossing_at,
            minimum: input.minimum,
            short,
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &JunctionsClearance) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &JunctionsClearance) -> CommandResult<JunctionsCleared> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: JunctionsClearance) -> CommandResult<JunctionsCleared> {
    plan(cx, &input)
}

/// The clearance for a window, or why it does not read.
pub fn preview(doc: &Document, input: &JunctionsClearance) -> Result<(JunctionsCleared, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
