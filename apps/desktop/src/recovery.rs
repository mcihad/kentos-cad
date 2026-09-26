//! Local recovery copies of unsaved work on the desktop (TODOS.md FILE-19,
//! docs/adr/0030): while the drawing has changes no save wrote, a copy of it
//! is kept in the app's data folder, apart from any `.kcad` file, so a crash
//! or a power cut does not lose the work.
//!
//! - The folder is `$XDG_DATA_HOME/kentos-cad/kurtarma` (else
//!   `~/.local/share/kentos-cad/kurtarma`); each running KentOS has a folder of
//!   its own there and holds a lock on its `kilit` file while it runs. `App::boot`
//!   (tests, snapshots, the trace player) keeps no copies at all.
//! - A copy is the drawing's verified KCAD v2 bytes (`<n>.kurtarma`) and a few
//!   facts about it (`<n>.json`), each written to a temporary file and renamed,
//!   on a thread of its own: a few seconds after the drawing changes and at
//!   most every half minute while it keeps changing; never while a save or an
//!   open runs.
//! - A save that leaves the drawing clean removes its copy; so does dropping
//!   the changes on purpose (Kaydetmeden aç / çık). A drawing replaced keeps
//!   its copy only if its changes were not dropped on purpose.
//! - When KentOS starts, the copies of folders whose lock nobody holds (a
//!   KentOS that crashed or was killed) are offered one by one: Geri yükle
//!   opens the copy as unsaved work without a file (Save asks where; no file is
//!   written over by itself), Sonra keeps it for the next start, Sil deletes it.

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt as _, Stream};
use iced::widget::{button, row, space};
use iced::{Element, Task};
use kentos_ui::widget::{Dialog, overlay};
use kentos_ui::{label, style};
use serde::{Deserialize, Serialize};

use crate::app::{App, Dialog as Asking, Message};
use crate::opening::Purpose;

/// A copy is written this long after the last change…
const QUIET: Duration = Duration::from_secs(3);
/// …and at most this long after the first change it does not hold.
const LONGEST: Duration = Duration::from_secs(30);
/// How often the app looks whether a copy is due, while the drawing is unsaved.
const TICK: Duration = Duration::from_secs(2);

/// What a copy says about itself (`<n>.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Facts {
    version: u32,
    name: String,
    file: Option<String>,
    saved_at: u64,
    objects: usize,
}

/// A copy another KentOS left, to offer.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    pub name: String,
    /// The file the drawing was opened from or saved to, if any.
    pub file: Option<String>,
    /// When it was written (seconds since the epoch).
    pub saved_at: u64,
    pub objects: usize,
    /// The drawing: KCAD v2 bytes.
    pub data: PathBuf,
    facts: PathBuf,
    /// The folder of the KentOS that left it (removed once empty).
    folder: PathBuf,
}

impl Offer {
    /// Its key while it is offered.
    pub fn id(&self) -> String {
        self.data.display().to_string()
    }

    fn remove(&self) {
        let _ = std::fs::remove_file(&self.data);
        let _ = std::fs::remove_file(&self.facts);
        remove_if_left(&self.folder);
    }
}

/// Removes a gone KentOS's folder once only its lock file is left.
fn remove_if_left(folder: &Path) {
    let rest = std::fs::read_dir(folder)
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| e.file_name() != "kilit")
                .count()
        })
        .unwrap_or(1);
    if rest == 0 {
        let _ = std::fs::remove_dir_all(folder);
    }
}

/// This KentOS's recovery copies.
#[derive(Default)]
pub struct Recovery {
    /// This KentOS's folder; none: no copies are kept.
    folder: Option<PathBuf>,
    /// Held while this KentOS runs: whoever can take it knows this one is gone.
    lock: Option<File>,
    /// The drawing on screen: which opened drawing it is (its session), and its copy's number.
    session: Option<u64>,
    drawing: u64,
    /// The revision its copy holds, if it has one.
    written: Option<u64>,
    /// The revision seen last, when it changed, and the first change the copy lacks.
    seen: Option<u64>,
    changed: Option<Instant>,
    first_unwritten: Option<Instant>,
    writing: bool,
    failed: bool,
    /// Copies gone KentOS processes left, newest first, still to be offered.
    pub offers: Vec<Offer>,
}

/// What the copies tell the app.
#[derive(Debug, Clone)]
pub enum Event {
    /// Time to look whether a copy is due.
    Tick,
    /// A copy was written (or could not be): of which drawing, which revision.
    Written {
        session: u64,
        drawing: u64,
        revision: u64,
        result: Result<(), String>,
    },
    /// The answer about the copy offered first.
    Answer(Answer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Restore,
    Later,
    Delete,
}

static DRAWINGS: AtomicU64 = AtomicU64::new(0);
fn next_drawing() -> u64 {
    DRAWINGS.fetch_add(1, Ordering::Relaxed) + 1
}

/// The user's data folder for the copies: `$XDG_DATA_HOME/kentos-cad/kurtarma`, else `~/.local/share/…`.
pub fn default_root() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(base.join("kentos-cad").join("kurtarma"))
}

impl Recovery {
    /// No copies (tests, snapshots, the trace player, a machine without a data folder).
    pub fn off() -> Self {
        Self::default()
    }

    /// Copies in `root`: a folder of this KentOS's own, locked while it runs,
    /// and the copies of KentOS processes that are gone, to offer. A root that
    /// cannot be used keeps no copies (said by the caller).
    pub fn open(root: &Path) -> Result<Self, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let folder = root.join(format!("{}-{now}", std::process::id()));
        std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(folder.join("kilit"))
            .map_err(|e| format!("{}: {e}", folder.display()))?;
        lock.try_lock()
            .map_err(|e| format!("{}: kilitlenemedi ({e})", folder.display()))?;
        let offers = gone_copies(root, &folder);
        Ok(Self {
            folder: Some(folder),
            lock: Some(lock),
            offers,
            ..Self::default()
        })
    }

    /// Whether copies are kept.
    pub fn on(&self) -> bool {
        self.folder.is_some()
    }

    fn paths(&self, drawing: u64) -> Option<(PathBuf, PathBuf)> {
        let folder = self.folder.as_ref()?;
        Some((
            folder.join(format!("{drawing}.kurtarma")),
            folder.join(format!("{drawing}.json")),
        ))
    }

    /// The copy of the drawing on screen goes (saved, or its changes dropped on purpose).
    fn remove_current(&mut self) {
        if let Some((data, facts)) = self.paths(self.drawing) {
            let _ = std::fs::remove_file(data);
            let _ = std::fs::remove_file(facts);
        }
        self.written = None;
        self.first_unwritten = None;
    }

    /// The user dropped the drawing's unsaved changes on purpose: its copy goes too.
    pub fn discard(&mut self) {
        self.remove_current();
        self.drawing = next_drawing();
    }

    /// KentOS closes normally: this KentOS's folder goes if no copy is left in it.
    pub fn finish(&mut self) {
        if let Some(folder) = &self.folder {
            remove_if_left(folder);
        }
        self.lock = None;
    }

    /// The copies files this KentOS holds now (tests).
    #[cfg(test)]
    pub fn files(&self) -> Vec<PathBuf> {
        let Some(folder) = &self.folder else {
            return Vec::new();
        };
        let mut out: Vec<PathBuf> = std::fs::read_dir(folder)
            .map(|d| d.filter_map(Result::ok).map(|e| e.path()).collect())
            .unwrap_or_default();
        out.retain(|p| p.file_name().is_some_and(|n| n != "kilit"));
        out.sort();
        out
    }
}

/// The copies in `root` whose KentOS is gone (its lock is free), newest first.
fn gone_copies(root: &Path, own: &Path) -> Vec<Offer> {
    let mut copies = Vec::new();
    let Ok(folders) = std::fs::read_dir(root) else {
        return copies;
    };
    for folder in folders.filter_map(Result::ok).map(|e| e.path()) {
        if folder == own || !folder.is_dir() {
            continue;
        }
        // A KentOS that runs holds its lock: its copies are its own.
        let alive = OpenOptions::new()
            .write(true)
            .open(folder.join("kilit"))
            .map(|f| f.try_lock().is_err())
            .unwrap_or(false);
        if alive {
            continue;
        }
        let Ok(files) = std::fs::read_dir(&folder) else {
            continue;
        };
        for facts in files
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
        {
            let data = facts.with_extension("kurtarma");
            let read = std::fs::read(&facts)
                .ok()
                .and_then(|b| serde_json::from_slice::<Facts>(&b).ok())
                .filter(|f| f.version == 1);
            // A copy whose facts or bytes are missing or unreadable is left where it is, not offered.
            if let Some(f) = read.filter(|_| data.is_file()) {
                copies.push(Offer {
                    name: f.name,
                    file: f.file,
                    saved_at: f.saved_at,
                    objects: f.objects,
                    data,
                    facts,
                    folder: folder.clone(),
                });
            }
        }
    }
    copies.sort_by_key(|c| std::cmp::Reverse(c.saved_at));
    copies
}

/// Writes `bytes` to `path` through a temporary file beside it, flushed and renamed.
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = path.with_extension("yaziliyor");
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temporary)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Every `TICK`, while the subscription lives (the drawing is unsaved).
pub fn ticks() -> impl Stream<Item = Message> {
    let (mut out, ticks) = mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(TICK);
            if iced::futures::executor::block_on(out.send(Message::Recovery(Event::Tick))).is_err()
            {
                break;
            }
        }
    });
    ticks
}

impl App {
    /// The copies' messages.
    pub(crate) fn recovery_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Tick => return self.recovery_due(Instant::now()),
            Event::Written {
                session,
                drawing,
                revision,
                result,
            } => {
                let r = &mut self.recovery;
                r.writing = false;
                match result {
                    Ok(()) if r.session == Some(session) && r.drawing == drawing => {
                        r.written = Some(revision);
                        r.failed = false;
                        if self
                            .document
                            .as_ref()
                            .is_some_and(|d| d.model.revision() == revision)
                        {
                            r.first_unwritten = None;
                        }
                        // Saved, or its changes dropped, while the copy was being written: it goes.
                        if !self
                            .document
                            .as_ref()
                            .is_some_and(crate::document::Document::dirty)
                        {
                            r.remove_current();
                        }
                    }
                    Ok(()) => {
                        // A copy of a drawing no longer on screen: kept, unless its changes were dropped.
                    }
                    Err(e) => {
                        if !r.failed {
                            r.failed = true;
                            self.warn(format!(
                                "Kaydedilmemiş çalışmanın kurtarma kopyası yazılamadı ({e}). Çizim etkilenmedi; çalışmanızı kaydetmeyi unutmayın."
                            ));
                        }
                    }
                }
            }
            Event::Answer(answer) => {
                self.dialog = None;
                let Some(copy) = self.recovery.offers.first().cloned() else {
                    return Task::none();
                };
                match answer {
                    // Removed once it is on screen (`recovered`); if it cannot be read it stays for next time.
                    Answer::Restore => {
                        return self.start_opening(
                            copy.data.clone(),
                            Purpose::Recovery {
                                id: copy.id(),
                                name: copy.name.clone(),
                            },
                        );
                    }
                    Answer::Delete => {
                        self.recovery.offers.remove(0);
                        copy.remove();
                        self.output(format!(
                            "“{}” çiziminin kurtarma kopyası silindi.",
                            copy.name
                        ));
                    }
                    Answer::Later => {
                        self.recovery.offers.remove(0);
                    }
                }
                if !self.recovery.offers.is_empty() {
                    self.dialog = Some(Asking::Recovery);
                }
            }
        }
        Task::none()
    }

    /// A recovery copy is on screen (opening.rs): it is removed where it was;
    /// the drawing, unsaved, gets its own copy at the next tick.
    pub(crate) fn recovered(&mut self, id: &str, name: &str) {
        if let Some(at) = self.recovery.offers.iter().position(|c| c.id() == id) {
            self.recovery.offers.remove(at).remove();
        }
        self.output(format!(
            "“{name}” kaydedilmemiş çalışması geri yüklendi. Çizim kaydedilmemiş sayılıyor; Kaydet dosyanın yerini sorar, hiçbir dosyanın üzerine kendiliğinden yazılmaz."
        ));
        if !self.recovery.offers.is_empty() {
            self.dialog = Some(Asking::Recovery);
        }
    }

    /// The drawing was saved: if nothing changed meanwhile its copy goes.
    pub(crate) fn recovery_saved(&mut self) {
        if !self
            .document
            .as_ref()
            .is_some_and(crate::document::Document::dirty)
        {
            self.recovery.remove_current();
        }
    }

    /// Writes the drawing's copy when one is due (see the module): the drawing
    /// is copied cheaply now (its objects are shared), encoded and written on
    /// a thread of its own.
    pub(crate) fn recovery_due(&mut self, now: Instant) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let r = &mut self.recovery;
        if !r.on() {
            return Task::none();
        }
        // Another drawing on screen: its copies take a number of their own; the old copy stays.
        if r.session != Some(doc.session) {
            r.session = Some(doc.session);
            r.drawing = next_drawing();
            r.written = None;
            r.seen = None;
            r.first_unwritten = None;
        }
        let revision = doc.model.revision();
        if !doc.dirty() || r.written == Some(revision) {
            return Task::none();
        }
        if r.seen != Some(revision) {
            r.seen = Some(revision);
            r.changed = Some(now);
            r.first_unwritten.get_or_insert(now);
        }
        let quiet = r.changed.is_some_and(|c| now.duration_since(c) >= QUIET);
        let overdue = r
            .first_unwritten
            .is_some_and(|f| now.duration_since(f) >= LONGEST);
        if r.writing || !(quiet || overdue) || self.saving.is_some() || self.opening.is_some() {
            return Task::none();
        }
        let Some((data, facts_path)) = r.paths(r.drawing) else {
            return Task::none();
        };
        r.writing = true;
        let (session, drawing) = (doc.session, r.drawing);
        let model = doc.model.clone();
        let facts = Facts {
            version: 1,
            name: doc.name().to_owned(),
            file: doc.path.as_ref().map(|p| p.display().to_string()),
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            objects: doc.entity_count(),
        };
        iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
            let result = (|| -> Result<(), String> {
                let bytes =
                    kentos_kcad::encode_verified(&model.to_snapshot_v2()).map_err(|e| e.message)?;
                drop(model);
                write_atomically(&data, &bytes).map_err(|e| e.to_string())?;
                let text = serde_json::to_vec_pretty(&facts).map_err(|e| e.to_string())?;
                write_atomically(&facts_path, &text).map_err(|e| e.to_string())
            })();
            let _ =
                iced::futures::executor::block_on(out.send(Message::Recovery(Event::Written {
                    session,
                    drawing,
                    revision,
                    result,
                })));
        })
    }

    /// The question about the copy offered first.
    pub(crate) fn recovery_dialog(&self) -> Element<'_, Message> {
        let Some(copy) = self.recovery.offers.first() else {
            return space::horizontal().into();
        };
        let ago = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .saturating_sub(copy.saved_at);
        let when = match ago {
            0..60 => "az önce".to_owned(),
            60..3600 => format!("{} dakika önce", ago / 60),
            3600..86400 => format!("{} saat önce", ago / 3600),
            _ => format!("{} gün önce", ago / 86400),
        };
        let file = copy
            .file
            .as_ref()
            .map_or_else(String::new, |f| format!(", dosyası {f}"));
        let answer = |text: &'static str,
                      a: Answer,
                      s: fn(&iced::Theme, button::Status) -> button::Style| {
            button(label::body(text))
                .on_press(Message::Recovery(Event::Answer(a)))
                .padding([5, 16])
                .style(s)
        };
        let actions = row![
            answer("Sil", Answer::Delete, style::button::danger),
            space::horizontal(),
            answer("Sonra", Answer::Later, style::button::secondary),
            answer("Geri yükle", Answer::Restore, style::button::primary),
        ]
        .spacing(6);
        overlay::modal(
            Dialog::new("Kaydedilmemiş çalışma bulundu")
                .push(label::body(format!(
                    "“{}” çiziminin kaydedilmemiş bir kopyası var: {when}, {} nesne{file}. KentOS beklenmedik biçimde kapanmış olabilir.",
                    copy.name, copy.objects
                )))
                .push(label::muted(
                    "Geri yükle: kopya açılır ve kaydedilmemiş sayılır; Kaydet dosyanın yerini sorar, hiçbir dosyanın üzerine kendiliğinden yazılmaz. Sonra: kopya bu bilgisayarda kalır, KentOS bir sonraki açılışta yeniden sorar. Sil: kopya kalıcı olarak silinir.",
                ))
                .action(actions)
                .width(520.0),
            Message::Recovery(Event::Answer(Answer::Later)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Picker, Then};
    use crate::files_testing::{app_with_drawing, drive, last_said, scratch};
    use crate::settings::Settings;

    /// The app with the sample drawing, keeping copies in `root`, the drawing changed.
    fn editing(root: &Path) -> App {
        let mut app = app_with_drawing();
        app.recovery = Recovery::open(root).expect("a recovery folder");
        let layer = app.document.as_ref().expect("a drawing").layers()[0]
            .id
            .clone();
        let _ = app.update(Message::LayerLocked(layer));
        app
    }

    /// Writes the drawing's copy as the ticks would: once it has rested.
    fn write_copy(app: &mut App) {
        let now = Instant::now();
        let first = app.recovery_due(now);
        drive(app, first);
        let task = app.recovery_due(now + QUIET + Duration::from_millis(1));
        drive(app, task);
    }

    #[test]
    fn unsaved_work_gets_a_copy_that_a_save_removes() {
        let root = scratch("recovery-save");
        let mut app = editing(&root);
        assert!(app.recovery.files().is_empty());
        write_copy(&mut app);
        let files = app.recovery.files();
        assert_eq!(files.len(), 2, "{files:?}");
        let data = files
            .iter()
            .find(|p| p.extension().is_some_and(|e| e == "kurtarma"))
            .expect("the copy");
        let copy = kentos_kcad::decode(&std::fs::read(data).expect("written")).expect("reads");
        assert_eq!(copy.entities.len(), 13);
        // Written once per change: nothing new, nothing written.
        assert!(app.recovery.written.is_some());

        let file = root.join("pafta.kcad");
        app.picker = Picker::File(file);
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        assert!(!app.document.as_ref().expect("a drawing").dirty());
        assert!(app.recovery.files().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_gone_kentos_copy_is_offered_and_restored_as_unsaved_work_without_a_file() {
        let root = scratch("recovery-offer");
        let mut crashed = editing(&root);
        write_copy(&mut crashed);
        // While that KentOS runs, another one leaves its copies alone.
        let other = Recovery::open(&root).expect("folder");
        assert!(other.offers.is_empty());
        // It is gone: its lock is free.
        crashed.recovery.lock = None;
        let next = Recovery::open(&root).expect("folder");
        assert_eq!(next.offers.len(), 1);
        assert_eq!(
            (next.offers[0].name.as_str(), next.offers[0].objects),
            ("Örnek pafta.kcad", 13)
        );
        let (mut app, _) = App::start_with(None, Settings::memory(), next);
        assert_eq!(app.dialog, Some(Asking::Recovery));
        let task = app.update(Message::Recovery(Event::Answer(Answer::Restore)));
        drive(&mut app, task);
        let doc = app.document.as_ref().expect("the copy on screen");
        assert_eq!(doc.entity_count(), 13);
        assert!(doc.dirty(), "restored work is unsaved");
        assert_eq!(doc.path, None, "and has no file: Save asks where");
        assert!(
            last_said(&app).contains("geri yüklendi"),
            "{}",
            last_said(&app)
        );
        assert!(app.recovery.offers.is_empty());
        // The gone KentOS's folder went with its copy.
        assert_eq!(
            std::fs::read_dir(&root).expect("lists").count(),
            2,
            "this KentOS's folder and the other running one's"
        );
        drop(other);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sonra_keeps_a_copy_for_next_time_and_sil_deletes_it() {
        let root = scratch("recovery-answers");
        let mut crashed = editing(&root);
        write_copy(&mut crashed);
        crashed.recovery.lock = None;
        let (mut first, _) = App::start_with(
            None,
            Settings::memory(),
            Recovery::open(&root).expect("folder"),
        );
        let _ = first.update(Message::Recovery(Event::Answer(Answer::Later)));
        assert!(first.dialog.is_none());
        let (mut second, _) = App::start_with(
            None,
            Settings::memory(),
            Recovery::open(&root).expect("folder"),
        );
        assert_eq!(second.recovery.offers.len(), 1, "offered again");
        let _ = second.update(Message::Recovery(Event::Answer(Answer::Delete)));
        assert!(last_said(&second).contains("kurtarma kopyası silindi"));
        let third = Recovery::open(&root).expect("folder");
        assert!(third.offers.is_empty());
        drop((first, second, third));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dropping_the_changes_on_purpose_removes_the_copy() {
        let root = scratch("recovery-drop");
        let mut app = editing(&root);
        write_copy(&mut app);
        assert_eq!(app.recovery.files().len(), 2);
        app.dialog = Some(Asking::Unsaved(Then::Open));
        // Kaydetmeden aç: the file dialog that follows is not answered here.
        let _ = app.update(Message::DialogConfirmed);
        assert!(app.recovery.files().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_app_in_tests_and_snapshots_keeps_no_copies() {
        let mut app = app_with_drawing();
        let layer = app.document.as_ref().expect("a drawing").layers()[0]
            .id
            .clone();
        let _ = app.update(Message::LayerLocked(layer));
        assert!(!app.recovery.on());
        let now = Instant::now();
        let _ = app.recovery_due(now);
        let task = app.recovery_due(now + LONGEST);
        assert_eq!(task.units(), 0);
    }
}
