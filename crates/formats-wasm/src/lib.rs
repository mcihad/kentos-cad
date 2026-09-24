//! The file formats in the browser. The formats Web Worker
//! (`src/io/formatsWorker.ts`) loads this module the first time the user
//! imports or exports a file; it never loads at start-up (CLAUDE.md §20).
//! Files cross as bytes; options and results as JSON (the contracts in
//! `kentos_contracts::formats`), whose float64 values serde_json writes as
//! the shortest round-trip decimal, so coordinates arrive bit for bit.

use kentos_contracts::{CoordReadOptions, CoordWriteInput, DxfReadOptions, FORMATS_VERSION};
use wasm_bindgen::prelude::*;

fn bad_input(what: &str, e: &serde_json::Error) -> JsError {
    JsError::new(&format!("{what} okunamadı ({e}); uygulama ile dosya biçimi paketi uyuşmuyor olabilir (pnpm wasm)."))
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<Vec<u8>, JsError> {
    serde_json::to_vec(v).map_err(|e| JsError::new(&format!("Sonuç yazılamadı: {e}")))
}

/// Version of the formats contracts this module speaks (`FORMATS_VERSION`).
#[wasm_bindgen(js_name = formatsVersion)]
pub fn formats_version() -> u32 {
    FORMATS_VERSION
}

/// A file a writer produced and what it did (`ExportReport` JSON).
#[wasm_bindgen]
pub struct Written {
    bytes: Vec<u8>,
    report: String,
}

#[wasm_bindgen]
impl Written {
    /// The file's bytes; taken once, so they are not copied twice.
    #[wasm_bindgen(js_name = takeBytes)]
    pub fn take_bytes(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.bytes)
    }

    #[wasm_bindgen(getter)]
    pub fn report(&self) -> String {
        self.report.clone()
    }
}

/// Reads a coordinate list: `options` is `CoordReadOptions`, the result `CoordRead` (JSON bytes).
#[wasm_bindgen(js_name = readCoords)]
pub fn read_coords(bytes: &[u8], options: &str) -> Result<Vec<u8>, JsError> {
    let opts: CoordReadOptions = serde_json::from_str(options).map_err(|e| bad_input("Okuma seçenekleri", &e))?;
    to_json(&kentos_formats::coords::read(bytes, &opts))
}

/// Reads an ASCII DXF file: `options` is `DxfReadOptions`, the result `ImportResult` (JSON bytes).
/// A file that is not a DXF (a DWG, a binary DXF, broken groups) throws the reason.
#[wasm_bindgen(js_name = readDxf)]
pub fn read_dxf(bytes: &[u8], options: &str) -> Result<Vec<u8>, JsError> {
    let opts: DxfReadOptions = serde_json::from_str(options).map_err(|e| bad_input("Okuma seçenekleri", &e))?;
    let result = kentos_formats::dxf::read(bytes, &opts).map_err(|e| JsError::new(&e))?;
    to_json(&result)
}

/// Writes a coordinate list from `CoordWriteInput` (JSON).
#[wasm_bindgen(js_name = writeCoords)]
pub fn write_coords(input: &str) -> Result<Written, JsError> {
    let input: CoordWriteInput = serde_json::from_str(input).map_err(|e| bad_input("Yazılacak noktalar", &e))?;
    let (bytes, report) = kentos_formats::coords::write(&input);
    Ok(Written { bytes, report: String::from_utf8(to_json(&report)?).unwrap_or_default() })
}
