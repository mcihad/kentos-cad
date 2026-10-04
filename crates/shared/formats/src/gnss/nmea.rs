//! NMEA 0183 GNSS logs (docs/adr/0169 §1, §6): a receiver's GGA fixes with
//! the date of the RMC sentences before them, read as the standard's
//! sentences describe them. Every sentence not read is named with the
//! reason. The independent reference is `scripts/fixtures/gnss_nmea_cases.py`
//! (fixtures/gnss/v1/nmea.json). Latitudes and longitudes are their exact
//! values rounded once to the nearest float64 (CLAUDE.md §23).

use kentos_contracts::{GnssPoint, GnssRead, LineError};

use crate::field::exact::Exact;
use crate::text;

/// What is said of a sentence not read; `{line}`, `{data}` and `{count}` are
/// filled in.
pub const CHECKSUM: &str =
    "Satır {line}: NMEA cümlesinin sağlama toplamı tutmuyor; cümle okunmadı.";
pub const POSITION: &str = "Satır {line}: konum “{data}” okunmuyor; cümle okunmadı.";
pub const VALUE: &str = "Satır {line}: “{data}” sayı değil; cümle okunmadı.";
pub const UNIT: &str =
    "Satır {line}: yükseklik birimi “{data}” okunmuyor; yalnız metre (M); cümle okunmadı.";
pub const NOFIX: &str =
    "Satır {line}: konumu olmayan GGA cümleleri ({count} cümle, kalite 0) okunmadı.";

/// GGA's fix qualities as the window names them.
fn quality(q: &str) -> String {
    match q {
        "1" => "GPS",
        "2" => "DGPS",
        "3" => "PPS",
        "4" => "RTK sabit",
        "5" => "RTK kayan",
        "6" => "tahmini",
        "7" => "elle",
        "8" => "benzetim",
        other => other,
    }
    .to_owned()
}

/// Why a sentence is not read: its text and the field's text.
struct Unread(&'static str, String);

/// A number field: blank is none.
fn number(data: &str) -> Result<Option<Exact>, Unread> {
    if data.is_empty() {
        return Ok(None);
    }
    Exact::decimal(data)
        .map(Some)
        .ok_or_else(|| Unread(VALUE, data.to_owned()))
}

/// A latitude (ddmm.mmmm) or longitude (dddmm.mmmm) and its hemisphere: the
/// exact degrees, within `limit`.
fn coordinate(
    value: &str,
    hemisphere: &str,
    degrees: usize,
    limit: i128,
    (positive, negative): (&str, &str),
) -> Result<Exact, Unread> {
    let bad = || Unread(POSITION, format!("{value},{hemisphere}"));
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if whole.len() != degrees + 2
        || !digits(whole)
        || !digits(fraction)
        || whole.len() + fraction.len() > 30
        || (hemisphere != positive && hemisphere != negative)
    {
        return Err(bad());
    }
    let minutes = Exact::decimal(&format!("{}.{fraction}", &whole[degrees..])).ok_or_else(bad)?;
    if minutes.size_cmp(60) != std::cmp::Ordering::Less {
        return Err(bad());
    }
    let d: i128 = whole[..degrees].parse().map_err(|_| bad())?;
    let v = minutes.over(60).plus(d);
    if v.size_cmp(limit) == std::cmp::Ordering::Greater {
        return Err(bad());
    }
    Ok(if hemisphere == negative {
        v.negated()
    } else {
        v
    })
}

/// A time hhmmss(.fraction) as hh:mm:ss(.fraction); none for one that is not.
fn clock(t: &str) -> Option<String> {
    let (hms, fraction) = match t.split_once('.') {
        Some((a, b)) if !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit()) => (a, Some(b)),
        Some(_) => return None,
        None => (t, None),
    };
    if hms.len() != 6 || !hms.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let part = |k: usize| hms[k..k + 2].parse::<u32>().unwrap_or(99);
    if part(0) > 23 || part(2) > 59 || part(4) > 60 {
        return None;
    }
    Some(match fraction {
        Some(f) => format!("{}:{}:{}.{f}", &hms[..2], &hms[2..4], &hms[4..]),
        None => format!("{}:{}:{}", &hms[..2], &hms[2..4], &hms[4..]),
    })
}

/// An RMC date ddmmyy as 20yy-mm-dd; none for one that is not.
fn date(d: &str) -> Option<String> {
    if d.len() != 6 || !d.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let (day, month): (u32, u32) = (d[..2].parse().ok()?, d[2..4].parse().ok()?);
    ((1..=12).contains(&month) && (1..=31).contains(&day))
        .then(|| format!("20{}-{}-{}", &d[4..], &d[2..4], &d[..2]))
}

/// A problem: `{line}`, then the placeholders in order.
fn said(text: &str, line: u32, fill: &[(&str, &str)]) -> LineError {
    let mut message = text.replace("{line}", &line.to_string());
    for (k, v) in fill {
        message = message.replace(k, v);
    }
    LineError { line, message }
}

/// A GGA sentence's fix.
fn gga(f: &[&str], line: u32, day: Option<&str>) -> Result<GnssPoint, Unread> {
    let get = |k: usize| f.get(k).copied().unwrap_or("");
    let lat = coordinate(get(2), get(3), 2, 90, ("N", "S"))?;
    let lon = coordinate(get(4), get(5), 3, 180, ("E", "W"))?;
    let sats = get(7);
    let satellites = if sats.is_empty() {
        None
    } else {
        let n = sats
            .bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| sats.parse::<u32>().ok())
            .flatten();
        Some(n.ok_or_else(|| Unread(VALUE, sats.to_owned()))?)
    };
    let hdop = number(get(8))?;
    let alt = number(get(9))?;
    if alt.is_some() && !matches!(get(10), "M" | "") {
        return Err(Unread(UNIT, get(10).to_owned()));
    }
    let sep = number(get(11))?;
    if sep.is_some() && !matches!(get(12), "M" | "") {
        return Err(Unread(UNIT, get(12).to_owned()));
    }
    let time = clock(get(1)).map(|t| match day {
        Some(d) => format!("{d}T{t}Z"),
        None => t,
    });
    Ok(GnssPoint {
        kind: "gga".to_owned(),
        name: None,
        lat: lat.value(),
        lon: lon.value(),
        height: alt.map(Exact::value),
        geoid: sep.map(Exact::value),
        ellipsoidal: alt.zip(sep).and_then(|(a, s)| a.sum(s)).map(Exact::value),
        time,
        fix: Some(quality(get(6))),
        satellites,
        hdop: hdop.map(Exact::value),
        line,
    })
}

/// Reads an NMEA 0183 log.
pub fn read(bytes: &[u8]) -> GnssRead {
    let (enc, bom) = text::sniff(bytes);
    let body = text::decode(&bytes[bom..], enc)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let mut read = GnssRead {
        format: "nmea".to_owned(),
        encoding: enc.label().to_owned(),
        points: Vec::new(),
        problems: Vec::new(),
    };
    let mut day: Option<String> = None;
    let mut nofix: Option<(u32, usize)> = None;
    for (i, raw) in body.split('\n').enumerate() {
        let line = u32::try_from(i + 1).unwrap_or(u32::MAX);
        let Some(at) = raw.find('$') else {
            continue;
        };
        let s = raw[at..].trim();
        let rest = &s[1..];
        let (sentence, check) = match rest.split_once('*') {
            Some((b, c)) => (b, Some(c)),
            None => (rest, None),
        };
        if let Some(c) = check {
            let x = sentence.bytes().fold(0u8, |x, b| x ^ b);
            let ok = c.len() == 2
                && c.bytes().all(|b| b.is_ascii_hexdigit())
                && u8::from_str_radix(c, 16) == Ok(x);
            if !ok {
                read.problems.push(said(CHECKSUM, line, &[]));
                continue;
            }
        }
        let raw_fields: Vec<&str> = sentence.split(',').collect();
        let head: Vec<char> = raw_fields[0].chars().collect();
        let f: Vec<&str> = raw_fields.iter().map(|s| s.trim()).collect();
        let kind: String = if head.len() == 5 {
            head[2..].iter().collect()
        } else {
            String::new()
        };
        match kind.as_str() {
            "RMC" => {
                if let Some(d) = f.get(9).and_then(|d| date(d)) {
                    day = Some(d);
                }
            }
            "GGA" => {
                if matches!(f.get(6).copied().unwrap_or(""), "" | "0") {
                    nofix.get_or_insert((line, 0)).1 += 1;
                    continue;
                }
                match gga(&f, line, day.as_deref()) {
                    Ok(p) => read.points.push(p),
                    Err(Unread(text, data)) => {
                        read.problems.push(said(text, line, &[("{data}", &data)]));
                    }
                }
            }
            _ => {}
        }
    }
    if let Some((line, count)) = nofix {
        read.problems
            .push(said(NOFIX, line, &[("{count}", &count.to_string())]));
        read.problems.sort_by_key(|p| p.line);
    }
    read
}
