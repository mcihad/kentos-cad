//! GeoPackage dışa aktar's write off the interface's thread (docs/adr/9191): `file.export.gpkg` runs on a thread of
//! its own over a reading copy of the drawing. The window's foot says how far it is (rows written of rows) and Durdur
//! stops it between two batches: the write goes into a part that takes the file's place only at the end, so the file
//! is as it was. A late answer of a write the window no longer waits for is dropped.

use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc;
use iced::widget::column;
use iced::{Element, Length, Task};
use kentos_commands::{CommandError, CommandWarning, ExportGpkg, GpkgExported};
use kentos_interaction::Level;
use kentos_native_application::geopackage::Flow;
use kentos_ui::widget::progress;

use super::gpkg_out::Event;
use super::{Event as Exchange, Window, message};
use crate::app::{App, Message};

/// The writes started: a late answer of one the window no longer waits for is dropped.
static RUNS: AtomicU64 = AtomicU64::new(0);
/// Least time between two progress messages of a write.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// What a write answers: what it wrote and said, or why it did not.
pub type Answer = Result<(GpkgExported, Vec<CommandWarning>), CommandError>;

/// A write under way: its number, its file, how far it is, the flag Durdur sets, whether Durdur was pressed.
#[derive(Debug, Clone)]
pub struct Running {
    pub run: u64,
    pub path: PathBuf,
    pub done: u64,
    pub total: u64,
    pub stop: Arc<AtomicBool>,
    pub stopping: bool,
}

fn event(e: Event) -> Message {
    message(Exchange::GpkgOut(e))
}

impl App {
    /// `input` written on a thread of its own; the window waits, its foot telling how far.
    pub(super) fn gpkg_out_start(&mut self, path: PathBuf, input: ExportGpkg) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        // The data layers' rows written too (docs/adr/9344).
        let copy = self.source_copy().map_or_else(|| doc.model.reading_copy(), |c| c.doc.reading_copy());
        let run = RUNS.fetch_add(1, Ordering::Relaxed) + 1;
        let stop = Arc::new(AtomicBool::new(false));
        if let Some(Window::GpkgOut(s)) = &mut self.exchange {
            s.running = Some(Running {
                run,
                path,
                done: 0,
                total: 0,
                stop: stop.clone(),
                stopping: false,
            });
            s.status = None;
        }
        let (out, replies) = mpsc::unbounded::<Event>();
        let spawned = std::thread::Builder::new().name("kentos-gpkg-teslim".into()).spawn(move || {
            let told = out.clone();
            let last: Cell<Option<Instant>> = Cell::new(None);
            let stopped = || stop.load(Ordering::Relaxed);
            let progress = |done: u64, total: u64| {
                let now = Instant::now();
                if last.get().is_none_or(|t| now.duration_since(t) >= PROGRESS_EVERY) {
                    last.set(Some(now));
                    let _ = told.unbounded_send(Event::Progress(run, done, total));
                }
            };
            let flow = Flow {
                stopped: &stopped,
                progress: &progress,
            };
            let answer = kentos_headless::file_commands::export_gpkg_flow(&copy, &input, &flow).map_err(|e| *e);
            let _ = out.unbounded_send(Event::Done(run, Box::new(answer)));
        });
        match spawned {
            Ok(_) => Task::run(replies, event),
            Err(e) => Task::done(event(Event::Done(
                run,
                Box::new(Err(CommandError {
                    code: kentos_commands::codes::FILE_UNWRITABLE.to_owned(),
                    message: format!("iş parçacığı başlatılamadı ({e}); yeniden deneyin."),
                    ..CommandError::default()
                })),
            ))),
        }
    }

    /// Durdur: the write is asked to stop before its next batch.
    pub(super) fn gpkg_out_stop(&mut self) {
        if let Some(Window::GpkgOut(s)) = &mut self.exchange
            && let Some(r) = &mut s.running
        {
            r.stop.store(true, Ordering::Relaxed);
            r.stopping = true;
        }
    }

    /// How far the write `run` is.
    pub(super) fn gpkg_out_progress(&mut self, run: u64, done: u64, total: u64) {
        if let Some(Window::GpkgOut(s)) = &mut self.exchange
            && let Some(r) = s.running.as_mut().filter(|r| r.run == run)
        {
            r.done = done;
            r.total = total;
        }
    }

    /// The write `run` back: said and the window closed; stopped or refused, the window stays and says why.
    pub(super) fn gpkg_out_done(&mut self, run: u64, answer: Answer) {
        let running = match &mut self.exchange {
            Some(Window::GpkgOut(s)) => s.running.take_if(|r| r.run == run),
            _ => None,
        };
        let Some(running) = running else {
            return;
        };
        let status = match answer {
            Ok((output, warnings)) => {
                self.gpkg_written(&running.path, &output, warnings);
                self.exchange = None;
                return;
            }
            Err(e) if e.code == kentos_commands::codes::CANCELLED => {
                let words = "Dışa aktarma durduruldu; dosyaya hiçbir şey yazılmadı.".to_owned();
                self.say(Level::Info, words.clone());
                (false, words)
            }
            Err(e) => {
                let words = format!("GeoPackage yazılamadı. {}", e.message);
                self.announce(Level::Error, words.clone());
                (true, words)
            }
        };
        if let Some(Window::GpkgOut(s)) = &mut self.exchange {
            s.status = Some(status);
        }
    }
}

/// The foot while the file is written: the rows written of the rows, the bar.
pub(super) fn foot(r: &Running) -> Element<'_, Message> {
    let thousands = |n: u64| crate::view::thousands(n as f64);
    let (said, share) = if r.stopping {
        ("Durduruluyor; dosyaya hiçbir şey yazılmayacak…".to_owned(), None)
    } else if r.total == 0 {
        ("Hazırlanıyor…".to_owned(), None)
    } else {
        let share = (r.done as f32 / r.total as f32).clamp(0.0, 1.0);
        (
            format!(
                "{} / {} satır yazıldı (%{:.0})",
                thousands(r.done),
                thousands(r.total),
                share * 100.0
            ),
            Some(share),
        )
    };
    column![kentos_ui::label::caption(said), progress::bar(share)]
        .spacing(4)
        .width(Length::Fill)
        .into()
}
