//! A GeoPackage opened (docs/adr/9116): from Dosya › Aç, a recent file, a file dropped on a window with no drawing, or
//! Veri kaynağından aç on a file that keeps no project. The native opening runs off the interface's thread: a file's
//! only project opens as a project kept there; a file of several projects opens Veri kaynağından aç on it to pick one;
//! a file of none opens as a new project of its tables, in memory: not a byte of the file changes (docs/adr/9122), and
//! Kaydet offers to keep the project in that file, showing the file and the name before it writes. What the opening
//! found is said: the system chosen, what was not taken.

use std::path::PathBuf;

use iced::Task;
use kentos_interaction::Level;
use kentos_native_application::gpkg_open::{self, Opened};

use crate::app::{App, Message};
use crate::document::{Document, Source, StoredSource};
use crate::opening::Purpose;

/// What opening a GeoPackage says back.
#[derive(Debug, Clone)]
pub enum Event {
    /// The file read: what it opens to, or why not.
    Read { path: PathBuf, purpose: Purpose, read: Box<Result<Read, String>> },
}

/// What a GeoPackage opened to.
#[derive(Debug, Clone)]
pub enum Read {
    /// Its only project, or the one asked for, read.
    Kept(Box<Document>),
    /// More than one project: their names.
    Several(Vec<String>),
    /// A new project of its tables, kept nowhere yet; what the opening found.
    Built { doc: Box<Document>, name: String, notes: Vec<gpkg_open::Note> },
}

fn message(e: Event) -> Message {
    Message::Gis(crate::gis::Event::GpkgOpen(e))
}

/// The file's name for a sentence.
fn file_name(path: &std::path::Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

/// Opens the GeoPackage at `path` (the project `name`, when one is asked for) on a thread of its own.
fn read(path: &std::path::Path, name: Option<&str>) -> Result<Read, String> {
    let address = gpkg_open::address_of(path);
    match gpkg_open::open(path, name)? {
        Opened::Kept { name, bytes } => {
            let snapshot = kentos_native_application::project_store::decode(&address, &bytes)
                .map_err(|e| format!("“{name}” projesi okunamadı: {e}"))?;
            let mut doc = Document::from_v2(snapshot, None)?;
            doc.source = Source::Stored(Box::new(StoredSource { address, schema: None, name }));
            Ok(Read::Kept(Box::new(doc)))
        }
        Opened::Several(kept) => Ok(Read::Several(kept.into_iter().map(|p| p.name).collect())),
        Opened::Built(built) => {
            let gpkg_open::Built { document, name, notes } = *built;
            let mut doc = Document::of_model(document);
            doc.source = Source::Offered(Box::new(StoredSource { address, schema: None, name: name.clone() }));
            Ok(Read::Built { doc: Box::new(doc), name, notes })
        }
    }
}

impl App {
    /// Opens a GeoPackage as a project (its only project, a pick of several, or a new one of its tables); `name`
    /// asks for one project. Only a program with data layers opens one (CBS, Harita, and kentos-cad whatever project
    /// is on screen: it opens in a tab of its own; docs/adr/9195 §2).
    pub(crate) fn open_geopackage(&mut self, path: PathBuf, purpose: Purpose, name: Option<String>) -> Task<Message> {
        if !crate::module_surfaces::shows_panel_in(self.program, crate::app::Panel::Sources) {
            self.warn(format!(
                "“{}” bir GeoPackage: veri katmanı olan programlarda (KentOS CBS, KentOS Harita) proje olarak açılır.",
                file_name(&path)
            ));
            return Task::none();
        }
        self.say(Level::Info, format!("“{}” açılıyor…", file_name(&path)));
        let at = path.clone();
        crate::db_connections::off_thread(
            move || read(&at, name.as_deref()),
            move |read| message(Event::Read { path, purpose, read: Box::new(read) }),
        )
    }

    pub(crate) fn gpkg_open_event(&mut self, event: Event) -> Task<Message> {
        let Event::Read { path, purpose, read } = event;
        let file = file_name(&path);
        match *read {
            Err(why) => {
                match self.gis.project_store.as_mut() {
                    Some(w) => w.failed(why),
                    None => self.warn(why),
                }
                Task::none()
            }
            Ok(Read::Several(names)) => {
                self.say(
                    Level::Info,
                    format!("“{file}” içinde {} proje var: {}. Açılacak olanı seçin.", names.len(), names.join(", ")),
                );
                self.open_project_store_at(path)
            }
            Ok(Read::Kept(doc)) => self.place_geopackage(path, purpose, doc, Vec::new()),
            Ok(Read::Built { doc, name, notes }) => {
                let layers = doc.model.layers().leaves().iter().filter(|n| n.data.is_some()).count();
                // The tile tables' rasters (docs/adr/9117 §1), said beside the data layers.
                let rasters = doc
                    .model
                    .entities()
                    .filter(|e| matches!(e, kentos_commands::Entity::Raster(r) if r.raster.tiles.is_some()))
                    .count();
                let rasters = if rasters > 0 { format!(", döşeme tablolarından {rasters} raster") } else { String::new() };
                let mut said = vec![(
                    Level::Success,
                    format!(
                        "“{file}” içinde KentOS projesi yok: tabloları “{name}” adıyla bir harita olarak açıldı ({layers} veri katmanı{rasters}). Dosyaya bir şey yazılmadı; Kaydet, projeyi bu dosyada saklamayı önerir."
                    ),
                )];
                said.extend(notes.into_iter().map(|n| {
                    let level = match n.code {
                        "mixed_systems" => Level::Info,
                        _ => Level::Warn,
                    };
                    (level, n.text)
                }));
                self.place_geopackage(path, purpose, doc, said)
            }
        }
    }

    /// A GeoPackage's project on screen (in its own tab for Yeni sekmede aç), the file first in the recent files, then
    /// what the opening said.
    fn place_geopackage(
        &mut self,
        path: PathBuf,
        purpose: Purpose,
        doc: Box<Document>,
        said: Vec<(Level, String)>,
    ) -> Task<Message> {
        if purpose == Purpose::Tab && !self.park_active() {
            return Task::none();
        }
        if self.gis.project_store.is_some() {
            self.gis.project_store = None;
            self.dialog = None;
        }
        let kept = doc.offered_source().is_none();
        let task = self.update(Message::Opened(Some(Ok(doc))));
        let module = self.document_module();
        let what = if kept { "GeoPackage — proje dosyanın içinde" } else { "GeoPackage — tabloları harita olarak" };
        self.recent.add_in(&path, what.into(), module);
        for (level, words) in said {
            self.say(level, words);
        }
        task
    }
}
