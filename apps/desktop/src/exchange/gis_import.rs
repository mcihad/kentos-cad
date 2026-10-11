//! GeoJSON, Shapefile ve KML/KMZ içe aktar (the web's
//! `ui/io/GisImportDialog.ts`, docs/adr/0046, 0053, 0222). The shared reader
//! reads the file once, off the UI thread (crates/shared/formats: geojson,
//! shp, kml), into points, lines, paths and areas with holes, their
//! attributes as text, and a report of what was converted or left out. A
//! Shapefile layer comes from its files chosen together, or from a zip
//! archive: each .shp in it is a layer, read one at a time. The window shows
//! the source's layers and where each goes (a project layer with the same
//! name, or a new layer in a group named after the file), the report, and
//! what the file says of its coordinate system: RFC 7946's WGS 84, a `crs`
//! member, a .prj or KML's WGS 84. A file in the project's system goes in
//! untouched; one in another system only when the user ticks “Projenin
//! sistemine çevir”, its objects then moved off the UI thread
//! (`kentos_formats::reproject`, docs/adr/0222 §2) before the window shows
//! what will come in. Nothing is transformed or guessed silently (CLAUDE.md
//! §5); an import is one undo step.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, column, row};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    GeoJsonReadOptions, ImportLayer, ImportResult, LayerStyle, ShapefileReadOptions,
};
use kentos_formats::shp;
use kentos_interaction::Level;
use kentos_ui::label;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog, overlay, swatch};

use super::apply::{self, ImportPlan, LayerTarget};
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Kind, Picked, Window, message, off_thread};
use crate::app::{App, Message};
use crate::crs::{CrsQuestion, Statement};
use crate::view::hex_color;

/// Reads told apart: a late answer to an older read is dropped.
static READS: AtomicU64 = AtomicU64::new(0);

/// A Shapefile's parts beside its .shp, in the order the web lists them.
const PARTS: [&str; 4] = ["shx", "dbf", "prj", "cpg"];

/// What the window reads.
#[derive(Debug, Clone)]
pub enum Source {
    GeoJson(Picked),
    /// A KML file or a KMZ archive (docs/adr/0222 §3).
    Kml(Picked),
    /// The files a user chose together for one Shapefile layer.
    Shapefile(Vec<Picked>),
    /// A zipped Shapefile: the archive, its layers (paths without extension,
    /// known once it is read) and the one read.
    Zip {
        file: Picked,
        layers: Vec<String>,
        layer: usize,
    },
}

/// A Shapefile layer among the files a user chose together (the web's
/// `shapefileSet`, io/shapefile.ts): the one .shp and the .shx, .dbf, .prj
/// and .cpg of its name (compared without case). A file of another name or
/// kind is not used, and said; two .shp files are two layers, imported one
/// at a time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    /// The .shp's name without extension: the layer's.
    pub name: String,
    /// Indices into the chosen files: the .shp, then [`PARTS`]'s.
    pub shp: usize,
    pub parts_at: [Option<usize>; 4],
    /// The extensions found (“.shp, .dbf, .shx”), in the order chosen.
    pub parts: Vec<String>,
    pub unused: Vec<String>,
}

fn extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, e)| e.to_lowercase())
        .unwrap_or_default()
}

pub fn shapefile_set(files: &[Picked]) -> Result<Set, String> {
    let shps: Vec<usize> = (0..files.len())
        .filter(|i| extension(&files[*i].name) == "shp")
        .collect();
    let Some(&shp) = shps.first() else {
        return Err("Seçilen dosyalarda .shp yok. Shapefile katmanının .shp dosyasını, yanındaki .shx, .dbf, .prj ve .cpg ile birlikte seçin ya da katmanın .zip arşivini seçin.".to_owned());
    };
    if shps.len() > 1 {
        let names: Vec<&str> = shps.iter().map(|i| files[*i].name.as_str()).collect();
        return Err(format!(
            "Seçilen dosyalarda {} .shp var ({}). Bir seferde bir katman alınır: “Başka dosya…” ile bir katmanın dosyalarını seçin.",
            shps.len(),
            names.join(", ")
        ));
    }
    let name = words::base_name(&files[shp].name);
    let key = name.to_lowercase();
    let mut set = Set {
        name,
        shp,
        parts_at: [None; 4],
        parts: vec![".shp".to_owned()],
        unused: Vec::new(),
    };
    for (i, f) in files.iter().enumerate() {
        if i == shp {
            continue;
        }
        let ext = extension(&f.name);
        let part = PARTS.iter().position(|p| *p == ext);
        match part {
            Some(p)
                if words::base_name(&f.name).to_lowercase() == key && set.parts_at[p].is_none() =>
            {
                set.parts_at[p] = Some(i);
                set.parts.push(format!(".{ext}"));
            }
            _ => set.unused.push(f.name.clone()),
        }
    }
    Ok(set)
}

/// The files a Shapefile pick brings: a layer's files, or one zip archive.
pub fn shapefile_source(files: Vec<Picked>) -> Result<Source, String> {
    let has_shp = files.iter().any(|f| extension(&f.name) == "shp");
    let zips: Vec<&Picked> = files
        .iter()
        .filter(|f| extension(&f.name) == "zip")
        .collect();
    match (has_shp, zips.as_slice()) {
        (false, [zip]) => Ok(Source::Zip {
            file: (*zip).clone(),
            layers: Vec::new(),
            layer: 0,
        }),
        (false, [_, _, ..]) => Err(format!(
            "Seçilen dosyalarda {} .zip var. Bir seferde bir arşiv alınır: “Başka dosya…” ile birini seçin.",
            zips.len()
        )),
        _ => Ok(Source::Shapefile(files)),
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub(super) source: Source,
    pub(super) read: u64,
    pub(super) reading: bool,
    pub(super) result: Option<Arc<ImportResult>>,
    pub(super) failed: Option<String>,
    /// The first object with a number that is not finite: nothing is imported.
    pub(super) unusable: Option<String>,
    /// Source layer names left out by the user.
    pub(super) excluded: BTreeSet<String>,
    pub(super) crs: CrsQuestion,
    /// The objects moved into the project's system, for the source system
    /// they were moved from (docs/adr/0222 §2); none until the move is asked.
    pub(super) moved: Option<(u32, Arc<ImportResult>)>,
    /// The move running: the read it moves and the source system it moves
    /// from. Another system chosen meanwhile is not waiting for it.
    pub(super) in_flight: Option<(u64, u32)>,
    /// What the footer says, and whether it is an error.
    pub(super) status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Read {
        read: u64,
        result: Result<Arc<ImportResult>, String>,
        unusable: Option<String>,
        /// A zip archive's layers, once listed.
        layers: Option<Vec<String>>,
    },
    /// Another layer of the zip archive.
    Layer(usize),
    Toggle(String),
    ToggleAll,
    Crs(u32),
    /// “Projenin sistemine çevir” ticked or not.
    Convert(bool),
    /// The objects moved from the system of `srid` (for the read `read`).
    Moved {
        read: u64,
        srid: u32,
        result: Arc<ImportResult>,
    },
    Another,
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::GisImport(e))
}

impl State {
    fn geojson(&self) -> bool {
        matches!(self.source, Source::GeoJson(_))
    }

    fn kml(&self) -> bool {
        matches!(self.source, Source::Kml(_))
    }

    /// What the window imports: its title and the undo step's kind.
    fn kind(&self) -> (&'static str, &'static str) {
        match &self.source {
            Source::GeoJson(_) => ("GeoJSON içe aktar", "GeoJSON"),
            Source::Kml(_) => ("KML/KMZ içe aktar", "KML"),
            Source::Shapefile(_) | Source::Zip { .. } => ("Shapefile içe aktar", "Shapefile"),
        }
    }

    /// The objects that will come in: moved when the move is asked and done
    /// for the system chosen now, else as read.
    /// Whether the objects are being moved from the system chosen now.
    fn moving(&self) -> bool {
        self.crs
            .srid
            .is_some_and(|at| self.in_flight == Some((self.read, at)))
    }

    fn effective(&self) -> Option<&Arc<ImportResult>> {
        match (&self.moved, self.crs.convert) {
            (Some((srid, moved)), true) if Some(*srid) == self.crs.srid => Some(moved),
            _ => self.result.as_ref(),
        }
    }

    /// The name the import goes by (the undo step, the new layers' group).
    fn name(&self) -> String {
        match &self.source {
            Source::GeoJson(f) | Source::Kml(f) | Source::Zip { file: f, .. } => f.name.clone(),
            Source::Shapefile(files) => match shapefile_set(files) {
                Ok(set) => format!("{}.shp", set.name),
                Err(_) => files
                    .first()
                    .map_or_else(|| "Shapefile".to_owned(), |f| f.name.clone()),
            },
        }
    }
}

/// Reads the source (off the UI thread): the result, and a zip's layers.
fn read_source(source: &Source) -> (Result<ImportResult, String>, Option<Vec<String>>) {
    match source {
        Source::GeoJson(f) => {
            let opts = GeoJsonReadOptions {
                layer: words::base_name(&f.name),
                max_entities: 0,
            };
            (kentos_formats::geojson::read(&f.bytes, &opts), None)
        }
        Source::Kml(f) => (kentos_formats::kml::read(&f.bytes, 0), None),
        Source::Shapefile(files) => {
            let result = shapefile_set(files).and_then(|set| {
                let part = |p: usize| set.parts_at[p].map(|i| &*files[i].bytes);
                let parts = shp::Files {
                    shp: &files[set.shp].bytes,
                    shx: part(0),
                    dbf: part(1),
                    prj: part(2),
                    cpg: part(3),
                };
                let opts = ShapefileReadOptions {
                    layer: set.name.clone(),
                    max_entities: 0,
                };
                shp::read(&parts, &opts)
            });
            (result, None)
        }
        Source::Zip { file, layer, .. } => match shp::zip_layers(&file.bytes) {
            Err(e) => (Err(e), None),
            Ok(layers) => {
                let path = layers
                    .get(*layer)
                    .or(layers.first())
                    .cloned()
                    .unwrap_or_default();
                let opts = ShapefileReadOptions {
                    layer: shp::zip_layer_name(&path).to_owned(),
                    max_entities: 0,
                };
                (shp::read_zip(&file.bytes, &path, &opts), Some(layers))
            }
        },
    }
}

impl App {
    /// Where a source layer's objects go: the project layer of the same
    /// name (and whether it is locked), or a new one.
    fn gis_target(&self, layer: &ImportLayer) -> (Option<String>, bool) {
        let Some(doc) = &self.document else {
            return (None, false);
        };
        match apply::layer_named(&doc.model, &layer.name) {
            Some(node) => (
                Some(node.id.clone()),
                doc.model.layers().is_locked(&node.id),
            ),
            None => (None, false),
        }
    }

    /// The layers that will be imported: chosen, and not onto a locked layer.
    fn gis_included<'a>(&self, state: &'a State) -> Vec<&'a ImportLayer> {
        state
            .effective()
            .map(|r| {
                r.layers
                    .iter()
                    .filter(|l| !state.excluded.contains(&l.name) && !self.gis_target(l).1)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn gis_import_picked(&mut self, source: Result<Source, String>) -> Task<Message> {
        let project = self.project_srid();
        let (source, failed) = match source {
            Ok(source) => (source, None),
            // Files without a layer: the window says why, “Başka dosya…” picks again.
            Err(e) => (Source::Shapefile(Vec::new()), Some(e)),
        };
        let mut state = State {
            source,
            read: READS.fetch_add(1, Ordering::Relaxed) + 1,
            reading: failed.is_none(),
            result: None,
            failed,
            unusable: None,
            excluded: BTreeSet::new(),
            crs: CrsQuestion::convertible(project),
            moved: None,
            in_flight: None,
            status: None,
        };
        if state.failed.is_some() {
            state.crs.declare(project, None, None);
            self.open_window(Window::GisImport(Box::new(state)));
            return Task::none();
        }
        let task = self.gis_read(&state);
        self.open_window(Window::GisImport(Box::new(state)));
        task
    }

    /// Reads the window's source off the UI thread.
    fn gis_read(&self, state: &State) -> Task<Message> {
        let source = state.source.clone();
        let read = state.read;
        off_thread(
            move || {
                let (result, layers) = read_source(&source);
                let unusable = result
                    .as_ref()
                    .ok()
                    .and_then(|r| apply::unusable(&r.entities))
                    .map(|(place, kind)| apply::unusable_text(place, kind));
                (result, unusable, layers)
            },
            move |(result, unusable, layers)| {
                Exchange::GisImport(Event::Read {
                    read,
                    result: result.map(Arc::new),
                    unusable,
                    layers,
                })
            },
        )
    }

    pub(super) fn gis_import_event(&mut self, e: Event) -> Task<Message> {
        let project = self.project_srid();
        let Some(Window::GisImport(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Another => {
                let kind = if s.geojson() {
                    Kind::GeoJson
                } else if s.kml() {
                    Kind::Kml
                } else {
                    Kind::Shapefile
                };
                return self.pick(kind);
            }
            Event::Run => {
                self.run_gis_import();
                return Task::none();
            }
            Event::Read {
                read,
                result,
                unusable,
                layers,
            } => {
                if read != s.read {
                    return Task::none();
                }
                s.reading = false;
                if let (Source::Zip { layers: known, .. }, Some(layers)) = (&mut s.source, layers) {
                    *known = layers;
                }
                let shapefile = !s.geojson();
                match result {
                    Ok(r) => {
                        let statement = Statement::of(r.declared_crs.as_ref(), shapefile);
                        s.crs.declare(project, Some(statement), r.bounds);
                        s.result = Some(r);
                    }
                    Err(e) => {
                        s.crs.declare(project, None, None);
                        s.failed = Some(e);
                    }
                }
                s.unusable = unusable;
                // A layer read again while the move is asked moves too.
                if s.crs.convert && s.result.is_some() {
                    return self.gis_move();
                }
            }
            Event::Layer(i) => {
                let Source::Zip { layer, layers, .. } = &mut s.source else {
                    return Task::none();
                };
                if *layer == i || i >= layers.len() {
                    return Task::none();
                }
                *layer = i;
                s.read = READS.fetch_add(1, Ordering::Relaxed) + 1;
                s.reading = true;
                s.result = None;
                s.failed = None;
                s.unusable = None;
                s.excluded.clear();
                s.status = None;
                s.moved = None;
                let state = (**s).clone();
                return self.gis_read(&state);
            }
            Event::Toggle(name) => {
                if !s.excluded.remove(&name) {
                    s.excluded.insert(name);
                }
            }
            Event::ToggleAll => {
                if s.excluded.is_empty() {
                    if let Some(r) = &s.result {
                        s.excluded = r.layers.iter().map(|l| l.name.clone()).collect();
                    }
                } else {
                    s.excluded.clear();
                }
            }
            Event::Crs(srid) => {
                s.crs.pick(srid);
                if s.crs.convert {
                    return self.gis_move();
                }
            }
            Event::Convert(on) => {
                s.crs.convert = on;
                if on {
                    return self.gis_move();
                }
            }
            Event::Moved { read, srid, result } => {
                if s.in_flight == Some((read, srid)) {
                    s.in_flight = None;
                }
                if read == s.read && Some(srid) == s.crs.srid {
                    s.moved = Some((srid, result));
                }
            }
        }
        Task::none()
    }

    /// Moves the objects read into the project's system off the UI thread
    /// (docs/adr/0222 §2), once for each source system chosen.
    fn gis_move(&mut self) -> Task<Message> {
        let own = self.project_system();
        let Some(Window::GisImport(s)) = &mut self.exchange else {
            return Task::none();
        };
        let (Some(result), Some(srid)) = (s.result.clone(), s.crs.srid) else {
            return Task::none();
        };
        if s.moved.as_ref().is_some_and(|(at, _)| *at == srid)
            || s.in_flight == Some((s.read, srid))
        {
            return Task::none();
        }
        let Some((from, to, choices)) = s.crs.move_between(&own) else {
            return Task::none();
        };
        let read = s.read;
        s.in_flight = Some((read, srid));
        off_thread(
            move || {
                let mut moved = (*result).clone();
                kentos_formats::reproject::reproject(&mut moved, &from, &to, &choices);
                moved
            },
            move |moved| {
                Exchange::GisImport(Event::Moved {
                    read,
                    srid,
                    result: Arc::new(moved),
                })
            },
        )
    }

    fn gis_can_import(&self, s: &State) -> bool {
        let own = self.project_system();
        // Moved: the objects moved for the system chosen now.
        let placed = s.crs.matches(&own)
            || (s.crs.ready(&own)
                && !s.moving()
                && s.moved
                    .as_ref()
                    .is_some_and(|(at, _)| Some(*at) == s.crs.srid));
        !s.reading
            && s.result.is_some()
            && s.unusable.is_none()
            && !self.gis_included(s).is_empty()
            && placed
    }

    fn run_gis_import(&mut self) {
        let Some(Window::GisImport(s)) = &self.exchange else {
            return;
        };
        if !self.gis_can_import(s) {
            return;
        }
        let Some(result) = s.effective().cloned() else {
            return;
        };
        let name = s.name();
        let kind = s.kind().1;
        let layers: Vec<(String, LayerTarget)> = self
            .gis_included(s)
            .into_iter()
            .map(|l| {
                let target = match self.gis_target(l).0 {
                    Some(id) => LayerTarget::Existing(id),
                    None => LayerTarget::New {
                        name: l.name.clone(),
                        style: Box::new(LayerStyle {
                            color: l.color.clone(),
                            line_type: l.line_type,
                            ..kentos_domain::default_style()
                        }),
                        visible: l.visible,
                        locked: l.locked,
                        // The file's fields (docs/adr/0199 §6): a DBF's, a GeoJSON's properties' kinds.
                        fields: l.fields.clone(),
                    },
                };
                (l.name.clone(), target)
            })
            .collect();
        let chosen = layers.len();
        let plan = ImportPlan {
            label: format!("{kind}: {name}"),
            layers,
            group: Some(name.clone()),
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        match apply::apply_import(&mut doc.model, result.entities.clone(), Vec::new(), &plan) {
            Err(error) => {
                if let Some(Window::GisImport(s)) = &mut self.exchange {
                    s.status = Some((true, error));
                }
            }
            Ok(applied) => {
                self.zoom_to(&applied.slots);
                let into = if applied.created.is_empty() {
                    String::new()
                } else {
                    format!("; {} yeni katman “{name}” grubunda", applied.created.len())
                };
                self.say(
                    Level::Success,
                    format!(
                        "“{name}”: {} nesne {chosen} katmana alındı{into}. Tek adımda geri alınabilir.",
                        applied.slots.len()
                    ),
                );
                if !result.report.skipped.is_empty() {
                    let skipped: Vec<String> = result
                        .report
                        .skipped
                        .iter()
                        .map(words::report_text)
                        .collect();
                    self.warn(format!(
                        "“{name}” içinde alınmayanlar: {}",
                        skipped.join(" ")
                    ));
                }
                self.close_exchange();
            }
        }
    }

    pub(super) fn gis_import_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let project = self.project_system();
        let facts = match (&s.result, &s.failed) {
            (Some(r), _) => words::facts(&r.report.source),
            (None, Some(_)) => "okunamadı".to_owned(),
            (None, None) => "okunuyor…".to_owned(),
        };
        let meta = match &s.source {
            Source::Shapefile(files) => match shapefile_set(files) {
                Ok(set) if facts.is_empty() => set.parts.join(", "),
                Ok(set) => format!("{} · {facts}", set.parts.join(", ")),
                Err(_) => facts,
            },
            _ => facts,
        };
        let mut body = Column::new()
            .spacing(12)
            .push(words::file_line(&s.name(), meta));
        if let Source::Zip { layers, layer, .. } = &s.source
            && layers.len() > 1
        {
            let pick = Select::new(
                layers.iter().map(|l| Choice::new(l.clone())),
                Some(*layer),
                |i| event(Event::Layer(i)),
            );
            body = body.push(words::field(
                "Arşivdeki katman",
                pick,
                Some(format!(
                    "Arşivde {} Shapefile katmanı var; bir seferde biri alınır. Öbürlerini sonra aynı arşivden alın: yeni katmanlar aynı gruba girer.",
                    layers.len()
                )),
            ));
        }
        body = body
            .push(self.gis_layers(s))
            // The system before the report: whether the file can go in at all is decided there.
            .push(s.crs.view(&project, &self.number_format(), |srid| {
                event(Event::Crs(srid))
            }));
        if let Some(convert) = s.crs.convert_view(&project, |on| event(Event::Convert(on))) {
            body = body.push(convert);
        }
        body = body.push(self.gis_summary(s));
        if let Some((error, words)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                words.clone(),
            ));
        }
        let can = self.gis_can_import(s);
        overlay::blocking(
            Dialog::new(s.kind().0)
                // As tall as its content; a long body (a file with many layers) scrolls, the buttons stay in view.
                .scroll(body)
                .action(words::ghost("Başka dosya…", Some(event(Event::Another))))
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("İçe aktar", can.then(|| event(Event::Run))))
                .width(900.0)
                .max_height(820.0),
        )
    }

    fn gis_layers<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = s.effective() else {
            return words::empty(if s.failed.is_some() {
                "Katman yok."
            } else {
                "Dosya okunuyor…"
            });
        };
        if r.layers.is_empty() {
            return words::empty("Dosyada alınacak nesne yok.");
        }
        let all = match s.excluded.len() {
            0 => Check::Checked,
            n if n == r.layers.len() => Check::Unchecked,
            _ => Check::Mixed,
        };
        let head = row![
            check_box(all, Some(event(Event::ToggleAll))),
            label::caption(format!("Bütün katmanlar ({})", r.layers.len())),
        ]
        .spacing(8)
        .align_y(Center);
        let rows = r.layers.iter().map(|l| {
            let (existing, locked) = self.gis_target(l);
            let checked = !s.excluded.contains(&l.name) && !locked;
            let toggle = (!locked).then(|| event(Event::Toggle(l.name.clone())));
            let where_to = match (&existing, locked) {
                (Some(id), true) => format!(
                    "“{}” katmanı kilitli; alınmaz. Kilidini Katmanlar panelinden açın.",
                    self.layer_name(id)
                ),
                (Some(id), false) => format!("“{}” katmanına eklenir", self.layer_path(id)),
                (None, _) => "yeni katman".to_owned(),
            };
            TableRow::new([
                check_box(
                    if checked {
                        Check::Checked
                    } else {
                        Check::Unchecked
                    },
                    toggle,
                ),
                row![swatch(hex_color(&l.color)), label::body(l.name.clone())]
                    .spacing(6)
                    .align_y(Center)
                    .into(),
                label::body(l.count.to_string()).into(),
                if locked {
                    label::caption(where_to)
                        .style(kentos_ui::style::text::danger)
                        .into()
                } else {
                    label::caption(where_to).into()
                },
            ])
        });
        let table = words::fitted(
            Table::new([
                TableColumn::new("").width(22),
                TableColumn::new("Katman").width(Length::FillPortion(2)),
                TableColumn::new("Nesne").width(60).align_right(),
                TableColumn::new("Nereye").width(Length::FillPortion(3)),
            ])
            .extend(rows),
            r.layers.len(),
        );
        column![head, table].spacing(6).width(Fill).into()
    }

    fn gis_summary<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = s.effective() else {
            return words::summary(vec![match &s.failed {
                Some(e) => words::text_line(Line::Error, e.clone()),
                None => words::text_line(
                    Line::Info,
                    "Dosya okunuyor; büyük dosyalar biraz sürebilir.",
                ),
            }]);
        };
        let included = self.gis_included(s);
        let chosen: BTreeSet<&str> = included.iter().map(|l| l.name.as_str()).collect();
        let mut counts = Vec::new();
        for e in &r.entities {
            if chosen.contains(e.base().layer_id.as_str()) {
                words::count(&mut counts, e.kind());
            }
        }
        let total: u32 = counts.iter().map(|(_, n)| n).sum();
        let created = included
            .iter()
            .filter(|l| self.gis_target(l).0.is_none())
            .count();
        let mut lines = vec![if total > 0 {
            let group = if created > 0 {
                format!(" {created} yeni katman “{}” grubunda kurulacak.", s.name())
            } else {
                String::new()
            };
            words::text_line(
                Line::Ok,
                format!(
                    "{total} nesne alınacak: {}.{group}",
                    words::kind_counts(&counts)
                ),
            )
        } else {
            words::text_line(Line::Warn, "Alınacak nesne yok; en az bir katman seçin.")
        }];
        let own = self.project_system();
        if s.crs.convert && s.crs.ready(&own) && !s.crs.matches(&own) {
            lines.push(if s.moving() {
                words::text_line(Line::Info, "Koordinatlar projenin sistemine çevriliyor…")
            } else {
                words::text_line(Line::Ok, "Koordinatlar projenin sistemine çevrildi; aşağıdaki sayılar çevrilmiş nesnelerindir.")
            });
        } else if !s.crs.matches(&own) {
            lines.push(words::text_line(
                Line::Warn,
                "Koordinatların sistemi projeninki değil: içe aktarma kapalı (yukarıdaki koordinat sistemi notuna bakın).",
            ));
        }
        if let Some(unusable) = &s.unusable {
            lines.push(words::text_line(Line::Error, unusable.clone()));
        }
        lines.extend(words::report_lines(&r.report.notes, Line::Info, 12));
        lines.extend(words::report_lines(&r.report.skipped, Line::Warn, 12));
        if let Source::Shapefile(files) = &s.source
            && let Ok(set) = shapefile_set(files)
            && !set.unused.is_empty()
        {
            lines.push(words::text_line(
                Line::Info,
                format!(
                    "Kullanılmayan dosyalar (başka katmanın ya da tanınmayan): {}.",
                    set.unused.join(", ")
                ),
            ));
        }
        if let Some(b) = &r.bounds {
            lines.push(words::text_line(
                Line::Info,
                words::extent_text(&self.number_format(), b),
            ));
        }
        words::summary(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, byte: u8) -> Picked {
        Picked {
            name: name.to_owned(),
            bytes: vec![byte].into(),
        }
    }

    #[test]
    fn a_layer_is_the_shp_and_the_files_of_its_name_whatever_their_case() {
        let files = [
            file("Parsel.dbf", 2),
            file("PARSEL.SHP", 1),
            file("parsel.shx", 3),
            file("parsel.prj", 4),
            file("yol.dbf", 5),
            file("parsel.qmd", 6),
        ];
        let set = shapefile_set(&files).expect("a layer");
        assert_eq!(set.name, "PARSEL");
        assert_eq!(set.parts, [".shp", ".dbf", ".shx", ".prj"]);
        let byte = |i: Option<usize>| i.map(|i| files[i].bytes[0]);
        assert_eq!(files[set.shp].bytes[0], 1);
        assert_eq!(set.parts_at.map(byte), [Some(3), Some(2), Some(4), None]);
        assert_eq!(set.unused, ["yol.dbf", "parsel.qmd"]);
    }

    #[test]
    fn no_shp_asks_for_one_and_several_ask_for_one_layer_at_a_time() {
        let none = shapefile_set(&[file("a.dbf", 1), file("a.prj", 1)]).expect_err("no .shp");
        assert!(none.contains(".shp yok"), "{none}");
        let two = shapefile_set(&[file("a.shp", 1), file("b.shp", 1), file("a.dbf", 1)])
            .expect_err("two layers");
        assert!(
            two.contains("2 .shp var (a.shp, b.shp)") && two.contains("bir katman"),
            "{two}"
        );
        // The first of a repeated part is used, the second is not.
        let set = shapefile_set(&[file("a.shp", 1), file("a.dbf", 7), file("A.DBF", 8)])
            .expect("a layer");
        assert_eq!(set.parts_at[1], Some(1));
        assert_eq!(set.unused, ["A.DBF"]);
    }

    #[test]
    fn one_zip_without_a_shp_is_an_archive_and_two_are_refused() {
        assert!(matches!(
            shapefile_source(vec![file("katmanlar.ZIP", 1)]),
            Ok(Source::Zip { .. })
        ));
        // With a .shp the files are the layer; the archive is one of the files not used.
        assert!(matches!(
            shapefile_source(vec![file("a.shp", 1), file("a.zip", 1)]),
            Ok(Source::Shapefile(_))
        ));
        let two = shapefile_source(vec![file("a.zip", 1), file("b.zip", 1)]).expect_err("two");
        assert!(two.contains("2 .zip var"), "{two}");
    }
}
