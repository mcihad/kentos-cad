//! File exchange (the web's `app/fileExchange.ts` and `ui/io/`): coordinate
//! lists (Netcad NCN, TXT, CSV) and DXF, in and out; Netcad NCZ in
//! (docs/adr/0138); GeoJSON in and out and Shapefile in, from its files or a
//! zip archive; GNSS (GPX, NMEA) in, moved from WGS 84 into the project's
//! system (docs/adr/0169 §6); KML and KMZ in and out and GPX out
//! (docs/adr/0222); geotagged photos in (docs/adr/0225 §3); through the shared
//! readers and writers (`crates/shared/formats`, the ones the web runs in its
//! formats worker; CLAUDE.md §9.7, docs/adr/0009). Files are read and
//! written off the UI thread. The source coordinate system is always asked,
//! never guessed, and nothing is reprojected silently (§5): a GeoJSON,
//! Shapefile or KML import moves into the project's system only when the
//! user asks, and KML, GPX and GeoJSON (asked) are written in WGS 84; an
//! import is one undo step; what a reader or writer converted or left out is
//! said.
//!
//! One window at a time ([`Window`]), shown as `Dialog::Exchange`; its
//! messages are [`Event`]s. A late answer to an older read, or to a window
//! closed since, carries a generation and is dropped.

mod area_queries;
#[cfg(test)]
mod area_tests;
pub mod apply;
mod atlas_export;
#[cfg(test)]
mod atlas_tests;
pub(crate) mod coord_export;
mod csv_exchange;
#[cfg(test)]
mod csv_tests;
mod fgb_exchange;
#[cfg(test)]
mod fgb_tests;
mod gml_exchange;
mod gpkg_tiles;
#[cfg(test)]
mod gpkg_tiles_tests;
#[cfg(test)]
mod gml_tests;
mod xyz_exchange;
pub(crate) mod coord_import;
#[cfg(test)]
mod dimension_tests;
pub(crate) mod drawing_import;
pub(crate) mod dxf_export;
pub(crate) mod field_send;
#[cfg(test)]
mod field_send_tests;
mod geojson_export;
mod gis_check;
mod gis_export;
#[cfg(test)]
mod gis_derivation_tests;
mod gis_import;
#[cfg(test)]
mod gis_tests;
mod gnss_import;
#[cfg(test)]
mod gnss_tests;
pub(crate) mod gpkg;
pub(crate) mod pdf_import;
#[cfg(test)]
mod gpkg_tests;
#[cfg(test)]
mod kml_tests;
pub(super) mod package;
pub(crate) mod photo_import;
mod postgis;
#[cfg(test)]
mod postgis_tests;
pub(crate) mod shp_export;
#[cfg(test)]
mod tests;
mod wgs84_export;
pub(crate) mod words;

pub(crate) use coord_export::TableRows;
pub(crate) use dxf_export::dimension_text;

use std::path::PathBuf;
use std::sync::Arc;

use iced::futures::channel::mpsc;
use iced::{Element, Task};
use kentos_domain::Slot;
use kentos_interaction::Format;
use kentos_render_wgpu::Bounds;
use kentos_render_wgpu::scene;

use crate::app::{App, Dialog, Message, Picker};

/// The file an import reads: its name and bytes, and where it was when
/// known (what a command's journal row names, docs/adr/0257 §4).
#[derive(Debug, Clone)]
pub struct Picked {
    pub name: String,
    pub bytes: Arc<[u8]>,
    pub path: Option<PathBuf>,
}

/// What an import picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Dxf,
    /// A Netcad NCZ drawing (docs/adr/0138).
    Ncz,
    Coords,
    /// A coordinate list for Noktalar's İçe aktar: once the points are in,
    /// Çift noktaları ayıkla by name (docs/adr/0153 §5).
    TableCoords,
    GeoJson,
    /// A Shapefile layer's files chosen together, or its zip archive.
    Shapefile,
    /// A receiver's GPX or NMEA file (docs/adr/0169 §6).
    Gnss,
    /// A KML file or KMZ archive (docs/adr/0222).
    Kml,
}

/// The open exchange window.
#[derive(Debug)]
pub enum Window {
    /// DXF or NCZ in (drawing_import.rs).
    DrawingImport(drawing_import::State),
    CoordImport(coord_import::State),
    DxfExport(dxf_export::State),
    CoordExport(coord_export::State),
    /// Boxed: the largest window's state.
    GisImport(Box<gis_import::State>),
    GeoJsonExport(geojson_export::State),
    /// Boxed: the plan it holds.
    GnssImport(Box<gnss_import::State>),
    /// Cihaza gönder (docs/adr/0169 §4); boxed: the points and the file.
    FieldSend(Box<field_send::State>),
    /// KML/KMZ or GPX out (docs/adr/0222 §4, §5).
    Wgs84Export(wgs84_export::State),
    /// Konumlu fotoğraflar (docs/adr/0225 §3); boxed: the photos' bytes and the plan.
    PhotoImport(Box<photo_import::State>),
    /// GeoPackage içe aktar (docs/adr/0502).
    Gpkg(gpkg::State),
    /// PostGIS'e bağlan (docs/adr/0505).
    Postgis(postgis::State),
    /// Shapefile dışa aktar (docs/adr/0514).
    ShpExport(shp_export::State),
    /// CSV dışa aktar and içe aktar (docs/adr/0522).
    /// Atlası PDF yap (docs/adr/0534); boxed: the plan it holds.
    AtlasExport(Box<atlas_export::State>),
    CsvExport(csv_exchange::ExportState),
    CsvImport(csv_exchange::ImportState),
    /// FlatGeobuf dışa aktar and içe aktar (docs/adr/0544).
    FgbExport(fgb_exchange::ExportState),
    FgbImport(fgb_exchange::ImportState),
    /// GML dışa aktar and içe aktar, XYZ dışa aktar (docs/adr/0545).
    GmlExport(gml_exchange::ExportState),
    GmlImport(gml_exchange::ImportState),
    XyzExport(xyz_exchange::ExportState),
    /// Nokta bulutu alan istatistiği, Nokta bulutu profili and Mesh sor (docs/adr/0552).
    CloudRegion(area_queries::RegionState),
    CloudProfile(area_queries::ProfileState),
    MeshQuery(area_queries::MeshState),
    /// GeoPackage karolarını oku and Rasteri GeoPackage karosu olarak yaz (docs/adr/0556); boxed: the tables and the plan.
    GpkgTilesRead(Box<gpkg_tiles::ReadState>),
    GpkgTilesWrite(Box<gpkg_tiles::WriteState>),
    /// PDF içe aktar (docs/adr/0352); boxed: the plan it holds.
    PdfImport(Box<pdf_import::State>),
    /// GIS türevi dışa aktar and GIS türevini denetle (docs/adr/0530).
    GisExport(gis_export::State),
    GisCheck(gis_check::State),
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A file picked for an import; `None` when the dialog was cancelled or
    /// the file could not be read (`Err`: why).
    Picked(Kind, Option<Result<Picked, String>>),
    /// Files picked together (a Shapefile's parts, or its archive).
    PickedMany(Kind, Option<Result<Vec<Picked>, String>>),
    DrawingImport(drawing_import::Event),
    CoordImport(coord_import::Event),
    DxfExport(dxf_export::Event),
    CoordExport(coord_export::Event),
    GisImport(gis_import::Event),
    GeoJsonExport(geojson_export::Event),
    GnssImport(gnss_import::Event),
    FieldSend(field_send::Event),
    Wgs84Export(wgs84_export::Event),
    PhotoImport(photo_import::Event),
    Gpkg(gpkg::Event),
    GpkgTiles(gpkg_tiles::Event),
    Postgis(postgis::Event),
    PdfImport(pdf_import::Event),
    Package(package::Event),
    ShpExport(shp_export::Event),
    Csv(csv_exchange::Event),
    Fgb(fgb_exchange::Event),
    Gml(gml_exchange::Event),
    Xyz(xyz_exchange::Event),
    Area(area_queries::Event),
    Atlas(atlas_export::Event),
    GisExport(gis_export::Event),
    GisCheck(gis_check::Event),
    /// An export written: the file's name, or why not; `None` when the save
    /// dialog was cancelled.
    Written(Option<Result<String, String>>),
    Close,
}

/// The web command ids this module runs.
pub const COMMANDS: [&str; 39] = [
    "file.export.atlas",
    "file.import.dxf",
    "file.import.ncz",
    "file.import.ncn",
    "crs.points",
    "file.export.dxf",
    "file.export.ncn",
    "file.import.geojson",
    "file.import.shp",
    "file.export.geojson",
    "file.import.gnss",
    "field.send",
    "file.import.kml",
    "file.export.kml",
    "file.export.gpx",
    "file.import.photos",
    "file.import.gpkg",
    "raster.gpkgRead",
    "raster.gpkgWrite",
    "file.import.pdf",
    "file.export.gpkg",
    "file.export.shp",
    "file.export.csv",
    "file.import.csv",
    "file.export.fgb",
    "file.import.fgb",
    "file.export.gml",
    "file.import.gml",
    "file.export.xyz",
    "pointcloud.region",
    "pointcloud.profile",
    "mesh.query",
    "file.export.gis",
    "gis.derivation",
    "source.link",
    "source.postgis",
    "file.package",
    "file.export.maplibre",
    "file.import.maplibre",
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
            "file.import.ncz" => self.pick(Kind::Ncz),
            "file.import.ncn" | "crs.points" => self.pick(Kind::Coords),
            "file.import.geojson" => self.pick(Kind::GeoJson),
            "file.import.shp" => self.pick(Kind::Shapefile),
            "file.import.gnss" => self.pick(Kind::Gnss),
            "file.import.kml" => self.pick(Kind::Kml),
            "file.import.photos" => self.pick_photos(),
            "file.import.gpkg" => self.gpkg_pick(false),
            "raster.gpkgRead" => self.gpkg_tiles_read_pick(),
            "raster.gpkgWrite" => self.gpkg_tiles_write_open(),
            "file.import.pdf" => self.pdf_pick(),
            "source.link" => self.gpkg_pick(true),
            "source.postgis" => self.postgis_open(),
            "file.package" => self.package_pick(),
            // The visible layers as one MapLibre style, and a style's looks into every layer it is for (docs/adr/0560).
            "file.export.maplibre" => self.maplibre_export_ask(None),
            "file.import.maplibre" => self.maplibre_import_ask(None),
            "file.export.gpkg" => self.gpkg_save(),
            "file.export.shp" => self.shp_export_open(),
            "file.export.csv" => self.csv_export_open(),
            "file.import.csv" => self.csv_import_pick(),
            "file.export.fgb" => self.fgb_export_open(),
            "file.export.gml" => self.gml_export_open(),
            "file.import.gml" => self.gml_import_pick(),
            "file.export.xyz" => self.xyz_export_open(),
            "pointcloud.region" => self.region_open(),
            "pointcloud.profile" => self.profile_open(),
            "mesh.query" => self.mesh_query_open(),
            "file.import.fgb" => self.fgb_import_pick(),
            "file.export.atlas" => self.atlas_export_open(),
            "file.export.gis" => self.gis_export_open(),
            "gis.derivation" => self.gis_check_open(),
            "field.send" => self.field_send_selection(),
            "file.export.kml" => {
                let state = wgs84_export::State::new(self, wgs84_export::Format::Kml);
                self.open_window(Window::Wgs84Export(state));
                Task::none()
            }
            "file.export.gpx" => {
                let state = wgs84_export::State::new(self, wgs84_export::Format::Gpx);
                self.open_window(Window::Wgs84Export(state));
                Task::none()
            }
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

    /// Noktalar's Dışa aktar (docs/adr/0153 §5): the coordinate list window
    /// over the table's target rows, in its order.
    pub(crate) fn export_table_points(&mut self, rows: TableRows) -> Task<Message> {
        self.open_window(Window::CoordExport(coord_export::State::for_table(rows)));
        Task::none()
    }

    /// Noktalar's İçe aktar: the coordinate list window; once the points are
    /// in, Çift noktaları ayıkla by name over the points carrying their names.
    pub(crate) fn import_table_points(&mut self) -> Task<Message> {
        if self.document.is_none() {
            return Task::none();
        }
        self.pick(Kind::TableCoords)
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
            Kind::Ncz => ("NCZ içe aktar", "Netcad çizimi (NCZ)", &["ncz"]),
            Kind::Coords | Kind::TableCoords => (
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
            Kind::Gnss => (
                "GNSS içe aktar",
                "GNSS dosyası (GPX, NMEA)",
                &["gpx", "nmea", "nma", "txt", "log"],
            ),
            Kind::Kml => ("KML/KMZ içe aktar", "KML ya da KMZ", &["kml", "kmz"]),
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
                        let path = Some(file.path().to_path_buf());
                        let bytes = file.read().await;
                        files.push(Picked {
                            name,
                            bytes: bytes.into(),
                            path,
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
                let path = Some(file.path().to_path_buf());
                let bytes = file.read().await;
                Some(Ok(Picked {
                    name,
                    bytes: bytes.into(),
                    path,
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
            Event::Picked(Kind::Dxf, Some(Ok(file))) => {
                self.drawing_import_picked(drawing_import::Source::Dxf, file)
            }
            Event::Picked(Kind::Ncz, Some(Ok(file))) => {
                self.drawing_import_picked(drawing_import::Source::Ncz, file)
            }
            Event::Picked(kind @ (Kind::Coords | Kind::TableCoords), Some(Ok(file))) => {
                self.coord_import_picked(file, kind == Kind::TableCoords)
            }
            Event::Picked(Kind::GeoJson, Some(Ok(file))) => {
                self.gis_import_picked(Ok(gis_import::Source::GeoJson(file)))
            }
            Event::Picked(Kind::Shapefile, Some(Ok(file))) => {
                self.gis_import_picked(gis_import::shapefile_source(vec![file]))
            }
            Event::Picked(Kind::Gnss, Some(Ok(file))) => self.gnss_import_picked(file),
            Event::Picked(Kind::Kml, Some(Ok(file))) => {
                self.gis_import_picked(Ok(gis_import::Source::Kml(file)))
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
            Event::DrawingImport(e) => self.drawing_import_event(e),
            Event::CoordImport(e) => self.coord_import_event(e),
            Event::DxfExport(e) => self.dxf_export_event(e),
            Event::CoordExport(e) => self.coord_export_event(e),
            Event::GisImport(e) => self.gis_import_event(e),
            Event::GeoJsonExport(e) => self.geojson_export_event(e),
            Event::GnssImport(e) => self.gnss_import_event(e),
            Event::FieldSend(e) => self.field_send_event(e),
            Event::Wgs84Export(e) => self.wgs84_export_event(e),
            Event::PhotoImport(e) => self.photo_import_event(e),
            Event::Gpkg(e) => self.gpkg_event(e),
            Event::GpkgTiles(e) => self.gpkg_tiles_event(e),
            Event::PdfImport(e) => self.pdf_event(e),
            Event::Postgis(e) => self.postgis_event(e),
            Event::Package(e) => self.package_event(e),
            Event::ShpExport(e) => self.shp_export_event(e),
            Event::Csv(e) => self.csv_event(e),
            Event::Fgb(e) => self.fgb_event(e),
            Event::Gml(e) => self.gml_event(e),
            Event::Xyz(e) => self.xyz_event(e),
            Event::Area(e) => self.area_event(e),
            Event::Atlas(e) => self.atlas_export_event(e),
            Event::GisExport(e) => self.gis_export_event(e),
            Event::GisCheck(e) => self.gis_check_event(e),
            Event::Written(outcome) => self.export_written(outcome),
            Event::Close => {
                self.close_exchange();
                Task::none()
            }
        }
    }

    /// Closes the window; a read still running is asked to stop, and dropped when it answers.
    pub(crate) fn close_exchange(&mut self) {
        if let Some(Window::DrawingImport(s)) = &self.exchange {
            s.stop_reading();
        }
        // Cihaza gönder opened from Aplikasyon goes back to it.
        let back = match &self.exchange {
            Some(Window::FieldSend(s)) => s.back,
            _ => None,
        };
        self.exchange = None;
        if self.dialog == Some(Dialog::Exchange) {
            self.dialog = back;
        }
    }

    pub(crate) fn exchange_view(&self) -> Element<'_, Message> {
        match &self.exchange {
            Some(Window::DrawingImport(s)) => self.drawing_import_view(s),
            Some(Window::CoordImport(s)) => self.coord_import_view(s),
            Some(Window::DxfExport(s)) => self.dxf_export_view(s),
            Some(Window::CoordExport(s)) => self.coord_export_view(s),
            Some(Window::GisImport(s)) => self.gis_import_view(s),
            Some(Window::GeoJsonExport(s)) => self.geojson_export_view(s),
            Some(Window::GnssImport(s)) => self.gnss_import_view(s),
            Some(Window::FieldSend(s)) => self.field_send_view(s),
            Some(Window::Wgs84Export(s)) => self.wgs84_export_view(s),
            Some(Window::PhotoImport(s)) => self.photo_import_view(s),
            Some(Window::Gpkg(s)) => self.gpkg_view(s),
            Some(Window::GpkgTilesRead(s)) => self.gpkg_tiles_read_view(s),
            Some(Window::GpkgTilesWrite(s)) => self.gpkg_tiles_write_view(s),
            Some(Window::Postgis(s)) => self.postgis_view(s),
            Some(Window::ShpExport(s)) => self.shp_export_view(s),
            Some(Window::CsvExport(s)) => self.csv_export_view(s),
            Some(Window::AtlasExport(s)) => self.atlas_export_view(s),
            Some(Window::CsvImport(s)) => self.csv_import_view(s),
            Some(Window::FgbExport(s)) => self.fgb_export_view(s),
            Some(Window::FgbImport(s)) => self.fgb_import_view(s),
            Some(Window::GmlExport(s)) => self.gml_export_view(s),
            Some(Window::GmlImport(s)) => self.gml_import_view(s),
            Some(Window::XyzExport(s)) => self.xyz_export_view(s),
            Some(Window::CloudRegion(s)) => self.region_view(s),
            Some(Window::CloudProfile(s)) => self.profile_view(s),
            Some(Window::MeshQuery(s)) => self.mesh_view(s),
            Some(Window::GisExport(s)) => self.gis_export_view(s),
            Some(Window::GisCheck(s)) => self.gis_check_view(s),
            Some(Window::PdfImport(s)) => self.pdf_import_view(s),
            None => iced::widget::text("").into(),
        }
    }

    /// Shows what an import added: its extent without far strays
    /// (`kentos_formats::import::view_bounds`), never smaller than a few
    /// metres, so one point alone is shown in context (web `zoomToImported`).
    pub(crate) fn zoom_to(&mut self, slots: &[Slot]) {
        let Some(doc) = &self.document else { return };
        let boxes: Vec<kentos_commands::Bounds> = slots
            .iter()
            .filter_map(|s| doc.model.get(*s))
            .filter_map(|e| scene::extents_of(doc, std::iter::once(e)))
            .map(|b| kentos_commands::Bounds {
                min_x: b.min_x,
                min_y: b.min_y,
                max_x: b.max_x,
                max_y: b.max_y,
            })
            .collect();
        if let Some(b) = kentos_formats::import::view_bounds(&boxes) {
            self.zoom_to_bounds(&b);
        }
    }

    /// Shows `b`, never smaller than a few metres across.
    pub(crate) fn zoom_to_bounds(&mut self, b: &kentos_commands::Bounds) {
        /// At least this much around what was imported.
        const MIN_SPAN: f64 = 20.0;
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
            Some(Window::FieldSend(_)) => self.field_send_written(outcome),
            Some(Window::Wgs84Export(_)) => self.wgs84_export_written(outcome),
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
        path: Some(path.clone()),
    })
}

/// Writes an export whole or not at all (docs/adr/0253 §10): a temporary file
/// beside the target, made durable, read back, renamed over it. A failed write
/// leaves the file that was there as it was.
fn write_file(path: &std::path::Path, bytes: &[u8]) -> Result<String, String> {
    kentos_headless::files::write_bytes(bytes, path)
        .map_err(|e| format!("{} Başka bir yere kaydetmeyi deneyin.", e.message))?;
    Ok(path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    ))
}
