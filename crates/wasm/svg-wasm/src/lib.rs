//! The SVG editor's geometry in the browser (docs/adr/0008 “SVG
//! düzenleyicisi”). A package of its own, loaded with the editor (CLAUDE.md
//! §20): the app's start does not pay for it. Calls cross as JSON, as in
//! the geometry core's table (`apps/web/src/style/svg/core.ts`).

use wasm_bindgen::prelude::*;

/// The id of an operation by its TypeScript name, or −1 when this build has no such operation.
#[wasm_bindgen(js_name = opId)]
pub fn op_id(name: &str) -> i32 {
    kentos_svg_core::api::find(name).map_or(-1, |i| i as i32)
}

/// Runs an operation on a JSON array of arguments; the result is JSON. Unreadable arguments throw.
#[wasm_bindgen(js_name = callOp)]
pub fn call_op(id: u32, args: &str) -> Result<String, JsError> {
    kentos_svg_core::api::run(id as usize, args).map_err(|e| JsError::new(&e))
}

/// The editor's snap index, built once per drawing and view and asked on every pointer move.
#[wasm_bindgen]
pub struct SnapIndex(kentos_svg_core::snap::SnapIndex);

#[wasm_bindgen]
impl SnapIndex {
    /// Builds the index from its source (JSON: shapes, guides, page, kinds, excluded shapes, skipped nodes).
    pub fn of(source: &str) -> Result<SnapIndex, JsError> {
        use kentos_geometry_core::api::json::{FromJson, Json};
        let v = Json::parse(source).map_err(|e| JsError::new(&e))?;
        let src = kentos_svg_core::snap::SnapSource::from_json(&v).map_err(|e| JsError::new(&e))?;
        kentos_svg_core::snap::SnapIndex::new(&src)
            .map(SnapIndex)
            .map_err(|e| JsError::new(&e))
    }

    /// The best snap within `r` of (x, y) as JSON, or "null"; `from` (when `has_from`) is where the tool started.
    pub fn query(&self, x: f64, y: f64, r: f64, has_from: bool, fx: f64, fy: f64) -> String {
        let from = has_from.then_some([fx, fy]);
        kentos_geometry_core::api::json::to_string(&self.0.query([x, y], r, from))
    }
}

/// Ink (1) where a pixel (RGBA bytes, row by row) on white paper is darker than the threshold.
#[wasm_bindgen(js_name = inkMask)]
pub fn ink_mask(width: u32, height: u32, data: &[u8], threshold: f64, invert: bool) -> Vec<u8> {
    kentos_svg_core::trace::ink_mask(width as usize, height as usize, data, threshold, invert)
}

/// `inkMask` for pixels given as numbers (a plain array).
#[wasm_bindgen(js_name = inkMaskNumbers)]
pub fn ink_mask_numbers(
    width: u32,
    height: u32,
    data: &[f64],
    threshold: f64,
    invert: bool,
) -> Vec<u8> {
    kentos_svg_core::trace::ink_mask(width as usize, height as usize, data, threshold, invert)
}

/// Outlines of a mask's ink (marching squares) as JSON rings.
#[wasm_bindgen(js_name = traceContours)]
pub fn trace_contours(mask: &[u8], w: u32, h: u32) -> String {
    kentos_geometry_core::api::json::to_string(&kentos_svg_core::trace::trace_contours(
        mask, w as usize, h as usize,
    ))
}

fn trace_with<P: kentos_svg_core::trace::Pixels + ?Sized>(
    width: u32,
    height: u32,
    data: &P,
    options: &str,
) -> Result<String, JsError> {
    use kentos_geometry_core::api::json::{FromJson, Json};
    let v = Json::parse(options).map_err(|e| JsError::new(&e))?;
    let o = kentos_svg_core::trace::TraceOptions::from_json(&v).map_err(|e| JsError::new(&e))?;
    let r = kentos_svg_core::trace::trace_bitmap(width as usize, height as usize, data, &o);
    Ok(kentos_geometry_core::api::json::to_string(&r))
}

/// The whole trace of a picture (RGBA bytes) with every option given (JSON).
#[wasm_bindgen(js_name = traceBitmap)]
pub fn trace_bitmap(
    width: u32,
    height: u32,
    data: &[u8],
    options: &str,
) -> Result<String, JsError> {
    trace_with(width, height, data, options)
}

/// `traceBitmap` for pixels given as numbers (a plain array).
#[wasm_bindgen(js_name = traceBitmapNumbers)]
pub fn trace_bitmap_numbers(
    width: u32,
    height: u32,
    data: &[f64],
    options: &str,
) -> Result<String, JsError> {
    trace_with(width, height, data, options)
}

/// A PNG with its resolution recorded (a pHYs chunk after IHDR); undefined when the bytes are not a PNG.
#[wasm_bindgen(js_name = withPngDpi)]
pub fn with_png_dpi(png: &[u8], dpi: f64) -> Option<Vec<u8>> {
    kentos_svg_core::export::with_png_dpi(png, dpi)
}

/// CRC-32 of the bytes (as PNG chunks carry it).
#[wasm_bindgen]
pub fn crc32(bytes: &[u8]) -> u32 {
    kentos_svg_core::export::crc32(bytes)
}
