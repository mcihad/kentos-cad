//! Kaynaklar (docs/adr/0199 §7; the web's `ui/sources/SourcesPanel.ts`): the
//! dock's sources, a tab beside Katmanlar, İşlemler, Bloklar and Şablonlar.
//! Klasörler lists the folders the user added (Klasör ekle; kept in
//! `kaynak-klasorleri.json` beside the recent files) with their folders and
//! the files the panel adds (`kentos_interaction::sources`); a file's Katman
//! olarak ekle (its button, a double click, its menu) opens its format's
//! import window with it (exchange/). KentOS lists the projects the
//! signed-in account reaches; opening one downloads its drawing (a database
//! project's snapshot, a file project's newest revision) and lists its
//! layers, and a layer's Katman olarak ekle takes it with its objects into
//! the open drawing as one undo step (drawing_exchange.rs
//! `take_layer_into`). Veritabanları lists the device's PostgreSQL
//! connections (docs/adr/0623, as QGIS's Browser lists its own): a
//! connection opens into its schemas and their tables, views and
//! materialized views, read from the database's catalogue off the UI thread
//! (`kentos_postgis::probe::browse`); a table's Katman olarak ekle binds it
//! as a bound layer through `cad.sources.edit`, after asking to carry it into
//! the project's system when it is in another. A server asking for a password
//! KentOS does not know opens the password question (db_connections.rs).
//! What is opened is read again when it is opened again.
//! Folders are read and drawings decoded off the UI thread; an answer for a
//! row closed since is dropped.
//!
//! The source browser (docs/adr/0599): a GeoPackage opens into its tables (`cad.sources.describe`), a table into its
//! declared box and five sample rows (`cad.sources.query`), with Bağlı katman olarak aç, Kopya olarak al, Katmana git
//! for a bound one and Kapsamı göster; Veritabanları lists PostGIS addresses and their tables by schema; Servisler the
//! ready basemaps and the services added before, opened into their layers; the search box keeps the rows that match
//! and their ancestors (`kentos_interaction::sources::search_*`); Son kullanılanlar and Sık kullanılanlar are kept
//! with the folders (`kaynak-klasorleri.json` version 2); a row dragged onto the drawing, or a file dropped on the
//! window, is added as its Katman olarak ekle adds it.

mod view;

#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::Task;
use iced::futures::channel::mpsc;
use kentos_cloud::CatalogQuery;
use kentos_commands::{
    CatalogSort, CatalogView, CommandResult, DocumentSnapshotV2, ProjectStorage, ProjectSummary,
    ServiceKind, SourceTable, SourcesQueried, SourcesQuery,
};
use kentos_interaction::Level;
use kentos_interaction::sources::{
    Listing, SourceEntry, SourceFile, SourceKind, listing, recent_with, source_kind, starred,
};
use kentos_native_application::ExecutionContext;
use kentos_postgis::probe::{self, Catalog, Diagnosis, Problem};
use kentos_services::connect::Connecting;
use serde::{Deserialize, Serialize};

use crate::app::{App, Message, Panel};
use crate::exchange::{Event as ExchangeEvent, Kind, Picked};

/// The lists of projects the panel shows: their views and names.
pub(crate) const LISTS: [(CatalogView, &str, &str); 2] = [
    (CatalogView::Mine, "mine", "Projelerim"),
    (CatalogView::Shared, "shared", "Benimle paylaşılanlar"),
];
/// Projects a list shows at most.
pub(crate) const LIMIT: u32 = 200;
/// The sections' keys; they start open.
pub(crate) const FOLDERS: &str = "klasorler";
pub(crate) const KENTOS: &str = "kentos";
/// The browser's sections and groups (docs/adr/0599); they start open but Hazır altlıklar.
pub(crate) const RECENT: &str = "son";
pub(crate) const FAVOURITES: &str = "sik";
pub(crate) const DATABASES: &str = "veritabanlari";
pub(crate) const SERVICES: &str = "servisler";
pub(crate) const PRESETS: &str = "hazir";
pub(crate) const SAVED: &str = "kayitli";
/// The sample rows a table's preview reads, and the columns each shows.
pub(crate) const SAMPLE_ROWS: u32 = 5;
pub(crate) const SAMPLE_COLUMNS: usize = 4;

/// Where a previewed table is (docs/adr/0599 §2): a GeoPackage of a folder. A database's tables are Veritabanları's,
/// by their connection (docs/adr/0623).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Origin {
    File(PathBuf),
}

impl Origin {
    /// Its path as the commands take it.
    pub(crate) fn path(&self) -> String {
        match self {
            Origin::File(p) => p.display().to_string(),
        }
    }

    /// A kept source for one of its tables.
    pub(crate) fn kept(&self, table: &str) -> Kept {
        match self {
            Origin::File(path) => Kept::Table {
                path: path.clone(),
                table: table.to_owned(),
            },
        }
    }
}

/// A service the device keeps for Servisler (docs/adr/0599 §4): its kind, address, name and the project's
/// connection it was added with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedService {
    pub kind: ServiceKind,
    pub url: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection: Option<String>,
}

/// A recent or favourite source (docs/adr/0599 §6), by what it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Kept {
    File {
        path: PathBuf,
    },
    Table {
        path: PathBuf,
        table: String,
    },
    /// A table of a saved database connection (docs/adr/0623), by the connection's name.
    Database {
        connection: String,
        table: String,
    },
    Service {
        service: SavedService,
    },
    ServiceLayer {
        service: SavedService,
        item: String,
        title: String,
    },
    Basemap {
        command: String,
        name: String,
    },
}

impl Kept {
    /// Its name as the row says it.
    pub(crate) fn label(&self) -> String {
        match self {
            Kept::File { path } => folder_name(path),
            Kept::Table { table, .. } | Kept::Database { table, .. } => table.clone(),
            Kept::Service { service } => service.name.clone(),
            Kept::ServiceLayer { title, .. } => title.clone(),
            Kept::Basemap { name, .. } => name.clone(),
        }
    }

    /// What it is and where, after the name.
    pub(crate) fn meta(&self) -> String {
        match self {
            Kept::File { path } => source_kind(&folder_name(path))
                .map_or_else(|| "Dosya".to_owned(), |k| k.label().to_owned()),
            Kept::Table { path, .. } => folder_name(path),
            Kept::Database { connection, .. } => connection.clone(),
            Kept::Service { service } => service.kind.label().to_owned(),
            Kept::ServiceLayer { service, .. } => service.name.clone(),
            Kept::Basemap { .. } => "Hazır altlık".to_owned(),
        }
    }

    /// Its icon.
    pub(crate) fn glyph(&self) -> &'static str {
        match self {
            Kept::File { path } => source_kind(&folder_name(path)).map_or("fileOpen", |k| k.icon()),
            Kept::Table { .. } => "table",
            Kept::Database { .. } => "sourceDatabase",
            Kept::Service { .. } | Kept::ServiceLayer { .. } => "serviceAdd",
            Kept::Basemap { .. } => "basemap",
        }
    }

    /// Whether its file is not where it was (docs/adr/0599 §6): a database and a service say so when opened.
    pub(crate) fn missing(&self) -> bool {
        match self {
            Kept::File { path } | Kept::Table { path, .. } => !path.exists(),
            _ => false,
        }
    }

    /// The words the search looks in.
    pub(crate) fn fields(&self) -> Vec<String> {
        let mut out = vec![self.label(), self.meta()];
        match self {
            Kept::File { path } | Kept::Table { path, .. } => out.push(path.display().to_string()),
            Kept::Service { service } | Kept::ServiceLayer { service, .. } => {
                out.push(service.url.clone())
            }
            _ => {}
        }
        out
    }
}

/// A row being dragged (docs/adr/0599 §8): its key, what a press without a drop does, what a drop on the drawing adds,
/// and where the pointer is.
#[derive(Debug, Clone)]
pub(crate) struct Drag {
    pub(crate) key: String,
    click: Option<Message>,
    add: Message,
    at: Option<iced::Point>,
}

/// A connection's catalogue as the panel knows it.
#[derive(Debug, Clone)]
pub(crate) enum Browse {
    Reading,
    Ready(Arc<Catalog>),
    Failed(Diagnosis),
}

/// Katman olarak ekle on a table in another system, waiting for its answer.
#[derive(Debug, Clone)]
pub(crate) struct ReprojectAsk {
    pub(crate) connection: String,
    pub(crate) table: String,
    pub(crate) area: bool,
    pub(crate) srid: u32,
}

/// What is known of something read when it is opened.
#[derive(Debug, Clone)]
pub(crate) enum Read<T> {
    Reading,
    Ready(T),
    Failed(String),
}

/// A list's projects and how many the list holds.
pub(crate) type Projects = (Vec<ProjectSummary>, u64);

/// A project's drawing as downloaded, and how many objects each layer has.
#[derive(Debug)]
pub(crate) struct ProjectDrawing {
    pub(crate) drawing: DocumentSnapshotV2,
    pub(crate) counts: HashMap<String, usize>,
}

/// What the panel keeps while the app runs.
#[derive(Debug)]
pub struct SourcesPanel {
    pub(crate) folders: SourceFolders,
    /// Rows opened by key (the sections start open).
    pub(crate) open: BTreeSet<String>,
    pub(crate) listings: HashMap<PathBuf, Read<Listing>>,
    pub(crate) lists: HashMap<&'static str, Read<Projects>>,
    /// Downloaded drawings by project id.
    pub(crate) drawings: HashMap<String, Read<Arc<ProjectDrawing>>>,
    /// The connections' catalogues by connection name.
    pub(crate) catalogs: HashMap<String, Browse>,
    /// The question of a table in another system.
    pub(crate) reproject: Option<ReprojectAsk>,
    /// Requests under way by row key, stopped when dropped (the row closed).
    requests: HashMap<String, iced::task::Handle>,
    /// The row chosen last (a click), by key.
    pub(crate) focused: Option<String>,
    /// A GeoPackage's tables by its path, a database's by its address (docs/adr/0599 §2, §3).
    pub(crate) tables: HashMap<Origin, Read<Vec<SourceTable>>>,
    /// A table's sample rows by its row's key.
    pub(crate) samples: HashMap<String, Read<Vec<String>>>,
    /// A saved service's offer by its row's key (docs/adr/0599 §4).
    pub(crate) services: HashMap<String, Read<Box<Connecting>>>,
    /// The search box's words (docs/adr/0599 §5).
    pub(crate) search: String,
    pub(crate) drag: Option<Drag>,
    /// Kapsamı göster's frame over the drawing (docs/adr/0599 §9): the drawing's session, the box in the project's
    /// system and the table's name.
    pub(crate) extent_mark: Option<(u64, [f64; 4], String)>,
}

impl Default for SourcesPanel {
    fn default() -> Self {
        Self {
            folders: SourceFolders::memory(),
            open: BTreeSet::from(
                [
                    FOLDERS, KENTOS, RECENT, FAVOURITES, DATABASES, SERVICES, SAVED,
                ]
                .map(str::to_owned),
            ),
            listings: HashMap::new(),
            lists: HashMap::new(),
            drawings: HashMap::new(),
            catalogs: HashMap::new(),
            reproject: None,
            requests: HashMap::new(),
            focused: None,
            tables: HashMap::new(),
            samples: HashMap::new(),
            services: HashMap::new(),
            search: String::new(),
            drag: None,
            extent_mark: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Klasör ekle: the system's folder window.
    AddFolder,
    FolderPicked(Option<PathBuf>),
    /// Listeden kaldır.
    Remove(PathBuf),
    /// A row opened or closed, by key.
    Toggle(String),
    /// A row chosen (a click).
    Focus(String),
    /// A folder read: what it shows, or why not.
    Listed(PathBuf, Result<Listing, String>),
    /// A list's projects, or why not.
    Projects(&'static str, Result<Projects, String>),
    /// A project's drawing downloaded and read, or why not.
    Downloaded(String, Result<Arc<ProjectDrawing>, String>),
    /// Katman olarak ekle on a file of `folder`.
    AddFile(PathBuf, SourceFile),
    /// A file's parts read, for its import window.
    FileRead(SourceKind, Result<Vec<Picked>, String>),
    /// Katman olarak ekle on a project's layer: its id and the layer's path.
    AddLayer(String, String),
    /// The search box (docs/adr/0599 §5).
    Search(String),
    /// A GeoPackage's tables, or why not.
    Tables(Origin, Result<Vec<SourceTable>, String>),
    /// A table's sample rows by its row's key, or why not.
    Sample(String, Result<Vec<String>, String>),
    /// A saved service read, by its row's key.
    ServiceRead(String, Result<Box<Connecting>, String>),
    /// Listeden kaldır on a saved service.
    ServiceRemove(usize),
    /// Add a source by what it is (a recent or favourite row, a drop); it becomes the newest recent one.
    Use(Kept),
    /// A table's Bağlı katman olarak aç (`true`) or Kopya olarak al.
    OpenTable(Origin, String, bool),
    /// Katmana git: the bound layer chosen in Katmanlar and shown.
    GoToLayer(String),
    /// Kapsamı göster: a table's system and declared box, and its name.
    ShowExtent(Option<u32>, [f64; 4], String),
    /// Sık kullanılanlara ekle or çıkar.
    Star(Kept),
    /// A draggable row pressed: its key, what a click does, what a drop on the drawing adds.
    Press(String, Option<Box<Message>>, Box<Message>),
    /// The pointer while a row is dragged.
    DragAt(iced::Point),
    /// The button let go while a row is dragged.
    DragEnd,
    /// A file dropped on the window from the system's file manager (docs/adr/0599 §8).
    Dropped(PathBuf),
    /// A connection's catalogue read, or why not; `ask`: a password the server wants is asked for.
    Browsed(String, Result<Arc<Catalog>, Diagnosis>, bool),
    /// Yenile on a connection: its catalogue read again.
    Refresh(String),
    /// The password question for a connection's row.
    AskPassword(String),
    /// Katman olarak ekle on a connection's table: the connection, `schema.name`, only the view's area.
    AddTable(String, String, bool),
    /// The question of a table in another system answered: carried into the project's system, or not bound.
    Reproject(bool),
}

pub(crate) fn msg(event: Event) -> Message {
    Message::Sources(event)
}

/// Runs `work` on a thread of its own and brings its answer back as a message.
fn off_thread<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(T) -> Event + Send + 'static,
) -> Task<Message> {
    iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
        let answer = msg(done(work()));
        let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(&mut out, answer));
    })
}

/// What a folder shows, read from the disk.
pub(crate) fn read_folder(path: &Path) -> Result<Listing, String> {
    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Klasör okunamadı: {e}. Klasör taşınmış ya da silinmiş olabilir."))?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        // A link is what it points at.
        let dir = std::fs::metadata(entry.path()).is_ok_and(|m| m.is_dir());
        out.push(SourceEntry { name, dir });
    }
    Ok(listing(&out))
}

/// A file's parts read from its folder.
fn read_parts(folder: &Path, file: &SourceFile) -> Result<Vec<Picked>, String> {
    file.parts
        .iter()
        .map(|name| {
            std::fs::read(folder.join(name))
                .map(|bytes| Picked {
                    name: name.clone(),
                    bytes: bytes.into(),
                    path: Some(folder.join(name)),
                })
                .map_err(|e| {
                    format!("“{name}” okunamadı: {e}. Dosya taşınmış ya da silinmiş olabilir; klasörü kapatıp açın.")
                })
        })
        .collect()
}

/// A downloaded `.kcad` as the contract's drawing, through the document's
/// rules (the web's `otherDrawing`).
fn drawing_of(bytes: &[u8]) -> Result<Arc<ProjectDrawing>, String> {
    let snapshot = kentos_kcad::decode(bytes).map_err(|e| e.message)?;
    let doc = crate::document::Document::from_v2(snapshot, None)?;
    let drawing = doc.model.to_snapshot_v2();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for e in &drawing.entities {
        *counts.entry(e.base().layer_id.clone()).or_default() += 1;
    }
    Ok(Arc::new(ProjectDrawing { drawing, counts }))
}

impl App {
    /// `data.sources`: the dock shows the Kaynaklar tab.
    pub(crate) fn show_sources(&mut self) {
        if !self.right_panel_shown() {
            self.toggle_right_panel();
        }
        self.docks
            .update(kentos_ui::widget::docking::Event::Selected(Panel::Sources));
    }

    pub(crate) fn sources_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::AddFolder => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Klasör ekle")
                            .pick_folder()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |path| msg(Event::FolderPicked(path)),
                );
            }
            Event::FolderPicked(None) => {}
            Event::FolderPicked(Some(path)) => {
                if self.sources.folders.add(&path) {
                    self.output(format!(
                        "Kaynaklar: “{}” klasörü eklendi.",
                        folder_name(&path)
                    ));
                }
                let key = folder_key(&path);
                self.sources.open.insert(key);
                return self.list_folder(path);
            }
            Event::Remove(path) => {
                self.sources.open.remove(&folder_key(&path));
                self.sources.listings.remove(&path);
                if self.sources.folders.remove(&path) {
                    self.output(format!(
                        "Kaynaklar: “{}” listeden kaldırıldı.",
                        folder_name(&path)
                    ));
                }
            }
            Event::Toggle(key) => return self.toggle_source(key),
            Event::Focus(key) => self.sources.focused = Some(key),
            Event::Listed(path, result) => {
                if !self.sources.open.contains(&folder_key(&path)) {
                    return Task::none();
                }
                self.sources.requests.remove(&folder_key(&path));
                self.sources.listings.insert(
                    path,
                    match result {
                        Ok(l) => Read::Ready(l),
                        Err(why) => Read::Failed(why),
                    },
                );
            }
            Event::Projects(view, result) => {
                let key = list_key(view);
                if !self.sources.open.contains(&key) {
                    return Task::none();
                }
                self.sources.requests.remove(&key);
                self.sources.lists.insert(
                    view,
                    match result {
                        Ok(p) => Read::Ready(p),
                        Err(why) => Read::Failed(format!("Projeler alınamadı: {why}")),
                    },
                );
            }
            Event::Downloaded(id, result) => {
                let key = project_key(&id);
                if !self.sources.open.contains(&key) {
                    return Task::none();
                }
                self.sources.requests.remove(&key);
                self.sources.drawings.insert(
                    id,
                    match result {
                        Ok(d) => Read::Ready(d),
                        Err(why) => Read::Failed(format!("Çizim indirilemedi: {why}")),
                    },
                );
            }
            Event::AddFile(folder, file) => {
                if self.document.is_none() {
                    self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
                    return Task::none();
                }
                let path = folder.join(&file.name);
                self.sources
                    .folders
                    .used(&Kept::File { path: path.clone() });
                if let Some(open) = opener(file.kind, &path) {
                    return self.update(open);
                }
                let kind = file.kind;
                return off_thread(
                    move || read_parts(&folder, &file),
                    move |parts| Event::FileRead(kind, parts),
                );
            }
            Event::FileRead(_, Err(why)) => self.error(why),
            Event::FileRead(kind, Ok(mut parts)) => {
                let kind = match kind {
                    SourceKind::GeoJson => Kind::GeoJson,
                    SourceKind::Shapefile => {
                        return self.exchange_event(ExchangeEvent::PickedMany(
                            Kind::Shapefile,
                            Some(Ok(parts)),
                        ));
                    }
                    SourceKind::Dxf => Kind::Dxf,
                    SourceKind::Ncz => Kind::Ncz,
                    SourceKind::Gnss => Kind::Gnss,
                    SourceKind::Coords => Kind::Coords,
                    SourceKind::Kml => Kind::Kml,
                    // Opened with their path (`opener`), never read here.
                    SourceKind::GeoPackage
                    | SourceKind::FlatGeobuf
                    | SourceKind::Gml
                    | SourceKind::Raster
                    | SourceKind::PointCloud => return Task::none(),
                };
                let first = parts.drain(..).next();
                return self.exchange_event(ExchangeEvent::Picked(kind, first.map(Ok)));
            }
            Event::Browsed(name, result, ask) => {
                if !self.sources.open.contains(&connection_key(&name)) {
                    return Task::none();
                }
                let browse = match result {
                    Ok(catalog) => Browse::Ready(catalog),
                    Err(d) => {
                        if ask && matches!(d.problem, Problem::PasswordNeeded | Problem::Password) {
                            self.ask_db_password(
                                &name,
                                d.message.clone(),
                                crate::db_connections::Retry::Browse(name.clone()),
                            );
                        }
                        Browse::Failed(d)
                    }
                };
                self.sources.catalogs.insert(name, browse);
            }
            Event::Refresh(name) => {
                self.sources.open.insert(connection_key(&name));
                return self.browse_connection(&name);
            }
            Event::AskPassword(name) => {
                let reason = match self.sources.catalogs.get(&name) {
                    Some(Browse::Failed(d)) => d.message.clone(),
                    _ => "Sunucu parola istiyor.".to_owned(),
                };
                self.ask_db_password(&name, reason, crate::db_connections::Retry::Browse(name.clone()));
            }
            Event::AddTable(name, table, area) => return self.add_database_table(name, table, area),
            Event::Reproject(yes) => {
                if let Some(ask) = self.sources.reproject.take()
                    && yes
                {
                    return self.bind_from_sources(&ask.connection, &ask.table, true, ask.area);
                }
            }
            Event::AddLayer(id, path) => {
                let Some(Read::Ready(drawing)) = self.sources.drawings.get(&id).cloned() else {
                    return Task::none();
                };
                let from = self
                    .sources
                    .project(&id)
                    .map_or_else(String::new, |p| p.name.clone());
                self.take_layer_into(&drawing.drawing, &path, &from);
            }
            Event::Search(words) => self.sources.search = words,
            Event::Tables(origin, result) => {
                let key = tables_key(&origin);
                if !self.sources.open.contains(&key) {
                    return Task::none();
                }
                self.sources.requests.remove(&key);
                self.sources.tables.insert(
                    origin,
                    match result {
                        Ok(t) => Read::Ready(t),
                        Err(why) => Read::Failed(format!("Tablolar okunamadı: {why}")),
                    },
                );
            }
            Event::Sample(key, result) => {
                if !self.sources.open.contains(&key) {
                    return Task::none();
                }
                self.sources.requests.remove(&key);
                self.sources.samples.insert(
                    key,
                    match result {
                        Ok(rows) => Read::Ready(rows),
                        Err(why) => Read::Failed(format!("Örnek satırlar okunamadı: {why}")),
                    },
                );
            }
            Event::ServiceRead(key, result) => {
                if !self.sources.open.contains(&key) {
                    return Task::none();
                }
                self.sources.requests.remove(&key);
                self.sources.services.insert(
                    key,
                    match result {
                        Ok(c) => Read::Ready(c),
                        Err(why) => Read::Failed(format!("Servis okunamadı: {why}")),
                    },
                );
            }
            Event::ServiceRemove(at) => {
                if let Some(gone) = self.sources.folders.remove_service(at) {
                    let key = service_key(&gone);
                    self.sources.open.remove(&key);
                    self.sources.services.remove(&key);
                    self.output(format!("Kaynaklar: “{}” listeden kaldırıldı.", gone.name));
                }
            }
            Event::Use(kept) => return self.use_source(kept),
            Event::OpenTable(origin, table, bind) => {
                if self.document.is_none() {
                    self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
                    return Task::none();
                }
                self.sources.folders.used(&origin.kept(&table));
                return self.open_table(origin, table, bind);
            }
            Event::GoToLayer(id) => {
                if !self.right_panel_shown() {
                    self.toggle_right_panel();
                }
                self.docks
                    .update(kentos_ui::widget::docking::Event::Selected(Panel::Layers));
                self.selected_layer = Some(id.clone());
                self.zoom_to_layer(&id);
            }
            Event::ShowExtent(srid, extent, name) => self.show_extent(srid, extent, name),
            Event::Star(kept) => {
                let name = kept.label();
                if self.sources.folders.is_favourite(&kept) {
                    self.sources.folders.star(&kept);
                    self.output(format!(
                        "Kaynaklar: “{name}” sık kullanılanlardan çıkarıldı."
                    ));
                } else if self.sources.folders.star(&kept) {
                    self.output(format!("Kaynaklar: “{name}” sık kullanılanlara eklendi."));
                } else {
                    self.warn(format!(
                        "Sık kullanılanlar dolu (en çok {}); “{name}” eklenmedi. Önce birini çıkarın.",
                        kentos_interaction::sources::FAVOURITES_MOST
                    ));
                }
            }
            Event::Press(key, click, add) => {
                self.sources.focused = Some(key.clone());
                self.sources.drag = Some(Drag {
                    key,
                    click: click.map(|c| *c),
                    add: *add,
                    at: None,
                });
            }
            Event::DragAt(at) => {
                if let Some(d) = &mut self.sources.drag {
                    d.at = Some(at);
                }
            }
            Event::DragEnd => {
                let Some(d) = self.sources.drag.take() else {
                    return Task::none();
                };
                // Let go over the drawing: added there; else a click on the row.
                if d.at.is_some_and(|at| self.viewport.bounds.contains(at)) {
                    return self.update(d.add);
                }
                if let Some(click) = d.click {
                    return self.update(click);
                }
            }
            Event::Dropped(path) => {
                if self.document.is_none() {
                    self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
                    return Task::none();
                }
                return self.use_source(Kept::File { path });
            }
        }
        Task::none()
    }

    /// Adds a recent or favourite source, a dropped file, by what it is (docs/adr/0599 §6, §8); it becomes the newest
    /// recent source. A file no longer where it was is said, nothing made.
    fn use_source(&mut self, kept: Kept) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        if kept.missing() {
            self.warn(format!(
                "“{}” bulunamadı ({}). Dosya taşınmış ya da silinmiş olabilir.",
                kept.label(),
                kept.meta()
            ));
            return Task::none();
        }
        match &kept {
            Kept::File { path } => {
                let name = folder_name(path);
                let Some(kind) = source_kind(&name) else {
                    self.warn(format!(
                        "Kaynaklar “{name}” dosyasını tanımıyor. Eklenebilen dosyalar: GeoPackage, GeoJSON, Shapefile, \
                         DXF, NCZ, KML/KMZ, GPX, NMEA, FlatGeobuf, GML, GeoTIFF, VRT, LAS, LAZ ve koordinat listeleri."
                    ));
                    return Task::none();
                };
                self.sources.folders.used(&kept);
                if let Some(open) = opener(kind, path) {
                    return self.update(open);
                }
                let folder = path.parent().map(Path::to_path_buf).unwrap_or_default();
                let file = read_folder(&folder)
                    .ok()
                    .and_then(|l| l.files.into_iter().find(|f| f.name == name))
                    .unwrap_or(SourceFile {
                        name,
                        kind,
                        parts: vec![folder_name(path)],
                    });
                off_thread(
                    move || read_parts(&folder, &file),
                    move |parts| Event::FileRead(kind, parts),
                )
            }
            Kept::Table { path, table } => {
                self.sources.folders.used(&kept);
                self.update(crate::exchange::open_gpkg(
                    path.clone(),
                    Some(table.clone()),
                    true,
                ))
            }
            Kept::Database { connection, table } => {
                self.sources.folders.used(&kept);
                let (connection, table) = (connection.clone(), table.clone());
                if self.db_store.get(&connection).is_none() {
                    self.warn(format!(
                        "“{connection}” bağlantısı artık yok; Veritabanları'nda yeniden tanımlayın."
                    ));
                    return Task::none();
                }
                // The connection's catalogue is read first when it was not; the table is then added from its row.
                if matches!(self.sources.catalogs.get(&connection), Some(Browse::Ready(_))) {
                    return self.add_database_table(connection, table, false);
                }
                self.output(format!(
                    "“{connection}” bağlantısı okunuyor; “{table}” tablosunu Veritabanları'ndaki satırından ekleyin."
                ));
                self.browse_connection_in_sources(&connection)
            }
            Kept::Service { service } => {
                self.sources.folders.used(&kept);
                let read = self.read_service(service);
                let s = service.clone();
                self.open_saved_service(s.kind, s.url, s.connection, None, read)
            }
            Kept::ServiceLayer { service, item, .. } => {
                self.sources.folders.used(&kept);
                let read = self.read_service(service);
                let s = service.clone();
                self.open_saved_service(s.kind, s.url, s.connection, Some(item.clone()), read)
            }
            Kept::Basemap { command, .. } => {
                self.sources.folders.used(&kept);
                let command = command.clone();
                self.basemap_command(&command)
            }
        }
    }

    /// A saved service's offer as the panel read it, for its window.
    fn read_service(&self, service: &SavedService) -> Option<Connecting> {
        match self.sources.services.get(&service_key(service)) {
            Some(Read::Ready(c)) => Some((**c).clone()),
            _ => None,
        }
    }

    /// Bağlı katman olarak aç or Kopya olarak al on a GeoPackage's table: the file's window with the table alone chosen.
    fn open_table(&mut self, origin: Origin, table: String, bind: bool) -> Task<Message> {
        match origin {
            Origin::File(path) => self.update(crate::exchange::open_gpkg(path, Some(table), bind)),
        }
    }

    /// Kapsamı göster (docs/adr/0599 §9): the table's declared box in the project's system, shown and framed.
    fn show_extent(&mut self, srid: Option<u32>, extent: [f64; 4], name: String) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return;
        };
        let session = doc.session;
        match kentos_native_application::sources::extent_in_project(doc.settings(), srid, extent) {
            Ok(b) => {
                self.navigating(|app| {
                    app.zoom_to_bounds(&kentos_commands::Bounds {
                        min_x: b[0],
                        min_y: b[1],
                        max_x: b[2],
                        max_y: b[3],
                    })
                });
                self.sources.extent_mark = Some((session, b, name.clone()));
                self.say(
                    Level::Info,
                    format!(
                        "“{name}” tablosunun kapsamı gösterildi. İşareti kaldır çerçeveyi siler."
                    ),
                );
            }
            Err(why) => self.warn(format!("“{name}” tablosunun kapsamı gösterilemez: {why}.")),
        }
    }

    #[cfg(test)]
    /// The layer a table is bound to in the open drawing (docs/adr/0599 §7): by its file's path or database's
    /// address and its name.
    pub(crate) fn bound_layer_of(&self, path: &str, table: &str) -> Option<String> {
        let doc = self.document.as_ref()?;
        doc.settings()
            .sources
            .iter()
            .find(|l| l.table == table && (l.path == path || Path::new(&l.path) == Path::new(path)))
            .map(|l| l.layer.clone())
    }

    /// Kapsamı göster's frame for the drawing as it is.
    pub(crate) fn extent_mark(&self) -> Option<([f64; 4], String)> {
        let doc = self.document.as_ref()?;
        self.sources
            .extent_mark
            .as_ref()
            .filter(|(session, ..)| *session == doc.session)
            .map(|(_, b, name)| (*b, name.clone()))
    }

    fn read_tables(&mut self, origin: Origin) -> Task<Message> {
        self.sources.tables.insert(origin.clone(), Read::Reading);
        let key = tables_key(&origin);
        let back = origin.clone();
        let (task, handle) = off_thread(
            move || crate::exchange::list_tables(origin.path()),
            move |r| Event::Tables(back, r),
        )
        .abortable();
        self.sources.requests.insert(key, handle.abort_on_drop());
        task
    }

    fn read_sample(&mut self, key: String) -> Task<Message> {
        let Some((origin, table)) = table_of_key(&key) else {
            return Task::none();
        };
        let columns: Vec<String> = match self.sources.tables.get(&origin) {
            Some(Read::Ready(tables)) => tables
                .iter()
                .find(|t| t.name == table)
                .map(|t| {
                    t.columns
                        .iter()
                        .take(SAMPLE_COLUMNS)
                        .map(|c| c.name.clone())
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        let settings = self.document.as_ref().map(|d| d.settings().clone());
        self.sources.samples.insert(key.clone(), Read::Reading);
        let back = key.clone();
        let (task, handle) = off_thread(
            move || sample_rows(&origin, &table, columns, settings),
            move |r| Event::Sample(back, r),
        )
        .abortable();
        self.sources.requests.insert(key, handle.abort_on_drop());
        task
    }

    fn read_saved_service(&mut self, key: String) -> Task<Message> {
        let Some(service) = self
            .sources
            .folders
            .services()
            .iter()
            .find(|s| service_key(s) == key)
            .cloned()
        else {
            return Task::none();
        };
        let conn = service.connection.as_ref().and_then(|id| {
            let c = self
                .document
                .as_ref()?
                .model
                .settings()
                .connections
                .iter()
                .find(|c| &c.id == id)?
                .clone();
            let secret = crate::services::secrets::secrets().get(&c.origin, &c.id);
            Some((c, secret))
        });
        self.sources.services.insert(key.clone(), Read::Reading);
        let back = key.clone();
        let (task, handle) = off_thread(
            move || {
                crate::services::window::connect_now(
                    service.kind,
                    &service.url,
                    conn.as_ref().map(|(c, s)| (c, s.as_ref())),
                )
            },
            move |r| Event::ServiceRead(back, r),
        )
        .abortable();
        self.sources.requests.insert(key, handle.abort_on_drop());
        task
    }

    /// A row opened (its things read) or closed (what was read forgotten).
    fn toggle_source(&mut self, key: String) -> Task<Message> {
        let s = &mut self.sources;
        if s.open.remove(&key) {
            s.requests.remove(&key);
            if let Some(path) = key.strip_prefix("d:") {
                s.listings.remove(Path::new(path));
            } else if let Some(view) = key.strip_prefix("l:") {
                s.lists.retain(|v, _| *v != view);
            } else if let Some(id) = key.strip_prefix("p:") {
                s.drawings.remove(id);
            } else if let Some(origin) = origin_of_key(&key) {
                s.tables.remove(&origin);
            } else if key.starts_with("gt:") {
                s.samples.remove(&key);
            } else if key.starts_with("s:") {
                s.services.remove(&key);
            } else if let Some(name) = key.strip_prefix("c:") {
                s.catalogs.remove(name);
            }
            return Task::none();
        }
        s.open.insert(key.clone());
        if let Some(name) = key.strip_prefix("c:") {
            return self.browse_connection(name);
        }
        if let Some(path) = key.strip_prefix("d:") {
            return self.list_folder(PathBuf::from(path));
        }
        if let Some(view) = key.strip_prefix("l:") {
            return self.read_list(view);
        }
        if let Some(id) = key.strip_prefix("p:") {
            return self.download(id.to_owned());
        }
        if let Some(origin) = origin_of_key(&key) {
            return self.read_tables(origin);
        }
        if key.starts_with("gt:") {
            return self.read_sample(key);
        }
        if key.starts_with("s:") {
            return self.read_saved_service(key);
        }
        Task::none()
    }

    /// A connection's catalogue read off the UI thread; an answer for a row closed since is dropped.
    pub(crate) fn browse_connection(&mut self, name: &str) -> Task<Message> {
        let Some(c) = self.db_store.get(name) else {
            return Task::none();
        };
        self.sources.catalogs.insert(name.to_owned(), Browse::Reading);
        let back = name.to_owned();
        off_thread(
            move || probe::browse(&c, None).map(Arc::new),
            move |r| Event::Browsed(back, r, true),
        )
    }

    /// Kaynaklar shown with the connection named `name` open (a connection just saved).
    pub(crate) fn browse_connection_in_sources(&mut self, name: &str) -> Task<Message> {
        self.show_sources();
        self.sources.open_databases();
        self.sources.open.insert(connection_key(name));
        self.sources.focused = Some(connection_key(name));
        self.browse_connection(name)
    }

    /// Katman olarak ekle on a connection's table: refused with why when it cannot be bound, the system's question
    /// when it is in another one, bound otherwise.
    fn add_database_table(&mut self, name: String, table: String, area: bool) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O) ya da yeni proje oluşturun.");
            return Task::none();
        };
        let project = doc.settings().srid;
        let Some(Browse::Ready(catalog)) = self.sources.catalogs.get(&name) else {
            return Task::none();
        };
        let Some(r) = catalog.relations.iter().find(|r| r.table() == table) else {
            return Task::none();
        };
        if let Some(why) = r.why_not() {
            self.warn(format!("“{table}” bağlı katman olarak eklenemez: {why}"));
            return Task::none();
        }
        if let Some(srid) = r.srid.filter(|&s| s != project) {
            self.sources.reproject = Some(ReprojectAsk {
                connection: name,
                table,
                area,
                srid,
            });
            return Task::none();
        }
        self.bind_from_sources(&name, &table, false, area)
    }

    fn bind_from_sources(&mut self, name: &str, table: &str, reproject: bool, area: bool) -> Task<Message> {
        let Some(c) = self.db_store.get(name) else {
            return Task::none();
        };
        let geometry = match self.sources.catalogs.get(name) {
            Some(Browse::Ready(catalog)) => catalog
                .relations
                .iter()
                .find(|r| r.table() == table)
                .is_some_and(|r| r.geometry.is_some()),
            _ => true,
        };
        let area = area.then(|| {
            let b = self.viewport.camera.visible_bounds();
            [b.min_x, b.min_y, b.max_x, b.max_y]
        });
        match self.bind_database(&c.address(), &[(table.to_owned(), geometry)], reproject, area) {
            Ok(done) => {
                self.zoom_to(&done.slots);
                self.announce(
                    if done.warnings.is_empty() {
                        kentos_interaction::Level::Success
                    } else {
                        kentos_interaction::Level::Warn
                    },
                    format!(
                        "“{table}” ({}) bağlı katman oldu, {} nesne. Değişiklikleri Kaynağa yaz ile veritabanına yazın.",
                        c.name, done.objects
                    ),
                );
                for w in done.warnings {
                    self.say(kentos_interaction::Level::Warn, w);
                }
            }
            Err(why) => self.error(why),
        }
        Task::none()
    }

    fn list_folder(&mut self, path: PathBuf) -> Task<Message> {
        self.sources.listings.insert(path.clone(), Read::Reading);
        let back = path.clone();
        off_thread(move || read_folder(&path), move |r| Event::Listed(back, r))
    }

    fn read_list(&mut self, view: &str) -> Task<Message> {
        let Some(&(catalog_view, key, _)) = LISTS.iter().find(|(_, k, _)| *k == view) else {
            return Task::none();
        };
        let Some(client) = self.cloud.signed_in().cloned() else {
            return Task::none();
        };
        self.sources.lists.insert(key, Read::Reading);
        let mut query = CatalogQuery::new(catalog_view);
        query.sort = Some(CatalogSort::Name);
        query.limit = Some(LIMIT);
        let (task, handle) = Task::perform(client.catalog(&query), move |result| {
            msg(Event::Projects(
                key,
                result
                    .map(|page| (page.projects, u64::from(page.total)))
                    .map_err(|e| e.message.clone()),
            ))
        })
        .abortable();
        self.sources
            .requests
            .insert(list_key(key), handle.abort_on_drop());
        task
    }

    fn download(&mut self, id: String) -> Task<Message> {
        let (Some(client), Some(p)) = (
            self.cloud.signed_in().cloned(),
            self.sources.project(&id).cloned(),
        ) else {
            return Task::none();
        };
        let (Some(tenant), Some(project)) =
            (crate::cloud::uuid(&p.tenant_id), crate::cloud::uuid(&p.id))
        else {
            return Task::none();
        };
        self.sources.drawings.insert(id.clone(), Read::Reading);
        let storage = p.storage;
        let fetch = async move {
            let got = if storage == ProjectStorage::File {
                let revs = client
                    .file_revisions(tenant, project)
                    .await
                    .map_err(|e| e.message.clone())?;
                let Some(current) = revs.current.as_deref().and_then(|r| r.parse::<u64>().ok())
                else {
                    return Err("Projenin henüz kaydedilmiş bir revizyonu yok.".to_owned());
                };
                let listed = revs
                    .revisions
                    .iter()
                    .find(|r| r.revision == current.to_string())
                    .map(|r| r.sha256.clone());
                let got = client
                    .download_revision(tenant, project, current, None)
                    .await
                    .map_err(|e| e.message.clone())?;
                if listed.is_some_and(|want| want != got.sha256) {
                    return Err("İndirilen dosya sunucudakiyle aynı değil (SHA-256 tutmuyor); dosya kullanılmadı. Bağlantınızı denetleyip yeniden deneyin.".to_owned());
                }
                got
            } else {
                client
                    .snapshot(tenant, project, None)
                    .await
                    .map_err(|e| e.message.clone())?
            };
            Ok(got.bytes)
        };
        let key = project_key(&id);
        let back = id.clone();
        let (task, handle) = Task::perform(fetch, move |result: Result<Vec<u8>, String>| {
            (back.clone(), result)
        })
        .abortable();
        self.sources.requests.insert(key, handle.abort_on_drop());
        // The bytes become a drawing off the UI thread.
        task.then(|(id, result)| match result {
            Err(why) => Task::done(msg(Event::Downloaded(id, Err(why)))),
            Ok(bytes) => off_thread(
                move || drawing_of(&bytes),
                move |r| Event::Downloaded(id, r),
            ),
        })
    }
}

impl SourcesPanel {
    /// Veritabanları open (connections were added).
    pub(crate) fn open_databases(&mut self) {
        self.open.insert(DATABASES.to_owned());
    }

    /// What was read of a connection gone or renamed.
    pub(crate) fn forget_connection(&mut self, name: &str) {
        self.catalogs.remove(name);
        self.open.remove(&connection_key(name));
        if self.reproject.as_ref().is_some_and(|r| r.connection == name) {
            self.reproject = None;
        }
    }

    /// A project a list holds, by id.
    pub(crate) fn project(&self, id: &str) -> Option<&ProjectSummary> {
        self.lists.values().find_map(|r| match r {
            Read::Ready((projects, _)) => projects.iter().find(|p| p.id == id),
            _ => None,
        })
    }
}

/// A row's key: a folder by its path, a list by its view, a project by its id.
pub(crate) fn folder_key(path: &Path) -> String {
    format!("d:{}", path.display())
}

pub(crate) fn list_key(view: &str) -> String {
    format!("l:{view}")
}

pub(crate) fn project_key(id: &str) -> String {
    format!("p:{id}")
}

/// A GeoPackage's row (`g:`), opened into its tables.
pub(crate) fn tables_key(origin: &Origin) -> String {
    match origin {
        Origin::File(p) => format!("g:{}", p.display()),
    }
}

fn origin_of_key(key: &str) -> Option<Origin> {
    key.strip_prefix("g:").map(|p| Origin::File(PathBuf::from(p)))
}

/// A table's row, opened into its preview.
pub(crate) fn table_key(origin: &Origin, table: &str) -> String {
    format!("gt:{}\0{table}", tables_key(origin))
}

fn table_of_key(key: &str) -> Option<(Origin, String)> {
    let (origin, table) = key.strip_prefix("gt:")?.rsplit_once('\0')?;
    Some((origin_of_key(origin)?, table.to_owned()))
}

/// A saved service's row, opened into its layers.
pub(crate) fn service_key(s: &SavedService) -> String {
    format!(
        "s:{}\0{}\0{}",
        s.kind.name(),
        s.url,
        s.connection.as_deref().unwrap_or_default()
    )
}

/// The window a file of these kinds opens with (docs/adr/0599 §1); none for those whose files the panel reads.
fn opener(kind: SourceKind, path: &Path) -> Option<Message> {
    let path = path.to_path_buf();
    Some(match kind {
        SourceKind::GeoPackage => crate::exchange::open_gpkg(path, None, false),
        SourceKind::FlatGeobuf => crate::exchange::open_fgb(path),
        SourceKind::Gml => crate::exchange::open_gml(path),
        SourceKind::Raster => crate::rasters::add::open_file(path),
        SourceKind::PointCloud => crate::pointclouds::add::open_file(path),
        _ => return None,
    })
}

/// A table's first rows through `cad.sources.query` (docs/adr/0599 §2), each as “sütun: değer” words, in a drawing of
/// its own with the project's settings (the query reads no drawing).
fn sample_rows(
    origin: &Origin,
    table: &str,
    columns: Vec<String>,
    settings: Option<kentos_commands::ProjectSettings>,
) -> Result<Vec<String>, String> {
    let mut doc = crate::exchange::listing_document()?;
    if let Some(settings) = settings {
        doc.set_settings(settings);
    }
    let input = SourcesQuery {
        path: Some(origin.path()),
        table: Some(table.to_owned()),
        fields: Some(columns.clone()),
        geometry: Some(false),
        limit: Some(SAMPLE_ROWS),
        ..SourcesQuery::default()
    };
    match kentos_native_application::sources_query::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ) {
        CommandResult::Completed {
            output: SourcesQueried { rows, .. },
            ..
        } => Ok(rows
            .iter()
            .map(|r| {
                let mut words = String::new();
                for c in &columns {
                    if !words.is_empty() {
                        words.push_str(" · ");
                    }
                    let _ = write!(words, "{c}: {}", r.attrs.get(c).map_or("—", String::as_str));
                }
                if words.is_empty() {
                    words = format!("#{}", r.key.unwrap_or_default());
                }
                words
            })
            .collect()),
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => Err(error.message),
        _ => Err("okunamadı".into()),
    }
}

/// While a row is dragged (docs/adr/0599 §8): the pointer and the button let go, wherever they are.
pub(crate) fn dragging(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(msg(Event::DragAt(position)))
        }
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(msg(Event::DragEnd))
        }
        _ => None,
    }
}

/// A file dropped on the window from the system's file manager (docs/adr/0599 §8).
pub(crate) fn dropped(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(msg(Event::Dropped(path)))
        }
        _ => None,
    }
}

/// A connection's row by its name.
pub(crate) fn connection_key(name: &str) -> String {
    format!("c:{name}")
}

/// A folder's name as the panel shows it (the path's last part).
pub(crate) fn folder_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

// ── The folders kept ──────────────────────────────────────────────────

const FORMAT: &str = "kentos.source-folders";
const FILE: &str = "kaynak-klasorleri.json";
/// The list's version: 2 adds the services and the recent and favourite sources (docs/adr/0599);
/// version 1 (the folders alone) is read.
const VERSION: u32 = 2;

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    format: String,
    version: u32,
    folders: Vec<PathBuf>,
    #[serde(default)]
    services: Vec<SavedService>,
    #[serde(default)]
    recent: Vec<Kept>,
    #[serde(default)]
    favourites: Vec<Kept>,
}

/// The folders added, oldest first, the services added, the recent and favourite sources, and where
/// the list is kept (the device's: beside the recent files); in memory only without a folder.
#[derive(Debug, Default)]
pub struct SourceFolders {
    file: Option<PathBuf>,
    list: Vec<PathBuf>,
    services: Vec<SavedService>,
    recent: Vec<Kept>,
    favourites: Vec<Kept>,
}

impl SourceFolders {
    pub fn memory() -> Self {
        Self::default()
    }

    /// The list kept in `folder`; an unreadable file is an empty list.
    pub fn open(folder: &Path) -> Self {
        let stored = std::fs::read_to_string(folder.join(FILE))
            .ok()
            .and_then(|text| serde_json::from_str::<Stored>(&text).ok())
            .filter(|s| s.format == FORMAT && (1..=VERSION).contains(&s.version))
            .unwrap_or_default();
        Self {
            file: Some(folder.join(FILE)),
            list: stored.folders,
            services: stored.services,
            recent: stored.recent,
            favourites: stored.favourites,
        }
    }

    pub fn list(&self) -> &[PathBuf] {
        &self.list
    }

    pub fn services(&self) -> &[SavedService] {
        &self.services
    }

    pub fn recent(&self) -> &[Kept] {
        &self.recent
    }

    pub fn favourites(&self) -> &[Kept] {
        &self.favourites
    }

    /// Adds a folder; false when it is in the list already.
    pub fn add(&mut self, path: &Path) -> bool {
        if self.list.iter().any(|p| p == path) {
            return false;
        }
        self.list.push(path.to_path_buf());
        self.write();
        true
    }

    /// Takes a folder off the list (the folder itself stays); whether it was there.
    pub fn remove(&mut self, path: &Path) -> bool {
        let before = self.list.len();
        self.list.retain(|p| p != path);
        let removed = self.list.len() != before;
        if removed {
            self.write();
        }
        removed
    }

    /// Adds a service; one of the same kind, address and connection takes the new name.
    pub fn add_service(&mut self, service: SavedService) {
        match self.services.iter_mut().find(|s| {
            s.kind == service.kind && s.url == service.url && s.connection == service.connection
        }) {
            Some(s) => s.name = service.name,
            None => self.services.push(service),
        }
        self.write();
    }

    pub fn remove_service(&mut self, at: usize) -> Option<SavedService> {
        (at < self.services.len()).then(|| {
            let gone = self.services.remove(at);
            self.write();
            gone
        })
    }

    /// `kept` used: it becomes the newest recent source.
    pub fn used(&mut self, kept: &Kept) {
        self.recent = recent_with(&self.recent, kept);
        self.write();
    }

    /// Sık kullanılanlara ekle or çıkar; false when the list is full.
    pub fn star(&mut self, kept: &Kept) -> bool {
        let (favourites, refused) = starred(&self.favourites, kept);
        if refused {
            return false;
        }
        self.favourites = favourites;
        self.write();
        true
    }

    pub fn is_favourite(&self, kept: &Kept) -> bool {
        self.favourites.contains(kept)
    }

    /// Writes the list whole, then renames it into place; a failure is not
    /// something the user must act on.
    fn write(&self) {
        let Some(file) = &self.file else { return };
        let stored = Stored {
            format: FORMAT.to_owned(),
            version: VERSION,
            folders: self.list.clone(),
            services: self.services.clone(),
            recent: self.recent.clone(),
            favourites: self.favourites.clone(),
        };
        let Ok(text) = serde_json::to_string_pretty(&stored) else {
            return;
        };
        if let Some(dir) = file.parent()
            && std::fs::create_dir_all(dir).is_err()
        {
            return;
        }
        let temp = file.with_extension("json.yaziliyor");
        if std::fs::write(&temp, text).is_ok() {
            let _ = std::fs::rename(&temp, file);
        }
    }
}
