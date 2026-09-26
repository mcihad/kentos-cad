//! Saving a drawing on the desktop (TODOS.md FILE-16, FILE-18, docs/adr/0025,
//! 0030): off the UI thread, with its stages shown in a small panel and a way
//! to stop it, and every way it can fail said with what to do.
//!
//! The UI thread only takes a cheap copy of the document (its objects are
//! shared, `Arc`) and the revision; a thread of its own builds the snapshot,
//! encodes it and reads it back (`encode_verified`), writes a new temporary
//! file beside the target in chunks, flushes it to the disk, reads it back,
//! and only then renames it over the target and flushes the directory. A
//! save that fails anywhere, or is stopped before the rename, removes the
//! temporary file and leaves the previous file as it was; the drawing stays
//! unsaved. Only the revision that was written is marked saved.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc;
use iced::widget::container;
use iced::{Bottom, Element, Fill, Right, Task};
use kentos_contracts::DocumentSnapshotV2;
use kentos_kcad::{Code, Step};
use kentos_ui::style;
use kentos_ui::widget::progress::{Task as Job, TaskList};

use crate::app::{App, Message, Written};
use kentos_interaction::Level;

/// Bytes written between two looks at the stop flag and two reports.
const CHUNK: usize = 8 << 20;

/// A stage of a save, as the panel says it.
#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    /// The drawing is written into bytes: objects so far.
    Encoding { done: usize, total: usize },
    /// The bytes are read back and compared with the drawing.
    Verifying,
    /// The bytes go to the temporary file.
    Writing { done: u64, total: u64 },
    /// The file is flushed to the disk and read back.
    Checking,
}

impl Stage {
    /// In words, and how far the whole save is (0–1).
    pub fn describe(&self) -> (String, f32) {
        let part = |done: f64, total: f64| {
            if total > 0.0 {
                (done / total).min(1.0)
            } else {
                1.0
            }
        };
        match self {
            Stage::Encoding { done, total } => (
                format!("Çizim yazılıyor: {done} / {total} nesne"),
                0.3 * part(*done as f64, *total as f64) as f32,
            ),
            Stage::Verifying => ("Yazılan baytlar geri okunuyor".to_owned(), 0.4),
            Stage::Writing { done, total } => (
                format!(
                    "Dosyaya yazılıyor: {} / {} MB",
                    done / 1_000_000,
                    total / 1_000_000
                ),
                0.6 + 0.3 * part(*done as f64, *total as f64) as f32,
            ),
            Stage::Checking => ("Diskteki dosya denetleniyor".to_owned(), 0.95),
        }
    }
}

/// Why a save did not write the file.
#[derive(Debug, Clone, PartialEq)]
pub enum SaveError {
    /// The user stopped it before the file was replaced.
    Stopped,
    /// It failed; what happened and what to do (Turkish).
    Failed(String),
}

/// Where a test makes the disk fail (FILE-18: a full disk, a permission taken
/// away) without a full disk: the step fails with this error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    Create,
    Write,
    Sync,
    ReadBack,
    Rename,
}

/// The faults a save meets: none in the app, one chosen in tests.
#[derive(Debug, Clone, Copy, Default)]
pub struct Faults {
    pub fail: Option<(Fault, std::io::ErrorKind)>,
}

impl Faults {
    pub const NONE: Faults = Faults { fail: None };

    fn check(&self, at: Fault) -> std::io::Result<()> {
        match self.fail {
            Some((fault, kind)) if fault == at => {
                Err(std::io::Error::new(kind, "sınamada bozulan disk"))
            }
            _ => Ok(()),
        }
    }
}

/// What went wrong with the disk, and what to do about it, in Turkish.
pub fn disk_failure(path: &Path, e: &std::io::Error) -> String {
    use std::io::ErrorKind as K;
    let file = path.display();
    let (what, then) = match e.kind() {
        K::StorageFull => (
            "diskte yer kalmadı",
            "yer açıp yeniden kaydedin ya da başka bir diske kaydedin (Farklı kaydet)",
        ),
        K::PermissionDenied => (
            "bu klasöre ya da dosyaya yazma izniniz yok",
            "izni denetleyin ya da başka bir yere kaydedin (Farklı kaydet)",
        ),
        K::ReadOnlyFilesystem => (
            "disk salt okunur",
            "yazılabilir bir diske kaydedin (Farklı kaydet)",
        ),
        K::NotFound => (
            "klasör artık yok (taşınmış ya da silinmiş)",
            "Farklı kaydet ile yeni bir yer seçin",
        ),
        _ => ("", "başka bir yere kaydetmeyi deneyin (Farklı kaydet)"),
    };
    if what.is_empty() {
        format!("{file} yazılamadı: {e}. Önceki dosya olduğu gibi duruyor; {then}.")
    } else {
        format!("{file} yazılamadı: {what} ({e}). Önceki dosya olduğu gibi duruyor; {then}.")
    }
}

/// Writes a drawing as a `.kcad` v2 file (see the module comment), telling
/// `tell` its stages; `stop` ends it before the file is replaced. On any
/// failure or stop the previous file is as it was and no temporary file stays.
pub fn write_watched(
    snapshot: &DocumentSnapshotV2,
    path: &Path,
    stop: &AtomicBool,
    tell: &mut dyn FnMut(Stage),
    faults: &Faults,
) -> Result<(), SaveError> {
    let mut watch = |s: Step<'_>| {
        match s {
            Step::Writing { done, total } => tell(Stage::Encoding { done, total }),
            Step::Verifying => tell(Stage::Verifying),
            _ => {}
        }
        !stop.load(Ordering::Relaxed)
    };
    let bytes = match kentos_kcad::encode_verified_watched(snapshot, &mut watch) {
        Ok(bytes) => bytes,
        Err(e) if e.code == Code::Cancelled => return Err(SaveError::Stopped),
        Err(e) => return Err(SaveError::Failed(e.message)),
    };
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .map_or_else(|| "cizim".into(), |n| n.to_string_lossy());
    static SAVES: AtomicU64 = AtomicU64::new(0);
    let temporary = dir.join(format!(
        ".{name}.{}-{}.yaziliyor",
        std::process::id(),
        SAVES.fetch_add(1, Ordering::Relaxed)
    ));
    let stopped = || stop.load(Ordering::Relaxed);
    let result = (|| -> std::io::Result<bool> {
        faults.check(Fault::Create)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let total = bytes.len() as u64;
        let mut done = 0u64;
        for chunk in bytes.chunks(CHUNK) {
            if stopped() {
                return Ok(false);
            }
            faults.check(Fault::Write)?;
            file.write_all(chunk)?;
            done += chunk.len() as u64;
            tell(Stage::Writing { done, total });
        }
        tell(Stage::Checking);
        faults.check(Fault::Sync)?;
        file.sync_all()?;
        drop(file);
        faults.check(Fault::ReadBack)?;
        if std::fs::read(&temporary)? != bytes {
            return Err(std::io::Error::other(
                "diskten geri okunan baytlar yazılanlarla aynı değil",
            ));
        }
        // The last moment a stop is taken: after the rename the file is written.
        if stopped() {
            return Ok(false);
        }
        faults.check(Fault::Rename)?;
        std::fs::rename(&temporary, path)?;
        // The new name itself is on the disk only when the directory is flushed (POSIX).
        #[cfg(unix)]
        if let Ok(d) = std::fs::File::open(dir) {
            let _ = d.sync_all();
        }
        Ok(true)
    })();
    match result {
        Ok(true) => Ok(()),
        Ok(false) => {
            let _ = std::fs::remove_file(&temporary);
            Err(SaveError::Stopped)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&temporary);
            Err(SaveError::Failed(disk_failure(path, &e)))
        }
    }
}

// ── The app's save ───────────────────────────────────────────────────────

/// What a save tells the app.
#[derive(Debug, Clone)]
pub enum Event {
    /// The save dialog answered: where to write, or nothing (cancelled).
    Picked(Option<PathBuf>),
    /// How far the save `id` is.
    Progress { id: u64, stage: Stage },
    /// The save `id` ended.
    Done {
        id: u64,
        result: Result<(), SaveError>,
    },
    /// Durdur in the save's panel.
    Stop,
}

/// A save under way.
pub struct Saving {
    pub id: u64,
    pub path: PathBuf,
    /// Which opened drawing, and the revision being written.
    pub session: u64,
    pub revision: u64,
    pub stage: String,
    pub fraction: f32,
    started: Instant,
    stop: Arc<AtomicBool>,
}

/// How long a save runs before its panel shows (a quick save does not flash one).
const QUIET: Duration = Duration::from_millis(300);

fn next_id() -> u64 {
    static SAVES: AtomicU64 = AtomicU64::new(0);
    SAVES.fetch_add(1, Ordering::Relaxed) + 1
}

impl App {
    /// Starts writing the drawing to `path` on a thread of its own: a cheap
    /// copy of the document (shared objects) and its revision are taken now,
    /// so the file holds the drawing of this moment and a later edit stays
    /// unsaved. One save at a time.
    pub(crate) fn start_saving(&mut self, path: PathBuf) -> Task<Message> {
        if self.saving.is_some() {
            self.warn("Bir kayıt sürüyor; bitince yeniden kaydedin.");
            return Task::none();
        }
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let model = doc.model.clone();
        let (id, session, revision) = (next_id(), doc.session, doc.model.revision());
        let stop = Arc::new(AtomicBool::new(false));
        let faults = self.save_faults;
        self.saving = Some(Saving {
            id,
            path: path.clone(),
            session,
            revision,
            stage: "Çizim hazırlanıyor".to_owned(),
            fraction: 0.0,
            started: Instant::now(),
            stop: stop.clone(),
        });
        iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
            let mut last = Instant::now();
            let mut tell = |stage: Stage| {
                // Told a few times a second at most, and never waiting for the window.
                if last.elapsed() >= Duration::from_millis(50) {
                    last = Instant::now();
                    let _ = out.try_send(Message::Saving(Event::Progress { id, stage }));
                }
            };
            let snapshot = model.to_snapshot_v2();
            drop(model);
            let result = write_watched(&snapshot, &path, &stop, &mut tell, &faults);
            let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                &mut out,
                Message::Saving(Event::Done { id, result }),
            ));
        })
    }

    /// A save's messages.
    pub(crate) fn saving_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Picked(Some(path)) => return self.start_saving(path),
            Event::Picked(None) => {}
            Event::Progress { id, stage } => {
                if let Some(s) = self.saving.as_mut().filter(|s| s.id == id) {
                    (s.stage, s.fraction) = stage.describe();
                }
            }
            Event::Stop => {
                if let Some(s) = self.saving.as_mut() {
                    s.stop.store(true, Ordering::Relaxed);
                    s.stage = "Durduruluyor…".to_owned();
                }
            }
            Event::Done { id, result } => {
                let Some(s) = self.saving.take_if(|s| s.id == id) else {
                    return Task::none();
                };
                return match result {
                    Ok(()) => self.update(Message::Saved(Some(Ok(Written {
                        session: s.session,
                        path: s.path,
                        revision: s.revision,
                    })))),
                    Err(SaveError::Stopped) => {
                        self.warn(format!(
                            "{} kaydı durduruldu; dosyaya dokunulmadı, çizim kaydedilmemiş sayılıyor.",
                            s.path.display()
                        ));
                        Task::none()
                    }
                    Err(SaveError::Failed(why)) => {
                        self.say(
                            Level::Error,
                            format!("{why} Çizim kaydedilmemiş sayılıyor."),
                        );
                        Task::none()
                    }
                };
            }
        }
        Task::none()
    }

    /// The save's panel at the window's bottom right, once the save takes a moment.
    pub(crate) fn saving_view(&self) -> Option<Element<'_, Message>> {
        let s = self
            .saving
            .as_ref()
            .filter(|s| s.started.elapsed() >= QUIET)?;
        let name = s.path.file_name().map_or_else(
            || s.path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let list = TaskList::new().push(
            Job::new(format!("Kaydediliyor: {name}"))
                .detail(s.stage.clone())
                .running(Some(s.fraction))
                .on_cancel(Message::Saving(Event::Stop)),
        );
        let panel = container(list)
            .width(360)
            .padding(10)
            .style(style::container::popover);
        Some(
            container(panel)
                .width(Fill)
                .height(Fill)
                .padding([72, 16])
                .align_x(Right)
                .align_y(Bottom)
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;
    use std::sync::atomic::AtomicBool;

    use super::*;
    use crate::app::Picker;
    use crate::files_testing::{app_with_drawing, drawing, drive, last_said, scratch};

    /// The files in `dir`, sorted: a failed or stopped save leaves no temporary one.
    fn names(dir: &Path) -> Vec<String> {
        let mut n: Vec<String> = std::fs::read_dir(dir)
            .expect("lists")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        n.sort();
        n
    }

    #[test]
    fn a_failing_disk_leaves_the_previous_file_and_says_what_to_do() {
        let dir = scratch("disk");
        let path = dir.join("pafta.kcad");
        let snapshot = drawing(5).model.to_snapshot_v2();
        crate::document::write(&snapshot, &path).expect("a good file first");
        let good = std::fs::read(&path).expect("written");
        let cases = [
            (
                Fault::Create,
                ErrorKind::ReadOnlyFilesystem,
                "disk salt okunur",
            ),
            (Fault::Write, ErrorKind::StorageFull, "diskte yer kalmadı"),
            (Fault::Sync, ErrorKind::StorageFull, "diskte yer kalmadı"),
            (Fault::ReadBack, ErrorKind::Other, "sınamada bozulan disk"),
            (
                Fault::Rename,
                ErrorKind::PermissionDenied,
                "yazma izniniz yok",
            ),
        ];
        for (fault, kind, says) in cases {
            let faults = Faults {
                fail: Some((fault, kind)),
            };
            let e = write_watched(
                &snapshot,
                &path,
                &AtomicBool::new(false),
                &mut |_| {},
                &faults,
            )
            .expect_err("fails");
            let SaveError::Failed(why) = e else {
                panic!("{fault:?}: {e:?}");
            };
            assert!(why.contains(says), "{fault:?}: {why}");
            assert!(why.contains("Önceki dosya olduğu gibi duruyor"), "{why}");
            assert!(
                std::fs::read(&path).expect("still there") == good,
                "{fault:?}"
            );
            assert_eq!(names(&dir), ["pafta.kcad"], "{fault:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_save_stopped_at_any_stage_before_the_rename_leaves_the_previous_file() {
        let dir = scratch("stop");
        let path = dir.join("pafta.kcad");
        // Large enough to be written in more than one batch of objects.
        let snapshot = drawing(20_000).model.to_snapshot_v2();
        std::fs::write(&path, b"onceki").expect("a previous file");
        for stage in ["encoding", "verifying", "writing", "checking"] {
            let stop = AtomicBool::new(false);
            let mut tell = |s: Stage| {
                let now = match s {
                    Stage::Encoding { .. } => "encoding",
                    Stage::Verifying => "verifying",
                    Stage::Writing { .. } => "writing",
                    Stage::Checking => "checking",
                };
                if now == stage {
                    stop.store(true, Ordering::Relaxed);
                }
            };
            let result = write_watched(&snapshot, &path, &stop, &mut tell, &Faults::NONE);
            assert_eq!(result, Err(SaveError::Stopped), "{stage}");
            assert_eq!(std::fs::read(&path).expect("there"), b"onceki", "{stage}");
            assert_eq!(names(&dir), ["pafta.kcad"], "{stage}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_app_saves_off_its_thread_and_a_failed_save_keeps_the_drawing_unsaved() {
        let dir = scratch("app-save");
        let path = dir.join("pafta.kcad");
        let mut app = app_with_drawing();
        app.picker = Picker::File(path.clone());
        let layer = app.document.as_ref().expect("a drawing").layers()[0]
            .id
            .clone();
        let _ = app.update(Message::LayerLocked(layer));
        assert!(app.document.as_ref().expect("a drawing").dirty());

        app.save_faults = Faults {
            fail: Some((Fault::Write, std::io::ErrorKind::StorageFull)),
        };
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        assert!(app.saving.is_none());
        assert!(app.document.as_ref().expect("a drawing").dirty());
        let said = last_said(&app);
        assert!(
            said.contains("diskte yer kalmadı") && said.ends_with("Çizim kaydedilmemiş sayılıyor."),
            "{said}"
        );
        assert!(!path.exists());
        assert!(names(&dir).is_empty());

        app.save_faults = Faults::NONE;
        let task = app.update(Message::Run("file.save"));
        drive(&mut app, task);
        let doc = app.document.as_ref().expect("a drawing");
        assert!(!doc.dirty(), "{}", last_said(&app));
        assert_eq!(doc.path.as_deref(), Some(path.as_path()));
        assert!(last_said(&app).starts_with("Kaydedildi:"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stopped_save_says_so_and_the_drawing_stays_unsaved() {
        let dir = scratch("app-stop");
        let path = dir.join("pafta.kcad");
        let mut app = app_with_drawing();
        let layer = app.document.as_ref().expect("a drawing").layers()[0]
            .id
            .clone();
        let _ = app.update(Message::LayerLocked(layer));
        let task = app.start_saving(path.clone());
        let id = app.saving.as_ref().expect("saving").id;
        // Durdur, then the thread's answer: stopped before the rename.
        let _ = app.update(Message::Saving(Event::Stop));
        let _ = app.update(Message::Saving(Event::Done {
            id,
            result: Err(SaveError::Stopped),
        }));
        assert!(app.document.as_ref().expect("a drawing").dirty());
        assert!(
            last_said(&app).contains("kaydı durduruldu; dosyaya dokunulmadı"),
            "{}",
            last_said(&app)
        );
        // Whatever the thread itself answers later belongs to no save any more.
        drive(&mut app, task);
        assert!(app.document.as_ref().expect("a drawing").dirty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_window_closing_while_a_save_runs_waits_for_it() {
        let dir = scratch("close");
        let mut app = app_with_drawing();
        let task = app.start_saving(dir.join("pafta.kcad"));
        let _ = app.update(Message::CloseRequested(iced::window::Id::unique()));
        assert!(last_said(&app).contains("kayıt bitince pencereyi yeniden kapatın"));
        assert!(app.dialog.is_none());
        drive(&mut app, task);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
