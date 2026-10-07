//! Sayıların kuralı (docs/adr/0200 §4, `kentos.statistics/1`): Bilgi al's
//! and Özet istatistik's numbers. A value is read as a decimal number (sign,
//! digits, at most one `.` or `,`, no exponent, at most 30 digits; the
//! spaces round it dropped: ADR 0199 §1's decimal rule); one that is not is
//! skipped and counted. Sum, least and greatest are exact (a 128-bit
//! mantissa at the inputs' largest scale; a sum too large for it is said,
//! never rounded). The mean is the sum over the count at the inputs' largest
//! scale plus two (or the scale asked), rounded half to even; the sample
//! standard deviation (n − 1) is in double precision, two passes in the
//! values' order, written at the mean's scale (Rust's formatting: the exact
//! binary value, half to even). The independent reference is
//! scripts/fixtures/spatial_query_cases.py.

use std::cmp::Ordering;

use crate::api::Op;
use crate::op;
use crate::ops::point_editor::natural_order;

/// The rule's version, written beside the results that depend on it.
pub const POLICY: &str = "kentos.statistics/1";
/// Most digits a value may have.
const MAX_DIGITS: usize = 30;
/// JavaScript's trim set (ADR 0199 §1).
fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{a}' | '\u{b}' | '\u{c}' | '\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// An exact decimal: `m` × 10^−`scale`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dec {
    pub m: i128,
    pub scale: u32,
}

fn pow10(k: u32) -> Option<i128> {
    10i128.checked_pow(k)
}

impl Dec {
    /// At a larger scale (none when it does not fit).
    fn at(self, scale: u32) -> Option<Dec> {
        if scale < self.scale {
            return None;
        }
        Some(Dec {
            m: self.m.checked_mul(pow10(scale - self.scale)?)?,
            scale,
        })
    }

    /// The canonical text: `-` for a negative, the integer part without
    /// leading zeros (`0` at least), `.` and every fraction digit of its scale.
    pub fn text(self) -> String {
        let neg = self.m < 0;
        let digits = self.m.unsigned_abs().to_string();
        let s = self.scale as usize;
        let (int, frac) = if digits.len() > s {
            (
                digits[..digits.len() - s].to_owned(),
                digits[digits.len() - s..].to_owned(),
            )
        } else {
            (
                "0".to_owned(),
                format!("{}{digits}", "0".repeat(s - digits.len())),
            )
        };
        let body = if s == 0 { int } else { format!("{int}.{frac}") };
        if neg { format!("-{body}") } else { body }
    }

    /// The nearest double (the text read, correctly rounded).
    pub fn to_f64(self) -> f64 {
        self.text().parse().unwrap_or(f64::NAN)
    }
}

/// Exact order of two decimals.
pub fn compare(a: Dec, b: Dec) -> Ordering {
    let s = a.scale.max(b.scale);
    match (a.at(s), b.at(s)) {
        (Some(x), Some(y)) => x.m.cmp(&y.m),
        // One does not fit at the other's scale: it is the larger in size.
        _ => a.to_f64().total_cmp(&b.to_f64()),
    }
}

/// A value read as a decimal number, or none.
pub fn read_number(text: &str) -> Option<Dec> {
    let t = text.trim_matches(is_js_space);
    let (neg, body) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let mut seps = body.match_indices(['.', ',']);
    let (int, frac) = match (seps.next(), seps.next()) {
        (None, _) => (body, ""),
        (Some((i, _)), None) => (&body[..i], &body[i + 1..]),
        _ => return None,
    };
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    if !int.chars().chain(frac.chars()).all(|c| c.is_ascii_digit()) {
        return None;
    }
    let int = int.trim_start_matches('0');
    if int.len() + frac.len() > MAX_DIGITS {
        return None;
    }
    let digits = format!("{int}{frac}");
    let m: i128 = if digits.is_empty() {
        0
    } else {
        digits.parse().ok()?
    };
    Some(Dec {
        m: if neg { -m } else { m },
        scale: frac.len() as u32,
    })
}

/// Half to even: `n / d` at `scale` more fraction digits than `n` has (d > 0).
fn divide(n: Dec, d: i128, scale: u32) -> Option<Dec> {
    let num = n.at(scale)?.m;
    let (q, r) = (num / d, num % d);
    let twice = r.unsigned_abs().checked_mul(2)?;
    let half = twice.cmp(&d.unsigned_abs());
    let away = match half {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => q % 2 != 0,
    };
    let q = if away { q + num.signum() } else { q };
    Some(Dec { m: q, scale })
}

/// A statistic of Bilgi al and Özet istatistik.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    Count,
    Sum,
    Mean,
    Min,
    Max,
    First,
}

impl Stat {
    pub const ALL: [Stat; 6] = [
        Stat::Count,
        Stat::Sum,
        Stat::Mean,
        Stat::Min,
        Stat::Max,
        Stat::First,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Stat::Count => "count",
            Stat::Sum => "sum",
            Stat::Mean => "mean",
            Stat::Min => "min",
            Stat::Max => "max",
            Stat::First => "first",
        }
    }

    pub fn from_key(key: &str) -> Option<Stat> {
        Stat::ALL.into_iter().find(|s| s.key() == key)
    }
}

/// What the numbers of a group give.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Figures {
    /// Values read as numbers.
    pub read: usize,
    /// Values given but not read as numbers.
    pub skipped: usize,
    pub sum: Option<Dec>,
    pub mean: Option<Dec>,
    pub min: Option<Dec>,
    pub max: Option<Dec>,
    /// The sample standard deviation's text at the mean's scale.
    pub std: Option<String>,
}

/// Why the numbers cannot be summed exactly.
pub const TOO_LARGE: &str =
    "Sayılar kesin toplanamayacak kadar büyük ya da çok basamaklı; toplam yazılmadı.";

/// The figures of `values` (empty and absent ones are no value); the mean
/// and the deviation at `scale` when given, else the largest scale + 2.
pub fn figures(values: &[Option<&str>], scale: Option<u32>) -> Result<Figures, String> {
    let mut f = Figures::default();
    let mut nums = Vec::new();
    for v in values.iter().flatten() {
        if v.trim_matches(is_js_space).is_empty() {
            continue;
        }
        match read_number(v) {
            Some(d) => nums.push(d),
            None => f.skipped += 1,
        }
    }
    f.read = nums.len();
    if nums.is_empty() {
        return Ok(f);
    }
    let top = nums.iter().map(|d| d.scale).max().unwrap_or(0);
    let mut sum = Dec { m: 0, scale: top };
    for d in &nums {
        let d = d.at(top).ok_or(TOO_LARGE)?;
        sum.m = sum.m.checked_add(d.m).ok_or(TOO_LARGE)?;
    }
    f.sum = Some(sum);
    let out = scale.unwrap_or(top + 2);
    f.mean = Some(round_to(sum, nums.len() as i128, out).ok_or(TOO_LARGE)?);
    f.min = nums.iter().copied().min_by(|a, b| compare(*a, *b));
    f.max = nums.iter().copied().max_by(|a, b| compare(*a, *b));
    if nums.len() >= 2 {
        let xs: Vec<f64> = nums.iter().map(|d| d.to_f64()).collect();
        let n = xs.len() as f64;
        let mean = xs.iter().fold(0.0, |s, x| s + x) / n;
        let ss = xs.iter().fold(0.0, |s, x| s + (x - mean) * (x - mean));
        let std = (ss / (n - 1.0)).sqrt();
        f.std = Some(fixed(std, out));
    }
    Ok(f)
}

/// `sum / n` rounded half to even at `scale` fraction digits, whatever the sum's.
fn round_to(sum: Dec, n: i128, scale: u32) -> Option<Dec> {
    if scale >= sum.scale {
        return divide(sum, n, scale);
    }
    // Fewer digits than the sum has: divide by n · 10^(s − scale) at the target scale.
    let d = n.checked_mul(pow10(sum.scale - scale)?)?;
    divide(Dec { m: sum.m, scale }, d, scale)
}

/// A double at `scale` fraction digits, half to even on its exact value; no minus on zero.
pub fn fixed(x: f64, scale: u32) -> String {
    let s = format!("{:.*}", scale as usize, x);
    if s.starts_with('-') && s[1..].chars().all(|c| c == '0' || c == '.') {
        s[1..].to_owned()
    } else {
        s
    }
}

/// One statistic over `values` for Bilgi al: Sayı counts the values given
/// (read or not), İlk değer is the first given as written; the others are
/// the figures' (none when nothing is read).
pub fn statistic(
    values: &[Option<&str>],
    stat: Stat,
    scale: Option<u32>,
) -> Result<(Option<String>, usize), String> {
    let given: Vec<&str> = values
        .iter()
        .flatten()
        .copied()
        .filter(|v| !v.trim_matches(is_js_space).is_empty())
        .collect();
    match stat {
        Stat::Count => Ok((Some(values.len().to_string()), 0)),
        Stat::First => Ok((given.first().map(|v| (*v).to_owned()), 0)),
        _ => {
            let f = figures(values, scale)?;
            let v = match stat {
                Stat::Sum => f.sum,
                Stat::Mean => f.mean,
                Stat::Min => f.min,
                Stat::Max => f.max,
                Stat::Count | Stat::First => None,
            };
            Ok((v.map(Dec::text), f.skipped))
        }
    }
}

/// Özet istatistik's table: its columns and rows (texts), and the values
/// skipped. Rows by group (the natural order of their texts, the empty
/// group, “(boş)”, last) and a Toplam row after them when grouped.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Summary {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub skipped: usize,
}

/// The summary's columns past the group's.
pub const COLUMNS: [&str; 7] = [
    "Nesne",
    "Değer",
    "Toplam",
    "Ortalama",
    "En az",
    "En çok",
    "Std. sapma",
];
/// The empty group's name, and the total row's.
pub const EMPTY_GROUP: &str = "(boş)";
pub const TOTAL: &str = "Toplam";

fn row_of(name: Option<&str>, objects: usize, f: &Figures) -> Vec<String> {
    let text = |d: Option<Dec>| d.map(Dec::text).unwrap_or_default();
    let mut row: Vec<String> = name.map(str::to_owned).into_iter().collect();
    row.extend([
        objects.to_string(),
        f.read.to_string(),
        text(f.sum),
        text(f.mean),
        text(f.min),
        text(f.max),
        f.std.clone().unwrap_or_default(),
    ]);
    row
}

/// The summary of objects' values, each with its group when grouped.
pub fn summarize(items: &[(Option<&str>, Option<&str>)], grouped: bool) -> Result<Summary, String> {
    let mut out = Summary::default();
    if grouped {
        out.columns.push("Grup".to_owned());
    }
    out.columns.extend(COLUMNS.iter().map(|c| (*c).to_owned()));
    let all: Vec<Option<&str>> = items.iter().map(|(_, v)| *v).collect();
    let total = figures(&all, None)?;
    out.skipped = total.skipped;
    if !grouped {
        out.rows.push(row_of(None, items.len(), &total));
        return Ok(out);
    }
    // Groups by their text as written, spaces round it dropped; empty is its own.
    let mut names: Vec<String> = Vec::new();
    let mut members: Vec<Vec<Option<&str>>> = Vec::new();
    let mut empty: Vec<Option<&str>> = Vec::new();
    let mut empty_count = 0;
    for (g, v) in items {
        let key = g
            .map(|g| g.trim_matches(is_js_space))
            .filter(|g| !g.is_empty());
        match key {
            None => {
                empty.push(*v);
                empty_count += 1;
            }
            Some(k) => match names.iter().position(|n| n == k) {
                Some(i) => members[i].push(*v),
                None => {
                    names.push(k.to_owned());
                    members.push(vec![*v]);
                }
            },
        }
    }
    let scale = Some(
        all.iter()
            .flatten()
            .filter_map(|v| read_number(v))
            .map(|d| d.scale)
            .max()
            .unwrap_or(0)
            + 2,
    );
    for i in natural_order(&names) {
        let i = i as usize;
        let f = figures(&members[i], scale)?;
        out.rows.push(row_of(Some(&names[i]), members[i].len(), &f));
    }
    if empty_count > 0 {
        let f = figures(&empty, scale)?;
        out.rows.push(row_of(Some(EMPTY_GROUP), empty_count, &f));
    }
    out.rows.push(row_of(Some(TOTAL), items.len(), &total));
    Ok(out)
}

/// A key as Anahtarla birleştir compares it (docs/adr/0200 §6): the
/// spaces round it dropped; a number's canonical text (`007` and `7`
/// alike, `#` before it so it never meets a text); empty is no key.
pub fn join_key(text: &str) -> Option<String> {
    let t = text.trim_matches(is_js_space);
    if t.is_empty() {
        return None;
    }
    Some(match read_number(t) {
        Some(d) => format!("#{}", canonical_number(d)),
        None => format!("${t}"),
    })
}

/// A number's text without the scale's trailing zeros (`1.50` and `1.5` alike).
fn canonical_number(d: Dec) -> String {
    let t = d.text();
    if t.contains('.') {
        let t = t.trim_end_matches('0').trim_end_matches('.');
        if t == "-0" {
            "0".to_owned()
        } else {
            t.to_owned()
        }
    } else {
        t
    }
}

/// How the targets meet the source rows: per target the source row it
/// takes (none: unmatched), the source keys met more than once (the first
/// row taken), and the source rows no target took.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct JoinPlan {
    pub matches: Vec<Option<usize>>,
    pub repeated: usize,
    pub unused: usize,
}

pub fn join_plan(targets: &[Option<&str>], sources: &[Option<&str>]) -> JoinPlan {
    let mut first: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut repeated = std::collections::HashSet::new();
    for (i, s) in sources.iter().enumerate() {
        let Some(k) = s.and_then(join_key) else {
            continue;
        };
        match first.entry(k) {
            std::collections::hash_map::Entry::Occupied(e) => {
                repeated.insert(e.key().clone());
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(i);
            }
        }
    }
    let matches: Vec<Option<usize>> = targets
        .iter()
        .map(|t| t.and_then(join_key).and_then(|k| first.get(&k).copied()))
        .collect();
    let used: std::collections::HashSet<usize> = matches.iter().flatten().copied().collect();
    let keyed = sources
        .iter()
        .filter(|s| s.and_then(join_key).is_some())
        .count();
    JoinPlan {
        unused: keyed - used.len() - repeated_rows(sources, &first),
        repeated: repeated.len(),
        matches,
    }
}

/// Source rows past the first of a repeated key: never taken, not counted as unused.
fn repeated_rows(
    sources: &[Option<&str>],
    first: &std::collections::HashMap<String, usize>,
) -> usize {
    sources
        .iter()
        .enumerate()
        .filter(|(i, s)| {
            s.and_then(join_key)
                .is_some_and(|k| first.get(&k) != Some(i))
        })
        .count()
}

// ── Apportioning (docs/adr/0201 §5) ──────────────────────────────────

/// `a` × `b` (`b` below 2⁶⁴) as a 256-bit number: its high and low halves.
fn mul_wide(a: u128, b: u64) -> (u128, u128) {
    let (a_hi, a_lo) = (a >> 64, a & u128::from(u64::MAX));
    let lo_part = a_lo * u128::from(b);
    let hi_part = a_hi * u128::from(b);
    let (lo, carry) = lo_part.overflowing_add(hi_part << 64);
    ((hi_part >> 64) + u128::from(carry), lo)
}

/// The 256-bit number's lowest `s` bits (s ≤ 256).
fn low_bits(hi: u128, lo: u128, s: u32) -> (u128, u128) {
    match s {
        0 => (0, 0),
        1..=127 => (0, lo & ((1u128 << s) - 1)),
        128 => (0, lo),
        129..=255 => (hi & ((1u128 << (s - 128)) - 1), lo),
        _ => (hi, lo),
    }
}

/// The 256-bit number over 2^`s`, half to even; none when it does not fit 128 bits.
fn shr_even(hi: u128, lo: u128, s: u32) -> Option<u128> {
    if s == 0 {
        return (hi == 0).then_some(lo);
    }
    // The number is below 2¹⁹² (a value's 100 bits, a double's 53 and 100's 7).
    if s > 200 {
        return Some(0);
    }
    let (q_hi, q_lo) = if s >= 128 {
        (0, hi >> (s - 128))
    } else {
        (hi >> s, (lo >> s) | (hi << (128 - s)))
    };
    if q_hi != 0 {
        return None;
    }
    let half = if s > 128 {
        (1u128 << (s - 1 - 128), 0)
    } else {
        (0, 1u128 << (s - 1))
    };
    let up = match low_bits(hi, lo, s).cmp(&half) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => q_lo & 1 == 1,
    };
    if up { q_lo.checked_add(1) } else { Some(q_lo) }
}

/// A value's share of a piece (Kesişim's and Birleşim's Alan oranıyla
/// paylaştır, docs/adr/0201 §5; the rule's addendum): the value read as a
/// decimal number times `share` (the double, exactly), rounded half to even
/// at two more fraction digits than the value has. None when the value is
/// not read as a number, the share is not a number from 0 up, or the result
/// does not fit.
pub fn apportion(value: &str, share: f64) -> Option<String> {
    let d = read_number(value)?;
    if !(share >= 0.0) || !share.is_finite() {
        return None;
    }
    let scale = d.scale + 2;
    if share == 0.0 || d.m == 0 {
        return Some(Dec { m: 0, scale }.text());
    }
    // The share as k · 2^e exactly, k odd.
    let bits = share.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    let (mut k, mut e) = if exp == 0 {
        (frac, -1074)
    } else {
        (frac | (1u64 << 52), exp - 1075)
    };
    while k % 2 == 0 {
        k /= 2;
        e += 1;
    }
    // |m| · 100 · k (below 2¹⁹²), then times 2^e.
    let (hi, lo) = mul_wide(d.m.unsigned_abs(), k.checked_mul(100)?);
    let q = if e >= 0 {
        let e = e as u32;
        if hi != 0 || e >= 128 || lo.leading_zeros() <= e {
            return None;
        }
        lo << e
    } else {
        shr_even(hi, lo, e.unsigned_abs())?
    };
    let m = i128::try_from(q).ok()?;
    Some(
        Dec {
            m: if d.m < 0 { -m } else { m },
            scale,
        }
        .text(),
    )
}

// ── The web's calls (`model/ops/statistics.ts`) ─────────────────────────

/// One group's statistic for Bilgi al: its text (none: nothing to write) and the values skipped;
/// `statisticMany` takes each group's scale (the target's field's, for Ortalama).
#[derive(Clone, Debug, PartialEq)]
pub struct StatValue {
    pub value: Option<String>,
    pub skipped: usize,
}

crate::json_struct!(out StatValue { value, skipped });
crate::json_struct!(out Summary { columns, rows, skipped });
crate::json_struct!(out JoinPlan { matches, repeated, unused });

pub(crate) const OPS: &[Op] = &[
    op!(
        "statisticMany",
        |groups: Vec<Vec<Option<String>>>, stat: String, scales: Vec<Option<usize>>| {
            let stat = Stat::from_key(&stat).ok_or("bilinmeyen istatistik")?;
            groups
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let values: Vec<Option<&str>> = g.iter().map(|v| v.as_deref()).collect();
                    let scale = scales.get(i).copied().flatten().map(|s| s as u32);
                    statistic(&values, stat, scale)
                        .map(|(value, skipped)| StatValue { value, skipped })
                })
                .collect::<Result<Vec<StatValue>, String>>()
        }
    ),
    op!(
        "summarizeValues",
        |groups: Vec<Option<String>>, values: Vec<Option<String>>, grouped: bool| {
            let items: Vec<(Option<&str>, Option<&str>)> = values
                .iter()
                .enumerate()
                .map(|(i, v)| (groups.get(i).and_then(|g| g.as_deref()), v.as_deref()))
                .collect();
            summarize(&items, grouped)
        }
    ),
    op!("apportion", |values: Vec<Option<String>>, share: f64| {
        values
            .iter()
            .map(|v| v.as_deref().and_then(|v| apportion(v, share)))
            .collect::<Vec<Option<String>>>()
    }),
    op!(
        "joinPlan",
        |targets: Vec<Option<String>>, sources: Vec<Option<String>>| {
            let t: Vec<Option<&str>> = targets.iter().map(|v| v.as_deref()).collect();
            let s: Vec<Option<&str>> = sources.iter().map(|v| v.as_deref()).collect();
            join_plan(&t, &s)
        }
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_and_add_exactly() {
        let d = |t: &str| read_number(t).map(Dec::text);
        assert_eq!(d(" 1,50 "), Some("1.50".into()));
        assert_eq!(d("-0.0"), Some("0.0".into()));
        assert_eq!(d("+007"), Some("7".into()));
        assert_eq!(d("1e3"), None);
        assert_eq!(d("1.2.3"), None);
        let f = figures(&[Some("0.1"), Some("0.2"), Some("x"), None], None).expect("figures");
        assert_eq!(f.sum.map(Dec::text), Some("0.3".into()));
        assert_eq!(f.mean.map(Dec::text), Some("0.150".into()));
        assert_eq!(f.skipped, 1);
        // Half to even: 0.125 → 0.12, 0.135 → 0.14.
        let mean = |vals: &[&str], s: u32| {
            figures(&vals.iter().map(|v| Some(*v)).collect::<Vec<_>>(), Some(s))
                .expect("figures")
                .mean
                .map(Dec::text)
        };
        assert_eq!(mean(&["0.125"], 2), Some("0.12".into()));
        assert_eq!(mean(&["0.135"], 2), Some("0.14".into()));
        assert_eq!(mean(&["1", "2"], 0), Some("2".into()));
        assert_eq!(mean(&["1", "4"], 0), Some("2".into()));
    }

    #[test]
    fn keys_meet_as_numbers_or_texts() {
        assert_eq!(join_key(" 007 "), join_key("7"));
        assert_eq!(join_key("1.50"), join_key("1,5"));
        assert_ne!(join_key("7a"), join_key("7"));
        let p = join_plan(
            &[Some("7"), Some("8"), None],
            &[Some("007"), Some("9"), Some("7")],
        );
        assert_eq!(p.matches, vec![Some(0), None, None]);
        assert_eq!((p.repeated, p.unused), (1, 1));
    }
}
