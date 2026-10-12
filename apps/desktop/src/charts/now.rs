//! What the Grafik tab shows, worked out again only when something it
//! depends on changed (docs/adr/0227 §11; the web's `ChartPanel.refresh`):
//! the data is read once for the drawing as it is (a layer's objects, or
//! the İşlemler table), the chart is worked out when its spec or its rows
//! change, and only its marks are found again when the drawing's selection
//! changes. The view and the canvases read what is kept here.

use kentos_geometry_core::ops::charts::Work;
use kentos_geometry_core::ops::charts::data::{DataSet, Kind};
use kentos_geometry_core::ops::charts::model::{ChartKind, Spec, Table};
use kentos_geometry_core::ops::charts::specs::{complete, for_table};
use kentos_interaction::chart_data::table_columns;

use super::data::LayerData;
use super::{Source, View};
use crate::app::App;
use crate::document::Document;
use crate::features::Show;

/// A layer's data as read: the drawing's session and generation, and the layer.
pub(crate) type LayerKey = (u64, u64, String);

/// What the data is.
#[derive(Clone, Debug, PartialEq)]
enum DataKey {
    Layer(LayerKey),
    Table(u64),
}

/// Göster's rows as they depend on the selection (Seçililer) and the view (Görünümdekiler).
type RowsKey = (Show, u64, [u64; 4]);

/// A column as the settings list it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ColumnInfo {
    pub key: String,
    pub label: String,
    pub kind: Kind,
    pub labelled: bool,
}

/// The chart as last worked out.
#[derive(Default)]
pub(crate) struct ChartNow {
    key: Option<(DataKey, RowsKey, Spec)>,
    /// The spec as made whole for the columns.
    pub spec: Option<Spec>,
    pub kind: Option<ChartKind>,
    /// Why the chart cannot be drawn.
    pub error: Option<String>,
    pub notes: Vec<String>,
    /// The chart's numbers (Tablo, Panoya kopyala).
    pub table: Option<Table>,
    /// The value axis's title (a saved picture's name).
    pub y_title: String,
    pub columns: Vec<ColumnInfo>,
    /// The selection's version as last marked.
    marked: Option<u64>,
    /// Bumped whenever the picture changes: the chart or its marks.
    pub stamp: u64,
}

impl App {
    /// The layer's data as the drawing is now, read again when it changed.
    pub(crate) fn layer_data_now(&self, doc: &Document, layer: &str) -> LayerKey {
        let key = (doc.session, doc.model.generation(), layer.to_owned());
        let mut held = self.charts.layer_data.borrow_mut();
        if held.as_ref().is_none_or(|(k, _)| *k != key) {
            *held = Some((key.clone(), LayerData::of(self, doc, layer)));
        }
        key
    }

    /// The İşlemler table's data, read once a table.
    fn table_data_now(&self) -> Option<u64> {
        let table = self.charts.table.as_ref()?;
        let version = self.charts.table_version;
        let mut held = self.charts.table_data.borrow_mut();
        if held.as_ref().is_none_or(|(v, _)| *v != version) {
            let columns = table_columns(&table.columns, &table.rows);
            let work = Work::new(DataSet {
                rows: table.rows.len(),
                columns,
            });
            *held = Some((version, work));
        }
        Some(version)
    }

    /// The data of the source shown, read once for the drawing as it is.
    fn chart_data_now(&self) -> Option<DataKey> {
        let p = &self.charts;
        if p.source == Source::Table && p.table.is_some() {
            return self.table_data_now().map(DataKey::Table);
        }
        let doc = self.document.as_ref()?;
        let layer = self.chart_layer()?;
        Some(DataKey::Layer(self.layer_data_now(doc, &layer)))
    }

    /// Whether the tab reads the table now (else a layer).
    pub(crate) fn charting_table(&self) -> bool {
        self.charts.source == Source::Table && self.charts.table.is_some()
    }

    /// Something read from the data of the source shown.
    pub(crate) fn with_chart_data<R>(&self, read: impl FnOnce(&DataSet) -> R) -> Option<R> {
        match self.chart_data_now()? {
            DataKey::Table(_) => self
                .charts
                .table_data
                .borrow()
                .as_ref()
                .map(|(_, w)| read(&w.data)),
            DataKey::Layer(_) => self
                .charts
                .layer_data
                .borrow()
                .as_ref()
                .map(|(_, d)| read(&d.work.data)),
        }
    }

    /// The spec kept for the source shown, made whole for its data (a
    /// table's first: its tool's suggestion).
    pub(crate) fn whole_spec(&self) -> Option<Spec> {
        let p = &self.charts;
        let table = self.charting_table();
        self.with_chart_data(|data| {
            let kept = if table {
                p.table_spec
                    .clone()
                    .or_else(|| p.table.as_ref().map(|t| for_table(&t.tool, data)))
            } else {
                p.layer_spec.clone()
            };
            complete(&kept.unwrap_or_else(|| Spec::new(ChartKind::Bar)), data)
        })
    }

    /// Göster's rows of the layer and what they depend on.
    fn rows_key(&self) -> RowsKey {
        let show = self.charts.show;
        let selection = if show == Show::Selected {
            self.selection.version()
        } else {
            0
        };
        let view = if show == Show::InView {
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

    /// The chart as the drawing, the selection, the view and the choices are
    /// now: worked out again when its data, its rows or its spec changed,
    /// marked again when the selection did.
    pub(crate) fn chart_refresh(&self) {
        let p = &self.charts;
        let (Some(data), Some(spec)) = (self.chart_data_now(), self.whole_spec()) else {
            let stamp = p.chart.borrow().stamp + 1;
            *p.chart.borrow_mut() = ChartNow {
                stamp,
                ..ChartNow::default()
            };
            return;
        };
        let table = matches!(data, DataKey::Table(_));
        let rows_key = if table {
            (Show::All, 0, [0; 4])
        } else {
            self.rows_key()
        };
        let key = (data, rows_key, spec.clone());
        let mut now = p.chart.borrow_mut();
        if now.key.as_ref() != Some(&key) {
            let worked = if table {
                let mut held = p.table_data.borrow_mut();
                held.as_mut().map(|(_, w)| {
                    let rows: Vec<u32> = (0..w.rows() as u32).collect();
                    worked_out(w, &spec, &rows)
                })
            } else {
                let mut held = p.layer_data.borrow_mut();
                held.as_mut().map(|(_, d)| {
                    let rows = d.rows(self, p.show);
                    worked_out(&mut d.work, &spec, &rows)
                })
            };
            let Some(worked) = worked else {
                return;
            };
            *now = ChartNow {
                key: Some(key),
                spec: Some(spec),
                stamp: now.stamp + 1,
                marked: None,
                ..worked
            };
        }
        // The drawing's selection marks the chart (a table's rows are no objects).
        if !table && now.marked != Some(self.selection.version()) {
            let mut held = p.layer_data.borrow_mut();
            if let Some((_, d)) = held.as_mut() {
                let selected = d.selected_rows(self);
                if d.work.select(&selected) {
                    now.stamp += 1;
                }
            }
            now.marked = Some(self.selection.version());
        }
    }

    /// Whether the Grafik tab shows its numbers (else the chart).
    pub(crate) fn chart_table_view(&self) -> bool {
        self.charts.view == View::Table
    }
}

/// The chart worked out on `rows`: its kind, notes and numbers, or why not.
fn worked_out(work: &mut Work, spec: &Spec, rows: &[u32]) -> ChartNow {
    let columns = work
        .data
        .columns
        .iter()
        .map(|c| ColumnInfo {
            key: c.key.clone(),
            label: c.label.clone(),
            kind: c.kind,
            labelled: c.labelled,
        })
        .collect();
    match work.chart(spec, rows) {
        Ok(m) => ChartNow {
            kind: Some(m.kind),
            notes: m.notes.clone(),
            table: Some(m.table.clone()),
            y_title: m.y_title.clone(),
            columns,
            ..ChartNow::default()
        },
        Err(e) => ChartNow {
            error: Some(e),
            columns,
            ..ChartNow::default()
        },
    }
}
