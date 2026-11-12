//! Malzemeler › Kitaplık (docs/adr/0791): the material libraries outside the drawing, as Revit's Material Browser
//! shows its libraries beside the project's materials and Archicad's Attribute Manager takes Building Materials from
//! another file. KentOS gives its own (`kentos-temel`); an office keeps `.kmalzeme` files (`kentos.materials` 1) in its
//! folders, kept beside the recent files (`malzeme-kitapliklari.json`). A library's materials are shown with what the
//! project has of each; Seçilenleri al takes the chosen through `cad.materials.edit` `import` (one undo step “Malzeme”),
//! the project's kept or the library's put in their place as the user chose; Kitaplığa yaz writes the project's
//! materials to a file through `export`.

use std::path::{Path, PathBuf};

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    ArchMaterial, CommandResult, MaterialConflict, MaterialsEdit, MaterialsFile, MaterialsOperation,
};
use kentos_interaction::Level;
use kentos_ui::icon::icon;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::{label, style};
use serde::{Deserialize, Serialize};

use crate::app::{App, Message};
use crate::exchange::words::{self, Kind};
use crate::materials_window::{Conflict, Event, TITLE, Tab, report_words};

/// The Kitaplık view's buttons, which a trace names.
pub const TAKE: &str = "Seçilenleri al";
pub const EXPORT: &str = "Kitaplığa yaz…";
pub const ADD_FOLDER: &str = "Klasör ekle…";
const REMOVE_FOLDER: &str = "Klasörü çıkar";
const CLOSE: &str = "Kapat";

const FORMAT: &str = "kentos.material-libraries";
const FILE: &str = "malzeme-kitapliklari.json";
const VERSION: u32 = 1;
/// A library file's extension.
pub const EXTENSION: &str = "kmalzeme";

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    format: String,
    version: u32,
    folders: Vec<PathBuf>,
}

/// The office's library folders, kept beside the recent files.
#[derive(Debug, Default, Clone)]
pub struct Libraries {
    file: Option<PathBuf>,
    pub folders: Vec<PathBuf>,
}

/// Where a library is: KentOS's own by its name, or a file.
#[derive(Debug, Clone, PartialEq)]
pub enum Place {
    Kentos(&'static str),
    File(PathBuf),
}

/// A library as the window lists it: the name it is shown with, where it is, its materials or why it is not read.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub place: Place,
    pub read: Result<MaterialsFile, String>,
}

/// What the project has of a library's material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// None of its id or name: taken as new.
    New,
    /// The same one.
    Same,
    /// One of its id, other fields: kept or replaced.
    Differs,
    /// Another material of its name: kept, else refused.
    NameTaken,
}

impl Status {
    pub fn words(self) -> &'static str {
        match self {
            Status::New => "Yeni",
            Status::Same => "Projede aynısı var",
            Status::Differs => "Projede var · farklı",
            Status::NameTaken => "Projede aynı adlı başka malzeme",
        }
    }
}

/// What the project has of `m`.
pub fn status(project: &[ArchMaterial], m: &ArchMaterial) -> Status {
    let folded = m.name.to_lowercase();
    match project.iter().find(|p| p.id == m.id) {
        Some(p) if p == m => Status::Same,
        Some(_) => Status::Differs,
        None if project.iter().any(|p| p.name.to_lowercase() == folded) => Status::NameTaken,
        None => Status::New,
    }
}

fn name_of(path: &Path) -> String {
    path.file_stem()
        .or(path.file_name())
        .map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

/// A library file's materials, or why they are not read.
pub fn read(path: &Path) -> Result<MaterialsFile, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|_| format!("{} okunamadı: dosya yok ya da açılamıyor.", name_of(path)))?;
    let file: MaterialsFile = serde_json::from_str(&text)
        .map_err(|e| format!("{} bir malzeme kitaplığı değil ({e}).", name_of(path)))?;
    if let Some((_, why)) = file.problem() {
        return Err(why);
    }
    Ok(file)
}

impl Libraries {
    /// The folders kept in `folder`; an unreadable file is none.
    pub fn open(folder: &Path) -> Self {
        let stored = std::fs::read_to_string(folder.join(FILE))
            .ok()
            .and_then(|text| serde_json::from_str::<Stored>(&text).ok())
            .filter(|s| s.format == FORMAT && s.version == VERSION)
            .unwrap_or_default();
        Self {
            file: Some(folder.join(FILE)),
            folders: stored.folders,
        }
    }

    fn keep(&self) {
        let Some(file) = &self.file else {
            return;
        };
        let stored = Stored {
            format: FORMAT.into(),
            version: VERSION,
            folders: self.folders.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&stored) {
            let _ = std::fs::write(file, text + "\n");
        }
    }

    /// Adds a folder; false when it is there already.
    pub fn add(&mut self, folder: &Path) -> bool {
        if self.folders.iter().any(|f| f == folder) {
            return false;
        }
        self.folders.push(folder.to_path_buf());
        self.keep();
        true
    }

    pub fn remove(&mut self, folder: &Path) {
        self.folders.retain(|f| f != folder);
        self.keep();
    }

    /// Every library: KentOS's, then each folder's `.kmalzeme` files in the natural order.
    pub fn entries(&self) -> Vec<Entry> {
        let mut out: Vec<Entry> = kentos_native_application::materials_edit::BUILTIN_LIBRARIES
            .iter()
            .map(|(name, _)| {
                let read = kentos_native_application::materials_edit::builtin_library(name)
                    .ok_or_else(|| format!("KentOS kitaplığı “{name}” okunamadı."));
                Entry {
                    name: read.as_ref().map_or_else(|_| (*name).to_owned(), |f| f.library.clone()),
                    place: Place::Kentos(name),
                    read,
                }
            })
            .collect();
        for folder in &self.folders {
            let Ok(list) = std::fs::read_dir(folder) else {
                continue;
            };
            let mut files: Vec<PathBuf> = list
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
                        && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))
                })
                .collect();
            let names: Vec<String> = files.iter().map(|p| name_of(p)).collect();
            let order = kentos_geometry_core::ops::point_editor::natural_order(&names);
            files = order.into_iter().map(|i| files[i as usize].clone()).collect();
            for path in files {
                let read = read(&path);
                out.push(Entry {
                    name: read.as_ref().map_or_else(|_| name_of(&path), |f| f.library.clone()),
                    place: Place::File(path),
                    read,
                });
            }
        }
        out
    }
}

fn msg(e: Event) -> Message {
    Message::Materials(e)
}

/// The import that takes `entry`'s ticked materials, `conflict` for one the project has; none when none is ticked.
pub fn import_of(entry: &Entry, picked: &[bool], conflict: Conflict) -> Option<MaterialsOperation> {
    let file = entry.read.as_ref().ok()?;
    let materials: Vec<String> = file
        .materials
        .iter()
        .zip(picked)
        .filter(|(_, p)| **p)
        .map(|(m, _)| m.id.clone())
        .collect();
    if materials.is_empty() {
        return None;
    }
    let (library, name) = match &entry.place {
        Place::Kentos(n) => (None, Some((*n).to_owned())),
        Place::File(_) => (Some(file.clone()), None),
    };
    Some(MaterialsOperation::Import {
        library,
        name,
        materials,
        on_conflict: match conflict {
            Conflict::Keep => MaterialConflict::Keep,
            Conflict::Replace => MaterialConflict::Replace,
        },
    })
}

impl App {
    /// The libraries listed again, the chosen one's materials ticked afresh (the new ones) and the take previewed.
    pub(crate) fn material_library_read(&mut self) {
        let entries = self.material_libraries.entries();
        let project = self.document.as_ref().map(|d| d.model.materials().to_vec()).unwrap_or_default();
        let Some(w) = self.materials.as_mut() else {
            return;
        };
        w.entry = w.entry.min(entries.len().saturating_sub(1));
        w.picked = entries
            .get(w.entry)
            .and_then(|e| e.read.as_ref().ok())
            .map(|f| f.materials.iter().map(|m| status(&project, m) == Status::New).collect())
            .unwrap_or_default();
        w.entries = entries;
        self.material_library_preview();
    }

    /// What Seçilenleri al would do, nothing written.
    fn material_library_preview(&mut self) {
        let (Some(w), Some(doc)) = (self.materials.as_ref(), self.document.as_ref()) else {
            return;
        };
        let preview = w.entries.get(w.entry).and_then(|e| import_of(e, &w.picked, w.conflict)).map(|operation| {
            match kentos_native_application::materials_edit::plan_in(
                &doc.model,
                &MaterialsEdit { operation, expected_revision: None },
            ) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                    Err(error.message)
                }
                _ => Err("Önizlenemedi.".into()),
            }
        });
        if let Some(w) = self.materials.as_mut() {
            w.take_preview = preview;
        }
    }

    pub(crate) fn material_library_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Tab(t) => {
                if let Some(w) = self.materials.as_mut() {
                    w.tab = t;
                }
                if t == Tab::Library {
                    self.material_library_read();
                }
            }
            Event::Entry(i) => {
                if let Some(w) = self.materials.as_mut() {
                    w.entry = i;
                }
                self.material_library_read();
            }
            Event::Pick(i) => {
                if let Some(p) = self.materials.as_mut().and_then(|w| w.picked.get_mut(i)) {
                    *p = !*p;
                }
                self.material_library_preview();
            }
            Event::PickAll(on) => {
                if let Some(w) = self.materials.as_mut() {
                    w.picked.iter_mut().for_each(|p| *p = on);
                }
                self.material_library_preview();
            }
            Event::Conflict(c) => {
                if let Some(w) = self.materials.as_mut() {
                    w.conflict = c;
                }
                self.material_library_preview();
            }
            Event::Take => {
                let Some(op) = self
                    .materials
                    .as_ref()
                    .and_then(|w| w.entries.get(w.entry).and_then(|e| import_of(e, &w.picked, w.conflict)))
                else {
                    return Task::none();
                };
                let keep = self.materials_chosen().map(|m| m.id);
                if let Some(done) = self.materials_command(op) {
                    self.say(Level::Success, format!("Kitaplıktan: {}.", report_words(&done.report)));
                }
                self.materials_read(keep);
            }
            Event::AddFolder => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Malzeme kitaplığı klasörü ekle")
                            .pick_folder()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |path| msg(Event::FolderChosen(path)),
                );
            }
            Event::FolderChosen(Some(path)) => {
                if self.material_libraries.add(&path) {
                    self.say(Level::Success, format!("Malzemeler: “{}” klasörü kitaplıklara eklendi.", path.display()));
                }
                self.material_library_read();
            }
            Event::RemoveFolder => {
                let folder = self.materials.as_ref().and_then(|w| match w.entries.get(w.entry).map(|e| &e.place) {
                    Some(Place::File(p)) => p.parent().map(Path::to_path_buf),
                    _ => None,
                });
                if let Some(folder) = folder {
                    self.material_libraries.remove(&folder);
                    self.say(
                        Level::Info,
                        format!("Malzemeler: “{}” klasörü listeden çıkarıldı; dosyalarına dokunulmadı.", folder.display()),
                    );
                    if let Some(w) = self.materials.as_mut() {
                        w.entry = 0;
                    }
                    self.material_library_read();
                }
            }
            Event::Export => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Malzemeleri kitaplığa yaz")
                            .add_filter("KentOS malzeme kitaplığı", &[EXTENSION])
                            .set_file_name(format!("malzemeler.{EXTENSION}"))
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |path| msg(Event::Exported(path)),
                );
            }
            Event::Exported(Some(path)) => self.export_materials(&path),
            Event::FolderChosen(None) | Event::Exported(None) => {}
            _ => {}
        }
        Task::none()
    }

    /// Kitaplığa yaz…: the project's materials as a `.kmalzeme` file named after it.
    pub(crate) fn export_materials(&mut self, path: &Path) {
        let library = path
            .file_stem()
            .map_or_else(|| "Ofis malzemeleri".to_owned(), |s| s.to_string_lossy().into_owned());
        let Some(file) = self
            .materials_command(MaterialsOperation::Export { library, materials: Vec::new() })
            .and_then(|d| d.library)
        else {
            return;
        };
        let text = serde_json::to_string_pretty(&file).unwrap_or_default();
        match std::fs::write(path, text + "\n") {
            Ok(()) => {
                self.say(Level::Success, format!("{} malzeme kitaplığa yazıldı.", file.materials.len()));
                self.material_library_read();
            }
            Err(e) => self.warn(format!("Kitaplık yazılamadı ({e}); başka bir klasör seçip yeniden deneyin.")),
        }
    }

    pub(crate) fn material_library_view<'a>(
        &'a self,
        w: &'a crate::materials_window::Window,
        tabs: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let project = self.document.as_ref().map(|d| d.model.materials()).unwrap_or(&[]);
        let mut list = Column::new().spacing(4);
        let mut group: Option<String> = None;
        for (i, e) in w.entries.iter().enumerate() {
            let here = match &e.place {
                Place::Kentos(_) => "KentOS".to_owned(),
                Place::File(p) => p.parent().map_or_else(String::new, |f| f.display().to_string()),
            };
            if group.as_deref() != Some(here.as_str()) {
                list = list.push(container(label::caption(here.clone())).padding([4, 8]));
                group = Some(here);
            }
            let facts = match &e.read {
                Ok(f) => format!("{} malzeme", f.materials.len()),
                Err(why) => why.clone(),
            };
            let glyph = if e.read.is_ok() { "materials" } else { "warning" };
            let text = Column::new().spacing(2).push(label::body(e.name.clone())).push(label::caption(facts));
            let entry = row![icon(crate::icons::from_web(Some(glyph))).size(18.0), container(text).width(Fill)]
                .spacing(8)
                .align_y(Center);
            list = list.push(
                button(entry)
                    .on_press(msg(Event::Entry(i)))
                    .padding([6, 8])
                    .width(Fill)
                    .style(style::button::row(i == w.entry)),
            );
        }
        if w.entries.len() == 1 {
            list = list.push(label::caption(
                "Ofisin kitaplıkları: .kmalzeme dosyalarının klasörünü Klasör ekle… ile ekleyin; Kitaplığa yaz… projenin malzemelerini böyle bir dosyaya yazar.",
            ));
        }
        let left = container(iced::widget::scrollable(list).height(Fill)).width(280.0);
        let right: Element<'a, Message> = match w.entries.get(w.entry) {
            None => label::caption("").into(),
            Some(e) => match &e.read {
                Err(why) => words::summary(vec![words::text_line(Kind::Warn, why.clone())]),
                Ok(f) => {
                    let mut rows = Column::new().spacing(2);
                    for (i, m) in f.materials.iter().enumerate() {
                        let st = status(project, m);
                        let hatch = m.hatch.as_ref().map_or_else(|| "taramasız".to_owned(), |h| h.pattern.clone());
                        let facts = format!("{} · {} · {hatch} · {}", m.category.label(), m.quantity.label(), st.words());
                        let ticked = w.picked.get(i).copied().unwrap_or(false);
                        rows = rows.push(
                            row![
                                words::check(ticked, "", Some(msg(Event::Pick(i)))),
                                Column::new().spacing(1).push(label::body(m.name.clone())).push(label::caption(facts)),
                            ]
                            .spacing(8)
                            .align_y(Center),
                        );
                    }
                    let mut summary = vec![words::text_line(
                        Kind::Info,
                        format!("{} · {} malzeme · aynı kimlikli malzeme için seçim aşağıda.", f.library, f.materials.len()),
                    )];
                    match &w.take_preview {
                        Some(Ok(done)) => summary.push(words::text_line(
                            if done.report.locked.is_empty() { Kind::Ok } else { Kind::Warn },
                            format!("Seçilenleri al: {}.", report_words(&done.report)),
                        )),
                        Some(Err(why)) => summary.push(words::text_line(Kind::Warn, why.clone())),
                        None => summary.push(words::text_line(Kind::Info, "Almak için malzemeleri işaretleyin.")),
                    }
                    let conflict = kentos_ui::widget::Segmented::new(
                        [Conflict::Keep, Conflict::Replace],
                        w.conflict,
                        |c| msg(Event::Conflict(c)),
                    );
                    let all = row![
                        words::secondary("Hepsini işaretle", Some(msg(Event::PickAll(true)))),
                        words::secondary("Hiçbirini", Some(msg(Event::PickAll(false)))),
                    ]
                    .spacing(8);
                    Column::new()
                        .spacing(10)
                        .push(words::summary(summary))
                        .push(words::field("Projede aynı kimlikli malzeme varsa", conflict, None))
                        .push(all)
                        .push(iced::widget::scrollable(container(rows).padding(iced::Padding { right: 16.0, ..iced::Padding::ZERO })).height(Fill))
                        .into()
                }
            },
        };
        let body = row![left, container(right).width(Fill)].spacing(16).height(440.0);
        let in_folder = matches!(w.entries.get(w.entry).map(|e| &e.place), Some(Place::File(_)));
        let can_take = matches!(w.take_preview, Some(Ok(_)));
        Frame::new(TITLE)
            .push(tabs)
            .push(body)
            .action(words::secondary(ADD_FOLDER, Some(msg(Event::AddFolder))))
            .action(words::secondary(REMOVE_FOLDER, in_folder.then(|| msg(Event::RemoveFolder))))
            .action(words::secondary(EXPORT, (!project.is_empty()).then(|| msg(Event::Export))))
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(TAKE, can_take.then(|| msg(Event::Take))))
            .width(920.0)
            .modal(msg(Event::Close))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, name: &str) -> ArchMaterial {
        ArchMaterial {
            id: id.into(),
            name: name.into(),
            category: Default::default(),
            quantity: Default::default(),
            hatch: None,
            cut: None,
        }
    }

    #[test]
    fn a_library_material_is_said_against_the_project() {
        let project = vec![m("tugla", "Tuğla"), m("beton", "Betonarme")];
        assert_eq!(status(&project, &m("tugla", "Tuğla")), Status::Same);
        assert_eq!(status(&project, &m("tugla", "Tuğla duvar")), Status::Differs);
        assert_eq!(status(&project, &m("betonarme", "BETONARME")), Status::NameTaken);
        assert_eq!(status(&project, &m("eps", "EPS")), Status::New);
    }

    #[test]
    fn folders_are_kept_and_their_files_listed_after_kentos() {
        let dir = std::env::temp_dir().join(format!("kentos-malzeme-{}", std::process::id()));
        let lib = dir.join("ofis");
        std::fs::create_dir_all(&lib).unwrap();
        let file = MaterialsFile {
            format: kentos_commands::MATERIALS_FORMAT.into(),
            version: kentos_commands::MATERIALS_FORMAT_VERSION,
            library: "Ofis".into(),
            materials: vec![m("eps", "EPS")],
        };
        std::fs::write(lib.join("ofis.kmalzeme"), serde_json::to_string(&file).unwrap()).unwrap();
        std::fs::write(lib.join("bozuk.kmalzeme"), "{").unwrap();
        std::fs::write(lib.join("not.json"), "{}").unwrap();
        let mut libs = Libraries::open(&dir);
        assert!(libs.add(&lib));
        assert!(!libs.add(&lib));
        let again = Libraries::open(&dir);
        assert_eq!(again.folders, vec![lib.clone()]);
        let entries = again.entries();
        assert_eq!(entries[0].place, Place::Kentos("kentos-temel"));
        assert_eq!(entries[0].read.as_ref().unwrap().materials.len(), 21);
        let names: Vec<&str> = entries[1..].iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["bozuk", "Ofis"]);
        assert!(entries[1].read.is_err());
        libs.remove(&lib);
        assert!(Libraries::open(&dir).folders.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
