//! A data layer's chart drawn from its field as its source counted it (docs/adr/9181): a chart of every row that reads
//! one field (Çubuk and Pasta by count, Histogram, Kutu without groups) takes the field's count (data_counts.rs) and
//! the core draws it as if each row had been read. Its settings list the layer's fields without reading a row; its
//! marks are the drawing's selected rows of the layer. A chart reading two fields reads the rows (now.rs).

use kentos_geometry_core::ops::charts::Work;
use kentos_geometry_core::ops::charts::data::DataSet;
use kentos_geometry_core::ops::charts::model::{Aggregate, ChartKind, Spec};

use super::{ChartNow, ColumnInfo, DataKey, layer_key, worked_out};
use crate::app::App;
use crate::charts::CountedChart;
use crate::data_counts::{Counts, FieldNow, shape_columns};
use crate::features::Show;

/// While a data layer's source counts the field a chart reads.
const COUNTING: &str = "Kaynak sayıyor…";

/// The one field a chart reads, when it reads one (docs/adr/9181): Çubuk and Pasta by count their category, Histogram
/// its value, Kutu without groups its value. A chart reading two (a value by category, a box by group, Saçılım,
/// Çizgi) has none.
fn one_field(spec: &Spec) -> Option<String> {
    let field = match spec.kind {
        ChartKind::Bar if matches!(spec.aggregate, Aggregate::Count | Aggregate::None) => {
            spec.category.clone()
        }
        ChartKind::Pie if matches!(spec.aggregate, Aggregate::Count | Aggregate::None) => {
            spec.category.clone()
        }
        ChartKind::Histogram => spec.value.clone(),
        ChartKind::Box if spec.group.is_none() => spec.value.clone(),
        _ => None,
    };
    field.filter(|f| !f.starts_with('$'))
}

impl App {
    /// The chart's layer's fields as columns without rows, when it is a data layer (docs/adr/9181).
    pub(super) fn data_layer_shape(&self) -> Option<DataSet> {
        let doc = self.document.as_ref()?;
        let layer = self.chart_layer()?;
        doc.model.layers().get(&layer)?.data.as_ref()?;
        let columns = shape_columns(self, doc, &layer)
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        Some(DataSet { rows: 0, columns })
    }

    /// A data layer's chart over every row that reads one field, drawn from the field as its source counted it
    /// (docs/adr/9181); its marks are the drawing's selected rows of the layer. False when the chart is drawn
    /// otherwise (the rows read, a table, a drawing layer).
    pub(super) fn chart_refresh_counted(&self) -> bool {
        let p = &self.gis.charts;
        if self.charting_table() || p.show != Show::All {
            return false;
        }
        let (Some(doc), Some(layer), Some(spec)) = (
            self.document.as_ref(),
            self.chart_layer(),
            self.whole_spec(),
        ) else {
            return false;
        };
        if doc
            .model
            .layers()
            .get(&layer)
            .is_none_or(|n| n.data.is_none())
        {
            return false;
        }
        let Some(field) = one_field(&spec) else {
            return false;
        };
        let shape = shape_columns(self, doc, &layer);
        let Some((column, f)) = shape.iter().find(|(c, _)| c.key == field) else {
            return false;
        };
        let counts = match self.data_counts_of(&layer, &field) {
            None | Some(Counts::Rows) => return false,
            Some(c) => c,
        };
        let seq = match &counts {
            Counts::Ready(_, n) => Some(*n),
            _ => None,
        };
        let key = (
            DataKey::Counted(
                layer_key(doc, &layer),
                doc.model.data_written(),
                field.clone(),
                seq,
            ),
            (Show::All, 0, [0; 4]),
            spec.clone(),
        );
        let mut now = p.chart.borrow_mut();
        if now.key.as_ref() != Some(&key) {
            let columns: Vec<ColumnInfo> = shape
                .iter()
                .map(|(c, _)| ColumnInfo {
                    key: c.key.clone(),
                    label: c.label.clone(),
                    kind: c.kind,
                    labelled: c.labelled,
                })
                .collect();
            let stamp = now.stamp + 1;
            let waiting = |error: String| ChartNow {
                key: Some(key.clone()),
                spec: Some(spec.clone()),
                error: Some(error),
                columns: columns.clone(),
                stamp,
                counted: true,
                ..ChartNow::default()
            };
            *now = match self.field_now(&layer, column, f) {
                FieldNow::Failed(why) => waiting(format!("Kaynak sayamadı: {why}")),
                FieldNow::Ready(g, _) => {
                    let data = DataSet {
                        rows: g.column.cells.len(),
                        columns: vec![g.column],
                    };
                    let mut work = Work::counted(data, g.rows.clone());
                    let worked = worked_out(&mut work, &spec, &g.rows);
                    *p.counted.borrow_mut() = Some(CountedChart {
                        layer: layer.clone(),
                        field: field.clone(),
                        picker: g.picker,
                        work,
                    });
                    ChartNow {
                        key: Some(key.clone()),
                        spec: Some(spec.clone()),
                        stamp,
                        marked: None,
                        counted: true,
                        columns: columns.clone(),
                        ..worked
                    }
                }
                FieldNow::Counting | FieldNow::Rows => waiting(COUNTING.to_owned()),
            };
        }
        // The drawing's selected rows of the layer (taken or new) mark the chart; the source's rows are no objects.
        if now.marked != Some(self.selection.version()) {
            if let Some(c) = p.counted.borrow_mut().as_mut() {
                let rows = c.picker.rows_of_slots(self.selection.ids());
                if c.work.select(&rows) {
                    now.stamp += 1;
                }
            }
            now.marked = Some(self.selection.version());
        }
        true
    }
}
