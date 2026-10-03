//! PDF and GeoPDF (design §9a): the sheets of a book as one PDF file, one
//! page a sheet at the paper's exact size, everything the core draws as
//! vectors, the drawing's TrueType faces embedded as subsets with a
//! `ToUnicode` map (Turkish text is selectable and searchable), pictures
//! as they are (JPEG) or Flate with a soft mask (PNG), an SVG picture as the
//! PNG the host draws of it (the core draws no SVG; [`svg_sizes`] says at
//! what size, [`findings`] says it was done), each map's content from the
//! host (the drawing's vectors, one optional content group a layer, or a
//! picture), and a geographic viewport per map frame (GDAL, QGIS and Acrobat
//! read coordinates off the map).
//!
//! The same inputs give the same bytes on every target: no clock is read
//! (the date is the project's), the file's id is a digest of its content,
//! and every number is written by the same code in the browser and on the
//! desktop.

mod content;
mod fonts;
mod geo;
mod images;
mod maps;
mod write;

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

pub use geo::{TmCrs, tm_wkt};

use crate::display::{Prim, RenderInputs};
use crate::error::{Result, SheetError};
use crate::model::{AssetKind, ItemId, SheetBook, SheetId};
use crate::preflight::{Finding, Severity};
use crate::style::{LineCap, LineJoin};

/// Bytes crossing JSON as base64 (fonts, pictures, a map's PNG).
pub(crate) mod b64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&crate::template::base64_encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(d)?;
        crate::template::base64_decode(&text)
            .ok_or_else(|| serde::de::Error::custom("base64 değil"))
    }
}

/// Optional bytes crossing JSON as base64.
pub(crate) mod b64_opt {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match bytes {
            Some(b) => s.serialize_str(&crate::template::base64_encode(b)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        match Option::<String>::deserialize(d)? {
            Some(text) => crate::template::base64_decode(&text)
                .map(Some)
                .ok_or_else(|| serde::de::Error::custom("base64 değil")),
            None => Ok(None),
        }
    }
}

/// One of the drawing's faces: the table's id (`TextPrim.font`, `DRAWING_FONTS`), weight and slant.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PdfFace {
    pub font: String,
    pub weight: u16,
    #[serde(default)]
    pub italic: bool,
}

/// A face's TrueType file (the desktop's `apps/desktop/assets/fonts/drawing/*.ttf`; the web loads
/// the same files): `data` is base64 in JSON.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PdfFont {
    pub font: String,
    pub weight: u16,
    #[serde(default)]
    pub italic: bool,
    #[serde(with = "b64")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub data: Vec<u8>,
}

/// A picture's bytes (the book keeps only its metadata): base64 in JSON.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PdfAsset {
    pub sha256: String,
    #[serde(with = "b64")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub data: Vec<u8>,
    /// For an SVG picture: a PNG the host drew of it (the core draws no SVG), at the size
    /// [`svg_sizes`] gives for the PDF's resolution; the PDF embeds it in the picture's place,
    /// as it embeds a PNG. Base64 in JSON. Without it an SVG picture is the missing picture's
    /// box, and [`findings`] warns of it.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "b64_opt")]
    #[cfg_attr(feature = "ts", ts(optional, type = "string"))]
    pub raster: Option<Vec<u8>>,
}

/// The pixels an SVG picture is drawn at for a PDF (`PdfAsset.raster`): its largest frame on the
/// sheets going, at the resolution asked for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PdfSvgSize {
    pub sha256: String,
    pub width: u32,
    pub height: u32,
}

/// A line of the drawing on a map: its colour, its width and dashes on the paper (millimetres).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapStroke {
    /// `#rrggbb` or `#rrggbbaa`.
    pub color: String,
    /// On the paper, millimetres; 0: the thinnest line the device draws.
    pub width: f64,
    /// Dash and gap lengths on the paper, millimetres, alternating; empty: solid.
    #[serde(default)]
    pub dash: Vec<f64>,
    #[serde(default = "round_cap")]
    pub cap: LineCap,
    #[serde(default = "round_join")]
    pub join: LineJoin,
}

fn round_cap() -> LineCap {
    LineCap::Round
}

fn round_join() -> LineJoin {
    LineJoin::Round
}

/// A path of the drawing: ground points `[east (Y), north (X)]` in metres, the map's own system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapPath {
    pub points: Vec<[f64; 2]>,
    /// A ring (the last point joins the first).
    #[serde(default)]
    pub closed: bool,
    /// Rings cut out of a closed path's fill (an area's holes), even-odd.
    #[serde(default)]
    pub holes: Vec<Vec<[f64; 2]>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub stroke: Option<MapStroke>,
    /// A closed path's fill, `#rrggbb` or `#rrggbbaa`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub fill: Option<String>,
}

/// Where a map text's point is on its line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum TextAnchor {
    #[default]
    LeftBaseline,
    CenterBaseline,
    RightBaseline,
    LeftMiddle,
    CenterMiddle,
    RightMiddle,
}

/// A text of the drawing on a map (a parcel number, an ada's name, a label): its point on the
/// ground, its size on the paper.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapText {
    /// `[east (Y), north (X)]`, metres.
    pub at: [f64; 2],
    pub text: String,
    /// The letters' size (the em) on the paper, millimetres.
    pub size: f64,
    /// Degrees counter-clockwise from the map's east: 0 reads along the map's own x axis; the
    /// map's turn and the frame's are added.
    #[serde(default)]
    pub rotation: f64,
    #[serde(default = "black")]
    pub color: String,
    /// A face of the table (`barlow`, `arimo` …); an unknown one is written as barlow.
    #[serde(default = "barlow")]
    pub font: String,
    #[serde(default = "regular")]
    pub weight: u16,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub anchor: TextAnchor,
    /// A light outline under the letters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub halo: Option<String>,
}

fn black() -> String {
    "#000000".to_owned()
}

fn barlow() -> String {
    "barlow".to_owned()
}

fn regular() -> u16 {
    400
}

/// A layer of the drawing on a map: one optional content group of the PDF (its name in the
/// viewer's layer list), shared by every map and page that shows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct MapLayerContent {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub paths: Vec<MapPath>,
    #[serde(default)]
    pub texts: Vec<MapText>,
}

/// A map frame's content, drawn in its frame, cut to it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub enum MapContent {
    /// The drawing as vectors: the core turns the ground into the paper by the map's view.
    Vector(VectorMap),
    /// The fallback: a PNG of the frame's content box (unturned, north of the view up), drawn
    /// over it; base64 in JSON.
    Raster(RasterMap),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct VectorMap {
    /// Bottom first: the drawing order.
    pub layers: Vec<MapLayerContent>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct RasterMap {
    #[serde(with = "b64")]
    #[cfg_attr(feature = "ts", ts(type = "string"))]
    pub png: Vec<u8>,
}

/// A map frame's content and, for GeoPDF in a system the core cannot invert, its corners.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PdfMap {
    pub item: ItemId,
    pub content: MapContent,
    /// Latitude and longitude (degrees) of the content box's corners: top left, top right,
    /// bottom right, bottom left. Only where `RenderInputs.crs.tm` is none (the core works them
    /// out for a transverse Mercator system itself).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub corners: Option<[[f64; 2]; 4]>,
}

/// The maps' coordinate system for GeoPDF: the measure's `GCS`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PdfCrs {
    /// WKT 1 of the system (`PROJCS[…]`, or `GEOGCS[…]` for a geographic one); `tmWkt` builds
    /// it for a transverse Mercator system from the registry's values.
    pub wkt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub epsg: Option<u32>,
}

/// What a PDF is made of besides the book.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PdfInputs {
    /// What the display lists are built with (variables, tables, legends, the project, its
    /// system with its TM values …): the same as for the screen.
    #[serde(default)]
    pub render: RenderInputs,
    /// The faces the sheets write with (`pdfFonts` lists them).
    #[serde(default)]
    pub fonts: Vec<PdfFont>,
    /// The pictures' bytes; a picture without them is drawn as a missing one (the preflight says so).
    #[serde(default)]
    pub assets: Vec<PdfAsset>,
    /// Each map frame's content; a map without it is the light “harita” box.
    #[serde(default)]
    pub maps: Vec<PdfMap>,
    /// For GeoPDF; none: no geographic viewport is written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub crs: Option<PdfCrs>,
}

/// How the PDF is written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct PdfOptions {
    /// The sheets, in this order, a page each; empty: every sheet in the book's order.
    #[serde(default)]
    pub sheets: Vec<SheetId>,
    /// A geographic viewport (GeoPDF) for every placed map frame, when the inputs have a system.
    #[serde(default = "yes")]
    pub geo: bool,
    /// A vector map's layers as optional content groups (the viewer's layer list).
    #[serde(default = "yes")]
    pub layers: bool,
}

fn yes() -> bool {
    true
}

impl Default for PdfOptions {
    fn default() -> Self {
        PdfOptions {
            sheets: Vec::new(),
            geo: true,
            layers: true,
        }
    }
}

/// The sheets a PDF writes, in its order: the options' or the book's.
fn chosen_sheets(book: &SheetBook, options: &PdfOptions) -> Result<Vec<SheetId>> {
    if options.sheets.is_empty() {
        if book.sheets.is_empty() {
            return Err(SheetError::new(
                "pdf_no_sheets",
                "Kitapta pafta yok: PDF'e yazılacak bir pafta ekleyin.",
            ));
        }
        return Ok(book.sheets.iter().map(|s| s.id.clone()).collect());
    }
    for id in &options.sheets {
        if book.sheet(id).is_none() {
            return Err(SheetError::at(
                "pdf_unknown_sheet",
                id.clone(),
                format!("“{id}” kitapta yok: PDF'e yazılacak paftaları yeniden seçin."),
            ));
        }
    }
    Ok(options.sheets.clone())
}

/// The faces the chosen sheets (and the maps' texts) write with: the TrueType files the host
/// passes in `PdfInputs.fonts`. Sorted, each once.
pub fn fonts_needed(
    book: &SheetBook,
    inputs: &PdfInputs,
    options: &PdfOptions,
) -> Result<Vec<PdfFace>> {
    let sheets = chosen_sheets(book, options)?;
    write::faces_of(book, inputs, &sheets)
}

/// The book's chosen sheets as a PDF file.
pub fn to_pdf(book: &SheetBook, inputs: &PdfInputs, options: &PdfOptions) -> Result<Vec<u8>> {
    let sheets = chosen_sheets(book, options)?;
    write::document(book, inputs, options, &sheets)
}

/// An SVG picture's longest side drawn for a PDF, pixels at most (a larger frame is drawn smaller).
pub const SVG_MAX_SIDE: u32 = 8192;

/// Whether a picture of the book is an SVG one.
fn is_svg(book: &SheetBook, sha256: &str) -> bool {
    book.assets
        .iter()
        .any(|a| a.sha256 == sha256 && a.kind == AssetKind::Svg)
}

/// The SVG pictures of the chosen sheets and the pixels each is drawn at for `PdfAsset.raster`
/// (design §9a): its largest frame at `dpi`, the longest side at most [`SVG_MAX_SIDE`]. Each
/// picture once, by its SHA-256.
pub fn svg_sizes(
    book: &SheetBook,
    inputs: &PdfInputs,
    options: &PdfOptions,
    dpi: u16,
) -> Result<Vec<PdfSvgSize>> {
    let sheets = chosen_sheets(book, options)?;
    let lists = write::lists(book, inputs, &sheets)?;
    let px = |um: i32| (f64::from(um.max(0)) / 25_400.0 * f64::from(dpi.max(1))).ceil();
    let mut out: std::collections::BTreeMap<String, (f64, f64)> = std::collections::BTreeMap::new();
    for l in &lists {
        for p in &l.prims {
            if let Prim::Image(i) = p
                && is_svg(book, &i.asset)
            {
                let (w, h) = (px(i.rect.width).max(1.0), px(i.rect.height).max(1.0));
                let e = out.entry(i.asset.clone()).or_insert((0.0, 0.0));
                // The larger frame's: an SVG is drawn once and shown at every size it has.
                if w * h > e.0 * e.1 {
                    *e = (w, h);
                }
            }
        }
    }
    Ok(out
        .into_iter()
        .map(|(sha256, (w, h))| {
            let k = (f64::from(SVG_MAX_SIDE) / w.max(h)).min(1.0);
            PdfSvgSize {
                sha256,
                width: ((w * k).round() as u32).max(1),
                height: ((h * k).round() as u32).max(1),
            }
        })
        .collect())
}

/// What the PDF writes differently from the screen and the sheet's own preflight cannot know (it
/// depends on what the host passes): each SVG picture of the chosen sheets embedded as the PNG
/// the host drew of it (`svg_as_picture`, information), or, without one, drawn as the missing
/// picture's box (`svg_not_in_pdf`, a warning). Once a picture item, in the pages' order.
pub fn findings(
    book: &SheetBook,
    inputs: &PdfInputs,
    options: &PdfOptions,
) -> Result<Vec<Finding>> {
    let sheets = chosen_sheets(book, options)?;
    let lists = write::lists(book, inputs, &sheets)?;
    let mut out: Vec<Finding> = Vec::new();
    for l in &lists {
        for p in &l.prims {
            let Prim::Image(i) = p else {
                continue;
            };
            if !is_svg(book, &i.asset) || out.iter().any(|f| f.item.as_ref() == Some(&i.item)) {
                continue;
            }
            let who = book
                .item(&i.item)
                .map_or_else(|| i.item.clone(), crate::preflight::named);
            let drawn = inputs
                .assets
                .iter()
                .find(|a| a.sha256 == i.asset)
                .and_then(|a| a.raster.as_deref())
                .and_then(images::picture);
            out.push(match drawn {
                Some(pic) => Finding {
                    severity: Severity::Info,
                    code: "svg_as_picture".into(),
                    item: Some(i.item.clone()),
                    message: format!(
                        "{who}: SVG resim PDF'e resim olarak gömülür ({} × {} piksel).",
                        pic.width, pic.height
                    ),
                    fix: "Daha keskin bir resim için PDF'i daha yüksek çözünürlükle yazın.".into(),
                    fixes: Vec::new(),
                    placeholder: false,
                },
                None => Finding {
                    severity: Severity::Warning,
                    code: "svg_not_in_pdf".into(),
                    item: Some(i.item.clone()),
                    message: format!(
                        "{who}: SVG resim PDF'e çizilemedi; yerine boş resim kutusu basıldı."
                    ),
                    fix: "Resmin PNG ya da JPEG sürümünü seçin.".into(),
                    fixes: Vec::new(),
                    placeholder: false,
                },
            });
        }
    }
    Ok(out)
}
