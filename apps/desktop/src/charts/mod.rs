//! Grafik, the bottom panel's chart tab (docs/adr/0227 §4–§8; the web's
//! `ui/bottom/ChartPanel.ts` and `app/charts.ts`), and the choices it shares
//! with İstatistik (statistics/): a layer's or an İşlemler table's chart,
//! one of six kinds. The shared core works the chart out and lays it out as
//! a drawing list (`ops::charts`); this tab paints it (paint.rs, canvas.rs),
//! shows a tooltip, selects what a click (or Saçılım's dragged box) meets and
//! marks the drawing's selection. Grafik | Tablo shows its numbers; Panoya
//! kopyala copies them; Kaydet saves it as PNG or SVG (export.rs).
//!
//! The choices are kept for the session; another drawing starts them again.

use std::cell::{Cell, RefCell};

use iced::Task;
use kentos_geometry_core::ops::charts::Work;
use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind, Sort, Spec};
use kentos_interaction::Level;

use crate::app::{App, Message};
use crate::bottom::BottomTab;
use crate::features::Show;

pub(crate) mod canvas;
pub(crate) mod data;
mod export;
pub(crate) mod now;
mod paint;
pub(crate) mod palette;
#[cfg(test)]
mod tests;
mod view;

use data::{LayerData, layer_at_hand};

/// The tab's words (the web's `CHART_TEXTS`).
pub mod texts {
    pub const SOURCE: &str = "Kaynak";
    pub const SHOW: &str = "Göster";
    pub fn table(title: &str) -> String {
        format!("İşlem tablosu: {title}")
    }
    pub const CHART_VIEW: &str = "Grafik";
    pub const TABLE_VIEW: &str = "Tablo";
    pub const COPY: &str = "Panoya kopyala";
    pub const COPY_HINT: &str =
        "Grafiğin sayılarını (Tablo görünümünü) sekmeyle ayrılmış olarak panoya kopyalar";
    pub const COPIED: &str = "Grafiğin tablosu panoya kopyalandı.";
    pub const SAVE: &str = "Kaydet";
    pub const SAVE_HINT: &str =
        "Grafiği açık temanın renkleriyle beyaz kâğıtta PNG (2×) ya da SVG olarak kaydeder";
    pub const PNG: &str = "PNG resmi (2×)";
    pub const SVG: &str = "SVG çizimi";
    pub fn saved(file: &str) -> String {
        format!("Grafik kaydedildi: {file}")
    }
    pub const CATEGORY: &str = "Kategori";
    pub const AGGREGATE: &str = "Toplama";
    pub const VALUE: &str = "Değer";
    pub const SORT: &str = "Sırala";
    pub const BINS: &str = "Aralık sayısı";
    pub const AUTO: &str = "Otomatik";
    pub const GUIDES: &str = "Ortalama ve ortanca";
    pub const GROUP: &str = "Grupla";
    pub const NONE: &str = "Yok";
    pub const X: &str = "X";
    pub const Y: &str = "Y";
    pub const SERIES: &str = "Seri";
    pub const TREND: &str = "Eğilim çizgisi";
    pub const NO_LAYERS: &str = "Çizimde katman yok.";
    pub const NO_NUMBERS: &str = "Bu kaynakta sayı alanı yok.";
    pub const GEOMETRY: &str = "geometri";
}

/// A kind as the tab, the ribbon and the commands show it (the web's `CHART_KINDS`).
pub(crate) struct KindInfo {
    pub kind: ChartKind,
    pub command: &'static str,
    pub icon: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

/// The six kinds in the tab's order.
pub(crate) const KINDS: [KindInfo; 6] = [
    KindInfo {
        kind: ChartKind::Bar,
        command: "chart.bar",
        icon: "chartBar",
        title: "Çubuk grafik",
        description: "Bir alanın değerlerine göre nesne sayısı ya da başka bir alanın toplamı, ortalaması, en azı, en çoğu, ortancası; en çok 30 çubuk, gerisi “Diğer”.",
    },
    KindInfo {
        kind: ChartKind::Pie,
        command: "chart.pie",
        icon: "chartPie",
        title: "Pasta grafik",
        description: "Bir alanın değerlerinin payları (nesne sayısı ya da bir alanın toplamı) halka dilimleriyle; en çok 8 renk, gerisi “Diğer”.",
    },
    KindInfo {
        kind: ChartKind::Histogram,
        command: "chart.histogram",
        icon: "chartHistogram",
        title: "Histogram",
        description: "Bir sayı alanının dağılımı eşit aralıklarla; aralık sayısı kendiliğinden (Sturges) ya da seçilerek; ortalama ve ortanca çizgileri.",
    },
    KindInfo {
        kind: ChartKind::Box,
        command: "chart.box",
        icon: "chartBox",
        title: "Kutu grafiği",
        description: "Bir sayı alanının çeyrekleri, ortancası, bıyıkları ve aykırı değerleri; isteğe bağlı olarak bir alanın gruplarına göre.",
    },
    KindInfo {
        kind: ChartKind::Scatter,
        command: "chart.scatter",
        icon: "chartScatter",
        title: "Saçılım grafiği",
        description: "İki sayı alanının ilişkisi: her nesne bir nokta; en küçük kareler eğilim çizgisi ve R².",
    },
    KindInfo {
        kind: ChartKind::Line,
        command: "chart.line",
        icon: "chartLine",
        title: "Çizgi grafik",
        description: "Bir sayı alanına göre bir ya da birkaç değer (seriler); her satır bir nokta ya da her X için toplama. ROC, Kesit ve Zaman serisi tabloları için.",
    },
];

/// A kind's entry in [`KINDS`].
pub(crate) fn kind_info(kind: ChartKind) -> &'static KindInfo {
    KINDS.iter().find(|k| k.kind == kind).unwrap_or(&KINDS[0])
}

/// Where the Grafik tab reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Source {
    #[default]
    Layer,
    Table,
}

/// Grafik | Tablo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Chart,
    Table,
}

impl std::fmt::Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            View::Chart => texts::CHART_VIEW,
            View::Table => texts::TABLE_VIEW,
        })
    }
}

/// A İşlemler table charted: its tool's name and id (the chart its tool
/// suggests, `specs::for_table`), its columns and rows (the web's `ChartTable`).
#[derive(Debug, Clone, PartialEq)]
pub struct ChartTable {
    pub title: String,
    pub tool: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// What the Grafik tab shows, as a trace reads it (docs/adr/0227): the
/// kind, the numbers' headers and rows (the cells joined by “ | ”) and the
/// elements the selection marks (1 first).
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeen {
    pub kind: String,
    pub columns: Vec<String>,
    pub rows: Vec<String>,
    pub marked: Vec<u32>,
}

/// A saved picture's format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picture {
    Png,
    Svg,
}

/// A setting of the left column, as chosen.
#[derive(Debug, Clone, PartialEq)]
pub enum Setting {
    Category(String),
    Aggregate(Aggregate),
    Value(String),
    Sort(Sort),
    /// Aralık sayısı; none: Otomatik.
    Bins(Option<u32>),
    Guides(bool),
    Group(Option<String>),
    X(String),
    /// Saçılım's Y: the one column.
    Y(String),
    /// Çizgi's Y: a column taken or given back (at most eight).
    YToggle(String),
    Series(Option<String>),
    Trend(bool),
}

/// The tab's messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Kaynak ▾: a layer by id.
    Layer(String),
    /// Kaynak ▾: the İşlemler table.
    TableSource,
    Show(Show),
    Kind(ChartKind),
    View(View),
    Set(Setting),
    Copy,
    Save(Picture),
    Saved(Result<String, String>),
    /// A press on the chart: the element under it (none: the background).
    Press(Option<u32>),
    /// Saçılım's dragged box, its corners in the chart's pixels.
    Within([f32; 4]),
    /// İstatistik for a layer (Katmanlar's menu names one), else as it was.
    OpenStatistics(Option<String>),
    /// Grafik for a layer, with a kind (its columns chosen by the tab) or a spec.
    Open {
        layer: Option<String>,
        kind: Option<ChartKind>,
        spec: Option<Box<Spec>>,
    },
}

/// The panel's height the tabs ask for: a chart needs room (the web's `CHART_PANEL_HEIGHT`).
const PANEL_HEIGHT: f32 = 300.0;

/// The Histogram's interval counts offered (0: Otomatik).
pub(crate) const BINS: [u32; 8] = [0, 5, 10, 15, 20, 30, 50, 100];

/// The tabs' choices and what was worked out from them, kept for as long as
/// the drawing is open (the web's `charts`).
#[derive(Default)]
pub(crate) struct ChartsPanel {
    /// The layer charted and counted; none: the one at hand (pinned once a tab shows it).
    pub layer: Option<String>,
    pub show: Show,
    pub source: Source,
    pub table: Option<ChartTable>,
    /// The last spec, by source: a layer's and the table's.
    pub layer_spec: Option<Spec>,
    pub table_spec: Option<Spec>,
    pub view: View,
    /// İstatistik's chosen field.
    pub field: Option<String>,
    /// Bumped with every table charted.
    table_version: u64,
    /// The chart's room in the window as last drawn.
    room: Cell<Option<iced::Rectangle>>,
    /// A layer's data and the work on it (both tabs read the same layer).
    pub(crate) layer_data: RefCell<Option<(now::LayerKey, LayerData)>>,
    /// The İşlemler table's data and the work on it.
    pub(crate) table_data: RefCell<Option<(u64, Work)>>,
    /// The chart as last worked out.
    pub(crate) chart: RefCell<now::ChartNow>,
    /// İstatistik's summaries and the chosen field's statistics.
    pub(crate) stats: RefCell<crate::statistics::StatsNow>,
    /// İstatistik's Kaynağın tümü (a bound layer's whole source).
    pub(crate) whole: crate::statistics::Whole,
    /// The chart's picture as last painted.
    pub(crate) painted: canvas::Painted,
}

/// A spec with another kind: its columns stay, its aggregate starts at the
/// kind's own (a line's points each a row).
fn with_kind(spec: Spec, kind: ChartKind) -> Spec {
    if spec.kind == kind {
        return spec;
    }
    Spec {
        kind,
        aggregate: Spec::new(kind).aggregate,
        ..spec
    }
}

/// A setting applied to a spec (Çizgi's Y keeps at most eight).
fn set(spec: &mut Spec, setting: Setting) {
    match setting {
        Setting::Category(k) => spec.category = Some(k),
        Setting::Aggregate(a) => spec.aggregate = a,
        Setting::Value(k) => spec.value = Some(k),
        Setting::Sort(s) => spec.sort = s,
        Setting::Bins(b) => spec.bins = b,
        Setting::Guides(on) => spec.guides = on,
        Setting::Group(k) => spec.group = k,
        Setting::X(k) => spec.x = Some(k),
        Setting::Y(k) => spec.y = vec![k],
        Setting::YToggle(k) => {
            if let Some(at) = spec.y.iter().position(|y| *y == k) {
                spec.y.remove(at);
            } else if spec.y.len() < 8 {
                spec.y.push(k);
            }
        }
        Setting::Series(k) => spec.series = k,
        Setting::Trend(on) => spec.trend = on,
    }
}

impl App {
    /// The bottom panel on a tab of the two, as tall as a chart needs at least.
    pub(crate) fn open_chart_tab(&mut self, tab: BottomTab) {
        self.show_bottom(tab);
        let bar = kentos_ui::widget::tabs::height();
        if self.bottom_log() + bar < PANEL_HEIGHT {
            self.bottom_dragged(Some(PANEL_HEIGHT - bar), std::time::Instant::now());
        }
        self.follow_charts();
    }

    /// İstatistik for a layer (Katmanlar's menu names one), else as it was.
    pub(crate) fn open_statistics(&mut self, layer: Option<String>) {
        if let Some(layer) = layer {
            if self.charts.layer.as_ref() != Some(&layer) {
                self.charts.field = None;
            }
            self.charts.layer = Some(layer);
        }
        self.open_chart_tab(BottomTab::Statistics);
    }

    /// Grafik for a layer, with a kind (its columns chosen by the tab) or a spec.
    pub(crate) fn open_chart(
        &mut self,
        layer: Option<String>,
        kind: Option<ChartKind>,
        spec: Option<Spec>,
    ) {
        let at_hand = self.chart_layer();
        let p = &mut self.charts;
        // Another layer has other columns: the kind stays, its columns are chosen again.
        if let Some(l) = &layer
            && at_hand.as_ref() != Some(l)
            && let Some(s) = &p.layer_spec
        {
            p.layer_spec = Some(Spec::new(s.kind));
        }
        if layer.is_some() {
            p.layer = layer;
        }
        p.source = Source::Layer;
        if let Some(spec) = spec {
            p.layer_spec = Some(spec);
        } else if let Some(kind) = kind {
            let kept = p.layer_spec.take().unwrap_or_else(|| Spec::new(kind));
            p.layer_spec = Some(with_kind(kept, kind));
        }
        p.view = View::Chart;
        self.open_chart_tab(BottomTab::Chart);
    }

    /// A İşlemler table as a chart: its tool's suggestion, else bars.
    pub(crate) fn open_table_chart(&mut self, table: ChartTable) {
        let p = &mut self.charts;
        p.table = Some(table);
        p.table_version += 1;
        p.source = Source::Table;
        p.table_spec = None;
        p.view = View::Chart;
        self.open_chart_tab(BottomTab::Chart);
    }

    /// What the Grafik tab shows while it is open (the traces read it).
    pub(crate) fn chart_seen(&self) -> Option<ChartSeen> {
        if !self.command_expanded || self.bottom_tab != BottomTab::Chart {
            return None;
        }
        self.chart_refresh();
        let now = self.charts.chart.borrow();
        let (kind, table) = (now.kind?, now.table.as_ref()?);
        let marked = self
            .charts_work(|w| {
                w.marks()
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| **m)
                    .map(|(i, _)| i as u32 + 1)
                    .collect()
            })
            .unwrap_or_default();
        Some(ChartSeen {
            kind: kind.word().to_owned(),
            columns: table.columns.clone(),
            rows: table.rows.iter().map(|r| r.join(" | ")).collect(),
            marked,
        })
    }

    /// The layer the tabs read: the one kept while it is a layer, else the one at hand.
    pub(crate) fn chart_layer(&self) -> Option<String> {
        let doc = self.document.as_ref()?;
        layer_at_hand(self, doc, self.charts.layer.as_deref())
    }

    /// After every message: while a tab of the two is on screen, the layer it
    /// reads is kept (a selection on another layer leaves it), and a layer
    /// gone gives its place to the one at hand (the web's `charts.layer = layer`).
    pub(crate) fn follow_charts(&mut self) {
        let shown = self.command_expanded
            && matches!(self.bottom_tab, BottomTab::Statistics | BottomTab::Chart);
        if !shown {
            return;
        }
        let at_hand = self.chart_layer();
        if at_hand != self.charts.layer {
            self.charts.layer = at_hand;
        }
    }

    fn put_spec(&mut self, spec: Spec) {
        match self.charts.source {
            Source::Layer => self.charts.layer_spec = Some(spec),
            Source::Table => self.charts.table_spec = Some(spec),
        }
    }

    pub(crate) fn charts_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Layer(id) => {
                // Another layer has other columns: the kind stays, its columns are chosen again.
                if self.chart_layer().as_ref() != Some(&id)
                    && let Some(s) = &self.charts.layer_spec
                {
                    self.charts.layer_spec = Some(Spec::new(s.kind));
                }
                if self.charts.layer.as_ref() != Some(&id) {
                    self.charts.field = None;
                }
                // The table's rows are of the table's layer: another layer takes all of its own.
                if self.charts.show == Show::Table
                    && self
                        .document
                        .as_ref()
                        .and_then(|d| self.feature_layer(d))
                        .as_ref()
                        != Some(&id)
                {
                    self.charts.show = Show::All;
                }
                self.charts.layer = Some(id);
                self.charts.source = Source::Layer;
            }
            Event::TableSource => {
                if self.charts.table.is_some() {
                    self.charts.source = Source::Table;
                }
            }
            Event::Show(show) => {
                // Tablonun süzgeciyle reads the table's layer.
                if show == Show::Table
                    && let Some(layer) = self
                        .document
                        .as_ref()
                        .and_then(|d| self.feature_layer(d))
                {
                    if self.charts.layer.as_ref() != Some(&layer) {
                        self.charts.field = None;
                        self.charts.layer_spec = self
                            .charts
                            .layer_spec
                            .as_ref()
                            .map(|s| Spec::new(s.kind));
                    }
                    self.charts.layer = Some(layer);
                    self.charts.source = Source::Layer;
                }
                self.charts.show = show;
            }
            Event::Kind(kind) => {
                if let Some(spec) = self.whole_spec()
                    && spec.kind != kind
                {
                    self.put_spec(with_kind(spec, kind));
                }
            }
            Event::View(view) => {
                self.charts.view = view;
            }
            Event::Set(setting) => {
                if let Some(mut spec) = self.whole_spec() {
                    set(&mut spec, setting);
                    self.put_spec(spec);
                }
            }
            Event::Copy => {
                let table = self.charts.chart.borrow().table.clone();
                let Some(t) = table else {
                    return Task::none();
                };
                let mut lines = vec![t.columns.clone()];
                lines.extend(t.rows.iter().cloned());
                self.say(Level::Success, texts::COPIED);
                return iced::clipboard::write(crate::layer_list::tsv(&lines));
            }
            Event::Save(picture) => return self.save_chart(picture),
            Event::Saved(Ok(file)) => self.say(Level::Success, texts::saved(&file)),
            Event::Saved(Err(why)) => self.error(why),
            Event::Press(element) => self.chart_press(element),
            Event::Within(r) => {
                let rows = self.charts_work(|w| w.rows_within(r[0], r[1], r[2], r[3]));
                if let Some(rows) = rows {
                    self.pick_rows(&rows);
                }
            }
            Event::OpenStatistics(layer) => self.open_statistics(layer),
            Event::Open { layer, kind, spec } => self.open_chart(layer, kind, spec.map(|s| *s)),
        }
        Task::none()
    }

    /// A press on the chart: the element's objects selected (Shift adds, Ctrl
    /// takes away); the background clears the selection. A table's rows are
    /// no objects: a press there selects nothing.
    fn chart_press(&mut self, element: Option<u32>) {
        if self.charts.source != Source::Layer {
            return;
        }
        match element {
            Some(e) => {
                let rows = self.charts_work(|w| w.element_rows(e as usize).to_vec());
                if let Some(rows) = rows {
                    self.pick_rows(&rows);
                }
            }
            None => {
                let m = self.modifiers;
                if !m.shift() && !m.control() && !m.logo() {
                    self.selection.clear();
                }
            }
        }
    }

    /// The layer's rows picked as the drawing's selection.
    pub(crate) fn pick_rows(&mut self, rows: &[u32]) {
        let slots = self
            .charts
            .layer_data
            .borrow()
            .as_ref()
            .map(|(_, d)| d.slots_of(rows));
        if let Some(slots) = slots {
            self.apply_pick(slots);
        }
    }

    /// Something read from the chart's work as it is now (the layer's or the table's).
    pub(crate) fn charts_work<R>(&self, read: impl FnOnce(&Work) -> R) -> Option<R> {
        match self.charts.source {
            Source::Layer => self
                .charts
                .layer_data
                .borrow()
                .as_ref()
                .map(|(_, d)| read(&d.work)),
            Source::Table => self
                .charts
                .table_data
                .borrow()
                .as_ref()
                .map(|(_, w)| read(w)),
        }
    }
}
