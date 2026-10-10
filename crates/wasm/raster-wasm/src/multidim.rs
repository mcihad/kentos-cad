//! Mesh and multidimensional data in the analysis worker (docs/adr/0243): a
//! NetCDF raster's file read in the pieces it asks for ([`CubeOpening`]: its
//! header, then what its dataset's slice needs), as an input of the raster
//! jobs (the slice the raster shows) or of Çok boyutlu veri's tools
//! ([`MultidimAnalysis`]: Kesit and Zaman serisi over bands on any opened
//! raster, Zaman serisi over time steps and Mesh hesaplayıcı on a cube).

use kentos_formats::multidim::cube::{Cube, Part, Want};
use kentos_formats::multidim::netcdf;
use kentos_formats::raster::source::Reader;
use kentos_formats::raster::{ByteStore, Need, Step};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::{Entity, Shape};
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::multidim::calc::{CalcSpec, MeshCalc};
use kentos_raster::multidim::series::{TimeJob, bands_job, named_points};
use kentos_raster::multidim::{Finished, MultidimJob, profile};
use wasm_bindgen::prelude::*;

use crate::{Analysis, OpsOpening};

fn fail(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

/// The most bytes a NetCDF header may take.
const HEADER_MOST: u64 = 128 * 1024 * 1024;

/// A NetCDF raster's file opened: its header, then what its slice needs.
#[wasm_bindgen]
pub struct CubeOpening {
    store: ByteStore,
    size: u64,
    taken: u64,
    cube: Option<Cube>,
    part: Part,
    pending: Vec<Need>,
}

#[wasm_bindgen]
impl CubeOpening {
    /// A file of `size` bytes showing `part` (the raster's part JSON, as its key carries it).
    #[wasm_bindgen(constructor)]
    pub fn new(size: f64, part: &str) -> Result<CubeOpening, JsError> {
        Ok(CubeOpening {
            store: ByteStore::new(),
            size: size.max(0.0) as u64,
            taken: 0,
            cube: None,
            part: Part::from_json(part)
                .ok_or_else(|| JsError::new("Veri setinin tanımı okunamadı."))?,
            pending: Vec::new(),
        })
    }

    /// Whether a file's first bytes are a NetCDF's (classic or NetCDF-4; the latter is refused saying why).
    #[wasm_bindgen(js_name = isNetcdf)]
    pub fn is_netcdf(head: &[u8]) -> bool {
        matches!(
            netcdf::sniff(head),
            netcdf::Sniff::Classic(_) | netcdf::Sniff::Hdf5
        )
    }

    /// The next run to read, `[offset, length]`: the header's, then the slice's
    /// (coordinates, a mesh); empty once it has all.
    pub fn need(&mut self) -> Result<Vec<f64>, JsError> {
        if self.cube.is_none() {
            match Cube::parse(&self.store, self.size).map_err(fail)? {
                Step::Done(c) => {
                    self.store.clear();
                    self.pending = c.needs(Want::Part(&self.part));
                    self.cube = Some(c);
                }
                Step::Need(n) => {
                    self.taken += n.len;
                    if self.taken > HEADER_MOST {
                        return Err(JsError::new("NetCDF'in başlığı çok büyük."));
                    }
                    return Ok(vec![n.offset as f64, n.len as f64]);
                }
            }
        }
        Ok(self
            .pending
            .pop()
            .map_or_else(Vec::new, |n| vec![n.offset as f64, n.len as f64]))
    }

    /// The bytes that were at `offset`.
    pub fn put(&mut self, offset: f64, bytes: Vec<u8>) {
        let at = offset.max(0.0) as u64;
        match self.cube.as_mut() {
            Some(c) => c.put(at, bytes),
            None => self.store.put(at, bytes),
        }
    }

    /// The analysis of `spec` (`kentos_raster::job::Spec` JSON) over the slice.
    pub fn analysis(&mut self, spec: &str) -> Result<Analysis, JsError> {
        Analysis::of(self.reader()?, spec)
    }
}

impl CubeOpening {
    fn cube(&mut self) -> Result<&mut Cube, JsError> {
        self.cube
            .as_mut()
            .ok_or_else(|| JsError::new("NetCDF'in başlığı okunmadı."))
    }

    /// The slice's reader (its runs read).
    pub(crate) fn reader(&mut self) -> Result<Reader, JsError> {
        let part = self.part.clone();
        self.cube()?.open(&part, READER_BUDGET).map_err(fail)
    }
}

#[wasm_bindgen]
impl OpsOpening {
    /// The next input: the NetCDF slice `opening` opened.
    #[wasm_bindgen(js_name = addCube)]
    pub fn add_cube(&mut self, opening: &mut CubeOpening) -> Result<(), JsError> {
        self.readers.push(opening.reader()?);
        Ok(())
    }
}

/// The objects of a JSON array: each one's shape and its `ad`.
fn objects(json: &str) -> Result<Vec<(Option<String>, Shape)>, JsError> {
    let j = Json::parse(json).map_err(|e| JsError::new(&format!("Nesneler okunamadı: {e}")))?;
    let Json::Arr(list) = j else {
        return Err(JsError::new("Nesneler bir liste olmalı."));
    };
    list.iter()
        .map(|o| {
            let e = Entity::from_json(o).map_err(fail)?;
            let name = match o.get("attrs").get("ad") {
                Json::Str(s) => Some(s.clone()),
                _ => None,
            };
            Ok((name, e.shape))
        })
        .collect()
}

fn affine_of(a: &[f64]) -> Result<[f64; 6], JsError> {
    a.try_into()
        .map_err(|_| JsError::new("Rasterin yeri altı sayı olmalı."))
}

/// The one input of an opening, placed.
fn input_of(
    opening: &mut OpsOpening,
    affine: &[f64],
    nodata: Option<f64>,
) -> Result<Input, JsError> {
    let mut readers = std::mem::take(&mut opening.readers);
    if readers.len() != 1 {
        return Err(JsError::new("Bu araç tek raster okur."));
    }
    Input::new(readers.remove(0), affine_of(affine)?, nodata).map_err(fail)
}

/// A Çok boyutlu veri tool's run: its job, the blocks it asked for last, what it gave.
#[wasm_bindgen]
pub struct MultidimAnalysis {
    job: Option<MultidimJob>,
    finished: Option<Finished>,
    file: Vec<u8>,
    output: String,
}

impl MultidimAnalysis {
    fn of(job: MultidimJob) -> MultidimAnalysis {
        MultidimAnalysis {
            job: Some(job),
            finished: None,
            file: Vec::new(),
            output: String::new(),
        }
    }

    fn job(&mut self) -> Result<&mut MultidimJob, JsError> {
        self.job
            .as_mut()
            .ok_or_else(|| JsError::new("Çözümleme bitti."))
    }
}

#[wasm_bindgen]
impl MultidimAnalysis {
    /// Kesit over `opening`'s one input placed by `affine`: band `band` (from
    /// 1) along `lines` (objects' JSON), a point every `step`; `axes` the
    /// axes' names, east's first, a comma between.
    pub fn profile(
        opening: &mut OpsOpening,
        affine: &[f64],
        nodata: Option<f64>,
        lines: &str,
        step: f64,
        band: u32,
        axes: &str,
    ) -> Result<MultidimAnalysis, JsError> {
        let input = input_of(opening, affine, nodata)?;
        let shapes: Vec<Shape> = objects(lines)?.into_iter().map(|(_, s)| s).collect();
        let (east, north) = axes.split_once(',').unwrap_or(("Y", "X"));
        let job = profile::job(
            input,
            &shapes,
            step,
            band.max(1) as usize - 1,
            [east.to_owned(), north.to_owned()],
        )
        .map_err(fail)?;
        Ok(MultidimAnalysis::of(MultidimJob::Points(Box::new(job))))
    }

    /// Zaman serisi over the bands of `opening`'s one input at `points` (objects' JSON).
    #[wasm_bindgen(js_name = bandSeries)]
    pub fn band_series(
        opening: &mut OpsOpening,
        affine: &[f64],
        nodata: Option<f64>,
        points: &str,
    ) -> Result<MultidimAnalysis, JsError> {
        let input = input_of(opening, affine, nodata)?;
        let job = bands_job(input, &named_points(objects(points)?)).map_err(fail)?;
        Ok(MultidimAnalysis::of(MultidimJob::Points(Box::new(job))))
    }

    /// Zaman serisi over the time steps of the NetCDF slice `cube` opened, at `points`.
    #[wasm_bindgen(js_name = timeSeries)]
    pub fn time_series(
        cube: &mut CubeOpening,
        affine: &[f64],
        nodata: Option<f64>,
        points: &str,
    ) -> Result<MultidimAnalysis, JsError> {
        let named = named_points(objects(points)?);
        let xy: Vec<[f64; 2]> = named.iter().map(|p| [p.x, p.y]).collect();
        let part = cube.part.clone();
        let affine = affine_of(affine)?;
        let series = cube
            .cube()?
            .series(&part, affine, nodata, &xy)
            .map_err(fail)?;
        let job = TimeJob::new(series, &named).map_err(fail)?;
        Ok(MultidimAnalysis::of(MultidimJob::Series(Box::new(job))))
    }

    /// Mesh hesaplayıcı over the mesh of the slice `cube` opened (`spec` the `CalcSpec` JSON).
    #[wasm_bindgen(js_name = meshCalc)]
    pub fn mesh_calc(cube: &mut CubeOpening, spec: &str) -> Result<MultidimAnalysis, JsError> {
        let spec: CalcSpec = serde_json::from_str(spec)
            .map_err(|e| JsError::new(&format!("Hesabın ayarları okunamadı: {e}")))?;
        let part = cube.part.clone();
        let calc = MeshCalc::new(cube.cube()?, &part, &spec).map_err(fail)?;
        let (variable, mesh, steps) = calc.output();
        let output = serde_json::json!({
            "variable": variable,
            "mesh": mesh,
            "steps": steps.map(|(values, time)| serde_json::json!({ "values": values, "time": time })),
        })
        .to_string();
        let mut a = MultidimAnalysis::of(MultidimJob::Calc(Box::new(calc)));
        a.output = output;
        Ok(a)
    }

    /// The runs the next step reads, `[offset, length, …]`.
    pub fn needs(&mut self) -> Result<Vec<f64>, JsError> {
        let job = self.job()?;
        let runs = job.needs().map_err(fail)?;
        Ok(runs
            .iter()
            .flat_map(|&(a, n)| [a as f64, n as f64])
            .collect())
    }

    /// Need `i`'s bytes; a JPEG block's stream back (empty otherwise).
    #[wasm_bindgen(js_name = putBlock)]
    pub fn put_block(&mut self, i: usize, bytes: Vec<u8>) -> Result<Vec<u8>, JsError> {
        Ok(self.job()?.put(i, bytes).map_err(fail)?.unwrap_or_default())
    }

    /// A JPEG block's pixels (`components` a pixel).
    #[wasm_bindgen(js_name = putPixels)]
    pub fn put_pixels(
        &mut self,
        i: usize,
        pixels: Vec<u8>,
        components: u32,
    ) -> Result<(), JsError> {
        self.job()?.put_pixels(i, pixels, components).map_err(fail)
    }

    pub fn step(&mut self) -> Result<(), JsError> {
        self.job()?.step().map_err(fail)
    }

    pub fn done(&self) -> bool {
        self.job.as_ref().is_none_or(MultidimJob::done)
    }

    pub fn share(&self) -> f64 {
        self.job.as_ref().map_or(1.0, MultidimJob::share)
    }

    /// What the run gave (JSON: `table`, `pieces`, `summary`, `warnings`); Mesh hesaplayıcı's file kept for [`MultidimAnalysis::file`].
    pub fn finish(&mut self) -> Result<String, JsError> {
        let job = self
            .job
            .take()
            .ok_or_else(|| JsError::new("Çözümleme bitti."))?;
        let mut file = Vec::new();
        let f = job
            .finish(&mut |b: &[u8]| {
                file.extend_from_slice(b);
                Ok(())
            })
            .map_err(fail)?;
        self.file = file;
        let out = serde_json::json!({
            "table": f.table.as_ref().map(|t| serde_json::json!({ "columns": t.columns, "rows": t.rows })),
            "pieces": f.pieces.iter().map(|p| serde_json::json!({
                "line": p.line,
                "points": p.points.iter().map(|q| [q[0], q[1], q[2]]).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "summary": f.summary,
            "warnings": f.warnings,
        })
        .to_string();
        self.finished = Some(f);
        Ok(out)
    }

    /// Mesh hesaplayıcı's UGRID file (once finished).
    pub fn file(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.file)
    }

    /// Mesh hesaplayıcı's new dataset (JSON: `variable`, `mesh`, `steps`: their values and whether moments, or null).
    pub fn output(&self) -> String {
        self.output.clone()
    }
}
