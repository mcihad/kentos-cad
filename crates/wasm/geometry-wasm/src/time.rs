//! Temporal layers across the boundary (docs/adr/0210 §4–§6): a layer's
//! objects' times from their texts in one call, and the store's times and
//! the time slider's window; and the layers' filters' marks (docs/adr/0211 §3). The small calls (reading, writing, steps and
//! positions) go through the core's call table (`time::OPS`).

use kentos_geometry_core::time::{self, Mode, Rule, Time, Window};
use wasm_bindgen::prelude::*;

use crate::store::GeometryStore;

/// `texts` cut by their lengths in UTF-16 units; −1 is a text the object lacks.
fn split<'a>(texts: &'a str, lens: &[i32]) -> Result<Vec<Option<&'a str>>, JsError> {
    let short = || JsError::new("Zaman değerlerinin yazıları uzunluklarından kısa.");
    let bytes = texts.as_bytes();
    let mut out = Vec::with_capacity(lens.len());
    let mut at = 0;
    for &len in lens {
        if len < 0 {
            out.push(None);
            continue;
        }
        let len = len as usize;
        // ASCII text (every date) is as long in bytes as in UTF-16 units.
        let end = match bytes.get(at..at + len) {
            Some(b) if b.is_ascii() => at + len,
            _ => {
                let (mut units, mut end) = (0, at);
                for c in texts.get(at..).ok_or_else(short)?.chars() {
                    if units >= len {
                        break;
                    }
                    units += c.len_utf16();
                    end += c.len_utf8();
                }
                if units < len {
                    return Err(short());
                }
                end
            }
        };
        out.push(Some(texts.get(at..end).ok_or_else(short)?));
        at = end;
    }
    Ok(out)
}

/// The times of a temporal layer's objects (docs/adr/0210 §4) in one call:
/// `texts` holds each object's start and end values one after another (two
/// per object), `lens` their lengths in UTF-16 units, −1 for a value the
/// object lacks. Out, per object, `s, e, mode` (mode −1: timeless, s and e
/// NaN), then `timed, timeless, unreadable, extent start, extent end` (NaN
/// without an extent).
#[wasm_bindgen(js_name = timeLayer)]
pub fn time_layer(
    ranged: bool,
    cumulative: bool,
    texts: &str,
    lens: &[i32],
) -> Result<Vec<f64>, JsError> {
    if !lens.len().is_multiple_of(2) {
        return Err(JsError::new("Zaman değerleri nesne başına iki olmalı."));
    }
    let values = split(texts, lens)?;
    let (times, sum) = time::layer_times(
        Rule { ranged, cumulative },
        values.chunks_exact(2).map(|c| (c[0], c[1])),
    );
    let mut out = Vec::with_capacity(times.len() * 3 + 5);
    for t in &times {
        match t {
            Some(t) => out.extend([t.s, t.e, f64::from(t.mode.code())]),
            None => out.extend([f64::NAN, f64::NAN, -1.0]),
        }
    }
    let (lo, hi) = sum.extent.unwrap_or((f64::NAN, f64::NAN));
    out.extend([
        sum.timed as f64,
        sum.timeless as f64,
        sum.unreadable as f64,
        lo,
        hi,
    ]);
    Ok(out)
}

/// Which of a temporal layer's objects show in a window (Veri karşılaştır's
/// side at a date, docs/adr/0210 §8): `texts` and `lens` as `timeLayer`'s,
/// the window as `setTimeWindow`'s (`kind` 1 the moment `a`, 2 from `a` up to
/// `b`); per object 1 when it shows, a timeless one always.
#[wasm_bindgen(js_name = timeLayerMask)]
pub fn time_layer_mask(
    ranged: bool,
    cumulative: bool,
    texts: &str,
    lens: &[i32],
    kind: u8,
    a: f64,
    b: f64,
) -> Result<Vec<u8>, JsError> {
    if !lens.len().is_multiple_of(2) {
        return Err(JsError::new("Zaman değerleri nesne başına iki olmalı."));
    }
    let window = if kind == 2 {
        Window::Range(a, b)
    } else {
        Window::Instant(a)
    };
    let values = split(texts, lens)?;
    let (times, _) = time::layer_times(
        Rule { ranged, cumulative },
        values.chunks_exact(2).map(|c| (c[0], c[1])),
    );
    Ok(times
        .iter()
        .map(|t| u8::from(t.as_ref().is_none_or(|t| time::shows(t, &window))))
        .collect())
}

#[wasm_bindgen]
impl GeometryStore {
    /// The objects' times (docs/adr/0210 §6): per id `s, e, mode` in `times`;
    /// mode −1 (or one not known) takes the object's time away.
    #[wasm_bindgen(js_name = setTimes)]
    pub fn set_times(&mut self, ids: &[f64], times: &[f64]) {
        let entries = ids.iter().zip(times.chunks_exact(3)).map(|(&id, t)| {
            let mode = (t[2] >= 0.0).then(|| Mode::from_code(t[2] as u8)).flatten();
            (
                id,
                mode.map(|mode| Time {
                    s: t[0],
                    e: t[1],
                    mode,
                }),
            )
        });
        self.store_mut().set_times(entries);
    }

    /// A temporal layer's objects' times read from their values straight into
    /// the store (the viewport's path: nothing crosses back): `texts` and
    /// `lens` as `timeLayer`'s, two per id.
    #[wasm_bindgen(js_name = setLayerTimes)]
    pub fn set_layer_times(
        &mut self,
        ids: &[f64],
        ranged: bool,
        cumulative: bool,
        texts: &str,
        lens: &[i32],
    ) -> Result<(), JsError> {
        if lens.len() != ids.len() * 2 {
            return Err(JsError::new("Zaman değerleri nesne başına iki olmalı."));
        }
        let values = split(texts, lens)?;
        let (times, _) = time::layer_times(
            Rule { ranged, cumulative },
            values.chunks_exact(2).map(|c| (c[0], c[1])),
        );
        self.store_mut().set_times(ids.iter().copied().zip(times));
        Ok(())
    }

    /// Every object's time taken away (no layer is temporal any more).
    #[wasm_bindgen(js_name = clearTimes)]
    pub fn clear_times(&mut self) {
        self.store_mut().clear_times();
    }

    /// The time slider's window: `kind` 0 none (the filter ends), 1 the moment
    /// `a`, 2 from `a` up to `b`.
    #[wasm_bindgen(js_name = setTimeWindow)]
    pub fn set_time_window(&mut self, kind: u8, a: f64, b: f64) {
        let window = match kind {
            1 => Some(Window::Instant(a)),
            2 => Some(Window::Range(a, b)),
            _ => None,
        };
        self.store_mut().set_time_window(window);
    }

    /// How many objects have a time, then the extent of their starts and ends (NaN, NaN without one).
    #[wasm_bindgen(js_name = timeSummary)]
    pub fn time_summary(&self) -> Vec<f64> {
        let (n, extent) = self.store().time_summary();
        let (lo, hi) = extent.unwrap_or((f64::NAN, f64::NAN));
        vec![n as f64, lo, hi]
    }

    /// For each of `ids`, 1 when it shows at the slider's window (the layer builder's filter).
    #[wasm_bindgen(js_name = timeMask)]
    pub fn time_mask(&self, ids: &[f64]) -> Vec<u8> {
        self.store().time_mask(ids)
    }

    /// The objects their layer's filter leaves out (docs/adr/0211 §3): per id
    /// in `ids`, 1 in `out` leaves it out, 0 lets it in again.
    #[wasm_bindgen(js_name = setFiltered)]
    pub fn set_filtered(&mut self, ids: &[f64], out: &[u8]) {
        self.store_mut()
            .set_filtered(ids.iter().copied().zip(out.iter().map(|&o| o != 0)));
    }

    /// Lets every object in again (no layer is filtered any more).
    #[wasm_bindgen(js_name = clearFiltered)]
    pub fn clear_filtered(&mut self) {
        self.store_mut().clear_filtered();
    }

    /// For each of `ids`, 1 when the view shows it: it passes its layer's
    /// filter and shows at the slider's window (the layer builder's filter).
    #[wasm_bindgen(js_name = viewMask)]
    pub fn view_mask(&self, ids: &[f64]) -> Vec<u8> {
        self.store().view_mask(ids)
    }
}
