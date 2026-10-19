//! A search's datetime (docs/adr/0528 §3): an RFC 3339 instant or an interval `start/end` whose ends are instants,
//! bare dates or open (`..`, or empty). A bare date is its day: `T00:00:00Z` as a start (and as a lone date's
//! start), `T23:59:59Z` as an end. The text sent is the ends normalized (`T` and `Z` in upper case, the
//! fraction and the offset as written); a start after its end is refused, compared as instants.

/// An instant: seconds since 1970-01-01T00:00:00Z (the offset applied) and the fraction's digits.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Instant {
    seconds: i64,
    /// The fraction's digits padded with zeros to nine places.
    nanos: String,
}

impl Instant {
    fn key(&self) -> (i64, &str) {
        (self.seconds, &self.nanos)
    }
}

/// Days from 1970-01-01 of a civil date (proleptic Gregorian; H. Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(y) => 29,
        _ => 28,
    }
}

fn digits(text: &str, at: std::ops::Range<usize>) -> Option<i64> {
    let s = text.get(at)?;
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// A bare date `YYYY-MM-DD`: its year, month and day when it is a real date.
fn date_of(text: &str) -> Option<(i64, i64, i64)> {
    if text.len() != 10 || text.as_bytes()[4] != b'-' || text.as_bytes()[7] != b'-' {
        return None;
    }
    let (y, m, d) = (
        digits(text, 0..4)?,
        digits(text, 5..7)?,
        digits(text, 8..10)?,
    );
    ((1..=12).contains(&m) && (1..=days_in(y, m)).contains(&d)).then_some((y, m, d))
}

/// An instant `YYYY-MM-DDThh:mm:ss[.f…](Z|±hh:mm)` normalized (`T` and `Z` upper case) with its value.
fn instant_of(text: &str) -> Option<(String, Instant)> {
    let b = text.as_bytes();
    if b.len() < 20 || !matches!(b[10], b'T' | b't') || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let (y, m, d) = date_of(text.get(0..10)?)?;
    let (hh, mm, ss) = (
        digits(text, 11..13)?,
        digits(text, 14..16)?,
        digits(text, 17..19)?,
    );
    if hh > 23 || mm > 59 || ss > 60 {
        return None;
    }
    let mut at = 19;
    let mut nanos = String::new();
    if b[at] == b'.' {
        let start = at + 1;
        at = start;
        while at < b.len() && b[at].is_ascii_digit() {
            at += 1;
        }
        if at == start || at - start > 9 {
            return None;
        }
        nanos = text[start..at].to_owned();
    }
    while nanos.len() < 9 {
        nanos.push('0');
    }
    let rest = text.get(at..)?;
    let offset = match rest {
        "Z" | "z" => 0,
        _ if rest.len() == 6
            && matches!(rest.as_bytes()[0], b'+' | b'-')
            && rest.as_bytes()[3] == b':' =>
        {
            let (oh, om) = (digits(rest, 1..3)?, digits(rest, 4..6)?);
            if oh > 23 || om > 59 {
                return None;
            }
            let v = oh * 3600 + om * 60;
            if rest.as_bytes()[0] == b'-' { -v } else { v }
        }
        _ => return None,
    };
    let seconds = days_from_civil(y, m, d) * 86_400 + hh * 3600 + mm * 60 + ss - offset;
    let mut normal = text.to_owned();
    normal.replace_range(10..11, "T");
    if rest == "z" {
        normal.replace_range(at..at + 1, "Z");
    }
    Some((normal, Instant { seconds, nanos }))
}

/// One end of an interval: open, or an instant (a bare date as the start or the end of its day).
enum End {
    Open,
    At(String, Instant),
}

fn end_of(text: &str, is_end: bool) -> Result<End, String> {
    let t = text.trim();
    if t.is_empty() || t == ".." {
        return Ok(End::Open);
    }
    let full = if date_of(t).is_some() {
        format!("{t}{}", if is_end { "T23:59:59Z" } else { "T00:00:00Z" })
    } else {
        t.to_owned()
    };
    match instant_of(&full) {
        Some((normal, at)) => Ok(End::At(normal, at)),
        None => Err(format!(
            "“{t}” bir RFC 3339 tarih ya da an değil (2024-07-01 ya da 2024-07-01T08:30:00Z)."
        )),
    }
}

/// The datetime text a search sends for `text`, or why it is not one.
pub fn normalize(text: &str) -> Result<String, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("Tarih boş.".into());
    }
    let Some((a, b)) = t.split_once('/') else {
        // A lone date is its day; a lone instant is that instant.
        if date_of(t).is_some() {
            return normalize(&format!("{t}/{t}"));
        }
        return match end_of(t, false)? {
            End::At(normal, _) => Ok(normal),
            End::Open => Err("Tarih boş.".into()),
        };
    };
    if b.contains('/') {
        return Err("Tarih aralığı tek bir “/” içerir: başlangıç/bitiş.".into());
    }
    let (start, end) = (end_of(a, false)?, end_of(b, true)?);
    match (start, end) {
        (End::Open, End::Open) => Err("Tarih aralığının iki ucu da açık olamaz.".into()),
        (End::At(sa, ia), End::At(sb, ib)) => {
            if ia.key() > ib.key() {
                Err(format!("Başlangıç ({sa}) bitişten ({sb}) sonra."))
            } else {
                Ok(format!("{sa}/{sb}"))
            }
        }
        (End::At(sa, _), End::Open) => Ok(format!("{sa}/..")),
        (End::Open, End::At(sb, _)) => Ok(format!("../{sb}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_instants_and_intervals() {
        assert_eq!(
            normalize("2024-07-01/2024-07-20").as_deref(),
            Ok("2024-07-01T00:00:00Z/2024-07-20T23:59:59Z")
        );
        assert_eq!(
            normalize("2024-07-13").as_deref(),
            Ok("2024-07-13T00:00:00Z/2024-07-13T23:59:59Z")
        );
        assert_eq!(
            normalize("2024-07-13t08:29:15z").as_deref(),
            Ok("2024-07-13T08:29:15Z")
        );
        assert_eq!(
            normalize("2024-07-01/..").as_deref(),
            Ok("2024-07-01T00:00:00Z/..")
        );
        assert_eq!(
            normalize("/2024-07-01").as_deref(),
            Ok("../2024-07-01T23:59:59Z")
        );
        assert_eq!(
            normalize("2024-07-01T00:00:00.5+03:00/2024-07-01T00:00:00.25Z").as_deref(),
            Ok("2024-07-01T00:00:00.5+03:00/2024-07-01T00:00:00.25Z")
        );
    }

    #[test]
    fn what_is_refused() {
        assert!(normalize("").is_err());
        assert!(normalize("../..").is_err());
        assert!(normalize("2024-02-30").is_err());
        assert!(normalize("2023-02-29").is_err());
        assert!(normalize("2024-02-29").is_ok());
        assert!(normalize("2024-13-01").is_err());
        assert!(normalize("2024-07-01T25:00:00Z").is_err());
        assert!(normalize("2024-07-01T00:00:00").is_err());
        assert!(normalize("2024-07-20/2024-07-01").is_err());
        assert!(normalize("a/b/c").is_err());
        // The offset decides: 01:00+03:00 is 22:00Z the day before.
        assert!(normalize("2024-07-01T01:00:00+03:00/2024-06-30T21:59:59Z").is_err());
        assert!(normalize("2024-07-01T01:00:00+03:00/2024-06-30T22:00:00Z").is_ok());
    }
}
