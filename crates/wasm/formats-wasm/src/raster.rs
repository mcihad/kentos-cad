//! Rasters in the browser (docs/adr/0204): the web's raster workers
//! (`apps/web/src/io/rasterWorker.ts`) read a file's pieces as its reader
//! asks for them (`File.slice`), hand them over, decode a JPEG block with the
//! browser's decoder and get a tile's colours back; the same reader, look,
//! pyramid pass and resampling as the desktop's (`kentos_formats::raster`).
//! Numbers cross as typed arrays, a tile's colours as bytes; what a window
//! shows as JSON.

use kentos_contracts::RasterStyle;
use kentos_formats::raster::pyramid::Builder;
use kentos_formats::raster::source::{BlockNeed, Put, Reader};
use kentos_formats::raster::stats::Stats;
use kentos_formats::raster::warp::{self, Grid, Job};
use kentos_formats::raster::{ByteStore, Samples, Step, place, style, tiff, world};
use kentos_geometry_core::ops::georef::{Gcp, Method, solve};
use kentos_geometry_core::vec2::Vec2;
use wasm_bindgen::prelude::*;

fn fail(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

/// A file's world file text read into its affine, when given.
fn world_of(text: Option<String>) -> Result<Option<[f64; 6]>, JsError> {
    text.map(|t| world::read(&t)).transpose().map_err(fail)
}

/// A TIFF's header read in the pieces it asks for (a pyramid file's too).
#[wasm_bindgen]
pub struct RasterOpening {
    store: ByteStore,
    size: u64,
    taken: u64,
    parsed: Option<tiff::Tiff>,
}

/// The most bytes a header may take.
const HEADER_MOST: u64 = 64 * 1024 * 1024;

#[wasm_bindgen]
impl RasterOpening {
    /// A TIFF of `size` bytes about to be read.
    #[wasm_bindgen(constructor)]
    pub fn new(size: f64) -> RasterOpening {
        RasterOpening {
            store: ByteStore::new(),
            size: size.max(0.0) as u64,
            taken: 0,
            parsed: None,
        }
    }

    /// Whether a file's first bytes are a TIFF's.
    #[wasm_bindgen(js_name = isTiff)]
    pub fn is_tiff(head: &[u8]) -> bool {
        tiff::sniff(head)
    }

    /// The next run the header needs, `[offset, length]`; empty once it has all.
    pub fn need(&mut self) -> Result<Vec<f64>, JsError> {
        if self.parsed.is_some() {
            return Ok(Vec::new());
        }
        match tiff::parse(&self.store, self.size).map_err(fail)? {
            Step::Done(t) => {
                self.parsed = Some(t);
                Ok(Vec::new())
            }
            Step::Need(n) => {
                self.taken += n.len;
                if self.taken > HEADER_MOST {
                    return Err(JsError::new("TIFF'in başlığı çok büyük."));
                }
                Ok(vec![n.offset as f64, n.len as f64])
            }
        }
    }

    /// The bytes that were at `offset`.
    pub fn put(&mut self, offset: f64, bytes: Vec<u8>) {
        self.store.put(offset.max(0.0) as u64, bytes);
    }

    /// The raster the header opens, placed by its world file's text when its file does not place it.
    pub fn open(&self, world: Option<String>, budget: usize) -> Result<RasterFile, JsError> {
        let t = self
            .parsed
            .as_ref()
            .ok_or_else(|| JsError::new("TIFF'in başlığı okunmadı."))?;
        let reader = Reader::tiff(t, world_of(world)?, budget).map_err(fail)?;
        Ok(RasterFile::of(reader))
    }

    /// The header read as a pyramid file of `raster` (its levels then come from it).
    #[wasm_bindgen(js_name = attachTo)]
    pub fn attach_to(&self, raster: &mut RasterFile) -> Result<(), JsError> {
        let t = self
            .parsed
            .as_ref()
            .ok_or_else(|| JsError::new("Piramidin başlığı okunmadı."))?;
        raster.reader.attach_pyramid(t).map_err(fail)
    }
}

/// A raster being read: its reader, the blocks it asked for last, its
/// statistics once worked out, a pyramid pass and a resampling under way.
#[wasm_bindgen]
pub struct RasterFile {
    reader: Reader,
    needs: Vec<BlockNeed>,
    stats: Option<Stats>,
    pyramid: Option<Builder>,
    pyramid_head: Vec<u8>,
    warp: Option<Job>,
    warp_head: Vec<u8>,
}

impl RasterFile {
    fn of(reader: Reader) -> RasterFile {
        RasterFile {
            reader,
            needs: Vec::new(),
            stats: None,
            pyramid: None,
            pyramid_head: Vec::new(),
            warp: None,
            warp_head: Vec::new(),
        }
    }
}

#[wasm_bindgen]
impl RasterFile {
    /// A PNG's raster (decoded here), placed by its world file's text.
    #[wasm_bindgen(js_name = fromPng)]
    pub fn from_png(
        bytes: &[u8],
        world: Option<String>,
        budget: usize,
    ) -> Result<RasterFile, JsError> {
        let p = kentos_formats::raster::png::decode(bytes).map_err(fail)?;
        let reader = Reader::image(
            p.width,
            p.height,
            p.bands,
            p.samples,
            p.nodata,
            world_of(world)?,
            budget,
        )
        .map_err(fail)?;
        Ok(RasterFile::of(reader))
    }

    /// A JPEG's raster from the browser's decoded pixels (`components` a pixel; RGBA taken as RGB).
    #[wasm_bindgen(js_name = fromPixels)]
    pub fn from_pixels(
        width: u32,
        height: u32,
        components: u32,
        pixels: Vec<u8>,
        world: Option<String>,
        budget: usize,
    ) -> Result<RasterFile, JsError> {
        let (bands, samples) = match components {
            4 => (
                3,
                pixels
                    .chunks_exact(4)
                    .flat_map(|c| [c[0], c[1], c[2]])
                    .collect(),
            ),
            c => (c, pixels),
        };
        let reader = Reader::image(
            width,
            height,
            bands,
            Samples::U8(samples),
            None,
            world_of(world)?,
            budget,
        )
        .map_err(fail)?;
        Ok(RasterFile::of(reader))
    }

    /// What Raster ekle shows (`RasterInfo` JSON).
    pub fn info(&self) -> String {
        serde_json::to_string(&self.reader.info).unwrap_or_default()
    }

    /// The look a raster of its kind starts with (`RasterStyle` JSON; docs/adr/0204 §4).
    #[wasm_bindgen(js_name = defaultStyle)]
    pub fn default_style(&self) -> String {
        let i = &self.reader.info;
        serde_json::to_string(&style::default_style(
            i.bands,
            i.sample,
            self.reader.palette.is_some(),
        ))
        .unwrap_or_default()
    }

    /// Raster ekle's rule (`place::Rule` JSON) in a project of `project_srid`,
    /// with where it goes (`affine`, `srid`) when it goes (`confirmed`: an
    /// unknown system said to be the project's; `view`: min x, min y, max x, max y).
    pub fn placement(&self, project_srid: u32, confirmed: bool, view: &[f64]) -> String {
        let info = &self.reader.info;
        let view = [0, 1, 2, 3].map(|k| view.get(k).copied().unwrap_or(0.0));
        let rule = serde_json::to_value(place::rule(info, project_srid)).unwrap_or_default();
        let mut out = rule.as_object().cloned().unwrap_or_default();
        if let Some((affine, srid)) = place::placement(info, project_srid, confirmed, view) {
            out.insert("affine".into(), serde_json::json!(affine));
            out.insert("srid".into(), serde_json::json!(srid));
        }
        serde_json::Value::Object(out).to_string()
    }

    /// The blocks a region of `level` wants that are not kept, three numbers
    /// each: the file (0 the raster's, 1 its pyramid's), the offset and the
    /// length; `putBlock` takes them by their index here.
    pub fn needs(&mut self, level: u32, x: f64, y: f64, w: u32, h: u32) -> Vec<f64> {
        self.needs = self.reader.needs(level as usize, x as i64, y as i64, w, h);
        self.needs
            .iter()
            .flat_map(|n| [f64::from(n.file), n.offset as f64, n.len as f64])
            .collect()
    }

    /// Need `i`'s bytes: decoded and kept; a JPEG block's stream comes back
    /// for the browser to decode (`putPixels`), else nothing.
    #[wasm_bindgen(js_name = putBlock)]
    pub fn put_block(&mut self, i: usize, bytes: &[u8]) -> Result<Vec<u8>, JsError> {
        let need = *self
            .needs
            .get(i)
            .ok_or_else(|| JsError::new("Böyle bir blok istenmedi."))?;
        match self.reader.put_block(&need, bytes).map_err(fail)? {
            Put::Done => Ok(Vec::new()),
            Put::Jpeg(stream) => Ok(stream),
        }
    }

    /// A JPEG block's pixels (`components` a pixel) for need `i`.
    #[wasm_bindgen(js_name = putPixels)]
    pub fn put_pixels(
        &mut self,
        i: usize,
        pixels: Vec<u8>,
        components: u32,
    ) -> Result<(), JsError> {
        let need = *self
            .needs
            .get(i)
            .ok_or_else(|| JsError::new("Böyle bir blok istenmedi."))?;
        self.reader
            .put_pixels(&need, pixels, components)
            .map_err(fail)
    }

    /// The level the statistics are taken from and its size: `[level, width, height]`.
    #[wasm_bindgen(js_name = statsRegion)]
    pub fn stats_region(&self) -> Vec<u32> {
        let level = self.reader.stats_level();
        let (w, h) = self
            .reader
            .levels
            .get(level)
            .map_or((0, 0), |l| (l.width, l.height));
        vec![level as u32, w, h]
    }

    /// The bands' statistics worked out (the statistics level's blocks kept)
    /// and kept for the looks that stretch by them, as JSON for Raster
    /// stili; none while a block is missing.
    #[wasm_bindgen(js_name = takeStats)]
    pub fn take_stats(&mut self) -> Option<String> {
        if self.stats.is_none() {
            self.stats = Some(self.reader.stats()?);
        }
        self.stats.as_ref().map(stats_json)
    }

    /// Tile (`tx`, `ty`) of `level` coloured by `look` (`RasterStyle` JSON)
    /// for a raster placed by `affine`: 258 × 258 premultiplied RGBA; empty
    /// while a block it wants is not kept.
    #[wasm_bindgen(js_name = renderTile)]
    pub fn render_tile(
        &mut self,
        level: u32,
        tx: u32,
        ty: u32,
        look: &str,
        affine: &[f64],
    ) -> Result<Vec<u8>, JsError> {
        let style = RasterStyle::from_json_text(look)
            .ok_or_else(|| JsError::new("Rasterin görünüşü okunamadı."))?;
        let affine = [0, 1, 2, 3, 4, 5].map(|k| affine.get(k).copied().unwrap_or(0.0));
        let mut out = Vec::new();
        if self.reader.render_tile(
            level as usize,
            tx,
            ty,
            &style,
            self.stats.as_ref(),
            &affine,
            &mut out,
        ) {
            Ok(out)
        } else {
            Ok(Vec::new())
        }
    }

    /// The bands' values at pixel (`i`, `j`) of level 0 (its block kept); none outside or while missing.
    pub fn sample(&mut self, i: f64, j: f64) -> Option<Vec<f64>> {
        self.reader.sample(i.floor() as i64, j.floor() as i64)
    }

    /// Whether a pyramid file is to be made (a large raster without overviews).
    #[wasm_bindgen(js_name = needsPyramid)]
    pub fn needs_pyramid(&self) -> bool {
        self.reader.info.needs_pyramid
    }

    /// Starts the pyramid pass: the header to write first.
    #[wasm_bindgen(js_name = pyramidStart)]
    pub fn pyramid_start(&mut self) -> Result<Vec<u8>, JsError> {
        let i = &self.reader.info;
        let (builder, header) = Builder::new(
            i.width,
            i.height,
            i.bands,
            i.sample,
            self.reader.nodata,
            self.reader.palette.is_some(),
        )
        .map_err(fail)?;
        self.pyramid = Some(builder);
        Ok(header)
    }

    /// Rows `y` to `y + n` of level 0 (their blocks kept) into the pass: the bytes to append.
    #[wasm_bindgen(js_name = pyramidPush)]
    pub fn pyramid_push(&mut self, y: u32, n: u32) -> Result<Vec<u8>, JsError> {
        let w = self.reader.info.width;
        let region = self
            .reader
            .region(0, 0, i64::from(y), w, n)
            .ok_or_else(|| JsError::new("Rasterin satırları okunamadı."))?;
        let builder = self
            .pyramid
            .as_mut()
            .ok_or_else(|| JsError::new("Piramit başlamadı."))?;
        builder.push(&region).map_err(fail)
    }

    /// Ends the pass: the directories to append (`pyramidHead` the header to write over the first one).
    #[wasm_bindgen(js_name = pyramidFinish)]
    pub fn pyramid_finish(&mut self) -> Result<Vec<u8>, JsError> {
        let builder = self
            .pyramid
            .take()
            .ok_or_else(|| JsError::new("Piramit başlamadı."))?;
        let (dirs, head) = builder.finish().map_err(fail)?;
        self.pyramid_head = head;
        Ok(dirs)
    }

    #[wasm_bindgen(js_name = pyramidHead)]
    pub fn pyramid_head(&self) -> Vec<u8> {
        self.pyramid_head.clone()
    }

    /// Raster oturt's resampling (docs/adr/0204 §6): `points` five numbers
    /// each (column, row, x, y, used 0 or 1), `method` the transform's name,
    /// `pixel` the output's (0: the transform's mean scale), sampled
    /// nearest or bilinear; the output's system `epsg` (0: none). The header
    /// to write first; why not.
    #[wasm_bindgen(js_name = warpStart)]
    pub fn warp_start(
        &mut self,
        points: &[f64],
        method: &str,
        pixel: f64,
        nearest: bool,
        epsg: u32,
    ) -> Result<Vec<u8>, JsError> {
        let gcps: Vec<Gcp> = points
            .chunks_exact(5)
            .map(|p| Gcp {
                pixel: Vec2::new(p[0], p[1]),
                target: Vec2::new(p[2], p[3]),
                used: p[4] != 0.0,
            })
            .collect();
        let method =
            Method::from_name(method).ok_or_else(|| JsError::new("Bilinmeyen dönüşüm."))?;
        let g = solve(&gcps, method).map_err(|e| JsError::new(e.code()))?;
        let i = &self.reader.info;
        let forward = |p: Vec2| g.forward(p);
        let used: Vec<Vec2> = gcps.iter().filter(|p| p.used).map(|p| p.pixel).collect();
        let size = if pixel > 0.0 {
            Some(pixel)
        } else {
            warp::default_pixel(&forward, &used)
        };
        let size = size.ok_or_else(|| JsError::new("Çıktının piksel boyu bulunamadı."))?;
        let edge: Vec<Vec2> = warp::border(i.width, i.height, 64)
            .into_iter()
            .filter_map(forward)
            .collect();
        let grid = Grid::covering(&edge, size).ok_or_else(|| {
            JsError::new(
                "Çıktının ızgarası kurulamadı: dönüşüm rasteri çok büyütüyor ya da bozuyor.",
            )
        })?;
        let (w, h, bands, sample) = (i.width, i.height, i.bands, i.sample);
        let (nodata, palette) = (self.reader.nodata, self.reader.palette.clone());
        let inverse = Box::new(move |p: Vec2| g.inverse(p));
        let (job, header) = Job::new(
            grid,
            inverse,
            w,
            h,
            bands,
            sample,
            nodata,
            palette,
            nearest,
            (epsg > 0).then_some(epsg),
            false,
        )
        .map_err(fail)?;
        self.warp = Some(job);
        Ok(header)
    }

    /// The output's grid and kind: `[x₀, s, 0, y₀, 0, −s, width, height, bands, alpha (0, 1)]`.
    #[wasm_bindgen(js_name = warpGrid)]
    pub fn warp_grid(&self) -> Vec<f64> {
        let Some(j) = &self.warp else {
            return Vec::new();
        };
        let mut out = j.grid.affine.to_vec();
        out.extend([
            f64::from(j.grid.width),
            f64::from(j.grid.height),
            f64::from(j.bands),
            if j.alpha { 1.0 } else { 0.0 },
        ]);
        out
    }

    /// The source region the next tile reads: `[x, y, width, height]`, `[]`
    /// when it reads none, `[-1]` when every tile is written.
    #[wasm_bindgen(js_name = warpRegion)]
    pub fn warp_region(&mut self) -> Vec<f64> {
        match self.warp.as_mut().and_then(Job::region) {
            None => vec![-1.0],
            Some(None) => Vec::new(),
            Some(Some((x, y, w, h))) => vec![x as f64, y as f64, f64::from(w), f64::from(h)],
        }
    }

    /// The next tile's bytes to append (its region's blocks kept).
    #[wasm_bindgen(js_name = warpTile)]
    pub fn warp_tile(&mut self) -> Result<Vec<u8>, JsError> {
        let job = self
            .warp
            .as_mut()
            .ok_or_else(|| JsError::new("Yeniden örnekleme başlamadı."))?;
        let region = match job.region() {
            Some(Some((x, y, w, h))) => Some(
                self.reader
                    .region(0, x, y, w, h)
                    .ok_or_else(|| JsError::new("Kaynağın blokları okunmadı."))?,
            ),
            _ => None,
        };
        job.tile(region.as_ref()).map_err(fail)
    }

    /// The share done, 0 to 1.
    #[wasm_bindgen(js_name = warpShare)]
    pub fn warp_share(&self) -> f64 {
        self.warp.as_ref().map_or(0.0, Job::share)
    }

    /// Ends the resampling: the directories to append (`warpHead` the header to write over the first one).
    #[wasm_bindgen(js_name = warpFinish)]
    pub fn warp_finish(&mut self) -> Result<Vec<u8>, JsError> {
        let job = self
            .warp
            .take()
            .ok_or_else(|| JsError::new("Yeniden örnekleme başlamadı."))?;
        let (dirs, head) = job.finish().map_err(fail)?;
        self.warp_head = head;
        Ok(dirs)
    }

    #[wasm_bindgen(js_name = warpHead)]
    pub fn warp_head(&self) -> Vec<u8> {
        self.warp_head.clone()
    }
}

/// The statistics as Raster stili shows them: each band's least, greatest, 2 % and 98 %.
fn stats_json(s: &Stats) -> String {
    let bands: Vec<serde_json::Value> = (0..s.bands.len())
        .map(|i| {
            let (min, max) = s.minmax(i).unwrap_or((f64::NAN, f64::NAN));
            let (lo, hi) = s.percent(i).unwrap_or((f64::NAN, f64::NAN));
            let n = |v: f64| {
                if v.is_finite() {
                    serde_json::json!(v)
                } else {
                    serde_json::Value::Null
                }
            };
            serde_json::json!({ "min": n(min), "max": n(max), "low": n(lo), "high": n(hi) })
        })
        .collect();
    serde_json::Value::Array(bands).to_string()
}

/// The text of a world file for `affine` (`[x₀, a, b, y₀, c, d]`, pixel corners).
#[wasm_bindgen(js_name = rasterWorldFile)]
pub fn raster_world_file(affine: &[f64]) -> String {
    let affine = [0, 1, 2, 3, 4, 5].map(|k| affine.get(k).copied().unwrap_or(0.0));
    world::write(&affine)
}

/// The world file extensions that may sit beside a file of `extension`, the likeliest first.
#[wasm_bindgen(js_name = rasterWorldExtensions)]
pub fn raster_world_extensions(extension: &str) -> Vec<String> {
    world::extensions(extension)
}

/// The world file extension written beside a file of `extension`.
#[wasm_bindgen(js_name = rasterWorldExtension)]
pub fn raster_world_extension(extension: &str) -> String {
    world::extension_for(extension)
}
