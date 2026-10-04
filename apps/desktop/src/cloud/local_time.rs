//! The device's local time for the lists' dates: the web writes them with
//! `toLocaleString('tr-TR')` in the device's time zone (“26.09.2026 14:05”),
//! and the desktop writes the same. The UTC offset at an instant comes from
//! the system's zone file (`$TZ` as a zone name or a path, else
//! /etc/localtime, TZif): its transitions, then its POSIX rule for later
//! instants (a fixed offset such as Turkey's `<+03>-3`, or a daylight rule
//! such as `CET-1CEST,M3.5.0,M10.5.0/3`). Without a readable zone: UTC.
//! No library: the standard library reads the file.

use std::sync::OnceLock;

/// A time zone: offsets by transition, and the rule after the last one.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    /// (instant, offset) from each transition on, seconds east of UTC.
    transitions: Vec<(i64, i32)>,
    /// Before the first transition.
    initial: i32,
    /// After the last transition.
    rule: Option<Rule>,
}

/// A POSIX zone rule: a standard offset, and a daylight one with its dates.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rule {
    std: i32,
    dst: Option<(i32, Date, Date)>,
}

/// `Mm.w.d/time`: the `w`th `d`ay (0 Sunday) of month `m` (week 5: the
/// last), at `time` seconds of local time.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Date {
    month: u32,
    week: u32,
    weekday: u32,
    time: i64,
}

impl Zone {
    /// A fixed offset (tests, and UTC when no zone is readable).
    pub fn fixed(offset: i32) -> Self {
        Self {
            transitions: Vec::new(),
            initial: offset,
            rule: Some(Rule {
                std: offset,
                dst: None,
            }),
        }
    }

    /// The zone this device is in.
    pub fn system() -> &'static Zone {
        static ZONE: OnceLock<Zone> = OnceLock::new();
        ZONE.get_or_init(|| {
            let file = match std::env::var("TZ") {
                Ok(tz) if !tz.is_empty() => {
                    let name = tz.trim_start_matches(':');
                    if name.starts_with('/') {
                        name.to_owned()
                    } else {
                        format!("/usr/share/zoneinfo/{name}")
                    }
                }
                _ => "/etc/localtime".to_owned(),
            };
            std::fs::read(file)
                .ok()
                .and_then(|b| Zone::parse(&b))
                .unwrap_or_else(|| Zone::fixed(0))
        })
    }

    /// A TZif file (versions 1 to 4); none when it is not one.
    pub fn parse(b: &[u8]) -> Option<Self> {
        let header = |at: usize| -> Option<(u8, [usize; 6])> {
            if b.get(at..at + 4)? != b"TZif" {
                return None;
            }
            let version = *b.get(at + 4)?;
            let mut counts = [0usize; 6];
            for (i, c) in counts.iter_mut().enumerate() {
                let s = at + 20 + 4 * i;
                *c = u32::from_be_bytes(b.get(s..s + 4)?.try_into().ok()?) as usize;
            }
            Some((version, counts))
        };
        let (version, counts) = header(0)?;
        // [isutcnt, isstdcnt, leapcnt, timecnt, typecnt, charcnt]
        let v1_len = |c: [usize; 6], time: usize| {
            c[3] * time + c[3] + c[4] * 6 + c[5] + c[2] * (time + 4) + c[1] + c[0]
        };
        let (start, counts, time) = if version >= b'2' {
            let second = 44 + v1_len(counts, 4);
            let (_, c2) = header(second)?;
            (second + 44, c2, 8)
        } else {
            (44, counts, 4)
        };
        let [_, _, _, timecnt, typecnt, _] = counts;
        let read_time = |at: usize| -> Option<i64> {
            let bytes = b.get(at..at + time)?;
            Some(if time == 8 {
                i64::from_be_bytes(bytes.try_into().ok()?)
            } else {
                i64::from(i32::from_be_bytes(bytes.try_into().ok()?))
            })
        };
        let types_at = start + timecnt * time;
        let infos_at = types_at + timecnt;
        let offsets: Vec<i32> = (0..typecnt)
            .map(|i| {
                let s = infos_at + 6 * i;
                b.get(s..s + 4)
                    .and_then(|x| x.try_into().ok())
                    .map(i32::from_be_bytes)
            })
            .collect::<Option<_>>()?;
        let transitions: Vec<(i64, i32)> = (0..timecnt)
            .map(|i| {
                let at = read_time(start + i * time)?;
                let ty = *b.get(types_at + i)? as usize;
                Some((at, *offsets.get(ty)?))
            })
            .collect::<Option<_>>()?;
        let initial = offsets.first().copied().unwrap_or(0);
        let rule = if version >= b'2' {
            let end = infos_at + typecnt * 6 + counts[5] + counts[2] * 12 + counts[1] + counts[0];
            b.get(end..)
                .and_then(|f| std::str::from_utf8(f).ok())
                .and_then(|f| f.trim_matches('\n').lines().next().map(str::to_owned))
                .and_then(|f| parse_rule(&f))
        } else {
            None
        };
        Some(Self {
            transitions,
            initial,
            rule,
        })
    }

    /// The offset east of UTC at `t` (seconds since 1970), seconds.
    pub fn offset_at(&self, t: i64) -> i32 {
        match self.transitions.last() {
            Some(&(last, offset)) if t >= last => self.rule.map_or(offset, |r| r.offset_at(t)),
            Some(_) => {
                let i = self.transitions.partition_point(|&(at, _)| at <= t);
                i.checked_sub(1)
                    .map_or(self.initial, |i| self.transitions[i].1)
            }
            None => self.rule.map_or(self.initial, |r| r.offset_at(t)),
        }
    }
}

impl Rule {
    fn offset_at(self, t: i64) -> i32 {
        let Some((dst, start, end)) = self.dst else {
            return self.std;
        };
        let year = civil(t + i64::from(self.std)).0;
        // The change instants in UTC: the start in standard time, the end in daylight time.
        let on = start.at(year) - i64::from(self.std);
        let off = end.at(year) - i64::from(dst);
        let inside = if on < off {
            t >= on && t < off
        } else {
            t >= on || t < off
        };
        if inside { dst } else { self.std }
    }
}

impl Date {
    /// Local seconds since 1970 of this date in `year`.
    fn at(self, year: i64) -> i64 {
        let first = days_from_civil(year, i64::from(self.month), 1);
        // 1970-01-01 was a Thursday (4).
        let first_weekday = (first + 4).rem_euclid(7);
        let mut day = 1 + (i64::from(self.weekday) - first_weekday).rem_euclid(7);
        day += 7 * (i64::from(self.week) - 1);
        let days_in_month = days_from_civil(
            if self.month == 12 { year + 1 } else { year },
            if self.month == 12 {
                1
            } else {
                i64::from(self.month) + 1
            },
            1,
        ) - first;
        while day > days_in_month {
            day -= 7;
        }
        (first + day - 1) * 86_400 + self.time
    }
}

/// A POSIX TZ rule (`<+03>-3`, `UTC0`, `CET-1CEST,M3.5.0,M10.5.0/3`); none
/// for what this reader does not know (the transitions still answer then).
fn parse_rule(text: &str) -> Option<Rule> {
    let mut s = text;
    let name = |s: &mut &str| -> Option<()> {
        if let Some(rest) = s.strip_prefix('<') {
            let end = rest.find('>')?;
            *s = &rest[end + 1..];
        } else {
            let end = s
                .find(|c: char| !c.is_ascii_alphabetic())
                .unwrap_or(s.len());
            (end >= 3).then_some(())?;
            *s = &s[end..];
        }
        Some(())
    };
    // `[+-]hh[:mm[:ss]]`, POSIX's sign: west positive.
    let offset = |s: &mut &str| -> Option<i32> {
        let (sign, rest) = match s.as_bytes().first()? {
            b'-' => (1, &s[1..]),
            b'+' => (-1, &s[1..]),
            _ => (-1, *s),
        };
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == ':'))
            .unwrap_or(rest.len());
        let mut parts = rest[..end].split(':').map(|p| p.parse::<i32>().ok());
        let h = parts.next()??;
        let m = parts.next().flatten().unwrap_or(0);
        let sec = parts.next().flatten().unwrap_or(0);
        *s = &rest[end..];
        Some(sign * (h * 3600 + m * 60 + sec))
    };
    name(&mut s)?;
    let std = offset(&mut s)?;
    if s.is_empty() {
        return Some(Rule { std, dst: None });
    }
    name(&mut s)?;
    let dst = if s.starts_with(',') {
        std + 3600
    } else {
        offset(&mut s)?
    };
    let date = |part: &str| -> Option<Date> {
        let (day, time) = part.split_once('/').unwrap_or((part, "2"));
        let mut d = day
            .strip_prefix('M')?
            .split('.')
            .map(|x| x.parse::<u32>().ok());
        let (month, week, weekday) = (d.next()??, d.next()??, d.next()??);
        let mut t = time.split(':').map(|x| x.parse::<i64>().ok());
        let h = t.next()??;
        let seconds = h * 3600 + t.next().flatten().unwrap_or(0) * 60;
        Some(Date {
            month,
            week,
            weekday,
            time: seconds,
        })
    };
    let mut rules = s.strip_prefix(',')?.split(',');
    let start = date(rules.next()?)?;
    let end = date(rules.next()?)?;
    Some(Rule {
        std,
        dst: Some((dst, start, end)),
    })
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// (year, month, day, seconds of the day) of seconds since 1970.
fn civil(seconds: i64) -> (i64, i64, i64, i64) {
    let days = seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day, seconds.rem_euclid(86_400))
}

/// A server's time as the lists write it in the device's time, tr-TR short
/// date and time: “26.09.2026 14:05”; empty for anything else.
pub fn when(text: &str, zone: &Zone) -> String {
    let Some(t) = super::words::epoch(text) else {
        return String::new();
    };
    let (year, month, day, secs) = civil(t + i64::from(zone.offset_at(t)));
    format!(
        "{day:02}.{month:02}.{year} {:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60
    )
}

/// A moment (seconds since 1970) in the device's zone as a time stamp to the
/// second, “2026-10-04T10:15:00” (Trimble JobXML's, docs/adr/0169 §4).
pub fn iso_local(t: i64, zone: &Zone) -> String {
    let (year, month, day, secs) = civil(t + i64::from(zone.offset_at(t)));
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// A moment (milliseconds since 1970) as the time of day in the device's
/// zone, tr-TR: “14:05:09” (the web's `toLocaleTimeString`).
pub fn clock(ms: u64, zone: &Zone) -> String {
    let t = i64::try_from(ms / 1000).unwrap_or(0);
    let (_, _, _, secs) = civil(t + i64::from(zone.offset_at(t)));
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}

/// Seconds since 1970 of a day's midnight, the day read as if in UTC (a
/// local time before [`from_local`] takes the zone off).
pub fn seconds_of(year: i64, month: i64, day: i64) -> i64 {
    days_from_civil(year, month, day) * 86_400
}

/// The moment a local time `t` (seconds, read as if UTC) is in `zone`: its
/// offset taken off, the offset of the moment itself around a DST change.
pub fn from_local(t: i64, zone: &Zone) -> i64 {
    let guess = t - i64::from(zone.offset_at(t));
    t - i64::from(zone.offset_at(guess))
}

/// A moment as the server's RFC 3339 in UTC with milliseconds, as
/// JavaScript's `toISOString` writes it: “2026-12-31T20:59:59.000Z”.
pub fn iso_utc(t: i64) -> String {
    iso_utc_ms(t * 1000)
}

/// [`iso_utc`] of milliseconds since 1970.
pub fn iso_utc_ms(ms: i64) -> String {
    let (year, month, day, secs) = civil(ms.div_euclid(1000));
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60,
        ms.rem_euclid(1000)
    )
}

/// A server's time as a date in the device's time: “26.09.2026”.
pub fn day(text: &str, zone: &Zone) -> Option<String> {
    super::words::epoch(text).map(|t| date_of(t, zone))
}

/// The date of seconds since 1970 in the device's time: “26.09.2026”.
pub fn date_of(t: i64, zone: &Zone) -> String {
    let (year, month, day, _) = civil(t + i64::from(zone.offset_at(t)));
    format!("{day:02}.{month:02}.{year}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkey_is_three_hours_east_and_a_daylight_rule_turns() {
        let istanbul = Zone::fixed(3 * 3600);
        assert_eq!(when("2026-09-26T11:05:00Z", &istanbul), "26.09.2026 14:05");
        // 2026-09-26T11:05:09.5Z: the time of day, as the processing history writes it.
        assert_eq!(clock(1_790_420_709_500, &istanbul), "14:05:09");
        assert_eq!(
            day("2026-10-25T21:30:00Z", &istanbul).as_deref(),
            Some("26.10.2026")
        );
        let rule = parse_rule("<+03>-3").expect("reads");
        assert_eq!(rule.offset_at(0), 3 * 3600);
        let berlin = Zone {
            transitions: Vec::new(),
            initial: 3600,
            rule: parse_rule("CET-1CEST,M3.5.0,M10.5.0/3"),
        };
        // 2026: summer time from 29 March 01:00 UTC to 25 October 01:00 UTC.
        let t = |text: &str| super::super::words::epoch(text).expect("a time");
        assert_eq!(berlin.offset_at(t("2026-03-29T00:59:59Z")), 3600);
        assert_eq!(berlin.offset_at(t("2026-03-29T01:00:00Z")), 7200);
        assert_eq!(berlin.offset_at(t("2026-10-25T00:59:59Z")), 7200);
        assert_eq!(berlin.offset_at(t("2026-10-25T01:00:00Z")), 3600);
    }

    #[test]
    fn the_system_zone_file_reads_when_there_is_one() {
        if let Ok(bytes) = std::fs::read("/usr/share/zoneinfo/Europe/Istanbul") {
            let zone = Zone::parse(&bytes).expect("a TZif file");
            let t = super::super::words::epoch("2030-01-01T00:00:00Z").expect("a time");
            assert_eq!(zone.offset_at(t), 3 * 3600, "past the table, the rule");
            let old = super::super::words::epoch("2010-07-01T00:00:00Z").expect("a time");
            assert_eq!(zone.offset_at(old), 3 * 3600, "summer time in 2010");
        }
        assert!(Zone::parse(b"not a zone").is_none());
    }
}
