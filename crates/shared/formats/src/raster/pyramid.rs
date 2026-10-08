//! The pyramid file of a large raster without overviews (docs/adr/0204 §3),
//! made in one pass over level 0: its rows go in from the top, each level
//! below takes the mean of 2 × 2 samples of the one above (nodata and NaN
//! left out; a palette's upper left one; held as the sample type holds it,
//! so the file is what `Reader` works out itself), and every level from
//! `PYRAMID_FIRST` on is cut into 256-row bands and written tile by tile.
//! Memory is a row a level and a band a written level, whatever the size of
//! the raster.

use kentos_contracts::RasterSample;

use super::samples::stored;
use super::source::{PYRAMID_FIRST, Region, level_sizes};
use super::write::{Image, Writer};
use super::{RasterError, Samples, TILE};

/// The pass's state.
#[derive(Debug)]
pub struct Builder {
    sizes: Vec<(u32, u32)>,
    bands: usize,
    sample: RasterSample,
    nodata: Option<f64>,
    nearest: bool,
    writer: Writer,
    /// Each level's row waiting for its pair (level k's row, to make k + 1's).
    held: Vec<Option<Vec<f64>>>,
    /// Rows each level has had.
    rows: Vec<u32>,
    /// Each written level's band of rows (`TILE` rows at most).
    band: Vec<Vec<Vec<f64>>>,
    /// Bytes for the host to append, from the last push.
    out: Vec<u8>,
}

impl Builder {
    /// A pass for a raster of `width` × `height` and `bands` bands of
    /// `sample`; `nearest` for a palette. Returns it and the header to write first.
    pub fn new(
        width: u32,
        height: u32,
        bands: u32,
        sample: RasterSample,
        nodata: Option<f64>,
        nearest: bool,
    ) -> Result<(Builder, Vec<u8>), RasterError> {
        let sizes = level_sizes(width, height);
        if sizes.len() <= PYRAMID_FIRST {
            return Err(RasterError::new(
                "Bu raster önizleme piramidi istemeyecek kadar küçük.",
            ));
        }
        let images = sizes[PYRAMID_FIRST..]
            .iter()
            .map(|&(w, h)| Image {
                width: w,
                height: h,
                bands,
                sample,
                alpha: false,
                geo: None,
                nodata,
            })
            .collect();
        let (writer, header) = Writer::new(images, 1)?;
        let n = sizes.len();
        let written = n - PYRAMID_FIRST;
        Ok((
            Builder {
                sizes,
                bands: bands as usize,
                sample,
                nodata,
                nearest,
                writer,
                held: vec![None; n],
                rows: vec![0; n],
                band: vec![Vec::new(); written],
                out: Vec::new(),
            },
            header,
        ))
    }

    /// How many level-0 rows have gone in.
    pub fn rows_done(&self) -> u32 {
        self.rows[0]
    }

    /// Takes the region's rows (the whole width of level 0, the next rows in
    /// order) and gives the bytes to append.
    pub fn push(&mut self, region: &Region) -> Result<Vec<u8>, RasterError> {
        let w = self.sizes[0].0 as usize;
        if region.width as usize != w || region.x != 0 || region.y != i64::from(self.rows[0]) {
            return Err(RasterError::new(
                "Önizleme piramidine satırlar sırasıyla ve bütün genişlikte verilmeli.",
            ));
        }
        let b = self.bands;
        for j in 0..region.height {
            let start = j as usize * w * b;
            let row: Vec<f64> = (0..w * b).map(|k| region.samples.get(start + k)).collect();
            self.level_row(0, row)?;
        }
        Ok(std::mem::take(&mut self.out))
    }

    /// A row of `level` arrived: keep it for the band, pair it for the level below.
    fn level_row(&mut self, level: usize, row: Vec<f64>) -> Result<(), RasterError> {
        let (lw, lh) = self.sizes[level];
        self.rows[level] += 1;
        let last = self.rows[level] == lh;
        if level >= PYRAMID_FIRST {
            let k = level - PYRAMID_FIRST;
            self.band[k].push(row.clone());
            if self.band[k].len() == TILE as usize || last {
                self.flush(level)?;
            }
        }
        if level + 1 < self.sizes.len() {
            match self.held[level].take() {
                Some(upper) => {
                    let next = self.halve(level, &upper, Some(&row), lw);
                    self.level_row(level + 1, next)?;
                }
                None if last => {
                    let next = self.halve(level, &row, None, lw);
                    self.level_row(level + 1, next)?;
                }
                None => self.held[level] = Some(row),
            }
        }
        Ok(())
    }

    /// The next level's row from two rows (the second may be missing at an odd end).
    fn halve(&self, level: usize, upper: &[f64], lower: Option<&[f64]>, width: u32) -> Vec<f64> {
        let nw = self.sizes[level + 1].0 as usize;
        let b = self.bands;
        let w = width as usize;
        let mut out = vec![0.0; nw * b];
        for i in 0..nw {
            for k in 0..b {
                let mut sum = 0.0;
                let mut n = 0u32;
                let cells = [
                    (2 * i, Some(upper)),
                    (2 * i + 1, Some(upper)),
                    (2 * i, lower),
                    (2 * i + 1, lower),
                ];
                for (x, r) in cells {
                    let Some(r) = r else {
                        continue;
                    };
                    if x >= w {
                        continue;
                    }
                    let v = r[x * b + k];
                    if v.is_nan() || self.nodata.is_some_and(|d| v == d) {
                        continue;
                    }
                    if self.nearest {
                        sum = v;
                        n = 1;
                        break;
                    }
                    sum += v;
                    n += 1;
                }
                // As the level's samples hold it, so the next level is made from what is
                // written (as `Reader`'s worked-out levels are).
                out[i * b + k] = if n > 0 {
                    stored(self.sample, sum / f64::from(n))
                } else {
                    self.nodata
                        .unwrap_or(if self.sample.float() { f64::NAN } else { 0.0 })
                };
            }
        }
        out
    }

    /// Writes a written level's band as tiles.
    fn flush(&mut self, level: usize) -> Result<(), RasterError> {
        let k = level - PYRAMID_FIRST;
        let rows = std::mem::take(&mut self.band[k]);
        let (lw, _) = self.sizes[level];
        let ty = (self.rows[level] - 1) / TILE;
        let b = self.bands;
        let fill = self
            .nodata
            .unwrap_or(if self.sample.float() { f64::NAN } else { 0.0 });
        for tx in 0..lw.div_ceil(TILE) {
            let mut s = Samples::filled(self.sample, (TILE * TILE) as usize * b, fill);
            for (j, row) in rows.iter().enumerate() {
                let x0 = (tx * TILE) as usize;
                let x1 = (x0 + TILE as usize).min(lw as usize);
                for x in x0..x1 {
                    for c in 0..b {
                        s.set((j * TILE as usize + (x - x0)) * b + c, row[x * b + c]);
                    }
                }
            }
            let bytes = self.writer.tile(k, tx, ty, &s)?;
            self.out.extend(bytes);
        }
        Ok(())
    }

    /// The directories to append and the header to write over the first one.
    pub fn finish(self) -> Result<(Vec<u8>, Vec<u8>), RasterError> {
        if self.rows[0] != self.sizes[0].1 {
            return Err(RasterError::new("Önizleme piramidi bitmeden bırakıldı."));
        }
        self.writer.finish()
    }
}
