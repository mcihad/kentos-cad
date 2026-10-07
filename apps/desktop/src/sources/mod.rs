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
//! `take_layer_into`). What is opened is read again when it is opened again.
//! Folders are read and drawings decoded off the UI thread; an answer for a
//! row closed since is dropped.

mod view;

#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::Task;
use iced::futures::channel::mpsc;
use kentos_cloud::CatalogQuery;
use kentos_contracts::{
    CatalogSort, CatalogView, DocumentSnapshotV2, ProjectStorage, ProjectSummary,
};
use kentos_interaction::sources::{Listing, SourceEntry, SourceFile, SourceKind, listing};
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
/// The two sections' keys; they start open.
pub(crate) const FOLDERS: &str = "klasorler";
pub(crate) const KENTOS: &str = "kentos";

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
    /// Requests under way by row key, stopped when dropped (the row closed).
    requests: HashMap<String, iced::task::Handle>,
    /// The row chosen last (a click), by key.
    pub(crate) focused: Option<String>,
}

impl Default for SourcesPanel {
    fn default() -> Self {
        Self {
            folders: SourceFolders::memory(),
            open: BTreeSet::from([FOLDERS.to_owned(), KENTOS.to_owned()]),
            listings: HashMap::new(),
            lists: HashMap::new(),
            drawings: HashMap::new(),
            requests: HashMap::new(),
            focused: None,
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
                };
                let first = parts.drain(..).next();
                return self.exchange_event(ExchangeEvent::Picked(kind, first.map(Ok)));
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
        }
        Task::none()
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
            }
            return Task::none();
        }
        s.open.insert(key.clone());
        if let Some(path) = key.strip_prefix("d:") {
            return self.list_folder(PathBuf::from(path));
        }
        if let Some(view) = key.strip_prefix("l:") {
            return self.read_list(view);
        }
        if let Some(id) = key.strip_prefix("p:") {
            return self.download(id.to_owned());
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

#[derive(Serialize, Deserialize)]
struct Stored {
    format: String,
    version: u32,
    folders: Vec<PathBuf>,
}

/// The folders added, oldest first, and where the list is kept (the
/// device's: beside the recent files); in memory only without a folder.
#[derive(Debug, Default)]
pub struct SourceFolders {
    file: Option<PathBuf>,
    list: Vec<PathBuf>,
}

impl SourceFolders {
    pub fn memory() -> Self {
        Self::default()
    }

    /// The list kept in `folder`; an unreadable file is an empty list.
    pub fn open(folder: &Path) -> Self {
        let list = std::fs::read_to_string(folder.join(FILE))
            .ok()
            .and_then(|text| serde_json::from_str::<Stored>(&text).ok())
            .filter(|s| s.format == FORMAT && s.version == 1)
            .map(|s| s.folders)
            .unwrap_or_default();
        Self {
            file: Some(folder.join(FILE)),
            list,
        }
    }

    pub fn list(&self) -> &[PathBuf] {
        &self.list
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

    /// Writes the list whole, then renames it into place; a failure is not
    /// something the user must act on.
    fn write(&self) {
        let Some(file) = &self.file else { return };
        let stored = Stored {
            format: FORMAT.to_owned(),
            version: 1,
            folders: self.list.clone(),
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
