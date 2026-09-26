//! Opening a drawing on the desktop in stages, off the UI thread (TODOS.md
//! FILE-20, docs/adr/0030; CLAUDE.md §21.2).
//!
//! A thread of its own reads the file in chunks, checks its integrity,
//! learns the project (name, layers, objects) before its objects, reads the
//! objects and builds the document; each stage goes to a window over the
//! drawing, with Vazgeç. Nothing is drawn or edited meanwhile: the window is
//! modal and the app takes no command until the open ends (only Esc and
//! Vazgeç). The drawing on screen is replaced in one step, and only by a
//! document read and checked whole, only by the latest open, and only if the
//! drawing on screen did not change meanwhile. A stopped open returns
//! nothing; the file's bytes go as soon as they are read, so a large file is
//! not held twice.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc;
use iced::widget::{button, column};
use iced::{Element, Task};
use kentos_contracts::DocumentSnapshotV1;
use kentos_kcad::{Code, Sniff, Step};
use kentos_ui::widget::{Dialog, overlay, progress};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::document::Document;
use kentos_interaction::Level;

/// A stage of an open.
#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    /// The file's bytes so far.
    File { done: u64, total: u64 },
    /// The integrity check (SHA-256): bytes so far.
    Checking { done: u64, total: u64 },
    /// The project, known before its objects.
    Project {
        name: String,
        layers: usize,
        objects: usize,
    },
    /// Objects read so far.
    Reading { done: usize, total: usize },
    /// The document is being built and checked.
    Building,
}

/// The largest file an open reads (a KCAD v2 payload is at most 2³⁰ bytes):
/// a larger one is refused before memory is taken for it.
const LARGEST: u64 = (1 << 30) + (1 << 20);
/// Bytes read between two looks at the stop flag.
const CHUNK: u64 = 8 << 20;

fn read_bytes(
    path: &Path,
    stop: &AtomicBool,
    tell: &mut dyn FnMut(Stage),
) -> Result<Option<Vec<u8>>, String> {
    let failed = |e: std::io::Error| format!("{} okunamadı: {e}.", path.display());
    let file = std::fs::File::open(path).map_err(failed)?;
    let total = file.metadata().map_err(failed)?.len();
    if total > LARGEST {
        return Err(format!(
            "{}: dosya {} MB; KentOS en çok {} MB'lık bir çizim dosyası açar. Dosya bozuk olabilir ya da başka bir türdendir.",
            path.display(),
            total / 1_000_000,
            LARGEST / 1_000_000
        ));
    }
    let mut data = Vec::with_capacity(total as usize);
    let mut file = file;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let n = file
            .by_ref()
            .take(CHUNK)
            .read_to_end(&mut data)
            .map_err(failed)?;
        if n == 0 {
            break;
        }
        if data.len() as u64 > LARGEST {
            return Err(format!(
                "{}: dosya okunurken büyüdü; açılmadı.",
                path.display()
            ));
        }
        tell(Stage::File {
            done: data.len() as u64,
            total: total.max(data.len() as u64),
        });
    }
    Ok(Some(data))
}

/// Reads a `.kcad` file, v2 or v1, whatever its name says, in stages told to
/// `tell`; `stop` ends it (`Ok(None)`). Anything else is refused with the reason.
pub fn read(
    path: &Path,
    stop: &AtomicBool,
    tell: &mut dyn FnMut(Stage),
) -> Result<Option<Document>, String> {
    let at = |e: String| format!("{}: {e}", path.display());
    let Some(data) = read_bytes(path, stop, tell)? else {
        return Ok(None);
    };
    match kentos_kcad::sniff(&data) {
        Sniff::Kcad | Sniff::KcadDamaged => {
            let mut watch = |s: Step<'_>| {
                match s {
                    Step::Checking { done, total } => tell(Stage::Checking { done, total }),
                    Step::Project {
                        name,
                        layers,
                        objects,
                    } => tell(Stage::Project {
                        name: name.to_owned(),
                        layers,
                        objects,
                    }),
                    Step::Reading { done, total } => tell(Stage::Reading { done, total }),
                    Step::Writing { .. } | Step::Verifying => {}
                }
                !stop.load(Ordering::Relaxed)
            };
            let snapshot = match kentos_kcad::decode_watched(&data, &mut watch) {
                Ok(snapshot) => snapshot,
                Err(e) if e.code == Code::Cancelled => return Ok(None),
                Err(e) => return Err(at(e.message)),
            };
            // The file's bytes go before the document is built: a large file is not held twice.
            drop(data);
            tell(Stage::Building);
            if stop.load(Ordering::Relaxed) {
                return Ok(None);
            }
            Document::from_v2(snapshot, Some(path.to_path_buf()))
                .map(Some)
                .map_err(at)
        }
        Sniff::Json => {
            tell(Stage::Building);
            let text = std::str::from_utf8(&data).map_err(|_| {
                at("metin UTF-8 değil; eski (v1) bir KentOS çizimi okunamadı.".to_owned())
            })?;
            let snapshot = DocumentSnapshotV1::from_json(text).map_err(at)?;
            drop(data);
            if stop.load(Ordering::Relaxed) {
                return Ok(None);
            }
            Document::new(snapshot, Some(path.to_path_buf()))
                .map(Some)
                .map_err(at)
        }
        Sniff::Empty => Err(at("dosya boş; içinde çizim yok.".to_owned())),
        Sniff::Foreign => Err(at(
            "KentOS çizim dosyası değil. DXF ve koordinat listeleri İçe aktar ile açılır."
                .to_owned(),
        )),
    }
}

// ── The app's open ───────────────────────────────────────────────────────

/// What an open tells the app.
#[derive(Debug, Clone)]
pub enum Event {
    /// The open dialog answered: which file, or nothing (cancelled).
    Picked(Option<PathBuf>),
    /// How far the open `id` is.
    Progress { id: u64, stage: Stage },
    /// The open `id` ended: the drawing, the reason it could not be read, or
    /// nothing (it was stopped).
    Done {
        id: u64,
        result: Option<Result<Box<Document>, String>>,
    },
    /// Vazgeç, Esc, or a click beside the window.
    Cancel,
}

/// What an open puts on screen once it has read the file.
#[derive(Debug, Clone, PartialEq)]
pub enum Purpose {
    /// The file itself, as Aç opens it.
    File,
    /// A recovery copy of unsaved work (recovery.rs): unsaved, without a file.
    Recovery { id: String, name: String },
}

/// An open under way.
pub struct Opening {
    pub id: u64,
    /// What is being opened, for the window and the messages.
    pub name: String,
    pub project: Option<String>,
    pub stage: String,
    pub fraction: f32,
    pub purpose: Purpose,
    started: Instant,
    /// The drawing on screen when the open began: which one, and its revision.
    was: Option<(u64, u64)>,
    stop: Arc<AtomicBool>,
}

/// How long an open runs before its window shows (a quick open does not flash one).
const QUIET: Duration = Duration::from_millis(250);

fn next_id() -> u64 {
    static OPENS: AtomicU64 = AtomicU64::new(0);
    OPENS.fetch_add(1, Ordering::Relaxed) + 1
}

/// A stage in words, and how far the whole open is (0–1).
fn describe(stage: &Stage) -> (String, f32) {
    let part = |done: f64, total: f64| {
        if total > 0.0 {
            (done / total).min(1.0) as f32
        } else {
            1.0
        }
    };
    match stage {
        Stage::File { done, total } => (
            format!(
                "Dosya okunuyor: {} / {} MB",
                done / 1_000_000,
                total / 1_000_000
            ),
            0.15 * part(*done as f64, *total as f64),
        ),
        Stage::Checking { done, total } => (
            format!(
                "Dosya denetleniyor (bütünlük özeti): %{:.0}",
                100.0 * part(*done as f64, *total as f64)
            ),
            0.15 + 0.15 * part(*done as f64, *total as f64),
        ),
        Stage::Project { .. } => ("Nesneler okunuyor".to_owned(), 0.3),
        Stage::Reading { done, total } => (
            format!("Nesneler okunuyor: {done} / {total}"),
            0.3 + 0.55 * part(*done as f64, *total as f64),
        ),
        Stage::Building => ("Çizim kuruluyor ve denetleniyor".to_owned(), 0.9),
    }
}

impl App {
    /// Starts opening `path` on a thread of its own; the window shows if it
    /// takes a moment. An open still running gives way to this one.
    pub(crate) fn start_opening(&mut self, path: PathBuf, purpose: Purpose) -> Task<Message> {
        if let Some(earlier) = self.opening.take() {
            earlier.stop.store(true, Ordering::Relaxed);
        }
        let id = next_id();
        let stop = Arc::new(AtomicBool::new(false));
        let name = match &purpose {
            Purpose::File => path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            ),
            Purpose::Recovery { name, .. } => format!("“{name}” kurtarma kopyası"),
        };
        self.opening = Some(Opening {
            id,
            name,
            project: None,
            stage: "Dosya okunuyor".to_owned(),
            fraction: 0.0,
            purpose,
            started: Instant::now(),
            was: self
                .document
                .as_ref()
                .map(|d| (d.session, d.model.revision())),
            stop: stop.clone(),
        });
        iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
            let mut last = Instant::now();
            let mut tell = |stage: Stage| {
                let project = matches!(stage, Stage::Project { .. });
                let message = Message::Opening(Event::Progress { id, stage });
                if project {
                    // The project is told once: it waits for its turn.
                    let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                        &mut out, message,
                    ));
                } else if last.elapsed() >= Duration::from_millis(50) {
                    // Progress a few times a second at most, never waiting for the window.
                    last = Instant::now();
                    let _ = out.try_send(message);
                }
            };
            let result = match read(&path, &stop, &mut tell) {
                Ok(Some(doc)) => Some(Ok(Box::new(doc))),
                Ok(None) => None,
                Err(e) => Some(Err(e)),
            };
            let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(
                &mut out,
                Message::Opening(Event::Done { id, result }),
            ));
        })
    }

    /// An open's messages.
    pub(crate) fn opening_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Picked(Some(path)) => return self.start_opening(path, Purpose::File),
            Event::Picked(None) => {}
            Event::Progress { id, stage } => {
                if let Some(o) = self.opening.as_mut().filter(|o| o.id == id) {
                    if let Stage::Project {
                        name,
                        layers,
                        objects,
                    } = &stage
                    {
                        o.project = Some(format!("“{name}”: {objects} nesne, {layers} üst katman"));
                    }
                    (o.stage, o.fraction) = describe(&stage);
                }
            }
            Event::Cancel => {
                if let Some(o) = self.opening.take() {
                    o.stop.store(true, Ordering::Relaxed);
                    self.output(format!(
                        "{} açılışı durduruldu; ekrandaki çizim olduğu gibi duruyor.",
                        quoted(&o.name)
                    ));
                }
            }
            Event::Done { id, result } => {
                // A stopped or overtaken open: whatever it read is dropped here.
                let Some(o) = self.opening.take_if(|o| o.id == id) else {
                    return Task::none();
                };
                match result {
                    None => {}
                    Some(Err(error)) => self.say(Level::Error, error),
                    Some(Ok(mut doc)) => {
                        let now = self
                            .document
                            .as_ref()
                            .map(|d| (d.session, d.model.revision()));
                        if now != o.was {
                            self.warn(format!(
                                "{} açılmadı: açılış sürerken ekrandaki çizim değişti ya da başka bir çizim açıldı; o çizim olduğu gibi duruyor. Dosyayı yeniden açın.",
                                quoted(&o.name)
                            ));
                            return Task::none();
                        }
                        if let Purpose::Recovery { id, name } = &o.purpose {
                            // Unsaved work, not a file: Save asks where; no file is written over by itself.
                            doc.path = None;
                            doc.legacy = false;
                            doc.model.mark_unsaved();
                            let task = self.update(Message::Opened(Some(Ok(doc))));
                            self.recovered(id, name);
                            return task;
                        }
                        return self.update(Message::Opened(Some(Ok(doc))));
                    }
                }
            }
        }
        Task::none()
    }

    /// While an open runs the app takes no command: Esc stops the open, the
    /// window's own buttons work; everything else waits (the drawing is about
    /// to be replaced). None: the message goes on as usual.
    pub(crate) fn while_opening(&mut self, message: &Message) -> Option<Task<Message>> {
        self.opening.as_ref()?;
        match message {
            Message::Key(press) => {
                if press.named() == Some(iced::keyboard::key::Named::Escape) {
                    return Some(self.opening_event(Event::Cancel));
                }
                Some(Task::none())
            }
            Message::Run(_)
            | Message::CommandSubmitted
            | Message::CommandRun(_)
            | Message::PromptOption(_)
            | Message::LayerVisible(_)
            | Message::LayerLocked(_)
            | Message::Settings(_) => Some(Task::none()),
            _ => None,
        }
    }

    /// The open's window, once the open takes a moment.
    pub(crate) fn opening_view(&self) -> Option<Element<'_, Message>> {
        let o = self.opening.as_ref()?;
        if o.started.elapsed() < QUIET {
            return None;
        }
        let stop = button(label::body("Vazgeç"))
            .on_press(Message::Opening(Event::Cancel))
            .padding([5, 16])
            .style(style::button::secondary);
        let body = column![
            label::body(o.name.clone()),
            label::muted(o.project.clone().unwrap_or_default()),
            progress::bar(Some(o.fraction)),
            label::caption(o.stage.clone()),
            label::muted(
                "Açık çizim, yenisi bütünüyle okunup denetlenene kadar olduğu gibi kalır; Vazgeç ona dokunmaz."
            ),
        ]
        .spacing(10);
        Some(overlay::modal(
            Dialog::new("Çizim açılıyor")
                .push(body)
                .action(stop)
                .width(460.0),
            Message::Opening(Event::Cancel),
        ))
    }
}

/// A name in quotes, unless it is quoted already.
fn quoted(name: &str) -> String {
    if name.starts_with('“') {
        name.to_owned()
    } else {
        format!("“{name}”")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::{app_with_drawing, drive, last_said, saved, scratch};

    /// What a stage is called in the tests.
    fn kind(s: &Stage) -> &'static str {
        match s {
            Stage::File { .. } => "file",
            Stage::Checking { .. } => "checking",
            Stage::Project { .. } => "project",
            Stage::Reading { .. } => "reading",
            Stage::Building => "building",
        }
    }

    #[test]
    fn a_file_is_read_in_stages_the_project_before_its_objects() {
        let dir = scratch("stages");
        let path = saved(&dir, "büyük.kcad", 5000);
        let mut seen: Vec<Stage> = Vec::new();
        let doc = read(&path, &AtomicBool::new(false), &mut |s| seen.push(s))
            .expect("reads")
            .expect("not stopped");
        assert_eq!(doc.entity_count(), 5013);
        let mut kinds: Vec<&str> = seen.iter().map(kind).collect();
        kinds.dedup();
        assert_eq!(
            kinds,
            ["file", "checking", "project", "reading", "building"]
        );
        assert!(seen.contains(&Stage::Project {
            name: "Örnek pafta.kcad".into(),
            layers: 2,
            objects: 5013
        }));
        let reads = seen.iter().filter(|s| kind(s) == "reading").count();
        assert!(reads >= 3, "{reads} reports while reading 5013 objects");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_read_stopped_at_any_stage_gives_nothing() {
        let dir = scratch("stopped");
        let path = saved(&dir, "büyük.kcad", 5000);
        for at in ["file", "checking", "project", "reading", "building"] {
            let stop = AtomicBool::new(false);
            let got = read(&path, &stop, &mut |s| {
                if kind(&s) == at {
                    stop.store(true, Ordering::Relaxed);
                }
            });
            assert!(
                matches!(got, Ok(None)),
                "{at}: {:?}",
                got.map(|d| d.map(|d| d.entity_count()))
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_open_replaces_the_drawing_once_off_the_ui_thread() {
        let dir = scratch("open");
        let path = saved(&dir, "pafta.kcad", 3000);
        let mut app = app_with_drawing();
        let before = app.document.as_ref().expect("a drawing").session;
        let task = app.start_opening(path.clone(), Purpose::File);
        assert!(app.opening.is_some());
        // Commands wait while it runs; the drawing on screen is still the old one.
        let _ = app.update(Message::Run("layer.showAll"));
        assert_eq!(app.document.as_ref().expect("a drawing").session, before);
        drive(&mut app, task);
        let doc = app.document.as_ref().expect("a drawing");
        assert_ne!(doc.session, before);
        assert_eq!(doc.entity_count(), 3013);
        assert_eq!(doc.path.as_deref(), Some(path.as_path()));
        assert!(!doc.dirty());
        assert!(app.opening.is_none());
        assert!(
            last_said(&app).contains("açıldı: 3013 nesne"),
            "{}",
            last_said(&app)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn vazgec_or_esc_stops_it_and_the_drawing_stays() {
        let dir = scratch("cancel");
        let path = saved(&dir, "pafta.kcad", 3000);
        for how in ["button", "esc"] {
            let mut app = app_with_drawing();
            let before = app.document.as_ref().expect("a drawing").session;
            let task = app.start_opening(path.clone(), Purpose::File);
            let _ = if how == "esc" {
                app.update(Message::Key(crate::keys::KeyPress {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    physical: iced::keyboard::key::Physical::Code(
                        iced::keyboard::key::Code::Escape,
                    ),
                    modifiers: iced::keyboard::Modifiers::empty(),
                    text: None,
                    repeat: false,
                }))
            } else {
                app.update(Message::Opening(Event::Cancel))
            };
            assert!(app.opening.is_none(), "{how}");
            assert!(
                last_said(&app).contains("açılışı durduruldu; ekrandaki çizim olduğu gibi duruyor"),
                "{how}"
            );
            // The thread's answer comes to no open: whatever it read is dropped.
            drive(&mut app, task);
            assert_eq!(
                app.document.as_ref().expect("a drawing").session,
                before,
                "{how}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_later_open_overtakes_an_earlier_one() {
        let dir = scratch("overtaken");
        let first = saved(&dir, "birinci.kcad", 10);
        let second = saved(&dir, "ikinci.kcad", 20);
        let mut app = app_with_drawing();
        let early = app.start_opening(first, Purpose::File);
        let late = app.start_opening(second.clone(), Purpose::File);
        // The later one ends first; the earlier one's answer, coming after, is dropped.
        drive(&mut app, late);
        drive(&mut app, early);
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.path.as_deref(), Some(second.as_path()));
        assert_eq!(doc.entity_count(), 33);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_open_gives_way_when_the_drawing_on_screen_changed_meanwhile() {
        let dir = scratch("changed");
        let path = saved(&dir, "pafta.kcad", 10);
        let mut app = app_with_drawing();
        let task = app.start_opening(path, Purpose::File);
        // Changed by something that is not a command (a layer toggle straight on the document).
        let layer = app.document.as_ref().expect("a drawing").layers()[0]
            .id
            .clone();
        app.document
            .as_mut()
            .expect("a drawing")
            .model
            .toggle_layer_locked(&layer);
        let before = app.document.as_ref().expect("a drawing").session;
        drive(&mut app, task);
        assert_eq!(app.document.as_ref().expect("a drawing").session, before);
        assert!(
            last_said(&app).contains("açılış sürerken ekrandaki çizim değişti"),
            "{}",
            last_said(&app)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_file_never_becomes_the_drawing() {
        let dir = scratch("broken");
        let path = saved(&dir, "pafta.kcad", 10);
        let mut bytes = std::fs::read(&path).expect("written");
        let at = bytes.len() / 2;
        bytes[at] ^= 0x40;
        std::fs::write(&path, &bytes).expect("damaged");
        let mut app = app_with_drawing();
        let before = app.document.as_ref().expect("a drawing").session;
        let task = app.start_opening(path, Purpose::File);
        drive(&mut app, task);
        assert_eq!(app.document.as_ref().expect("a drawing").session, before);
        assert!(last_said(&app).contains("SHA-256"), "{}", last_said(&app));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
