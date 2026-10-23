//! The project's sheet book as a PDF without a window (docs/adr/0252 §3):
//! what the headless host (Python, MCP) writes for `export_pdf`, as the
//! desktop's Dışa aktar › PDF writes it — the sheets' display lists with the
//! same inputs (the project, its system, the tables, coordinate lists and
//! legends of [`crate::inputs`]), each placed map frame as the shown layers'
//! paths at its scale ([`crate::maps`]), the drawing's faces, GeoPDF where the
//! project's system is a transverse Mercator one.
//!
//! Map text is laid out by the shared spatial/paragraph engine with bundled faces.
//! Rich map styles use the native CPU twin of the GPU shaders; SVG assets use
//! the bundled-font SVG rasterizer. Missing image sources fail the whole export.

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
use crate::maps::{MapSource, vector_content_profile};

/// A PDF of the book, the sheets it holds in order, and what it writes
/// differently from the window's.
#[derive(Debug)]
pub struct BookPdf {
    pub bytes: Vec<u8>,
    /// The sheets written, a page each, by id.
    pub sheets: Vec<String>,
    /// The shared core's preflight findings.
    pub findings: Vec<Finding>,
    /// Findings retain page ownership, including a shared master item on several pages.
    pub sheet_findings: Vec<(String, Finding)>,
    /// Rasterization choices, target resolution and explicit resource limits.
    pub notes: Vec<String>,
}

/// Why a book PDF was not written: a code a program decides by and the words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlotError {
    pub code: String,
    pub message: String,
}

impl PlotError {
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

/// The project's system as the sheet's title block names it; none for a local project.
pub(crate) fn crs_name(settings: &ProjectSettings) -> Option<String> {
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
pub(crate) fn base_inputs(doc: &Document, user: &str, date: &str) -> RenderInputs {
    let settings = doc.settings();
    let name = crs_name(settings);
    RenderInputs {
        mode: RenderMode::Export,
        named_views: settings.named_views.clone(),
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

/// Draws the maps of a book's pages as vectors (the book PDF's and the atlas's): each map of a display
/// list its content at its scale, what could not be said told in `finish`.
pub(crate) struct MapDrawer<'a> {
    doc: &'a Document,
    spatial: &'a Spatial,
    library: &'a StyleLibrary,
    /// The moment the temporal layers show at (docs/adr/0534 §3); none: no time cut.
    window: Option<kentos_geometry_core::time::Window>,
    /// The drawing as the maps at a scale draw it (docs/adr/0323 §2): a copy per scale where an annotative
    /// object draws otherwise there.
    annotative: bool,
    scaled: std::collections::HashMap<(u64, u64), Option<(Document, Spatial)>>,
    notes: Vec<String>,
    errors: Vec<String>,
    drawing: Option<String>,
    pub(crate) raster_only: bool,
}

impl<'a> MapDrawer<'a> {
    pub(crate) fn new(
        doc: &'a Document,
        spatial: &'a Spatial,
        library: &'a StyleLibrary,
        window: Option<kentos_geometry_core::time::Window>,
    ) -> Self {
        Self {
            doc,
            spatial,
            library,
            window,
            annotative: doc.has_annotative(),
            scaled: std::collections::HashMap::new(),
            notes: Vec::new(),
            errors: Vec::new(),
            drawing: None,
            raster_only: false,
        }
    }

    /// One map's content, or none for a map with no ground (an unplaced one).
    pub(crate) fn draw(
        &mut self,
        book: &SheetBook,
        m: &kentos_sheet::display::MapPrim,
        sheet: &str,
    ) -> Option<PdfMap> {
        let doc = self.doc;
        let extent = m.extent?;
        let center = m.view.center?;
        let origin = kentos_geometry_core::Vec2::new(center.x, center.y);
        let at = m.view.scale.max(1e-6);
        let annotation = m.view.annotation_denominator();
        let key = (at.to_bits(), annotation.to_bits());
        // Walls at a detail other than the drawing's (docs/adr/0339 §4) want a copy of their own too.
        let walls = Spatial::walls_differ_at(doc, at);
        if (self.annotative || walls) && !self.scaled.contains_key(&key) {
            let copy = self
                .annotative
                .then(|| doc.at_scale(annotation))
                .flatten()
                .or_else(|| walls.then(|| doc.reading_copy()));
            let window = self.window;
            let copy = copy.map(|copy| {
                let mut spatial = Spatial::new();
                spatial.set_wall_scale(Some(at));
                spatial.reload(&copy);
                spatial.set_time_window(window);
                (copy, spatial)
            });
            self.scaled.insert(key, copy);
        }
        let own;
        let src = match self.scaled.get(&key) {
            Some(Some((copy, spatial))) => {
                own = MapSource {
                    doc: copy,
                    store: spatial.store(),
                    library: self.library,
                    origin,
                };
                &own
            }
            _ => &MapSource {
                doc,
                store: self.spatial.store(),
                library: self.library,
                origin,
            },
        };
        let label_spatial = match self.scaled.get(&key) {
            Some(Some((_, spatial))) => spatial,
            _ => self.spatial,
        };
        let labels = crate::labels::map_labels(src.doc, label_spatial, m);
        let vector = if self.raster_only {
            Err(Vec::new())
        } else {
            vector_content_profile(
                src,
                &m.layers,
                &m.layer_overrides,
                m.view.scale,
                extent,
                &labels,
                kentos_sheet::plot_profile::for_sheet(book, sheet)
                    .ok()
                    .flatten(),
            )
        };
        let content = match vector {
            Ok(content) => content,
            Err(unsupported) => {
                let dpi = book.sheet(sheet).map_or(300, |s| s.export.dpi);
                match crate::raster::map(
                    src,
                    m,
                    &labels,
                    dpi,
                    self.drawing.as_deref(),
                    kentos_sheet::plot_profile::for_sheet(book, sheet)
                        .ok()
                        .flatten(),
                ) {
                    Ok(png) => {
                        self.notes.push(format!("“{}” haritası {} dpi hedefiyle resim olarak yazıldı (en çok 8192 px/32 MP): {}.",book.item(&m.item).map_or(m.item.as_str(),|i|i.name.as_str()),dpi,
                            unsupported.iter().map(|(name,why)|format!("{name} ({})",why.join(", "))).collect::<Vec<_>>().join("; ")));
                        kentos_sheet::pdf::MapContent::Raster(png)
                    }
                    Err(e) => {
                        self.errors.push(e);
                        return None;
                    }
                }
            }
        };
        Some(PdfMap {
            sheet: None,
            item: m.item.clone(),
            content,
            corners: None,
        })
    }

    /// What the headless host leaves out, said once.
    pub(crate) fn finish(self) -> Result<Vec<String>, PlotError> {
        if !self.errors.is_empty() {
            return Err(PlotError::new(
                "plot_asset_unreadable",
                self.errors.join(" "),
            ));
        }
        Ok(self.notes)
    }
}

/// The book's chosen sheets (none: all, in the book's order) as a PDF of the drawing `doc`.
pub fn book_pdf(
    doc: &Document,
    sheets: &[String],
    user: &str,
    date: &str,
) -> Result<BookPdf, PlotError> {
    book_pdf_at(doc, sheets, user, date, None)
}

/// As book_pdf, with the drawing's file for linked image resolution.
pub fn book_pdf_at(
    doc: &Document,
    sheets: &[String],
    user: &str,
    date: &str,
    drawing: Option<&str>,
) -> Result<BookPdf, PlotError> {
    book_pdf_options(doc, sheets, user, date, drawing, None)
}

/// Every native host uses this preparation and writer; options change selection/presentation only.
pub fn book_pdf_options(
    doc: &Document,
    sheets: &[String],
    user: &str,
    date: &str,
    drawing: Option<&str>,
    options: Option<kentos_sheet::pdf::PdfOptions>,
) -> Result<BookPdf, PlotError> {
    book_pdf_controlled(doc, sheets, user, date, drawing, options, None)
}

/// A cancellable batch uses the same scene and writer; progress advances at page boundaries.
pub fn book_pdf_controlled(
    doc: &Document,
    sheets: &[String],
    user: &str,
    date: &str,
    drawing: Option<&str>,
    options: Option<kentos_sheet::pdf::PdfOptions>,
    progress: Option<&kentos_native_application::jobs::Progress>,
) -> Result<BookPdf, PlotError> {
    let (book, mut inputs, defaults, notes) =
        prepare(doc, sheets, user, date, drawing, false, progress)?;
    let options = options.unwrap_or(defaults);
    crate::images::svg_assets(
        &book,
        &mut inputs,
        &options,
        book.sheets
            .iter()
            .map(|s| s.export.dpi)
            .max()
            .unwrap_or(300),
    )
    .map_err(|e| PlotError::new("plot_asset_unreadable", e))?;
    check_stop(progress)?;
    let bytes = kentos_sheet::pdf::to_pdf(&book, &inputs, &options)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    check_stop(progress)?;
    let mut findings = kentos_sheet::pdf::findings(&book, &inputs, &options)
        .map_err(|e| PlotError::new(&e.code, e.message))?;
    let mut sheet_findings = Vec::new();
    for id in &options.sheets {
        let page = kentos_sheet::pdf::PdfOptions {
            sheets: vec![id.clone()],
            ..options.clone()
        };
        let mut own = kentos_sheet::pdf::findings(&book, &inputs, &page)
            .map_err(|e| PlotError::new(&e.code, e.message))?;
        for finding in kentos_sheet::preflight::preflight(&book, id, &inputs.render)
            .map_err(|e| PlotError::new(&e.code, e.message))?
        {
            if finding.code != "svg_not_in_pdf"
                && !own.iter().any(|f| {
                    f.code == finding.code
                        && f.item == finding.item
                        && f.severity == finding.severity
                })
            {
                own.push(finding);
            }
        }
        for finding in own {
            if !findings.iter().any(|f| {
                f.code == finding.code && f.item == finding.item && f.severity == finding.severity
            }) {
                findings.push(finding.clone());
            }
            sheet_findings.push((id.clone(), finding));
        }
    }
    Ok(BookPdf {
        bytes,
        sheets: options.sheets,
        findings,
        sheet_findings,
        notes,
    })
}

pub(crate) fn prepare(
    doc: &Document,
    sheets: &[String],
    user: &str,
    date: &str,
    drawing: Option<&str>,
    raster_only: bool,
    progress: Option<&kentos_native_application::jobs::Progress>,
) -> Result<(SheetBook, PdfInputs, PdfOptions, Vec<String>), PlotError> {
    check_stop(progress)?;
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
    for id in &chosen {
        check_stop(progress)?;
        let findings = kentos_sheet::preflight::preflight(&book, id, &render)
            .map_err(|e| PlotError::new(&e.code, e.message))?;
        if let Some(f) = findings.iter().find(|f| {
            matches!(
                f.code.as_str(),
                "missing_named_view" | "bad_scale" | "bad_value"
            )
        }) {
            return Err(PlotError::new(&f.code, &f.message));
        }
    }
    // Each sheet's tables, coordinate lists and legends, from the ground its maps cover.
    for id in &chosen {
        check_stop(progress)?;
        let Some(sheet) = book.sheet(id) else {
            continue;
        };
        let list = display::display_list(&book, id, &render).ok();
        let data = sheet_data(doc, &spatial, &library, &[], &book, sheet, list.as_ref());
        render.tables.extend(data.tables);
        render.coordinates.extend(data.coordinates);
        render.legends.extend(data.legends);
    }
    if chosen.is_empty() {
        return Err(PlotError::new("no_sheets", "Pafta kitabında pafta yok."));
    }
    let mut drawer = MapDrawer::new(doc, &spatial, &library, None);
    drawer.raster_only = raster_only;
    drawer.drawing = drawing.map(str::to_owned);
    let mut maps: Vec<PdfMap> = Vec::new();
    for (page, id) in chosen.iter().enumerate() {
        check_stop(progress)?;
        if let Some(p) = progress {
            p.set(page as u64, chosen.len() as u64);
        }
        let list = display::display_list(&book, id, &render)
            .map_err(|e| PlotError::new(&e.code, e.message))?;
        for p in &list.prims {
            let Prim::Map(m) = p else {
                continue;
            };
            if m.view.center.is_none()
                || maps
                    .iter()
                    .any(|x| x.item == m.item && x.sheet.as_ref() == Some(id))
            {
                continue;
            }
            if let Some(mut map) = drawer.draw(&book, m, id) {
                map.sheet = Some(id.clone());
                maps.push(map);
            }
        }
    }
    if let Some(p) = progress {
        p.set(chosen.len() as u64, chosen.len() as u64);
    }
    check_stop(progress)?;
    let notes = drawer.finish()?;
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
        atlas: None,
        keywords: None,
    };
    let options = PdfOptions {
        sheets: chosen.clone(),
        geo: inputs.crs.is_some(),
        layers: true,
    };
    Ok((book, inputs, options, notes))
}

fn check_stop(
    progress: Option<&kentos_native_application::jobs::Progress>,
) -> Result<(), PlotError> {
    if progress.is_some_and(|p| p.stopped()) {
        Err(PlotError::new("cancelled", "Pafta çıktısı iptal edildi."))
    } else {
        Ok(())
    }
}

/// Native host default for an unsigned/local print job; callers may supply an explicit author.
pub fn current_user() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default()
}
