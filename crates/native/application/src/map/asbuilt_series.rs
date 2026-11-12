//! A monitoring job's series (`cad.asbuilt.check`'s `epochs`, docs/adr/4420; plan/map-cad.md MAP-19): the points
//! measured now tracked through the earlier epochs, Trimble Business Center's way: the first epoch gives a point's
//! starting place, every later one is told against it; a point is matched by name; its movement shows only where it
//! was measured in two epochs or more.
//!
//! The series is the earlier epochs, oldest first, then this one. A point's reference is the first epoch it was measured
//! in; at each epoch from it on its movement from the reference (east, north, height when both heights are there, and
//! the horizontal), flagged beyond the job's limits. A name found twice in an epoch is not matched there. With the days
//! of a point's reference and of this epoch, its mean rates: its last movement over the years between (a year
//! 365.25 days). A day is `YYYY-MM-DD`, a real one (the proleptic Gregorian calendar), told apart by its day number.
//!
//! Asked (docs/adr/4840): a line fitted through each point's dated epochs (the shared core's `survey::trend`: its
//! movement from its reference against the years since its first dated epoch, east, north and up apart; three such
//! epochs at least), the components that move at the confidence asked, and its state against two thresholds, warning
//! and alarm: the highest level its last movement or its speed (the line's, else the mean rate) goes beyond.

use kentos_commands::{
    AsbuiltCheck, AsbuiltFit, AsbuiltKind, AsbuiltLevel, AsbuiltState, AsbuiltStep, AsbuiltThresholds, AsbuiltTrack,
};
use kentos_domain::Document;
use kentos_geometry_core::survey::trend::{moves, velocity};

use crate::asbuilt_check::{Point, points};
use crate::checks::{Stop, error};
use crate::codes;

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// The day number of `YYYY-MM-DD` (days from 1970-01-01), a real day; none otherwise.
pub(crate) fn day_number(text: &str) -> Option<i64> {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let digits = |r: std::ops::Range<usize>| -> Option<i64> {
        text.get(r).filter(|s| s.bytes().all(|c| c.is_ascii_digit())).and_then(|s| s.parse().ok())
    };
    let (y, m, d) = (digits(0..4)?, digits(5..7)?, digits(8..10)?);
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1..=12).contains(&m) || d < 1 || d > days[(m - 1) as usize] {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// The series' own refusals (docs/adr/4420), after the job's: a series or a day outside a monitoring job; a day that is
/// no `YYYY-MM-DD` day.
pub(crate) fn asked(input: &AsbuiltCheck) -> Result<(), Stop> {
    let monitoring = input.kind == Some(AsbuiltKind::Monitoring);
    if !monitoring && (!input.epochs.is_empty() || input.date.is_some()) {
        let path = if input.epochs.is_empty() { "date" } else { "epochs" };
        return Err(refuse(
            codes::INVALID_ASBUILT_CHECK,
            "Dönem dizisi ve dönemin günü izleme işinindir (kind: monitoring).".into(),
            path,
        ));
    }
    let bad = |text: &str, path: &str| {
        refuse(
            codes::INVALID_ASBUILT_CHECK,
            format!("“{text}” bir gün değil; günü YYYY-AA-GG olarak yazın (ör. 2026-04-15)."),
            path,
        )
    };
    for (k, e) in input.epochs.iter().enumerate() {
        if let Some(d) = &e.date
            && day_number(d.trim()).is_none()
        {
            return Err(bad(d, &format!("epochs[{k}].date")));
        }
    }
    if let Some(d) = &input.date
        && day_number(d.trim()).is_none()
    {
        return Err(bad(d, "date"));
    }
    trend_asked(input)
}

/// The words of a threshold's limits, in the order they are checked.
const LIMITS: [(&str, &str); 4] = [
    ("horizontal", "Yatay hareketin"),
    ("vertical", "Düşey hareketin"),
    ("horizontalRate", "Yatay hızın"),
    ("verticalRate", "Düşey hızın"),
];

fn limit_of(level: &AsbuiltLevel, key: &str) -> Option<f64> {
    match key {
        "horizontal" => level.horizontal,
        "vertical" => level.vertical,
        "horizontalRate" => level.horizontal_rate,
        _ => level.vertical_rate,
    }
}

/// Docs/adr/4840's refusals: a trend or thresholds without a series, a confidence not between 0 and 1, a limit not
/// above zero, a warning not below its alarm.
fn trend_asked(input: &AsbuiltCheck) -> Result<(), Stop> {
    let bad = |message: String, path: &str| Err(refuse(codes::INVALID_ASBUILT_CHECK, message, path));
    let series = input.kind == Some(AsbuiltKind::Monitoring) && !input.epochs.is_empty();
    for (asked, path) in [(input.trend.is_some(), "trend"), (input.thresholds.is_some(), "thresholds")] {
        if asked && !series {
            return bad(
                "Doğru uydurma ve eşikler izleme işinin dönem dizisinindir (kind: monitoring ve önceki dönemler, epochs).".into(),
                path,
            );
        }
    }
    if let Some(t) = &input.trend
        && !(t.confidence.is_finite() && t.confidence > 0.0 && t.confidence < 1.0)
    {
        return bad("Güven düzeyi 0 ile 1 arasında bir sayı olmalı (ör. 0.95).".into(), "trend.confidence");
    }
    let Some(AsbuiltThresholds { warning, alarm }) = &input.thresholds else { return Ok(()) };
    for (level, name, words) in [(warning, "warning", "uyarı"), (alarm, "alarm", "alarm")] {
        for (key, what) in LIMITS {
            if limit_of(level, key).is_some_and(|v| !(v.is_finite() && v > 0.0)) {
                return bad(
                    format!("{what} {words} eşiği 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın."),
                    &format!("thresholds.{name}.{key}"),
                );
            }
        }
    }
    for (key, what) in LIMITS {
        if let (Some(w), Some(a)) = (limit_of(warning, key), limit_of(alarm, key))
            && w >= a
        {
            return bad(format!("{what} uyarı eşiği alarm eşiğinden küçük olmalı."), &format!("thresholds.warning.{key}"));
        }
    }
    Ok(())
}

/// The line through a point's dated epochs (docs/adr/4840): `steps` and `days` along the series.
fn fit_of(steps: &[Option<AsbuiltStep>], days: &[Option<i64>], confidence: f64) -> Option<AsbuiltFit> {
    let dated: Vec<(i64, &AsbuiltStep)> = steps.iter().zip(days).filter_map(|(s, d)| Some(((*d)?, s.as_ref()?))).collect();
    if dated.len() < 3 {
        return None;
    }
    let first = dated[0].0;
    let t: Vec<f64> = dated.iter().map(|(d, _)| (d - first) as f64 / 365.25).collect();
    let column = |f: fn(&AsbuiltStep) -> f64| dated.iter().map(|(_, s)| f(s)).collect::<Vec<f64>>();
    let east = velocity(&t, &column(|s| s.de))?;
    let north = velocity(&t, &column(|s| s.dn))?;
    let heights: Vec<(f64, f64)> = t.iter().zip(&dated).filter_map(|(t, (_, s))| s.dz.map(|z| (*t, z))).collect();
    let up = velocity(&heights.iter().map(|p| p.0).collect::<Vec<_>>(), &heights.iter().map(|p| p.1).collect::<Vec<_>>());
    let mut moving = Vec::new();
    for (name, v, n) in [("east", Some(east), dated.len()), ("north", Some(north), dated.len()), ("up", up, heights.len())] {
        if v.is_some_and(|v| moves(v, n, confidence)) {
            moving.push(name.to_owned());
        }
    }
    Some(AsbuiltFit {
        epochs: u32::try_from(dated.len()).unwrap_or(u32::MAX),
        east: east.rate,
        north: north.rate,
        up: up.map(|v| v.rate),
        sigma_east: east.sigma,
        sigma_north: north.sigma,
        sigma_up: up.map(|v| v.sigma),
        speed: east.rate.hypot(north.rate),
        moving,
    })
}

/// A point's state against the thresholds (docs/adr/4840): its last horizontal movement, its last height movement
/// either way, its horizontal and vertical speed (the line's, else the mean rate); the highest level any goes beyond,
/// and the limits of that level it goes beyond.
fn state_of(t: &AsbuiltThresholds, track: &AsbuiltTrack) -> (AsbuiltState, Vec<String>) {
    let last = track.steps.last().cloned().flatten();
    let fit = track.fit.as_ref();
    let values = [
        ("horizontal", last.as_ref().map(|s| s.horizontal)),
        ("vertical", last.as_ref().and_then(|s| s.dz).map(f64::abs)),
        ("horizontalRate", fit.map(|f| f.speed).or(track.rate_horizontal)),
        ("verticalRate", fit.and_then(|f| f.up).or(track.rate_vertical).map(f64::abs)),
    ];
    let beyond = |level: &AsbuiltLevel| -> Vec<String> {
        values
            .iter()
            .filter(|(k, v)| limit_of(level, k).zip(*v).is_some_and(|(l, v)| v > l))
            .map(|(k, _)| (*k).to_owned())
            .collect()
    };
    let alarm = beyond(&t.alarm);
    if !alarm.is_empty() {
        return (AsbuiltState::Alarm, alarm);
    }
    let warning = beyond(&t.warning);
    if !warning.is_empty() {
        return (AsbuiltState::Warning, warning);
    }
    (AsbuiltState::Normal, Vec::new())
}

/// The epochs' layers are the drawing's (`layer_not_found` at `epochs[k].layer`).
pub(crate) fn layers(doc: &Document, input: &AsbuiltCheck) -> Result<(), Stop> {
    for (k, e) in input.epochs.iter().enumerate() {
        if doc.layers().get(&e.layer).is_none() {
            return Err(refuse(
                codes::LAYER_NOT_FOUND,
                format!("“{}” kimlikli katman çizimde yok.", e.layer),
                &format!("epochs[{k}].layer"),
            ));
        }
    }
    Ok(())
}

/// The point named `name` in an epoch, when it is there once.
fn one<'a>(epoch: &'a [Point], name: &str) -> Option<&'a Point> {
    let mut hits = epoch.iter().filter(|p| p.name == name);
    let first = hits.next()?;
    hits.next().is_none().then_some(first)
}

/// The points measured now (`measured`, this epoch) tracked through the series (the module's rules).
pub(crate) fn tracks(doc: &Document, input: &AsbuiltCheck, field: &str, measured: &[Point]) -> Vec<AsbuiltTrack> {
    if input.epochs.is_empty() {
        return Vec::new();
    }
    let earlier: Vec<Vec<Point>> = input.epochs.iter().map(|e| points(doc, &e.layer, field, None)).collect();
    let days: Vec<Option<i64>> = input
        .epochs
        .iter()
        .map(|e| e.date.as_deref().and_then(|d| day_number(d.trim())))
        .chain(std::iter::once(input.date.as_deref().and_then(|d| day_number(d.trim()))))
        .collect();
    let mut out = Vec::new();
    for p in measured {
        if p.name.is_empty() || one(measured, &p.name).is_none() {
            continue;
        }
        let at: Vec<Option<&Point>> =
            earlier.iter().map(|e| one(e, &p.name)).chain(std::iter::once(Some(p))).collect();
        if at.iter().flatten().count() < 2 {
            continue;
        }
        let Some(from) = at.iter().position(Option::is_some) else { continue };
        let Some(r) = at[from] else { continue };
        let steps: Vec<Option<AsbuiltStep>> = at
            .iter()
            .map(|q| {
                q.map(|q| {
                    let (de, dn) = (q.p.x - r.p.x, q.p.y - r.p.y);
                    let dz = q.z.zip(r.z).map(|(a, b)| a - b);
                    let horizontal = de.hypot(dn);
                    let outside = input.horizontal.is_some_and(|h| horizontal > h)
                        || input.vertical.zip(dz).is_some_and(|(v, dz)| dz.abs() > v);
                    AsbuiltStep { de, dn, dz, horizontal, outside }
                })
            })
            .collect();
        let last = steps.last().cloned().flatten();
        let years = days[from].zip(*days.last().unwrap_or(&None)).map(|(a, b)| (b - a) as f64 / 365.25).filter(|y| *y > 0.0);
        let (rate_horizontal, rate_vertical) = match (years, &last) {
            (Some(y), Some(s)) => (Some(s.horizontal / y), s.dz.map(|dz| dz / y)),
            _ => (None, None),
        };
        // Docs/adr/4840: the line through its dated epochs, asked.
        let fit = input.trend.as_ref().and_then(|tr| fit_of(&steps, &days, tr.confidence));
        let mut track = AsbuiltTrack {
            name: p.name.clone(),
            from: u32::try_from(from).unwrap_or(u32::MAX),
            steps,
            rate_horizontal,
            rate_vertical,
            fit,
            state: None,
            exceeded: Vec::new(),
        };
        if let Some(th) = &input.thresholds {
            let (state, exceeded) = state_of(th, &track);
            track.state = Some(state);
            track.exceeded = exceeded;
        }
        out.push(track);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_a_real_one() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(day_number("2026-04-15").zip(day_number("2026-01-15")).map(|(a, b)| a - b), Some(90));
        assert_eq!(day_number("2024-03-01").zip(day_number("2024-02-28")).map(|(a, b)| a - b), Some(2));
        assert!(day_number("2024-02-29").is_some());
        for bad in ["2026-02-29", "2026-13-01", "2026-00-10", "2026-4-15", "26-04-15", "2026-04-31", "2026/04/15"] {
            assert_eq!(day_number(bad), None, "{bad}");
        }
    }
}
