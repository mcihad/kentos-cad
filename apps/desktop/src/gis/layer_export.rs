//! Katmanlar › Dışa aktar › Farklı kaydet… and Seçilenleri farklı kaydet… (docs/adr/9106 §4): the window that writes
//! a layer through `cad.layers.export`.
//!
//! Its parts, top to bottom: Biçim, Dosya (Gözat…) or Bağlantı and Şema, Tablo adı, Koordinat sistemi, Kodlama; what
//! is written (only the selected, only what the view shows, a condition); Alanlar (each one on or off, its name, its
//! listed values' labels); Geometri (kind, multi-part, Z, M); the format's own choices; what is done when the target is
//! there; Üst veriyi yaz, Haritaya ekle. Only what the format has is shown; a layer with many fields keeps them in a
//! list of its own height that scrolls. The export runs off the interface's thread (a data layer's million rows are
//! read and written there): the window's foot says how far it is and Durdur stops it, taking back what it wrote. The
//! data layer it adds when asked is added once it is back, as its own undo step.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc;
use iced::widget::{Column, column, container, row, scrollable};
use iced::{Element, Length, Task};
use kentos_commands::{
    CommandWarning, ExportEncoding, ExportExisting, ExportField, ExportFormat, ExportGeometry, ExportOptions,
    LayersData, LayersExport, LayersExported,
};
use kentos_native_application::ExecutionContext;
use kentos_native_application::layer_export::ExportProgress;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::progress;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::switch::Switch;
use kentos_ui::widget::{Dialog, Form, overlay};

use crate::app::{App, Message};
use crate::exchange::words;
use crate::layer_export_extent::{BAD_BOX, EXTENTS, Extent, Sides, box_of, extent_label, sides_view};

/// The exports started: a late answer of one the window no longer waits for is dropped.
static RUNS: AtomicU64 = AtomicU64::new(0);
/// Least time between two progress messages of an export.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);
/// The field list's most height: about eight fields, then it scrolls.
const FIELDS_HEIGHT: f32 = 264.0;

pub(crate) const FORMATS: [ExportFormat; 6] = [
    ExportFormat::Gpkg,
    ExportFormat::Postgis,
    ExportFormat::Geojson,
    ExportFormat::Shapefile,
    ExportFormat::Csv,
    ExportFormat::Kml,
];
const GEOMETRIES: [ExportGeometry; 5] = [
    ExportGeometry::Auto,
    ExportGeometry::Point,
    ExportGeometry::Line,
    ExportGeometry::Area,
    ExportGeometry::None,
];
const EXISTING: [ExportExisting; 4] = [
    ExportExisting::Refuse,
    ExportExisting::OverwriteFile,
    ExportExisting::ReplaceLayer,
    ExportExisting::Append,
];

fn format_label(f: ExportFormat) -> &'static str {
    match f {
        ExportFormat::Gpkg => "GeoPackage",
        ExportFormat::Postgis => "PostGIS tablosu",
        ExportFormat::Geojson => "GeoJSON",
        ExportFormat::Shapefile => "Shapefile",
        ExportFormat::Csv => "CSV (virgülle ayrılmış değerler)",
        ExportFormat::Kml => "KML",
    }
}

fn extension(f: ExportFormat) -> &'static str {
    match f {
        ExportFormat::Gpkg => "gpkg",
        ExportFormat::Geojson => "geojson",
        ExportFormat::Shapefile => "shp",
        ExportFormat::Csv => "csv",
        ExportFormat::Kml => "kml",
        ExportFormat::Postgis => "",
    }
}

fn geometry_label(g: ExportGeometry) -> &'static str {
    match g {
        ExportGeometry::Auto => "Katmanınki",
        ExportGeometry::Point => "Nokta",
        ExportGeometry::Line => "Çizgi",
        ExportGeometry::Area => "Alan",
        ExportGeometry::None => "Geometrisiz (yalnız alanlar)",
    }
}

fn existing_label(e: ExportExisting) -> &'static str {
    match e {
        ExportExisting::Refuse => "Yazma (dosya ya da tablo varsa dur)",
        ExportExisting::OverwriteFile => "Dosyanın üstüne yaz",
        ExportExisting::ReplaceLayer => "Katmanı değiştir",
        ExportExisting::Append => "Katmana ekle",
    }
}

/// Where it was written, short: a GeoPackage's file and table, PostGIS's table, a file's name.
fn place_of(out: &LayersExported) -> String {
    let file = out
        .files
        .first()
        .map(|f| Path::new(f).file_name().map_or_else(|| f.clone(), |n| n.to_string_lossy().into_owned()));
    match (out.table.clone(), file) {
        (Some(t), Some(f)) => format!("{f} › {t}"),
        (Some(t), None) => t,
        (None, Some(f)) => f,
        (None, None) => String::new(),
    }
}

/// The geometry written, as the interface names it: `MULTIPOLYGON Z` is “çok parçalı alan, Z”.
fn geometry_words(written: Option<&str>) -> String {
    let Some(written) = written else {
        return "geometrisiz".to_owned();
    };
    let mut parts = written.split(' ');
    let kind = parts.next().unwrap_or_default();
    let (multi, base) = match kind.strip_prefix("MULTI") {
        Some(rest) => (true, rest),
        None => (false, kind),
    };
    let word = match base {
        "POINT" => "nokta",
        "LINESTRING" | "CURVE" => "çizgi",
        "POLYGON" | "SURFACE" => "alan",
        _ => "karışık geometri",
    };
    let mut out = if multi { format!("çok parçalı {word}") } else { word.to_owned() };
    if let Some(dims) = parts.next() {
        out.push_str(", ");
        out.push_str(dims);
    }
    out
}

/// A field's row in the window.
#[derive(Debug, Clone)]
pub(crate) struct FieldRow {
    name: String,
    /// Its kind as the interface names it.
    kind: &'static str,
    on: bool,
    to: String,
    labels: bool,
    /// It has listed values whose labels can be written.
    listed: bool,
}

/// The window's state.
#[derive(Debug, Clone)]
pub(crate) struct ExportWindow {
    layer: String,
    title: String,
    selected: bool,
    format: ExportFormat,
    path: String,
    connection: Option<usize>,
    schema: String,
    name: String,
    /// The system written; none: the data layer's own, or the project's (system_select.rs).
    srid: Option<u32>,
    encoding: ExportEncoding,
    /// Which objects: all, those in the view, those touching the box (layer_export_extent.rs).
    extent: Extent,
    sides: Sides,
    /// The window stepped aside while the box is drawn on the map.
    drawing: bool,
    filter: String,
    fields: Vec<FieldRow>,
    geometry: ExportGeometry,
    multi: bool,
    z: bool,
    m: bool,
    existing: ExportExisting,
    description: String,
    key: String,
    geometry_column: String,
    spatial_index: bool,
    separator: String,
    xy: bool,
    project_coordinates: bool,
    metadata: bool,
    add_to_map: bool,
    running: bool,
    /// The export under way: its number, how far it is, the flag Durdur sets, whether Durdur was pressed.
    run: u64,
    progress: Option<ExportProgress>,
    stop: Option<Arc<AtomicBool>>,
    stopping: bool,
    said: Option<String>,
    /// What is said is Durdur's answer, not a failure: said as information.
    said_stopped: bool,
}

/// An export back: what was written and said, the data layer to add; or why not.
pub type Exported = Result<(LayersExported, Vec<CommandWarning>, Option<LayersData>), String>;

/// What the window says.
#[derive(Debug, Clone)]
pub enum Event {
    /// Farklı kaydet… on a layer (true: Seçilenleri farklı kaydet…).
    Open(String, bool),
    Format(usize),
    Path(String),
    Browse,
    Browsed(Option<PathBuf>),
    Connection(usize),
    Schema(String),
    Name(String),
    Srid(Option<u32>),
    Encoding(usize),
    Selected(bool),
    Extent(usize),
    /// A side of the box typed: west, south, east, north.
    Side(usize, String),
    BoxFromView,
    BoxFromLayer,
    DrawBox,
    Filter(String),
    FieldOn(usize, bool),
    FieldTo(usize, String),
    FieldLabels(usize, bool),
    AllFields(bool),
    Geometry(usize),
    Multi(bool),
    Z(bool),
    M(bool),
    Existing(usize),
    Description(String),
    Key(String),
    GeometryColumn(String),
    SpatialIndex(bool),
    Separator(String),
    Xy(bool),
    ProjectCoordinates(bool),
    Metadata(bool),
    AddToMap(bool),
    Run,
    /// How far the export (its number) is.
    Progress(u64, ExportProgress),
    /// Durdur: the export stops and what it wrote is taken back.
    Stop,
    /// The export (its number) back.
    Done(u64, Box<Exported>),
    Close,
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::LayerExport(e))
}

impl ExportWindow {
    /// The layer exported.
    pub(crate) fn layer_id(&self) -> &str {
        &self.layer
    }

    /// The box's sides set (from the view, the layer, the map): the box is the extent now.
    pub(crate) fn set_box(&mut self, sides: Sides) {
        self.sides = sides;
        self.extent = Extent::Box;
        self.said = None;
    }

    pub(crate) fn set_drawing(&mut self, on: bool) {
        self.drawing = on;
    }

    pub(crate) fn is_drawing(&self) -> bool {
        self.drawing
    }

    #[cfg(test)]
    pub(crate) fn is_running(&self) -> bool {
        self.running
    }

    #[cfg(test)]
    pub(crate) fn extent(&self) -> Extent {
        self.extent
    }

    fn table(&self) -> bool {
        matches!(self.format, ExportFormat::Gpkg | ExportFormat::Postgis)
    }

    /// Why the export did not write, as the window's foot says it.
    #[cfg(test)]
    pub(crate) fn said(&self) -> Option<&str> {
        self.said.as_deref()
    }
}

/// The format writes WGS 84 whatever is asked: KML, and GeoJSON unless the project's coordinates are asked.
fn wgs84_only(w: &ExportWindow) -> bool {
    w.format == ExportFormat::Kml || (w.format == ExportFormat::Geojson && !w.project_coordinates)
}

impl App {
    /// The system written when none is chosen, as the picker's first row names it: a data layer's own, else the
    /// project's.
    fn export_default_system(&self, w: &ExportWindow) -> (String, Option<u32>) {
        let Some(doc) = &self.document else {
            return ("Varsayılan".to_owned(), None);
        };
        let own = doc
            .model
            .layers()
            .get(&w.layer)
            .and_then(|n| n.data.as_ref())
            .and_then(|d| d.geometry.as_ref())
            .and_then(|g| g.srid);
        let project = doc.model.settings().srid;
        let named = |srid: u32| kentos_project::crs::system(srid).map_or_else(|| format!("EPSG:{srid}"), |s| format!("{} (EPSG:{srid})", s.name));
        match own {
            Some(srid) => (format!("Katmanın sistemi · {}", named(srid)), Some(srid)),
            None if project > 0 => (format!("Projenin sistemi · {}", named(project)), Some(project)),
            None => ("Projenin sistemi (yerel, koordinat sistemi yok)".to_owned(), None),
        }
    }

    /// Farklı kaydet… (`selected`: Seçilenleri farklı kaydet…) on a layer.
    pub(crate) fn open_layer_export(&mut self, layer: &str, selected: bool) {
        let Some(node) = self.document.as_ref().and_then(|d| d.model.layers().get(layer)).cloned() else {
            return;
        };
        let fields = node
            .fields
            .iter()
            .map(|f| FieldRow {
                name: f.name.clone(),
                kind: f.kind.label(),
                on: true,
                to: f.name.clone(),
                labels: false,
                listed: f.values.as_ref().is_some_and(|v| !v.is_empty()),
            })
            .collect();
        self.gis.layer_export = Some(ExportWindow {
            layer: node.id.clone(),
            title: node.name.clone(),
            selected,
            format: ExportFormat::Gpkg,
            path: String::new(),
            connection: None,
            schema: "public".into(),
            name: kentos_native_application::layer_export::table_name_of(&node.name),
            srid: None,
            encoding: ExportEncoding::Utf8,
            extent: Extent::All,
            sides: Sides::default(),
            drawing: false,
            filter: String::new(),
            fields,
            geometry: ExportGeometry::Auto,
            multi: false,
            z: false,
            m: false,
            existing: ExportExisting::Refuse,
            description: String::new(),
            key: "fid".into(),
            geometry_column: "geom".into(),
            spatial_index: true,
            separator: ",".into(),
            xy: false,
            project_coordinates: false,
            metadata: false,
            add_to_map: true,
            running: false,
            run: 0,
            progress: None,
            stop: None,
            stopping: false,
            said: None,
            said_stopped: false,
        });
        self.dialog = Some(crate::app::Dialog::Gis(crate::gis::GisDialogKind::LayerExport));
    }

    /// The selected objects' ids (a drawing layer) or the table's selected rows' keys (a data layer).
    fn export_selection(&self, layer: &str) -> (Option<Vec<String>>, Option<Vec<i64>>) {
        let Some(doc) = &self.document else {
            return (None, None);
        };
        let Some(node) = doc.model.layers().get(layer) else {
            return (None, None);
        };
        if node.data.is_some() {
            return (None, Some(self.gis.features.data.selected_of(layer)));
        }
        let uids = self
            .selection
            .ids()
            .iter()
            .filter(|s| doc.model.get(**s).is_some_and(|e| e.base().layer_id == layer))
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        (Some(uids), None)
    }

    /// Whether the layer has anything selected (Seçilenleri farklı kaydet… is greyed out otherwise).
    pub(crate) fn layer_has_selection(&self, layer: &str) -> bool {
        let Some(doc) = &self.document else {
            return false;
        };
        if doc.model.layers().get(layer).is_some_and(|n| n.data.is_some()) {
            return self.gis.features.data.any_selected_of(layer);
        }
        self.selection
            .ids()
            .iter()
            .any(|s| doc.model.get(*s).is_some_and(|e| e.base().layer_id == layer))
    }

    /// The command's input for the window as it is.
    pub(crate) fn export_input(&self, w: &ExportWindow) -> LayersExport {
        let (uids, keys) = if w.selected { self.export_selection(&w.layer) } else { (None, None) };
        let bbox = match w.extent {
            Extent::All => None,
            Extent::View => {
                let b = self.viewport.camera.visible_bounds();
                Some([b.min_x, b.min_y, b.max_x, b.max_y])
            }
            Extent::Box => box_of(&w.sides),
        };
        let address = (w.format == ExportFormat::Postgis)
            .then(|| {
                w.connection
                    .and_then(|i| kentos_postgis::connections::store().list().get(i).map(|c| c.address()))
            })
            .flatten();
        let fields: Vec<ExportField> = w
            .fields
            .iter()
            .filter(|f| f.on)
            .map(|f| ExportField {
                name: f.name.clone(),
                to: (f.to.trim() != f.name && !f.to.trim().is_empty()).then(|| f.to.trim().to_owned()),
                labels: (f.listed && f.labels).then_some(true),
            })
            .collect();
        let table = w.table();
        let options = ExportOptions {
            description: (table && !w.description.trim().is_empty()).then(|| w.description.trim().to_owned()),
            key: (table && w.key.trim() != "fid" && !w.key.trim().is_empty()).then(|| w.key.trim().to_owned()),
            geometry_column: (table && w.geometry_column.trim() != "geom" && !w.geometry_column.trim().is_empty())
                .then(|| w.geometry_column.trim().to_owned()),
            no_spatial_index: (table && !w.spatial_index).then_some(true),
            separator: (w.format == ExportFormat::Csv && w.separator != ",").then(|| w.separator.clone()),
            xy: (w.format == ExportFormat::Csv && w.xy).then_some(true),
            project_coordinates: (w.format == ExportFormat::Geojson && w.project_coordinates).then_some(true),
        };
        let empty = options == ExportOptions::default();
        LayersExport {
            layer: w.layer.clone(),
            format: w.format,
            path: (w.format != ExportFormat::Postgis).then(|| w.path.trim().to_owned()),
            address,
            schema: (w.format == ExportFormat::Postgis).then(|| w.schema.trim().to_owned()),
            name: (table || w.format == ExportFormat::Kml).then(|| w.name.trim().to_owned()).filter(|n| !n.is_empty()),
            srid: w.srid.filter(|_| !wgs84_only(w)),
            encoding: matches!(w.format, ExportFormat::Shapefile | ExportFormat::Csv)
                .then_some(w.encoding)
                .filter(|e| *e != ExportEncoding::Utf8),
            uids,
            keys,
            bbox,
            filter: (!w.filter.trim().is_empty()).then(|| w.filter.trim().to_owned()),
            fields: Some(fields),
            geometry: (w.geometry != ExportGeometry::Auto).then_some(w.geometry),
            multi: w.multi.then_some(true),
            z: w.z.then_some(true),
            m: w.m.then_some(true),
            existing: (w.existing != ExportExisting::Refuse).then_some(w.existing),
            options: (!empty).then_some(options),
            metadata: (w.format == ExportFormat::Gpkg && w.metadata).then_some(true),
            add_to_map: (table && w.add_to_map).then_some(true),
        }
    }

    pub(crate) fn layer_export_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Open(layer, selected) = &event {
            self.open_layer_export(layer, *selected);
            return Task::none();
        }
        let Some(w) = self.gis.layer_export.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Format(i) => {
                w.format = FORMATS[i.min(FORMATS.len() - 1)];
                if !w.table() && matches!(w.existing, ExportExisting::ReplaceLayer | ExportExisting::Append) {
                    w.existing = ExportExisting::Refuse;
                }
                // The file's extension follows the format.
                if !w.path.trim().is_empty() && w.format != ExportFormat::Postgis {
                    let p = PathBuf::from(w.path.trim()).with_extension(extension(w.format));
                    w.path = p.display().to_string();
                }
            }
            Event::Path(p) => w.path = p,
            Event::Browse => {
                let (format, title) = (w.format, w.title.clone());
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title("Farklı kaydet")
                            .set_file_name(format!(
                                "{}.{}",
                                kentos_native_application::layer_export::table_name_of(&title),
                                extension(format)
                            ))
                            .add_filter(format_label(format), &[extension(format)])
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| message(Event::Browsed(p)),
                );
            }
            Event::Browsed(Some(p)) => w.path = p.display().to_string(),
            Event::Browsed(None) => {}
            Event::Connection(i) => w.connection = Some(i),
            Event::Schema(s) => w.schema = s,
            Event::Name(n) => w.name = n,
            Event::Srid(s) => w.srid = s,
            Event::Encoding(i) => {
                w.encoding = if i == 1 { ExportEncoding::Windows1254 } else { ExportEncoding::Utf8 }
            }
            Event::Selected(on) => w.selected = on,
            Event::Extent(i) => {
                w.extent = EXTENTS[i.min(EXTENTS.len() - 1)];
                w.said = None;
            }
            Event::Side(i, t) => {
                if let Some(side) = w.sides.get_mut(i) {
                    *side = t;
                }
            }
            Event::BoxFromView => self.export_box_from_view(),
            Event::BoxFromLayer => self.export_box_from_layer(),
            Event::DrawBox => self.export_draw_box(),
            Event::Filter(f) => w.filter = f,
            Event::FieldOn(i, on) => {
                if let Some(f) = w.fields.get_mut(i) {
                    f.on = on;
                }
            }
            Event::FieldTo(i, to) => {
                if let Some(f) = w.fields.get_mut(i) {
                    f.to = to;
                }
            }
            Event::FieldLabels(i, on) => {
                if let Some(f) = w.fields.get_mut(i) {
                    f.labels = on;
                }
            }
            Event::AllFields(on) => w.fields.iter_mut().for_each(|f| f.on = on),
            Event::Geometry(i) => w.geometry = GEOMETRIES[i.min(GEOMETRIES.len() - 1)],
            Event::Multi(on) => w.multi = on,
            Event::Z(on) => w.z = on,
            Event::M(on) => w.m = on,
            Event::Existing(i) => w.existing = EXISTING[i.min(EXISTING.len() - 1)],
            Event::Description(d) => w.description = d,
            Event::Key(k) => w.key = k,
            Event::GeometryColumn(g) => w.geometry_column = g,
            Event::SpatialIndex(on) => w.spatial_index = on,
            Event::Separator(s) => w.separator = s.chars().last().map(String::from).unwrap_or_default(),
            Event::Xy(on) => w.xy = on,
            Event::ProjectCoordinates(on) => w.project_coordinates = on,
            Event::Metadata(on) => w.metadata = on,
            Event::AddToMap(on) => w.add_to_map = on,
            Event::Close => {
                // Closed while it writes: the export stops (nothing of it is kept).
                if let Some(stop) = &w.stop {
                    stop.store(true, Ordering::Relaxed);
                }
                self.gis.layer_export = None;
                self.dialog = None;
            }
            Event::Open(..) => {}
            Event::Run => return self.run_layer_export(),
            Event::Progress(run, p) => {
                if w.running && w.run == run {
                    w.progress = Some(p);
                }
            }
            Event::Stop => {
                if let Some(stop) = &w.stop {
                    stop.store(true, Ordering::Relaxed);
                    w.stopping = true;
                }
            }
            Event::Done(run, answer) => {
                if w.run == run {
                    return self.layer_export_done(*answer);
                }
            }
        }
        Task::none()
    }

    /// Dışa aktar: checked at once (a refusal stays in the window), then written off the thread.
    fn run_layer_export(&mut self) -> Task<Message> {
        let Some(w) = self.gis.layer_export.clone() else {
            return Task::none();
        };
        // A box that is not four numbers around a place is said here: the command would take it as no box at all.
        if w.extent == Extent::Box && box_of(&w.sides).is_none() {
            if let Some(w) = self.gis.layer_export.as_mut() {
                w.said = Some(BAD_BOX.to_owned());
                w.said_stopped = false;
            }
            return Task::none();
        }
        let input = self.export_input(&w);
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        if let kentos_commands::CommandResult::Failed { error } =
            kentos_native_application::layer_export::validate(&ExecutionContext::new(&mut doc.model), &input)
        {
            if let Some(w) = self.gis.layer_export.as_mut() {
                w.said = Some(error.message);
                w.said_stopped = false;
            }
            return Task::none();
        }
        let copy = doc.model.reading_copy();
        let run = RUNS.fetch_add(1, Ordering::Relaxed) + 1;
        let stop = Arc::new(AtomicBool::new(false));
        if let Some(w) = self.gis.layer_export.as_mut() {
            w.running = true;
            w.run = run;
            w.progress = None;
            w.stop = Some(stop.clone());
            w.stopping = false;
            w.said = None;
            w.said_stopped = false;
        }
        let (out, replies) = mpsc::unbounded::<Event>();
        let spawned = std::thread::Builder::new().name("kentos-disa-aktar".into()).spawn(move || {
            let told = out.clone();
            let last: std::cell::Cell<Option<Instant>> = std::cell::Cell::new(None);
            let answer = kentos_native_application::layer_export::written(
                &copy,
                &input,
                &|| stop.load(Ordering::Relaxed),
                &|p| {
                    let now = Instant::now();
                    if last.get().is_none_or(|t| now.duration_since(t) >= PROGRESS_EVERY) {
                        last.set(Some(now));
                        let _ = told.unbounded_send(Event::Progress(run, p));
                    }
                },
            );
            let _ = out.unbounded_send(Event::Done(run, Box::new(answer)));
        });
        match spawned {
            Ok(_) => Task::run(replies, message),
            Err(e) => Task::done(message(Event::Done(
                run,
                Box::new(Err(format!("Dışa aktarılamadı: iş parçacığı başlatılamadı ({e})."))),
            ))),
        }
    }

    /// The export back: said; the data layer added when asked; the window closed. A failure stays in the window.
    fn layer_export_done(&mut self, answer: Exported) -> Task<Message> {
        match answer {
            Ok((out, warnings, add)) => {
                for w in warnings {
                    self.warn(w.message);
                }
                let words = format!(
                    "{} {}: {} nesne yazıldı ({}).",
                    format_label(out.format),
                    place_of(&out),
                    crate::view::thousands(out.written as f64),
                    geometry_words(out.geometry.as_deref())
                );
                if let Some(input) = add
                    && let Some(doc) = self.document.as_mut()
                {
                    let result =
                        kentos_native_application::layers_data::execute(&mut ExecutionContext::new(&mut doc.model), input);
                    self.answered(result);
                }
                self.say(kentos_interaction::Level::Success, words.clone());
                self.toast_saved("Dışa aktarıldı", words);
                self.gis.layer_export = None;
                self.dialog = None;
            }
            Err(why) => {
                if let Some(w) = self.gis.layer_export.as_mut() {
                    w.running = false;
                    w.progress = None;
                    w.stop = None;
                    w.said_stopped = w.stopping;
                    w.stopping = false;
                    w.said = Some(why);
                }
            }
        }
        Task::none()
    }

    /// The window.
    pub(crate) fn layer_export_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.gis.layer_export else {
            return column![].into();
        };
        let text = |placeholder: &str, value: &str, on: fn(String) -> Event| -> Element<'_, Message> {
            kentos_ui::widget::text_field(placeholder, value).on_input(move |t| message(on(t))).into()
        };
        let switch = |on: bool, label: &'static str, event: fn(bool) -> Event| -> Element<'_, Message> {
            Switch::new(on, move |b| message(event(b))).label(label).into()
        };
        let select = |choices: Vec<&'static str>, at: Option<usize>, event: fn(usize) -> Event| -> Element<'_, Message> {
            container(Select::new(choices.into_iter().map(Choice::new), at, move |i| message(event(i))).searchable(false))
                .width(320)
                .into()
        };
        let mut form = Form::new().label_width(170.0).field(
            "Biçim",
            select(FORMATS.iter().map(|f| format_label(*f)).collect(), FORMATS.iter().position(|f| *f == w.format), Event::Format),
        );
        if w.format == ExportFormat::Postgis {
            let list = kentos_postgis::connections::store().list();
            let names: Vec<String> = list.iter().map(|c| c.name.clone()).collect();
            let picker: Element<'_, Message> = if names.is_empty() {
                kentos_ui::label::muted("Kayıtlı bağlantı yok: önce Katman ekle › PostGIS'e bağlan ile bir bağlantı kurun.").into()
            } else {
                container(
                    Select::new(names.into_iter().map(Choice::new), w.connection, |i| message(Event::Connection(i)))
                        .searchable(false),
                )
                .width(320)
                .into()
            };
            form = form.field("Bağlantı", picker).field("Şema", text("public", &w.schema, Event::Schema));
        } else {
            form = form.field(
                "Dosya",
                row![
                    container(text("Yazılacak dosyanın yolu", &w.path, Event::Path)).width(Length::Fill),
                    words::secondary("Gözat…", Some(message(Event::Browse))),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            );
        }
        if w.table() || w.format == ExportFormat::Kml {
            form = form.field(if w.table() { "Tablo adı" } else { "Belge adı" }, text("", &w.name, Event::Name));
        }
        if wgs84_only(w) {
            let why = if w.format == ExportFormat::Kml {
                "KML her zaman WGS 84 (boylam, enlem) yazılır."
            } else {
                "GeoJSON WGS 84 (boylam, enlem) yazılır; başka bir sistem için aşağıdaki Proje koordinatları seçeneğini açın."
            };
            form = form.field("Koordinat sistemi", kentos_ui::label::muted("WGS 84 (EPSG:4326)")).help(why);
        } else {
            let (label, own) = self.export_default_system(w);
            let project = self.document.as_ref().map_or(0, |d| d.model.settings().srid);
            let picker = container(crate::system_select::select(
                crate::system_select::rows(label, own, project),
                w.srid,
                |s| message(Event::Srid(s)),
            ))
            .width(420);
            form = form
                .field("Koordinat sistemi", picker)
                .help("Satırlar yazılırken bu sisteme çevrilir; varsayılan katmanın kendi sistemidir.");
        }
        if matches!(w.format, ExportFormat::Shapefile | ExportFormat::Csv) {
            form = form.field(
                "Kodlama",
                select(
                    vec!["UTF-8", "Windows-1254 (Türkçe Windows)"],
                    Some(usize::from(w.encoding == ExportEncoding::Windows1254)),
                    Event::Encoding,
                ),
            );
        }
        form = form
            .section("Yazılacak nesneler")
            .row(switch(w.selected, "Yalnız seçilenleri yaz", Event::Selected))
            .field(
                "Kapsam",
                select(EXTENTS.iter().map(|e| extent_label(*e)).collect(), EXTENTS.iter().position(|e| *e == w.extent), Event::Extent),
            );
        if w.extent == Extent::Box {
            form = form.wide(sides_view(
                &w.sides,
                self.export_axis_names(),
                |i, t| message(Event::Side(i, t)),
                message(Event::BoxFromView),
                message(Event::BoxFromLayer),
                message(Event::DrawBox),
            ));
        }
        form = form.field("Süzgeç (İfadeyle seç'in dili)", text("örn. kullanim = 'Konut'", &w.filter, Event::Filter));
        // Alanlar: each on or off, its kind, its written name, its labels when it has a list. The heads stay; the
        // rows scroll in a list of their own height, so a layer of many fields does not stretch the window.
        let on = w.fields.iter().filter(|f| f.on).count();
        let tools = row![
            words::ghost("Tümünü seç", Some(message(Event::AllFields(true)))),
            words::ghost("Hiçbirini seçme", Some(message(Event::AllFields(false)))),
            container(kentos_ui::label::muted(format!("{} alandan {} tanesi yazılacak", w.fields.len(), on)))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);
        let heads = kentos_ui::widget::table::band(
            row![
                container(kentos_ui::label::caption("Alan")).width(200),
                container(kentos_ui::label::caption("Türü")).width(110),
                container(kentos_ui::label::caption("Yazılacak adı")).width(200),
            ]
            .spacing(12)
            .padding([0, 8]),
        );
        // Room on the right for the list's own bar, below so the last row is not cut by the frame.
        let mut rows = Column::new().spacing(4).padding(iced::Padding {
            top: 6.0,
            right: 18.0,
            bottom: 10.0,
            left: 8.0,
        });
        for (i, f) in w.fields.iter().enumerate() {
            let mut line = row![
                container(Switch::new(f.on, move |b| message(Event::FieldOn(i, b))).label(f.name.as_str())).width(200),
                container(kentos_ui::label::muted(f.kind)).width(110),
                container(kentos_ui::widget::text_field("Yazılacak adı", &f.to).on_input(move |t| message(Event::FieldTo(i, t))))
                    .width(200),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center);
            if f.listed {
                line = line.push(Switch::new(f.labels, move |b| message(Event::FieldLabels(i, b))).label("Görünen değer"));
            }
            rows = rows.push(line);
        }
        let list = container(column![
            heads,
            // The bar at the frame's right edge: the list as wide as the frame.
            container(
                scrollable(rows.width(Length::Fill))
                    .direction(style::field::body_scrollbar())
                    .width(Length::Fill)
                    .height(Length::Shrink)
            )
            .max_height(typography::scaled(FIELDS_HEIGHT)),
        ])
        .width(Length::Fill)
        .padding(1)
        .style(style::container::bordered);
        form = form.section("Alanlar").wide(column![tools, list].spacing(6));
        form = form
            .section("Geometri")
            .field(
                "Geometri türü",
                select(GEOMETRIES.iter().map(|g| geometry_label(*g)).collect(), GEOMETRIES.iter().position(|g| *g == w.geometry), Event::Geometry),
            )
            .row(switch(w.multi, "Çok parçalıya zorla", Event::Multi))
            .row(switch(w.z, "Z boyutunu yaz (kotsuz nesne 0)", Event::Z))
            .row(switch(w.m, "M boyutunu yaz", Event::M));
        match w.format {
            ExportFormat::Gpkg | ExportFormat::Postgis => {
                form = form
                    .section("Tablo seçenekleri")
                    .field("Açıklama", text("", &w.description, Event::Description))
                    .field("Anahtar sütunu", text("fid", &w.key, Event::Key))
                    .field("Geometri sütunu", text("geom", &w.geometry_column, Event::GeometryColumn))
                    .row(switch(w.spatial_index, "Mekânsal dizin yap", Event::SpatialIndex));
            }
            ExportFormat::Csv => {
                form = form
                    .section("CSV seçenekleri")
                    .field("Ayırıcı", text(",", &w.separator, Event::Separator))
                    .row(switch(w.xy, "Noktaları X ve Y sütunlarıyla yaz (WKT yerine)", Event::Xy));
            }
            ExportFormat::Geojson => {
                form = form
                    .section("GeoJSON seçenekleri")
                    .row(switch(w.project_coordinates, "Proje koordinatlarıyla yaz (eski crs üyesiyle)", Event::ProjectCoordinates));
            }
            _ => {}
        }
        let existing: Vec<&'static str> = EXISTING
            .iter()
            .filter(|e| w.table() || !matches!(e, ExportExisting::ReplaceLayer | ExportExisting::Append))
            .filter(|e| !(w.format == ExportFormat::Postgis && **e == ExportExisting::OverwriteFile))
            .map(|e| existing_label(*e))
            .collect();
        let at = existing.iter().position(|l| *l == existing_label(w.existing));
        form = form.section("Hedef varsa").field(
            "Ne yapılsın",
            container(
                Select::new(existing.clone().into_iter().map(Choice::new), at, move |i| {
                    let label = existing.get(i).copied().unwrap_or_default();
                    let pos = EXISTING.iter().position(|e| existing_label(*e) == label).unwrap_or(0);
                    message(Event::Existing(pos))
                })
                .searchable(false),
            )
            .width(320),
        );
        if w.format == ExportFormat::Gpkg {
            form = form.row(switch(w.metadata, "Katmanın veri kümesi bilgisini üst veri olarak yaz", Event::Metadata));
        }
        if w.table() {
            form = form.row(switch(w.add_to_map, "Yazılan tabloyu haritaya ekle", Event::AddToMap));
        }
        let title = if w.selected {
            format!("Seçilenleri farklı kaydet — {}", w.title)
        } else {
            format!("Farklı kaydet — {}", w.title)
        };
        let mut dialog = Dialog::new(title).scroll(form);
        // The foot: how far the export is while it writes, why it did not otherwise (where it is seen, not at the
        // end of a long form).
        if w.running {
            dialog = dialog
                .status(running_status(w))
                .action(words::secondary("Durdur", (!w.stopping).then(|| message(Event::Stop))))
                .action(words::primary("Dışa aktar", None));
        } else {
            if let Some(said) = &w.said {
                let kind = if w.said_stopped { words::Kind::Info } else { words::Kind::Error };
                dialog = dialog.status(words::text_line(kind, said.clone()));
            }
            dialog = dialog
                .action(words::secondary("Vazgeç", Some(message(Event::Close))))
                .action(words::primary("Dışa aktar", Some(message(Event::Run))));
        }
        overlay::blocking(dialog.width(760.0).max_height(820.0).on_close(Some(message(Event::Close))))
    }
}

/// The foot while the export writes: what is read of what it reads, what is written, the bar.
fn running_status(w: &ExportWindow) -> Element<'_, Message> {
    let thousands = |n: u64| crate::view::thousands(n as f64);
    let (said, share) = match (w.stopping, w.progress) {
        (true, _) => ("Durduruluyor; yazılan geri alınıyor…".to_owned(), None),
        (false, None) => ("Hazırlanıyor…".to_owned(), None),
        (false, Some(p)) => match p.total.filter(|t| *t > 0) {
            Some(total) => {
                let share = (p.read as f32 / total as f32).clamp(0.0, 1.0);
                (
                    format!(
                        "{} / {} kayıt okundu, {} nesne yazıldı (%{:.0})",
                        thousands(p.read),
                        thousands(total),
                        thousands(p.written),
                        share * 100.0
                    ),
                    Some(share),
                )
            }
            None => (format!("{} nesne yazıldı", thousands(p.written)), None),
        },
    };
    column![kentos_ui::label::caption(said), progress::bar(share)].spacing(4).width(Length::Fill).into()
}

