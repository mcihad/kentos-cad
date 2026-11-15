//! An extension panel's chart (docs/adr/8057 §25): the rows a command answered, read as KentOS's charts read an
//! İşlemler table (`table_columns`: each column's kind from its cells), worked out by the panel's `chart` (its kind,
//! category, value and aggregate) and painted as the Grafik tab paints its own (`charts::canvas::Still`).
//!
//! - **Worked out when its rows come**, not at every frame: a chart's model (its categories counted, its values
//!   summed) is built once from the rows; only its layout follows the panel's width.
//! - **Its rows are a button's or another component's:** a button's `table` may name it; its `of` names a table, a
//!   tree, a card or a figure whose rows it draws too (Değer say's tree and its bars, one command's rows).
//! - Kept beside the open panels, by extension, panel and chart: a painted chart's cache is not a panel's state to
//!   copy.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{canvas, column};
use iced::{Element, Fill};
use kentos_extensions::ui_panel::{ChartAggregate, ChartKind as Drawn, Component};
use kentos_geometry_core::ops::charts::data::DataSet;
use kentos_geometry_core::ops::charts::{Aggregate, ChartKind, Spec, Work};
use kentos_ui::label;
use serde_json::Value;

use super::panels::OpenPanel;
use crate::app::{App, Message};
use crate::charts::canvas::{Painted, Still};

/// How tall a panel's chart is drawn.
const HEIGHT: f32 = 220.0;

/// A chart of a panel, worked out when its rows came.
pub(crate) struct PanelChart {
    pub(crate) work: RefCell<Work>,
    pub(crate) painted: Painted,
    /// Its rows' stamp: the chart is painted again when they change.
    pub(crate) stamp: u64,
    /// Why it draws nothing: a column it reads is not in the rows, none of them is a number …
    pub(crate) error: Option<String>,
    pub(crate) rows: usize,
}

/// A cell as an İşlemler table's: its text, a number as JSON writes it, none empty.
fn cell_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(true) => "true".to_owned(),
        Value::Bool(false) => "false".to_owned(),
        other => other.to_string(),
    }
}

/// The chart's spec as KentOS's charts take it: bars and a pie group `category` (counted, or `value` summed when it
/// is given), a histogram reads `value`'s numbers, a line `value` over `category`.
pub(crate) fn spec_of(
    kind: Option<Drawn>,
    category: Option<&str>,
    value: Option<&str>,
    aggregate: Option<ChartAggregate>,
) -> Spec {
    let kind = match kind.unwrap_or(Drawn::Bar) {
        Drawn::Bar => ChartKind::Bar,
        Drawn::Pie => ChartKind::Pie,
        Drawn::Histogram => ChartKind::Histogram,
        Drawn::Line => ChartKind::Line,
    };
    let aggregate = aggregate.map(|a| match a {
        ChartAggregate::Count => Aggregate::Count,
        ChartAggregate::Sum => Aggregate::Sum,
        ChartAggregate::Mean => Aggregate::Mean,
        ChartAggregate::Min => Aggregate::Min,
        ChartAggregate::Max => Aggregate::Max,
        ChartAggregate::Median => Aggregate::Median,
    });
    let mut spec = Spec::new(kind);
    match kind {
        ChartKind::Histogram => spec.value = value.map(str::to_owned),
        ChartKind::Line => {
            spec.x = category.map(str::to_owned);
            spec.y = value.map(str::to_owned).into_iter().collect();
            if let Some(a) = aggregate {
                spec.aggregate = a;
            }
        }
        _ => {
            spec.category = category.map(str::to_owned);
            spec.value = value.map(str::to_owned);
            spec.aggregate = aggregate.unwrap_or(if value.is_some() {
                Aggregate::Sum
            } else {
                Aggregate::Count
            });
        }
    }
    spec
}

impl PanelChart {
    /// The chart `spec` of `rows` (their `columns`), worked out now.
    pub(crate) fn of(spec: &Spec, columns: &[String], rows: &[Value]) -> PanelChart {
        static STAMPS: AtomicU64 = AtomicU64::new(1);
        let cells: Vec<Vec<String>> = rows
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|c| row.get(c).map(cell_text).unwrap_or_default())
                    .collect()
            })
            .collect();
        let mut work = Work::new(DataSet {
            rows: rows.len(),
            columns: kentos_interaction::chart_data::table_columns(columns, &cells),
        });
        let all: Vec<u32> = (0..rows.len() as u32).collect();
        // A column the chart reads that the rows lack is the extension's mistake: said by its name (the engine says
        // "not chosen", the Grafik tab's word).
        let read = spec
            .category
            .iter()
            .chain(&spec.value)
            .chain(&spec.x)
            .chain(&spec.y);
        let error = match read.into_iter().find(|c| !columns.contains(c)) {
            Some(missing) => Some(format!(
                "“{missing}” sütunu komutun satırlarında yok: grafik onu okur."
            )),
            None => work.chart(spec, &all).err(),
        };
        PanelChart {
            work: RefCell::new(work),
            painted: Painted::default(),
            stamp: STAMPS.fetch_add(1, Ordering::Relaxed),
            error,
            rows: rows.len(),
        }
    }
}

impl OpenPanel {
    /// The panel's charts that draw `output`'s rows (theirs, or another component's by `of`), each worked out from
    /// the rows `output` holds now: (its id, the chart).
    pub(crate) fn charts_of(&self, output: &str) -> Vec<(String, PanelChart)> {
        let Some((columns, rows)) = self.tables.get(output) else {
            return Vec::new();
        };
        self.definition
            .body
            .iter()
            .filter_map(|c| match c {
                Component::Chart {
                    id,
                    kind,
                    of,
                    category,
                    value,
                    aggregate,
                    ..
                } if of.as_deref().unwrap_or(id) == output => {
                    let spec = spec_of(*kind, category.as_deref(), value.as_deref(), *aggregate);
                    Some((id.clone(), PanelChart::of(&spec, columns, rows)))
                }
                _ => None,
            })
            .collect()
    }
}

impl App {
    /// The charts of the panel `ext`/`local` drawing `output`'s rows, worked out again (its rows just came).
    pub(crate) fn ext_panel_charts(&mut self, ext: &str, local: &str, output: &str) {
        let Some(panel) = self
            .extensions
            .panels
            .iter()
            .find(|p| p.ext == ext && p.local == local)
        else {
            return;
        };
        for (id, chart) in panel.charts_of(output) {
            self.extensions
                .panel_charts
                .insert((ext.to_owned(), local.to_owned(), id), chart);
        }
    }

    /// A panel's chart as it is drawn: its heading, then the chart, or why there is none yet.
    pub(super) fn panel_chart<'a>(
        &'a self,
        p: &'a OpenPanel,
        id: &str,
        words: Option<&'a str>,
        of: Option<&str>,
    ) -> Element<'a, Message> {
        let heading = label::caption(words.unwrap_or("Grafik").to_owned());
        let key = (p.ext.clone(), p.local.clone(), id.to_owned());
        let body: Element<'a, Message> = match self.extensions.panel_charts.get(&key) {
            None => label::muted(p.waiting_words(of.unwrap_or(id), "grafik olur")).into(),
            Some(chart) if chart.rows == 0 => label::muted("Grafik için satır yok.").into(),
            Some(PanelChart {
                error: Some(why), ..
            }) => label::muted(why.clone()).into(),
            Some(chart) => canvas(Still {
                work: &chart.work,
                painted: &chart.painted,
                stamp: chart.stamp,
                scale: self.viewport.scale_factor() as f32,
            })
            .width(Fill)
            .height(HEIGHT)
            .into(),
        };
        column![heading, body].spacing(4).into()
    }
}

#[cfg(test)]
mod tests {
    use kentos_geometry_core::ops::charts::model::Body;
    use serde_json::json;

    use super::*;

    /// A chart of a tree's rows: bars counting each value once the command answered; a column the rows lack said by
    /// its name; a pie summing a value.
    #[test]
    fn a_charts_bars_are_its_rows_counted_by_category() {
        let rows = vec![
            json!({ "uid": "a", "Değer": "Okul", "Nesne": "1" }),
            json!({ "uid": "b", "Değer": "Park", "Nesne": "2" }),
            json!({ "uid": "c", "Değer": "Okul", "Nesne": "3" }),
        ];
        let columns: Vec<String> = ["uid", "Değer", "Nesne"].map(str::to_owned).to_vec();
        let chart = PanelChart::of(&spec_of(None, Some("Değer"), None, None), &columns, &rows);
        assert_eq!(chart.error, None);
        let work = chart.work.borrow();
        let Some(Body::Bars { bars, integer }) = work.model().map(|m| &m.body) else {
            panic!("bars: {:?}", work.model().map(|m| m.kind));
        };
        let counted: Vec<(&str, usize)> = bars.iter().map(|b| (b.label.as_str(), b.objects)).collect();
        assert_eq!(counted, [("Okul", 2), ("Park", 1)]);
        assert!(*integer, "a count is whole");
        drop(work);
        let missing = PanelChart::of(&spec_of(None, Some("Mahalle"), None, None), &columns, &rows);
        assert_eq!(
            missing.error.as_deref(),
            Some("“Mahalle” sütunu komutun satırlarında yok: grafik onu okur.")
        );
        let areas = vec![
            json!({ "mahalle": "Merkez", "alan": 120.5 }),
            json!({ "mahalle": "Merkez", "alan": 79.5 }),
            json!({ "mahalle": "Yeni", "alan": 50 }),
        ];
        let pie = PanelChart::of(
            &spec_of(Some(Drawn::Pie), Some("mahalle"), Some("alan"), None),
            &["mahalle".to_owned(), "alan".to_owned()],
            &areas,
        );
        assert_eq!(pie.error, None, "a value given: summed");
        let work = pie.work.borrow();
        let Some(Body::Pie { total, .. }) = work.model().map(|m| &m.body) else {
            panic!("a pie");
        };
        assert_eq!(
            total.m as f64 / 10f64.powi(total.scale as i32),
            250.0,
            "120.5 + 79.5 + 50"
        );
    }
}
