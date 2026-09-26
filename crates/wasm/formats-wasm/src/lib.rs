//! The file formats in the browser. The formats Web Worker
//! (`apps/web/src/io/formatsWorker.ts`) loads this module the first time the user
//! imports or exports a file, or opens or saves a drawing (the `.kcad` v2
//! codec, `kentos-kcad`; a v1 drawing's persistent ids); it never loads at
//! start-up (CLAUDE.md §20).
//! Files cross as bytes; options and results as JSON (the contracts in
//! `kentos_contracts::formats`), whose float64 values serde_json writes as
//! the shortest round-trip decimal, so coordinates arrive bit for bit.

use kentos_contracts::{
    CoordReadOptions, CoordWriteInput, DocumentSnapshotV2, DxfReadOptions, FORMATS_VERSION,
};
use wasm_bindgen::prelude::*;

fn bad_input(what: &str, e: &serde_json::Error) -> JsError {
    JsError::new(&format!(
        "{what} okunamadı ({e}); uygulama ile dosya biçimi paketi uyuşmuyor olabilir (pnpm wasm)."
    ))
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
    let opts: CoordReadOptions =
        serde_json::from_str(options).map_err(|e| bad_input("Okuma seçenekleri", &e))?;
    to_json(&kentos_formats::coords::read(bytes, &opts))
}

/// Reads an ASCII DXF file: `options` is `DxfReadOptions`, the result `ImportResult` (JSON bytes).
/// A file that is not a DXF (a DWG, a binary DXF, broken groups) throws the reason.
#[wasm_bindgen(js_name = readDxf)]
pub fn read_dxf(bytes: &[u8], options: &str) -> Result<Vec<u8>, JsError> {
    let opts: DxfReadOptions =
        serde_json::from_str(options).map_err(|e| bad_input("Okuma seçenekleri", &e))?;
    let result = kentos_formats::dxf::read(bytes, &opts).map_err(|e| JsError::new(&e))?;
    to_json(&result)
}

/// Writes a coordinate list from `CoordWriteInput` (JSON).
#[wasm_bindgen(js_name = writeCoords)]
pub fn write_coords(input: &str) -> Result<Written, JsError> {
    let input: CoordWriteInput =
        serde_json::from_str(input).map_err(|e| bad_input("Yazılacak noktalar", &e))?;
    let (bytes, report) = kentos_formats::coords::write(&input);
    Ok(Written {
        bytes,
        report: String::from_utf8(to_json(&report)?).unwrap_or_default(),
    })
}

/// The persistent ids of a v1 drawing's objects (`V1Identities`, JSON bytes;
/// docs/adr/0014), from the drawing's text: derived from its content, so the
/// same file gets the same ids here, in the desktop app and on the server. A
/// drawing the contract cannot read throws the reason.
#[wasm_bindgen(js_name = v1Identities)]
pub fn v1_identities(text: &str) -> Result<Vec<u8>, JsError> {
    let ids = kentos_contracts::v1_identities(text).map_err(|e| JsError::new(&e))?;
    to_json(&ids)
}

/// A drawing (`DocumentSnapshotV2`, JSON) as the bytes of a `.kcad` v2 file
/// (docs/specs/kcad-v2.md). The formats worker reads them back and compares
/// them with what the page sent before the page writes them (io/kcad.ts).
/// A drawing the file cannot hold (a NaN, a repeated id) throws the reason.
#[wasm_bindgen(js_name = encodeKcad)]
pub fn encode_kcad(snapshot: &str) -> Result<Vec<u8>, JsError> {
    let doc: DocumentSnapshotV2 =
        serde_json::from_str(snapshot).map_err(|e| bad_input("Kaydedilecek çizim", &e))?;
    kentos_kcad::encode(&doc).map_err(|e| JsError::new(&e.message))
}

/// Reads a `.kcad` v2 file. Never throws for a bad file: the result (JSON
/// bytes) is `{"ok":true,"document":DocumentSnapshotV2}` or
/// `{"ok":false,"code":…,"message":…}` with the specification's error code
/// (§9) and a Turkish message that says the cause and the fix.
#[wasm_bindgen(js_name = decodeKcad)]
pub fn decode_kcad(bytes: &[u8]) -> Result<Vec<u8>, JsError> {
    #[derive(serde::Serialize)]
    #[serde(untagged)]
    enum Read<'a> {
        Ok {
            ok: bool,
            document: &'a DocumentSnapshotV2,
        },
        Refused {
            ok: bool,
            code: &'a str,
            message: &'a str,
        },
    }
    // Written straight from the typed drawing: no JSON value tree in between.
    match kentos_kcad::decode(bytes) {
        Ok(document) => to_json(&Read::Ok {
            ok: true,
            document: &document,
        }),
        Err(e) => to_json(&Read::Refused {
            ok: false,
            code: e.code.as_str(),
            message: &e.message,
        }),
    }
}

/// Writes an AutoCAD 2007 DXF from `DxfWriteInput` (JSON). The objects are
/// read by the formats crate's own visitor (`dxf::WriteInput`), not the
/// contract's derived code, which would weigh ~100 KB in this module.
#[wasm_bindgen(js_name = writeDxf)]
pub fn write_dxf(input: &str) -> Result<Written, JsError> {
    let input: kentos_formats::dxf::WriteInput =
        serde_json::from_str(input).map_err(|e| bad_input("Yazılacak nesneler", &e))?;
    let (bytes, report) = kentos_formats::dxf::write(&input.0);
    Ok(Written {
        bytes,
        report: String::from_utf8(to_json(&report)?).unwrap_or_default(),
    })
}
