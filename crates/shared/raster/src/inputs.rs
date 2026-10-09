//! Rasters read on another raster's grid (docs/adr/0233 §2): an operation's
//! inputs, each with its place in the drawing (its object's affine), read a
//! piece of the result at a time. For a block of the result's cells the
//! input's region under them is asked for (its blocks named for the host,
//! then copied out as float64 bands, NaN where a cell has no value: its
//! nodata, NaN, an alpha of 0, or past the raster's edge), and each result
//! cell's centre is looked up in it: the cell it falls in (En yakın), or
//! the four or sixteen centres round it (Çift doğrusal, Kübik).

use kentos_contracts::RasterSample;
use kentos_formats::raster::samples::Samples;
use kentos_formats::raster::source::{BlockNeed, Reader};

use crate::grid::Grid;

/// How a value is read at a point between cell centres.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sampling {
    /// The cell the point is in.
    Nearest,
    /// The four cell centres round it.
    Bilinear,
    /// The sixteen round it, Keys' cubic convolution (a = −½).
    Cubic,
}

impl Sampling {
    pub fn from_key(key: &str) -> Option<Sampling> {
        Some(match key {
            "nearest" => Sampling::Nearest,
            "bilinear" => Sampling::Bilinear,
            "cubic" => Sampling::Cubic,
            _ => return None,
        })
    }

    /// Cells it reads past the one the point is in.
    pub fn margin(self) -> i64 {
        match self {
            Sampling::Nearest => 0,
            Sampling::Bilinear => 1,
            Sampling::Cubic => 2,
        }
    }
}

/// An input raster: its reader and place.
pub struct Input {
    pub reader: Reader,
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
    /// Bands in the file, alpha included.
    pub bands: u32,
    /// The last band is alpha (a mask, not a value).
    pub alpha: bool,
    pub sample: RasterSample,
    pub nodata: Option<f64>,
}

impl std::fmt::Debug for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Input")
            .field("affine", &self.affine)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl Input {
    /// An opened raster at `affine` (its object's), its nodata the look's
    /// (else the file's); its place must turn.
    pub fn new(reader: Reader, affine: [f64; 6], nodata: Option<f64>) -> Result<Input, String> {
        let [_, a, b, _, c, d] = affine;
        let det = a * d - b * c;
        if affine.iter().any(|v| !v.is_finite()) || det == 0.0 || !det.is_finite() {
            return Err(
                "Rasterin yeri (afin dönüşümü) tersinmiyor; önce Raster oturt ile yerleştirin."
                    .into(),
            );
        }
        let info = &reader.info;
        if info.width == 0 || info.height == 0 || info.bands == 0 {
            return Err("Rasterin boyu sıfır.".into());
        }
        Ok(Input {
            affine,
            width: info.width,
            height: info.height,
            bands: info.bands,
            alpha: info.alpha && info.bands > 1,
            sample: info.sample,
            nodata: nodata.or(info.nodata),
            reader,
        })
    }

    /// Its value bands (alpha left out).
    pub fn values(&self) -> u32 {
        self.bands - u32::from(self.alpha)
    }

    /// Its grid.
    pub fn grid(&self) -> Grid {
        Grid {
            affine: self.affine,
            width: self.width,
            height: self.height,
        }
    }

    /// The grid place (u, v) of a drawing point (§2: dx, dy from the origin,
    /// u = (d·dx − b·dy)/D, v = (a·dy − c·dx)/D, in this order).
    #[inline]
    pub fn place(&self, x: f64, y: f64) -> (f64, f64) {
        place_in(&self.affine, x, y)
    }
}

/// ⌊x⌋ as a whole number, without the C library's call (x finite, within ±2⁶³).
#[inline]
pub fn floor_i(x: f64) -> i64 {
    let i = x as i64;
    if (i as f64) > x { i - 1 } else { i }
}

/// The place (u, v) of a drawing point on a grid of `affine` (§2's formula).
#[inline]
pub fn place_in(affine: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    let [x0, a, b, y0, c, d] = *affine;
    let (dx, dy) = (x - x0, y - y0);
    let det = a * d - b * c;
    ((d * dx - b * dy) / det, (a * dy - c * dx) / det)
}

/// The drawing point of grid place (u, v) on `affine`.
#[inline]
pub fn point_of(affine: &[f64; 6], u: f64, v: f64) -> (f64, f64) {
    let [x0, a, b, y0, c, d] = *affine;
    (x0 + a * u + b * v, y0 + c * u + d * v)
}

/// How a result grid's cells sit on an input's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mapping {
    /// The same lattice, cells shifted by whole numbers (k, l): cell (i, j)
    /// is the input's (i + k, j + l). En yakın reads it directly.
    Offset(i64, i64),
    /// Any other: each centre through the drawing.
    Affine,
}

/// How the cells of `grid` sit on `input`'s.
pub fn mapping(grid: &Grid, input: &[f64; 6]) -> Mapping {
    let [_, a, b, _, c, d] = grid.affine;
    let [_, ia, ib, _, ic, id] = *input;
    if (a, b, c, d) != (ia, ib, ic, id) {
        return Mapping::Affine;
    }
    // The grid's origin corner on the input: whole numbers within 10⁻⁹ of a
    // cell, so that every centre's computed place is a whole cell off it.
    let (u, v) = place_in(input, grid.affine[0], grid.affine[3]);
    let (k, l) = (libm::round(u), libm::round(v));
    if (u - k).abs() <= 1e-9 && (v - l).abs() <= 1e-9 && k.abs() < 1e15 && l.abs() < 1e15 {
        Mapping::Offset(k as i64, l as i64)
    } else {
        Mapping::Affine
    }
}

/// A piece of an input under some result cells: its value bands as
/// float64, NaN where there is no value; cells (`x`, `y`) to (`x + w`, `y + h`).
#[derive(Clone, Debug, Default)]
pub struct View {
    pub x: i64,
    pub y: i64,
    pub w: u32,
    pub h: u32,
    /// One vector a value band, row by row.
    pub bands: Vec<Vec<f64>>,
}

impl View {
    /// Band `b`'s value at input cell (i, j); NaN outside the piece.
    #[inline]
    pub fn at(&self, b: usize, i: i64, j: i64) -> f64 {
        let (di, dj) = (i - self.x, j - self.y);
        if di < 0 || dj < 0 || di >= i64::from(self.w) || dj >= i64::from(self.h) {
            return f64::NAN;
        }
        self.bands[b][dj as usize * self.w as usize + di as usize]
    }

    /// Band `b`'s row `j` from column `i` on, `n` cells (NaN outside the piece).
    pub fn row_into(&self, b: usize, i: i64, j: i64, out: &mut [f64]) {
        let dj = j - self.y;
        if dj < 0 || dj >= i64::from(self.h) {
            out.fill(f64::NAN);
            return;
        }
        let row =
            &self.bands[b][dj as usize * self.w as usize..(dj as usize + 1) * self.w as usize];
        for (k, o) in out.iter_mut().enumerate() {
            let di = i + k as i64 - self.x;
            *o = if di >= 0 && di < i64::from(self.w) {
                row[di as usize]
            } else {
                f64::NAN
            };
        }
    }
}

/// The input cells a block of result cells reads: columns `i0..i1`, rows
/// `j0..j1` of `grid`, with `margin` more round them; clipped to the raster.
/// None when the block is off it.
pub fn region_for(
    input: &Input,
    grid: &Grid,
    (i0, i1, j0, j1): (u32, u32, u32, u32),
    margin: i64,
) -> Option<(i64, i64, u32, u32)> {
    if i0 >= i1 || j0 >= j1 {
        return None;
    }
    let (mut umin, mut umax, mut vmin, mut vmax) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for (i, j) in [(i0, j0), (i1 - 1, j0), (i0, j1 - 1), (i1 - 1, j1 - 1)] {
        let (x, y) = point_of(&grid.affine, f64::from(i) + 0.5, f64::from(j) + 0.5);
        let (u, v) = input.place(x, y);
        umin = umin.min(u);
        umax = umax.max(u);
        vmin = vmin.min(v);
        vmax = vmax.max(v);
    }
    if !(umin.is_finite() && umax.is_finite() && vmin.is_finite() && vmax.is_finite()) {
        return None;
    }
    // One cell more each way: an inner centre's computed place may pass a corner's by rounding.
    let m = margin + 1;
    let x0 = (umin.floor() as i64 - m).max(0);
    let y0 = (vmin.floor() as i64 - m).max(0);
    let x1 = (umax.floor() as i64 + m + 1).min(i64::from(input.width));
    let y1 = (vmax.floor() as i64 + m + 1).min(i64::from(input.height));
    (x0 < x1 && y0 < y1).then(|| (x0, y0, (x1 - x0) as u32, (y1 - y0) as u32))
}

/// An input's region as the file holds it, bands interleaved: read a cell
/// at a time, its value bands turned into float64 only as they are read
/// (the vectorizing tools read each cell once; docs/adr/0234 §11).
pub struct Raw {
    pub x: i64,
    pub y: i64,
    pub w: u32,
    pub h: u32,
    samples: Samples,
    all: usize,
    alpha: bool,
    nodata: Option<f64>,
}

impl Raw {
    /// A region of no cells (every read NaN).
    pub fn empty() -> Raw {
        Raw {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            samples: Samples::F64(Vec::new()),
            all: 1,
            alpha: false,
            nodata: None,
        }
    }

    /// Input cells `i …` of row `j`, `out.len() / bands.len()` of them: each
    /// cell's value bands `bands` (indices) in turn as float64, NaN without
    /// a value (nodata, NaN, alpha 0) and outside the region.
    pub fn cells(&self, j: i64, i: i64, bands: &[usize], out: &mut [f64]) {
        let k = bands.len();
        if k == 0 {
            return;
        }
        let dj = j - self.y;
        if dj < 0 || dj >= i64::from(self.h) {
            out.fill(f64::NAN);
            return;
        }
        let (all, alpha, nodata) = (self.all, self.alpha, self.nodata);
        let row = dj as usize * self.w as usize;
        macro_rules! fill {
            ($v:expr) => {{
                let v = $v;
                for (c, o) in out.chunks_exact_mut(k).enumerate() {
                    let di = i + c as i64 - self.x;
                    if di < 0 || di >= i64::from(self.w) {
                        o.fill(f64::NAN);
                        continue;
                    }
                    let at = (row + di as usize) * all;
                    let px = &v[at..at + all];
                    if alpha && f64::from(px[all - 1]) == 0.0 {
                        o.fill(f64::NAN);
                        continue;
                    }
                    for (slot, &b) in o.iter_mut().zip(bands) {
                        let s = f64::from(px[b]);
                        *slot = if s.is_nan() || nodata.is_some_and(|d| s == d) {
                            f64::NAN
                        } else {
                            s
                        };
                    }
                }
            }};
        }
        match &self.samples {
            Samples::U8(v) => fill!(v),
            Samples::I8(v) => fill!(v),
            Samples::U16(v) => fill!(v),
            Samples::I16(v) => fill!(v),
            Samples::U32(v) => fill!(v),
            Samples::I32(v) => fill!(v),
            Samples::F32(v) => fill!(v),
            Samples::F64(v) => fill!(v),
        }
    }
}

impl Input {
    /// The region's samples as the file holds them (its blocks given).
    pub fn raw(&mut self, r: (i64, i64, u32, u32)) -> Result<Raw, String> {
        let (x, y, w, h) = r;
        let region = self
            .reader
            .region(0, x, y, w, h)
            .ok_or_else(|| "Rasterin blokları eksik verildi.".to_owned())?;
        Ok(Raw {
            x,
            y,
            w,
            h,
            all: region.bands as usize,
            samples: region.samples,
            alpha: self.alpha,
            nodata: self.nodata,
        })
    }

    /// The blocks a region wants that the reader does not hold.
    pub fn needs(&mut self, r: (i64, i64, u32, u32)) -> Vec<BlockNeed> {
        self.reader.needs(0, r.0, r.1, r.2, r.3)
    }

    /// The region's value bands (its blocks given), turned into float64s on `threads`.
    pub fn view(&mut self, r: (i64, i64, u32, u32), threads: usize) -> Result<View, String> {
        let (x, y, w, h) = r;
        let region = self
            .reader
            .region(0, x, y, w, h)
            .ok_or_else(|| "Rasterin blokları eksik verildi.".to_owned())?;
        let all = region.bands as usize;
        let values = self.values() as usize;
        let n = w as usize * h as usize;
        let nodata = self.nodata;
        let alpha = self.alpha;
        let mut bands = vec![vec![0.0; n]; values];
        macro_rules! fill {
            ($v:expr) => {{
                let v = $v;
                for (b, band) in bands.iter_mut().enumerate() {
                    crate::par::rows(threads, band, w as usize, &|first, chunk: &mut [f64]| {
                        let at = first * w as usize;
                        for (k, o) in chunk.iter_mut().enumerate() {
                            let px = &v[(at + k) * all..(at + k + 1) * all];
                            let s = f64::from(px[b]);
                            let masked = alpha && f64::from(px[all - 1]) == 0.0;
                            *o = if masked || s.is_nan() || nodata.is_some_and(|d| s == d) {
                                f64::NAN
                            } else {
                                s
                            };
                        }
                    });
                }
            }};
        }
        match &region.samples {
            Samples::U8(v) => fill!(v),
            Samples::I8(v) => fill!(v),
            Samples::U16(v) => fill!(v),
            Samples::I16(v) => fill!(v),
            Samples::U32(v) => fill!(v),
            Samples::I32(v) => fill!(v),
            Samples::F32(v) => fill!(v),
            Samples::F64(v) => fill!(v),
        }
        Ok(View { x, y, w, h, bands })
    }
}

/// Keys' cubic convolution kernel, a = −½ (GDAL's `cubic`).
#[inline]
fn keys(t: f64) -> f64 {
    let t = t.abs();
    if t <= 1.0 {
        (1.5 * t - 2.5) * t * t + 1.0
    } else if t < 2.0 {
        ((-0.5 * t + 2.5) * t - 4.0) * t + 2.0
    } else {
        0.0
    }
}

/// The least sum of the weights left that a value stands on (§8).
pub const LEAST_WEIGHT: f64 = 1e-6;

/// Band `b`'s value at grid place (u, v) of `view`'s input, read by `how`:
/// the cells without a value left out, the rest's weights over their sum.
#[inline]
pub fn sample(view: &View, b: usize, u: f64, v: f64, how: Sampling) -> f64 {
    match how {
        Sampling::Nearest => view.at(b, floor_i(u), floor_i(v)),
        Sampling::Bilinear => {
            let (su, sv) = (u - 0.5, v - 0.5);
            let (i, j) = (floor_i(su), floor_i(sv));
            let (fx, fy) = (su - i as f64, sv - j as f64);
            let mut acc = 0.0;
            let mut wsum = 0.0;
            for (di, wx) in [(0, 1.0 - fx), (1, fx)] {
                for (dj, wy) in [(0, 1.0 - fy), (1, fy)] {
                    let w = wx * wy;
                    let x = view.at(b, i + di, j + dj);
                    if w != 0.0 && !x.is_nan() {
                        acc += w * x;
                        wsum += w;
                    }
                }
            }
            weighted(acc, wsum)
        }
        Sampling::Cubic => {
            let (su, sv) = (u - 0.5, v - 0.5);
            let (i, j) = (floor_i(su), floor_i(sv));
            let (fx, fy) = (su - i as f64, sv - j as f64);
            let wx = [keys(fx + 1.0), keys(fx), keys(1.0 - fx), keys(2.0 - fx)];
            let wy = [keys(fy + 1.0), keys(fy), keys(1.0 - fy), keys(2.0 - fy)];
            let mut acc = 0.0;
            let mut wsum = 0.0;
            for (dj, &wyj) in wy.iter().enumerate() {
                for (di, &wxi) in wx.iter().enumerate() {
                    let w = wxi * wyj;
                    let x = view.at(b, i + di as i64 - 1, j + dj as i64 - 1);
                    if w != 0.0 && !x.is_nan() {
                        acc += w * x;
                        wsum += w;
                    }
                }
            }
            weighted(acc, wsum)
        }
    }
}

#[inline]
fn weighted(acc: f64, wsum: f64) -> f64 {
    if wsum.abs() < LEAST_WEIGHT {
        f64::NAN
    } else {
        acc / wsum
    }
}

/// Band `b` of result row `j`, columns `i0..i0 + out.len()`, read from `view`.
pub fn sample_row(
    (grid, input): (&Grid, &[f64; 6]),
    map: Mapping,
    view: &View,
    b: usize,
    (i0, j): (u32, u32),
    how: Sampling,
    out: &mut [f64],
) {
    if let (Mapping::Offset(k, l), Sampling::Nearest) = (map, how) {
        view.row_into(b, i64::from(i0) + k, i64::from(j) + l, out);
        return;
    }
    let cv = f64::from(j) + 0.5;
    for (n, o) in out.iter_mut().enumerate() {
        let (x, y) = point_of(&grid.affine, f64::from(i0) + n as f64 + 0.5, cv);
        let (u, v) = place_in(input, x, y);
        *o = sample(view, b, u, v, how);
    }
}

/// What a result without a value holds (§2): the source's nodata if it
/// has one; NaN in floats; an alpha band (added to RGB) in 3- or 4-band
/// bytes; else the integer type's end (the least signed, the largest
/// unsigned). The nodata to write, and whether the result's last band is alpha.
pub fn empty_of(
    sample: RasterSample,
    values: u32,
    alpha: bool,
    nodata: Option<f64>,
) -> (Option<f64>, bool) {
    if let Some(d) = nodata {
        return (Some(d), alpha);
    }
    match sample {
        RasterSample::F32 | RasterSample::F64 => (Some(f64::NAN), alpha),
        RasterSample::U8 if alpha || values == 3 => (None, true),
        RasterSample::U8 => (Some(255.0), false),
        RasterSample::U16 => (Some(65_535.0), false),
        RasterSample::U32 => (Some(4_294_967_295.0), false),
        RasterSample::I8 => (Some(-128.0), false),
        RasterSample::I16 => (Some(-32_768.0), false),
        RasterSample::I32 => (Some(-2_147_483_648.0), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_kernel_interpolates() {
        assert_eq!(keys(0.0), 1.0);
        assert_eq!(keys(1.0), 0.0);
        assert_eq!(keys(2.0), 0.0);
        // The weights of any offset sum to one.
        for f in [0.0, 0.1, 0.25, 0.5, 0.9] {
            let s = keys(f + 1.0) + keys(f) + keys(1.0 - f) + keys(2.0 - f);
            assert!((s - 1.0).abs() < 1e-15, "{f}: {s}");
        }
    }

    #[test]
    fn places_and_mappings() {
        let g = Grid {
            affine: [100.0, 2.0, 0.0, 200.0, 0.0, -2.0],
            width: 10,
            height: 10,
        };
        assert_eq!(place_in(&g.affine, 103.0, 197.0), (1.5, 1.5));
        assert_eq!(mapping(&g, &g.affine), Mapping::Offset(0, 0));
        // Two cells east, one south.
        let other = [96.0, 2.0, 0.0, 202.0, 0.0, -2.0];
        assert_eq!(mapping(&g, &other), Mapping::Offset(2, 1));
        // Half a cell off: through the drawing.
        assert_eq!(
            mapping(&g, &[101.0, 2.0, 0.0, 200.0, 0.0, -2.0]),
            Mapping::Affine
        );
    }

    #[test]
    fn sampling_leaves_out_cells_without_a_value() {
        // 2 × 2: 0 1 / 2 NaN.
        let view = View {
            x: 0,
            y: 0,
            w: 2,
            h: 2,
            bands: vec![vec![0.0, 1.0, 2.0, f64::NAN]],
        };
        assert!(sample(&view, 0, 1.0, 1.0, Sampling::Nearest).is_nan());
        assert_eq!(sample(&view, 0, 0.5, 0.5, Sampling::Nearest), 0.0);
        // At the middle of the four centres: the three with a value, equally.
        assert_eq!(sample(&view, 0, 1.0, 1.0, Sampling::Bilinear), 1.0);
        // On a centre: that cell.
        assert_eq!(sample(&view, 0, 1.5, 0.5, Sampling::Bilinear), 1.0);
        assert_eq!(sample(&view, 0, 1.5, 0.5, Sampling::Cubic), 1.0);
    }
}
