//! The geometry core in the browser. The boundary is kept narrow and cheap:
//! coordinates cross as flat `Float64Array`s (x0, y0, x1, y1, …) and results
//! come back as numbers or small arrays, never as objects to walk. The
//! authoritative checks still run on the server before a commit
//! (CLAUDE.md §14); this is for a responsive interface.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::hatch::hatch_lines;
use kentos_geometry_core::geom::offset::offset_path;
use kentos_geometry_core::geometry::{
    angle_deg, bearing_grad, centroid, dist, dist_to_segment, path_length, point_in_polygon,
    signed_area,
};
use kentos_geometry_core::processing::numbering::corner_text_at;
use kentos_geometry_core::triangulate::triangulate_many;
use wasm_bindgen::prelude::*;

pub mod faces;
pub mod store;

fn points(xy: &[f64]) -> Vec<Vec2> {
    xy.chunks_exact(2).map(|c| Vec2::new(c[0], c[1])).collect()
}

/// The id of a core operation by its TypeScript name (`apps/web/src/wasm/core.ts`
/// asks once per operation), or −1 when this build has no such operation. The
/// style core's operations come after the geometry core's.
#[wasm_bindgen(js_name = opId)]
pub fn op_id(name: &str) -> i32 {
    kentos_geometry_core::api::find(name)
        .or_else(|| {
            kentos_style_core::api::find(name).map(|i| kentos_geometry_core::api::all().len() + i)
        })
        .map_or(-1, |i| i as i32)
}

/// Runs a core operation on a JSON array of arguments; the result is JSON
/// (docs/adr/0008). Unreadable arguments throw.
#[wasm_bindgen(js_name = callOp)]
pub fn call_op(id: u32, args: &str) -> Result<String, JsError> {
    let geometry = kentos_geometry_core::api::all().len();
    let id = id as usize;
    if id < geometry {
        kentos_geometry_core::api::run(id, args)
    } else {
        kentos_style_core::api::run(id - geometry, args)
    }
    .map_err(|e| JsError::new(&e))
}

/// One expression's values for a table of objects
/// (`kentos_style_core::expr::rows` has the table's layout).
#[wasm_bindgen]
pub struct ExprColumn {
    kinds: Vec<u8>,
    numbers: Vec<f64>,
    texts: String,
    text_lens: Vec<u32>,
}

#[wasm_bindgen]
impl ExprColumn {
    /// Per object: 0 empty, 1 number, 2 text, 3 true/false.
    #[wasm_bindgen(getter)]
    pub fn kinds(&self) -> Vec<u8> {
        self.kinds.clone()
    }

    /// Per object: the number, 1/0 for true/false, NaN otherwise.
    #[wasm_bindgen(getter)]
    pub fn numbers(&self) -> Vec<f64> {
        self.numbers.clone()
    }

    /// The text values one after another…
    #[wasm_bindgen(getter)]
    pub fn texts(&self) -> String {
        self.texts.clone()
    }

    /// …and their lengths in UTF-16 code units.
    #[wasm_bindgen(getter, js_name = textLengths)]
    pub fn text_lengths(&self) -> Vec<u32> {
        self.text_lens.clone()
    }
}

/// Evaluates an expression for `n` objects in one call (drawing a layer, a
/// processing run), each value as `want` asks (0 as it is, 1 a number, 2
/// text, 3 true/false, 4 the number its text reads as). The source is compiled again here: compiling costs
/// microseconds, and no compiled expression lives on in the core's memory.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = exprEvaluate)]
pub fn expr_evaluate(
    source: &str,
    n: u32,
    texts: &str,
    text_lens: &[i32],
    numbers: &[f64],
    measures: &[f64],
    scale: f64,
    want: u8,
) -> Result<ExprColumn, JsError> {
    use kentos_style_core::expr::{compile, rows};
    let e = compile(source).map_err(|e| JsError::new(&e.text()))?;
    let c = rows::evaluate_rows(
        &e,
        &rows::RowsInput {
            n: n as usize,
            texts,
            text_lens,
            numbers,
            measures,
            scale,
        },
        rows::As::from_code(want),
    )
    .map_err(|e| JsError::new(&e))?;
    Ok(ExprColumn {
        kinds: c.kinds,
        numbers: c.numbers,
        texts: c.texts,
        text_lens: c.text_lens,
    })
}

/// Version of the core, to tell a stale WASM build from the app.
#[wasm_bindgen(js_name = coreVersion)]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// `model/geometry.ts` on numbers (S3b): the tools call these on every
// pointer move, and a JSON call would cost a hundred times the arithmetic.

#[wasm_bindgen(js_name = dist)]
pub fn dist_js(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    dist(Vec2::new(ax, ay), Vec2::new(bx, by))
}

#[wasm_bindgen(js_name = angleDeg)]
pub fn angle_deg_js(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    angle_deg(Vec2::new(ax, ay), Vec2::new(bx, by))
}

#[wasm_bindgen(js_name = bearingGrad)]
pub fn bearing_grad_js(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    bearing_grad(Vec2::new(ax, ay), Vec2::new(bx, by))
}

#[wasm_bindgen(js_name = distToSegment)]
pub fn dist_to_segment_js(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    dist_to_segment(Vec2::new(px, py), Vec2::new(ax, ay), Vec2::new(bx, by))
}

// A scratch buffer in the core's memory for the small ring measures (S3b):
// the page writes a ring's coordinates straight into it and reads the answer
// back from it, so a call allocates nothing on either side. The style engine
// asks for a centroid per object while a layer is built.
thread_local! {
    static SCRATCH: std::cell::RefCell<Vec<f64>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Where `len` numbers can be written (valid until the next `scratch` call
/// asks for more); the page views it through the module's memory.
#[wasm_bindgen]
pub fn scratch(len: usize) -> *const f64 {
    SCRATCH.with(|s| {
        let mut v = s.borrow_mut();
        if v.len() < len {
            v.resize(len, 0.0);
        }
        v.as_ptr()
    })
}

fn with_scratch<R>(n: usize, f: impl FnOnce(&[Vec2], &mut [f64]) -> R) -> R {
    SCRATCH.with(|s| {
        let mut v = s.borrow_mut();
        let n = n.min(v.len() / 2);
        let pts = points(&v[..2 * n]);
        f(&pts, &mut v)
    })
}

/// Centroid of the `n` points in the scratch buffer, written back to its first two numbers.
#[wasm_bindgen(js_name = scratchCentroid)]
pub fn scratch_centroid(n: usize) {
    with_scratch(n, |pts, out| {
        let c = centroid(pts);
        if out.len() >= 2 {
            out[0] = c.x;
            out[1] = c.y;
        }
    });
}

#[wasm_bindgen(js_name = scratchSignedArea)]
pub fn scratch_signed_area(n: usize) -> f64 {
    with_scratch(n, |pts, _| signed_area(pts))
}

#[wasm_bindgen(js_name = scratchPathLength)]
pub fn scratch_path_length(n: usize, closed: bool) -> f64 {
    with_scratch(n, |pts, _| path_length(pts, closed))
}

#[wasm_bindgen(js_name = scratchPointInPolygon)]
pub fn scratch_point_in_polygon(n: usize, px: f64, py: f64) -> bool {
    with_scratch(n, |pts, _| point_in_polygon(Vec2::new(px, py), pts))
}

/// Point ranges of rings laid one after another (`sizes`: vertex counts),
/// clamped to the `points` there are.
fn ranges(points: usize, sizes: &[u32]) -> Vec<(usize, usize)> {
    let mut out = Vec::with_capacity(sizes.len());
    let mut at = 0usize;
    for &n in sizes {
        let start = at.min(points);
        let end = (start + n as usize).min(points);
        out.push((start, end));
        at = end;
    }
    out
}

/// `offsetPath` on flat coordinates: the style engine offsets a path per
/// object and symbol layer while a layer is built (docs/adr/0008, S3).
#[wasm_bindgen(js_name = offsetPathXY)]
pub fn offset_path_xy(xy: &[f64], d: f64, closed: bool) -> Vec<f64> {
    flat(&offset_path(&points(xy), d, closed))
}

/// `hatchLines` on flat coordinates (the hatch tool's hover preview, every
/// frame): the ring, then every hole's points one after another with
/// `hole_sizes` giving each hole's vertex count. The first number is 1 when
/// the lines were capped, then `ax, ay, bx, by` per segment.
#[wasm_bindgen(js_name = hatchLinesXY)]
pub fn hatch_lines_xy(
    ring: &[f64],
    holes: &[f64],
    hole_sizes: &[u32],
    angle_deg: f64,
    spacing: f64,
) -> Vec<f64> {
    let inner: Vec<Vec<Vec2>> = ranges(holes.len() / 2, hole_sizes)
        .into_iter()
        .map(|(start, end)| points(&holes[2 * start..2 * end]))
        .collect();
    let h = hatch_lines(&points(ring), angle_deg, spacing, &inner);
    let mut out = Vec::with_capacity(1 + 4 * h.segments.len());
    out.push(if h.capped { 1.0 } else { 0.0 });
    for [a, b] in &h.segments {
        out.extend_from_slice(&[a.x, a.y, b.x, b.y]);
    }
    out
}

fn flat(pts: &[Vec2]) -> Vec<f64> {
    pts.iter().flat_map(|p| [p.x, p.y]).collect()
}

/// Fill triangles of many polygons in one call (a layer's fills,
/// docs/adr/0008): `xy` holds every ring's points one after another,
/// `ring_sizes` each ring's vertex count and `poly_rings` each polygon's ring
/// count (its outer ring, then its holes). Three vertex indices (into the
/// points of `xy`) per triangle come back, polygon after polygon.
#[wasm_bindgen(js_name = triangulateMany)]
pub fn triangulate_many_js(xy: &[f64], ring_sizes: &[u32], poly_rings: &[u32]) -> Vec<u32> {
    let sizes: Vec<usize> = ring_sizes.iter().map(|&n| n as usize).collect();
    let polys: Vec<usize> = poly_rings.iter().map(|&n| n as usize).collect();
    triangulate_many(&points(xy), &sizes, &polys)
}

/// Where the texts beside numbered corners go (`corner_text_at`): four
/// numbers per corner in `corners` (x, y, outward x and y), its text in
/// `texts`, measured in the drawing typeface `font`; x, y per corner come back.
#[wasm_bindgen(js_name = cornerTexts)]
pub fn corner_texts_js(corners: &[f64], texts: Vec<String>, height: f64, font: &str) -> Vec<f64> {
    let font = kentos_geometry_core::text::Font::from_id(font);
    corners
        .chunks_exact(4)
        .zip(&texts)
        .flat_map(|(c, t)| {
            let at = corner_text_at(
                Vec2::new(c[0], c[1]),
                Vec2::new(c[2], c[3]),
                t,
                height,
                font,
            );
            [at.x, at.y]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_rings_laid_one_after_another() {
        assert_eq!(ranges(7, &[3, 4]), [(0, 3), (3, 7)]);
    }

    #[test]
    fn sizes_beyond_the_data_are_clamped_not_a_panic() {
        assert_eq!(ranges(1, &[5, 2]), [(0, 1), (1, 1)]);
        let segs = hatch_lines_xy(
            &[0.0, 0.0, 10.0, 0.0, 10.0, 10.0, 0.0, 10.0],
            &[1.0],
            &[5, 2],
            0.0,
            2.5,
        );
        // [capped, then four numbers a segment]: the stray hole numbers are ignored.
        assert_eq!(segs[0], 0.0);
        assert_eq!((segs.len() - 1) % 4, 0);
    }
}
