//! A raster resampled onto a north-up grid by a transform (docs/adr/0204 §6):
//! each output pixel's centre goes back through the inverse transform to the
//! source's pixels and takes the nearest sample or the bilinear mean of four.
//! The inverse is worked out exactly on knots `STEP` pixels apart and
//! bilinearly between, a cell exactly when its middle shows the bilinear
//! off (GDAL's approximate transformer does the like along rows); a pixel
//! that falls outside the source is empty: an 8-bit raster's alpha 0 (its
//! output is RGBA), another's nodata.

use kentos_contracts::RasterSample;
use kentos_geometry_core::vec2::Vec2;

use super::source::Region;
use super::{Samples, TILE};

/// Exact inverse on knots this many output pixels apart, both ways.
pub const STEP: u32 = 8;

/// A cell's bilinear source pixel at most this far (pixels, each axis) from
/// exact at its middle pixel, or the cell is worked out exactly.
pub const CELL_TOLERANCE: f64 = 0.02;

/// A share of a pixel past a grid's edge that makes no row or column.
pub const SLIVER: f64 = 1e-6;

/// The output grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    /// `[x₀, s, 0, y₀, 0, −s]`.
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
}

impl Grid {
    /// The north-up grid of pixel `size` covering the points (the forward
    /// map of the source's border), its corner on a multiple of the size; a
    /// sliver under `SLIVER` of a pixel past an edge makes no row or column
    /// (the transform's last bits would decide it).
    pub fn covering(points: &[Vec2], size: f64) -> Option<Grid> {
        if !(size > 0.0) || points.is_empty() {
            return None;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for p in points {
            if !(p.x.is_finite() && p.y.is_finite()) {
                continue;
            }
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        if !(x1 > x0 && y1 > y0) {
            return None;
        }
        let left = (x0 / size + SLIVER).floor() * size;
        let top = (y1 / size - SLIVER).ceil() * size;
        let width = ((x1 - left) / size - SLIVER).ceil();
        let height = ((top - y0) / size - SLIVER).ceil();
        if width > f64::from(kentos_contracts::MAX_RASTER_SIDE)
            || height > f64::from(kentos_contracts::MAX_RASTER_SIDE)
        {
            return None;
        }
        Some(Grid {
            affine: [left, size, 0.0, top, 0.0, -size],
            width: width.max(1.0) as u32,
            height: height.max(1.0) as u32,
        })
    }

    /// The centre of output pixel (i, j).
    pub fn centre(&self, i: u32, j: u32) -> Vec2 {
        let [x0, s, _, y0, _, t] = self.affine;
        Vec2::new(x0 + (f64::from(i) + 0.5) * s, y0 + (f64::from(j) + 0.5) * t)
    }
}

/// The source pixels (column, row) of every pixel of output tile (`tx`, `ty`),
/// row by row, NaN where the inverse gives none; with the bounding box of
/// the finite ones. The inverse is exact on knots every `step` pixels both
/// ways (`STEP`; 1 exact) and bilinear in the cells between, where the cell's
/// middle pixel shows the bilinear within `CELL_TOLERANCE` of exact; a cell
/// that is not (or has a knot without a pixel) is worked out exactly.
pub fn source_pixels(
    grid: &Grid,
    tx: u32,
    ty: u32,
    step: u32,
    inverse: &dyn Fn(Vec2) -> Option<Vec2>,
) -> (Vec<[f64; 2]>, Option<[f64; 4]>) {
    let mut out = vec![[f64::NAN; 2]; (TILE * TILE) as usize];
    if tx * TILE >= grid.width || ty * TILE >= grid.height {
        return (out, None);
    }
    let (ox, oy) = (tx * TILE, ty * TILE);
    let (lx, ly) = (
        TILE.min(grid.width - ox) - 1,
        TILE.min(grid.height - oy) - 1,
    );
    let step = step.max(1) as usize;
    let knots = |last: u32| {
        let mut k: Vec<u32> = (0..=last).step_by(step).collect();
        if k.last() != Some(&last) {
            k.push(last);
        }
        k
    };
    let (kx, ky) = (knots(lx), knots(ly));
    let exact =
        |i: u32, j: u32| inverse(grid.centre(ox + i, oy + j)).map_or([f64::NAN; 2], |q| [q.x, q.y]);
    let nx = kx.len();
    let mut at = Vec::with_capacity(nx * ky.len());
    for &j in &ky {
        for &i in &kx {
            at.push(exact(i, j));
        }
    }
    // A dimension's cells: the knots on either side (one knot: a cell of its own).
    let cells = |k: &[u32]| -> Vec<(usize, usize)> {
        if k.len() == 1 {
            vec![(0, 0)]
        } else {
            (0..k.len() - 1).map(|c| (c, c + 1)).collect()
        }
    };
    let (cx, cy) = (cells(&kx), cells(&ky));
    for (n, &(y0, y1)) in cy.iter().enumerate() {
        let (ja, jb) = (ky[y0], ky[y1]);
        // A cell has its rows from its first knot up to the next cell's (the last cell its last too).
        let rows = if n + 1 == cy.len() {
            ja..=jb
        } else {
            ja..=jb - 1
        };
        for (m, &(x0, x1)) in cx.iter().enumerate() {
            let (ia, ib) = (kx[x0], kx[x1]);
            let cols = if m + 1 == cx.len() {
                ia..=ib
            } else {
                ia..=ib - 1
            };
            let corners = [
                at[y0 * nx + x0],
                at[y0 * nx + x1],
                at[y1 * nx + x0],
                at[y1 * nx + x1],
            ];
            let lerp = |i: u32, j: u32| -> [f64; 2] {
                let fx = if ib > ia {
                    f64::from(i - ia) / f64::from(ib - ia)
                } else {
                    0.0
                };
                let fy = if jb > ja {
                    f64::from(j - ja) / f64::from(jb - ja)
                } else {
                    0.0
                };
                let [a, b, c, d] = corners;
                let mut p = [0.0; 2];
                for (k, v) in p.iter_mut().enumerate() {
                    let top = a[k] + (b[k] - a[k]) * fx;
                    let bottom = c[k] + (d[k] - c[k]) * fx;
                    *v = top + (bottom - top) * fy;
                }
                p
            };
            let finite = corners.iter().all(|p| p[0].is_finite() && p[1].is_finite());
            let (im, jm) = ((ia + ib) / 2, (ja + jb) / 2);
            let bilinear = finite
                && ((ib - ia <= 1 && jb - ja <= 1) || {
                    let (e, l) = (exact(im, jm), lerp(im, jm));
                    (e[0] - l[0]).abs() <= CELL_TOLERANCE && (e[1] - l[1]).abs() <= CELL_TOLERANCE
                });
            for j in rows.clone() {
                for i in cols.clone() {
                    out[(j * TILE + i) as usize] = if bilinear { lerp(i, j) } else { exact(i, j) };
                }
            }
        }
    }
    let (mut a, mut b, mut c, mut d) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in &out {
        if p[0].is_finite() && p[1].is_finite() {
            a = a.min(p[0]);
            b = b.min(p[1]);
            c = c.max(p[0]);
            d = d.max(p[1]);
        }
    }
    let bbox = (c >= a && d >= b).then_some([a, b, c, d]);
    (out, bbox)
}

/// The source region an output tile needs (with a pixel round it), from the
/// box of its source pixels; none when it falls outside the source.
pub fn region_for(bbox: [f64; 4], width: u32, height: u32) -> Option<(i64, i64, u32, u32)> {
    let [a, b, c, d] = bbox;
    let x0 = (a.floor() as i64 - 1).max(0);
    let y0 = (b.floor() as i64 - 1).max(0);
    let x1 = (c.ceil() as i64 + 1).min(i64::from(width));
    let y1 = (d.ceil() as i64 + 1).min(i64::from(height));
    (x1 > x0 && y1 > y0).then(|| (x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
}

/// An output tile's samples (`TILE` × `TILE`, `out_bands` interleaved) from
/// the source `region`: nearest or bilinear; empty where the pixel falls
/// outside the source (`width` × `height`). With `alpha`, the output's
/// last band is 255 inside and 0 outside, and a grey source is spread to
/// red, green and blue.
#[allow(clippy::too_many_arguments)]
pub fn render(
    pixels: &[[f64; 2]],
    region: &Region,
    width: u32,
    height: u32,
    nearest: bool,
    sample: RasterSample,
    alpha: bool,
    empty: f64,
    palette: Option<&[[u16; 3]]>,
) -> Samples {
    // A palette's indices are taken as they are and spread to their colours.
    let nearest = nearest || palette.is_some();
    let src_bands = region.bands as usize;
    let colour = if alpha { 3 } else { src_bands };
    let out_bands = if alpha { 4 } else { src_bands };
    let mut out = Samples::filled(
        sample,
        (TILE * TILE) as usize * out_bands,
        if alpha { 0.0 } else { empty },
    );
    let (w, h) = (f64::from(width), f64::from(height));
    let get = |i: i64, j: i64, b: usize| -> f64 {
        let (li, lj) = (i - region.x, j - region.y);
        if li < 0 || lj < 0 || li >= i64::from(region.width) || lj >= i64::from(region.height) {
            return f64::NAN;
        }
        region.at(li as u32, lj as u32, b as u32)
    };
    for (k, p) in pixels.iter().enumerate() {
        let (x, y) = (p[0], p[1]);
        if !(x >= 0.0 && y >= 0.0 && x < w && y < h) {
            continue;
        }
        if let Some(pal) = palette {
            let idx = get(x.floor() as i64, y.floor() as i64, 0);
            let c = pal.get(idx as usize).copied().unwrap_or([0, 0, 0]);
            for (k2, v) in c.iter().enumerate() {
                out.set(k * out_bands + k2, f64::from(v >> 8));
            }
            if alpha {
                out.set(k * out_bands + 3, 255.0);
            }
            continue;
        }
        for c in 0..colour {
            let b = if alpha && src_bands < 3 { 0 } else { c };
            let v = if nearest {
                get(x.floor() as i64, y.floor() as i64, b)
            } else {
                // Pixel centres are at +½; the four round the point, clamped to the source.
                let (fx, fy) = (x - 0.5, y - 0.5);
                let (i0, j0) = (fx.floor(), fy.floor());
                let (dx, dy) = (fx - i0, fy - j0);
                let clamp = |v: f64, hi: u32| (v as i64).clamp(0, i64::from(hi) - 1);
                let (i0c, i1c) = (clamp(i0, width), clamp(i0 + 1.0, width));
                let (j0c, j1c) = (clamp(j0, height), clamp(j0 + 1.0, height));
                let v00 = get(i0c, j0c, b);
                let v10 = get(i1c, j0c, b);
                let v01 = get(i0c, j1c, b);
                let v11 = get(i1c, j1c, b);
                (v00 * (1.0 - dx) + v10 * dx) * (1.0 - dy) + (v01 * (1.0 - dx) + v11 * dx) * dy
            };
            out.set(k * out_bands + c, v);
        }
        if alpha {
            let a = if src_bands == 4 || src_bands == 2 {
                get(x.floor() as i64, y.floor() as i64, src_bands - 1)
            } else {
                255.0
            };
            out.set(k * out_bands + 3, a);
        }
    }
    out
}

// ── A resampling under way ────────────────────────────────────────────────

/// Points along a source of `width` × `height` pixels' border (pixel
/// corners, `per_side` steps a side): the forward map of these bounds the
/// output (a polynomial's or a thin plate's border bends).
pub fn border(width: u32, height: u32, per_side: u32) -> Vec<Vec2> {
    let n = per_side.max(1);
    let (w, h) = (f64::from(width), f64::from(height));
    let mut out = Vec::with_capacity(4 * n as usize);
    for k in 0..n {
        let t = f64::from(k) / f64::from(n);
        out.push(Vec2::new(t * w, 0.0));
        out.push(Vec2::new(w, t * h));
        out.push(Vec2::new(w - t * w, h));
        out.push(Vec2::new(0.0, h - t * h));
    }
    out
}

/// The output's pixel by default (docs/adr/0204 §6): the transform's mean
/// scale at the used points' pixels, the square root of the area a pixel
/// covers there; none when no used point gives one.
pub fn default_pixel(forward: &dyn Fn(Vec2) -> Option<Vec2>, pixels: &[Vec2]) -> Option<f64> {
    let (mut sum, mut n) = (0.0, 0.0);
    for &p in pixels {
        let (Some(a), Some(b), Some(c)) = (
            forward(p),
            forward(Vec2::new(p.x + 1.0, p.y)),
            forward(Vec2::new(p.x, p.y + 1.0)),
        ) else {
            continue;
        };
        let area = ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs();
        if area.is_finite() && area > 0.0 {
            sum += area.sqrt();
            n += 1.0;
        }
    }
    (n > 0.0).then(|| sum / n)
}

/// The most source samples one output tile may read (an extreme transform
/// would gather a whole raster for one tile).
pub const MOST_REGION: u64 = 4096 * 4096;

/// A raster resampled into a tiled GeoTIFF, a tile at a time (docs/adr/0204
/// §6): the host asks [`Job::region`] which source pixels the next tile
/// reads, keeps their blocks in the reader, then [`Job::tile`] gives the
/// tile's bytes to append; [`Job::finish`] the directories and the header.
/// An 8-bit or paletted source becomes RGBA, its outside alpha 0; another
/// keeps its bands and samples, its outside `empty` (the file's nodata,
/// else NaN for floats and the type's least for integers).
pub struct Job {
    pub grid: Grid,
    inverse: Box<dyn Fn(Vec2) -> Option<Vec2> + Send>,
    src_w: u32,
    src_h: u32,
    nearest: bool,
    palette: Option<Vec<[u16; 3]>>,
    pub sample: RasterSample,
    pub bands: u32,
    pub alpha: bool,
    pub empty: Option<f64>,
    writer: Option<super::write::Writer>,
    next: u32,
    across: u32,
    down: u32,
    current: Option<Pending>,
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("grid", &self.grid)
            .field("next", &self.next)
            .finish()
    }
}

/// A source region: x, y, width, height (level 0 pixels).
pub type SourceRegion = (i64, i64, u32, u32);

/// The next tile's source pixels and the region they read.
type Pending = (Vec<[f64; 2]>, Option<SourceRegion>);

/// The least an integer sample type holds (an output's empty when the file names no nodata).
fn least(sample: RasterSample) -> f64 {
    match sample {
        RasterSample::U8 | RasterSample::U16 | RasterSample::U32 => 0.0,
        RasterSample::I8 => f64::from(i8::MIN),
        RasterSample::I16 => f64::from(i16::MIN),
        RasterSample::I32 => f64::from(i32::MIN),
        RasterSample::F32 | RasterSample::F64 => f64::NAN,
    }
}

impl Job {
    /// A resampling of a source of `width` × `height`, `bands` bands of
    /// `sample` (with its `nodata` and `palette`), by `inverse` (the
    /// drawing to the source's pixels) onto `grid`; `epsg` and `geographic`
    /// name the output's system. Returns it and the header to write first.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        grid: Grid,
        inverse: Box<dyn Fn(Vec2) -> Option<Vec2> + Send>,
        width: u32,
        height: u32,
        bands: u32,
        sample: RasterSample,
        nodata: Option<f64>,
        palette: Option<Vec<[u16; 3]>>,
        nearest: bool,
        epsg: Option<u32>,
        geographic: bool,
    ) -> Result<(Job, Vec<u8>), super::RasterError> {
        let rgba = sample == RasterSample::U8 || palette.is_some();
        let (out_sample, out_bands, empty) = if rgba {
            (RasterSample::U8, 4, None)
        } else {
            (sample, bands, Some(nodata.unwrap_or_else(|| least(sample))))
        };
        let image = super::write::Image {
            width: grid.width,
            height: grid.height,
            bands: out_bands,
            sample: out_sample,
            alpha: rgba,
            geo: Some(super::write::Geo {
                affine: grid.affine,
                epsg,
                geographic,
            }),
            nodata: empty.filter(|v| !v.is_nan()),
        };
        let (writer, header) = super::write::Writer::new(vec![image], 6)?;
        let job = Job {
            across: grid.width.div_ceil(TILE),
            down: grid.height.div_ceil(TILE),
            grid,
            inverse,
            src_w: width,
            src_h: height,
            nearest,
            palette,
            sample: out_sample,
            bands: out_bands,
            alpha: rgba,
            empty,
            writer: Some(writer),
            next: 0,
            current: None,
        };
        Ok((job, header))
    }

    /// The share done, 0 to 1.
    pub fn share(&self) -> f64 {
        f64::from(self.next) / f64::from((self.across * self.down).max(1))
    }

    /// Whether every tile is written.
    pub fn done(&self) -> bool {
        self.next >= self.across * self.down
    }

    /// The source region (level 0: x, y, width, height) the next tile reads,
    /// `Some(None)` when it reads none (it falls outside); none when done.
    pub fn region(&mut self) -> Option<Option<SourceRegion>> {
        if self.done() {
            return None;
        }
        if self.current.is_none() {
            let (tx, ty) = (self.next % self.across, self.next / self.across);
            let (pixels, bbox) = source_pixels(&self.grid, tx, ty, STEP, &*self.inverse);
            let region = bbox
                .and_then(|b| region_for(b, self.src_w, self.src_h))
                .filter(|r| u64::from(r.2) * u64::from(r.3) <= MOST_REGION);
            self.current = Some((pixels, region));
        }
        self.current.as_ref().map(|(_, r)| *r)
    }

    /// The next tile's bytes to append, its region's samples from `region`
    /// (none when it reads none).
    pub fn tile(
        &mut self,
        region: Option<&super::source::Region>,
    ) -> Result<Vec<u8>, super::RasterError> {
        if self.region().is_none() {
            return Err(super::RasterError::new("Bütün karolar yazıldı."));
        }
        let (pixels, wanted) = self.current.take().unwrap_or_default();
        let (tx, ty) = (self.next % self.across, self.next / self.across);
        let empty = self.empty.unwrap_or(0.0);
        let samples = match (wanted, region) {
            (Some(_), Some(r)) => render(
                &pixels,
                r,
                self.src_w,
                self.src_h,
                self.nearest,
                self.sample,
                self.alpha,
                empty,
                self.palette.as_deref(),
            ),
            _ => Samples::filled(
                self.sample,
                (TILE * TILE * self.bands) as usize,
                if self.alpha { 0.0 } else { empty },
            ),
        };
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| super::RasterError::new("Yazıcı kapandı."))?;
        let bytes = writer.tile(0, tx, ty, &samples)?;
        self.next += 1;
        Ok(bytes)
    }

    /// The directories to append and the header to write over the first one.
    pub fn finish(mut self) -> Result<(Vec<u8>, Vec<u8>), super::RasterError> {
        self.writer
            .take()
            .ok_or_else(|| super::RasterError::new("Yazıcı kapandı."))?
            .finish()
    }
}
