//! The geometry store's flat outlines as the preview draws them: what
//! `outline_paths`, `transform_outlines` and `stretch_outlines` give,
//! `flags, n, x0, y0, …` per path (0 open, 1 closed, 2 a marker for a point
//! or a text), read into lines and point marks (the web's `strokePaths`).

use crate::Vec2;
use crate::tool::Stroke;

/// The paths' lines, each drawn by `line` from its points and whether it
/// closes, and the markers' points. A line needs two points.
pub(crate) fn read(
    paths: &[f64],
    line: impl Fn(Vec<Vec2>, bool) -> Stroke,
) -> (Vec<Stroke>, Vec<Vec2>) {
    let mut lines = Vec::new();
    let mut marks = Vec::new();
    let mut at = 0;
    while at + 1 < paths.len() {
        let flags = paths[at];
        let n = paths[at + 1] as usize;
        let pts: Vec<Vec2> = (0..n)
            .filter_map(|k| {
                let (x, y) = (*paths.get(at + 2 + 2 * k)?, *paths.get(at + 3 + 2 * k)?);
                Some(Vec2::new(x, y))
            })
            .collect();
        at += 2 + 2 * n;
        if flags == 2.0 {
            marks.extend(pts.first());
        } else if pts.len() >= 2 {
            lines.push(line(pts, flags == 1.0));
        }
    }
    (lines, marks)
}
