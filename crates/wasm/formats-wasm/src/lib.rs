//! The file formats in the browser. The formats Web Worker
//! (`apps/web/src/io/formatsWorker.ts`) loads this module the first time the user
//! imports or exports a file, or opens or saves a drawing (the `.kcad` v2
//! codec, `kentos-kcad`; a v1 drawing's persistent ids); it never loads at
//! start-up (CLAUDE.md §20).
//! Files cross as bytes; options and results as JSON (the contracts in
//! `kentos_contracts::formats`), whose float64 values serde_json writes as
//! the shortest round-trip decimal, so coordinates arrive bit for bit. A
//! drawing to save or one read crosses as typed columns (docs/adr/0030).

use kentos_contracts::{CoordReadOptions, CoordWriteInput, DxfReadOptions, FORMATS_VERSION};
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

// ── The project file, `.kcad` v2 (docs/specs/kcad-v2.md, docs/adr/0025, 0030) ──
// The drawing crosses as typed columns (`kentos_kcad::columns`), not as JSON:
// the objects in six typed arrays the worker and the page hand each other
// without a copy, the rest (name, settings, layers, styles) as the contract's
// JSON. Long reads and writes report their stages to the worker, which posts
// them to the page; the page stops one by stopping the worker.

#[wasm_bindgen]
extern "C" {
    /// Hears a long read or write (`io/kcad.ts` gives a plain object with these two methods).
    pub type KcadProgress;

    /// A stage and how far it is: `checking` (bytes), `reading`, `writing` (objects), `verifying`.
    #[wasm_bindgen(method)]
    fn step(this: &KcadProgress, stage: &str, done: f64, total: f64);

    /// The project, known before its objects are read: name, top-level layers, objects.
    #[wasm_bindgen(method)]
    fn project(this: &KcadProgress, name: &str, layers: u32, objects: u32);
}

/// The codec's steps, told to the page's side.
struct Relay<'p>(&'p KcadProgress);

impl kentos_kcad::Watch for Relay<'_> {
    fn step(&mut self, s: kentos_kcad::Step<'_>) -> bool {
        use kentos_kcad::Step;
        let n = |x: usize| x as f64;
        match s {
            Step::Checking { done, total } => self.0.step("checking", done as f64, total as f64),
            Step::Project {
                name,
                layers,
                objects,
            } => self.0.project(
                name,
                u32::try_from(layers).unwrap_or(u32::MAX),
                u32::try_from(objects).unwrap_or(u32::MAX),
            ),
            Step::Reading { done, total } => self.0.step("reading", n(done), n(total)),
            Step::Writing { done, total } => self.0.step("writing", n(done), n(total)),
            Step::Verifying => self.0.step("verifying", 0.0, 0.0),
        }
        // Stopping is the page's: it ends the worker (a call here cannot be interrupted).
        true
    }
}

/// What a KCAD call gives back: a refusal (`ok` false, the specification's
/// `code` and a Turkish `message`), or the bytes written, or the drawing read
/// (its head as JSON and its objects as columns). Each array is taken once,
/// so it is copied out of the module once.
#[wasm_bindgen]
#[derive(Default)]
pub struct Kcad {
    ok: bool,
    code: String,
    message: String,
    bytes: Vec<u8>,
    head: String,
    columns: kentos_kcad::Columns,
}

impl Kcad {
    fn refused(e: &kentos_kcad::KcadError) -> Self {
        Self {
            code: e.code.as_str().to_owned(),
            message: e.message.clone(),
            ..Self::default()
        }
    }
}

#[wasm_bindgen]
impl Kcad {
    #[wasm_bindgen(getter)]
    pub fn ok(&self) -> bool {
        self.ok
    }

    #[wasm_bindgen(getter)]
    pub fn code(&self) -> String {
        self.code.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// The drawing without its objects (the contract's JSON; `entities` and `uids` empty).
    #[wasm_bindgen(js_name = takeHead)]
    pub fn take_head(&mut self) -> String {
        std::mem::take(&mut self.head)
    }

    #[wasm_bindgen(js_name = takeBytes)]
    pub fn take_bytes(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.bytes)
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

/// A drawing as the bytes of a `.kcad` v2 file: `head` is the contract's JSON
/// of everything but the objects, the arrays its objects as typed columns.
/// The bytes are read back and compared with the columns as they came, every
/// float bit for bit, before they are given (`kentos_kcad::encode_columns`);
/// the head as the file holds it comes too, for the worker to compare with
/// the head it sent. A drawing the file cannot hold is refused with the reason.
#[allow(clippy::too_many_arguments)]
#[wasm_bindgen(js_name = encodeKcad)]
pub fn encode_kcad(
    head: &str,
    kinds: Vec<u8>,
    uids: Vec<u8>,
    ints: Vec<u32>,
    floats: Vec<f64>,
    text: Vec<u16>,
    text_lengths: Vec<u32>,
    progress: &KcadProgress,
) -> Kcad {
    let columns = kentos_kcad::Columns {
        kinds,
        uids,
        ints,
        floats,
        text,
        text_lengths,
    };
    match kentos_kcad::encode_columns(head, &columns, &mut Relay(progress)) {
        Ok((bytes, head)) => Kcad {
            ok: true,
            bytes,
            head,
            ..Kcad::default()
        },
        Err(e) => Kcad::refused(&e),
    }
}

/// Reads a `.kcad` v2 file into its head and its objects as columns. Never
/// throws for a bad file: a refusal says the specification's code (§9) and a
/// Turkish message with the cause and the fix. The file's bytes go as soon as
/// the drawing is read, and each object once it is packed, so a large file is
/// not in memory three times.
#[wasm_bindgen(js_name = decodeKcad)]
pub fn decode_kcad(bytes: Vec<u8>, progress: &KcadProgress) -> Kcad {
    let read = kentos_kcad::decode_watched(&bytes, &mut Relay(progress));
    drop(bytes);
    match read.and_then(kentos_kcad::split) {
        Ok((head, columns)) => Kcad {
            ok: true,
            head,
            columns,
            ..Kcad::default()
        },
        Err(e) => Kcad::refused(&e),
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
