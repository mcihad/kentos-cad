//! A chart's work kept between calls (docs/adr/0227 §11): the data is read
//! once, a chart is worked out when its spec or its rows change, laid out
//! when its frame does, and only its marks are found again when the
//! drawing's selection changes. The desktop keeps one in its panels; the web
//! keeps one in WASM (`ChartWork`), so nothing crosses as JSON per row.

use super::data::{DataSet, Read};
use super::fields::{Detail, Summary, detail, summary};
use super::layout::{Frame, Layout, element_middle, hit, lay_out, within};
use super::model::{ChartKind, Input, Model, Spec, build, marks, tip};
use super::svg::{Palette, svg};

/// A chart's data and what was worked out from it.
#[derive(Clone, Debug, Default)]
pub struct Work {
    pub data: DataSet,
    reads: Vec<Read>,
    loaded: Vec<bool>,
    all: Vec<u32>,
    model: Option<Model>,
    selected: Vec<bool>,
    marks: Vec<bool>,
    layout: Option<Layout>,
    frame_font: f32,
    frame_family: String,
    /// The last field's statistics: its value rows' rows (a click selects them).
    detail: Option<Detail>,
}

impl Work {
    pub fn new(data: DataSet) -> Work {
        let n = data.columns.len();
        let all = (0..data.rows as u32).collect();
        Work {
            reads: vec![Read::default(); n],
            loaded: vec![false; n],
            selected: vec![false; data.rows],
            all,
            data,
            ..Work::default()
        }
    }

    pub fn rows(&self) -> usize {
        self.data.rows
    }

    /// Reads the columns not read yet.
    fn ensure(&mut self, cols: &[usize]) {
        for &c in cols {
            if c < self.loaded.len() && !self.loaded[c] {
                self.reads[c] = Read::of(&self.data.columns[c]);
                self.loaded[c] = true;
            }
        }
    }

    /// A column read (the İstatistik tab reads them one a frame).
    pub fn read(&mut self, col: usize) -> &Read {
        self.ensure(&[col]);
        &self.reads[col]
    }

    pub fn summary(&mut self, col: usize, rows: &[u32]) -> Summary {
        self.ensure(&[col]);
        summary(&self.data.columns[col], &self.reads[col], rows)
    }

    pub fn detail(&mut self, col: usize, rows: &[u32]) -> &Detail {
        self.ensure(&[col]);
        self.detail
            .insert(detail(&self.data.columns[col], &self.reads[col], rows))
    }

    /// The rows of the last field's value row `i` (none past its rows).
    pub fn value_rows(&self, i: usize) -> &[u32] {
        self.detail
            .as_ref()
            .and_then(|d| d.values.get(i))
            .map_or(&[], |v| v.rows.as_slice())
    }

    /// Works a chart out over `rows` (Göster's), marks it by the selection.
    pub fn chart(&mut self, spec: &Spec, rows: &[u32]) -> Result<&Model, String> {
        let keys = spec
            .category
            .iter()
            .chain(&spec.value)
            .chain(&spec.group)
            .chain(&spec.x)
            .chain(&spec.y)
            .chain(&spec.series);
        let cols: Vec<usize> = keys.filter_map(|k| self.data.find(k)).collect();
        self.ensure(&cols);
        let input = Input {
            data: &self.data,
            reads: &self.reads,
            rows,
            all: &self.all,
        };
        let model = build(spec, &input);
        self.layout = None;
        match model {
            Ok(m) => {
                self.marks = marks(&m, &self.selected);
                Ok(self.model.insert(m))
            }
            Err(e) => {
                self.model = None;
                self.marks.clear();
                Err(e)
            }
        }
    }

    pub fn model(&self) -> Option<&Model> {
        self.model.as_ref()
    }

    pub fn kind(&self) -> Option<ChartKind> {
        self.model.as_ref().map(|m| m.kind)
    }

    /// The elements the selection marks, by element.
    pub fn marks(&self) -> &[bool] {
        &self.marks
    }

    /// The drawing's selection by its rows: the elements holding one are
    /// marked. Whether the marks changed (the chart is drawn again then).
    pub fn select(&mut self, rows: &[u32]) -> bool {
        self.selected.iter_mut().for_each(|s| *s = false);
        for &r in rows {
            if let Some(s) = self.selected.get_mut(r as usize) {
                *s = true;
            }
        }
        let Some(m) = &self.model else {
            return false;
        };
        let marks = marks(m, &self.selected);
        if marks == self.marks {
            return false;
        }
        self.marks = marks;
        self.layout = None;
        true
    }

    /// The chart laid out in a frame (kept until the frame, the chart or its marks change).
    pub fn lay_out(&mut self, frame: &Frame) -> Option<&Layout> {
        let model = self.model.as_ref()?;
        let fresh = self.layout.as_ref().is_some_and(|l| {
            l.width == frame.width
                && l.height == frame.height
                && self.frame_font == frame.font
                && self.frame_family == frame.family
        });
        if !fresh {
            self.layout = Some(lay_out(model, &self.marks, frame));
            self.frame_font = frame.font;
            self.frame_family = frame.family.clone();
        }
        self.layout.as_ref()
    }

    pub fn layout(&self) -> Option<&Layout> {
        self.layout.as_ref()
    }

    /// The element under a point of the last layout.
    pub fn hit(&self, x: f32, y: f32) -> Option<u32> {
        self.layout.as_ref().and_then(|l| hit(l, x, y))
    }

    /// Where element `e` is pressed on the last layout.
    pub fn element_middle(&self, e: u32) -> Option<(f32, f32)> {
        self.layout.as_ref().and_then(|l| element_middle(l, e))
    }

    /// An element's tooltip: its value first, with how many of its objects are selected.
    pub fn tip(&self, element: usize) -> Vec<(String, String)> {
        let Some(m) = &self.model else {
            return Vec::new();
        };
        let selected = m.elements.get(element).map_or(0, |e| {
            e.rows
                .iter()
                .filter(|&&r| self.selected[r as usize])
                .count()
        });
        tip(m, element, selected)
    }

    /// An element's rows.
    pub fn element_rows(&self, element: usize) -> &[u32] {
        self.model
            .as_ref()
            .and_then(|m| m.elements.get(element))
            .map_or(&[], |e| e.rows.as_slice())
    }

    /// The rows of the points within a rectangle of the last layout.
    pub fn rows_within(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<u32> {
        let (Some(l), Some(m)) = (&self.layout, &self.model) else {
            return Vec::new();
        };
        let mut rows: Vec<u32> = within(l, x0, y0, x1, y1)
            .into_iter()
            .filter_map(|e| m.elements.get(e as usize))
            .flat_map(|e| e.rows.iter().copied())
            .collect();
        rows.sort_unstable();
        rows.dedup();
        rows
    }

    /// The chart laid out in a frame of its own, unmarked (a saved picture):
    /// the cached layout stays the screen's.
    pub fn plain_layout(&self, frame: &Frame) -> Option<Layout> {
        self.model.as_ref().map(|m| lay_out(m, &[], frame))
    }

    /// The chart as an SVG document in a frame of its own (a saved
    /// picture: the selection is not marked in it).
    pub fn svg(&self, frame: &Frame, palette: &Palette) -> Option<String> {
        let model = self.model.as_ref()?;
        Some(svg(&lay_out(model, &[], frame), palette))
    }
}
