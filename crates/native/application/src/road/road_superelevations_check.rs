//! `road.superelevations.check` v1 (plan/road.md Y27, docs/adr/0931 §2): a superelevation schedule's relative gradients
//! about its axis of rotation and each curve's side friction at a design speed, against the user's limits
//! (`kentos_road::superelevation_physics`). Reads only. The shared cases are
//! `fixtures/commands/v1/road.superelevations.check.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CurveFrictionRow, RelativeGradient, SuperelevationFinding, SuperelevationPivot,
    SuperelevationsCheck, SuperelevationsChecked,
};
use kentos_domain::Document;
use kentos_road::superelevation_physics::{self as phys, Way};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_superelevations as common;
use crate::road_terrains::fail;

/// The check now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &SuperelevationsCheck) -> Result<(SuperelevationsChecked, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let bad = |path: &str, words: &str| fail(codes::INVALID_SUPERELEVATION_CHECK, words.into(), path);
    let positive = |v: Option<f64>| v.is_none_or(|x| x.is_finite() && x > 0.0);
    if !positive(input.speed) {
        return Err(bad("speed", "Proje hızı 0'dan büyük olmalı (km/sa)."));
    }
    if !positive(input.most_friction) || input.most_friction.is_some_and(|f| f >= 1.0) {
        return Err(bad("mostFriction", "En büyük yanal sürtünme 0 ile 1 arası olmalı."));
    }
    if !positive(input.most_relative) || input.most_relative.is_some_and(|g| g >= 1.0) {
        return Err(bad("mostRelative", "En büyük göreli eğim 0 ile 1 arası olmalı (oran; 0,005 = 1:200)."));
    }
    let step = input.step.unwrap_or(1.0);
    if !(step.is_finite() && step > 0.0 && step <= 100.0) {
        return Err(bad("step", "Klotoidin okunma adımı 0'dan büyük, en çok 100 m olmalı."));
    }
    let (_, def) = common::find(doc.road_superelevations(), &input.superelevation, "superelevation")?;
    check_def(doc, def, input, step)
}

/// The check of a schedule as typed (the desktop's Dever window, before it is saved): `input`'s limits on `def`.
pub fn preview(doc: &Document, def: &kentos_commands::SuperelevationDef, input: &SuperelevationsCheck) -> Result<(SuperelevationsChecked, Vec<CommandWarning>), String> {
    let words = |stop: Stop| match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.message,
        Stop::Cancelled => String::new(),
    };
    common::road_project(doc).map_err(words)?;
    check_def(doc, def, input, input.step.unwrap_or(1.0)).map_err(words)
}

fn check_def(
    doc: &Document,
    def: &kentos_commands::SuperelevationDef,
    input: &SuperelevationsCheck,
    step: f64,
) -> Result<(SuperelevationsChecked, Vec<CommandWarning>), Stop> {
    let bad = |path: &str, words: &str| fail(codes::INVALID_SUPERELEVATION_CHECK, words.into(), path);
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
    let (schedule, _) = common::schedule(def, &built)?;
    let normal = common::normal(def);
    // The traveled way: the sides' slopes (the schedule's, the normal where it has none) and widths (lanes + widening).
    let slope = |pts: &[[f64; 2]], n: f64| if pts.is_empty() { vec![[0.0, n]] } else { pts.to_vec() };
    let width = |pts: &[[f64; 2]], w: f64| if pts.is_empty() { vec![[0.0, w]] } else { pts.iter().map(|p| [p[0], w + p[1]]).collect() };
    let way = Way {
        left_slope: slope(&schedule.left, normal.left),
        right_slope: slope(&schedule.right, normal.right),
        left_width: width(&schedule.left_widening, wl),
        right_width: width(&schedule.right_widening, wr),
    };
    let length = built.core.length;
    let gradients: Vec<RelativeGradient> = phys::relative_gradients(&way, schedule.pivot, 0.0, length)
        .into_iter()
        .map(|g| RelativeGradient { from: g.from, to: g.to, left: g.left, right: g.right })
        .collect();
    // Each curve at the speed given, else at its own design speed (docs/adr/0986): its PI's, else its alignment's.
    let own = crate::road_alignments::curve_speeds(doc, &def.alignment);
    if input.most_friction.is_some() && input.speed.is_none() && own.is_empty() {
        return Err(bad(
            "speed",
            "Yanal sürtünme bir hızda okunur: proje hızını (speed) verin ya da aksa proje hızı yazın.",
        ));
    }
    let speed_of = |pi: &str| input.speed.or_else(|| own.get(pi).copied());
    let mut speeds: Vec<f64> = match input.speed {
        Some(v) => vec![v],
        None => own.values().copied().collect(),
    };
    speeds.sort_by(f64::total_cmp);
    speeds.dedup();
    let mut curves: Vec<CurveFrictionRow> = speeds
        .iter()
        .flat_map(|&v| {
            phys::curve_friction(&built.core, &schedule, normal, v, step)
                .into_iter()
                .filter(move |c| speed_of(&c.pi) == Some(v))
                .map(|c| CurveFrictionRow { pi: c.pi, from: c.from, to: c.to, friction: c.most, at: c.at, radius: c.radius, e: c.e })
        })
        .collect();
    curves.sort_by(|a, b| a.from.total_cmp(&b.from));
    let mut findings = Vec::new();
    if let Some(limit) = input.most_relative {
        for g in &gradients {
            for (value, side) in [(g.left, "sol"), (g.right, "sağ")] {
                if value > limit + 1e-12 {
                    findings.push(SuperelevationFinding {
                        kind: "relative".into(),
                        from: g.from,
                        to: g.to,
                        value,
                        limit,
                        message: format!(
                            "{:.2} – {:.2} m: {side} kenarın göreli eğimi 1:{:.0}, en dik 1:{:.0}.",
                            g.from,
                            g.to,
                            1.0 / value,
                            1.0 / limit
                        ),
                    });
                }
            }
        }
    }
    if let Some(limit) = input.most_friction {
        for c in &curves {
            if c.friction > limit + 1e-12 {
                findings.push(SuperelevationFinding {
                    kind: "friction".into(),
                    from: c.from,
                    to: c.to,
                    value: c.friction,
                    limit,
                    message: format!(
                        "{} kurbu {:.0} km/sa'te yanal sürtünme {:.3} ister (km {:.2}, R {:.1} m, dever %{:.2}), en çok {:.3}.",
                        c.pi,
                        speed_of(&c.pi).unwrap_or(0.0),
                        c.friction,
                        c.at,
                        c.radius,
                        100.0 * c.e,
                        limit
                    ),
                });
            }
        }
    }
    findings.sort_by(|a, b| a.from.total_cmp(&b.from).then(a.kind.cmp(&b.kind)));
    let mut warnings = Vec::new();
    if !findings.is_empty() {
        warnings.push(CommandWarning {
            code: codes::SUPERELEVATION_LIMIT_BROKEN.into(),
            message: format!("“{}” dever çizelgesinde {} yer verilen sınırları aşıyor.", def.name, findings.len()),
            path: Some("superelevation".into()),
        });
    }
    let pivot = def.pivot.unwrap_or(SuperelevationPivot::Centerline);
    Ok((SuperelevationsChecked { superelevation: def.id.clone(), pivot, gradients, curves, findings, step }, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &SuperelevationsCheck) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &SuperelevationsCheck) -> CommandResult<SuperelevationsChecked> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SuperelevationsCheck) -> CommandResult<SuperelevationsChecked> {
    plan(cx, &input)
}
