//! `road.superelevations.transitions` v1 (plan/road.md Y34, docs/adr/0940): each curve's transition proposed for a
//! superelevation schedule made from its curves: the runoff by the relative gradient (read on the curve's own
//! transition about the schedule's axis of rotation, its widening in it), the clothoid by comfort and by its shift, the
//! runout and the length held at full superelevation, and whether consecutive curves' transitions fit the tangent
//! between them (`kentos_road::superelevation_transition`). Reads only: the desktop's Dever window writes a proposal into
//! the schedule through `road.superelevations.edit`. The shared cases are
//! `fixtures/commands/v1/road.superelevations.transitions.json`.

use kentos_commands::{
    CommandResult, CommandWarning, SuperCurveDef, SuperelevationDef, SuperelevationMethod, SuperelevationsTransitioned,
    SuperelevationsTransitions, SuperTransitionFinding, SuperTransitionRow,
};
use kentos_domain::Document;
use kentos_road::superelevation_physics::{self as phys, Way};
use kentos_road::superelevation_transition as tr;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_superelevations as common;
use crate::road_terrains::fail;

fn bad(path: &str, words: &str) -> Stop {
    fail(codes::INVALID_SUPERELEVATION_TRANSITION, words.into(), path)
}

/// The input's own values judged, before the drawing is read.
fn judged(input: &SuperelevationsTransitions) -> Result<(), Stop> {
    let positive = |v: Option<f64>| v.is_none_or(|x| x.is_finite() && x > 0.0);
    if !positive(input.most_relative) || input.most_relative.is_some_and(|g| g >= 1.0) {
        return Err(bad("mostRelative", "En büyük göreli eğim 0 ile 1 arası olmalı (oran; 0,005 = 1:200)."));
    }
    if !positive(input.speed) {
        return Err(bad("speed", "Proje hızı 0'dan büyük olmalı (km/sa)."));
    }
    if !positive(input.comfort) {
        return Err(bad("comfort", "Konfor ölçütü (ivmenin değişme hızı) 0'dan büyük olmalı (m/s³)."));
    }
    if !positive(input.shift) || !positive(input.most_shift) {
        return Err(bad("shift", "Klotoidin kaydırması 0'dan büyük olmalı (m)."));
    }
    if let (Some(a), Some(b)) = (input.shift, input.most_shift)
        && b < a
    {
        return Err(bad("mostShift", "En büyük kaydırma en küçükten küçük olamaz."));
    }
    if input.on_tangent.is_some_and(|t| !(t.is_finite() && (0.0..=1.0).contains(&t))) {
        return Err(bad("onTangent", "Geçişin teğetteki payı 0 ile 1 arası olmalı."));
    }
    if input.step.is_some_and(|s| !(s.is_finite() && s > 0.0 && s <= 100.0)) {
        return Err(bad("step", "Yuvarlama adımı 0'dan büyük, en çok 100 m olmalı."));
    }
    Ok(())
}

/// The proposal now, and its warnings.
pub(crate) fn answer(
    doc: &Document,
    input: &SuperelevationsTransitions,
) -> Result<(SuperelevationsTransitioned, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    judged(input)?;
    let (_, def) = common::find(doc.road_superelevations(), &input.superelevation, "superelevation")?;
    propose(doc, def, input)
}

/// The proposal for a schedule as typed (the desktop's Dever window, before it is saved).
pub fn preview(
    doc: &Document,
    def: &SuperelevationDef,
    input: &SuperelevationsTransitions,
) -> Result<(SuperelevationsTransitioned, Vec<CommandWarning>), String> {
    let words = |stop: Stop| match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.message,
        Stop::Cancelled => String::new(),
    };
    common::road_project(doc).map_err(words)?;
    judged(input).map_err(words)?;
    propose(doc, def, input).map_err(words)
}

/// What a curve's transition is: its runoff and share on the tangent, as proposed or as it is.
struct Taken {
    runoff: f64,
    share: f64,
    normal: f64,
    rate: f64,
}

fn propose(
    doc: &Document,
    def: &SuperelevationDef,
    input: &SuperelevationsTransitions,
) -> Result<(SuperelevationsTransitioned, Vec<CommandWarning>), Stop> {
    let SuperelevationMethod::Curves { curves } = &def.method else {
        return Err(bad("superelevation", "Geçiş önerisi kurplardan kurulan dever çizelgesinde olur; bu çizelge km satırlarıyla yazılmış."));
    };
    // A curve's comfort at the speed given, else at its own design speed (docs/adr/0986).
    let own = crate::road_alignments::curve_speeds(doc, &def.alignment);
    if input.comfort.is_some() && input.speed.is_none() && own.is_empty() {
        return Err(bad(
            "speed",
            "Konforlu klotoid bir hızda okunur: proje hızını (speed) verin ya da aksa proje hızı yazın.",
        ));
    }
    let template = doc
        .road_templates()
        .iter()
        .find(|t| t.id == input.template || t.name.to_lowercase() == input.template.to_lowercase())
        .ok_or_else(|| {
            fail(codes::UNKNOWN_ROAD_TEMPLATE, format!("“{}” kimlikli ya da adlı tip kesit projede yok.", input.template), "template")
        })?;
    let core = crate::road_sections::core_template(template);
    let (wl, wr) = crate::road_sections::lane_widths(&core);
    if wl <= 0.0 || wr <= 0.0 {
        return Err(bad("template", "Tip kesitin iki yanında da şerit olmalı: göreli eğim şerit kenarında okunur."));
    }
    let built = common::alignment_of(doc, def)?;
    let normal = common::normal(def);
    let mut rows: Vec<(SuperTransitionRow, Taken)> = Vec::new();
    for rule in curves {
        let Some(c) = built.core.curves.iter().find(|c| c.pi == rule.pi) else { continue };
        let gradient_now = gradient_of(def, rule, &built, normal, (wl, wr));
        let runoff_gradient = match (input.most_relative, gradient_now) {
            (Some(most), Some(g)) if g > 0.0 => Some(tr::runoff_for(rule.runoff, g, most)),
            _ => None,
        };
        let e = rule.rate;
        let speed = input.speed.or_else(|| own.get(&c.pi).copied());
        let spiral_comfort = speed.zip(input.comfort).map(|(v, k)| tr::comfort_spiral(v, c.radius, e, k));
        let spiral_shift = input.shift.map(|p| tr::shift_spiral(p, c.radius));
        let spiral_most = input.most_shift.map(|p| tr::shift_spiral(p, c.radius));
        let spiral = match (spiral_comfort, spiral_shift) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        let clothoid = c.spiral_in > 0.0 || c.spiral_out > 0.0;
        // On a clothoid the runoff lies on it (AASHTO): at least its length, nothing before the curve.
        let base = if clothoid { Some(runoff_gradient.unwrap_or(0.0).max(c.spiral_in.max(c.spiral_out))) } else { runoff_gradient };
        let runoff = base.map(|v| tr::round_up(v, input.step));
        let on_tangent = if clothoid { Some(0.0) } else { input.on_tangent };
        // Turning left the right side is outside.
        let n_out = if c.delta > 0.0 { normal.right } else { normal.left };
        let taken = Taken {
            runoff: runoff.unwrap_or(rule.runoff),
            share: on_tangent.unwrap_or(rule.on_tangent),
            normal: n_out,
            rate: rule.rate,
        };
        let row = SuperTransitionRow {
            pi: rule.pi.clone(),
            from: c.stations[0],
            to: c.stations[3],
            radius: c.radius,
            rate: rule.rate,
            spiral_in: c.spiral_in,
            spiral_out: c.spiral_out,
            runoff_now: rule.runoff,
            on_tangent_now: rule.on_tangent,
            gradient_now,
            runoff_gradient,
            spiral_comfort,
            spiral_shift,
            spiral_most,
            spiral,
            runoff,
            on_tangent,
            runout: Some(tr::runout(taken.runoff, taken.normal, taken.rate)),
            full: Some(tr::full_length(c.stations[0], c.stations[3], taken.runoff, taken.share)),
        };
        rows.push((row, taken));
    }
    rows.sort_by(|a, b| a.0.from.total_cmp(&b.0.from));
    let mut findings = Vec::new();
    for (r, _) in &rows {
        if let Some(least) = r.spiral {
            for (len, side) in [(r.spiral_in, "giriş"), (r.spiral_out, "çıkış")] {
                if len > 0.0 && len < least - 1e-9 {
                    findings.push(SuperTransitionFinding {
                        kind: "spiralShort".into(),
                        pi: r.pi.clone(),
                        value: len,
                        limit: least,
                        message: format!("{} kurbunun {side} klotoidi {len:.2} m; konfor ve kaydırma en az {least:.2} m ister.", r.pi),
                    });
                }
            }
        }
        if let Some(most) = r.spiral_most {
            for (len, side) in [(r.spiral_in, "giriş"), (r.spiral_out, "çıkış")] {
                if len > most + 1e-9 {
                    findings.push(SuperTransitionFinding {
                        kind: "spiralLong".into(),
                        pi: r.pi.clone(),
                        value: len,
                        limit: most,
                        message: format!("{} kurbunun {side} klotoidi {len:.2} m; en büyük kaydırma en çok {most:.2} m'ye izin verir.", r.pi),
                    });
                }
            }
        }
        if let Some(full) = r.full
            && full < -1e-9
        {
            findings.push(SuperTransitionFinding {
                kind: "full".into(),
                pi: r.pi.clone(),
                value: full,
                limit: 0.0,
                message: format!("{} kurbu tam devere varamıyor: giriş ve çıkış geçişleri kurpta {:.2} m üst üste biniyor.", r.pi, -full),
            });
        }
    }
    for w in rows.windows(2) {
        let ((a, ta), (b, tb)) = (&w[0], &w[1]);
        let room = b.from - a.to;
        let need = tr::tangent_need(ta.runoff, ta.share, ta.normal, ta.rate) + tr::tangent_need(tb.runoff, tb.share, tb.normal, tb.rate);
        if need > room + 1e-9 {
            findings.push(SuperTransitionFinding {
                kind: "tangent".into(),
                pi: format!("{}–{}", a.pi, b.pi),
                value: need,
                limit: room,
                message: format!(
                    "{} ile {} arasındaki {room:.2} m teğete iki kurbun geçişleri {need:.2} m ister ({:.2} m eksik).",
                    a.pi,
                    b.pi,
                    need - room
                ),
            });
        }
    }
    let mut warnings = Vec::new();
    if findings.iter().any(|f| f.kind == "full" || f.kind == "tangent") {
        warnings.push(CommandWarning {
            code: codes::SUPERELEVATION_TRANSITION_CONFLICT.into(),
            message: format!("“{}” dever çizelgesine önerilen geçişler kurulamıyor; bulgular yerlerini söyler.", def.name),
            path: Some("superelevation".into()),
        });
    }
    Ok((
        SuperelevationsTransitioned { superelevation: def.id.clone(), rows: rows.into_iter().map(|r| r.0).collect(), findings },
        warnings,
    ))
}

/// The largest relative gradient of a curve's own transition (the schedule with that curve alone, about its axis of
/// rotation, the lanes widened as it widens them); none where the curve alone does not build.
fn gradient_of(
    def: &SuperelevationDef,
    rule: &SuperCurveDef,
    built: &crate::road_alignments::Built,
    normal: kentos_road::superelevation::Crossfall,
    (wl, wr): (f64, f64),
) -> Option<f64> {
    let single = SuperelevationDef { method: SuperelevationMethod::Curves { curves: vec![rule.clone()] }, ..def.clone() };
    let (schedule, _) = common::schedule(&single, built).ok()?;
    let slope = |pts: &[[f64; 2]], n: f64| if pts.is_empty() { vec![[0.0, n]] } else { pts.to_vec() };
    let width = |pts: &[[f64; 2]], w: f64| if pts.is_empty() { vec![[0.0, w]] } else { pts.iter().map(|p| [p[0], w + p[1]]).collect() };
    let way = Way {
        left_slope: slope(&schedule.left, normal.left),
        right_slope: slope(&schedule.right, normal.right),
        left_width: width(&schedule.left_widening, wl),
        right_width: width(&schedule.right_widening, wr),
    };
    let most = phys::relative_gradients(&way, schedule.pivot, 0.0, built.core.length)
        .into_iter()
        .map(|g| g.left.max(g.right))
        .fold(0.0, f64::max);
    Some(most)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SuperelevationsTransitions) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &SuperelevationsTransitions) -> CommandResult<SuperelevationsTransitioned> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SuperelevationsTransitions) -> CommandResult<SuperelevationsTransitioned> {
    plan(cx, &input)
}
