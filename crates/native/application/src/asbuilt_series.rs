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

use kentos_commands::{AsbuiltCheck, AsbuiltKind, AsbuiltStep, AsbuiltTrack};
use kentos_domain::Document;

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
    Ok(())
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
        out.push(AsbuiltTrack {
            name: p.name.clone(),
            from: u32::try_from(from).unwrap_or(u32::MAX),
            steps,
            rate_horizontal,
            rate_vertical,
        });
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
