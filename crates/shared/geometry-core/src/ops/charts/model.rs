//! The six charts' models (docs/adr/0227 §4): what a chart counts from its
//! rows, before any layout. Categories are a column's keys in its order
//! (`data::groups`); the aggregates follow `kentos.statistics/1` (the mean
//! and the median at the chart's largest scale plus two). Every part a
//! click selects is an [`Element`] with its rows.

use std::collections::HashMap;

use super::data::{DataSet, EMPTY, Key, Read, Val, groups, top_scale};
use super::fields::quartile;
use super::ticks::sig4;
use crate::ops::statistics::{Dec, compare, figures_of};

/// Most bars (the last “Diğer” when there are more categories).
pub const MAX_BARS: usize = 30;
/// Most boxes.
pub const MAX_BOXES: usize = 30;
/// The categorical colours: a pie's slices and a line chart's series.
pub const MAX_SERIES: usize = 8;
/// The automatic histogram's most intervals; the most asked.
pub const AUTO_BINS: u32 = 50;
pub const MAX_BINS: u32 = 100;
/// The folded categories' name.
pub const OTHER: &str = "Diğer";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartKind {
    Bar,
    Pie,
    Histogram,
    Box,
    Scatter,
    Line,
}

impl ChartKind {
    pub const ALL: [ChartKind; 6] = [
        ChartKind::Bar,
        ChartKind::Pie,
        ChartKind::Histogram,
        ChartKind::Box,
        ChartKind::Scatter,
        ChartKind::Line,
    ];

    pub fn word(self) -> &'static str {
        match self {
            ChartKind::Bar => "bar",
            ChartKind::Pie => "pie",
            ChartKind::Histogram => "histogram",
            ChartKind::Box => "box",
            ChartKind::Scatter => "scatter",
            ChartKind::Line => "line",
        }
    }

    pub fn from_word(word: &str) -> Option<ChartKind> {
        ChartKind::ALL.into_iter().find(|k| k.word() == word)
    }

    pub fn label(self) -> &'static str {
        match self {
            ChartKind::Bar => "Çubuk",
            ChartKind::Pie => "Pasta",
            ChartKind::Histogram => "Histogram",
            ChartKind::Box => "Kutu",
            ChartKind::Scatter => "Saçılım",
            ChartKind::Line => "Çizgi",
        }
    }
}

/// How a category's (or an X value's) rows give one number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aggregate {
    /// Each row a point (Çizgi only).
    None,
    Count,
    Sum,
    Mean,
    Min,
    Max,
    Median,
}

impl Aggregate {
    pub const ALL: [Aggregate; 7] = [
        Aggregate::None,
        Aggregate::Count,
        Aggregate::Sum,
        Aggregate::Mean,
        Aggregate::Min,
        Aggregate::Max,
        Aggregate::Median,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Aggregate::None => "none",
            Aggregate::Count => "count",
            Aggregate::Sum => "sum",
            Aggregate::Mean => "mean",
            Aggregate::Min => "min",
            Aggregate::Max => "max",
            Aggregate::Median => "median",
        }
    }

    pub fn from_word(word: &str) -> Option<Aggregate> {
        Aggregate::ALL.into_iter().find(|a| a.word() == word)
    }

    pub fn label(self) -> &'static str {
        match self {
            Aggregate::None => "Yok",
            Aggregate::Count => "Sayı",
            Aggregate::Sum => "Toplam",
            Aggregate::Mean => "Ortalama",
            Aggregate::Min => "En az",
            Aggregate::Max => "En çok",
            Aggregate::Median => "Ortanca",
        }
    }
}

/// Çubuk's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Category,
    Desc,
    Asc,
}

impl Sort {
    pub const ALL: [Sort; 3] = [Sort::Category, Sort::Desc, Sort::Asc];

    pub fn word(self) -> &'static str {
        match self {
            Sort::Category => "category",
            Sort::Desc => "desc",
            Sort::Asc => "asc",
        }
    }

    pub fn from_word(word: &str) -> Option<Sort> {
        Sort::ALL.into_iter().find(|s| s.word() == word)
    }

    pub fn label(self) -> &'static str {
        match self {
            Sort::Category => "Kategori",
            Sort::Desc => "Değer azalan",
            Sort::Asc => "Değer artan",
        }
    }
}

/// What to chart: the columns by their keys and the choices (docs/adr/0227 §4).
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub kind: ChartKind,
    pub category: Option<String>,
    pub value: Option<String>,
    pub aggregate: Aggregate,
    pub sort: Sort,
    /// Histogram's intervals; none: Sturges.
    pub bins: Option<u32>,
    /// Histogram's mean and median lines.
    pub guides: bool,
    pub group: Option<String>,
    pub x: Option<String>,
    pub y: Vec<String>,
    pub series: Option<String>,
    /// Saçılım's trend line.
    pub trend: bool,
}

impl Spec {
    /// A kind's spec with its defaults: a line chart's points each a row
    /// (Toplama Yok), the others counting.
    pub fn new(kind: ChartKind) -> Spec {
        Spec {
            kind,
            category: None,
            value: None,
            aggregate: if kind == ChartKind::Line {
                Aggregate::None
            } else {
                Aggregate::Count
            },
            sort: Sort::Category,
            bins: None,
            guides: true,
            group: None,
            x: None,
            y: Vec::new(),
            series: None,
            trend: true,
        }
    }
}

/// A colour: a categorical slot (0–7) or the folded categories' grey.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Series(u8),
    Other,
}

/// A part a click selects: its rows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Element {
    pub rows: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BarItem {
    pub label: String,
    pub value: Option<Dec>,
    pub objects: usize,
    pub other: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SliceItem {
    pub label: String,
    pub value: Dec,
    pub share: String,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinItem {
    pub lo: String,
    pub hi: String,
    pub lo_x: f64,
    pub hi_x: f64,
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoxItem {
    pub label: String,
    pub n: usize,
    pub min: Dec,
    pub q1: Dec,
    pub median: Dec,
    pub q3: Dec,
    pub max: Dec,
    pub low: Dec,
    pub high: Dec,
    pub mean: Option<Dec>,
    /// Its outliers: their value and their element.
    pub outliers: Vec<(Dec, usize)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trend {
    pub a: f64,
    pub b: f64,
    pub r2: Option<f64>,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinePoint {
    pub x: Dec,
    pub y: Dec,
    pub element: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeriesItem {
    pub label: String,
    pub color: Color,
    pub points: Vec<LinePoint>,
}

/// What a chart draws, by its kind.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Bars {
        bars: Vec<BarItem>,
        integer: bool,
    },
    Pie {
        slices: Vec<SliceItem>,
        total: Dec,
        /// The centre's word: Toplam or Nesne.
        word: String,
    },
    Histogram {
        bins: Vec<BinItem>,
        min: Dec,
        max: Dec,
        mean: Option<Dec>,
        median: Option<Dec>,
        guides: bool,
    },
    Boxes {
        boxes: Vec<BoxItem>,
    },
    Scatter {
        x: Vec<f64>,
        y: Vec<f64>,
        x_range: Option<(Dec, Dec)>,
        y_range: Option<(Dec, Dec)>,
        /// Whether every X (Y) is a whole number: the axis then steps by wholes.
        x_whole: bool,
        y_whole: bool,
        trend: Option<Trend>,
    },
    Lines {
        series: Vec<SeriesItem>,
        x_range: Option<(Dec, Dec)>,
        y_range: Option<(Dec, Dec)>,
    },
}

/// The chart's numbers as a table (Tablo, Panoya kopyala).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// A chart worked out.
#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    pub kind: ChartKind,
    pub x_title: String,
    pub y_title: String,
    pub notes: Vec<String>,
    pub elements: Vec<Element>,
    pub body: Body,
    pub table: Table,
}

fn d(m: i128, scale: u32) -> Dec {
    Dec { m, scale }
}

/// The numbers of these rows of a column, with their rows.
fn numbers(read: &Read, rows: &[u32]) -> Vec<(Dec, u32)> {
    rows.iter()
        .filter_map(|&r| read.number(r as usize).map(|n| (n, r)))
        .collect()
}

/// An aggregate of these rows' numbers; `s` the chart's largest scale.
fn aggregate(agg: Aggregate, read: Option<&Read>, rows: &[u32], s: u32) -> Option<Dec> {
    if agg == Aggregate::Count {
        return Some(d(rows.len() as i128, 0));
    }
    let read = read?;
    let nums: Vec<Dec> = numbers(read, rows).into_iter().map(|(n, _)| n).collect();
    if nums.is_empty() {
        return None;
    }
    match agg {
        Aggregate::Median => {
            let mut ints: Vec<i128> = nums.iter().filter_map(|n| n.at(s).map(|x| x.m)).collect();
            if ints.len() != nums.len() {
                return None;
            }
            ints.sort_unstable();
            quartile(&ints, 2).map(|m| d(m, s + 2))
        }
        _ => {
            let f = figures_of(&nums, Some(s + 2)).ok()?;
            match agg {
                Aggregate::Sum => f.sum,
                Aggregate::Mean => f.mean,
                Aggregate::Min => f.min,
                Aggregate::Max => f.max,
                _ => None,
            }
        }
    }
}

/// How many rows have a value that is not a number.
fn not_numbers(read: &Read, rows: &[u32]) -> usize {
    rows.iter()
        .filter(|&&r| !matches!(read.vals[r as usize], Val::Num(_)))
        .count()
}

fn skipped_note(n: usize, what: &str) -> Option<String> {
    (n > 0).then(|| format!("{n} nesnenin {what} sayı değil: hesaba katılmadı."))
}

/// `value / total` as a percentage, one fraction digit, half to even (exact; both positive).
fn dec_share(value: Dec, total: Dec) -> String {
    let s = value.scale.max(total.scale);
    let (Some(v), Some(t)) = (value.at(s), total.at(s)) else {
        return String::new();
    };
    if t.m <= 0 {
        return String::new();
    }
    let n = v.m.checked_mul(1000);
    let Some(n) = n else {
        return String::new();
    };
    let (q, r) = (n / t.m, n % t.m);
    let tenths = match (2 * r).cmp(&t.m) {
        std::cmp::Ordering::Greater => q + 1,
        std::cmp::Ordering::Less => q,
        std::cmp::Ordering::Equal => q + (q % 2),
    };
    format!("{}.{}", tenths / 10, tenths % 10)
}

fn text_of(v: Option<Dec>) -> String {
    v.map(Dec::text).unwrap_or_default()
}

fn exact_range(values: impl Iterator<Item = Dec>) -> Option<(Dec, Dec)> {
    let mut out: Option<(Dec, Dec)> = None;
    for v in values {
        out = Some(match out {
            None => (v, v),
            Some((lo, hi)) => (
                if compare(v, lo).is_lt() { v } else { lo },
                if compare(v, hi).is_gt() { v } else { hi },
            ),
        });
    }
    out
}

/// The data's reads, one per column, and the rows a chart takes.
pub struct Input<'a> {
    pub data: &'a DataSet,
    pub reads: &'a [Read],
    /// The rows Göster chose.
    pub rows: &'a [u32],
    /// Every row (a pie's colours follow them).
    pub all: &'a [u32],
}

impl Input<'_> {
    fn column(&self, key: &Option<String>) -> Option<usize> {
        key.as_deref().and_then(|k| self.data.find(k))
    }

    fn label(&self, i: usize) -> &str {
        &self.data.columns[i].label
    }
}

/// The chart a spec asks of these rows; none when its columns are missing.
pub fn build(spec: &Spec, input: &Input) -> Result<Model, String> {
    match spec.kind {
        ChartKind::Bar => bars(spec, input),
        ChartKind::Pie => pie(spec, input),
        ChartKind::Histogram => histogram(spec, input),
        ChartKind::Box => boxes(spec, input),
        ChartKind::Scatter => scatter(spec, input),
        ChartKind::Line => lines(spec, input),
    }
}

fn missing(what: &str) -> String {
    format!("Grafiğin {what} seçilmedi.")
}

fn value_needed(agg: Aggregate) -> bool {
    !matches!(agg, Aggregate::Count | Aggregate::None)
}

fn bars(spec: &Spec, input: &Input) -> Result<Model, String> {
    let cat = input
        .column(&spec.category)
        .ok_or_else(|| missing("kategorisi"))?;
    let agg = match spec.aggregate {
        Aggregate::None => Aggregate::Count,
        a => a,
    };
    let value = if value_needed(agg) {
        Some(input.column(&spec.value).ok_or_else(|| missing("değeri"))?)
    } else {
        None
    };
    let cat_read = &input.reads[cat];
    let value_read = value.map(|v| &input.reads[v]);
    let s = value_read.map_or(0, |r| top_scale(r, input.rows));
    let mut cats: Vec<(String, Vec<u32>, Option<Dec>)> = groups(cat_read, input.rows)
        .into_iter()
        .map(|(key, rows)| {
            let v = aggregate(agg, value_read, &rows, s);
            (cat_read.show(key), rows, v)
        })
        .collect();
    match spec.sort {
        Sort::Category => {}
        Sort::Desc | Sort::Asc => {
            let desc = spec.sort == Sort::Desc;
            // Stable: equals keep the category order; the ones without a value last.
            cats.sort_by(|a, b| match (a.2, b.2) {
                (Some(x), Some(y)) => {
                    let o = compare(x, y);
                    if desc { o.reverse() } else { o }
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            });
        }
    }
    let mut notes = Vec::new();
    let mut folded = false;
    if cats.len() > MAX_BARS {
        folded = true;
        let rest: Vec<(String, Vec<u32>, Option<Dec>)> = cats.split_off(MAX_BARS - 1);
        let mut rows: Vec<u32> = rest.iter().flat_map(|c| c.1.iter().copied()).collect();
        rows.sort_unstable();
        let v = aggregate(agg, value_read, &rows, s);
        cats.push((format!("{OTHER} ({})", rest.len()), rows, v));
        notes.push(format!(
            "{} kategori “{OTHER}” çubuğunda toplandı.",
            rest.len()
        ));
    }
    if let (Some(r), true) = (value_read, value_needed(agg))
        && let Some(n) = skipped_note(not_numbers(r, input.rows), "değeri")
    {
        notes.push(n);
    }
    let mut items = Vec::new();
    let mut elements = Vec::new();
    let mut table = Table {
        columns: vec![input.label(cat).to_owned(), "Nesne".to_owned()],
        rows: Vec::new(),
    };
    if agg != Aggregate::Count {
        table.columns.push(agg.label().to_owned());
    }
    let last = cats.len().saturating_sub(1);
    for (i, (label, rows, v)) in cats.into_iter().enumerate() {
        let mut row = vec![label.clone(), rows.len().to_string()];
        if agg != Aggregate::Count {
            row.push(text_of(v));
        }
        table.rows.push(row);
        items.push(BarItem {
            label,
            value: v,
            objects: rows.len(),
            other: folded && i == last,
        });
        elements.push(Element { rows });
    }
    let y_title = match value {
        Some(v) => format!("{} · {}", agg.label(), input.label(v)),
        None => "Nesne sayısı".to_owned(),
    };
    Ok(Model {
        kind: ChartKind::Bar,
        x_title: input.label(cat).to_owned(),
        y_title,
        notes,
        elements,
        body: Body::Bars {
            bars: items,
            integer: agg == Aggregate::Count
                || (matches!(agg, Aggregate::Sum | Aggregate::Min | Aggregate::Max) && s == 0),
        },
        table,
    })
}

fn pie(spec: &Spec, input: &Input) -> Result<Model, String> {
    let cat = input
        .column(&spec.category)
        .ok_or_else(|| missing("kategorisi"))?;
    let agg = if spec.aggregate == Aggregate::Count || spec.aggregate == Aggregate::None {
        Aggregate::Count
    } else {
        Aggregate::Sum
    };
    let value = if agg == Aggregate::Sum {
        Some(input.column(&spec.value).ok_or_else(|| missing("değeri"))?)
    } else {
        None
    };
    let cat_read = &input.reads[cat];
    let value_read = value.map(|v| &input.reads[v]);
    // The colours follow the categories over every row: their values largest first.
    let mut ranked: Vec<(Key, Dec)> = groups(cat_read, input.all)
        .into_iter()
        .filter_map(|(key, rows)| {
            aggregate(agg, value_read, &rows, 0)
                .filter(|v| v.m > 0)
                .map(|v| (key, v))
        })
        .collect();
    ranked.sort_by(|a, b| compare(b.1, a.1));
    let fold = ranked.len() > MAX_SERIES;
    let colours: HashMap<Key, (usize, Color)> = ranked
        .iter()
        .enumerate()
        .map(|(i, (k, _))| {
            let c = if fold && i >= MAX_SERIES - 1 {
                Color::Other
            } else {
                Color::Series(i as u8)
            };
            (*k, (i, c))
        })
        .collect();
    let mut slices: Vec<(usize, String, Vec<u32>, Dec, Color)> = Vec::new();
    let mut other_rows: Vec<u32> = Vec::new();
    let mut other_count = 0usize;
    let mut refused = 0usize;
    for (key, rows) in groups(cat_read, input.rows) {
        let Some(v) = aggregate(agg, value_read, &rows, 0).filter(|v| v.m > 0) else {
            refused += 1;
            continue;
        };
        match colours.get(&key) {
            Some(&(_, Color::Other)) => {
                other_rows.extend(rows);
                other_count += 1;
            }
            Some(&(rank, c)) => slices.push((rank, cat_read.show(key), rows, v, c)),
            None => refused += 1,
        }
    }
    slices.sort_by_key(|s| s.0);
    if !other_rows.is_empty() {
        other_rows.sort_unstable();
        if let Some(v) = aggregate(agg, value_read, &other_rows, 0) {
            slices.push((
                usize::MAX,
                format!("{OTHER} ({other_count})"),
                other_rows,
                v,
                Color::Other,
            ));
        }
    }
    let mut total = d(0, 0);
    for s in &slices {
        let t = total.scale.max(s.3.scale);
        if let (Some(a), Some(b)) = (total.at(t), s.3.at(t)) {
            total = d(a.m + b.m, t);
        }
    }
    let mut notes = Vec::new();
    if refused > 0 {
        notes.push(format!(
            "{refused} kategorinin değeri sıfır, eksi ya da yok: dilimi çizilmedi."
        ));
    }
    if let Some(r) = value_read
        && let Some(n) = skipped_note(not_numbers(r, input.rows), "değeri")
    {
        notes.push(n);
    }
    let mut table = Table {
        columns: vec![
            input.label(cat).to_owned(),
            match value {
                Some(v) => format!("{} · {}", agg.label(), input.label(v)),
                None => "Nesne".to_owned(),
            },
            "Pay (%)".to_owned(),
        ],
        rows: Vec::new(),
    };
    let mut items = Vec::new();
    let mut elements = Vec::new();
    for (_, label, rows, v, color) in slices {
        let share = dec_share(v, total);
        table
            .rows
            .push(vec![label.clone(), v.text(), share.clone()]);
        items.push(SliceItem {
            label,
            value: v,
            share,
            color,
        });
        elements.push(Element { rows });
    }
    Ok(Model {
        kind: ChartKind::Pie,
        x_title: input.label(cat).to_owned(),
        y_title: match value {
            Some(v) => format!("{} · {}", agg.label(), input.label(v)),
            None => "Nesne sayısı".to_owned(),
        },
        notes,
        elements,
        body: Body::Pie {
            slices: items,
            total,
            word: if agg == Aggregate::Count {
                "Nesne"
            } else {
                "Toplam"
            }
            .to_owned(),
        },
        table,
    })
}

/// Sturges' intervals: ⌈log₂ n⌉ + 1, at most [`AUTO_BINS`].
pub fn sturges(n: usize) -> u32 {
    let mut p = 0u32;
    while (1usize << p) < n {
        p += 1;
    }
    (p + 1).min(AUTO_BINS)
}

/// `num / den` half to even (den > 0).
fn div_even(num: i128, den: i128) -> i128 {
    let (q, r) = (num.div_euclid(den), num.rem_euclid(den));
    match (2 * r).cmp(&den) {
        std::cmp::Ordering::Greater => q + 1,
        std::cmp::Ordering::Less => q,
        std::cmp::Ordering::Equal => q + (q.rem_euclid(2)),
    }
}

fn histogram(spec: &Spec, input: &Input) -> Result<Model, String> {
    let col = input.column(&spec.value).ok_or_else(|| missing("değeri"))?;
    let read = &input.reads[col];
    let nums = numbers(read, input.rows);
    let mut notes: Vec<String> = skipped_note(not_numbers(read, input.rows), "değeri")
        .into_iter()
        .collect();
    let label = input.label(col).to_owned();
    let mut table = Table {
        columns: vec![
            "Alt sınır".to_owned(),
            "Üst sınır".to_owned(),
            "Sayı".to_owned(),
        ],
        rows: Vec::new(),
    };
    if nums.is_empty() {
        notes.push("Sayı değeri olan nesne yok.".to_owned());
        return Ok(Model {
            kind: ChartKind::Histogram,
            x_title: label,
            y_title: "Nesne sayısı".to_owned(),
            notes,
            elements: Vec::new(),
            body: Body::Histogram {
                bins: Vec::new(),
                min: d(0, 0),
                max: d(0, 0),
                mean: None,
                median: None,
                guides: spec.guides,
            },
            table,
        });
    }
    let s = nums.iter().map(|(n, _)| n.scale).max().unwrap_or(0);
    let ints: Vec<(i128, u32)> = nums
        .iter()
        .filter_map(|(n, r)| n.at(s).map(|x| (x.m, *r)))
        .collect();
    if ints.len() != nums.len() {
        return Err(crate::ops::statistics::TOO_LARGE.to_owned());
    }
    let (a, b) = ints
        .iter()
        .fold((i128::MAX, i128::MIN), |(lo, hi), (x, _)| {
            (lo.min(*x), hi.max(*x))
        });
    let k: i128 = if a == b {
        1
    } else {
        i128::from(
            spec.bins
                .map(|k| k.clamp(1, MAX_BINS))
                .unwrap_or_else(|| sturges(nums.len())),
        )
    };
    let mut members: Vec<Vec<u32>> = vec![Vec::new(); k as usize];
    for (x, r) in &ints {
        let i = if a == b {
            0
        } else {
            ((k * (x - a)).div_euclid(b - a)).min(k - 1)
        };
        members[i as usize].push(*r);
    }
    let edge = |j: i128| -> i128 {
        // (a·k + j·(b − a)) / k at s + 2, half to even.
        div_even((a * k + j * (b - a)) * 100, k)
    };
    let (af, bf) = (d(a, s).to_f64(), d(b, s).to_f64());
    let mut bins = Vec::new();
    let mut elements = Vec::new();
    for (j, rows) in members.into_iter().enumerate() {
        let j = j as i128;
        let lo = d(edge(j), s + 2).text();
        let hi = d(edge(j + 1), s + 2).text();
        let frac = |t: i128| af + (bf - af) * (t as f64) / (k as f64);
        table
            .rows
            .push(vec![lo.clone(), hi.clone(), rows.len().to_string()]);
        bins.push(BinItem {
            lo,
            hi,
            lo_x: frac(j),
            hi_x: frac(j + 1),
            count: rows.len(),
        });
        elements.push(Element { rows });
    }
    let nums_only: Vec<Dec> = nums.iter().map(|(n, _)| *n).collect();
    let mean = figures_of(&nums_only, Some(s + 2))
        .ok()
        .and_then(|f| f.mean);
    let mut sorted: Vec<i128> = ints.iter().map(|(x, _)| *x).collect();
    sorted.sort_unstable();
    let median = quartile(&sorted, 2).map(|m| d(m, s + 2));
    Ok(Model {
        kind: ChartKind::Histogram,
        x_title: label,
        y_title: "Nesne sayısı".to_owned(),
        notes,
        elements,
        body: Body::Histogram {
            bins,
            min: d(a, s),
            max: d(b, s),
            mean,
            median,
            guides: spec.guides,
        },
        table,
    })
}

fn boxes(spec: &Spec, input: &Input) -> Result<Model, String> {
    let col = input.column(&spec.value).ok_or_else(|| missing("değeri"))?;
    let read = &input.reads[col];
    let group = input.column(&spec.group);
    let s = top_scale(read, input.rows);
    let mut parts: Vec<(String, Vec<u32>)> = match group {
        Some(g) => {
            let gr = &input.reads[g];
            groups(gr, input.rows)
                .into_iter()
                .map(|(k, rows)| (gr.show(k), rows))
                .collect()
        }
        None => vec![(input.label(col).to_owned(), input.rows.to_vec())],
    };
    let mut notes: Vec<String> = skipped_note(not_numbers(read, input.rows), "değeri")
        .into_iter()
        .collect();
    if parts.len() > MAX_BOXES {
        let hidden = parts.len() - MAX_BOXES;
        parts.truncate(MAX_BOXES);
        notes.push(format!(
            "{hidden} grup gösterilmedi (en çok {MAX_BOXES} kutu)."
        ));
    }
    let mut table = Table {
        columns: [
            "Grup",
            "Nesne",
            "En az",
            "Birinci çeyrek",
            "Ortanca",
            "Üçüncü çeyrek",
            "En çok",
            "Ortalama",
            "Aykırı",
        ]
        .iter()
        .map(|c| (*c).to_owned())
        .collect(),
        rows: Vec::new(),
    };
    if group.is_none() {
        table.columns.remove(0);
    }
    let mut items = Vec::new();
    let mut elements: Vec<Element> = Vec::new();
    let mut outlier_elements: Vec<(usize, Vec<(Dec, u32)>)> = Vec::new();
    for (label, rows) in parts {
        let nums = numbers(read, &rows);
        let mut ints: Vec<(i128, u32)> = nums
            .iter()
            .filter_map(|(n, r)| n.at(s).map(|x| (x.m, *r)))
            .collect();
        ints.sort_by_key(|p| p.0);
        let box_index = items.len();
        elements.push(Element { rows: rows.clone() });
        let sorted: Vec<i128> = ints.iter().map(|p| p.0).collect();
        let (Some(q1), Some(med), Some(q3)) = (
            quartile(&sorted, 1),
            quartile(&sorted, 2),
            quartile(&sorted, 3),
        ) else {
            // A group without numbers: its name stays, no box.
            items.push(BoxItem {
                label,
                n: 0,
                min: d(0, 0),
                q1: d(0, 0),
                median: d(0, 0),
                q3: d(0, 0),
                max: d(0, 0),
                low: d(0, 0),
                high: d(0, 0),
                mean: None,
                outliers: Vec::new(),
            });
            continue;
        };
        let iqr = q3 - q1;
        // The fences at s + 3: Q1 − 1.5·IQR, Q3 + 1.5·IQR.
        let (lo_f, hi_f) = (q1 * 10 - 15 * iqr, q3 * 10 + 15 * iqr);
        let inside: Vec<i128> = sorted
            .iter()
            .copied()
            .filter(|x| x * 1000 >= lo_f && x * 1000 <= hi_f)
            .collect();
        let low = inside.first().copied().unwrap_or(sorted[0]);
        let high = inside.last().copied().unwrap_or(sorted[sorted.len() - 1]);
        let outs: Vec<(Dec, u32)> = ints
            .iter()
            .filter(|(x, _)| x * 1000 < lo_f || x * 1000 > hi_f)
            .map(|(x, r)| (d(*x, s), *r))
            .collect();
        let nums_only: Vec<Dec> = nums.iter().map(|(n, _)| *n).collect();
        let mean = figures_of(&nums_only, Some(s + 2))
            .ok()
            .and_then(|f| f.mean);
        let item = BoxItem {
            label: label.clone(),
            n: sorted.len(),
            min: d(sorted[0], s),
            q1: d(q1, s + 2),
            median: d(med, s + 2),
            q3: d(q3, s + 2),
            max: d(sorted[sorted.len() - 1], s),
            low: d(low, s),
            high: d(high, s),
            mean,
            outliers: Vec::new(),
        };
        let mut row = vec![
            label,
            item.n.to_string(),
            item.min.text(),
            item.q1.text(),
            item.median.text(),
            item.q3.text(),
            item.max.text(),
            text_of(item.mean),
            outs.len().to_string(),
        ];
        if group.is_none() {
            row.remove(0);
        }
        table.rows.push(row);
        items.push(item);
        outlier_elements.push((box_index, outs));
    }
    // The outliers' elements after the boxes', each its one row.
    for (box_index, outs) in outlier_elements {
        for (v, r) in outs {
            let e = elements.len();
            elements.push(Element { rows: vec![r] });
            items[box_index].outliers.push((v, e));
        }
    }
    Ok(Model {
        kind: ChartKind::Box,
        x_title: group.map(|g| input.label(g).to_owned()).unwrap_or_default(),
        y_title: input.label(col).to_owned(),
        notes,
        elements,
        body: Body::Boxes { boxes: items },
        table,
    })
}

/// The least squares line through the points, and its words.
pub fn trend_of(x: &[f64], y: &[f64]) -> Option<Trend> {
    let n = x.len();
    if n < 2 {
        return None;
    }
    let nf = n as f64;
    let mx = x.iter().fold(0.0, |s, v| s + v) / nf;
    let my = y.iter().fold(0.0, |s, v| s + v) / nf;
    let (mut sxx, mut sxy, mut syy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (dx, dy) = (x[i] - mx, y[i] - my);
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx == 0.0 {
        return None;
    }
    let b = sxy / sxx;
    let a = my - b * mx;
    let r2 = (syy != 0.0).then(|| sxy * sxy / (sxx * syy));
    let sign = if a < 0.0 { "-" } else { "+" };
    let mut text = format!("y = {}x {} {}", sig4(b), sign, sig4(a.abs()));
    if let Some(r2) = r2 {
        text.push_str(&format!("; R² = {}", sig4(r2)));
    }
    Some(Trend { a, b, r2, text })
}

fn scatter(spec: &Spec, input: &Input) -> Result<Model, String> {
    let xc = input.column(&spec.x).ok_or_else(|| missing("X'i"))?;
    let yc = input
        .column(&spec.y.first().cloned())
        .ok_or_else(|| missing("Y'si"))?;
    let (xr, yr) = (&input.reads[xc], &input.reads[yc]);
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut xd = Vec::new();
    let mut yd = Vec::new();
    let mut elements = Vec::new();
    let mut table = Table {
        columns: vec![input.label(xc).to_owned(), input.label(yc).to_owned()],
        rows: Vec::new(),
    };
    let mut skipped = 0;
    for &r in input.rows {
        match (xr.number(r as usize), yr.number(r as usize)) {
            (Some(x), Some(y)) => {
                xs.push(x.to_f64());
                ys.push(y.to_f64());
                xd.push(x);
                yd.push(y);
                table.rows.push(vec![x.text(), y.text()]);
                elements.push(Element { rows: vec![r] });
            }
            _ => skipped += 1,
        }
    }
    let notes: Vec<String> = skipped_note(skipped, "X'i ya da Y'si")
        .into_iter()
        .collect();
    let trend = if spec.trend { trend_of(&xs, &ys) } else { None };
    let (x_whole, y_whole) = (
        xd.iter().all(|d| d.scale == 0),
        yd.iter().all(|d| d.scale == 0),
    );
    Ok(Model {
        kind: ChartKind::Scatter,
        x_title: input.label(xc).to_owned(),
        y_title: input.label(yc).to_owned(),
        notes,
        elements,
        body: Body::Scatter {
            x: xs,
            y: ys,
            x_range: exact_range(xd.into_iter()),
            y_range: exact_range(yd.into_iter()),
            x_whole,
            y_whole,
            trend,
        },
        table,
    })
}

fn lines(spec: &Spec, input: &Input) -> Result<Model, String> {
    let xc = input.column(&spec.x).ok_or_else(|| missing("X'i"))?;
    let ys: Vec<usize> = spec.y.iter().filter_map(|k| input.data.find(k)).collect();
    let agg = spec.aggregate;
    if ys.is_empty() && agg != Aggregate::Count {
        return Err(missing("Y'si"));
    }
    let xr = &input.reads[xc];
    let series_col = if ys.len() <= 1 {
        input.column(&spec.series)
    } else {
        None
    };
    // The series: by a column's categories, or one per Y column.
    let mut sets: Vec<(String, Option<usize>, Vec<u32>)> = match series_col {
        Some(sc) => {
            let sr = &input.reads[sc];
            groups(sr, input.rows)
                .into_iter()
                .map(|(k, rows)| (sr.show(k), ys.first().copied(), rows))
                .collect()
        }
        None if ys.is_empty() => vec![(input.label(xc).to_owned(), None, input.rows.to_vec())],
        None => ys
            .iter()
            .map(|&y| (input.label(y).to_owned(), Some(y), input.rows.to_vec()))
            .collect(),
    };
    let mut notes = Vec::new();
    if sets.len() > MAX_SERIES {
        let hidden = sets.len() - MAX_SERIES;
        sets.truncate(MAX_SERIES);
        notes.push(format!(
            "{hidden} seri gösterilmedi (en çok {MAX_SERIES} seri)."
        ));
    }
    let s = ys
        .iter()
        .map(|&y| top_scale(&input.reads[y], input.rows))
        .max()
        .unwrap_or(0);
    let mut skipped = 0;
    let mut series = Vec::new();
    let mut elements: Vec<Element> = Vec::new();
    let y_label = match (agg, ys.len()) {
        (Aggregate::Count, _) => "Nesne sayısı".to_owned(),
        (Aggregate::None, 1) => input.label(ys[0]).to_owned(),
        (_, 1) => format!("{} · {}", agg.label(), input.label(ys[0])),
        (Aggregate::None, _) => "Değer".to_owned(),
        (a, _) => a.label().to_owned(),
    };
    let mut table = Table {
        columns: vec![
            "Seri".to_owned(),
            input.label(xc).to_owned(),
            y_label.clone(),
        ],
        rows: Vec::new(),
    };
    for (i, (label, y, rows)) in sets.into_iter().enumerate() {
        let yr = y.map(|y| &input.reads[y]);
        let mut points: Vec<(Dec, Option<Dec>, u32)> = Vec::new();
        for &r in &rows {
            let x = xr.number(r as usize);
            let v = yr.and_then(|yr| yr.number(r as usize));
            match (x, v, agg) {
                (Some(x), _, Aggregate::Count) => points.push((x, None, r)),
                (Some(x), Some(v), _) => points.push((x, Some(v), r)),
                _ => skipped += 1,
            }
        }
        // By X's exact value; equal X keep the rows' order.
        points.sort_by(|a, b| compare(a.0, b.0));
        let mut out: Vec<LinePoint> = Vec::new();
        if agg == Aggregate::None {
            for (x, v, r) in points {
                let Some(v) = v else { continue };
                let e = elements.len();
                elements.push(Element { rows: vec![r] });
                table.rows.push(vec![label.clone(), x.text(), v.text()]);
                out.push(LinePoint {
                    x,
                    y: v,
                    element: e,
                });
            }
        } else {
            let mut start = 0;
            while start < points.len() {
                let mut end = start + 1;
                while end < points.len() && compare(points[end].0, points[start].0).is_eq() {
                    end += 1;
                }
                let rows: Vec<u32> = points[start..end].iter().map(|p| p.2).collect();
                let v = aggregate(agg, yr, &rows, s);
                if let Some(v) = v {
                    let e = elements.len();
                    elements.push(Element { rows });
                    table
                        .rows
                        .push(vec![label.clone(), points[start].0.text(), v.text()]);
                    out.push(LinePoint {
                        x: points[start].0,
                        y: v,
                        element: e,
                    });
                }
                start = end;
            }
        }
        series.push(SeriesItem {
            label,
            color: Color::Series(i as u8),
            points: out,
        });
    }
    if let Some(n) = skipped_note(skipped, "X'i ya da Y'si") {
        notes.push(n);
    }
    let x_range = exact_range(series.iter().flat_map(|s| s.points.iter().map(|p| p.x)));
    let y_range = exact_range(series.iter().flat_map(|s| s.points.iter().map(|p| p.y)));
    if series.len() == 1 && series_col.is_none() {
        table.columns.remove(0);
        for r in &mut table.rows {
            r.remove(0);
        }
    }
    Ok(Model {
        kind: ChartKind::Line,
        x_title: input.label(xc).to_owned(),
        y_title: y_label,
        notes,
        elements,
        body: Body::Lines {
            series,
            x_range,
            y_range,
        },
        table,
    })
}

/// Which elements hold a selected row (`selected` by row).
pub fn marks(model: &Model, selected: &[bool]) -> Vec<bool> {
    model
        .elements
        .iter()
        .map(|e| {
            e.rows
                .iter()
                .any(|&r| selected.get(r as usize).copied().unwrap_or(false))
        })
        .collect()
}

/// The empty group's word, for the layout.
pub fn empty_word() -> &'static str {
    EMPTY
}

/// A box's five values and the others as the tooltip lists them.
pub fn box_tip(b: &BoxItem) -> Vec<(String, String)> {
    let mut out = vec![("Nesne".to_owned(), b.n.to_string())];
    if b.n == 0 {
        return out;
    }
    out.extend([
        ("Üst bıyık".to_owned(), b.high.text()),
        ("Üçüncü çeyrek".to_owned(), b.q3.text()),
        ("Ortanca".to_owned(), b.median.text()),
        ("Birinci çeyrek".to_owned(), b.q1.text()),
        ("Alt bıyık".to_owned(), b.low.text()),
    ]);
    if let Some(m) = b.mean {
        out.push(("Ortalama".to_owned(), m.text()));
    }
    out.push(("Aykırı".to_owned(), b.outliers.len().to_string()));
    out
}

/// What an element's tooltip says (docs/adr/0227 §6): its value first, its name after.
pub fn tip(model: &Model, element: usize, selected: usize) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    match &model.body {
        Body::Bars { bars, .. } => {
            if let Some(b) = bars.get(element) {
                out.push((model.y_title.clone(), text_of(b.value)));
                out.push((model.x_title.clone(), b.label.clone()));
                out.push(("Nesne".to_owned(), b.objects.to_string()));
            }
        }
        Body::Pie { slices, .. } => {
            if let Some(s) = slices.get(element) {
                out.push((model.y_title.clone(), s.value.text()));
                out.push(("Pay".to_owned(), format!("%{}", s.share)));
                out.push((model.x_title.clone(), s.label.clone()));
            }
        }
        Body::Histogram { bins, .. } => {
            if let Some(b) = bins.get(element) {
                out.push(("Nesne".to_owned(), b.count.to_string()));
                let last = element + 1 == bins.len();
                out.push((
                    "Aralık".to_owned(),
                    format!("[{}, {}{}", b.lo, b.hi, if last { "]" } else { ")" }),
                ));
            }
        }
        Body::Boxes { boxes } => {
            if let Some(b) = boxes.get(element) {
                out.extend(box_tip(b));
                if !model.x_title.is_empty() {
                    out.push((model.x_title.clone(), b.label.clone()));
                }
            } else if let Some((b, v)) = boxes
                .iter()
                .find_map(|b| b.outliers.iter().find(|o| o.1 == element).map(|o| (b, o.0)))
            {
                out.push(("Aykırı değer".to_owned(), v.text()));
                if !model.x_title.is_empty() {
                    out.push((model.x_title.clone(), b.label.clone()));
                }
            }
        }
        Body::Scatter { .. } => {
            if let Some(row) = model.table.rows.get(element) {
                out.push((model.x_title.clone(), row[0].clone()));
                out.push((model.y_title.clone(), row[1].clone()));
            }
        }
        Body::Lines { series, .. } => {
            for s in series {
                if let Some(p) = s.points.iter().find(|p| p.element == element) {
                    out.push((model.y_title.clone(), p.y.text()));
                    out.push((model.x_title.clone(), p.x.text()));
                    if series.len() > 1 {
                        out.push(("Seri".to_owned(), s.label.clone()));
                    }
                }
            }
        }
    }
    if selected > 0 {
        out.push(("Seçili".to_owned(), selected.to_string()));
    }
    out
}
