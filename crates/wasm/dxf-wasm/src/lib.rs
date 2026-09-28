//! AutoCAD DXF in the browser. The formats worker
//! (`apps/web/src/io/formatsWorker.ts`) loads this module the first time a
//! DXF is imported or exported and never before (CLAUDE.md §20): opening
//! and saving a drawing loads the formats module, which no longer holds the
//! DXF reader and writer. The file crosses as bytes and the options as JSON;
//! the result's objects as the typed columns a drawing crosses in
//! (`kentos_kcad::columns`, docs/adr/0030), the rest (layers, report,
//! extent) as the contract's JSON. The read says how far it is as it goes;
//! the page stops it by ending the worker.

use kentos_contracts::{DxfReadOptions, EntityId, FORMATS_VERSION};
use kentos_formats::watch::Watch;
use wasm_bindgen::prelude::*;

/// Version of the formats contracts this module speaks (`FORMATS_VERSION`).
#[wasm_bindgen(js_name = dxfVersion)]
pub fn dxf_version() -> u32 {
    FORMATS_VERSION
}

#[wasm_bindgen]
extern "C" {
    /// Hears a long read (`io/protocol.ts` gives a plain object with this method).
    pub type ReadProgress;

    /// How far the read is: `done` of `total` (thousandths).
    #[wasm_bindgen(method)]
    fn step(this: &ReadProgress, done: f64, total: f64);
}

/// The reader's progress, told to the page's side.
struct Relay<'p>(&'p ReadProgress);

impl Watch for Relay<'_> {
    fn step(&mut self, done: u64, total: u64) -> bool {
        self.0.step(done as f64, total as f64);
        // Stopping is the page's: it ends the worker (a call here cannot be interrupted).
        true
    }
}

/// What a read gives back: a refusal (`ok` false and the reason), or the
/// result's head as JSON and its objects as columns. Each array is taken
/// once, so it is copied out of the module once.
#[wasm_bindgen]
#[derive(Default)]
pub struct Imported {
    ok: bool,
    message: String,
    head: String,
    columns: kentos_kcad::Columns,
}

#[wasm_bindgen]
impl Imported {
    #[wasm_bindgen(getter)]
    pub fn ok(&self) -> bool {
        self.ok
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// The `ImportResult` without its objects (`entities` empty).
    #[wasm_bindgen(js_name = takeHead)]
    pub fn take_head(&mut self) -> String {
        std::mem::take(&mut self.head)
    }

    #[wasm_bindgen(js_name = takeKinds)]
    pub fn take_kinds(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.columns.kinds)
    }

    #[wasm_bindgen(js_name = takeUids)]
    pub fn take_uids(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.columns.uids)
    }

    #[wasm_bindgen(js_name = takeInts)]
    pub fn take_ints(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.columns.ints)
    }

    #[wasm_bindgen(js_name = takeFloats)]
    pub fn take_floats(&mut self) -> Vec<f64> {
        std::mem::take(&mut self.columns.floats)
    }

    #[wasm_bindgen(js_name = takeText)]
    pub fn take_text(&mut self) -> Vec<u16> {
        std::mem::take(&mut self.columns.text)
    }

    #[wasm_bindgen(js_name = takeTextLengths)]
    pub fn take_text_lengths(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.columns.text_lengths)
    }
}

fn refused(message: String) -> Imported {
    Imported {
        message,
        ..Imported::default()
    }
}

/// Reads an ASCII DXF file: `options` is `DxfReadOptions`. A file that is
/// not a DXF (a DWG, a binary DXF, broken groups) is refused with the
/// reason, never thrown.
#[wasm_bindgen(js_name = readDxf)]
pub fn read_dxf(bytes: Vec<u8>, options: &str, progress: &ReadProgress) -> Imported {
    let opts: DxfReadOptions = match serde_json::from_str(options) {
        Ok(o) => o,
        Err(e) => {
            return refused(format!(
                "Okuma seçenekleri okunamadı ({e}); uygulama ile DXF modülü uyuşmuyor olabilir (pnpm wasm)."
            ));
        }
    };
    let read = kentos_formats::dxf::read_watched(&bytes, &opts, &mut Relay(progress));
    drop(bytes);
    match read {
        Ok(mut result) => {
            let entities = std::mem::take(&mut result.entities);
            let head = match serde_json::to_string(&result) {
                Ok(h) => h,
                Err(e) => return refused(format!("Sonuç yazılamadı: {e}")),
            };
            let n = entities.len();
            // The objects are not in a drawing yet: no persistent id (the page gives one).
            let columns = kentos_kcad::columns::pack(entities, vec![EntityId([0; 16]); n]);
            Imported {
                ok: true,
                head,
                columns,
                ..Imported::default()
            }
        }
        Err(e) => refused(e),
    }
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

/// Writes an AutoCAD 2007 DXF from `DxfWriteInput` (JSON). The objects are
/// read by the formats crate's own visitor (`dxf::WriteInput`), not the
/// contract's derived code, which would weigh ~100 KB in this module.
#[wasm_bindgen(js_name = writeDxf)]
pub fn write_dxf(input: &str) -> Result<Written, JsError> {
    let input: kentos_formats::dxf::WriteInput = serde_json::from_str(input).map_err(|e| {
        JsError::new(&format!(
            "Yazılacak nesneler okunamadı ({e}); uygulama ile DXF modülü uyuşmuyor olabilir (pnpm wasm)."
        ))
    })?;
    let (bytes, report) = kentos_formats::dxf::write(&input.0);
    let report = serde_json::to_string(&report).map_err(|e| JsError::new(&format!("Rapor yazılamadı: {e}")))?;
    Ok(Written { bytes, report })
}
