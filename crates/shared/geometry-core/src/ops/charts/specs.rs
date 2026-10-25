//! What to chart when nothing was chosen yet (docs/adr/0227 §4): a spec
//! made whole for its kind and its data (the columns it needs, the first
//! that fit when not given), a column's chart (İstatistik's Grafik) and an
//! İşlemler table's (its tool's suggestion when the table has its columns).
//! Both platforms ask these, so a chart starts alike on each.

use super::data::{DataSet, Kind};
use super::model::{Aggregate, ChartKind, Sort, Spec};

/// The aggregates Çubuk and Pasta offer.
pub const BAR_AGGREGATES: [Aggregate; 6] = [
    Aggregate::Count,
    Aggregate::Sum,
    Aggregate::Mean,
    Aggregate::Min,
    Aggregate::Max,
    Aggregate::Median,
];
pub const PIE_AGGREGATES: [Aggregate; 2] = [Aggregate::Count, Aggregate::Sum];

/// A spec made whole for its kind: the columns it needs, the first that fit
/// when not given; an aggregate the kind does not offer becomes the count.
pub fn complete(spec: &Spec, data: &DataSet) -> Spec {
    let columns = &data.columns;
    let numbers: Vec<&str> = columns
        .iter()
        .filter(|c| c.kind == Kind::Number)
        .map(|c| c.key.as_str())
        .collect();
    let has = |k: &Option<String>| {
        k.as_ref()
            .is_some_and(|k| columns.iter().any(|c| &c.key == k))
    };
    let is_number = |k: Option<&String>| k.is_some_and(|k| numbers.contains(&k.as_str()));
    let first_category = columns
        .iter()
        .find(|c| c.kind != Kind::Number || c.labelled)
        .or(columns.first())
        .map(|c| c.key.clone());
    let first_number = numbers.first().map(|k| (*k).to_owned());
    // Y's first: a number column other than X, else the first.
    let other_than = |x: &Option<String>| {
        numbers
            .iter()
            .find(|k| Some(**k) != x.as_deref())
            .or(numbers.first())
            .map(|k| vec![(*k).to_owned()])
            .unwrap_or_default()
    };
    let mut s = spec.clone();
    match spec.kind {
        ChartKind::Bar | ChartKind::Pie => {
            if !has(&s.category) {
                s.category = first_category;
            }
            let allowed: &[Aggregate] = if spec.kind == ChartKind::Pie {
                &PIE_AGGREGATES
            } else {
                &BAR_AGGREGATES
            };
            if !allowed.contains(&s.aggregate) {
                s.aggregate = Aggregate::Count;
            }
            if !is_number(s.value.as_ref()) {
                s.value = first_number;
            }
            if s.value.is_none() {
                s.aggregate = Aggregate::Count;
            }
        }
        ChartKind::Histogram => {
            if !is_number(s.value.as_ref()) {
                s.value = first_number;
            }
        }
        ChartKind::Box => {
            if !is_number(s.value.as_ref()) {
                s.value = first_number;
            }
            if !has(&s.group) {
                s.group = None;
            }
        }
        ChartKind::Scatter => {
            if !is_number(s.x.as_ref()) {
                s.x = first_number;
            }
            if !is_number(s.y.first()) {
                s.y = other_than(&s.x);
            }
        }
        ChartKind::Line => {
            if !is_number(s.x.as_ref()) {
                s.x = first_number;
            }
            s.y.retain(|k| numbers.contains(&k.as_str()));
            if s.y.is_empty() {
                s.y = other_than(&s.x);
            }
            if s.series.is_some() && (!has(&s.series) || s.y.len() > 1) {
                s.series = None;
            }
        }
    }
    s
}

/// İstatistik's Grafik for a column: a number's histogram, anything else's bars by count.
pub fn for_column(data: &DataSet, key: &str) -> Spec {
    let number = data
        .find(key)
        .is_some_and(|i| data.columns[i].kind == Kind::Number);
    if number {
        Spec {
            value: Some(key.to_owned()),
            ..Spec::new(ChartKind::Histogram)
        }
    } else {
        Spec {
            category: Some(key.to_owned()),
            ..Spec::new(ChartKind::Bar)
        }
    }
}

/// An İşlemler table's chart: its tool's suggestion when the table has the
/// columns it names (ROC ile doğrulama's curve, Kesit's profiles, Zaman
/// serisi's series, Histogram's intervals, DBSCAN taraması's k-distance graph), else bars by the first column
/// and the sum of the first other number column (else the count).
pub fn for_table(tool: &str, data: &DataSet) -> Spec {
    let has = |k: &str| data.find(k).is_some();
    let named = |k: &str| Some(k.to_owned());
    match tool {
        "suitability.roc" => {
            let x = ["Alan oranı (%)", "Yanlış pozitif oranı (%)"]
                .into_iter()
                .find(|k| has(k));
            if let Some(x) = x
                && has("Doğru pozitif oranı (%)")
            {
                return Spec {
                    x: named(x),
                    y: vec!["Doğru pozitif oranı (%)".to_owned()],
                    ..Spec::new(ChartKind::Line)
                };
            }
        }
        "multidim.profile" if has("Uzaklık (m)") && has("Değer") && has("Çizgi") => {
            return Spec {
                x: named("Uzaklık (m)"),
                y: vec!["Değer".to_owned()],
                series: named("Çizgi"),
                ..Spec::new(ChartKind::Line)
            };
        }
        // A series by time (Adım, Zaman, points) or by band (Bant, points): each point a line.
        "multidim.series" => {
            if let Some(x) = data.columns.first() {
                let y: Vec<String> = data
                    .columns
                    .iter()
                    .skip(1)
                    .filter(|c| c.kind == Kind::Number)
                    .take(8)
                    .map(|c| c.key.clone())
                    .collect();
                if x.kind == Kind::Number && !y.is_empty() {
                    return Spec {
                        x: Some(x.key.clone()),
                        y,
                        ..Spec::new(ChartKind::Line)
                    };
                }
            }
        }
        // DBSCAN's sweep's k-distance graph (docs/adr/0639 §3): the distance by rank, a line.
        "stats.dbscanSweep" if has("Sıra") => {
            if let Some(y) = data.columns.get(1).filter(|c| c.kind == Kind::Number) {
                return Spec {
                    x: named("Sıra"),
                    y: vec![y.key.clone()],
                    ..Spec::new(ChartKind::Line)
                };
            }
        }
        "raster.histogram" if has("Alt sınır") && has("Sayı") => {
            return Spec {
                category: named("Alt sınır"),
                value: named("Sayı"),
                aggregate: Aggregate::Sum,
                sort: Sort::Category,
                ..Spec::new(ChartKind::Bar)
            };
        }
        _ => {}
    }
    let value = data
        .columns
        .iter()
        .skip(1)
        .find(|c| c.kind == Kind::Number)
        .map(|c| c.key.clone());
    Spec {
        category: data.columns.first().map(|c| c.key.clone()),
        aggregate: if value.is_some() {
            Aggregate::Sum
        } else {
            Aggregate::Count
        },
        value,
        ..Spec::new(ChartKind::Bar)
    }
}
