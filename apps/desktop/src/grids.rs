//! The device's NTv2 grid library (docs/adr/0168 §4; the web's
//! `app/gridLibrary.ts`): grids kept in `$XDG_DATA_HOME/kentos-cad/izgara/`
//! (else `~/.local/share/…`) under their SHA-256, `<id>.gsb`, the name of the
//! file each came from beside it in `<id>.json`, as QGIS and ArcGIS keep
//! theirs in the user's folder. A project names its grid choice's file by
//! its SHA-256 and size (`GridChoice`). The grids the open project names are
//! read into the core (`ntv2::register`) on a thread of their own when it
//! opens and when its choices change; one this device does not have is said
//! once, with the file's name, and the transforms give no value by it
//! (`noGrid`) until it is added.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use iced::Task;
use iced::futures::channel::oneshot;
use kentos_geometry_core::crs::ntv2::{self, GridError};
use kentos_interaction::Level;

use crate::app::{App, Message};

/// A grid of the library: what its header says is kept beside it when it
/// is added, so the list needs no grid read whole.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// Its SHA-256, 64 lower-case hex digits: its name in the library.
    pub id: String,
    /// The name of the file it was added from.
    pub file: String,
    pub size: u64,
    /// The datums it shifts from and to, as its header names them.
    pub from: String,
    pub to: String,
    pub subgrids: usize,
    /// West, south, east, north (degrees).
    pub extent: [f64; 4],
}

/// A grid's line under its name (the web's `gridLine`): the datums, the
/// extent, the subgrids when more than one, the size, the SHA-256's start.
pub fn grid_line(e: &Entry) -> String {
    use kentos_interaction::fixed;
    let [w, s, east, n] = e.extent;
    let mut parts = vec![
        format!("{} → {}", e.from, e.to),
        format!(
            "{}°–{}° D, {}°–{}° K",
            fixed(w, 1),
            fixed(east, 1),
            fixed(s, 1),
            fixed(n, 1)
        ),
    ];
    if e.subgrids > 1 {
        parts.push(format!("{} alt ızgara", e.subgrids));
    }
    parts.push(if e.size < 1024 * 1024 {
        format!("{} KB", fixed(e.size as f64 / 1024.0, 0))
    } else {
        format!("{} MB", fixed(e.size as f64 / (1024.0 * 1024.0), 1))
    });
    parts.push(e.id.chars().take(12).collect());
    parts.join(" · ")
}

/// Why a grid the project names is not read into the core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missing {
    /// This device's library does not have it.
    NotHere,
    /// The library's file is no longer the one its name says (its SHA-256 differs).
    Changed,
    /// The library's file is not read as NTv2.
    Unread(GridError),
}

/// Why a file is not read as NTv2, in the user's words (the web's `gridErrorText`).
pub fn grid_error_text(e: GridError) -> &'static str {
    match e {
        GridError::TooLarge => "256 MiB'tan büyük",
        GridError::NotNtv2 => "NTv2 dosyası değil",
        GridError::NotSeconds => "kaymaları saniye cinsinden değil",
        GridError::Header => "başlığı bozuk",
        GridError::Extent => "kapsamı ya da aralığı geçersiz",
        GridError::Count => "kayıt sayısı kapsamıyla tutmuyor",
        GridError::Truncated => "dosya kayıtları bitmeden kesiliyor",
        GridError::Values => "sonlu olmayan kayma değerleri var",
    }
}

/// SHA-256 of `bytes` as 64 lower-case hex digits (the sheet core's, the same hash).
fn sha256(bytes: &[u8]) -> String {
    kentos_sheet::template::sha256_hex(bytes)
}

/// The library's folder.
#[derive(Clone, Debug)]
pub struct Library {
    dir: PathBuf,
}

impl Library {
    /// The device's: `$XDG_DATA_HOME/kentos-cad/izgara`, else `~/.local/share/kentos-cad/izgara`.
    pub fn device() -> Option<Self> {
        let base = std::env::var_os("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })?;
        Some(Self::at(base.join("kentos-cad").join("izgara")))
    }

    pub fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn grid(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.gsb"))
    }

    fn note(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// Adds the file at `path`: read whole, checked as NTv2, kept under its
    /// SHA-256 with its name (written next to it first, then moved in), and
    /// read into the core. The same grid added again keeps its place.
    pub fn add(&self, path: &Path) -> Result<Entry, String> {
        let file = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let size = std::fs::metadata(path)
            .map_err(|e| format!("“{file}” okunamadı: {e}."))?
            .len();
        if size > ntv2::MAX_BYTES as u64 {
            return Err(format!(
                "“{file}” eklenmedi: {}.",
                grid_error_text(GridError::TooLarge)
            ));
        }
        let bytes = std::fs::read(path).map_err(|e| format!("“{file}” okunamadı: {e}."))?;
        let grid = ntv2::read(&bytes).map_err(|e| {
            format!(
                "“{file}” NTv2 ızgarası olarak okunamadı: {}.",
                grid_error_text(e)
            )
        })?;
        let id = sha256(&bytes);
        let (subgrids, extent) = grid.summary();
        let entry = Entry {
            id: id.clone(),
            file: file.clone(),
            size: bytes.len() as u64,
            from: grid.from.clone(),
            to: grid.to.clone(),
            subgrids,
            extent,
        };
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| format!("Izgara klasörü açılamadı ({}): {e}.", self.dir.display()))?;
        let put = |to: &Path, content: &[u8]| -> std::io::Result<()> {
            let partial = to.with_extension("yaziliyor");
            std::fs::write(&partial, content)?;
            std::fs::rename(&partial, to)
        };
        let note = serde_json::json!({
            "file": file,
            "from": entry.from,
            "to": entry.to,
            "subgrids": subgrids,
            "extent": extent,
        });
        put(&self.grid(&id), &bytes)
            .and_then(|()| put(&self.note(&id), note.to_string().as_bytes()))
            .map_err(|e| format!("“{file}” ızgara klasörüne yazılamadı: {e}."))?;
        ntv2::register(&id, grid);
        Ok(entry)
    }

    /// The library's grids, by the names of the files they came from; a
    /// grid without its note is named by its SHA-256.
    pub fn list(&self) -> Vec<Entry> {
        let Ok(dir) = std::fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut out: Vec<Entry> = dir
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                let id = name.strip_suffix(".gsb")?;
                let hex = id.len() == 64
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
                if !hex {
                    return None;
                }
                let note = std::fs::read_to_string(self.note(id))
                    .ok()
                    .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                    .unwrap_or_default();
                let text = |k: &str| note[k].as_str().map(str::to_owned);
                let extent = note["extent"].as_array().map_or([0.0; 4], |a| {
                    std::array::from_fn(|i| {
                        a.get(i).and_then(serde_json::Value::as_f64).unwrap_or(0.0)
                    })
                });
                Some(Entry {
                    id: id.to_owned(),
                    file: text("file").unwrap_or_else(|| name.clone()),
                    size: e.metadata().ok()?.len(),
                    from: text("from").unwrap_or_default(),
                    to: text("to").unwrap_or_default(),
                    subgrids: note["subgrids"].as_u64().unwrap_or(1) as usize,
                    extent,
                })
            })
            .collect();
        out.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.id.cmp(&b.id)));
        out
    }

    /// Takes the grid out of the library and the core.
    pub fn remove(&self, id: &str) -> Result<(), String> {
        ntv2::forget(id);
        let _ = std::fs::remove_file(self.note(id));
        match std::fs::remove_file(self.grid(id)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("Izgara silinemedi: {e}.")),
        }
    }

    /// Reads the grid kept under `id` into the core, unless it is there.
    pub fn load(&self, id: &str) -> Result<(), Missing> {
        if ntv2::get(id).is_some() {
            return Ok(());
        }
        let bytes = std::fs::read(self.grid(id)).map_err(|_| Missing::NotHere)?;
        if sha256(&bytes) != id {
            return Err(Missing::Changed);
        }
        let grid = ntv2::read(&bytes).map_err(Missing::Unread)?;
        ntv2::register(id, grid);
        Ok(())
    }
}

/// What the app keeps of the library: the device's (none in tests and
/// snapshots, which never touch the user's files; `main` gives it), its
/// grids as Izgaralar lists them, and the grids the open project named that
/// were looked for in this run.
#[derive(Debug, Default)]
pub struct Grids {
    pub library: Option<Library>,
    pub entries: Vec<Entry>,
    asked: BTreeSet<String>,
}

/// What Proje ayarları' Izgaralar asks for (docs/adr/0168 §6).
#[derive(Debug, Clone)]
pub enum Event {
    /// Ekle…: the file to add is chosen.
    Add,
    /// The chosen file, read, checked and kept on a thread of its own.
    Picked(Option<PathBuf>),
    /// What adding it gave.
    Added(Result<Entry, String>),
    /// Sil, answered in the row.
    Remove(String),
}

/// A grid looked for: its file's name, and whether it was read into the core.
pub type Looked = (String, Result<(), Missing>);

impl App {
    /// Izgaralar's list read again from the library.
    pub(crate) fn refresh_grids(&mut self) {
        self.grids.entries = self
            .grids
            .library
            .as_ref()
            .map(Library::list)
            .unwrap_or_default();
    }

    pub(crate) fn grid_event(&mut self, e: Event) -> Task<Message> {
        let message =
            |e: Event| crate::project::settings_message(crate::project::SettingsEvent::Grid(e));
        match e {
            Event::Add => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("NTv2 ızgarası ekle")
                            .add_filter("NTv2 ızgarası (.gsb)", &["gsb", "GSB"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    move |path| message(Event::Picked(path)),
                );
            }
            Event::Picked(None) => {}
            Event::Picked(Some(path)) => {
                let Some(library) = self.grids.library.clone() else {
                    self.say(
                        Level::Error,
                        "Izgara eklenemedi: bu cihazda uygulama veri klasörü yok (XDG_DATA_HOME ya da HOME).",
                    );
                    return Task::none();
                };
                let (done, wait) = oneshot::channel();
                std::thread::spawn(move || {
                    let _ = done.send(library.add(&path));
                });
                return Task::perform(wait, move |added| {
                    message(Event::Added(added.unwrap_or_else(|_| {
                        Err("Izgara eklenemedi: okuma yarıda kaldı.".to_owned())
                    })))
                });
            }
            Event::Added(Ok(entry)) => {
                self.refresh_grids();
                self.say(
                    Level::Success,
                    format!(
                        "“{}” ızgara kitaplığına eklendi ({} → {}); projelerin datum seçimleri onu kullanabilir.",
                        entry.file, entry.from, entry.to
                    ),
                );
            }
            Event::Added(Err(why)) => self.say(Level::Error, why),
            Event::Remove(id) => {
                let Some(library) = self.grids.library.clone() else {
                    return Task::none();
                };
                let file = self
                    .grids
                    .entries
                    .iter()
                    .find(|e| e.id == id)
                    .map_or_else(|| id.clone(), |e| e.file.clone());
                match library.remove(&id) {
                    Ok(()) => {
                        // A project that names it may look for it again once it is added again.
                        self.grids.asked.remove(&id);
                        self.refresh_grids();
                        self.say(
                            Level::Success,
                            format!("“{file}” ızgara kitaplığından kaldırıldı."),
                        );
                    }
                    Err(why) => self.say(Level::Error, why),
                }
            }
        }
        Task::none()
    }

    /// After every message: the grids the open project's datum choices name
    /// that were not looked for yet are read into the core on a thread of
    /// their own.
    pub(crate) fn follow_grids(&mut self) -> Task<Message> {
        let (Some(doc), Some(library)) = (&self.document, self.grids.library.clone()) else {
            return Task::none();
        };
        let wanted: Vec<(String, String)> = doc
            .settings()
            .datum_transforms
            .iter()
            .filter_map(|t| t.grid.as_ref())
            .filter(|g| !self.grids.asked.contains(&g.id))
            .map(|g| (g.id.clone(), g.file.clone()))
            .collect();
        if wanted.is_empty() {
            return Task::none();
        }
        self.grids
            .asked
            .extend(wanted.iter().map(|(id, _)| id.clone()));
        let (done, wait) = oneshot::channel();
        std::thread::spawn(move || {
            let looked: Vec<Looked> = wanted
                .into_iter()
                .map(|(id, file)| (file, library.load(&id)))
                .collect();
            let _ = done.send(looked);
        });
        Task::perform(wait, |looked| {
            Message::GridsLooked(looked.unwrap_or_default())
        })
    }

    /// The grids looked for: one this device does not have, or cannot read,
    /// is said with its file's name; the values shown by them follow.
    pub(crate) fn grids_looked(&mut self, looked: Vec<Looked>) {
        for (file, result) in looked {
            let why = match result {
                Ok(()) => continue,
                Err(Missing::NotHere) => format!("NTv2 ızgarası bu cihazda yok: {file}."),
                Err(Missing::Changed) => format!(
                    "NTv2 ızgarası {file} cihazdaki kitaplıkta değişmiş (SHA-256'sı tutmuyor)."
                ),
                Err(Missing::Unread(e)) => {
                    format!("NTv2 ızgarası {file} okunamadı: {}.", grid_error_text(e))
                }
            };
            self.say(
                Level::Warn,
                format!(
                    "{why} Proje ayarları › Koordinat sistemi › Izgaralar'dan ekleyin; o zamana dek datum dönüşümü bu seçimle değer vermez."
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/geodesy/v1/ntv2/tr.gsb"
    );

    /// Added, listed by its file's name, read into the core under its
    /// SHA-256 again after it was let go, refused when its file changed,
    /// taken out; a file that is not NTv2 says why.
    #[test]
    fn a_grid_is_kept_by_its_hash_and_read_into_the_core() {
        let dir = crate::files_testing::scratch("izgara-kitaplik");
        let library = Library::at(dir.clone());
        let entry = library.add(Path::new(TR)).expect("the Türkiye grid");
        assert_eq!(entry.id.len(), 64);
        assert_eq!(entry.file, "tr.gsb");
        assert_eq!(library.list(), vec![entry.clone()]);
        ntv2::forget(&entry.id);
        assert!(ntv2::get(&entry.id).is_none());
        assert_eq!(library.load(&entry.id), Ok(()));
        assert!(ntv2::get(&entry.id).is_some());
        // A file changed in the library is not the grid its name says.
        ntv2::forget(&entry.id);
        let mut bytes = std::fs::read(TR).expect("the grid");
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        std::fs::write(dir.join(format!("{}.gsb", entry.id)), &bytes).expect("writes");
        assert_eq!(library.load(&entry.id), Err(Missing::Changed));
        assert_eq!(library.load(&"0".repeat(64)), Err(Missing::NotHere));
        library.remove(&entry.id).expect("removed");
        assert!(library.list().is_empty());
        // Not NTv2.
        let text = dir.join("not-a-grid.gsb");
        std::fs::write(&text, b"nokta listesi").expect("writes");
        let refused = library.add(&text).expect_err("not a grid");
        assert!(refused.contains("NTv2 dosyası değil"), "{refused}");
    }

    /// A project whose datum choice names a grid this device does not have:
    /// said once with its file's name, no value by it; added from Proje
    /// ayarları' Izgaralar, listed with what its header says, the values
    /// come; removed when asked in its row (docs/adr/0168 §4, §6).
    #[test]
    fn a_projects_grid_is_looked_for_added_and_removed() {
        use crate::files_testing::{app_with_drawing, drive, last_said};
        use crate::project::SettingsEvent;
        use kentos_contracts::{DatumTransform, GridChoice, RegistryDatum};
        use kentos_interaction::second::Second;

        let id = sha256(&std::fs::read(TR).expect("the grid"));
        let mut app = app_with_drawing();
        app.grids.library = Some(Library::at(crate::files_testing::scratch("izgara-proje")));
        // TUREF TM36's project, its second system WGS 84 by the project's grid.
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.second_srid = Some(4326);
        settings.datum_transforms = vec![DatumTransform {
            from: RegistryDatum::Turef,
            to: RegistryDatum::Wgs84,
            name: "TUREF → WGS 84: ızgara".to_owned(),
            helmert: None,
            grid: Some(GridChoice {
                id: id.clone(),
                file: "tr.gsb".to_owned(),
                size: 37_952,
                accuracy: Some(0.05),
            }),
        }];
        doc.model.set_settings(settings);
        ntv2::forget(&id);
        let task = app.update(Message::Run("view.zoomExtents"));
        drive(&mut app, task);
        assert!(
            last_said(&app).starts_with("NTv2 ızgarası bu cihazda yok: tr.gsb."),
            "{}",
            last_said(&app)
        );
        let at = kentos_interaction::Vec2::new(486_512.34, 4_420_187.52);
        let second =
            Second::of(app.document.as_ref().expect("a drawing").settings()).expect("WGS 84");
        assert_eq!(
            second.point(at).map(|_| ()),
            Err(kentos_geometry_core::crs::Unreached::NoGrid)
        );
        // Izgaralar: the project's grid missing, then added.
        let _ = app.update(Message::Run("crs.set"));
        assert!(app.grids.entries.is_empty());
        let send = |app: &mut App, e: SettingsEvent| {
            let task = app.update(crate::project::settings_message(e));
            drive(app, task);
        };
        send(
            &mut app,
            SettingsEvent::Grid(Event::Picked(Some(PathBuf::from(TR)))),
        );
        assert!(last_said(&app).contains("“tr.gsb” ızgara kitaplığına eklendi"));
        assert_eq!(app.grids.entries.len(), 1);
        let line = grid_line(&app.grids.entries[0]);
        assert!(
            line.contains(" KB · ") && line.ends_with(&id[..12]),
            "{line}"
        );
        let t = second.point(at).expect("by the grid");
        assert_eq!(second.accuracy(&t), "±0.05 m, TUREF → WGS 84: ızgara");
        // Kaldır, asked in the row, then Sil.
        send(&mut app, SettingsEvent::GridAsk(Some(id.clone())));
        send(&mut app, SettingsEvent::Grid(Event::Remove(id.clone())));
        assert!(app.grids.entries.is_empty());
        assert!(
            app.log
                .said(Level::Success, "“tr.gsb” ızgara kitaplığından kaldırıldı.")
        );
        // The project still names it: said again.
        assert!(last_said(&app).starts_with("NTv2 ızgarası bu cihazda yok: tr.gsb."));
        assert_eq!(
            second.point(at).map(|_| ()),
            Err(kentos_geometry_core::crs::Unreached::NoGrid)
        );
    }

    /// Proje ayarları' Izgaralar: a grid added, one a datum choice names
    /// that this device does not have, and Kaldır asked in its row; light at
    /// 1440 × 900, dark at 1100 × 650; `.run/shots/izgara-*` (the web's:
    /// `(cd apps/web && node scripts/e2e/shots.mjs grids)`):
    ///
    /// ```text
    /// cargo test -p kentos-desktop grids::tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use crate::files_testing::{app_with_drawing, drive, find_text};
        use crate::project::SettingsEvent;
        use iced::Size;
        use kentos_contracts::{DatumTransform, GridChoice, RegistryDatum};
        use kentos_ui::snapshot::{Input, Snapshot};

        let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        let id = sha256(&std::fs::read(TR).expect("the grid"));
        for (theme, w, h) in [("light", 1440.0, 900.0), ("dark", 1100.0, 650.0)] {
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            app.grids.library = Some(Library::at(crate::files_testing::scratch("izgara-resim")));
            let doc = app.document.as_mut().expect("a drawing");
            let mut settings = doc.settings().clone();
            settings.second_srid = Some(4326);
            let choice = |from, to, name: &str, id: String, file: &str| DatumTransform {
                from,
                to,
                name: name.to_owned(),
                helmert: None,
                grid: Some(GridChoice {
                    id,
                    file: file.to_owned(),
                    size: 1024,
                    accuracy: None,
                }),
            };
            settings.datum_transforms = vec![
                choice(
                    RegistryDatum::Turef,
                    RegistryDatum::Wgs84,
                    "TUREF → WGS 84: ızgara",
                    id.clone(),
                    "tr.gsb",
                ),
                choice(
                    RegistryDatum::Ed50,
                    RegistryDatum::Turef,
                    "ED50 → TUREF: bölge ızgarası",
                    "ab".repeat(32),
                    "ed50-turef-bolge.gsb",
                ),
            ];
            doc.model.set_settings(settings);
            let _ = app.update(Message::Run("crs.set"));
            let task = app.update(crate::project::settings_message(SettingsEvent::Grid(
                Event::Picked(Some(PathBuf::from(TR))),
            )));
            drive(&mut app, task);
            app.follow.flash = None;
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let tag = format!("{theme}-{w}");
            let save = |snapshot: &mut Snapshot, app: &App, name: &str| {
                let file = out.join(format!("izgara-{name}-{tag}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            };
            if let Some(list) = find_text(&mut snapshot, &app, "İkinci koordinat sistemi") {
                snapshot.input(
                    &mut app,
                    App::view,
                    &mut update,
                    Input::Scroll(list.center(), -12.0),
                );
            }
            save(&mut snapshot, &app, "liste");
            let _ = app.update(crate::project::settings_message(SettingsEvent::GridAsk(
                Some(id.clone()),
            )));
            snapshot.settle(&mut app, App::view, &mut update);
            save(&mut snapshot, &app, "kaldir");
        }
    }
}
