//! `road.junctions.tie` v1 (plan/road.md Y17.5, docs/adr/0918): at an at-grade junction the secondary road's profile
//! tied to the primary road's crown, which is kept: three PVIs without curves at the crossing and where the secondary
//! centreline crosses the primary road's edges (`kentos-road`'s `junction::edge_stations`), the secondary PVIs between
//! those edges taken off. One undo step “Kavşağa bağla”. The shared cases are
//! `fixtures/commands/v1/road.junctions.tie.json`.

use kentos_commands::{
    CommandResult, CommandWarning, JunctionKind, JunctionTie, JunctionsTie, JunctionsTied, ProfileDef, ProfilePvi,
};
use kentos_domain::Document;
use kentos_road::junction::{Deck, edge_stations};
use kentos_road::superelevation::Crossfall;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_junctions as common;
use crate::road_profiles as profiles;
use crate::road_superelevations as supers;
use crate::road_terrains::fail;

pub(crate) const STEP: &str = "Kavşağa bağla";

fn bad(words: String, path: &str) -> Stop {
    fail(codes::INVALID_TIE, words, path)
}

/// The profiles after the tie, and the answer (its revision the document's now).
struct Checked {
    list: Vec<ProfileDef>,
    tied: JunctionsTied,
    warnings: Vec<CommandWarning>,
}

fn check(doc: &Document, input: &JunctionsTie) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_junctions(), &input.junction, "junction")?;
    let (report, mut warnings) = common::read(doc, def)?;
    if def.kind == JunctionKind::Separated {
        return Err(fail(
            codes::INVALID_JUNCTION,
            format!("“{}” ayrı seviyeli kavşak; boykesit bağı yalnız hemzemin kavşakta olur.", def.name),
            "junction",
        ));
    }
    let (chosen, built) = common::leg_profiles(doc, &report, input.profiles.as_deref(), codes::INVALID_TIE)?;
    if input.primary > 1 {
        return Err(bad("Sırtı korunan kol (primary) 0 ya da 1 olmalı.".into(), "primary"));
    }
    let (p, q) = (input.primary as usize, 1 - input.primary as usize);
    // The primary road's cross slopes: given, or its superelevation's.
    let slopes: Box<dyn Fn(f64) -> Crossfall> = match (&input.crossfall, &input.superelevation) {
        (Some(_), Some(_)) | (None, None) => {
            return Err(bad(
                "Ana yolun enine eğimini ya crossfall ya da superelevation ile verin; biri verilir.".into(),
                "crossfall",
            ));
        }
        (Some(c), None) => {
            if !c.iter().all(|v| v.is_finite()) {
                return Err(bad("Enine eğimler sonlu sayılar olmalı.".into(), "crossfall"));
            }
            let c = Crossfall { left: c[0], right: c[1] };
            Box::new(move |_| c)
        }
        (None, Some(name)) => {
            let (_, sdef) = supers::find(doc.road_superelevations(), name, "superelevation")?;
            let on = supers::alignment_of(doc, sdef)?;
            let (schedule, _) = supers::schedule(sdef, &on)?;
            if sdef.alignment != report.legs[p].alignment {
                return Err(bad(format!("“{}” dever çizelgesi ana yolun aksında değil.", sdef.name), "superelevation"));
            }
            warnings.extend(supers::stale_warning(doc, sdef));
            let normal = supers::normal(sdef);
            Box::new(move |s| schedule.at(s, normal))
        }
    };
    if input.max_grade_change.is_some_and(|m| !(m.is_finite() && m >= 0.0)) {
        return Err(bad("En büyük eğim değişimi (maxGradeChange) 0 ya da daha büyük olmalı.".into(), "maxGradeChange"));
    }
    warnings.extend(common::profiles_stale(doc, &chosen));
    let (primary, secondary) = (&built[p], &built[q]);
    let deck = Deck { alignment: &primary.alignment.core, left: def.legs[p].left, right: def.legs[p].right };
    let sine = report.angle.to_radians().sin();
    let edges = edge_stations(deck, &secondary.alignment.core, report.legs[q].station, sine);
    let red = |b: &profiles::Built, s: f64| b.vertical.at(s).map(|r| r.0);
    let uncovered = |who: &ProfileDef| bad(format!("“{}” boykesiti bağı örtmüyor.", who.name), "profiles");
    // The ties, unordered: the crossing, then the left and the right edge.
    let mut ties: Vec<JunctionTie> = Vec::new();
    let (sp, sq) = (report.legs[p].station, report.legs[q].station);
    let z0 = red(primary, sp).ok_or_else(|| uncovered(chosen[p]))?;
    ties.push(JunctionTie {
        pvi: String::new(),
        station: sq,
        primary_station: sp,
        primary_offset: 0.0,
        crossfall: 0.0,
        elevation: z0,
        before: 0.0,
        difference: 0.0,
    });
    for (k, station) in edges.iter().enumerate() {
        let Some(station) = *station else {
            let side = if k == 0 { "sol" } else { "sağ" };
            return Err(bad(format!("Tali yolun ekseni ana yolun {side} kenarını kesmiyor."), "junction"));
        };
        let place = secondary.alignment.core.at(station).ok_or_else(|| uncovered(chosen[q]))?;
        let foot = primary.alignment.core.locate([place.x, place.y]).nearest.ok_or_else(|| uncovered(chosen[p]))?;
        let slope = slopes(foot.at.station);
        let (slope, width) = if k == 0 { (slope.left, deck.left) } else { (slope.right, deck.right) };
        let z = red(primary, foot.at.station).ok_or_else(|| uncovered(chosen[p]))? + slope * width;
        ties.push(JunctionTie {
            pvi: String::new(),
            station,
            primary_station: foot.at.station,
            primary_offset: foot.offset,
            crossfall: slope,
            elevation: z,
            before: 0.0,
            difference: 0.0,
        });
    }
    ties.sort_by(|a, b| a.station.total_cmp(&b.station));
    let sdef = chosen[q];
    let (lo, hi) = (ties[0].station - 1e-6, ties[2].station + 1e-6);
    let (Some(first), Some(last)) = (sdef.pvis.first(), sdef.pvis.last()) else { return Err(uncovered(sdef)) };
    if first.station >= lo || last.station <= hi {
        return Err(bad(
            format!("“{}” boykesiti bağı örtmüyor: ilk ve son PVI'sı ana yolun iki kenarının dışında olmalı.", sdef.name),
            "profiles",
        ));
    }
    for (t, suffix) in ties.iter_mut().zip(["a", "x", "b"]) {
        t.pvi = format!("{}.{suffix}", def.id);
        t.before = red(secondary, t.station).ok_or_else(|| uncovered(sdef))?;
        t.difference = t.elevation - t.before;
    }
    let inside = |v: &ProfilePvi| v.station >= lo && v.station <= hi;
    let removed: Vec<String> = sdef.pvis.iter().filter(|v| inside(v)).map(|v| v.id.clone()).collect();
    for t in &ties {
        if sdef.pvis.iter().any(|v| !inside(v) && v.id == t.pvi) {
            return Err(bad(format!("“{}” kimlikli PVI boykesitte zaten var.", t.pvi), "profiles"));
        }
    }
    let new: Vec<ProfilePvi> = ties
        .iter()
        .map(|t| ProfilePvi {
            id: t.pvi.clone(),
            station: t.station,
            elevation: t.elevation,
            length: 0.0,
            length_in: 0.0,
            length_out: 0.0,
            radius: 0.0,
        })
        .collect();
    let before: Vec<&ProfilePvi> = sdef.pvis.iter().filter(|v| v.station < lo).collect();
    let after: Vec<&ProfilePvi> = sdef.pvis.iter().filter(|v| v.station > hi).collect();
    let mut tied = sdef.clone();
    tied.pvis = before.iter().map(|v| (*v).clone()).chain(new.iter().cloned()).chain(after.iter().map(|v| (*v).clone())).collect();
    tied.revision = sdef.revision.saturating_add(1);
    let mut list = profiles::profiles(doc).to_vec();
    if let Some(slot) = list.iter_mut().find(|x| x.id == sdef.id) {
        *slot = tied.clone();
    }
    profiles::judged(doc, &list)?;
    profiles::build(doc, &tied)?;
    // The tangent grades around and between the ties, and the change at each.
    let (prev, next) = (before.last().copied(), after.first().copied());
    let points: Vec<(f64, f64)> = prev
        .map(|v| (v.station, v.elevation))
        .into_iter()
        .chain(ties.iter().map(|t| (t.station, t.elevation)))
        .chain(next.map(|v| (v.station, v.elevation)))
        .collect();
    let grades: Vec<f64> = points.windows(2).map(|w| (w[1].1 - w[0].1) / (w[1].0 - w[0].0)).collect();
    let changes: Vec<f64> = grades.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    if let Some(most) = input.max_grade_change {
        for (t, c) in ties.iter().zip(&changes) {
            if *c > most {
                warnings.push(CommandWarning {
                    code: codes::JUNCTION_GRADE_CHANGE.into(),
                    message: format!(
                        "“{}” PVI'sinde eğim değişimi %{:.3}, en çok %{:.3} istendi.",
                        t.pvi,
                        100.0 * c,
                        100.0 * most
                    ),
                    path: Some("maxGradeChange".into()),
                });
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked {
        list,
        tied: JunctionsTied {
            junction: def.id.clone(),
            primary: input.primary,
            profile: sdef.id.clone(),
            ties,
            removed,
            grades,
            changes,
            revision: doc.revision().to_string(),
        },
        warnings,
    })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &JunctionsTie) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: (), warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &JunctionsTie) -> CommandResult<JunctionsTied> {
    match check(cx.doc, input) {
        Ok(c) => CommandResult::Completed { output: c.tied, warnings: c.warnings },
        Err(stop) => stop.into(),
    }
}

/// Writes the tie: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: JunctionsTie) -> CommandResult<JunctionsTied> {
    crate::call::recorded(cx, kentos_commands::ROAD_JUNCTIONS_TIE, kentos_commands::ROAD_JUNCTIONS_TIE_VERSION, input, write)
}

fn write(cx: &mut ExecutionContext<'_>, input: JunctionsTie) -> CommandResult<JunctionsTied> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let doc = &mut *cx.doc;
    if let Err(r) = doc.set_road_profiles(c.list, STEP) {
        return fail(codes::INVALID_ROAD_PROFILE, format!("Boykesit kurallarına uymuyor: {}.", r.0), "").into();
    }
    let mut tied = c.tied;
    tied.revision = doc.revision().to_string();
    CommandResult::Completed { output: tied, warnings: c.warnings }
}

/// The tie for a window, or why it does not hold.
pub fn preview(doc: &Document, input: &JunctionsTie) -> Result<(JunctionsTied, Vec<CommandWarning>), String> {
    check(doc, input).map(|c| (c.tied, c.warnings)).map_err(crate::road_corridors::words)
}
