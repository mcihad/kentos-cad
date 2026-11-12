//! `road.profiles.sight` v1 (docs/adr/0921; ROAD-PROFILE-08): a profile's sight distance at stations, ahead and
//! behind, an eye and an object over the profile (`kentos_road::profile`'s sight). Reads only. The shared cases are
//! `fixtures/commands/v1/road.profiles.sight.json`.

use kentos_commands::{
    CommandResult, CommandWarning, MAX_ALIGNMENT_PLACES, ProfilesSight, ProfilesSightRead, SightLeast, SightReach,
    SightRow, SightShort, SightSide, SightUntil,
};
use kentos_domain::Document;
use kentos_road::profile::{SIGHT_RESOLUTION, Seen, Until};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_profiles as common;
use crate::road_terrains::fail;

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_SIGHT, message, path)
}

fn reach_of(s: Seen) -> SightReach {
    SightReach {
        distance: s.distance,
        until: match s.until {
            Until::Hidden => SightUntil::Hidden,
            Until::Reach => SightUntil::Reach,
            Until::End => SightUntil::End,
        },
        blocked: s.blocked,
    }
}

fn least(rows: &[SightRow], pick: impl Fn(&SightRow) -> Option<&SightReach>) -> Option<SightLeast> {
    rows.iter()
        .filter_map(|r| pick(r).filter(|x| x.until == SightUntil::Hidden).map(|x| (r.station, x.distance)))
        .fold(None, |best: Option<(f64, f64)>, (s, d)| match best {
            Some((_, b)) if b <= d => best,
            _ => Some((s, d)),
        })
        .map(|(station, distance)| SightLeast { station, distance })
}

fn shorts(rows: &[SightRow], side: SightSide, required: f64, pick: impl Fn(&SightRow) -> Option<&SightReach>) -> Vec<SightShort> {
    let mut out: Vec<SightShort> = Vec::new();
    let mut open = false;
    for r in rows {
        // Short only where the profile hides the object: the end of the profile or of the reach is not seen past.
        match pick(r).filter(|x| x.until == SightUntil::Hidden && x.distance < required) {
            Some(x) => {
                if open && let Some(last) = out.last_mut() {
                    last.to = r.station;
                    last.least = last.least.min(x.distance);
                } else {
                    out.push(SightShort { side, from: r.station, to: r.station, least: x.distance });
                }
                open = true;
            }
            None => open = false,
        }
    }
    out
}

/// The sight now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &ProfilesSight) -> Result<(ProfilesSightRead, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(common::profiles(doc), &input.profile, "profile")?;
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if !positive(input.eye) {
        return Err(invalid("Göz yüksekliği (eye) 0'dan büyük olmalı.".into(), "eye"));
    }
    if !positive(input.object) {
        return Err(invalid(
            "Nesne yüksekliği (object) 0'dan büyük olmalı: profilin üstündeki nokta her tepede çizgiye teğettir.".into(),
            "object",
        ));
    }
    for (v, name, path) in [(input.reach, "Bakılan uzaklık", "reach"), (input.step, "Adım", "step")] {
        if !positive(v) {
            return Err(invalid(format!("{name} ({path}) 0'dan büyük olmalı."), path));
        }
    }
    if input.required.is_some_and(|r| !positive(r)) {
        return Err(invalid("Gereken görüş mesafesi (required) 0'dan büyük olmalı.".into(), "required"));
    }
    let built = common::build(doc, def)?;
    let v = &built.vertical;
    let from = input.from.unwrap_or(v.start);
    let to = input.to.unwrap_or(v.end);
    let on = |s: f64| s.is_finite() && s >= v.start - 1e-9 && s <= v.end + 1e-9;
    if !(on(from) && on(to) && from < to) {
        return Err(invalid(
            format!("Km aralığı boykesitin üstünde ve ileri olmalı: boykesit {:.3} ile {:.3} m arası.", v.start, v.end),
            if on(from) { "to" } else { "from" },
        ));
    }
    let (from, to) = (from.clamp(v.start, v.end), to.clamp(v.start, v.end));
    let count = ((to - from) / input.step).ceil() + 1.0;
    if count > MAX_ALIGNMENT_PLACES as f64 {
        return Err(invalid(format!("Bir soruda en çok {MAX_ALIGNMENT_PLACES} km olur; adımı büyütün."), "step"));
    }
    let mut stations: Vec<f64> = Vec::new();
    let mut k = 0.0;
    loop {
        let s = from + k * input.step;
        if s >= to - 1e-9 {
            break;
        }
        stations.push(s);
        k += 1.0;
    }
    stations.push(to);
    let (ahead, behind) = match input.side {
        SightSide::Forward => (true, false),
        SightSide::Backward => (false, true),
        SightSide::Both => (true, true),
    };
    let rows: Vec<SightRow> = stations
        .iter()
        .map(|&s| {
            let look = |forward: bool| v.sight(s, forward, input.eye, input.object, input.reach, input.step).map(reach_of);
            SightRow {
                station: s,
                elevation: v.at(s).map_or(f64::NAN, |p| p.0),
                forward: if ahead { look(true) } else { None },
                backward: if behind { look(false) } else { None },
            }
        })
        .collect();
    let mut short = Vec::new();
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
    if let Some(required) = input.required {
        short.extend(shorts(&rows, SightSide::Forward, required, |r| r.forward.as_ref()));
        short.extend(shorts(&rows, SightSide::Backward, required, |r| r.backward.as_ref()));
        if let Some(worst) = short.iter().min_by(|a, b| a.least.total_cmp(&b.least)) {
            warnings.push(CommandWarning {
                code: codes::SIGHT_SHORT.into(),
                message: format!(
                    "“{}” boykesitinde {} aralıkta görüş {required} m'den kısa; en kısası {:.1} m ({:.1} – {:.1} m arası).",
                    def.name,
                    short.len(),
                    worst.least,
                    worst.from,
                    worst.to
                ),
                path: Some("required".into()),
            });
        }
    }
    Ok((
        ProfilesSightRead {
            profile: def.id.clone(),
            eye: input.eye,
            object: input.object,
            reach: input.reach,
            step: input.step,
            resolution: SIGHT_RESOLUTION,
            least_forward: least(&rows, |r| r.forward.as_ref()),
            least_backward: least(&rows, |r| r.backward.as_ref()),
            rows,
            short,
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &ProfilesSight) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &ProfilesSight) -> CommandResult<ProfilesSightRead> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ProfilesSight) -> CommandResult<ProfilesSightRead> {
    plan(cx, &input)
}

/// The sight for a window, or why it does not read.
pub fn preview(doc: &Document, input: &ProfilesSight) -> Result<(ProfilesSightRead, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
