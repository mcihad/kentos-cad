//! File exchange (the web's `app/fileExchange.ts` and `ui/io/`): coordinate
//! lists (Netcad NCN, TXT, CSV) and DXF, in and out; GeoJSON in and out and
//! Shapefile in, from its files or a zip archive; through the shared
//! readers and writers (`crates/shared/formats`, the ones the web runs in its
//! formats worker; CLAUDE.md §9.7, docs/adr/0009). Files are read and
//! written off the UI thread. The source coordinate system is always asked,
//! never guessed, and nothing is reprojected (§5); an import is one undo
//! step; what a reader or writer converted or left out is said.
//!
//! One window at a time ([`Window`]), shown as `Dialog::Exchange`; its
//! messages are [`Event`]s. A late answer to an older read, or to a window
//! closed since, carries a generation and is dropped.

pub mod apply;
mod coord_export;
mod coord_import;
mod dxf_export;
mod dxf_import;
mod geojson_export;
mod gis_import;
#[cfg(test)]
mod gis_tests;
#[cfg(test)]
mod tests;
pub(crate) mod words;

use std::path::PathBuf;
use std::sync::Arc;

use iced::futures::channel::mpsc;
use iced::{Element, Task};
use kentos_domain::Slot;
use kentos_interaction::Format;
use kentos_render_wgpu::Bounds;
use kentos_render_wgpu::scene;

use crate::app::{App, Dialog, Message, Picker};

/// The file an import reads: its name and bytes.
#[derive(Debug, Clone)]
pub struct Picked {
    pub name: String,
    pub bytes: Arc<[u8]>,
}

/// What an import picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Dxf,
    Coords,
    GeoJson,
    /// A Shapefile layer's files chosen together, or its zip archive.
    Shapefile,
}

/// The open exchange window.
#[derive(Debug)]
pub enum Window {
    DxfImport(dxf_import::State),
    CoordImport(coord_import::State),
    DxfExport(dxf_export::State),
    CoordExport(coord_export::State),
    /// Boxed: the largest window's state.
    GisImport(Box<gis_import::State>),
    GeoJsonExport(geojson_export::State),
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A file picked for an import; `None` when the dialog was cancelled or
    /// the file could not be read (`Err`: why).
    Picked(Kind, Option<Result<Picked, String>>),
    /// Files picked together (a Shapefile's parts, or its archive).
    PickedMany(Kind, Option<Result<Vec<Picked>, String>>),
    DxfImport(dxf_import::Event),
    CoordImport(coord_import::Event),
    DxfExport(dxf_export::Event),
    CoordExport(coord_export::Event),
    GisImport(gis_import::Event),
    GeoJsonExport(geojson_export::Event),
    /// An export written: the file's name, or why not; `None` when the save
    /// dialog was cancelled.
    Written(Option<Result<String, String>>),
    Close,
}

/// The web command ids this module runs.
pub const COMMANDS: [&str; 8] = [
    "file.import.dxf",
    "file.import.ncn",
    "crs.points",
    "file.export.dxf",
    "file.export.ncn",
    "file.import.geojson",
    "file.import.shp",
    "file.export.geojson",
];

fn message(event: Event) -> Message {
    Message::Exchange(Box::new(event))
}

/// Runs `work` on a thread of its own and brings its answer back as a message.
fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) -> Event + Send + 'static,
) -> Task<Message> {
    iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
        let answer = message(done(work()));
        let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(&mut out, answer));
    })
}

impl App {
    pub(crate) fn exchange_command(&mut self, id: &'static str) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        match id {
            "file.import.dxf" => self.pick(Kind::Dxf),
            "file.import.ncn" | "crs.points" => self.pick(Kind::Coords),
            "file.import.geojson" => self.pick(Kind::GeoJson),
            "file.import.shp" => self.pick(Kind::Shapefile),
            "file.export.dxf" => {
                let state = dxf_export::State::new(self);
                self.open_window(Window::DxfExport(state));
                Task::none()
            }
            "file.export.ncn" => {
                let state = coord_export::State::new(self);
                self.open_window(Window::CoordExport(state));
                Task::none()
            }
            "file.export.geojson" => {
                let state = geojson_export::State::new(self);
                self.open_window(Window::GeoJsonExport(state));
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn open_window(&mut self, window: Window) {
        self.exchange = Some(window);
        self.dialog = Some(Dialog::Exchange);
    }

    /// Asks for the file to import, straight from the command, and reads it.
    fn pick(&mut self, kind: Kind) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            if kind == Kind::Shapefile {
                return Task::perform(
                    async move { Some(read_file(&path).map(|f| vec![f])) },
                    move |files| message(Event::PickedMany(kind, files)),
                );
            }
            return Task::perform(async move { Some(read_file(&path)) }, move |file| {
                message(Event::Picked(kind, file))
            });
        }
        let (title, filter, extensions): (&str, &str, &[&str]) = match kind {
            Kind::Dxf => ("DXF içe aktar", "AutoCAD DXF (DWG değil)", &["dxf"]),
            Kind::Coords => (
                "Koordinat listesi içe aktar",
                "Koordinat listesi (NCN, TXT, CSV)",
                &["ncn", "txt", "csv", "xyz", "dat", "asc"],
            ),
            Kind::GeoJson => ("GeoJSON içe aktar", "GeoJSON", &["geojson", "json"]),
            Kind::Shapefile => (
                "Shapefile içe aktar",
                "Shapefile katmanı (.shp, .shx, .dbf, .prj, .cpg) ya da .zip",
                &["shp", "shx", "dbf", "prj", "cpg", "zip"],
            ),
        };
        if kind == Kind::Shapefile {
            // A layer's parts are chosen together.
            return Task::perform(
                async move {
                    let picked = rfd::AsyncFileDialog::new()
                        .set_title(title)
                        .add_filter(filter, extensions)
                        .pick_files()
                        .await?;
                    let mut files = Vec::with_capacity(picked.len());
                    for file in picked {
                        let name = file.file_name();
                        let bytes = file.read().await;
                        files.push(Picked {
                            name,
                            bytes: bytes.into(),
                        });
                    }
                    Some(Ok(files))
                },
                move |files| message(Event::PickedMany(kind, files)),
            );
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(title)
                    .add_filter(filter, extensions)
                    .pick_file()
                    .await?;
                let name = file.file_name();
                let bytes = file.read().await;
                Some(Ok(Picked {
                    name,
                    bytes: bytes.into(),
                }))
            },
            move |file| message(Event::Picked(kind, file)),
        )
    }

    pub(crate) fn exchange_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Picked(_, None) => Task::none(),
            Event::Picked(_, Some(Err(e))) => {
                self.error(e);
                Task::none()
            }
            Event::Picked(Kind::Dxf, Some(Ok(file))) => self.dxf_import_picked(file),
            Event::Picked(Kind::Coords, Some(Ok(file))) => self.coord_import_picked(file),
            Event::Picked(Kind::GeoJson, Some(Ok(file))) => {
                self.gis_import_picked(Ok(gis_import::Source::GeoJson(file)))
            }
            Event::Picked(Kind::Shapefile, Some(Ok(file))) => {
                self.gis_import_picked(gis_import::shapefile_source(vec![file]))
            }
            Event::PickedMany(_, None) => Task::none(),
            Event::PickedMany(_, Some(Err(e))) => {
                self.error(e);
                Task::none()
            }
            Event::PickedMany(Kind::Shapefile, Some(Ok(files))) => {
                self.gis_import_picked(gis_import::shapefile_source(files))
            }
            Event::PickedMany(kind, Some(Ok(mut files))) => match files.len() {
                1 => self.exchange_event(Event::Picked(kind, files.pop().map(Ok))),
                _ => Task::none(),
            },
            Event::DxfImport(e) => self.dxf_import_event(e),
            Event::CoordImport(e) => self.coord_import_event(e),
            Event::DxfExport(e) => self.dxf_export_event(e),
            Event::CoordExport(e) => self.coord_export_event(e),
            Event::GisImport(e) => self.gis_import_event(e),
            Event::GeoJsonExport(e) => self.geojson_export_event(e),
            Event::Written(outcome) => self.export_written(outcome),
            Event::Close => {
                self.close_exchange();
                Task::none()
            }
        }
    }

    /// Closes the window; a read still running is dropped when it answers.
    pub(crate) fn close_exchange(&mut self) {
        self.exchange = None;
        if self.dialog == Some(Dialog::Exchange) {
            self.dialog = None;
        }
    }

    pub(crate) fn exchange_view(&self) -> Element<'_, Message> {
        match &self.exchange {
            Some(Window::DxfImport(s)) => self.dxf_import_view(s),
            Some(Window::CoordImport(s)) => self.coord_import_view(s),
            Some(Window::DxfExport(s)) => self.dxf_export_view(s),
            Some(Window::CoordExport(s)) => self.coord_export_view(s),
            Some(Window::GisImport(s)) => self.gis_import_view(s),
            Some(Window::GeoJsonExport(s)) => self.geojson_export_view(s),
            None => iced::widget::text("").into(),
        }
    }

    /// Shows what an import added: its extent, never smaller than a few
    /// metres, so one point alone is shown in context (web `zoomToImported`).
    pub(crate) fn zoom_to(&mut self, slots: &[Slot]) {
        /// At least this much around what was imported.
        const MIN_SPAN: f64 = 20.0;
        let Some(doc) = &self.document else { return };
        let objects = slots.iter().filter_map(|s| doc.model.get(*s));
        let Some(b) = scene::extents_of(doc, objects) else {
            return;
        };
        let grow = |lo: f64, hi: f64| {
            let pad = ((MIN_SPAN - (hi - lo)) / 2.0).max(0.0);
            (lo - pad, hi + pad)
        };
        let (min_x, max_x) = grow(b.min_x, b.max_x);
        let (min_y, max_y) = grow(b.min_y, b.max_y);
        self.viewport.show(&Bounds {
            min_x,
            min_y,
            max_x,
            max_y,
        });
    }

    /// Asks where to write an export and writes it (web `saveExport`). The
    /// drawing is not affected: an export is not a save.
    fn save_export(
        &mut self,
        bytes: Vec<u8>,
        suggested: String,
        filter: (&'static str, &'static str),
    ) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return Task::perform(async move { Some(write_file(&path, &bytes)) }, |outcome| {
                message(Event::Written(outcome))
            });
        }
        let (description, extension) = filter;
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Dışa aktar")
                    .add_filter(description, &[extension])
                    .set_file_name(suggested)
                    .save_file()
                    .await?;
                let mut path = file.path().to_path_buf();
                if path.extension().is_none() {
                    path.set_extension(extension);
                }
                Some(write_file(&path, &bytes))
            },
            |outcome| message(Event::Written(outcome)),
        )
    }

    fn export_written(&mut self, outcome: Option<Result<String, String>>) -> Task<Message> {
        match self.exchange {
            Some(Window::DxfExport(_)) => self.dxf_export_written(outcome),
            Some(Window::CoordExport(_)) => self.coord_export_written(outcome),
            Some(Window::GeoJsonExport(_)) => self.geojson_export_written(outcome),
            _ => Task::none(),
        }
    }

    /// How the open drawing writes numbers (its units and decimals).
    fn number_format(&self) -> Format {
        self.document
            .as_ref()
            .map_or_else(Format::default, |d| Format::of(d.settings()))
    }

    /// The file name an export suggests: the drawing's name without `.kcad`
    /// and the format's extension (web `exportName`).
    fn export_name(&self, extension: &str) -> String {
        let name = self
            .document
            .as_ref()
            .map_or("", |d| d.name())
            .trim_end_matches(".kcad")
            .trim_end_matches(".KCAD");
        format!(
            "{}{extension}",
            if name.is_empty() { "cizim" } else { name }
        )
    }
}

fn read_file(path: &PathBuf) -> Result<Picked, String> {
    let bytes = std::fs::read(path).map_err(|e| {
        format!(
            "“{}” okunamadı: {e}. Dosyanın izinlerini denetleyin.",
            path.display()
        )
    })?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    Ok(Picked {
        name,
        bytes: bytes.into(),
    })
}

fn write_file(path: &std::path::Path, bytes: &[u8]) -> Result<String, String> {
    std::fs::write(path, bytes).map_err(|e| {
        format!(
            "“{}” yazılamadı: {e}. Başka bir yere kaydetmeyi deneyin.",
            path.display()
        )
    })?;
    Ok(path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    ))
}
