//! The grids a raster run read and wrote (docs/adr/0535, 0643 §4): the shared drivers note them as they open the
//! rasters and write the result; the runner puts them in the run's record when the tool gave none of its own. Kept
//! per thread: a tool runs whole on the thread that computes it.

use std::cell::RefCell;

use kentos_commands::{RasterFields, RecordGrid};
use kentos_raster::grid::Grid;

thread_local! {
    static SEEN: RefCell<Vec<RecordGrid>> = const { RefCell::new(Vec::new()) };
}

/// A run starts: nothing noted yet.
pub(crate) fn begin() {
    SEEN.with(|s| s.borrow_mut().clear());
}

/// What the run noted, in order.
pub(crate) fn take() -> Vec<RecordGrid> {
    SEEN.with(|s| std::mem::take(&mut *s.borrow_mut()))
}

fn push(g: RecordGrid) {
    SEEN.with(|s| s.borrow_mut().push(g));
}

/// A raster read whole (its own grid).
pub(crate) fn input(f: &RasterFields) {
    push(RecordGrid {
        role: "input".into(),
        file: f.file.clone().or_else(|| f.url.clone()),
        affine: f.affine,
        width: f.width,
        height: f.height,
        band: None,
        nodata: f.style.nodata,
        srid: (f.srid > 0).then_some(f.srid),
        window: None,
    });
}

/// The result written at `path` on `grid`.
pub(crate) fn output(path: &str, grid: &Grid, nodata: Option<f64>, srid: Option<u32>) {
    push(RecordGrid {
        role: "output".into(),
        file: Some(path.to_owned()),
        affine: grid.affine,
        width: grid.width,
        height: grid.height,
        band: None,
        nodata,
        srid,
        window: None,
    });
}
