//! A sheet PDF on the desktop (docs/sheet/design.md §9a): its maps as the
//! drawing's vectors — every shown layer through the style engine at the
//! map's scale on the paper's palette (the web's map frames: line weights,
//! line types, point symbols and hatches as styled), turned into paths
//! (kentos_plot, docs/adr/0252), and the drawing's text as its map frames write it
//! (labels.rs, anchored as the web anchors it) — the drawing's faces, the
//! system for GeoPDF, every sheet's tables and legends; written to the file
//! chosen, or, to print, to a file of its own opened in the system's viewer.

use std::path::Path;

use iced::Task;
#[cfg(test)]
use kentos_sheet_ui::export::PdfHost;

use crate::app::{App, Message};

// A map's content, its labels' unit and the paper's palette are the plotter's (docs/adr/0252).
pub(crate) use kentos_plot::maps::{PX_MM, paper_style};

/// A colour of the screen as the PDF writes it: `#rrggbb`, with its opacity `#rrggbbaa`.
#[cfg(test)]
pub(crate) fn hex_of(c: iced::Color) -> String {
    let [r, g, b, a] = c.into_rgba8();
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

impl App {
    /// What a sheet PDF takes from the desktop: the drawing's faces, the project's system as
    /// WKT, and the inputs every sheet going is drawn with (each one's tables, coordinate lists
    /// and legends; the screen's are the sheet in front's).
    #[cfg(test)]
    pub(crate) fn sheet_pdf_host(&self) -> PdfHost {
        let mut render = self.sheets.export_inputs();
        let open = self.sheets.open_sheet().map(|s| s.id.clone());
        if let Some(doc) = &self.document {
            let book = self.sheets.book();
            let unscoped = self
                .spatial
                .has_sheet_view()
                .then(|| kentos_interaction::Spatial::of(&doc.model));
            for id in self.sheets.pdf_sheets() {
                if Some(&id) == open.as_ref() {
                    continue;
                }
                let Some(sheet) = book.sheet(&id) else {
                    continue;
                };
                let list = kentos_sheet::display::display_list(book, &id, &render).ok();
                let data = kentos_plot::inputs::sheet_data(
                    &doc.model,
                    unscoped.as_ref().unwrap_or(&self.spatial),
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
            fonts: kentos_plot::fonts::pdf_fonts(),
            crs: self
                .document
                .as_ref()
                .and_then(|d| kentos_plot::inputs::pdf_crs(d.settings().srid)),
            render,
        }
    }

    /// Capture the open designer book, assets and drawing together. No screen cache participates.
    pub(crate) fn sheet_output_snapshot(&self) -> Result<kentos_domain::Document, String> {
        let mut doc = self
            .document
            .as_ref()
            .map(|d| d.model.reading_copy())
            .ok_or("Önce bir çizim açın.")?;
        let kept = self.sheets.project_sheets()?;
        let Some(kept) = kept else {
            return Err("Çizimde pafta yok.".into());
        };
        let (mut book, pictures) =
            kentos_sheet::project::read_project_sheets(&kept).map_err(|e| e.message)?;
        for sheet in &mut book.sheets {
            sheet.export.dpi = self.sheets.output_dpi();
        }
        let kept = kentos_sheet::project::project_sheets(&book, |hash| {
            pictures
                .iter()
                .find(|(h, _)| h == hash)
                .map(|(_, b)| b.clone())
        })
        .map_err(|e| e.message)?;
        doc.set_sheets(kept)?;
        Ok(doc)
    }

    pub(crate) fn sheet_native_pdf(&self) -> Result<kentos_plot::book::BookPdf, String> {
        let options = self.sheets.output_pdf_options();
        if options.sheets.is_empty() {
            return Err("Yazılacak bir pafta seçin.".into());
        }
        let doc = self.sheet_output_snapshot()?;
        let drawing = self
            .document
            .as_ref()
            .and_then(|d| d.path.as_ref())
            .map(|p| p.to_string_lossy().into_owned());
        kentos_plot::book::book_pdf_options(
            &doc,
            &options.sheets.clone(),
            "",
            &kentos_plot::book::today(),
            drawing.as_deref(),
            Some(options),
        )
        .map_err(|e| e.message)
    }

    pub(crate) fn sheet_print_preview(&mut self) {
        let result = (|| {
            let doc = self.sheet_output_snapshot()?;
            let id = self
                .sheets
                .pdf_sheets()
                .into_iter()
                .next()
                .ok_or("Yazılacak bir pafta seçin.")?;
            let drawing = self
                .document
                .as_ref()
                .and_then(|d| d.path.as_ref())
                .map(|p| p.to_string_lossy().into_owned());
            let image = kentos_plot::preview::page(
                &doc,
                &id,
                "",
                &kentos_plot::book::today(),
                drawing.as_deref(),
                96,
            )
            .map_err(|e| e.message)?;
            Ok::<_, String>((image, id))
        })();
        match result {
            Ok((image, id)) => {
                self.sheets.show_print_preview(
                image.width,
                image.height,
                image.rgba,
                format!(
                    "{} · 96 dpi önizleme · PDF ile aynı ölçek ve baskı profili · ilk seçili pafta",
                    self.sheets
                        .book()
                        .sheet(&id)
                        .map_or(id.as_str(), |s| s.name.as_str())
                ),
            );
            }
            Err(e) => self.warn(format!("Baskı önizlenemedi: {e}")),
        }
    }

    /// Yazdır: the PDF to a file of its own, opened in the system's viewer to print from there.
    pub(crate) fn sheet_print(&mut self) -> Task<Message> {
        // The project as it is now (see `sheet_export_to`).
        self.sheets.set_context(self.sheet_context());
        let made = self.sheet_native_pdf().map(|p| (p.bytes, p.findings));
        let name = self.sheets.pdf_file_name();
        let findings = made.as_ref().map(|(_, f)| f.clone()).unwrap_or_default();
        let made = made.map(|(bytes, _)| bytes);
        self.say_pdf_findings(&findings);
        match made.and_then(|bytes| {
            let dir = std::env::temp_dir().join("kentos-pafta");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(&name);
            // Whole before the viewer opens it (docs/adr/0253 §10).
            kentos_headless::files::write_bytes(&bytes, &path).map_err(|e| e.message)?;
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

/// Opens a file in the system's viewer, or an address in the browser (no package: the
/// platform's own opener), its process waited on beside the app.
#[cfg(not(test))]
pub(crate) fn open_with_viewer(path: &Path) -> std::io::Result<()> {
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
pub(crate) fn open_with_viewer(path: &Path) -> std::io::Result<()> {
    OPENED.with(|o| o.borrow_mut().push(path.to_owned()));
    Ok(())
}
