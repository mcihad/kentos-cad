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
        let t = self
            .parsed
            .as_ref()
            .ok_or_else(|| JsError::new("TIFF'in başlığı okunmadı."))?;
        Analysis::of(Reader::tiff(t, None, READER_BUDGET).map_err(fail)?, spec)
    }
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
        let p = kentos_formats::raster::png::decode(bytes).map_err(fail)?;
        let reader = Reader::image(
            p.width,
            p.height,
            p.bands,
            p.samples,
            p.nodata,
            None,
            READER_BUDGET,
        )
        .map_err(fail)?;
        Analysis::of(reader, spec)
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
            None,
            READER_BUDGET,
        )
        .map_err(fail)?;
        Analysis::of(reader, spec)
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
