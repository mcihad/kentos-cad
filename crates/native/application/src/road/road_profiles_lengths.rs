//! `road.profiles.lengths` v1 (plan/road.md Y31, docs/adr/0935 §2): each vertical curve's least length from the
//! stopping and passing sight, the headlights, comfort and the bisector (`kentos_road::profile::lengths`), with the
//! user's values. Reads only. The shared cases are `fixtures/commands/v1/road.profiles.lengths.json`.

use kentos_commands::{CommandResult, CommandWarning, CurveCriterion, CurveLengths, RoadProfileLengths, RoadProfilesLengths};
use kentos_domain::Document;
use kentos_road::profile::lengths as len;
use kentos_road::profile::vertical::VCurve;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_profiles as common;
use crate::road_terrains::fail;

fn bad(path: &str, words: &str) -> Stop {
    fail(codes::INVALID_PROFILE_LENGTHS, words.into(), path)
}

/// The input's own values judged.
fn judged(q: &RoadProfilesLengths) -> Result<(), Stop> {
    if !q.speed.is_none_or(|v| v.is_finite() && v > 0.0) {
        return Err(bad("speed", "Proje hızı 0'dan büyük olmalı (km/sa)."));
    }
    let positive = |v: Option<f64>| v.is_none_or(|x| x.is_finite() && x > 0.0);
    for (v, path, words) in [
        (q.time, "time", "Tepki süresi 0'dan büyük olmalı (s)."),
        (q.decel, "decel", "Yavaşlama 0'dan büyük olmalı (m/s²)."),
        (q.eye, "eye", "Göz yüksekliği 0'dan büyük olmalı (m)."),
        (q.object, "object", "Engel yüksekliği 0'dan büyük olmalı (m)."),
        (q.passing, "passing", "Geçme görüş mesafesi 0'dan büyük olmalı (m)."),
        (q.headlight, "headlight", "Far yüksekliği 0'dan büyük olmalı (m)."),
        (q.accel, "accel", "Düşey ivme 0'dan büyük olmalı (m/s²)."),
        (q.external, "external", "Bisektris 0'dan büyük olmalı (m)."),
        (q.step, "step", "Yuvarlama adımı 0'dan büyük olmalı (m)."),
    ] {
        if !positive(v) {
            return Err(bad(path, words));
        }
    }
    if !q.beam.is_none_or(|b| b.is_finite() && (0.0..45.0).contains(&b)) {
        return Err(bad("beam", "Far ışınının açısı 0 ile 45 derece arası olmalı."));
    }
    let crest = q.eye.is_some() || q.object.is_some();
    let sag = q.headlight.is_some() || q.beam.is_some();
    if crest && (q.eye.is_none() || q.object.is_none()) {
        return Err(bad("eye", "Tepe görüşü göz ve engel yüksekliğini birlikte ister."));
    }
    if sag && (q.headlight.is_none() || q.beam.is_none()) {
        return Err(bad("headlight", "Far görüşü far yüksekliğini ve ışın açısını birlikte ister."));
    }
    if (crest || sag) && (q.time.is_none() || q.decel.is_none()) {
        return Err(bad("time", "Durma görüşü tepki süresini (time) ve yavaşlamayı (decel) ister."));
    }
    if q.passing.is_some() && q.eye.is_none() {
        return Err(bad("passing", "Geçme görüşü göz yüksekliğini (eye) ister: karşıdan gelen aracın yüksekliği de odur."));
    }
    if !crest && !sag && q.passing.is_none() && q.accel.is_none() && q.external.is_none() {
        return Err(bad(
            "speed",
            "Bir ölçüt verin: tepe için göz ve engel, çukur için far, konfor için düşey ivme ya da bisektris.",
        ));
    }
    Ok(())
}

/// The answer now and its warnings.
pub(crate) fn answer(doc: &Document, q: &RoadProfilesLengths) -> Result<(RoadProfileLengths, Vec<CommandWarning>), Stop> {
    judged(q)?;
    common::road_project(doc)?;
    let (_, def) = common::find(common::profiles(doc), &q.profile, "profile")?;
    lengths_def(doc, def, q)
}

fn lengths_def(
    doc: &Document,
    def: &kentos_commands::ProfileDef,
    q: &RoadProfilesLengths,
) -> Result<(RoadProfileLengths, Vec<CommandWarning>), Stop> {
    // The speed given, else the alignment's design speed (docs/adr/0986).
    let speed = q.speed.or_else(|| crate::road_alignments::design_speed(doc, &def.alignment)).ok_or_else(|| {
        bad("speed", "Görüş ve konfor bir hızda okunur: proje hızını (speed) verin ya da aksa proje hızı yazın.")
    })?;
    let built = common::build(doc, def)?;
    let stopping_at = |grade: f64| match (q.time, q.decel) {
        (Some(t), Some(a)) => len::stopping(speed, t, a, grade),
        _ => None,
    };
    let level = stopping_at(0.0);
    let mut curves = Vec::new();
    // Every PVI between two others, a kink without a curve too (it asks for one most): the grades from the PVIs.
    let pvis = common::pvis_of(def);
    for w in pvis.windows(3) {
        let (p0, p, p1) = (&w[0], &w[1], &w[2]);
        let g_in = (p.elevation - p0.elevation) / (p.station - p0.station);
        let g_out = (p1.elevation - p.elevation) / (p1.station - p.station);
        let a = 100.0 * (g_out - g_in);
        if a == 0.0 {
            continue;
        }
        let crest = g_out < g_in;
        let length = match p.curve {
            VCurve::None => None,
            VCurve::Parabola { length } => Some(length),
            VCurve::Asymmetric { length_in, length_out } => Some(length_in + length_out),
            VCurve::Circle { radius } => Some(radius * a.abs() / 100.0),
        };
        // The stopping distance on each approach: ahead on the grade in, back on the grade out (its sign turned);
        // the longer of the two. On the level without `grades`.
        let s = if q.grades {
            match (stopping_at(g_in), stopping_at(-g_out)) {
                (Some(x), Some(y)) => Some(x.max(y)),
                // A downgrade steeper than the braking: no stopping distance there, no sight criterion.
                _ => None,
            }
        } else {
            level
        };
        let mut criteria = Vec::new();
        if crest {
            if let (Some(eye), Some(object), Some(s)) = (q.eye, q.object, s) {
                criteria.push(CurveCriterion { kind: "stopping".into(), sight: Some(s), length: len::crest(a, s, eye, object) });
            }
            if let (Some(ps), Some(eye)) = (q.passing, q.eye) {
                criteria.push(CurveCriterion { kind: "passing".into(), sight: Some(ps), length: len::crest(a, ps, eye, eye) });
            }
        } else {
            if let (Some(h), Some(b), Some(s)) = (q.headlight, q.beam, s) {
                criteria.push(CurveCriterion { kind: "headlight".into(), sight: Some(s), length: len::headlight(a, s, h, b) });
            }
            if let Some(acc) = q.accel {
                criteria.push(CurveCriterion { kind: "comfort".into(), sight: None, length: len::comfort(a, speed, acc) });
            }
        }
        if let Some(e) = q.external {
            criteria.push(CurveCriterion { kind: "bisector".into(), sight: None, length: len::bisector(a, e) });
        }
        let governing = criteria.iter().max_by(|x, y| x.length.total_cmp(&y.length));
        let required = governing.map_or(0.0, |g| g.length);
        let suggested = match q.step {
            Some(step) if required > 0.0 => (required / step - 1e-9).ceil() * step,
            _ => required,
        };
        curves.push(CurveLengths {
            pvi: p.id.clone(),
            crest,
            a,
            length,
            governing: governing.filter(|g| g.length > 0.0).map(|g| g.kind.clone()),
            criteria,
            required,
            suggested,
            short: length.unwrap_or(0.0) < required - 1e-9,
        });
    }
    let mut warnings = Vec::new();
    let report = common::report(doc, def, &built);
    if report.stale {
        warnings.push(CommandWarning {
            code: codes::PROFILE_STALE.into(),
            message: format!(
                "“{}” boykesitinin aksı değişti (boykesit {}. revizyona göre kurulu, aks şimdi {}.); PVI'ları denetleyip aksa yeniden bağlayın.",
                report.name, report.alignment_revision, report.alignment_now
            ),
            path: Some("profile".into()),
        });
    }
    let short: Vec<&str> = curves.iter().filter(|c| c.short).map(|c| c.pvi.as_str()).collect();
    if !short.is_empty() {
        warnings.push(CommandWarning {
            code: codes::VERTICAL_CURVE_SHORT.into(),
            message: format!("“{}” boykesitinde şu kurplar gereken boydan kısa: {}.", def.name, short.join(", ")),
            path: Some("profile".into()),
        });
    }
    Ok((RoadProfileLengths { profile: def.id.clone(), stopping: level, curves }, warnings))
}

/// Checks `q` against the document.
pub fn validate(cx: &ExecutionContext<'_>, q: &RoadProfilesLengths) -> CommandResult<()> {
    match answer(cx.doc, q) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, q: &RoadProfilesLengths) -> CommandResult<RoadProfileLengths> {
    match answer(cx.doc, q) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, q: RoadProfilesLengths) -> CommandResult<RoadProfileLengths> {
    plan(cx, &q)
}

/// The answer for a profile as typed (the desktop's Boykesit window, before it is saved).
pub fn preview(
    doc: &Document,
    def: &kentos_commands::ProfileDef,
    q: &RoadProfilesLengths,
) -> Result<(RoadProfileLengths, Vec<CommandWarning>), String> {
    let words = |stop: Stop| match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.message,
        Stop::Cancelled => String::new(),
    };
    judged(q).map_err(words)?;
    common::road_project(doc).map_err(words)?;
    lengths_def(doc, def, q).map_err(words)
}
