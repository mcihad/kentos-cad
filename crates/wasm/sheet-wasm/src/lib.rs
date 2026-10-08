//! The sheet core in the browser (docs/sheet/design.md, tasks-rust.md
//! Step 2): a package of its own, loaded the first time the sheet mode
//! opens (CLAUDE.md §20), never at the app's start.
//!
//! Calls cross as JSON, in the shapes of `apps/web/src/contracts/generated/sheet/`.
//! Every answer is a JSON envelope:
//!
//! ```text
//! {"ok":true,"value":<the result>}
//! {"ok":false,"error":{"code":"<stable code>","message":"<Turkish>","path":"<where>"?}}
//! ```
//!
//! An input that is not the JSON it should be is such an error too
//! (`bad_json`): a template read from a file is user input. Nothing throws
//! but the `SnapSession` constructor (a JS `Error` whose message is
//! `code: message`). serde_json writes a float as its shortest round-trip
//! decimal, so numbers arrive bit for bit; integers are micrometres and
//! thousandths of a degree, as in the core.

use kentos_contracts::{ProjectType, Workspace};
use kentos_sheet::display::{AtlasFeature, DisplayList, RenderInputs};
use kentos_sheet::hit::HitQuery;
use kentos_sheet::ops::{Handle, Op};
use kentos_sheet::pdf::{PdfInputs, PdfOptions, TmCrs};
use kentos_sheet::profile::{Capabilities, NewItem};
use kentos_sheet::svg::SvgOptions;
use kentos_sheet::sync::{LocalTemplate, RemoteTemplate};
use kentos_sheet::template::{
    AssetWithBytes, InstanceIds, InstanceOptions, Template, TemplateMeta,
};
use kentos_sheet::{Item, ItemId, Result, SheetBook, SheetError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct Value<'a, T: Serialize> {
    ok: bool,
    value: &'a T,
}

#[derive(Serialize)]
struct Refusal<'a> {
    ok: bool,
    error: &'a SheetError,
}

fn refusal(e: &SheetError) -> String {
    serde_json::to_string(&Refusal {
        ok: false,
        error: e,
    })
    .unwrap_or_else(|_| {
        // A `SheetError` is three strings; writing it cannot fail. Should it, the code still crosses.
        format!(
            r#"{{"ok":false,"error":{{"code":"{}","message":"Hata yazılamadı."}}}}"#,
            e.code
        )
    })
}

/// The envelope of a result.
fn answer<T: Serialize>(r: Result<T>) -> String {
    match r {
        Ok(v) => serde_json::to_string(&Value {
            ok: true,
            value: &v,
        })
        .unwrap_or_else(|e| {
            refusal(&SheetError::new(
                "bad_output",
                format!("Sonuç yazılamadı: {e}"),
            ))
        }),
        Err(e) => refusal(&e),
    }
}

/// An argument read strictly (an unknown field is an error, as everywhere in the core).
fn read<T: DeserializeOwned>(what: &str, json: &str) -> Result<T> {
    serde_json::from_str(json).map_err(|e| SheetError::json(what, &e))
}

/// A book as the engine's own answers give it (its rules are checked by `readBook` when it is loaded).
fn book(json: &str) -> Result<SheetBook> {
    read("Pafta kitabı", json)
}

fn workspace(json: &str) -> Result<Option<Workspace>> {
    read("Çalışma kipi", json)
}

// ── What the engine is ───────────────────────────────────────────────────

/// `EngineInfo`: the core's version, schemas, typefaces, placeholder codes, nudges and tolerance.
#[wasm_bindgen(js_name = engineInfo)]
pub fn engine_info() -> String {
    answer(Ok(kentos_sheet::info::engine_info()))
}

/// `PaperSize[]`: the ISO A and B sizes, portrait, in micrometres.
#[wasm_bindgen(js_name = paperSizes)]
pub fn paper_sizes() -> String {
    answer(Ok(kentos_sheet::paper::paper_sizes()))
}

/// `number[]`: the standard scales' denominators, smallest first.
#[wasm_bindgen(js_name = standardScales)]
pub fn standard_scales() -> String {
    answer(Ok(kentos_sheet::paper::standard_scales()))
}

/// `number | null`: the scale a fixed map's frame (`RectUm`) needs to show `w` × `h` metres of
/// ground with its content turned `rotation` m° (Görünüme sığdır, docs/adr/0206 §1).
#[wasm_bindgen(js_name = fitViewScale)]
pub fn fit_view_scale(w: f64, h: f64, rotation: i32, frame: &str) -> String {
    answer(
        read::<kentos_sheet::units::RectUm>("frame", frame)
            .map(|f| kentos_sheet::atlas::fit_view_scale(w, h, rotation, &f)),
    )
}

/// `string`: what a coordinate list's source (`CoordSource`) gives, as its section says it, from
/// the host's input for the list (`CoordinateInput`, or an empty text for none) and the source
/// layer's name (docs/adr/0206 §2).
#[wasm_bindgen(js_name = coordinateSummary)]
pub fn coordinate_summary(source: &str, input: &str, layer: &str) -> String {
    let input = if input.trim().is_empty() {
        Ok(None)
    } else {
        read::<kentos_sheet::display::CoordinateInput>("input", input).map(Some)
    };
    answer(
        read::<kentos_sheet::kinds::CoordSource>("source", source).and_then(|s| {
            input.map(|i| {
                kentos_sheet::display::coordinate_summary(
                    &s,
                    i.as_ref(),
                    (!layer.is_empty()).then_some(layer),
                )
            })
        }),
    )
}

/// `[string, string, string, string, string]`: a coordinate list's headings (Nokta, east, north,
/// Z) and its area row's word, each given or the default (docs/adr/0206 §3); `item` the list's
/// kind as an item holds it (its `type` too).
#[wasm_bindgen(js_name = coordinateHeadings)]
pub fn coordinate_headings(item: &str, georeferenced: bool) -> String {
    answer(
        read::<serde_json::Value>("item", item)
            .and_then(|mut v| {
                if let Some(o) = v.as_object_mut() {
                    o.remove("type");
                }
                serde_json::from_value::<kentos_sheet::kinds::CoordinateListItem>(v)
                    .map_err(|e| SheetError::json("item", &e))
            })
            .map(|c| kentos_sheet::display::coordinate_headings(&c, georeferenced)),
    )
}

/// `Bindable[]`: the item properties an expression can drive (ƒ), with their types.
#[wasm_bindgen(js_name = bindableProperties)]
pub fn bindable_properties() -> String {
    answer(Ok(kentos_sheet::bind::bindable_properties()))
}

// ── Books and templates ──────────────────────────────────────────────────

/// `SheetBook`: a stored book read, checked against every rule and normalised (as a load does).
#[wasm_bindgen(js_name = readBook)]
pub fn read_book(book: &str) -> String {
    answer(kentos_sheet::validate::read_book(book))
}

/// `string`: a `.kpafta` file's text: the book (checked) and the bytes given of its pictures
/// (`AssetWithBytes[]`; one the book does not list is left out, one whose bytes do not match is an error).
/// The one codec of the format (`kentos_sheet::kpafta`), the desktop's too.
#[wasm_bindgen(js_name = encodeKpafta)]
pub fn encode_kpafta(book_json: &str, assets: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let assets: Vec<AssetWithBytes> = read("Resimler", assets)?;
        kentos_sheet::kpafta::encode(&b, &assets)
    })())
}

/// `KpaftaFile`: a `.kpafta` file's text read: its format and version, its book read as `readBook`
/// reads one, every picture held to its metadata (`bad_json`, `unknown_schema`, `newer_schema`, `bad_asset`, a book's own codes).
#[wasm_bindgen(js_name = decodeKpafta)]
pub fn decode_kpafta(text: &str) -> String {
    answer(kentos_sheet::kpafta::decode(text))
}

/// `string`: the SHA-256 of a book's canonical JSON, the same on every platform (a cache key, a change test).
#[wasm_bindgen(js_name = bookDigest)]
pub fn book_digest(book_json: &str) -> String {
    answer(book(book_json).and_then(|b| {
        serde_json::to_string(&b)
            .map(|s| kentos_sheet::template::sha256_hex(s.as_bytes()))
            .map_err(|e| SheetError::new("bad_output", format!("Kitap yazılamadı: {e}")))
    }))
}

/// `Template[]`: the program's ten templates (read-only, `sys:` ids).
#[wasm_bindgen(js_name = systemTemplates)]
pub fn system_templates() -> String {
    let errors = kentos_sheet::template::system_template_errors();
    match errors.first() {
        // The crate's tests read every one; a broken one is a build defect, said rather than hidden.
        Some((_, e)) => refusal(e),
        None => answer(Ok(kentos_sheet::template::system_templates())),
    }
}

/// `Template`: a user's template read (an older schema migrated), checked and normalised.
#[wasm_bindgen(js_name = validateTemplate)]
pub fn validate_template(template: &str) -> String {
    answer(kentos_sheet::template::read_template(template))
}

/// A template given to `instantiateTemplate` (a system one, or a user's `validateTemplate` gave), checked again.
fn template(json: &str) -> Result<Template> {
    let t: Template = read("Şablon", json)?;
    kentos_sheet::template::validate_template(&t, t.meta.id.starts_with("sys:"))?;
    Ok(t)
}

/// `Instance`: a new sheet (and master) from a template: `ids` (`InstanceIds`) are the host's, `options` (`InstanceOptions`) the paper, place, scale and answers.
#[wasm_bindgen(js_name = instantiateTemplate)]
pub fn instantiate_template(template_json: &str, ids: &str, options: &str) -> String {
    answer((|| {
        let t = template(template_json)?;
        let ids: InstanceIds = read("Kimlikler", ids)?;
        let options: InstanceOptions = read("Şablon seçenekleri", options)?;
        kentos_sheet::template::instantiate(&t, &ids, &options)
    })())
}

/// `Template`: a sheet saved as a template with `meta` (`TemplateMeta`); `assets` (`AssetWithBytes[]`) are the bytes of the pictures it uses.
#[wasm_bindgen(js_name = extractTemplate)]
pub fn extract_template(book_json: &str, sheet: &str, meta: &str, assets: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let meta: TemplateMeta = read("Şablon bilgileri", meta)?;
        let assets: Vec<AssetWithBytes> = read("Şablonun resimleri", assets)?;
        kentos_sheet::template::extract(&b, sheet, &meta, &assets)
    })())
}

// ── Operations ───────────────────────────────────────────────────────────

/// `Applied`: the book after one operation (`Op`), the operations that take it back, and the undo step's name.
#[wasm_bindgen(js_name = applyOp)]
pub fn apply_op(book_json: &str, op: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let op: Op = read("İşlem", op)?;
        kentos_sheet::ops::apply(&b, &op)
    })())
}

/// `Applied`: several operations (`Op[]`) as one undo step; none is applied if one fails.
#[wasm_bindgen(js_name = applyOps)]
pub fn apply_ops(book_json: &str, ops: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let ops: Vec<Op> = read("İşlemler", ops)?;
        kentos_sheet::ops::apply_all(&b, &ops)
    })())
}

/// `LayoutVariant | null`: the layout variant a paper (`Page`) would put in force on a sheet or master page (`Owner`); null: its base layout (design §3.2a). The one in force is `activeVariant` on the sheet.
#[wasm_bindgen(js_name = variantFor)]
pub fn variant_for(book_json: &str, owner: &str, page: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let owner: kentos_sheet::Owner = read("Sahip", owner)?;
        let page: kentos_sheet::Page = read("Kâğıt", page)?;
        kentos_sheet::variants::variant_for(&b, &owner, &page).map(|v| v.cloned())
    })())
}

// ── Pointer ──────────────────────────────────────────────────────────────

/// `Hits`: the items under a point, inside a rectangle, or a selected item's handle (`HitQuery`).
#[wasm_bindgen(js_name = hitTest)]
pub fn hit_test(book_json: &str, sheet: &str, query: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let q: HitQuery = read("İsabet sorgusu", query)?;
        kentos_sheet::hit::hit_test(&b, sheet, &q)
    })())
}

/// A drag's snapping, computed once when it starts and asked on every pointer move.
#[wasm_bindgen]
pub struct SnapSession(kentos_sheet::snap::SnapSession);

#[wasm_bindgen]
impl SnapSession {
    /// The session for dragging `moving` (`string[]`, item ids) on a sheet; `options` (`SnapOptions`; `{}` or "" for all on).
    /// Throws an `Error` whose message is `code: message` when the book or the ids are wrong.
    #[wasm_bindgen(constructor)]
    pub fn new(
        book_json: &str,
        sheet: &str,
        moving: &str,
        options: &str,
    ) -> std::result::Result<SnapSession, JsError> {
        let open = || -> Result<SnapSession> {
            let b = book(book_json)?;
            let moving: Vec<ItemId> = read("Taşınan öğeler", moving)?;
            let options = if options.trim().is_empty() {
                kentos_sheet::snap::SnapOptions::default()
            } else {
                read("Yapışma seçenekleri", options)?
            };
            kentos_sheet::snap::SnapSession::new(&b, sheet, &moving, options).map(SnapSession)
        };
        open().map_err(|e| JsError::new(&format!("{}: {}", e.code, e.message)))
    }

    /// `SnapResult`: the drag's offset (micrometres) snapped within `tolerance` (micrometres), with the lines, gaps and spacing marks to paint.
    pub fn query(&self, dx: i32, dy: i32, tolerance: i32) -> String {
        answer(Ok(self.0.query([dx, dy], tolerance)))
    }

    /// `ResizeSnap`: the one moving item's handle (`"n"`, `"ne"`, … `"nw"`) dragged to (x, y), snapped.
    #[wasm_bindgen(js_name = queryResize)]
    pub fn query_resize(
        &self,
        handle: &str,
        x: i32,
        y: i32,
        tolerance: i32,
        keep_aspect: bool,
    ) -> String {
        answer(
            serde_json::from_value::<Handle>(serde_json::Value::String(handle.to_owned()))
                .map_err(|e| SheetError::json("Tutamaç", &e))
                .map(|h| self.0.query_resize(h, [x, y], tolerance, keep_aspect)),
        )
    }

    /// `RectUm`: the box the moving items make together at the drag's start.
    #[wasm_bindgen(js_name = movingBox)]
    pub fn moving_box(&self) -> String {
        answer(Ok(self.0.moving_box()))
    }
}

/// A rotation (thousandths of a degree) snapped: with `step` (Shift) to 15°, otherwise to a quarter turn within 2°.
#[wasm_bindgen(js_name = snapRotation)]
pub fn snap_rotation(angle: i32, step: bool) -> i32 {
    kentos_sheet::snap::snap_rotation(angle, step)
}

// ── Drawing ──────────────────────────────────────────────────────────────

/// `DisplayList`: what to paint for a sheet, with the host's `inputs` (`RenderInputs`; `{}` for none).
#[wasm_bindgen(js_name = displayList)]
pub fn display_list(book_json: &str, sheet: &str, inputs: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let inputs: RenderInputs = read("Çizim girdileri", inputs)?;
        kentos_sheet::display::display_list(&b, sheet, &inputs)
    })())
}

/// `string`: a sheet's export file name (design §9): its `export.fileName` (`[% … %]` text) written
/// in the sheet's scope, a fixed name as it is, the sheet's name when it writes nothing. The host
/// takes out what its file system refuses.
#[wasm_bindgen(js_name = exportName)]
pub fn export_name(book_json: &str, sheet: &str, inputs: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let inputs: RenderInputs = read("Çizim girdileri", inputs)?;
        kentos_sheet::display::export_name(&b, sheet, &inputs)
    })())
}

/// `NorthInfo`: what a north arrow of a sheet shows and from what (design §8a): its map's centre
/// as latitude and longitude, the convergence, the declination with its source (`model`/`hand`),
/// the model, the date used and where it came from (`sheet`/`project`/`today`), and whether that
/// date is inside the model's years. `inputs` are `displayList`'s.
#[wasm_bindgen(js_name = northInfo)]
pub fn north_info(book_json: &str, sheet: &str, item: &str, inputs: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let inputs: RenderInputs = read("Çizim girdileri", inputs)?;
        kentos_sheet::display::north_info(&b, sheet, item, &inputs)
    })())
}

/// `PaperPalette`: the colours a map's content is drawn in on a sheet (white paper, black ink,
/// the drawing's text #111111), on the screen and in a PDF alike.
#[wasm_bindgen(js_name = paperPalette)]
pub fn paper_palette() -> String {
    answer(Ok::<_, SheetError>(kentos_sheet::display::paper_palette()))
}

/// `string`: a display list (`DisplayList`) as an SVG document; `options` (`SvgOptions`) give the maps' and pictures' links.
#[wasm_bindgen(js_name = toSvg)]
pub fn to_svg(list: &str, options: &str) -> String {
    answer((|| {
        let list: DisplayList = read("Çizim planı", list)?;
        let options: SvgOptions = read("SVG seçenekleri", options)?;
        Ok(kentos_sheet::svg::to_svg(&list, &options))
    })())
}

/// `Finding[]`: what is wrong with a sheet before export, errors first.
#[wasm_bindgen]
pub fn preflight(book_json: &str, sheet: &str, inputs: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let inputs: RenderInputs = read("Çizim girdileri", inputs)?;
        kentos_sheet::preflight::preflight(&b, sheet, &inputs)
    })())
}

/// `AtlasPlan`: the atlas pages of a sheet for the coverage layer's objects (`AtlasFeature[]`).
#[wasm_bindgen(js_name = atlasPlan)]
pub fn atlas_plan(book_json: &str, sheet: &str, features: &str) -> String {
    answer((|| {
        let b = book(book_json)?;
        let features: Vec<AtlasFeature> = read("Atlas nesneleri", features)?;
        kentos_sheet::atlas::atlas_plan(&b, sheet, &features)
    })())
}

/// `null` when an expression (an ƒ binding, an atlas filter) compiles; otherwise an `expression_error` with the reason.
#[wasm_bindgen(js_name = checkExpression)]
pub fn check_expression(source: &str) -> String {
    answer(
        kentos_sheet::expr::check(source)
            .map(|()| serde_json::Value::Null)
            .map_err(|m| SheetError::new("expression_error", m)),
    )
}

// ── PDF and GeoPDF (design §9a) ──────────────────────────────────────────

fn pdf_args(
    book_json: &str,
    inputs: &str,
    options: &str,
) -> Result<(SheetBook, PdfInputs, PdfOptions)> {
    let b = book(book_json)?;
    let inputs: PdfInputs = if inputs.trim().is_empty() {
        PdfInputs::default()
    } else {
        read("PDF girdileri", inputs)?
    };
    let options: PdfOptions = if options.trim().is_empty() {
        PdfOptions::default()
    } else {
        read("PDF seçenekleri", options)?
    };
    Ok((b, inputs, options))
}

/// The book's chosen sheets as a PDF file (`inputs`: `PdfInputs`, `options`: `PdfOptions`;
/// "" or "{}" for the defaults). The file's bytes; throws an `Error` whose message is
/// `code: message` when it cannot be written (a binary answer has no JSON envelope).
#[wasm_bindgen(js_name = toPdf)]
pub fn to_pdf(
    book_json: &str,
    inputs: &str,
    options: &str,
) -> std::result::Result<Vec<u8>, JsError> {
    pdf_args(book_json, inputs, options)
        .and_then(|(b, i, o)| kentos_sheet::pdf::to_pdf(&b, &i, &o))
        .map_err(|e| JsError::new(&format!("{}: {}", e.code, e.message)))
}

/// `PdfFace[]`: the faces the chosen sheets and the maps' texts write with, whose TrueType
/// files `toPdf` needs in `inputs.fonts` (their `data` may be empty here).
#[wasm_bindgen(js_name = pdfFonts)]
pub fn pdf_fonts(book_json: &str, inputs: &str, options: &str) -> String {
    answer(
        pdf_args(book_json, inputs, options)
            .and_then(|(b, i, o)| kentos_sheet::pdf::fonts_needed(&b, &i, &o)),
    )
}

/// `PdfSvgSize[]`: the SVG pictures of the chosen sheets and the pixels the host draws each at
/// for `PdfAsset.raster` (its largest frame at `dpi`): the core draws no SVG.
#[wasm_bindgen(js_name = pdfSvgSizes)]
pub fn pdf_svg_sizes(book_json: &str, inputs: &str, options: &str, dpi: u16) -> String {
    answer(
        pdf_args(book_json, inputs, options)
            .and_then(|(b, i, o)| kentos_sheet::pdf::svg_sizes(&b, &i, &o, dpi)),
    )
}

/// `Finding[]`: what the PDF writes differently from the screen, by what the inputs hold: an SVG
/// picture embedded as the host's PNG (`svg_as_picture`, information) or, without one, as the
/// missing picture's box (`svg_not_in_pdf`, a warning).
#[wasm_bindgen(js_name = pdfFindings)]
pub fn pdf_findings(book_json: &str, inputs: &str, options: &str) -> String {
    answer(
        pdf_args(book_json, inputs, options)
            .and_then(|(b, i, o)| kentos_sheet::pdf::findings(&b, &i, &o)),
    )
}

/// `string`: the WKT 1 of a transverse Mercator system (`TmCrs`, the CRS registry's values),
/// for `PdfCrs.wkt`.
#[wasm_bindgen(js_name = tmWkt)]
pub fn tm_wkt(crs: &str) -> String {
    answer(read::<TmCrs>("Koordinat sistemi", crs).map(|c| kentos_sheet::pdf::tm_wkt(&c)))
}

/// `MagneticField`: the World Magnetic Model's field (design §8a, WMM2025) at a geodetic latitude
/// and longitude (degrees, WGS84), a height above the ellipsoid (km) and a decimal year:
/// declination and inclination (degrees), X, Y, Z, H, F (nT).
#[wasm_bindgen(js_name = magneticField)]
pub fn magnetic_field(lat: f64, lon: f64, height_km: f64, year: f64) -> String {
    answer(
        kentos_sheet::wmm::field(lat, lon, height_km, year).ok_or_else(|| {
            SheetError::new(
                "magnetic_bad_place",
                "Manyetik alan hesaplanamadı: enlem −90…90, sayılar sonlu olmalı.",
            )
        }),
    )
}

/// `WmmInfo`: the magnetic model's name, release and the years it is valid for.
#[wasm_bindgen(js_name = wmmInfo)]
pub fn wmm_info() -> String {
    answer(
        kentos_sheet::wmm::info()
            .ok_or_else(|| SheetError::new("wmm_missing", "Manyetik modelin tablosu okunamadı.")),
    )
}

/// `number`: an ISO date (`2026-10-03`) as the decimal year the magnetic model takes.
#[wasm_bindgen(js_name = decimalYear)]
pub fn decimal_year(iso: &str) -> String {
    answer(kentos_sheet::wmm::decimal_year(iso).ok_or_else(|| {
        SheetError::new("bad_date", format!("“{iso}” bir tarih değil (YYYY-AA-GG)."))
    }))
}

/// `TextRun[]`: a line of text in the pieces the drawing's faces draw (design §6, “Eksik
/// karakter”): a letter its face lacks in the first drawing face that has it, one no face has
/// as “?”. The PDF and SVG writers write a text so; a screen that does the same matches them.
#[wasm_bindgen(js_name = textRuns)]
pub fn text_runs(font: &str, weight: u16, italic: bool, text: &str) -> String {
    answer(Ok::<_, SheetError>(kentos_sheet::text::text_runs(
        font, weight, italic, text,
    )))
}

// ── Templates in the cloud ───────────────────────────────────────────────

/// `SyncPlan`: what to download, upload, delete or keep (`LocalTemplate[]`, `RemoteTemplate[]`).
#[wasm_bindgen(js_name = planSync)]
pub fn plan_sync(local: &str, remote: &str) -> String {
    answer((|| {
        let local: Vec<LocalTemplate> = read("Bu cihazın şablonları", local)?;
        let remote: Vec<RemoteTemplate> = read("Bulutun şablonları", remote)?;
        Ok(kentos_sheet::sync::plan_sync(&local, &remote))
    })())
}

// ── Work modes (design §11a) ─────────────────────────────────────────────

/// `Profile`: the ribbon of a work mode (`workspace`: JSON, `"cad"` or `null`) for a project's `Capabilities`.
#[wasm_bindgen(js_name = profileFor)]
pub fn profile_for(workspace_json: &str, capabilities: &str) -> String {
    answer((|| {
        let w = workspace(workspace_json)?;
        let caps: Capabilities = read("Proje yetenekleri", capabilities)?;
        Ok(kentos_sheet::profile::profile_for(w, &caps))
    })())
}

/// `ToolInfo[]`: every tool of the mode, flat, each shown, shown but disabled (with the reason) or hidden.
#[wasm_bindgen(js_name = toolAvailability)]
pub fn tool_availability(workspace_json: &str, capabilities: &str) -> String {
    answer((|| {
        let w = workspace(workspace_json)?;
        let caps: Capabilities = read("Proje yetenekleri", capabilities)?;
        Ok(kentos_sheet::profile::tool_availability(w, &caps))
    })())
}

/// `RankedTemplate[]`: the gallery's order of `TemplateMeta[]` for a project's mode and type (`"subdivision"` or `null`).
#[wasm_bindgen(js_name = rankTemplates)]
pub fn rank_templates(metas: &str, workspace_json: &str, project_type: &str) -> String {
    answer((|| {
        let metas: Vec<TemplateMeta> = read("Şablon bilgileri", metas)?;
        let w = workspace(workspace_json)?;
        let p: Option<ProjectType> = read("Proje türü", project_type)?;
        Ok(kentos_sheet::profile::rank_templates(&metas, w, p))
    })())
}

/// `Item`: a new item from a tool of the mode (`NewItem`: tool, preset, id, name, frame, linked map).
#[wasm_bindgen(js_name = newItem)]
pub fn new_item(workspace_json: &str, capabilities: &str, request: &str) -> String {
    answer((|| {
        let w = workspace(workspace_json)?;
        let caps: Capabilities = read("Proje yetenekleri", capabilities)?;
        let req: NewItem = read("Yeni öğe", request)?;
        kentos_sheet::profile::new_item(w, &caps, &req)
    })())
}

/// `string | null`: the inspector's note for an item another mode's tool made (edited here, not added here).
#[wasm_bindgen(js_name = itemNote)]
pub fn item_note(workspace_json: &str, item: &str) -> String {
    answer((|| {
        let w = workspace(workspace_json)?;
        let item: Item = read("Öğe", item)?;
        Ok(kentos_sheet::profile::item_note(w, &item))
    })())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(s: &str) -> serde_json::Value {
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["ok"], true, "{s}");
        v["value"].clone()
    }

    fn error(s: &str) -> serde_json::Value {
        let v: serde_json::Value = serde_json::from_str(s).unwrap();
        assert_eq!(v["ok"], false, "{s}");
        v["error"].clone()
    }

    #[test]
    fn answers_are_envelopes() {
        assert_eq!(value(&engine_info())["bookSchema"], "kentos.sheet/1");
        assert_eq!(value(&standard_scales())[0], 100);
        assert_eq!(value(&system_templates()).as_array().unwrap().len(), 10);
        assert_eq!(error(&apply_op("{", "{}"))["code"], "bad_json");
        assert_eq!(
            error(&read_book(
                r#"{"schema":"kentos.sheet/1","sheets":[],"zoom":1}"#
            ))["code"],
            "bad_json"
        );
        assert_eq!(value(&check_expression("1 + 2")), serde_json::Value::Null);
        assert_eq!(error(&check_expression("1 +"))["code"], "expression_error");
        assert_eq!(snap_rotation(16_000, true), 15_000);
    }

    #[test]
    fn a_kpafta_file_goes_round_through_the_binding() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/sheet/v1/kpafta/valid/buro-paftasi.kpafta"
        ))
        .unwrap();
        let file = value(&decode_kpafta(&text));
        assert_eq!(
            (
                file["format"].as_str(),
                file["assets"].as_array().map(Vec::len)
            ),
            (Some("kentos.sheet.file"), Some(1))
        );
        let written = value(&encode_kpafta(
            &file["book"].to_string(),
            &file["assets"].to_string(),
        ));
        assert_eq!(value(&decode_kpafta(written.as_str().unwrap())), file);
        assert_eq!(error(&decode_kpafta("{"))["code"], "bad_json");
        assert_eq!(error(&encode_kpafta("{}", "[]"))["code"], "bad_json");
    }

    #[test]
    fn a_template_becomes_a_sheet() {
        let t = serde_json::to_string(
            kentos_sheet::template::system_template("sys:genel-a4-dikey").unwrap(),
        )
        .unwrap();
        let tv: serde_json::Value = serde_json::from_str(&t).unwrap();
        let items: Vec<String> = tv["sheet"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| format!("n-{}", i["id"].as_str().unwrap()))
            .collect();
        let ids = serde_json::json!({ "sheet": "s9", "items": items, "masterItems": [] });
        let inst = value(&instantiate_template(&t, &ids.to_string(), "{}"));
        assert_eq!(inst["sheet"]["id"], "s9");
        let book = serde_json::json!({ "schema": "kentos.sheet/1", "sheets": [inst["sheet"]] })
            .to_string();
        let read = value(&read_book(&book));
        assert_eq!(read["sheets"][0]["page"]["paper"], "a4");
        let list = value(&display_list(&book, "s9", "{}"));
        let svg = value(&to_svg(&list.to_string(), "{}"));
        assert!(svg.as_str().unwrap().starts_with("<?xml"));
        assert_eq!(value(&book_digest(&book)).as_str().unwrap().len(), 64);
        let findings = value(&preflight(&book, "s9", "{}"));
        assert!(
            findings
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["code"] == "map_unplaced")
        );
        // The template is A4 portrait; on landscape its landscape layout comes into force.
        let owner = r#"{"kind":"sheet","id":"s9"}"#;
        let a4 = |o: &str, w: i32, h: i32| {
            serde_json::json!({ "paper": "a4", "orientation": o, "size": { "width": w, "height": h },
                                "margins": { "top": 10000, "right": 10000, "bottom": 10000, "left": 20000 } })
            .to_string()
        };
        assert_eq!(
            value(&variant_for(
                &book,
                owner,
                &a4("landscape", 297_000, 210_000)
            ))["id"],
            "yatay"
        );
        let op = serde_json::json!({ "op": "setPage", "owner": { "kind": "sheet", "id": "s9" }, "page": serde_json::from_str::<serde_json::Value>(&a4("landscape", 297_000, 210_000)).unwrap() });
        let applied = value(&apply_op(&book, &op.to_string()));
        assert_eq!(applied["book"]["sheets"][0]["activeVariant"], "yatay");
        assert_eq!(applied["inverse"][0]["op"], "replaceSheet");
    }

    #[test]
    fn modes_and_the_gallery() {
        let p = value(&profile_for("\"cad\"", "{}"));
        assert_eq!(p["id"], "cad");
        let tools = value(&tool_availability("null", r#"{"georeferenced":true}"#));
        assert!(tools.as_array().unwrap().iter().any(|t| t["id"] == "map"));
        assert_eq!(error(&profile_for("\"kip\"", "{}"))["code"], "bad_json");
        let metas: Vec<_> = kentos_sheet::template::system_templates()
            .iter()
            .map(|t| t.meta.clone())
            .collect();
        let ranked = value(&rank_templates(
            &serde_json::to_string(&metas).unwrap(),
            "\"gis\"",
            "\"gis\"",
        ));
        assert_eq!(ranked[0]["id"], "sys:gis-tematik");
        let item = value(&new_item(
            "\"cad\"",
            r#"{"plotScale":500}"#,
            r#"{"tool":"map","id":"h","name":"Görünüm","frame":{"left":10000,"top":10000,"width":100000,"height":80000}}"#,
        ));
        assert_eq!(item["kind"]["view"]["scale"], 500);
        let overview = serde_json::json!({
            "id": "o", "name": "Genel bakış",
            "frame": { "left": 0, "top": 0, "width": 10000, "height": 10000 },
            "kind": { "type": "map", "view": { "type": "fixed", "scale": 5000 }, "overviewOf": "h" }
        });
        let note = value(&item_note("\"cad\"", &overview.to_string()));
        assert!(note.as_str().unwrap().contains("CBS projelerinin aracıdır"));
        let plan = value(&plan_sync(
            r#"[{"id":"t","name":"A","baseRevision":1}]"#,
            r#"[{"id":"t","name":"A","revision":2}]"#,
        ));
        assert_eq!(plan["actions"][0]["type"], "download");
    }
}
