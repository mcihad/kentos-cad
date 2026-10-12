//! A field's statistics (docs/adr/0227 §3): the İstatistik tab's summary row
//! of every column and the chosen column's full list, its values with their
//! counts and shares. Numbers follow `kentos.statistics/1` (exact sums, the
//! mean half to even at the inputs' largest scale plus two); the quartiles
//! are Hyndman and Fan's seventh (NumPy's, R's, QUARTILE.INC), exact at the
//! mean's scale since their fraction is a quarter's multiple.

use super::data::{Column, EMPTY, FastMap, INVALID, Key, Kind, Read, Val, dec_text};
use crate::ops::statistics::{Dec, Figures, TOO_LARGE, compare, figures_of};
use crate::time::civil_from_days;

/// Most values a field's list names; the rest are counted.
pub const MAX_VALUES: usize = 500;

/// A column's row in the İstatistik tab's summary table.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Summary {
    pub key: String,
    pub label: String,
    pub kind: String,
    pub objects: usize,
    pub filled: usize,
    pub empty: usize,
    pub invalid: usize,
    pub distinct: usize,
    pub min: Option<String>,
    pub max: Option<String>,
    pub mean: Option<String>,
    pub sum: Option<String>,
}

/// One statistic of the full list: its name and its text.
#[derive(Clone, Debug, PartialEq)]
pub struct Stat {
    pub label: String,
    pub value: String,
}

/// What a value-list row counts: a value, the refused ones, the empty ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Value,
    Invalid,
    Empty,
}

impl Mark {
    pub fn word(self) -> &'static str {
        match self {
            Mark::Value => "value",
            Mark::Invalid => "invalid",
            Mark::Empty => "empty",
        }
    }
}

/// A row of a field's value list: the value as shown, its count, its share
/// of the objects (%, one fraction digit) and its rows.
#[derive(Clone, Debug, PartialEq)]
pub struct ValueRow {
    pub text: String,
    pub count: usize,
    pub share: String,
    pub mark: Mark,
    pub rows: Vec<u32>,
}

/// A field's full statistics.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Detail {
    pub summary: Summary,
    pub stats: Vec<Stat>,
    pub values: Vec<ValueRow>,
    /// Distinct values past the list's [`MAX_VALUES`].
    pub more: usize,
    pub notes: Vec<String>,
}

/// `count` of `total` as a percentage with one fraction digit, half to even (exact).
pub fn share(count: usize, total: usize) -> String {
    if total == 0 {
        return "0.0".to_owned();
    }
    let (n, d) = (count as u128 * 1000, total as u128);
    let (q, r) = (n / d, n % d);
    let tenths = match (2 * r).cmp(&d) {
        std::cmp::Ordering::Greater => q + 1,
        std::cmp::Ordering::Less => q,
        std::cmp::Ordering::Equal => q + (q % 2),
    };
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// A date's days as GG.AA.YYYY.
pub fn date_text(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{d:02}.{m:02}.{y:04}")
}

/// The p-quartile (p = k/4) of exact values sorted ascending, all at
/// `scale`: Hyndman and Fan's seventh definition, at `scale` + 2.
pub fn quartile(sorted: &[i128], k: u32) -> Option<i128> {
    let n = sorted.len();
    if n == 0 {
        return None;
    }
    let h4 = (n as u128 - 1) * u128::from(k);
    let (i, rem) = ((h4 / 4) as usize, (h4 % 4) as i128);
    let base = sorted[i].checked_mul(100)?;
    if rem == 0 {
        return Some(base);
    }
    let diff = sorted[i + 1].checked_sub(sorted[i])?;
    base.checked_add(diff.checked_mul(25 * rem)?)
}

/// The numbers sorted ascending as integers at their largest scale.
fn sorted_numbers(nums: &[Dec]) -> (Vec<i128>, u32) {
    let scale = nums.iter().map(|d| d.scale).max().unwrap_or(0);
    let mut ints: Vec<i128> = nums
        .iter()
        .filter_map(|d| d.at(scale).map(|d| d.m))
        .collect();
    ints.sort_unstable();
    (ints, scale)
}

/// Counts of a column's rows by key, in the column's order: (key, rows).
fn counted(read: &Read, rows: &[u32]) -> Vec<(Key, Vec<u32>)> {
    super::data::groups(read, rows)
}

/// The summary row of a column over these rows.
pub fn summary(col: &Column, read: &Read, rows: &[u32]) -> Summary {
    summary_of(col, read, rows).0
}

/// The summary row, the numbers read and their figures (the full list takes them too).
fn summary_of(
    col: &Column,
    read: &Read,
    rows: &[u32],
) -> (Summary, Vec<Dec>, Option<Result<Figures, String>>) {
    let mut s = Summary {
        key: col.key.clone(),
        label: col.label.clone(),
        kind: col.kind.label().to_owned(),
        objects: rows.len(),
        ..Summary::default()
    };
    let mut seen: FastMap<Key, ()> = FastMap::default();
    let mut nums: Vec<Dec> = Vec::new();
    let mut dates: (Option<i64>, Option<i64>) = (None, None);
    for &r in rows {
        let v = read.vals[r as usize];
        match v {
            Val::Empty => s.empty += 1,
            Val::Invalid(_) => s.invalid += 1,
            _ => {
                s.filled += 1;
                seen.insert(read.key(v), ());
                match v {
                    Val::Num(d) => nums.push(d),
                    Val::Date(d) => {
                        dates.0 = Some(dates.0.map_or(d, |m: i64| m.min(d)));
                        dates.1 = Some(dates.1.map_or(d, |m: i64| m.max(d)));
                    }
                    _ => {}
                }
            }
        }
    }
    s.distinct = seen.len();
    let mut figures = None;
    match col.kind {
        Kind::Number => {
            let f = figures_of(&nums, None);
            if let Ok(f) = &f {
                s.sum = f.sum.map(Dec::text);
                s.mean = f.mean.map(Dec::text);
                s.min = f.min.map(Dec::text);
                s.max = f.max.map(Dec::text);
            } else {
                s.min = nums
                    .iter()
                    .copied()
                    .min_by(|a, b| compare(*a, *b))
                    .map(Dec::text);
                s.max = nums
                    .iter()
                    .copied()
                    .max_by(|a, b| compare(*a, *b))
                    .map(Dec::text);
            }
            figures = Some(f);
        }
        Kind::Date => {
            s.min = dates.0.map(|d| read.show(Key::Date(d)));
            s.max = dates.1.map(|d| read.show(Key::Date(d)));
        }
        Kind::Text | Kind::Boolean => {}
    }
    (s, nums, figures)
}

fn stat(label: &str, value: impl Into<String>) -> Stat {
    Stat {
        label: label.to_owned(),
        value: value.into(),
    }
}

/// A column's full statistics over these rows.
pub fn detail(col: &Column, read: &Read, rows: &[u32]) -> Detail {
    let (summary, nums, figures) = summary_of(col, read, rows);
    let mut out = Detail {
        summary: summary.clone(),
        ..Detail::default()
    };
    let groups = counted(read, rows);
    let values: Vec<&(Key, Vec<u32>)> = groups
        .iter()
        .filter(|(k, _)| !matches!(k, Key::Empty | Key::Invalid(_)))
        .collect();
    // En sık: the most frequent value, the column's order among equals; none when every value is met once.
    let mode = values
        .iter()
        .fold(None::<&(Key, Vec<u32>)>, |best, g| match best {
            Some(b) if b.1.len() >= g.1.len() => Some(b),
            _ => Some(g),
        })
        .filter(|g| g.1.len() > 1);

    out.stats.push(stat("Nesne", summary.objects.to_string()));
    out.stats.push(stat("Dolu", summary.filled.to_string()));
    out.stats.push(stat("Boş", summary.empty.to_string()));
    if summary.invalid > 0 {
        out.stats.push(stat("Uymayan", summary.invalid.to_string()));
    }
    out.stats
        .push(stat("Benzersiz", summary.distinct.to_string()));
    match col.kind {
        Kind::Number => number_stats(&nums, figures, &mut out),
        Kind::Date => {
            let mut days: Vec<i64> = rows
                .iter()
                .filter_map(|&r| match read.vals[r as usize] {
                    Val::Date(d) => Some(d),
                    _ => None,
                })
                .collect();
            days.sort_unstable();
            if let (Some(&first), Some(&last)) = (days.first(), days.last()) {
                let median = days[(days.len() - 1) / 2];
                out.stats
                    .push(stat("En erken", read.show(Key::Date(first))));
                out.stats
                    .push(stat("Ortanca", read.show(Key::Date(median))));
                out.stats.push(stat("En geç", read.show(Key::Date(last))));
                out.stats
                    .push(stat("Aralık (gün)", (last - first).to_string()));
            }
        }
        Kind::Boolean => {
            let count = |b: bool| {
                rows.iter()
                    .filter(|&&r| read.vals[r as usize] == Val::Bool(b))
                    .count()
            };
            out.stats.push(stat("Evet", count(true).to_string()));
            out.stats.push(stat("Hayır", count(false).to_string()));
        }
        Kind::Text => {
            let lengths: Vec<usize> = rows
                .iter()
                .filter_map(|&r| match read.vals[r as usize] {
                    Val::Text(i) => Some(read.texts[i as usize].chars().count()),
                    _ => None,
                })
                .collect();
            if let (Some(min), Some(max)) = (lengths.iter().min(), lengths.iter().max()) {
                out.stats.push(stat("En kısa", format!("{min} karakter")));
                out.stats.push(stat("En uzun", format!("{max} karakter")));
            }
        }
    }
    if let Some((key, members)) = mode {
        out.stats.push(stat(
            "En sık",
            format!("{} ({} nesne)", read.show(*key), members.len()),
        ));
    }

    // The value list: counts from most to fewest, the column's order among equals (the groups
    // come in the column's order; the sort keeps the order of equals).
    let mut listed: Vec<&(Key, Vec<u32>)> = values.clone();
    listed.sort_by_key(|g| std::cmp::Reverse(g.1.len()));
    out.more = listed.len().saturating_sub(MAX_VALUES);
    for (key, members) in listed.into_iter().take(MAX_VALUES) {
        out.values.push(ValueRow {
            text: read.show(*key),
            count: members.len(),
            share: share(members.len(), rows.len()),
            mark: Mark::Value,
            rows: members.clone(),
        });
    }
    let invalid: Vec<u32> = rows
        .iter()
        .copied()
        .filter(|&r| matches!(read.vals[r as usize], Val::Invalid(_)))
        .collect();
    if !invalid.is_empty() {
        out.values.push(ValueRow {
            text: INVALID.to_owned(),
            count: invalid.len(),
            share: share(invalid.len(), rows.len()),
            mark: Mark::Invalid,
            rows: invalid,
        });
    }
    let empty: Vec<u32> = rows
        .iter()
        .copied()
        .filter(|&r| read.vals[r as usize] == Val::Empty)
        .collect();
    if !empty.is_empty() {
        out.values.push(ValueRow {
            text: EMPTY.to_owned(),
            count: empty.len(),
            share: share(empty.len(), rows.len()),
            mark: Mark::Empty,
            rows: empty,
        });
    }
    out
}

/// A number column's statistics: Toplam … Çeyrekler arası (its numbers and
/// their figures as the summary read them).
fn number_stats(nums: &[Dec], figures: Option<Result<Figures, String>>, out: &mut Detail) {
    if nums.is_empty() {
        return;
    }
    let figures = match figures.unwrap_or_else(|| figures_of(nums, None)) {
        Ok(f) => f,
        Err(e) => {
            out.notes.push(e);
            Default::default()
        }
    };
    if let Some(sum) = figures.sum {
        out.stats.push(stat("Toplam", sum.text()));
    }
    if let Some(mean) = figures.mean {
        out.stats.push(stat("Ortalama", mean.text()));
    }
    if let Some(std) = figures.std {
        out.stats.push(stat("Std. sapma", std));
    }
    let (sorted, scale) = sorted_numbers(nums);
    if sorted.len() != nums.len() {
        out.notes.push(TOO_LARGE.to_owned());
        return;
    }
    let q = |k| quartile(&sorted, k).map(|m| dec_text(m, scale + 2));
    let (min, max) = (sorted[0], sorted[sorted.len() - 1]);
    // The least and the greatest as they were written (the summary's); the range at the largest scale.
    if let Some(m) = figures.min {
        out.stats.push(stat("En az", m.text()));
    }
    if let Some(q1) = q(1) {
        out.stats.push(stat("Birinci çeyrek", q1));
    }
    if let Some(med) = q(2) {
        out.stats.push(stat("Ortanca", med));
    }
    if let Some(q3) = q(3) {
        out.stats.push(stat("Üçüncü çeyrek", q3));
    }
    if let Some(m) = figures.max {
        out.stats.push(stat("En çok", m.text()));
    }
    if let Some(range) = max.checked_sub(min) {
        out.stats.push(stat("Aralık", dec_text(range, scale)));
    }
    if let (Some(q1), Some(q3)) = (quartile(&sorted, 1), quartile(&sorted, 3))
        && let Some(iqr) = q3.checked_sub(q1)
    {
        out.stats
            .push(stat("Çeyrekler arası", dec_text(iqr, scale + 2)));
    }
}
