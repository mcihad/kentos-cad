//! A sheet PDF on the desktop (docs/sheet/design.md §9a): its maps as the
//! drawing's vectors — every shown layer through the style engine at the
//! map's scale on the paper's palette (the web's map frames: line weights,
//! line types, point symbols and hatches as styled), turned into paths
//! (map_vectors.rs), and the drawing's text as its map frames write it
//! (labels.rs, anchored as the web anchors it) — the drawing's faces, the
//! system for GeoPDF, every sheet's tables and legends; written to the file
//! chosen, or, to print, to a file of its own opened in the system's viewer.

use std::collections::HashMap;
use std::path::Path;

use iced::Task;
use kentos_geometry_core::store::Store;
use kentos_native_style::StylePalette;
use kentos_native_style::library::StyleLibrary;
use kentos_render_wgpu::{Bounds, scene};
use kentos_sheet::display::paper;
use kentos_sheet::kinds::MapLayers;
use kentos_sheet::pdf::{MapContent, MapLayerContent, MapPath, MapText, VectorMap};
use kentos_sheet_ui::export::PdfHost;

use crate::app::{App, Message};
use crate::document::Document;
use crate::labels::MapLabel;
use crate::map_vectors::{VectorScale, layer_paths};

/// A CSS pixel on the paper, millimetres: the unit the maps' labels are laid out in (the web's
/// `PX_MM`, app/sheet/mapLabels.ts), so a label is the same size on the screen's sheet and in
/// the PDF.
pub(crate) const PX_MM: f64 = 25.4 / 96.0;

fn hex_of(c: iced::Color) -> String {
    let [r, g, b, a] = c.into_rgba8();
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

/// A label as the PDF writes it (the web's pdfExport.ts `coreText`).
fn text_of(l: &MapLabel, font: &str) -> MapText {
    MapText {
        at: [l.at.x, l.at.y],
        text: l.text.clone(),
        size: l.size,
        rotation: l.rotation,
        color: hex_of(l.color),
        // A text's own typeface (docs/adr/0183 §2), else the project's.
        font: l.font.map_or(font, |f| f.id()).to_owned(),
        weight: l.weight,
        italic: l.italic,
        anchor: l.anchor,
        halo: Some(hex_of(l.halo)),
    }
}

/// A masked text's paper-coloured box.
fn mask_of(points: &[[f64; 2]]) -> MapPath {
    MapPath {
        points: points.to_vec(),
        closed: true,
        holes: Vec::new(),
        stroke: None,
        fill: Some(paper::PAPER.to_owned()),
    }
}

/// The paper's palette for the style engine (the web's `paperPalette`): its ink `#111111`.
pub(crate) fn paper_style() -> StylePalette {
    StylePalette {
        fg: paper::FG.to_owned(),
        fg_dim: paper::FG_DIM.to_owned(),
        ink: paper::INK.to_owned(),
        paper: paper::PAPER.to_owned(),
    }
}

/// What a map's vectors are built from: the drawing, its geometry store and the style library.
pub(crate) struct MapSource<'a> {
    pub drawing: &'a Document,
    pub store: &'a Store,
    pub library: &'a StyleLibrary,
}

/// The map's content as vectors (the web's app/sheet/mapContent.ts `vectorMap` and pdfExport.ts
/// `coreLayers`): the shown layers bottom first (only those a layer list names), each through
/// the style engine at the map's scale on the paper's palette, what reaches the map's box with
/// a margin of 20 paper mm; then every layer's text masks and texts over all of them. A layer
/// that draws something with no vector form is named with what (the map then goes as a picture).
pub(crate) fn vector_content(
    src: &MapSource<'_>,
    layers: &MapLayers,
    scale: u32,
    extent: [f64; 4],
    labels: &[MapLabel],
) -> Result<MapContent, Vec<(String, Vec<&'static str>)>> {
    let doc = src.drawing;
    let scale = f64::from(scale.max(1));
    let pad = 20.0 * scale / 1000.0;
    let reach = [
        extent[0] - pad,
        extent[1] - pad,
        extent[2] + pad,
        extent[3] + pad,
    ];
    let origin = scene::scene_origin(doc);
    let look = crate::style::scene::Look {
        palette: paper_style(),
        symbol_scale: scale,
        screen: false,
        hairlines: false,
        origin,
    };
    let clip = Bounds {
        min_x: reach[0],
        min_y: reach[1],
        max_x: reach[2],
        max_y: reach[3],
    };
    let mut names = HashMap::new();
    crate::style::scene::names(doc.model.layers().nodes(), &mut names);
    let only = match layers {
        MapLayers::List(l) => Some(&l.layers),
        _ => None,
    };
    let font = crate::drawing_fonts::sheet_font(
        doc.settings()
            .drawing_font
            .unwrap_or(kentos_contracts::DrawingFont::Barlow),
    );
    let o = VectorScale {
        scale,
        origin: [origin.x, origin.y],
    };
    let mut shapes: Vec<MapLayerContent> = Vec::new();
    let mut unsupported = Vec::new();
    for node in crate::style::scene::shown_layers(doc.model.layers().nodes()) {
        if only.is_some_and(|l| !l.contains(&node.id)) {
            continue;
        }
        let objects: Vec<&kentos_contracts::Entity> = doc.model.by_layer(&node.id).collect();
        if objects.is_empty() {
            continue;
        }
        let styled = crate::style::scene::build_whole(
            src.store,
            src.library,
            node,
            &objects,
            &look,
            clip,
            &names,
        );
        let (paths, missing) = layer_paths(&styled, &o, Some(reach));
        if !missing.is_empty() {
            unsupported.push((node.name.clone(), missing));
        }
        shapes.push(MapLayerContent {
            id: node.id.clone(),
            name: node.name.clone(),
            paths,
            texts: Vec::new(),
        });
    }
    if !unsupported.is_empty() {
        return Err(unsupported);
    }
    // The texts and their masks, by layer, over every layer's drawing.
    let writing: Vec<MapLayerContent> = shapes
        .iter()
        .filter_map(|l| {
            let mine: Vec<&MapLabel> = labels.iter().filter(|t| t.layer == l.id).collect();
            if mine.is_empty() {
                return None;
            }
            Some(MapLayerContent {
                id: l.id.clone(),
                name: l.name.clone(),
                paths: mine
                    .iter()
                    .filter_map(|t| t.mask.as_deref().map(mask_of))
                    .chain(mine.iter().filter_map(|t| {
                        t.underline.as_deref().map(|bar| MapPath {
                            points: bar.to_vec(),
                            closed: true,
                            holes: Vec::new(),
                            stroke: None,
                            fill: Some(hex_of(t.color)),
                        })
                    }))
                    .collect(),
                // A multi-line text's mask has no words (docs/adr/0182 §3).
                texts: mine
                    .iter()
                    .filter(|t| !t.text.is_empty())
                    .map(|t| text_of(t, font))
                    .collect(),
            })
        })
        .collect();
    let shown: Vec<MapLayerContent> = shapes
        .into_iter()
        .filter(|l| !l.paths.is_empty() || writing.iter().any(|w| w.id == l.id))
        .collect();
    Ok(MapContent::Vector(VectorMap {
        layers: shown.into_iter().chain(writing).collect(),
    }))
}

impl App {
    /// What a sheet PDF takes from the desktop: the drawing's faces, the project's system as
    /// WKT, and the inputs every sheet going is drawn with (each one's tables, coordinate lists
    /// and legends; the screen's are the sheet in front's).
    pub(crate) fn sheet_pdf_host(&self) -> PdfHost {
        let mut render = self.sheets.export_inputs();
        let open = self.sheets.open_sheet().map(|s| s.id.clone());
        if let Some(doc) = &self.document {
            let book = self.sheets.book();
            for id in self.sheets.pdf_sheets() {
                if Some(&id) == open.as_ref() {
                    continue;
                }
                let Some(sheet) = book.sheet(&id) else {
                    continue;
                };
                let list = kentos_sheet::display::display_list(book, &id, &render).ok();
                let data = crate::sheet_inputs::sheet_data(
                    &doc.model,
                    &self.spatial,
                    &self.styles.library,
                    self.selection.ids(),
                    book,
                    sheet,
                    list.as_ref(),
                );
                render.tables.extend(data.tables);
                render.coordinates.extend(data.coordinates);
                render.legends.extend(data.legends);
            }
        }
        PdfHost {
            fonts: crate::drawing_fonts::pdf_fonts(),
            crs: self
                .document
                .as_ref()
                .and_then(|d| crate::sheet_inputs::pdf_crs(d.settings().srid)),
            render,
        }
    }

    /// Yazdır: the PDF to a file of its own, opened in the system's viewer to print from there.
    pub(crate) fn sheet_print(&mut self) -> Task<Message> {
        let host = self.sheet_pdf_host();
        let painter = crate::sheets::painter_of(
            &self.sheet_maps,
            self.document.as_ref(),
            &self.spatial,
            &self.styles.library,
            &self.styles.images,
        );
        let made = self.sheets.pdf_and_findings(&painter, host);
        let name = self.sheets.pdf_file_name();
        let findings = made.as_ref().map(|(_, f)| f.clone()).unwrap_or_default();
        let made = made.map(|(bytes, _)| bytes);
        self.say_pdf_findings(&findings);
        match made.and_then(|bytes| {
            let dir = std::env::temp_dir().join("kentos-pafta");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(&name);
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            Ok(path)
        }) {
            Ok(path) => match open_with_viewer(&path) {
                Ok(()) => self.output(format!(
                    "“{name}” görüntüleyicide açıldı: oradan yazdırın ({}).",
                    path.display()
                )),
                Err(e) => self.warn(format!(
                    "“{name}” yazıldı ama görüntüleyici açılamadı ({e}): {} dosyasını açıp yazdırın.",
                    path.display()
                )),
            },
            Err(e) => self.warn(format!("Yazdırılamadı: PDF yazılamadı: {e}")),
        }
        Task::none()
    }
}

/// Opens a file in the system's viewer (no package: the platform's own opener), its process
/// waited on beside the app.
#[cfg(not(test))]
fn open_with_viewer(path: &Path) -> std::io::Result<()> {
    use std::process::Command;
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", ""]).arg(path);
        c
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut c = Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut command = {
        let mut c = Command::new("xdg-open");
        c.arg(path);
        c
    };
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
thread_local! {
    /// The files the tests' app opened in the viewer (none is launched in a test).
    pub(crate) static OPENED: std::cell::RefCell<Vec<std::path::PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn open_with_viewer(path: &Path) -> std::io::Result<()> {
    OPENED.with(|o| o.borrow_mut().push(path.to_owned()));
    Ok(())
}
