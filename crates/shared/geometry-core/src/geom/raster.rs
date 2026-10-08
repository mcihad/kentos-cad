//! A raster's frame (docs/adr/0204 §2, §7): the parallelogram its affine
//! gives its pixels. Column i and row j (pixel corners, 0, 0 the upper
//! left) lie at x = x₀ + a·i + b·j, y = y₀ + c·i + d·j (GDAL's order). The
//! frame is what the drawing knows of a raster: it is picked by its edge,
//! snapped at its corners and edges' middles, moved, turned, scaled and
//! mirrored by composing the affine; its pixels are the raster pass's, which
//! draws each frame the tiles of the level whose pixel is about a device
//! pixel that the view meets (`level_for`, `visible`; both platforms' passes).

use crate::entity::Shape;
use crate::geom::affine::{self, Affine};
use crate::vec2::Vec2;

/// A raster's affine and size; none for any other shape.
pub fn of(shape: &Shape) -> Option<([f64; 6], f64, f64)> {
    match shape {
        Shape::Raster {
            affine,
            width,
            height,
            ..
        } => Some((*affine, *width, *height)),
        _ => None,
    }
}

/// Where pixel corner (i, j) lies.
pub fn at([x0, a, b, y0, c, d]: [f64; 6], i: f64, j: f64) -> Vec2 {
    Vec2::new(x0 + a * i + b * j, y0 + c * i + d * j)
}

/// The frame's corners counter-clockwise from the lower left of a north-up
/// raster: pixel corners (0, h), (w, h), (w, 0), (0, 0), reversed when the
/// affine turns the frame over (a mirrored raster).
pub fn corners(shape: &Shape) -> Option<[Vec2; 4]> {
    let (affine, w, h) = of(shape)?;
    let mut out = [
        at(affine, 0.0, h),
        at(affine, w, h),
        at(affine, w, 0.0),
        at(affine, 0.0, 0.0),
    ];
    let [_, a, b, _, c, d] = affine;
    // The rows go down: a north-up raster's a·d − b·c is negative, and its frame turns counter-clockwise.
    if a * d - b * c > 0.0 {
        out.reverse();
    }
    Some(out)
}

/// The affine after `t` moves the drawing: x' = t(x) for every pixel's corner.
pub fn transformed([x0, a, b, y0, c, d]: [f64; 6], t: &Affine) -> [f64; 6] {
    let o = affine::apply(t, Vec2::new(x0, y0));
    // The linear part takes the columns' and the rows' steps.
    let col = affine::apply_linear(t, Vec2::new(a, c));
    let row = affine::apply_linear(t, Vec2::new(b, d));
    [o.x, col.x, row.x, o.y, col.y, row.y]
}

/// The pixel (column, row; fractional) a point lies on; none when the affine does not invert.
pub fn pixel_of([x0, a, b, y0, c, d]: [f64; 6], p: Vec2) -> Option<Vec2> {
    let det = a * d - b * c;
    if !(det.is_finite() && det != 0.0) {
        return None;
    }
    let (dx, dy) = (p.x - x0, p.y - y0);
    Some(Vec2::new((d * dx - b * dy) / det, (a * dy - c * dx) / det))
}

// ── Which tiles a view draws ─────────────────────────────────────────────

/// A tile's side, pixels (docs/adr/0204 §3).
pub const TILE: u32 = 256;

/// The sizes of a raster's levels: ⌈w / 2ᵏ⌉ × ⌈h / 2ᵏ⌉ down to the first that fits in a tile.
pub fn level_sizes(width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut out = vec![(width, height)];
    let (mut w, mut h) = (width, height);
    while w > TILE || h > TILE {
        w = w.div_ceil(2);
        h = h.div_ceil(2);
        out.push((w, h));
    }
    out
}

/// A tile to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawTile {
    pub level: u32,
    pub tx: u32,
    pub ty: u32,
    /// Its corners in the drawing: pixel corners (x₀, y₀), (x₁, y₀), (x₁, y₁), (x₀, y₁), as xs and ys.
    pub corners: [[f64; 4]; 2],
    /// The share of the texture's width and height (of 256) that are the raster's.
    pub uv: [f64; 2],
}

/// The level a view at `ppm` device pixels a metre draws, for a raster
/// whose level-0 pixel is `s` metres and which has `levels` levels: the
/// one whose pixel is about a device pixel, ⌊log₂ (1 / (s · ppm))⌋ between
/// 0 and the last (docs/adr/0204 §5).
pub fn level_for(s: f64, ppm: f64, levels: usize) -> usize {
    let mut q = s * ppm;
    let mut k = 0;
    if !(q > 0.0) || !q.is_finite() {
        return levels.saturating_sub(1);
    }
    while q * 2.0 <= 1.0 && k + 1 < levels {
        q *= 2.0;
        k += 1;
    }
    k
}

/// The tiles of `level` (`sizes[level]` its size) of a raster of `width` ×
/// `height` placed by `affine` that the view rectangle (`min_x`, `min_y`,
/// `max_x`, `max_y`) meets, into `out` (cleared first), nearest the view's
/// centre first. It runs every frame: no allocation once `out` is grown.
pub fn visible(
    affine: &[f64; 6],
    width: u32,
    height: u32,
    sizes: &[(u32, u32)],
    level: usize,
    view: [f64; 4],
    out: &mut Vec<DrawTile>,
) {
    use crate::jsmath::{js_max, js_min};
    out.clear();
    let Some(&(lw, lh)) = sizes.get(level) else {
        return;
    };
    let [x0, a, b, y0, c, d] = *affine;
    let det = a * d - b * c;
    if !(det.is_finite() && det != 0.0) {
        return;
    }
    let pixel = |x: f64, y: f64| {
        let (dx, dy) = (x - x0, y - y0);
        ((d * dx - b * dy) / det, (a * dy - c * dx) / det)
    };
    let corners = [
        pixel(view[0], view[1]),
        pixel(view[2], view[1]),
        pixel(view[2], view[3]),
        pixel(view[0], view[3]),
    ];
    let (mut i0, mut j0, mut i1, mut j1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for (i, j) in corners {
        i0 = js_min(i0, i);
        j0 = js_min(j0, j);
        i1 = js_max(i1, i);
        j1 = js_max(j1, j);
    }
    let (w, h) = (f64::from(width), f64::from(height));
    if !(i1 > 0.0 && j1 > 0.0 && i0 < w && j0 < h) {
        return;
    }
    let f = f64::from(1u32 << level.min(31));
    let span = f * f64::from(TILE);
    let last_x = lw.div_ceil(TILE).saturating_sub(1);
    let last_y = lh.div_ceil(TILE).saturating_sub(1);
    let tile_of = |v: f64, last: u32| (js_max((v / span).floor(), 0.0) as u32).min(last);
    let (tx0, tx1) = (
        tile_of(js_max(i0, 0.0), last_x),
        tile_of(js_max(js_min(i1, w) - 1e-9, 0.0), last_x),
    );
    let (ty0, ty1) = (
        tile_of(js_max(j0, 0.0), last_y),
        tile_of(js_max(js_min(j1, h) - 1e-9, 0.0), last_y),
    );
    let at = |i: f64, j: f64| [x0 + a * i + b * j, y0 + c * i + d * j];
    for ty in ty0..=ty1 {
        for tx in tx0..=tx1 {
            let px0 = f64::from(tx) * span;
            let py0 = f64::from(ty) * span;
            let px1 = js_min(px0 + span, w);
            let py1 = js_min(py0 + span, h);
            let p = [at(px0, py0), at(px1, py0), at(px1, py1), at(px0, py1)];
            out.push(DrawTile {
                level: level as u32,
                tx,
                ty,
                corners: [
                    [p[0][0], p[1][0], p[2][0], p[3][0]],
                    [p[0][1], p[1][1], p[2][1], p[3][1]],
                ],
                uv: [(px1 - px0) / span, (py1 - py0) / span],
            });
        }
    }
    // The view's centre first, so that what is looked at fills in first (in place: no allocation).
    let (ci, cj) = pixel(0.5 * (view[0] + view[2]), 0.5 * (view[1] + view[3]));
    let dist = |t: &DrawTile| {
        let (mx, my) = (
            (f64::from(t.tx) + 0.5) * span - ci,
            (f64::from(t.ty) + 0.5) * span - cj,
        );
        mx * mx + my * my
    };
    out.sort_unstable_by(|p, q| {
        dist(p)
            .total_cmp(&dist(q))
            .then((p.ty, p.tx).cmp(&(q.ty, q.tx)))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn north_up() -> Shape {
        Shape::Raster {
            affine: [100.0, 0.5, 0.0, 200.0, 0.0, -0.5],
            width: 40.0,
            height: 20.0,
            bands: 3.0,
            sample: "u8".into(),
            asset: None,
            file: Some("a.tif".into()),
            srid: 5256.0,
            style: crate::api::json::Json::Null,
            opacity: None,
        }
    }

    #[test]
    fn the_level_is_the_one_whose_pixel_is_about_a_device_pixel() {
        // 0.1 m pixels: at 10 px/m a pixel is a device pixel (level 0); at 2.5 px/m four are (level 2).
        assert_eq!(level_for(0.1, 10.0, 6), 0);
        assert_eq!(level_for(0.1, 40.0, 6), 0);
        assert_eq!(level_for(0.1, 2.5, 6), 2);
        assert_eq!(level_for(0.1, 2.4, 6), 2);
        assert_eq!(level_for(0.1, 0.001, 6), 5);
    }

    #[test]
    fn the_tiles_met_are_listed_centre_first() {
        // 1000 × 600 pixels of 1 m from (0, 600) down to (1000, 0).
        let affine = [0.0, 1.0, 0.0, 600.0, 0.0, -1.0];
        let sizes = level_sizes(1000, 600);
        assert_eq!(sizes, vec![(1000, 600), (500, 300), (250, 150)]);
        let mut out = Vec::new();
        visible(
            &affine,
            1000,
            600,
            &sizes,
            0,
            [300.0, 100.0, 700.0, 500.0],
            &mut out,
        );
        assert_eq!(out.len(), 4);
        // The whole raster: 4 × 3 tiles, (1, 1) nearest the centre (500, 300).
        visible(
            &affine,
            1000,
            600,
            &sizes,
            0,
            [-10.0, -10.0, 1010.0, 610.0],
            &mut out,
        );
        assert_eq!(out.len(), 12);
        assert_eq!((out[0].tx, out[0].ty), (1, 1));
        // The last column is cut: 1000 − 768 = 232 of 256.
        let edge = out.iter().find(|t| t.tx == 3).expect("an edge tile");
        assert_eq!(edge.uv[0], 232.0 / 256.0);
        assert_eq!(edge.corners[0][1], 1000.0);
        // Level 1: 500 × 300 pixels of 2 m, 2 × 2 tiles.
        visible(
            &affine,
            1000,
            600,
            &sizes,
            1,
            [-10.0, -10.0, 1010.0, 610.0],
            &mut out,
        );
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn a_north_up_frame_turns_counter_clockwise() {
        let c = corners(&north_up()).expect("a frame");
        assert_eq!(
            c,
            [
                Vec2::new(100.0, 190.0),
                Vec2::new(120.0, 190.0),
                Vec2::new(120.0, 200.0),
                Vec2::new(100.0, 200.0)
            ]
        );
        let area: f64 = (0..4)
            .map(|k| c[k].x * c[(k + 1) % 4].y - c[(k + 1) % 4].x * c[k].y)
            .sum();
        assert!(area > 0.0);
    }

    #[test]
    fn a_move_and_a_turn_compose_with_the_affine() {
        let a = [100.0, 0.5, 0.0, 200.0, 0.0, -0.5];
        let moved = transformed(a, &affine::translation(10.0, -5.0));
        assert_eq!(moved, [110.0, 0.5, 0.0, 195.0, 0.0, -0.5]);
        let turned = transformed(
            a,
            &affine::rotation(std::f64::consts::FRAC_PI_2, Vec2::new(100.0, 200.0)),
        );
        let p = at(turned, 2.0, 0.0);
        assert!(
            (p.x - 100.0).abs() < 1e-12 && (p.y - 201.0).abs() < 1e-12,
            "{p:?}"
        );
        let back = pixel_of(turned, p).expect("inverts");
        assert!(
            (back.x - 2.0).abs() < 1e-12 && back.y.abs() < 1e-12,
            "{back:?}"
        );
    }
}
