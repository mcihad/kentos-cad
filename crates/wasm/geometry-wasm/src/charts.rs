//! Grafikler ve alan istatistikleri across the boundary (docs/adr/0227 §11):
//! the İstatistik and Grafik tabs build a `ChartWork` once for a layer's (or
//! a table's) data, then ask it for statistics, charts, layouts and clicks.
//! A column crosses as one text with its cells one after another and their
//! lengths in UTF-16 units (−1: empty), as the expression engine's rows do;
//! the bulk points come back as a `Float32Array`, everything else as JSON.

use kentos_geometry_core::ops::charts::data::{Cell, Column, DataSet, Kind};
use kentos_geometry_core::ops::charts::{Work, json, specs};
use wasm_bindgen::prelude::*;

/// `text`'s pieces by UTF-16 lengths (−1: none).
fn pieces<'a>(text: &'a str, lens: &[i32]) -> Vec<Option<&'a str>> {
    let mut out = Vec::with_capacity(lens.len());
    let mut rest = text;
    for &len in lens {
        if len < 0 {
            out.push(None);
            continue;
        }
        let mut units = 0usize;
        let mut end = 0usize;
        for c in rest.chars() {
            if units >= len as usize {
                break;
            }
            units += c.len_utf16();
            end += c.len_utf8();
        }
        out.push(Some(&rest[..end]));
        rest = &rest[end..];
    }
    out
}

fn err(e: String) -> JsError {
    JsError::new(&e)
}

#[wasm_bindgen]
pub struct ChartWork {
    rows: usize,
    columns: Vec<Column>,
    work: Option<Work>,
}

#[wasm_bindgen]
impl ChartWork {
    /// A data set of `rows` rows; its columns are added, then it is sealed.
    #[wasm_bindgen(constructor)]
    pub fn new(rows: usize) -> ChartWork {
        ChartWork {
            rows,
            columns: Vec::new(),
            work: None,
        }
    }

    /// A column: its key, name, kind (`number`, `text`, `date`, `boolean`),
    /// whether it sorts by what it shows (a value list); its cells' values one
    /// after another with their lengths (−1: empty), what they show likewise
    /// (−1: the value) and the rows whose value its kind refuses.
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(js_name = addColumn)]
    pub fn add_column(
        &mut self,
        key: &str,
        label: &str,
        kind: &str,
        labelled: bool,
        values: &str,
        lens: &[i32],
        shown: &str,
        shown_lens: &[i32],
        refused: &[u32],
    ) -> Result<(), JsError> {
        let kind =
            Kind::from_word(kind).ok_or_else(|| err(format!("bilinmeyen sütun türü: {kind}")))?;
        if lens.len() != self.rows || shown_lens.len() != self.rows {
            return Err(err("sütunun hücre sayısı satır sayısı değil".to_owned()));
        }
        let values = pieces(values, lens);
        let shown = pieces(shown, shown_lens);
        let mut cells: Vec<Cell> = values
            .into_iter()
            .zip(shown)
            .map(|(v, s)| match v {
                None => Cell::Empty,
                Some(v) => Cell::Value(v.to_owned(), s.map(str::to_owned)),
            })
            .collect();
        for &r in refused {
            if let Some(c) = cells.get_mut(r as usize)
                && let Cell::Value(v, _) = c
            {
                *c = Cell::Invalid(std::mem::take(v));
            }
        }
        self.columns.push(Column {
            key: key.to_owned(),
            label: label.to_owned(),
            kind,
            labelled,
            cells,
        });
        Ok(())
    }

    /// A geometry measure's column: one double a row, NaN for none.
    #[wasm_bindgen(js_name = addMeasures)]
    pub fn add_measures(&mut self, key: &str, label: &str, values: &[f64]) -> Result<(), JsError> {
        if values.len() != self.rows {
            return Err(err("ölçülerin sayısı satır sayısı değil".to_owned()));
        }
        self.columns.push(Column::of_measures(key, label, values));
        Ok(())
    }

    /// Alan and Uzunluk from the geometry store's measure records
    /// (`GeometryStore.measures`, one a row), each where an object has one,
    /// in the unit `per_metre` of which make a metre (docs/adr/0165 §2): the
    /// keys of the columns added.
    #[wasm_bindgen(js_name = addGeometry)]
    pub fn add_geometry(
        &mut self,
        records: &[f64],
        per_metre: f64,
    ) -> Result<Vec<String>, JsError> {
        let stride = kentos_geometry_core::store::draw::MEASURE_STRIDE;
        if records.len() != self.rows * stride {
            return Err(err("ölçü kayıtlarının sayısı satır sayısı değil".to_owned()));
        }
        let added = kentos_geometry_core::ops::charts::data::geometry_columns(records, per_metre);
        let keys = added.iter().map(|c| c.key.clone()).collect();
        self.columns.extend(added);
        Ok(keys)
    }

    /// Ends the columns: the work starts.
    pub fn seal(&mut self) {
        let data = DataSet {
            rows: self.rows,
            columns: std::mem::take(&mut self.columns),
        };
        self.work = Some(Work::new(data));
    }

    fn work(&mut self) -> Result<&mut Work, JsError> {
        self.work
            .as_mut()
            .ok_or_else(|| err("grafiğin verisi kapatılmadı (seal)".to_owned()))
    }

    /// A column's summary row over these rows (JSON).
    pub fn summary(&mut self, column: usize, rows: &[u32]) -> Result<String, JsError> {
        let w = self.work()?;
        if column >= w.data.columns.len() {
            return Err(err(format!("sütun yok: {column}")));
        }
        Ok(kentos_geometry_core::api::json::to_string(
            &w.summary(column, rows),
        ))
    }

    /// A column's full statistics over these rows (JSON; the value rows' rows by `valueRows`).
    pub fn detail(&mut self, column: usize, rows: &[u32]) -> Result<String, JsError> {
        let w = self.work()?;
        if column >= w.data.columns.len() {
            return Err(err(format!("sütun yok: {column}")));
        }
        Ok(json::detail_json(w.detail(column, rows)))
    }

    /// The rows of the last statistics' value row `i`.
    #[wasm_bindgen(js_name = valueRows)]
    pub fn value_rows(&self, i: usize) -> Vec<u32> {
        self.work
            .as_ref()
            .map(|w| w.value_rows(i).to_vec())
            .unwrap_or_default()
    }

    /// A spec (JSON, its missing choices the kind's own) made whole for this
    /// data (JSON; `specs::complete`).
    pub fn complete(&mut self, spec: &str) -> Result<String, JsError> {
        let spec = json::spec_from_json(spec).map_err(err)?;
        let w = self.work()?;
        Ok(json::spec_json(&specs::complete(&spec, &w.data)))
    }

    /// İstatistik's Grafik for a column (JSON; `specs::for_column`).
    #[wasm_bindgen(js_name = forColumn)]
    pub fn for_column(&mut self, key: &str) -> Result<String, JsError> {
        let w = self.work()?;
        Ok(json::spec_json(&specs::for_column(&w.data, key)))
    }

    /// An İşlemler table's chart, its tool's suggestion first (JSON; `specs::for_table`).
    #[wasm_bindgen(js_name = forTable)]
    pub fn for_table(&mut self, tool: &str) -> Result<String, JsError> {
        let w = self.work()?;
        Ok(json::spec_json(&specs::for_table(tool, &w.data)))
    }

    /// A chart (its spec as JSON) over these rows: its model's words (JSON).
    pub fn chart(&mut self, spec: &str, rows: &[u32]) -> Result<String, JsError> {
        let spec = json::spec_from_json(spec).map_err(err)?;
        let w = self.work()?;
        w.chart(&spec, rows).map(json::model_json).map_err(err)
    }

    /// The drawing's selected rows: whether the chart's marks changed.
    pub fn select(&mut self, rows: &[u32]) -> Result<bool, JsError> {
        Ok(self.work()?.select(rows))
    }

    /// Where element `e` is pressed on the last layout (x, y), empty when it is not on the chart.
    #[wasm_bindgen(js_name = elementAt)]
    pub fn element_at(&self, e: u32) -> Vec<f32> {
        self.work
            .as_ref()
            .and_then(|w| w.element_middle(e))
            .map_or_else(Vec::new, |(x, y)| vec![x, y])
    }

    /// The elements the selection marks, by their places.
    pub fn marked(&self) -> Vec<u32> {
        self.work.as_ref().map_or_else(Vec::new, |w| {
            w.marks()
                .iter()
                .enumerate()
                .filter(|(_, m)| **m)
                .map(|(i, _)| i as u32)
                .collect()
        })
    }

    /// The chart laid out in a frame (JSON `{width, height, font, family}`):
    /// its primitives (JSON); its bulk points by `points`.
    #[wasm_bindgen(js_name = layOut)]
    pub fn lay_out(&mut self, frame: &str) -> Result<String, JsError> {
        let frame = json::frame_from_json(frame).map_err(err)?;
        let w = self.work()?;
        Ok(w.lay_out(&frame).map(json::layout_json).unwrap_or_default())
    }

    /// The last layout's bulk points (x, y pairs).
    pub fn points(&self) -> Vec<f32> {
        self.work
            .as_ref()
            .and_then(Work::layout)
            .map(|l| l.points.clone())
            .unwrap_or_default()
    }

    /// The last layout's bulk points' elements, in the points' order.
    #[wasm_bindgen(js_name = pointElements)]
    pub fn point_elements(&self) -> Vec<u32> {
        self.work
            .as_ref()
            .and_then(Work::layout)
            .map(|l| l.point_elems.clone())
            .unwrap_or_default()
    }

    /// The element under a point of the last layout, −1 for none.
    pub fn hit(&self, x: f32, y: f32) -> i32 {
        self.work
            .as_ref()
            .and_then(|w| w.hit(x, y))
            .map_or(-1, |e| e as i32)
    }

    /// An element's tooltip (JSON `[[label, value], …]`).
    pub fn tip(&self, element: usize) -> String {
        self.work
            .as_ref()
            .map(|w| json::tip_json(&w.tip(element)))
            .unwrap_or_else(|| "[]".to_owned())
    }

    /// An element's rows.
    #[wasm_bindgen(js_name = elementRows)]
    pub fn element_rows(&self, element: usize) -> Vec<u32> {
        self.work
            .as_ref()
            .map(|w| w.element_rows(element).to_vec())
            .unwrap_or_default()
    }

    /// The rows of the points within a rectangle of the last layout.
    #[wasm_bindgen(js_name = rowsWithin)]
    pub fn rows_within(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<u32> {
        self.work
            .as_ref()
            .map(|w| w.rows_within(x0, y0, x1, y1))
            .unwrap_or_default()
    }

    /// The chart laid out unmarked in a frame of its own (a saved picture),
    /// its bulk points inline (JSON).
    #[wasm_bindgen(js_name = layOutPlain)]
    pub fn lay_out_plain(&self, frame: &str) -> Result<String, JsError> {
        let frame = json::frame_from_json(frame).map_err(err)?;
        self.work
            .as_ref()
            .and_then(|w| w.plain_layout(&frame))
            .map(|l| json::layout_with_points_json(&l))
            .ok_or_else(|| err("grafik yok".to_owned()))
    }

    /// The chart as an SVG document in a frame and a palette (both JSON).
    pub fn svg(&self, frame: &str, palette: &str) -> Result<String, JsError> {
        let frame = json::frame_from_json(frame).map_err(err)?;
        let palette = json::palette_from_json(palette).map_err(err)?;
        self.work
            .as_ref()
            .and_then(|w| w.svg(&frame, &palette))
            .ok_or_else(|| err("grafik yok".to_owned()))
    }
}
