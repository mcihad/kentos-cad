//! Mesh and multidimensional data in the browser (docs/adr/0243): a NetCDF
//! file read in the pieces it asks for (its header, coordinates, a mesh's
//! nodes and faces), what it holds for Raster ekle and Mesh ekle, a slice's
//! reader (a `RasterFile`, as a TIFF's), and 2DM with ASCII DATs made one
//! UGRID file. The same code as the desktop's (`kentos_formats::multidim`).

use kentos_formats::multidim::cube::{Cube, Part, Want};
use kentos_formats::multidim::{netcdf, sms, write};
use kentos_formats::raster::{ByteStore, Step};
use wasm_bindgen::prelude::*;

use crate::raster::RasterFile;

fn fail(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

/// The most bytes a header may take, as the desktop's.
const HEADER_MOST: u64 = 128 * 1024 * 1024;

/// A NetCDF file being opened: its header, then what a want needs.
#[wasm_bindgen]
pub struct NetcdfFile {
    store: ByteStore,
    size: u64,
    taken: u64,
    cube: Option<Cube>,
}

#[wasm_bindgen]
impl NetcdfFile {
    /// A file of `size` bytes about to be read.
    #[wasm_bindgen(constructor)]
    pub fn new(size: f64) -> NetcdfFile {
        NetcdfFile {
            store: ByteStore::new(),
            size: size.max(0.0) as u64,
            taken: 0,
            cube: None,
        }
    }

    /// Whether a file's first bytes are a classic NetCDF's.
    #[wasm_bindgen(js_name = isNetcdf)]
    pub fn is_netcdf(head: &[u8]) -> bool {
        matches!(netcdf::sniff(head), netcdf::Sniff::Classic(_))
    }

    /// Why a NetCDF-4 (HDF5) file is not read, when the head is one's; empty otherwise.
    #[wasm_bindgen(js_name = refusal)]
    pub fn refusal(head: &[u8]) -> String {
        match netcdf::sniff(head) {
            netcdf::Sniff::Hdf5 => netcdf::HDF5_REFUSED.to_owned(),
            _ => String::new(),
        }
    }

    /// The runs still to read, `[offset, length, …]`: the header's first, then
    /// what `part` (a raster key's JSON; empty: everything the file lists) needs.
    pub fn needs(&mut self, part: &str) -> Result<Vec<f64>, JsError> {
        if self.cube.is_none() {
            match Cube::parse(&self.store, self.size).map_err(fail)? {
                Step::Done(c) => {
                    self.store.clear();
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
        let cube = self
            .cube
            .as_ref()
            .ok_or_else(|| JsError::new("NetCDF açılmadı."))?;
        let parsed = (!part.is_empty())
            .then(|| {
                Part::from_json(part).ok_or_else(|| JsError::new("Veri setinin tanımı okunamadı."))
            })
            .transpose()?;
        let want = match &parsed {
            Some(p) => Want::Part(p),
            None => Want::Inspect,
        };
        Ok(cube
            .needs(want)
            .iter()
            .flat_map(|n| [n.offset as f64, n.len as f64])
            .collect())
    }

    /// The bytes that were at `offset`.
    pub fn put(&mut self, offset: f64, bytes: Vec<u8>) {
        match self.cube.as_mut() {
            Some(c) => c.put(offset as u64, bytes),
            None => self.store.put(offset as u64, bytes),
        }
    }

    /// What the file holds (`CubeInfo` JSON), after `needs("")`'s runs.
    pub fn info(&mut self) -> Result<String, JsError> {
        let cube = self
            .cube
            .as_mut()
            .ok_or_else(|| JsError::new("NetCDF açılmadı."))?;
        serde_json::to_string(&cube.info().map_err(fail)?).map_err(fail)
    }

    /// A slice's reader (`part` a raster key's JSON), after `needs(part)`'s runs.
    pub fn open(&mut self, part: &str, budget: usize) -> Result<RasterFile, JsError> {
        let cube = self
            .cube
            .as_mut()
            .ok_or_else(|| JsError::new("NetCDF açılmadı."))?;
        let p =
            Part::from_json(part).ok_or_else(|| JsError::new("Veri setinin tanımı okunamadı."))?;
        Ok(RasterFile::of(cube.open(&p, budget).map_err(fail)?))
    }
}

/// The virtual grid a mesh is drawn in (docs/adr/0243 §5): cells of `cell`
/// over its box (x₁, y₁, x₂, y₂): the affine's six numbers, width, height;
/// empty when the cell is no length or a side passes 65 536 cells.
#[wasm_bindgen(js_name = meshGrid)]
pub fn mesh_grid(bbox: &[f64], cell: f64) -> Vec<f64> {
    let Ok(b) = <[f64; 4]>::try_from(bbox) else {
        return Vec::new();
    };
    match kentos_formats::multidim::mesh::grid_of_box(b, cell) {
        Some((a, w, h)) => a
            .iter()
            .copied()
            .chain([f64::from(w), f64::from(h)])
            .collect(),
        None => Vec::new(),
    }
}

/// The look a dataset's raster starts with (`RasterStyle` JSON): `sample` its samples' name.
#[wasm_bindgen(js_name = datasetStyle)]
pub fn dataset_style(sample: &str, mesh: bool, edges: bool) -> Result<String, JsError> {
    let sample: kentos_contracts::RasterSample =
        serde_json::from_value(serde_json::Value::String(sample.to_owned())).map_err(fail)?;
    serde_json::to_string(&kentos_formats::raster::style::dataset_style(
        sample, mesh, edges,
    ))
    .map_err(fail)
}

/// A 2DM and its DATs made one UGRID file: its bytes and the report's JSON.
#[wasm_bindgen]
pub struct SmsOut {
    bytes: Vec<u8>,
    report: String,
}

#[wasm_bindgen]
impl SmsOut {
    pub fn bytes(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.bytes)
    }

    pub fn report(&self) -> String {
        self.report.clone()
    }
}

/// 2DM (`mesh`) and its ASCII DATs (`dats` one after another, `lengths` each's
/// bytes, `names` their names a line each) as one UGRID file (docs/adr/0243
/// §4): times from `start` (ms since 1970; NaN none) or a DAT's `RT_JULIAN`;
/// `epsg` 0 none.
#[wasm_bindgen(js_name = smsToUgrid)]
pub fn sms_to_ugrid(
    mesh: &[u8],
    dats: &[u8],
    lengths: &[u32],
    names: &str,
    start: f64,
    epsg: u32,
    geographic: bool,
) -> Result<SmsOut, JsError> {
    let m = sms::read_2dm(mesh).map_err(fail)?;
    let mut list = Vec::new();
    let mut at = 0usize;
    for (k, name) in names.lines().enumerate() {
        let n = lengths.get(k).copied().unwrap_or(0) as usize;
        let bytes = dats
            .get(at..at + n)
            .ok_or_else(|| JsError::new("DAT dosyaları eksik geldi."))?;
        at += n;
        list.push((
            name.to_owned(),
            sms::read_dat(bytes).map_err(|e| fail(format!("“{name}”: {e}")))?,
        ));
    }
    let mut out = Vec::new();
    let mut sink = |b: &[u8]| -> Result<(), String> {
        out.extend_from_slice(b);
        Ok(())
    };
    let report = write::write_sms(
        &m,
        &list,
        start.is_finite().then_some(start),
        (epsg != 0).then_some(epsg),
        geographic,
        &mut sink,
    )
    .map_err(fail)?;
    Ok(SmsOut {
        bytes: out,
        report: serde_json::json!({
            "nodes": report.nodes,
            "faces": report.faces,
            "datasets": report.datasets,
            "notes": report.notes,
        })
        .to_string(),
    })
}
