//! İstatistik over a data layer's every row counted by its source (docs/adr/9181): each field by the source's own
//! `GROUP BY` (data_counts.rs), summed up by the core as its count comes, a few fields a frame; the chosen field's
//! statistics once it is counted. The rows are not read. Göster's other choices, a drawing layer and a field the
//! source leaves to the rows read count the rows (mod.rs).

use std::time::{Duration, Instant};

use kentos_geometry_core::ops::charts::data::{Column, Read};
use kentos_geometry_core::ops::charts::fields::{detail, summary};

use super::StatsNow;
use crate::app::App;
use crate::charts::now::LayerKey;
use crate::data_counts::{Counts, FieldNow, shape_columns};
use crate::features::Show;

/// What a data layer counted by its source is counted for: the layer's data as the drawing is now and the drawing's
/// writes to the sources.
pub(super) type CountedKey = (LayerKey, u64);

/// How a data layer's counts by its source stand.
#[derive(Clone, Debug, PartialEq)]
pub enum Counted {
    /// No field counted yet.
    Counting,
    /// A field counted at least.
    Ready,
    /// The source could not count a field: why.
    Failed(String),
}

/// A data layer's fields as columns without rows, each with its field.
type Fields = Vec<(Column, kentos_commands::LayerField)>;

impl App {
    /// A data layer's every row counted by its source (docs/adr/9181): the layer, its fields as columns, and their
    /// counts as they stand now; none when the tab counts otherwise (a drawing layer, another Göster, a field the
    /// source leaves to the rows read).
    fn stats_counted_now(&self) -> Option<(String, CountedKey, Fields, Vec<Counts>)> {
        if self.gis.charts.show != Show::All {
            return None;
        }
        let doc = self.document.as_ref()?;
        let layer = self.chart_layer()?;
        doc.model.layers().get(&layer)?.data.as_ref()?;
        let shape = shape_columns(self, doc, &layer);
        let counts: Vec<Counts> = shape
            .iter()
            .map(|(_, f)| self.data_counts_of(&layer, &f.name).unwrap_or(Counts::Rows))
            .collect();
        if counts.iter().any(|c| matches!(c, Counts::Rows)) {
            return None;
        }
        let key = (
            crate::charts::now::layer_key(doc, &layer),
            doc.model.data_written(),
        );
        Some((layer, key, shape, counts))
    }

    /// İstatistik over a data layer's every row counted by its source (docs/adr/9181): the fields' rows, how the
    /// counts stand, and the chosen field's statistics once it is counted (the summaries are summed up as frames
    /// come). False when the tab counts otherwise.
    pub(super) fn stats_refresh_counted(&self) -> bool {
        let Some((layer, key, shape, counts)) = self.stats_counted_now() else {
            return false;
        };
        let p = &self.gis.charts;
        let mut now = p.stats.borrow_mut();
        if now.counted_key.as_ref() != Some(&key) {
            *now = StatsNow {
                counted_key: Some(key),
                summaries: vec![None; shape.len()],
                seqs: vec![None; shape.len()],
                keys: shape.iter().map(|(c, _)| c.key.clone()).collect(),
                labels: shape.iter().map(|(c, _)| c.label.clone()).collect(),
                counted: Some(Counted::Counting),
                ..StatsNow::default()
            };
        }
        let failed = counts.iter().find_map(|c| match c {
            Counts::Failed(why) => Some(why.clone()),
            _ => None,
        });
        now.counted = Some(match failed {
            Some(why) => Counted::Failed(why),
            None if counts.iter().any(|c| matches!(c, Counts::Ready(..))) => Counted::Ready,
            None => Counted::Counting,
        });
        // A field counted again (its count's number changed) is summed up again.
        for (i, c) in counts.iter().enumerate() {
            if let Counts::Ready(_, n) = c
                && now.seqs.get(i).copied().flatten().is_some_and(|m| m != *n)
            {
                now.summaries[i] = None;
                now.seqs[i] = None;
            }
        }
        let chosen = p
            .field
            .clone()
            .and_then(|f| now.keys.iter().position(|k| *k == f).map(|i| (f, i)));
        match chosen {
            Some((field, i)) => match &counts[i] {
                Counts::Ready(_, n) if now.grouped_for.as_ref() != Some(&(field.clone(), *n)) => {
                    if let FieldNow::Ready(g, n) = self.field_now(&layer, &shape[i].0, &shape[i].1)
                    {
                        now.detail = Some(detail(&g.column, &Read::of(&g.column), &g.rows));
                        now.grouped = Some(g.picker);
                        now.grouped_for = Some((field, n));
                    }
                }
                Counts::Ready(..) => {}
                _ => {
                    now.detail = None;
                    now.grouped = None;
                    now.grouped_for = None;
                }
            },
            None => {
                now.detail = None;
                now.grouped = None;
                now.grouped_for = None;
            }
        }
        true
    }

    /// The summaries of a data layer's counted fields (docs/adr/9181): those whose counts came, as many as `budget`
    /// allows, one at least; all without one.
    pub(super) fn count_counted(&self, budget: Option<Duration>) {
        let Some((layer, _, shape, counts)) = self.stats_counted_now() else {
            return;
        };
        let start = Instant::now();
        let mut now = self.gis.charts.stats.borrow_mut();
        for (i, ((column, field), c)) in shape.iter().zip(&counts).enumerate() {
            if now.summaries.get(i).is_none_or(Option::is_some)
                || now.keys.get(i) != Some(&column.key)
            {
                continue;
            }
            if !matches!(c, Counts::Ready(..)) {
                continue;
            }
            if let FieldNow::Ready(g, n) = self.field_now(&layer, column, field) {
                let s = summary(&g.column, &Read::of(&g.column), &g.rows);
                now.objects = s.objects;
                now.summaries[i] = Some(s);
                now.seqs[i] = Some(n);
                if budget.is_some_and(|b| start.elapsed() > b) {
                    break;
                }
            }
        }
    }

    /// Whether a data layer counted by its source waits for summaries (the frames sum them up): while a field's count
    /// has come and is not summed up. None when the tab counts otherwise.
    pub(super) fn stats_counted_waiting(&self) -> Option<bool> {
        let (_, key, _, counts) = self.stats_counted_now()?;
        let stats = self.gis.charts.stats.borrow();
        Some(
            stats.counted_key.as_ref() != Some(&key)
                || stats
                    .summaries
                    .iter()
                    .zip(&counts)
                    .any(|(s, c)| s.is_none() && matches!(c, Counts::Ready(..))),
        )
    }
}
