//! İstatistik, the bottom panel's statistics tab (docs/adr/0227 §3, §8; the
//! web's `ui/bottom/StatisticsPanel.ts`): a layer's every field in a
//! summary table (dolu, boş, benzersiz, en az, en çok, ortalama, toplam),
//! the chosen field's full statistics and its values with their counts and
//! shares. A value's row selects its objects (Shift adds, Ctrl takes away).
//! The core counts (`ops::charts::fields`); the summaries are counted a few
//! fields a frame (8 ms at most, one at least) so a large layer never
//! freezes the window. The layer and Göster are Grafik's (charts/).

use std::time::{Duration, Instant};

use iced::Task;
use kentos_geometry_core::ops::charts::fields::{Detail, Summary};
use kentos_geometry_core::ops::charts::specs::for_column;
use kentos_interaction::Level;

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::charts::now::LayerKey;
use crate::features::Show;

mod source;
#[cfg(test)]
mod source_tests;
mod view;

pub(crate) use source::Whole;

/// The tab's words (the web's `STATISTICS_TEXTS`).
pub mod texts {
    pub const LAYER: &str = "Katman";
    pub const SHOW: &str = "Göster";
    pub const CHART: &str = "Grafik";
    pub const CHART_HINT: &str =
        "Seçilen alanın grafiği: sayılarda histogram, öbürlerinde değerlere göre çubuk";
    pub const COPY: &str = "Panoya kopyala";
    pub const COPY_HINT: &str =
        "Özet tabloyu ve seçilen alanın istatistiklerini sekmeyle ayrılmış olarak panoya kopyalar";
    pub const COPIED: &str = "İstatistikler panoya kopyalandı.";
    pub const NO_LAYERS: &str = "Çizimde katman yok.";
    pub const NONE: &str = "Katmanda nesne yok.";
    pub const PICK: &str = "Bütün istatistikleri ve değerleri için bir alan seçin.";
    pub const VALUES: &str = "Değerler";
    pub const GEOMETRY: &str = "Geometriden";
    pub fn more(n: usize) -> String {
        format!("+{n} değer daha")
    }
    pub fn objects(n: usize) -> String {
        format!("{n} nesne")
    }
    pub const COLUMNS: [&str; 9] = [
        "Alan",
        "Tür",
        "Dolu",
        "Boş",
        "Benzersiz",
        "En az",
        "En çok",
        "Ortalama",
        "Toplam",
    ];
}

/// How long a frame may count summaries: the rest wait for the next frame.
const FRAME_BUDGET: Duration = Duration::from_millis(8);

/// The tab's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Katman ▾: a layer by id.
    Layer(String),
    Show(Show),
    /// A summary row pressed: its field.
    Field(String),
    /// A value row pressed: its place in the chosen field's list.
    Value(usize),
    /// Grafik: the chosen field's chart in the Grafik tab.
    Chart,
    Copy,
    /// A frame: a few more summaries counted.
    Count,
    /// Kapsam: Kaynağın tümü (true) or Çalışma kopyası.
    Scope(bool),
    /// Durdur: the source's question stops.
    WholeStop,
    /// The source's answer for a layer and a field.
    WholeAnswered(String, String, Box<Result<kentos_commands::SourcesQueried, String>>),
}

/// Göster's rows as they depend on the selection and the view.
type RowsKey = (Show, u64, [u64; 4]);

/// What the tab shows, as last counted.
#[derive(Default)]
pub(crate) struct StatsNow {
    key: Option<(LayerKey, RowsKey)>,
    /// Göster's rows.
    pub rows: Vec<u32>,
    /// Each column's summary, counted as frames come.
    pub summaries: Vec<Option<Summary>>,
    /// Each column's key and whether it is the geometry's.
    pub keys: Vec<String>,
    pub labels: Vec<String>,
    /// The layer's objects.
    pub objects: usize,
    /// The chosen field's statistics, for the rows and the field it was counted for.
    pub detail: Option<Detail>,
    detail_for: Option<(LayerKey, RowsKey, String)>,
}

impl App {
    fn stats_rows_key(&self) -> RowsKey {
        let show = self.charts.show;
        let (table_stamp, table_view) = self.table_rows_stamp();
        let selection = match show {
            Show::Selected => self.selection.version(),
            Show::Table => table_stamp,
            _ => 0,
        };
        let view = if show == Show::Table {
            table_view
        } else if show == Show::InView {
            let v = self.viewport.camera.visible_bounds();
            [
                v.min_x.to_bits(),
                v.min_y.to_bits(),
                v.max_x.to_bits(),
                v.max_y.to_bits(),
            ]
        } else {
            [0; 4]
        };
        (show, selection, view)
    }

    /// The layer's data as the drawing is now (shared with Grafik), read again when it changed.
    fn stats_data_now(&self) -> Option<LayerKey> {
        let doc = self.document.as_ref()?;
        let layer = self.chart_layer()?;
        Some(self.layer_data_now(doc, &layer))
    }

    /// The tab's rows, summaries and chosen field as the drawing, the
    /// selection, the view and the choices are now: the summaries start
    /// again when the data or the rows changed; the chosen field's
    /// statistics are counted at once.
    pub(crate) fn stats_refresh(&self) {
        let p = &self.charts;
        let Some(data) = self.stats_data_now() else {
            *p.stats.borrow_mut() = StatsNow::default();
            return;
        };
        let rows_key = self.stats_rows_key();
        let mut now = p.stats.borrow_mut();
        let mut held = p.layer_data.borrow_mut();
        let Some((_, d)) = held.as_mut() else {
            return;
        };
        let key = (data.clone(), rows_key);
        if now.key.as_ref() != Some(&key) {
            let columns = &d.work.data.columns;
            *now = StatsNow {
                key: Some(key),
                rows: d.rows(self, p.show),
                summaries: vec![None; columns.len()],
                keys: columns.iter().map(|c| c.key.clone()).collect(),
                labels: columns.iter().map(|c| c.label.clone()).collect(),
                objects: d.slots.len(),
                ..StatsNow::default()
            };
        }
        let field = p.field.clone().filter(|f| now.keys.contains(f));
        match field {
            None => {
                now.detail = None;
                now.detail_for = None;
            }
            Some(field) => {
                let want = (data, rows_key, field.clone());
                if now.detail_for.as_ref() != Some(&want) {
                    let col = now.keys.iter().position(|k| *k == field).unwrap_or(0);
                    let rows = std::mem::take(&mut now.rows);
                    now.detail = Some(d.work.detail(col, &rows).clone());
                    now.rows = rows;
                    now.detail_for = Some(want);
                }
            }
        }
    }

    /// Whether summaries wait to be counted while the tab is on screen (the frames count them).
    pub(crate) fn stats_counting(&self) -> bool {
        if !self.command_expanded || self.bottom_tab != BottomTab::Statistics {
            return false;
        }
        let stats = self.charts.stats.borrow();
        let fresh = match (&stats.key, self.document.as_ref(), self.chart_layer()) {
            (Some((k, rows)), Some(doc), Some(layer)) => {
                *k == crate::charts::now::layer_key(doc, &layer) && *rows == self.stats_rows_key()
            }
            (None, _, None) | (None, None, _) => true,
            _ => false,
        };
        !fresh || stats.summaries.iter().any(Option::is_none)
    }

    /// Summaries counted: as many as `budget` allows, one at least; all without one.
    fn count_within(&self, budget: Option<Duration>) {
        self.stats_refresh();
        let start = Instant::now();
        let mut now = self.charts.stats.borrow_mut();
        let mut held = self.charts.layer_data.borrow_mut();
        let Some((_, d)) = held.as_mut() else {
            return;
        };
        let rows = std::mem::take(&mut now.rows);
        while let Some(i) = now.summaries.iter().position(Option::is_none) {
            now.summaries[i] = Some(d.work.summary(i, &rows));
            if budget.is_some_and(|b| start.elapsed() > b) {
                break;
            }
        }
        now.rows = rows;
    }

    /// A frame's summaries: as many as 8 ms allow, one at least.
    fn count_summaries(&mut self) {
        self.count_within(Some(FRAME_BUDGET));
    }

    /// What İstatistik shows while it is open, every row counted (the traces read it):
    /// its count and its summary rows (the cells joined by “ | ”).
    pub(crate) fn statistics_seen(&self) -> Option<(String, Vec<String>)> {
        if !self.command_expanded || self.bottom_tab != BottomTab::Statistics {
            return None;
        }
        self.count_within(None);
        let now = self.charts.stats.borrow();
        Some((
            texts::objects(now.rows.len()),
            now.summaries
                .iter()
                .map(|s| {
                    s.as_ref()
                        .map(|s| summary_cells(s).join(" | "))
                        .unwrap_or_default()
                })
                .collect(),
        ))
    }

    pub(crate) fn statistics_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Layer(id) => {
                let task = self.charts_event(crate::charts::Event::Layer(id));
                let whole = self.stats_whole();
                return Task::batch([task, whole]);
            }
            Event::Show(show) => return self.charts_event(crate::charts::Event::Show(show)),
            Event::Field(key) => {
                self.charts.field = Some(key);
                return self.stats_whole();
            }
            Event::Value(i) => {
                let rows = self
                    .charts
                    .stats
                    .borrow()
                    .detail
                    .as_ref()
                    .and_then(|d| d.values.get(i))
                    .map(|v| v.rows.clone());
                if let Some(rows) = rows {
                    self.pick_rows(&rows);
                }
            }
            Event::Chart => {
                let Some(field) = self.charts.field.clone() else {
                    return Task::none();
                };
                let spec = self
                    .charts
                    .layer_data
                    .borrow()
                    .as_ref()
                    .map(|(_, d)| for_column(&d.work.data, &field));
                let layer = self.chart_layer();
                if let Some(spec) = spec {
                    self.open_chart(layer, None, Some(spec));
                }
            }
            Event::Copy => {
                self.stats_refresh();
                let text = stats_text(&self.charts.stats.borrow());
                self.say(Level::Success, texts::COPIED);
                return iced::clipboard::write(text);
            }
            Event::Count => self.count_summaries(),
            e @ (Event::Scope(_) | Event::WholeStop | Event::WholeAnswered(..)) => {
                return self.stats_whole_event(e);
            }
        }
        Task::none()
    }
}

/// The summaries and the chosen field's statistics and values, tab-separated (the web's `copy`).
fn stats_text(now: &StatsNow) -> String {
    let mut lines = vec![texts::COLUMNS.join("\t")];
    for s in now.summaries.iter().flatten() {
        lines.push(summary_cells(s).join("\t"));
    }
    if let Some(d) = &now.detail {
        lines.push(String::new());
        lines.push(d.summary.label.clone());
        lines.extend(d.stats.iter().map(|s| format!("{}\t{}", s.label, s.value)));
        lines.push(String::new());
        lines.push(["Değer", "Nesne", "Pay (%)"].join("\t"));
        lines.extend(
            d.values
                .iter()
                .map(|v| format!("{}\t{}\t{}", v.text, v.count, v.share)),
        );
    }
    lines.join("\n")
}

/// A summary's cells in the table's order.
pub(crate) fn summary_cells(s: &Summary) -> [String; 9] {
    [
        s.label.clone(),
        s.kind.clone(),
        s.filled.to_string(),
        s.empty.to_string(),
        s.distinct.to_string(),
        s.min.clone().unwrap_or_default(),
        s.max.clone().unwrap_or_default(),
        s.mean.clone().unwrap_or_default(),
        s.sum.clone().unwrap_or_default(),
    ]
}
