//! Raster analysis in the browser (docs/adr/0231 §2, §11): the raster core's
//! jobs behind wasm-bindgen, a package of its own that the analysis worker
//! (`apps/web/src/io/rasterAnalysisWorker.ts`) loads when a job starts
//! (CLAUDE.md §20). The worker reads the raster's file by its slices: a
//! TIFF's header in the pieces [`AnalysisOpening`] asks for, a PNG decoded
//! here, a JPEG by the browser; then, a strip at a time, the blocks
//! [`Analysis::needs`] names, handing back the bytes [`Analysis::step`]
//! gives. A raster result ends as the GeoTIFF's directories and its header;
//! lines as typed arrays and their Kot texts. Numbers cross as typed arrays,
//! settings and looks as JSON; a refusal throws an `Error` whose message is
//! the core's Turkish words.
//!
//! Raster işlemleri (docs/adr/0233) are [`OpsAnalysis`]: its inputs opened
//! one after another into an [`OpsOpening`] (a TIFF by its header's pieces,
//! a PNG here, a JPEG's pixels), the areas of a mask or the zones as JSON;
//! then stepped as a raster analysis is, each block asked for with its
//! input; a table's figures come back as typed arrays and JSON.
//!
//! A raster made from points or lines (docs/adr/0232) is [`PointAnalysis`]:
//! the objects as JSON (a point's, a line's, a path's places and elevations;
//! for Çizgi yoğunluğu the objects' geometry), each object's value or weight
//! text, the settings; it is stepped as a raster analysis is, and gives its
//! notes and the cross-validation's rows besides.

use kentos_contracts::RasterSample;
use kentos_formats::raster::source::{BlockNeed, Put, Reader};
use kentos_formats::raster::{ByteStore, Samples, Step, tiff};
use kentos_raster::contours::Line;
use kentos_raster::job::{Finished, Job, READER_BUDGET, Spec};
use wasm_bindgen::prelude::*;

fn fail(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

/// The most bytes a TIFF's header may take.
const HEADER_MOST: u64 = 64 * 1024 * 1024;

/// A TIFF's header read in the pieces it asks for.
#[wasm_bindgen]
pub struct AnalysisOpening {
    store: ByteStore,
    size: u64,
    taken: u64,
    parsed: Option<tiff::Tiff>,
}

#[wasm_bindgen]
impl AnalysisOpening {
    /// A TIFF of `size` bytes about to be read.
    #[wasm_bindgen(constructor)]
    pub fn new(size: f64) -> AnalysisOpening {
        AnalysisOpening {
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

    /// The analysis of `spec` (`kentos_raster::job::Spec` JSON) over the TIFF the header opens.
    pub fn analysis(&self, spec: &str) -> Result<Analysis, JsError> {
        Analysis::of(self.reader()?, spec)
    }
}

impl AnalysisOpening {
    /// The reader of the TIFF the header opens.
    fn reader(&self) -> Result<Reader, JsError> {
        let t = self
            .parsed
            .as_ref()
            .ok_or_else(|| JsError::new("TIFF'in başlığı okunmadı."))?;
        Reader::tiff(t, None, READER_BUDGET).map_err(fail)
    }
}

/// A PNG's raster (decoded here) as a reader.
fn png_reader(bytes: &[u8]) -> Result<Reader, JsError> {
    let p = kentos_formats::raster::png::decode(bytes).map_err(fail)?;
    Reader::image(
        p.width,
        p.height,
        p.bands,
        p.samples,
        p.nodata,
        None,
        READER_BUDGET,
    )
    .map_err(fail)
}

/// A JPEG's raster from the browser's decoded pixels (`components` a pixel; RGBA taken as RGB).
fn pixels_reader(
    width: u32,
    height: u32,
    components: u32,
    pixels: Vec<u8>,
) -> Result<Reader, JsError> {
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
    Reader::image(
        width,
        height,
        bands,
        Samples::U8(samples),
        None,
        None,
        READER_BUDGET,
    )
    .map_err(fail)
}

/// An analysis under way: its job, the blocks it asked for last, and once
/// done, the GeoTIFF's header or the lines.
#[wasm_bindgen]
pub struct Analysis {
    job: Option<Job>,
    header: Vec<u8>,
    needs: Vec<BlockNeed>,
    result: Option<(u32, RasterSample)>,
    style: String,
    levels: Option<kentos_raster::contours::Spec>,
    head: Vec<u8>,
    lines: Vec<Line>,
}

impl Analysis {
    fn of(reader: Reader, spec: &str) -> Result<Analysis, JsError> {
        let spec: Spec = serde_json::from_str(spec)
            .map_err(|e| JsError::new(&format!("Çözümlemenin ayarları okunamadı: {e}")))?;
        let (job, header) = Job::new(reader, &spec, 1).map_err(fail)?;
        Ok(Analysis {
            result: job.result(),
            style: job
                .style()
                .map(|s| serde_json::to_string(&s).unwrap_or_default())
                .unwrap_or_default(),
            levels: job.contours().copied(),
            job: Some(job),
            header,
            needs: Vec::new(),
            head: Vec::new(),
            lines: Vec::new(),
        })
    }

    fn job(&mut self) -> Result<&mut Job, JsError> {
        self.job
            .as_mut()
            .ok_or_else(|| JsError::new("Çözümleme bitti."))
    }
}

#[wasm_bindgen]
impl Analysis {
    /// The analysis of `spec` over a PNG's raster (decoded here).
    #[wasm_bindgen(js_name = fromPng)]
    pub fn from_png(bytes: &[u8], spec: &str) -> Result<Analysis, JsError> {
        Analysis::of(png_reader(bytes)?, spec)
    }

    /// The analysis of `spec` over a JPEG's raster from the browser's decoded
    /// pixels (`components` a pixel; RGBA taken as RGB).
    #[wasm_bindgen(js_name = fromPixels)]
    pub fn from_pixels(
        width: u32,
        height: u32,
        components: u32,
        pixels: Vec<u8>,
        spec: &str,
    ) -> Result<Analysis, JsError> {
        Analysis::of(pixels_reader(width, height, components, pixels)?, spec)
    }

    /// The bytes to write first: a raster result's header (written over at
    /// the end by [`Analysis::head`]); empty for lines.
    pub fn header(&self) -> Vec<u8> {
        self.header.clone()
    }

    /// The blocks the next strip wants that are not kept, `[offset, length]`
    /// each; `putBlock` takes them by their index here.
    pub fn needs(&mut self) -> Result<Vec<f64>, JsError> {
        self.needs = self.job()?.needs();
        Ok(self
            .needs
            .iter()
            .flat_map(|n| [n.offset as f64, n.len as f64])
            .collect())
    }

    /// Need `i`'s bytes: decoded and kept; a JPEG block's stream comes back
    /// for the browser to decode (`putPixels`), else nothing.
    #[wasm_bindgen(js_name = putBlock)]
    pub fn put_block(&mut self, i: usize, bytes: &[u8]) -> Result<Vec<u8>, JsError> {
        let need = *self
            .needs
            .get(i)
            .ok_or_else(|| JsError::new("Böyle bir blok istenmedi."))?;
        match self.job()?.put(&need, bytes).map_err(fail)? {
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
        self.job()?
            .put_pixels(&need, pixels, components)
            .map_err(fail)
    }

    /// Works out the next strip (its blocks kept): the bytes to append.
    pub fn step(&mut self) -> Result<Vec<u8>, JsError> {
        self.job()?.step().map_err(fail)
    }

    /// Whether every strip is done.
    pub fn done(&self) -> bool {
        self.job.as_ref().is_none_or(Job::done)
    }

    /// The share done, 0 to 1.
    pub fn share(&self) -> f64 {
        self.job.as_ref().map_or(1.0, Job::share)
    }

    /// Ends the job: a raster result's directories to append (its header
    /// then from [`Analysis::head`]); nothing for lines (then read by `line…`).
    pub fn finish(&mut self) -> Result<Vec<u8>, JsError> {
        let job = self
            .job
            .take()
            .ok_or_else(|| JsError::new("Çözümleme bitti."))?;
        match job.finish().map_err(fail)? {
            Finished::Raster { tail, header } => {
                self.head = header;
                Ok(tail)
            }
            Finished::Lines(lines) => {
                self.lines = lines;
                Ok(Vec::new())
            }
        }
    }

    /// The header to write over the result's first bytes.
    pub fn head(&self) -> Vec<u8> {
        self.head.clone()
    }

    /// A raster result's bands (0 for lines).
    pub fn bands(&self) -> u32 {
        self.result.map_or(0, |r| r.0)
    }

    /// A raster result's samples, as the contract names them (`f32`, `u8`); empty for lines.
    pub fn sample(&self) -> String {
        self.result.map_or_else(String::new, |r| {
            serde_json::to_value(r.1)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default()
        })
    }

    /// A raster result's look (`RasterStyle` JSON, docs/adr/0231 §10); empty for lines.
    pub fn style(&self) -> String {
        self.style.clone()
    }

    /// The lines' levels (each line's value).
    #[wasm_bindgen(js_name = lineValues)]
    pub fn line_values(&self) -> Vec<f64> {
        self.lines.iter().map(|l| l.value).collect()
    }

    /// Whether each line is an Ana eğri (1) or an Ara eğri (0).
    #[wasm_bindgen(js_name = lineMain)]
    pub fn line_main(&self) -> Vec<u8> {
        self.lines.iter().map(|l| u8::from(l.index)).collect()
    }

    /// Each line's number of points.
    #[wasm_bindgen(js_name = lineSizes)]
    pub fn line_sizes(&self) -> Vec<u32> {
        self.lines
            .iter()
            .map(|l| u32::try_from(l.pts.len()).unwrap_or(u32::MAX))
            .collect()
    }

    /// The lines' points one after another, x and y each.
    #[wasm_bindgen(js_name = linePoints)]
    pub fn line_points(&self) -> Vec<f64> {
        self.lines
            .iter()
            .flat_map(|l| l.pts.iter().flat_map(|p| [p[0], p[1]]))
            .collect()
    }

    /// Each line's Kot as the drawing writes it (docs/adr/0231 §9).
    #[wasm_bindgen(js_name = lineTexts)]
    pub fn line_texts(&self) -> Vec<String> {
        match &self.levels {
            Some(s) => self.lines.iter().map(|l| s.level_text(l.value)).collect(),
            None => Vec::new(),
        }
    }
}

/// A raster from points or lines under way (docs/adr/0232).
#[wasm_bindgen]
pub struct PointAnalysis {
    job: Option<kentos_raster::PointJob>,
    header: Vec<u8>,
    grid: Vec<f64>,
    bands: u32,
    sample: String,
    styles: [String; 2],
    head: Vec<u8>,
    notes: String,
    rows: Vec<kentos_raster::from_points::CrossRow>,
}

#[wasm_bindgen]
impl PointAnalysis {
    /// `objects`: a JSON array of the objects (as the drawing holds them);
    /// `values`: a JSON array of each one's text (or null), or `null`;
    /// `spec`: `kentos_raster::from_points::PointSpec` JSON; `lines`: Çizgi yoğunluğu's input.
    #[wasm_bindgen(constructor)]
    pub fn new(
        objects: &str,
        values: &str,
        spec: &str,
        lines: bool,
    ) -> Result<PointAnalysis, JsError> {
        use kentos_geometry_core::api::json::{FromJson, Json};
        use kentos_raster::PointInput;
        let values: Option<Vec<Option<String>>> = serde_json::from_str(values)
            .map_err(|e| JsError::new(&format!("Değerler okunamadı: {e}")))?;
        let input = if lines {
            let j = Json::parse(objects)
                .map_err(|e| JsError::new(&format!("Nesneler okunamadı: {e}")))?;
            let Json::Arr(list) = j else {
                return Err(JsError::new("Nesneler bir liste olmalı."));
            };
            let shapes = list
                .iter()
                .map(|o| kentos_geometry_core::entity::Entity::from_json(o).map(|e| e.shape))
                .collect::<Result<Vec<_>, String>>()
                .map_err(fail)?;
            PointInput::Lines {
                shapes,
                weights: values,
            }
        } else {
            let sources: Vec<kentos_raster::points::Source> = serde_json::from_str(objects)
                .map_err(|e| JsError::new(&format!("Nesneler okunamadı: {e}")))?;
            PointInput::Sources { sources, values }
        };
        let spec: kentos_raster::PointSpec = serde_json::from_str(spec)
            .map_err(|e| JsError::new(&format!("Çözümlemenin ayarları okunamadı: {e}")))?;
        let (job, header) = kentos_raster::PointJob::new(input, &spec, 1).map_err(fail)?;
        let g = job.grid();
        let style = |b: u32| serde_json::to_string(&job.style(b)).unwrap_or_default();
        Ok(PointAnalysis {
            grid: g
                .affine
                .iter()
                .copied()
                .chain([f64::from(g.width), f64::from(g.height)])
                .collect(),
            bands: job.bands(),
            sample: serde_json::to_value(job.sample())
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            styles: [style(1), style(2)],
            job: Some(job),
            header,
            head: Vec::new(),
            notes: String::new(),
            rows: Vec::new(),
        })
    }

    /// The bytes to write first (written over at the end by [`PointAnalysis::head`]).
    pub fn header(&self) -> Vec<u8> {
        self.header.clone()
    }

    /// The grid: its affine's six numbers, its width and height.
    pub fn grid(&self) -> Vec<f64> {
        self.grid.clone()
    }

    pub fn bands(&self) -> u32 {
        self.bands
    }

    /// The result's samples as the contract names them (`f32`; Rasterleştir's its own).
    pub fn sample(&self) -> String {
        self.sample.clone()
    }

    /// Band `band`'s look (`RasterStyle` JSON).
    pub fn style(&self, band: u32) -> String {
        self.styles[if band == 2 { 1 } else { 0 }].clone()
    }

    /// Works out the next strip (or piece of cross-validation): the bytes to append.
    pub fn step(&mut self) -> Result<Vec<u8>, JsError> {
        self.job
            .as_mut()
            .ok_or_else(|| JsError::new("Çözümleme bitti."))?
            .step()
            .map_err(fail)
    }

    pub fn done(&self) -> bool {
        self.job.as_ref().is_none_or(kentos_raster::PointJob::done)
    }

    pub fn share(&self) -> f64 {
        self.job
            .as_ref()
            .map_or(1.0, kentos_raster::PointJob::share)
    }

    /// Ends the job: the GeoTIFF's directories to append (its header then from [`PointAnalysis::head`]).
    pub fn finish(&mut self) -> Result<Vec<u8>, JsError> {
        let job = self
            .job
            .take()
            .ok_or_else(|| JsError::new("Çözümleme bitti."))?;
        let done = job.finish().map_err(fail)?;
        let n = &done.notes;
        let mut notes = serde_json::json!({
            "taken": n.taken, "merged": n.merged, "unread": n.unread, "noElevation": n.no_elevation, "empty": n.empty,
            "outside": n.outside,
        });
        if let Some(r) = n.radius {
            notes["radius"] = serde_json::json!(r);
        }
        if let Some(g) = &n.variogram {
            notes["variogram"] = serde_json::json!({ "model": g.model.name(), "nugget": g.nugget, "sill": g.sill, "range": g.range });
        }
        if !done.rows.is_empty() {
            let s = kentos_raster::from_points::CrossSummary::of(&done.rows);
            notes["cross"] = serde_json::json!({
                "count": s.count, "missing": s.missing, "mean": s.mean, "rmse": s.rmse, "mae": s.mae,
                "stdMean": s.std_mean, "stdRmse": s.std_rmse,
            });
        }
        self.notes = notes.to_string();
        self.head = done.header;
        self.rows = done.rows;
        Ok(done.tail)
    }

    /// The header to write over the result's first bytes.
    pub fn head(&self) -> Vec<u8> {
        self.head.clone()
    }

    /// What the run met (JSON): taken, merged, unread, noElevation, empty, outside, radius, variogram, cross (its sums).
    pub fn notes(&self) -> String {
        self.notes.clone()
    }

    /// The cross-validation's rows: each point's place among the points.
    #[wasm_bindgen(js_name = crossPoint)]
    pub fn cross_point(&self) -> Vec<u32> {
        self.rows.iter().map(|r| r.point).collect()
    }

    /// Each row's object (its place in the input).
    #[wasm_bindgen(js_name = crossObject)]
    pub fn cross_object(&self) -> Vec<u32> {
        self.rows.iter().map(|r| r.object).collect()
    }

    /// Each row's x, y, measured, predicted (NaN: none) and standard error (NaN: none), five numbers a row.
    #[wasm_bindgen(js_name = crossValues)]
    pub fn cross_values(&self) -> Vec<f64> {
        self.rows
            .iter()
            .flat_map(|r| {
                [
                    r.x,
                    r.y,
                    r.measured,
                    r.predicted.unwrap_or(f64::NAN),
                    r.error.unwrap_or(f64::NAN),
                ]
            })
            .collect()
    }
}

/// The inputs of a raster operation (docs/adr/0233), opened one after another.
#[wasm_bindgen]
pub struct OpsOpening {
    readers: Vec<Reader>,
}

impl Default for OpsOpening {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl OpsOpening {
    #[wasm_bindgen(constructor)]
    pub fn new() -> OpsOpening {
        OpsOpening {
            readers: Vec::new(),
        }
    }

    /// The next input: the TIFF whose header `opening` read.
    #[wasm_bindgen(js_name = addTiff)]
    pub fn add_tiff(&mut self, opening: &AnalysisOpening) -> Result<(), JsError> {
        self.readers.push(opening.reader()?);
        Ok(())
    }

    /// The next input: a PNG's raster.
    #[wasm_bindgen(js_name = addPng)]
    pub fn add_png(&mut self, bytes: &[u8]) -> Result<(), JsError> {
        self.readers.push(png_reader(bytes)?);
        Ok(())
    }

    /// The next input: a JPEG's raster from the browser's pixels.
    #[wasm_bindgen(js_name = addPixels)]
    pub fn add_pixels(
        &mut self,
        width: u32,
        height: u32,
        components: u32,
        pixels: Vec<u8>,
    ) -> Result<(), JsError> {
        self.readers
            .push(pixels_reader(width, height, components, pixels)?);
        Ok(())
    }

    /// The run of `spec` (`kentos_raster::ops::OpsSpec` JSON) over the inputs
    /// and `shapes` (a JSON array of the mask's or the zones' objects).
    pub fn start(&mut self, spec: &str, shapes: &str) -> Result<OpsAnalysis, JsError> {
        use kentos_geometry_core::api::json::{FromJson, Json};
        let spec: kentos_raster::ops::OpsSpec = serde_json::from_str(spec)
            .map_err(|e| JsError::new(&format!("Çözümlemenin ayarları okunamadı: {e}")))?;
        let j =
            Json::parse(shapes).map_err(|e| JsError::new(&format!("Nesneler okunamadı: {e}")))?;
        let Json::Arr(list) = j else {
            return Err(JsError::new("Nesneler bir liste olmalı."));
        };
        let shapes = list
            .iter()
            .map(|o| kentos_geometry_core::entity::Entity::from_json(o).map(|e| e.shape))
            .collect::<Result<Vec<_>, String>>()
            .map_err(fail)?;
        // The inputs given are the ones the run reads (`opsReads`), in that order.
        let spec = spec.reading(&spec.reads().map_err(fail)?);
        let readers = std::mem::take(&mut self.readers);
        if readers.len() != spec.inputs.len() {
            return Err(JsError::new("Rasterlerin ayarları eksik verildi."));
        }
        let inputs = readers
            .into_iter()
            .zip(&spec.inputs)
            .map(|(r, s)| kentos_raster::inputs::Input::new(r, s.affine, s.nodata))
            .collect::<Result<Vec<_>, String>>()
            .map_err(fail)?;
        let (job, header) =
            kentos_raster::ops::OpsJob::new(inputs, &spec, shapes, 1).map_err(fail)?;
        let g = job.grid();
        Ok(OpsAnalysis {
            result: job.result(),
            style: job
                .style()
                .map(|s| serde_json::to_string(&s).unwrap_or_default())
                .unwrap_or_default(),
            grid: g
                .affine
                .iter()
                .copied()
                .chain([f64::from(g.width), f64::from(g.height)])
                .collect(),
            job: Some(job),
            header,
            needs: Vec::new(),
            head: Vec::new(),
            notes: String::new(),
            zones: Vec::new(),
            histogram: String::new(),
            features: None,
        })
    }
}

/// The inputs a raster operation of `spec` (`kentos_raster::ops::OpsSpec`
/// JSON) reads, before any is opened (docs/adr/0233 §3): Raster
/// hesaplayıcı's those its expression names, in the order it names them;
/// every other tool's all. The host opens and adds only these, in this order.
#[wasm_bindgen(js_name = opsReads)]
pub fn ops_reads(spec: &str) -> Result<Vec<u32>, JsError> {
    let spec: kentos_raster::ops::OpsSpec = serde_json::from_str(spec)
        .map_err(|e| JsError::new(&format!("Çözümlemenin ayarları okunamadı: {e}")))?;
    Ok(spec
        .reads()
        .map_err(fail)?
        .into_iter()
        .map(|k| k as u32)
        .collect())
}

/// A raster operation under way.
#[wasm_bindgen]
pub struct OpsAnalysis {
    job: Option<kentos_raster::ops::OpsJob>,
    header: Vec<u8>,
    needs: Vec<(u32, BlockNeed)>,
    result: Option<(u32, RasterSample)>,
    style: String,
    grid: Vec<f64>,
    head: Vec<u8>,
    notes: String,
    zones: Vec<f64>,
    histogram: String,
    features: Option<kentos_raster::vector::Features>,
}

impl OpsAnalysis {
    fn job(&mut self) -> Result<&mut kentos_raster::ops::OpsJob, JsError> {
        self.job
            .as_mut()
            .ok_or_else(|| JsError::new("Çözümleme bitti."))
    }

    fn need(&self, i: usize) -> Result<(u32, BlockNeed), JsError> {
        self.needs
            .get(i)
            .copied()
            .ok_or_else(|| JsError::new("Böyle bir blok istenmedi."))
    }
}

#[wasm_bindgen]
impl OpsAnalysis {
    /// The bytes to write first: a raster result's header; empty for a table.
    pub fn header(&self) -> Vec<u8> {
        self.header.clone()
    }

    /// The blocks the next step wants, `[input, offset, length]` each;
    /// `putBlock` takes them by their index here.
    pub fn needs(&mut self) -> Result<Vec<f64>, JsError> {
        self.needs = self.job()?.needs();
        Ok(self
            .needs
            .iter()
            .flat_map(|(k, n)| [f64::from(*k), n.offset as f64, n.len as f64])
            .collect())
    }

    /// Need `i`'s bytes: decoded and kept; a JPEG block's stream comes back for the browser.
    #[wasm_bindgen(js_name = putBlock)]
    pub fn put_block(&mut self, i: usize, bytes: Vec<u8>) -> Result<Vec<u8>, JsError> {
        let (k, need) = self.need(i)?;
        let mut jpeg = self.job()?.put_all(vec![(k, need, bytes)]).map_err(fail)?;
        Ok(jpeg.pop().map(|(_, _, stream)| stream).unwrap_or_default())
    }

    /// A JPEG block's pixels (`components` a pixel) for need `i`.
    #[wasm_bindgen(js_name = putPixels)]
    pub fn put_pixels(
        &mut self,
        i: usize,
        pixels: Vec<u8>,
        components: u32,
    ) -> Result<(), JsError> {
        let (k, need) = self.need(i)?;
        self.job()?
            .put_pixels(k, &need, pixels, components)
            .map_err(fail)
    }

    /// Works out the next block: the bytes to append.
    pub fn step(&mut self) -> Result<Vec<u8>, JsError> {
        self.job()?.step().map_err(fail)
    }

    pub fn done(&self) -> bool {
        self.job
            .as_ref()
            .is_none_or(kentos_raster::ops::OpsJob::done)
    }

    pub fn share(&self) -> f64 {
        self.job
            .as_ref()
            .map_or(1.0, kentos_raster::ops::OpsJob::share)
    }

    /// Ends the run: a raster result's directories to append (its header
    /// then from `head`); nothing for a table (then read by `zones`, `histogram`).
    pub fn finish(&mut self) -> Result<Vec<u8>, JsError> {
        use kentos_raster::ops::OpsFinished;
        let job = self
            .job
            .take()
            .ok_or_else(|| JsError::new("Çözümleme bitti."))?;
        let n = job.notes().clone();
        let h = &n.hydro;
        self.notes = serde_json::json!({
            "cells": n.cells,
            "emptyCells": n.empty_cells,
            // Hidroloji (docs/adr/0235): what the run met.
            "hydro": {
                "cells": h.cells,
                "empty": h.empty,
                "skipped": h.skipped,
                "emptyPoints": h.empty_points,
                "dropped": h.dropped,
                "threshold": h.threshold,
                "links": h.links,
                "most": h.most,
            },
            // Uzaklık ve maliyet (docs/adr/0236): what the run met (least and most null when no cell has a value).
            "distance": {
                "sources": n.distance.sources,
                "outside": n.distance.outside,
                "unreached": n.distance.unreached,
                "least": n.distance.least,
                "most": n.distance.most,
                "cells": n.distance.cells,
            },
        })
        .to_string();
        match job.finish().map_err(fail)? {
            OpsFinished::Raster { tail, header } => {
                self.head = header;
                Ok(tail)
            }
            OpsFinished::Zones(z) => {
                let none = |v: Option<f64>| v.unwrap_or(f64::NAN);
                self.zones = z
                    .iter()
                    .flat_map(|f| {
                        let m = &f.moments;
                        [
                            m.n as f64,
                            none(f.value),
                            none(m.sum()),
                            none(m.mean()),
                            none(m.min()),
                            none(m.max()),
                            none(m.std()),
                        ]
                    })
                    .collect();
                Ok(Vec::new())
            }
            OpsFinished::Histogram(h) => {
                self.histogram = serde_json::json!({
                    "lo": h.lo, "hi": h.hi, "counts": h.counts, "below": h.below,
                    "above": h.above, "valid": h.valid, "empty": h.empty,
                })
                .to_string();
                Ok(Vec::new())
            }
            OpsFinished::Features(f) => {
                self.features = Some(f);
                Ok(Vec::new())
            }
        }
    }

    /// The header to write over the result's first bytes.
    pub fn head(&self) -> Vec<u8> {
        self.head.clone()
    }

    /// A raster result's bands (0 for a table).
    pub fn bands(&self) -> u32 {
        self.result.map_or(0, |r| r.0)
    }

    /// A raster result's samples, as the contract names them; empty for a table.
    pub fn sample(&self) -> String {
        self.result.map_or_else(String::new, |r| {
            serde_json::to_value(r.1)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default()
        })
    }

    /// A raster result's look (`RasterStyle` JSON); empty for a table.
    pub fn style(&self) -> String {
        self.style.clone()
    }

    /// The result's grid: its affine's six numbers, width and height.
    pub fn grid(&self) -> Vec<f64> {
        self.grid.clone()
    }

    /// What the run met (JSON: cells, emptyCells).
    pub fn notes(&self) -> String {
        self.notes.clone()
    }

    /// Each zone's figures, seven a zone: cells with a value, the statistic
    /// asked for, the sum, mean, least, largest, standard deviation (NaN: none).
    pub fn zones(&self) -> Vec<f64> {
        self.zones.clone()
    }

    /// The histogram (JSON: lo, hi, counts, below, above, valid, empty).
    pub fn histogram(&self) -> String {
        self.histogram.clone()
    }

    /// A vectorizing run's features (docs/adr/0234): `polygons`, `lines` or
    /// `points`; empty for any other run.
    #[wasm_bindgen(js_name = featureKind)]
    pub fn feature_kind(&self) -> String {
        use kentos_raster::vector::FeatureKind;
        match self.features.as_ref().map(|f| f.kind) {
            Some(FeatureKind::Polygons) => "polygons",
            Some(FeatureKind::Lines) => "lines",
            Some(FeatureKind::Points) => "points",
            None => "",
        }
        .to_owned()
    }

    /// Each feature's value (NaN: none).
    #[wasm_bindgen(js_name = featureValues)]
    pub fn feature_values(&self) -> Vec<f64> {
        self.features
            .as_ref()
            .map_or_else(Vec::new, |f| f.values.clone())
    }

    /// Each feature's value as its text.
    #[wasm_bindgen(js_name = featureTexts)]
    pub fn feature_texts(&self) -> Vec<String> {
        self.features
            .as_ref()
            .map_or_else(Vec::new, |f| f.texts.clone())
    }

    /// Each feature's tag (a point: 1 a peak, 2 a pit; an area: 1 written unsimplified).
    #[wasm_bindgen(js_name = featureTags)]
    pub fn feature_tags(&self) -> Vec<u8> {
        self.features
            .as_ref()
            .map_or_else(Vec::new, |f| f.tags.clone())
    }

    /// Each area's ring count (empty for lines and points).
    #[wasm_bindgen(js_name = featureRings)]
    pub fn feature_rings(&self) -> Vec<u32> {
        self.features
            .as_ref()
            .map_or_else(Vec::new, |f| f.rings.clone())
    }

    /// Each ring's, line's or point's vertex count.
    #[wasm_bindgen(js_name = featureSizes)]
    pub fn feature_sizes(&self) -> Vec<u32> {
        self.features
            .as_ref()
            .map_or_else(Vec::new, |f| f.sizes.clone())
    }

    /// Every vertex's x, y in order.
    #[wasm_bindgen(js_name = featureXy)]
    pub fn feature_xy(&self) -> Vec<f64> {
        self.features
            .as_ref()
            .map_or_else(Vec::new, |f| f.xy.clone())
    }

    /// The names of the numbers each feature carries (docs/adr/0235: Havza, Alan …).
    #[wasm_bindgen(js_name = featureFields)]
    pub fn feature_fields(&self) -> Vec<String> {
        self.features.as_ref().map_or_else(Vec::new, |f| {
            f.fields.iter().map(|s| (*s).to_owned()).collect()
        })
    }

    /// Those numbers, `featureFields().length` a feature.
    #[wasm_bindgen(js_name = featureNumbers)]
    pub fn feature_numbers(&self) -> Vec<f64> {
        self.features
            .as_ref()
            .map_or_else(Vec::new, |f| f.numbers.clone())
    }
}
