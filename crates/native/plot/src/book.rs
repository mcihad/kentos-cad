//! The project's sheet book as a PDF without a window (docs/adr/0252 §3):
//! what the headless host (Python, MCP) writes for `export_pdf`, as the
//! desktop's Dışa aktar › PDF writes it — the sheets' display lists with the
//! same inputs (the project, its system, the tables, coordinate lists and
//! legends of [`crate::inputs`]), each placed map frame as the shown layers'
//! paths at its scale ([`crate::maps`]), the drawing's faces, GeoPDF where the
//! project's system is a transverse Mercator one.
//!
//! What a window draws and a headless host cannot is said, never faked:
//!
//! - the drawing's texts on a map (dimension values, text objects, labels)
//!   are laid out by the desktop's text system: a headless map has its paths
//!   and not its texts (`notes`);
//! - a map with something no vector writes (a pattern or picture fill, a
//!   picture marker) goes to the desktop's PDF as a picture; here it is the
//!   core's empty map box, the layers named (`notes`);
//! - an SVG picture of a sheet needs a rasterizer: the core's own finding
//!   (`svg_not_in_pdf`) says it is drawn as an empty box.

use kentos_contracts::{LayerNodeType, ProjectSettings};
use kentos_domain::Document;
use kentos_interaction::Spatial;
use kentos_native_style::library::{Source, StyleLibrary};
use kentos_sheet::display::{self, CrsInfo, Prim, ProjectInfo, RenderInputs, RenderMode};
use kentos_sheet::model::SheetBook;
use kentos_sheet::pdf::{PdfAsset, PdfInputs, PdfMap, PdfOptions};
use kentos_sheet::preflight::Finding;
use kentos_sheet::profile::Capabilities;
use kentos_sheet::project::read_project_sheets;

use crate::inputs::{crs_info, pdf_crs, sheet_data, sheet_variable};
use crate::maps::{MapSource, vector_content};

/// A PDF of the book, the sheets it holds in order, and what it writes
/// differently from the window's.
#[derive(Debug)]
pub struct BookPdf {
    pub bytes: Vec<u8>,
    /// The sheets written, a page each, by id.
    pub sheets: Vec<String>,
    /// The core's findings (an SVG picture drawn as a box …).
    pub findings: Vec<Finding>,
    /// What this host leaves out (the maps' texts, a map with no vector form).
    pub notes: Vec<String>,
}

/// Why a book PDF was not written: a code a program decides by and the words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlotError {
    pub code: String,
    pub message: String,
}

impl PlotError {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

/// The project's system as the sheet's title block names it; none for a local project.
fn crs_name(settings: &ProjectSettings) -> Option<String> {
    if !settings.has_system() {
        return None;
    }
    Some(match &settings.custom_crs {
        Some(d) if settings.srid == kentos_project::crs::LOCAL_SRID => d.name.clone(),
        _ => kentos_project::crs::system(settings.srid)
            .map_or_else(|| format!("EPSG:{}", settings.srid), |s| s.name.clone()),
    })
}

/// The style library a drawing's maps are built with: the system's symbols and the project's own.
pub fn library_of(doc: &Document) -> StyleLibrary {
    let mut library = kentos_native_style::system::library();
    let own = doc.styles();
    library.load(Source::Project, &own.items, &own.categories);
    library
}

/// What every sheet is drawn with besides its own data: the project, its system, the date and
/// the user the title block writes (`date` as `2026-10-13`).
fn base_inputs(doc: &Document, user: &str, date: &str) -> RenderInputs {
    let settings = doc.settings();
    let name = crs_name(settings);
    RenderInputs {
        mode: RenderMode::Export,
        project: ProjectInfo {
            name: doc.name().to_owned(),
            user: user.to_owned(),
            date: date.to_owned(),
            crs_name: name.clone().unwrap_or_default(),
            // The drawing's own variables, read after the book's (docs/adr/0214 §2.3).
            variables: settings.variables.iter().map(sheet_variable).collect(),
        },
        capabilities: Capabilities {
            georeferenced: name.is_some(),
            attribute_layers: doc
                .layers()
                .leaves()
                .iter()
                .any(|l| l.kind == LayerNodeType::Layer && !l.fields.is_empty()),
            plot_scale: Some(settings.plot_scale.round())
                .filter(|s| *s >= 1.0)
                .map(|s| s as u32),
        },
        crs: name.map(|n| {
            crs_info(settings.srid)
                .filter(|_| settings.srid != kentos_project::crs::LOCAL_SRID)
                .unwrap_or(CrsInfo { name: n, tm: None })
        }),
        ..RenderInputs::default()
    }
}

/// Today as the title block writes it (`2026-10-13`), from the system clock (UTC).
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    // A day number since 1970-01-01 as a civil date (H. Hinnant's algorithm).
    let z = i64::try_from(secs / 86_400).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The book's chosen sheets (none: all, in the book's order) as a PDF of the drawing `doc`.
pub fn book_pdf(
    doc: &Document,
    sheets: &[String],
    user: &str,
    date: &str,
) -> Result<BookPdf, PlotError> {
    let Some(kept) = doc.sheets() else {
        return Err(PlotError::new(
            "no_sheets",
            "Projede pafta yok: önce bir pafta ekleyin (cad.sheets.edit ya da Pafta kipi).",
        ));
    };
    let (book, pictures): (SheetBook, _) =
        read_project_sheets(kept).map_err(|e| PlotError::new(&e.code, e.message))?;
    let chosen: Vec<String> = if sheets.is_empty() {
        book.sheets.iter().map(|s| s.id.clone()).collect()
    } else {
        sheets.to_vec()
    };
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    let library = library_of(doc);
    let mut render = base_inputs(doc, user, date);
    // Each sheet's tables, coordinate lists and legends, from the ground its maps cover.
    for id in &chosen {
        let Some(sheet) = book.sheet(id) else {
            continue;
        };
        let list = display::display_list(&book, id, &render).ok();
        let data = sheet_data(doc, &spatial, &library, &[], &book, sheet, list.as_ref());
        render.tables.extend(data.tables);
        render.coordinates.extend(data.coordinates);
        render.legends.extend(data.legends);
    }
    // The drawing area's own local origin (`scene_origin`): the anchor, else the middle of the drawing.
    let anchor = doc.origin();
    let origin = if anchor.x.is_finite() && anchor.y.is_finite() {
        kentos_geometry_core::Vec2::new(anchor.x, anchor.y)
    } else {
        spatial
            .extent()
            .map_or(kentos_geometry_core::Vec2::new(0.0, 0.0), |b| {
                kentos_geometry_core::Vec2::new(
                    (b.min_x + b.max_x) / 2.0,
                    (b.min_y + b.max_y) / 2.0,
                )
            })
    };
    let src = MapSource {
        doc,
        store: spatial.store(),
        library: &library,
        origin,
    };
    let mut maps: Vec<PdfMap> = Vec::new();
    let mut notes = Vec::new();
    let mut texts_left = false;
    for id in &chosen {
        let list = display::display_list(&book, id, &render)
            .map_err(|e| PlotError::new(&e.code, e.message))?;
        for p in &list.prims {
            let Prim::Map(m) = p else {
                continue;
            };
            if m.view.center.is_none() || maps.iter().any(|x| x.item == m.item) {
                continue;
            }
            let Some(extent) = m.extent else {
                continue;
            };
            texts_left |= doc.entities().any(|e| {
                matches!(
                    e,
                    kentos_contracts::Entity::Text(_) | kentos_contracts::Entity::Dimension(_)
                ) || e.base().label.is_some()
            });
            match vector_content(&src, &m.layers, m.view.scale, extent, &[]) {
                Ok(content) => maps.push(PdfMap {
                    item: m.item.clone(),
                    content,
                    corners: None,
                }),
                Err(unsupported) => {
                    let named: Vec<String> = unsupported
                        .iter()
                        .map(|(layer, what)| format!("{layer} ({})", what.join(", ")))
                        .collect();
                    notes.push(format!(
                        "“{}” haritası vektörle yazılamadı: {}. Başsız çıktıda boş harita kutusu basıldı; masaüstünün PDF'i onu resim olarak yazar.",
                        book.item(&m.item).map_or(m.item.as_str(), |i| i.name.as_str()),
                        named.join("; ")
                    ));
                }
            }
        }
    }
    if texts_left {
        notes.push(
            "Haritaların yazıları (ölçü değerleri, yazılar, etiketler) başsız çıktıda henüz basılmaz: \
             çizgiler, alanlar ve taramalar basılır. Yazılarıyla PDF için masaüstünün Dışa aktar › PDF'ini kullanın."
                .to_owned(),
        );
    }
    let assets = pictures
        .into_iter()
        .map(|(sha256, data)| PdfAsset {
            sha256,
            data,
            raster: None,
        })
        .collect();
    let srid = doc.settings().srid;
    let inputs = PdfInputs {
        render,
        fonts: crate::fonts::pdf_fonts(),
        assets,
        maps,
        crs: pdf_crs(srid),
    };
    let options = PdfOptions {
        sheets: chosen.clone(),
        geo: inputs.crs.is_some(),
        layers: true,
    };
    let bytes = kentos_sheet::pdf::to_pdf(&book, &inputs, &options)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    let findings = kentos_sheet::pdf::findings(&book, &inputs, &options)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    Ok(BookPdf {
        bytes,
        sheets: chosen,
        findings,
        notes,
    })
}
