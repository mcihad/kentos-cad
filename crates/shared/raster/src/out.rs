//! A result raster written as it is worked out (docs/adr/0231 §2): a tiled,
//! Deflate GeoTIFF holding its reduced levels too, so it draws at once with
//! no pyramid pass. Level 0 comes in 256-row strips, cut into tiles; every
//! level below takes the mean of 2 × 2 samples of the one above (nodata and
//! NaN left out, held as the sample type holds it: ADR 0204's reader's own
//! rule), kept a band of 256 rows at a time. Tiles are coded on the job's
//! threads and appended in order. Memory is a strip and a band a level,
//! whatever the size of the raster.

use kentos_contracts::RasterSample;
use kentos_formats::raster::TILE;
use kentos_formats::raster::samples::Samples;
use kentos_formats::raster::samples::stored;
use kentos_formats::raster::source::level_sizes;
use kentos_formats::raster::write::{Geo, Image, Writer, code};

use crate::par;

/// Deflate's level for results (1: fastest; docs/adr/0231 §11).
pub const DEFLATE_LEVEL: u8 = 1;

/// A result's samples, bands interleaved: 32-bit floats or bytes, or
/// any sample type in its own vector (docs/adr/0233 §2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rows<'a> {
    F32(&'a [f32]),
    U8(&'a [u8]),
    Any(&'a Samples),
}

impl Rows<'_> {
    fn len(&self) -> usize {
        match self {
            Rows::F32(r) => r.len(),
            Rows::U8(r) => r.len(),
            Rows::Any(r) => r.len(),
        }
    }

    fn kind(&self) -> RasterSample {
        match self {
            Rows::F32(_) => RasterSample::F32,
            Rows::U8(_) => RasterSample::U8,
            Rows::Any(r) => r.kind(),
        }
    }

    /// Samples `from..to` as float64s.
    fn floats(&self, from: usize, to: usize) -> Vec<f64> {
        match self {
            Rows::F32(s) => s[from..to].iter().map(|&v| f64::from(v)).collect(),
            Rows::U8(s) => s[from..to].iter().map(|&v| f64::from(v)).collect(),
            Rows::Any(s) => (from..to).map(|i| s.get(i)).collect(),
        }
    }
}

/// The result's shape.
#[derive(Clone, Debug, PartialEq)]
pub struct OutSpec {
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub sample: RasterSample,
    /// The last band is alpha.
    pub alpha: bool,
    pub nodata: Option<f64>,
    pub geo: Geo,
}

#[derive(Debug)]
struct Level {
    width: u32,
    height: u32,
    /// Rows this level has had.
    rows: u32,
    /// The row waiting for its pair (to make the next level's).
    held: Option<Vec<f64>>,
    /// The band of rows not yet written (`TILE` rows at most), in the result's type.
    band: Samples,
    band_rows: u32,
}

/// A tile to code: its level, place and sample bytes.
struct Pending {
    level: usize,
    tx: u32,
    ty: u32,
    bytes: Vec<u8>,
}

/// The writer's state.
#[derive(Debug)]
pub struct Out {
    spec: OutSpec,
    writer: Writer,
    levels: Vec<Level>,
    threads: usize,
}

/// An empty vector of `kind`.
fn empty(kind: RasterSample) -> Samples {
    Samples::filled(kind, 0, 0.0)
}

/// `row`'s values (as the type holds them already) appended to `buf`.
fn extend(buf: &mut Samples, row: &[f64]) {
    macro_rules! ext {
        ($v:expr, $t:ty) => {
            $v.extend(row.iter().map(|&x| x as $t))
        };
    }
    match buf {
        Samples::U8(v) => ext!(v, u8),
        Samples::I8(v) => ext!(v, i8),
        Samples::U16(v) => ext!(v, u16),
        Samples::I16(v) => ext!(v, i16),
        Samples::U32(v) => ext!(v, u32),
        Samples::I32(v) => ext!(v, i32),
        Samples::F32(v) => ext!(v, f32),
        Samples::F64(v) => v.extend_from_slice(row),
    }
}

impl Out {
    /// What the result is: its size, bands, samples and place.
    pub fn spec(&self) -> &OutSpec {
        &self.spec
    }

    /// A result of `spec`, its tiles coded on `threads`; the header to write first.
    pub fn new(spec: OutSpec, threads: usize) -> Result<(Out, Vec<u8>), String> {
        let sizes = level_sizes(spec.width, spec.height);
        let images = sizes
            .iter()
            .enumerate()
            .map(|(k, &(w, h))| Image {
                width: w,
                height: h,
                bands: spec.bands,
                sample: spec.sample,
                alpha: spec.alpha,
                geo: (k == 0).then(|| spec.geo.clone()),
                nodata: spec.nodata,
            })
            .collect();
        let (writer, header) = Writer::new(images, DEFLATE_LEVEL).map_err(|e| e.0)?;
        let levels = sizes
            .iter()
            .map(|&(w, h)| Level {
                width: w,
                height: h,
                rows: 0,
                held: None,
                band: empty(spec.sample),
                band_rows: 0,
            })
            .collect();
        Ok((
            Out {
                spec,
                writer,
                levels,
                threads: threads.max(1),
            },
            header,
        ))
    }

    /// How many level-0 rows have come in.
    pub fn rows_done(&self) -> u32 {
        self.levels[0].rows
    }

    /// Takes the next `n` rows of level 0 (whole width, bands interleaved;
    /// `n` is 256 but for the last strip) and gives the bytes to append.
    pub fn push(&mut self, rows: Rows<'_>, n: u32) -> Result<Vec<u8>, String> {
        let w = self.spec.width as usize;
        let b = self.spec.bands as usize;
        let want = n as usize * w * b;
        let first = self.levels[0].rows;
        if rows.len() != want
            || rows.kind() != self.spec.sample
            || !first.is_multiple_of(TILE)
            || first + n > self.spec.height
            || (n != TILE && first + n != self.spec.height)
        {
            return Err(
                "Sonuç rasterinin satırları sırasıyla ve 256'lık şeritlerle verilmeli.".into(),
            );
        }
        // Level 0: the strip is a band of tiles, made and coded on the threads.
        let ty = first / TILE;
        let fill = self.fill();
        let tiles: Vec<u32> = (0..w.div_ceil(TILE as usize) as u32).collect();
        let coded = par::map(self.threads, &tiles, &|&tx| {
            code(&tile_bytes(rows, w, n as usize, b, tx, fill), DEFLATE_LEVEL)
        });
        let mut out = Vec::new();
        for (&tx, c) in tiles.iter().zip(coded) {
            out.extend(self.writer.coded(0, tx, ty, c).map_err(|e| e.0)?);
        }
        self.levels[0].rows += n;
        // Level 1's rows from the strip's pairs of rows (a strip starts on an
        // even row; an odd last row stands alone), on the threads; the levels
        // below it row by row.
        let mut pending = Vec::new();
        if self.levels.len() > 1 {
            let pairs: Vec<usize> = (0..(n as usize).div_ceil(2)).collect();
            let this = &*self;
            let halves = par::map(self.threads, &pairs, &|&k| {
                let row = w * b;
                let upper = rows.floats(2 * k * row, (2 * k + 1) * row);
                let lower = (2 * k + 1 < n as usize)
                    .then(|| rows.floats((2 * k + 1) * row, (2 * k + 2) * row));
                this.halve(0, &upper, lower.as_deref())
            });
            for half in halves {
                self.level_row(1, half, &mut pending)?;
            }
        }
        out.extend(self.code(pending)?);
        Ok(out)
    }

    /// The value of a sample that is not there, as the type holds it.
    fn fill(&self) -> f64 {
        let f = self.spec.nodata.unwrap_or(if self.spec.sample.float() {
            f64::NAN
        } else {
            0.0
        });
        Samples::filled(self.spec.sample, 1, f).get(0)
    }

    /// Row `row` of `level` (its last when `last`) pairs with the one held:
    /// the next level's row is made of them.
    fn pair(
        &mut self,
        level: usize,
        row: Vec<f64>,
        last: bool,
        pending: &mut Vec<Pending>,
    ) -> Result<(), String> {
        if level + 1 >= self.levels.len() {
            return Ok(());
        }
        let upper = self.levels[level].held.take();
        let next = match upper {
            Some(upper) => Some(self.halve(level, &upper, Some(&row))),
            None if last => Some(self.halve(level, &row, None)),
            None => {
                self.levels[level].held = Some(row);
                None
            }
        };
        if let Some(next) = next {
            self.level_row(level + 1, next, pending)?;
        }
        Ok(())
    }

    /// A row of `level` ≥ 1 arrived: keep it in the band, pair it for the level below.
    fn level_row(
        &mut self,
        level: usize,
        row: Vec<f64>,
        pending: &mut Vec<Pending>,
    ) -> Result<(), String> {
        let lw = self.levels[level].width as usize;
        let b = self.spec.bands as usize;
        {
            let lv = &mut self.levels[level];
            extend(&mut lv.band, &row);
            lv.rows += 1;
            lv.band_rows += 1;
        }
        let (rows, height, band_rows) = {
            let lv = &self.levels[level];
            (lv.rows, lv.height, lv.band_rows)
        };
        let last = rows == height;
        if band_rows == TILE || last {
            let ty = (rows - 1) / TILE;
            let fill = self.fill();
            let kind = self.spec.sample;
            let lv = &mut self.levels[level];
            let band = std::mem::replace(&mut lv.band, empty(kind));
            lv.band_rows = 0;
            let n = band_rows as usize;
            for tx in 0..lw.div_ceil(TILE as usize) as u32 {
                let bytes = match &band {
                    Samples::F32(v) => tile_bytes(Rows::F32(v), lw, n, b, tx, fill),
                    Samples::U8(v) => tile_bytes(Rows::U8(v), lw, n, b, tx, fill),
                    other => tile_bytes(Rows::Any(other), lw, n, b, tx, fill),
                };
                pending.push(Pending {
                    level,
                    tx,
                    ty,
                    bytes,
                });
            }
        }
        self.pair(level, row, last, pending)
    }

    /// The next level's row from two rows of `level` (the second missing at an odd end).
    fn halve(&self, level: usize, upper: &[f64], lower: Option<&[f64]>) -> Vec<f64> {
        let w = self.levels[level].width as usize;
        let nw = self.levels[level + 1].width as usize;
        let b = self.spec.bands as usize;
        let nodata = self.spec.nodata;
        let float = self.spec.sample.float();
        let mut out = vec![0.0; nw * b];
        for i in 0..nw {
            for k in 0..b {
                let mut sum = 0.0;
                let mut n = 0u32;
                for (x, r) in [
                    (2 * i, Some(upper)),
                    (2 * i + 1, Some(upper)),
                    (2 * i, lower),
                    (2 * i + 1, lower),
                ] {
                    let Some(r) = r else {
                        continue;
                    };
                    if x >= w {
                        continue;
                    }
                    let v = r[x * b + k];
                    if v.is_nan() || nodata.is_some_and(|d| v == d) {
                        continue;
                    }
                    sum += v;
                    n += 1;
                }
                out[i * b + k] = if n > 0 {
                    stored(self.spec.sample, sum / f64::from(n))
                } else {
                    Samples::filled(
                        self.spec.sample,
                        1,
                        nodata.unwrap_or(if float { f64::NAN } else { 0.0 }),
                    )
                    .get(0)
                };
            }
        }
        out
    }

    /// Codes the tiles on the threads and gives their bytes in order.
    fn code(&mut self, pending: Vec<Pending>) -> Result<Vec<u8>, String> {
        let coded = par::map(self.threads, &pending, &|p: &Pending| {
            code(&p.bytes, DEFLATE_LEVEL)
        });
        let mut out = Vec::new();
        for (p, c) in pending.iter().zip(coded) {
            out.extend(self.writer.coded(p.level, p.tx, p.ty, c).map_err(|e| e.0)?);
        }
        Ok(out)
    }

    /// The directories to append last and the header to write over the first one.
    pub fn finish(self) -> Result<(Vec<u8>, Vec<u8>), String> {
        if self.levels.iter().any(|l| l.rows != l.height) {
            return Err("Sonuç rasteri bitmeden bırakıldı.".into());
        }
        self.writer.finish().map_err(|e| e.0)
    }
}

/// A sample's little-endian bytes.
trait Le: Copy {
    fn put(self, out: &mut Vec<u8>);
}

macro_rules! le {
    ($($t:ty),*) => {
        $(impl Le for $t {
            #[inline]
            fn put(self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
        })*
    };
}
le!(u8, i8, u16, i16, u32, i32, f32, f64);

/// Tile `tx`'s samples (`TILE` × `TILE`, bands interleaved) from `n` rows
/// of width `w`, the part past the rows `fill`.
fn tile_of<T: Le>(s: &[T], w: usize, n: usize, b: usize, tx: u32, fill: T) -> Vec<u8> {
    let t = TILE as usize;
    let x0 = tx as usize * t;
    let x1 = (x0 + t).min(w);
    let mut out = Vec::with_capacity(t * t * b * std::mem::size_of::<T>());
    for j in 0..t {
        for x in x0..x0 + t {
            for k in 0..b {
                if j < n && x < x1 {
                    s[(j * w + x) * b + k].put(&mut out);
                } else {
                    fill.put(&mut out);
                }
            }
        }
    }
    out
}

/// Tile `tx`'s little-endian sample bytes from `n` rows of width `w`, the
/// part past the rows `fill` (a value its type holds).
fn tile_bytes(rows: Rows<'_>, w: usize, n: usize, b: usize, tx: u32, fill: f64) -> Vec<u8> {
    match rows {
        Rows::F32(s) => tile_of(s, w, n, b, tx, fill as f32),
        Rows::U8(s) => {
            // Bytes: whole runs copied.
            let t = TILE as usize;
            let x0 = tx as usize * t;
            let x1 = (x0 + t).min(w);
            let mut out = vec![fill as u8; t * t * b];
            for j in 0..n.min(t) {
                let src = &s[(j * w + x0) * b..(j * w + x1) * b];
                out[j * t * b..j * t * b + src.len()].copy_from_slice(src);
            }
            out
        }
        Rows::Any(s) => match s {
            Samples::U8(v) => tile_bytes(Rows::U8(v), w, n, b, tx, fill),
            Samples::I8(v) => tile_of(v, w, n, b, tx, fill as i8),
            Samples::U16(v) => tile_of(v, w, n, b, tx, fill as u16),
            Samples::I16(v) => tile_of(v, w, n, b, tx, fill as i16),
            Samples::U32(v) => tile_of(v, w, n, b, tx, fill as u32),
            Samples::I32(v) => tile_of(v, w, n, b, tx, fill as i32),
            Samples::F32(v) => tile_of(v, w, n, b, tx, fill as f32),
            Samples::F64(v) => tile_of(v, w, n, b, tx, fill),
        },
    }
}
