//! HTTP's caching rules for tiles and documents (RFC 9111; docs/adr/0208
//! §3): whether an answer may be kept, how long it stays fresh, what
//! revalidates it. `Cache-Control`'s `no-store` keeps nothing; `max-age`
//! (`s-maxage` is a shared cache's) gives the lifetime, else `Expires` less
//! `Date`, else a tenth of the time since `Last-Modified` (at most a day);
//! `no-cache` and `must-revalidate` ask again once stale; `Age` is what a
//! cache on the way already spent.

/// What a host keeps of an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// Whether it may be kept at all.
    pub store: bool,
    /// How long it is fresh after it arrived, ms.
    pub fresh_ms: u64,
    /// Once stale, whether it must be asked for again before it is shown
    /// (`no-cache`, `must-revalidate`); otherwise a stale tile may be shown
    /// while the network is gone.
    pub must_revalidate: bool,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// The most a heuristic lifetime gives, ms.
const HEURISTIC_MOST: u64 = 24 * 60 * 60 * 1000;
/// The lifetime of an answer that says nothing at all, ms (a tile service
/// that says nothing is asked again within the hour).
const DEFAULT_MS: u64 = 60 * 60 * 1000;

/// The policy of an answer by its headers (`(name, value)`, names in any case).
pub fn policy(headers: &[(String, String)]) -> Policy {
    let get = |name: &str| {
        headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim().to_owned())
    };
    let control = get("cache-control")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let directive = |name: &str| {
        control.split(',').map(str::trim).find_map(|d| {
            let (k, v) = d.split_once('=').unwrap_or((d, ""));
            (k.trim() == name).then(|| v.trim().trim_matches('"').to_owned())
        })
    };
    let etag = get("etag");
    let last_modified = get("last-modified");
    if directive("no-store").is_some() {
        return Policy {
            store: false,
            fresh_ms: 0,
            must_revalidate: true,
            etag,
            last_modified,
        };
    }
    let age_ms = get("age")
        .and_then(|a| a.parse::<u64>().ok())
        .map_or(0, |s| s.saturating_mul(1000));
    let lifetime = if directive("no-cache").is_some() {
        Some(0)
    } else if let Some(s) = directive("max-age").and_then(|v| v.parse::<u64>().ok()) {
        Some(s.saturating_mul(1000))
    } else if let (Some(exp), date) = (get("expires"), get("date")) {
        // An Expires that does not read is in the past (RFC 9111 §5.3).
        let at = http_date(&exp).unwrap_or(0);
        let now = date.as_deref().and_then(http_date).unwrap_or(at);
        Some(at.saturating_sub(now))
    } else if let (Some(lm), Some(date)) = (last_modified.as_deref(), get("date")) {
        match (http_date(lm), http_date(&date)) {
            (Some(m), Some(d)) => Some((d.saturating_sub(m) / 10).min(HEURISTIC_MOST)),
            _ => None,
        }
    } else {
        None
    };
    Policy {
        store: true,
        fresh_ms: lifetime.unwrap_or(DEFAULT_MS).saturating_sub(age_ms),
        must_revalidate: directive("no-cache").is_some() || directive("must-revalidate").is_some(),
        etag,
        last_modified,
    }
}

/// The headers that ask whether a kept answer is still the one.
pub fn revalidation(p: &Policy) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if let Some(e) = &p.etag {
        out.push(("If-None-Match".to_owned(), e.clone()));
    }
    if let Some(m) = &p.last_modified {
        out.push(("If-Modified-Since".to_owned(), m.clone()));
    }
    out
}

/// An HTTP date (IMF-fixdate `Sun, 06 Nov 1994 08:49:37 GMT`, RFC 850's
/// `Sunday, 06-Nov-94 08:49:37 GMT`, asctime's `Sun Nov  6 08:49:37 1994`)
/// as ms since the epoch; none for anything else.
pub fn http_date(text: &str) -> Option<u64> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let words: Vec<&str> = text
        .split([' ', ',', '-'])
        .filter(|w| !w.is_empty())
        .collect();
    let month_of = |w: &str| {
        MONTHS
            .iter()
            .position(|m| w.len() >= 3 && w[..3].eq_ignore_ascii_case(m))
            .map(|i| i as u32 + 1)
    };
    // IMF-fixdate and RFC 850: weekday day month year time GMT; asctime: weekday month day time year.
    let (day, month, year, time) = match words.as_slice() {
        [_, d, m, y, t, ..] if d.chars().all(|c| c.is_ascii_digit()) => {
            let mut year: i64 = y.parse().ok()?;
            if year < 100 {
                year += if year < 70 { 2000 } else { 1900 };
            }
            (d.parse::<u32>().ok()?, month_of(m)?, year, *t)
        }
        [_, m, d, t, y] => (d.parse::<u32>().ok()?, month_of(m)?, y.parse().ok()?, *t),
        _ => return None,
    };
    let hms: Vec<u32> = time
        .split(':')
        .map(|p| p.parse::<u32>().ok())
        .collect::<Option<Vec<_>>>()?;
    let [h, mi, s] = hms[..] else {
        return None;
    };
    if !(1..=31).contains(&day) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    // Days from the civil date (Howard Hinnant's algorithm).
    let (y, m) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * i64::from(m) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + i64::from(h) * 3600 + i64::from(mi) * 60 + i64::from(s);
    u64::try_from(secs).ok().map(|s| s * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect()
    }

    #[test]
    fn dates_read_in_the_three_forms() {
        let t = 784_111_777_000;
        assert_eq!(http_date("Sun, 06 Nov 1994 08:49:37 GMT"), Some(t));
        assert_eq!(http_date("Sunday, 06-Nov-94 08:49:37 GMT"), Some(t));
        assert_eq!(http_date("Sun Nov  6 08:49:37 1994"), Some(t));
        assert_eq!(http_date("0"), None);
        assert_eq!(http_date("Sun, 06 Nov 1994 25:49:37 GMT"), None);
    }

    #[test]
    fn lifetimes_by_the_headers() {
        // OpenFreeMap's tiles: ten years.
        let p = policy(&h(&[
            ("cache-control", "public, max-age=315360000"),
            ("etag", "W/\"6ac\""),
        ]));
        assert!(p.store && !p.must_revalidate);
        assert_eq!(p.fresh_ms, 315_360_000_000);
        assert_eq!(revalidation(&p), h(&[("If-None-Match", "W/\"6ac\"")]));
        assert!(!policy(&h(&[("Cache-Control", "no-store")])).store);
        let p = policy(&h(&[("Cache-Control", "no-cache")]));
        assert!(p.store && p.must_revalidate && p.fresh_ms == 0);
        // Age is spent.
        assert_eq!(
            policy(&h(&[("cache-control", "max-age=100"), ("age", "40")])).fresh_ms,
            60_000
        );
        let p = policy(&h(&[
            ("date", "Sun, 06 Nov 1994 08:49:37 GMT"),
            ("expires", "Sun, 06 Nov 1994 09:49:37 GMT"),
        ]));
        assert_eq!(p.fresh_ms, 3_600_000);
        assert_eq!(policy(&h(&[("expires", "0")])).fresh_ms, 0);
        // A tenth of the time since the last change, at most a day.
        let p = policy(&h(&[
            ("date", "Sun, 06 Nov 1994 08:49:37 GMT"),
            ("last-modified", "Sun, 06 Nov 1994 07:49:37 GMT"),
        ]));
        assert_eq!(p.fresh_ms, 360_000);
        assert_eq!(policy(&[]).fresh_ms, DEFAULT_MS);
    }
}
