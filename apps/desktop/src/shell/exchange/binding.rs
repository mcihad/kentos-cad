//! Tables bound on a thread of their own (docs/adr/0757; the owner's words: “50.000 satırı olan bir tablom var … 3 4
//! belki de 10 saniye kadar sistemi kilitliyor … qgis içinde bu problemlerin hiçbiri yok”). Each table's
//! `cad.sources.edit` link runs as a job on the drawing's reading copy (`jobs::Job`): the read, the rows made into
//! objects, the layer's digest and base all happen there; what it wrote comes into the drawing as one undo step when it
//! is done (the job's patch), then the next table's begins. The window that asked closes at once; a panel in the
//! corner says the table, its step (`sources::BIND_STEPS`) and how far it is, the bar eased each frame and moving on
//! through a step by the time it is expected to take (the table's estimated rows); Durdur ends it at once: the job is
//! asked to stop, its answer is not waited for and nothing of it is written (the tables bound before it stay, each its
//! own step, as before). Meanwhile the drawing takes no edit: the job's write would be refused (docs/adr/0253 §9); the
//! view moves, Esc stops. A file's tables known to hold fewer than [`THREAD_FROM`] rows each are bound at once, as
//! before: their bind takes less than the thread and the panel's first frame. A database's count is the catalogue's
//! estimate, which a bulk load leaves stale (`reltuples`), so its tables always go on the thread; the panel shows only a
//! bind that takes longer than [`SHOWN_AFTER`].

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use iced::widget::container;
use iced::{Bottom, Element, Fill, Right};
use kentos_commands::{CallAnswer, CommandResult, SourcesEdit, SourcesEditOp, SourcesEdited};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_native_application::jobs::Job;
use kentos_native_application::sources::{self, BIND_STEPS, bind_step};
use kentos_ui::widget::progress::{Task as Panel, TaskList};

use super::{Event as Exchange, message};
use crate::app::{App, Message};

/// A table to bind: its name (`schema.name`, a query, a GeoPackage's table), whether it has geometry, and about how many
/// rows it holds (the bar's pace), when known.
#[derive(Clone, Debug)]
pub(crate) struct Table {
    pub name: String,
    pub geometry: bool,
    pub rows: Option<u64>,
}

/// What is bound, from where, and what the announcement says at the end.
#[derive(Clone, Debug)]
pub(crate) struct Ask {
    /// A database's address or a file's path.
    pub source: String,
    /// The connection's or the file's name, as the panel and the announcement say it.
    pub shown: String,
    pub tables: Vec<Table>,
    pub reproject: bool,
    /// Only the rows in this area (the view's), for the tables with geometry.
    pub area: Option<[f64; 4]>,
    pub key: Option<String>,
    /// Whether the tables are a file's (a GeoPackage's): the announcement names the file.
    pub file: bool,
    /// The announcement's last words when no warning takes their place.
    pub then: String,
}

/// About how long one row takes all told, seconds: the 50 000 rows of 28 columns of docs/adr/0757's measure (1,41 s on
/// its machine); with the steps' shares, how long each step is expected to take.
const ROW_SECONDS: f64 = 28e-6;

/// Rows assumed when the catalogue does not say (a view, a query).
const UNKNOWN_ROWS: u64 = 20_000;

/// Rows from which a table is bound on a thread of its own (the processing runner's and the topology check's
/// threshold): fewer are bound sooner than the thread and the panel's first frame.
pub(crate) const THREAD_FROM: u64 = 2000;

/// How long a bind goes on before its panel shows: a small table's is over sooner, without a panel flashing by.
pub(crate) const SHOWN_AFTER: Duration = Duration::from_millis(150);

/// How a bind began.
#[derive(Debug, PartialEq)]
pub(crate) enum Started {
    /// Bound at once, said and shown.
    Done,
    /// Refused at once: why, for the window that asked to say.
    Failed(String),
    /// Going on its thread: the window that asked closes.
    Going,
}

/// What the tables bound so far gave.
#[derive(Default)]
struct Tally {
    /// The drawing's next slot before the first table: the bound objects' slots follow it.
    first: u64,
    bound: Vec<String>,
    objects: u32,
    warnings: Vec<String>,
}

impl Tally {
    fn add(
        &mut self,
        table: &Table,
        output: Option<SourcesEdited>,
        warnings: Vec<kentos_commands::CommandWarning>,
    ) {
        self.objects += output.map_or(0, |o| o.objects);
        self.bound.push(shown_name(&table.name));
        self.warnings
            .extend(warnings.into_iter().map(|w| w.message));
    }

    /// Why `table` was not bound, after those that were.
    fn failed(&self, table: &Table, why: String) -> String {
        if self.bound.is_empty() {
            why
        } else {
            format!(
                "{} bağlandı; “{}” bağlanamadı: {why}",
                self.bound.join(", "),
                shown_name(&table.name)
            )
        }
    }
}

/// Tables bound one after another, one job at a time (`App::binding`).
pub(crate) struct Binding {
    ask: Ask,
    left: VecDeque<Table>,
    /// The table bound now and its job.
    table: Table,
    job: Job,
    /// Its place among the tables, from 1.
    place: usize,
    /// The drawing bound into (its session), and what the tables bound so far gave.
    session: u64,
    tally: Tally,
    /// What the job said of its share last, and when; what the bar shows and when it last moved.
    said: f64,
    said_at: Instant,
    shown: f64,
    moved: Instant,
    /// When the first table began: the panel shows after [`SHOWN_AFTER`].
    began: Instant,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A frame: the job looked at, the bar eased.
    Frame,
    Stop,
}

fn event(e: Event) -> Message {
    message(Exchange::Binding(e))
}

/// A table's name as the panel and the announcement give it: a query as “Sorgu”.
pub(crate) fn shown_name(table: &str) -> String {
    super::postgis::shown_name(table)
}

/// `cad.sources.edit`'s link of `table`.
fn input_of(ask: &Ask, table: &Table) -> SourcesEdit {
    SourcesEdit {
        edit: SourcesEditOp::Link {
            extent: ask.area.filter(|_| table.geometry),
            path: ask.source.clone(),
            table: table.name.clone(),
            layer: None,
            reproject: ask.reproject,
            key: ask.key.clone(),
        },
    }
}

/// `cad.sources.edit`'s link of `table` as a job on `doc`'s reading copy (`sources::edit_job`: no protocol summary).
fn job_for(n: u64, doc: &kentos_domain::Document, ask: &Ask, table: &Table) -> Job {
    sources::edit_job(format!("bagla-{n}"), doc, input_of(ask, table))
}

/// Jobs told apart in their threads' names.
static JOBS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_job() -> u64 {
    JOBS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
}

impl Binding {
    /// The bar's next frame at `now`: towards what the job said, and within its step on by the time gone against the
    /// time the step is expected to take (63 % of the way at that time, 95 % at most); eased, never back.
    pub(crate) fn frame(&mut self, now: Instant) {
        let (_, next) = bind_step(self.said);
        let rows = self.table.rows.unwrap_or(UNKNOWN_ROWS) as f64;
        let expected = (rows * ROW_SECONDS * (next - self.said)).max(0.05);
        let gone = now.saturating_duration_since(self.said_at).as_secs_f64();
        let target = self.said + (next - self.said) * 0.95 * (1.0 - (-gone / expected).exp());
        let dt = now
            .saturating_duration_since(self.moved)
            .as_secs_f64()
            .min(0.25);
        self.moved = now;
        let a = 1.0 - (-dt / 0.12).exp();
        self.shown = self
            .shown
            .max(self.shown + (target - self.shown) * a)
            .min(1.0);
    }

    /// The job said `share`.
    pub(crate) fn heard(&mut self, share: f64) {
        if share > self.said {
            self.said = share;
            self.said_at = Instant::now();
        }
    }

    /// What the bar shows, 0..1.
    #[cfg(test)]
    pub(crate) fn shown(&self) -> f64 {
        self.shown
    }

    /// The next table's job begins on `doc`.
    fn next(&mut self, doc: &kentos_domain::Document, table: Table) {
        self.job = job_for(next_job(), doc, &self.ask, &table);
        self.table = table;
        self.place += 1;
        let now = Instant::now();
        (self.said, self.said_at, self.shown, self.moved) = (0.0, now, 0.0, now);
    }
}

impl App {
    /// Binds `ask`'s tables one after another: at once when each is known to be small, else on threads of their own
    /// (see the module).
    pub(crate) fn start_binding(&mut self, ask: Ask) -> Started {
        if self.binding.is_some() || self.importing.is_some() {
            return Started::Failed(
                "Bir bağlama ya da içe aktarma sürüyor; o bitince yeniden deneyin.".to_owned(),
            );
        }
        let Some(doc) = &mut self.document else {
            return Started::Failed(
                "Açık çizim yok. Önce bir çizim açın (Ctrl+O) ya da yeni proje oluşturun."
                    .to_owned(),
            );
        };
        let first = doc.model.next_slot();
        if ask.file
            && ask
                .tables
                .iter()
                .all(|t| t.rows.is_some_and(|r| r < THREAD_FROM))
        {
            let mut tally = Tally {
                first,
                ..Tally::default()
            };
            for table in &ask.tables {
                match sources::edit::execute(
                    &mut ExecutionContext::new(&mut doc.model),
                    input_of(&ask, table),
                ) {
                    CommandResult::Completed { output, warnings } => {
                        tally.add(table, Some(output), warnings);
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => {
                        return Started::Failed(tally.failed(table, error.message));
                    }
                    _ => return Started::Failed("Bağlanamadı.".to_owned()),
                }
            }
            self.bound(&ask, tally);
            return Started::Done;
        }
        let mut left: VecDeque<Table> = ask.tables.iter().cloned().collect();
        let Some(table) = left.pop_front() else {
            return Started::Done;
        };
        let job = job_for(next_job(), &doc.model, &ask, &table);
        let now = Instant::now();
        self.binding = Some(Binding {
            session: doc.session,
            tally: Tally {
                first,
                ..Tally::default()
            },
            ask,
            left,
            table,
            job,
            place: 1,
            said: 0.0,
            said_at: now,
            shown: 0.0,
            moved: now,
            began: now,
        });
        Started::Going
    }

    /// Whether a bind goes on.
    #[cfg(test)]
    pub(crate) fn binding_now(&self) -> bool {
        self.binding.is_some()
    }

    pub(super) fn binding_event(&mut self, e: Event) {
        match e {
            Event::Frame => self.binding_frame(),
            Event::Stop => self.stop_binding(),
        }
    }

    /// Durdur: the job asked to stop; its answer is not waited for, nothing of it is written.
    pub(crate) fn stop_binding(&mut self) {
        let Some(mut b) = self.binding.take() else {
            return;
        };
        b.job.cancel();
        let done = if b.tally.bound.is_empty() {
            String::new()
        } else {
            format!(" {} bağlı katman oldu, o kaldı.", b.tally.bound.join(", "))
        };
        self.say(
            Level::Info,
            format!(
                "“{}” bağlanması durduruldu; bir şey yazılmadı.{done}",
                shown_name(&b.table.name)
            ),
        );
    }

    /// A frame: the job looked at; its answer written when it is over, then the next table, or the end.
    fn binding_frame(&mut self) {
        let (Some(doc), Some(b)) = (self.document.as_mut(), self.binding.as_mut()) else {
            self.binding = None;
            return;
        };
        if doc.session != b.session {
            // The drawing it was started on is gone: nothing to write to.
            b.job.cancel();
            self.binding = None;
            return;
        }
        let status = b.job.status(&mut doc.model);
        if !status.phase.finished() {
            if let Some(share) = status.progress {
                b.heard(share);
            }
            b.frame(Instant::now());
            return;
        }
        let answer = status
            .answer
            .unwrap_or_else(|| CallAnswer::of(CommandResult::Cancelled));
        match answer.result {
            CommandResult::Completed { output, warnings } => {
                let table = b.table.clone();
                b.tally
                    .add(&table, serde_json::from_value(output).ok(), warnings);
                if let Some(table) = b.left.pop_front() {
                    b.next(&doc.model, table);
                    return;
                }
                if let Some(b) = self.binding.take() {
                    self.bound(&b.ask, b.tally);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                if let Some(b) = self.binding.take() {
                    self.error(b.tally.failed(&b.table, error.message));
                }
            }
            _ => self.binding = None,
        }
    }

    /// Every table bound: the view on what came, and what it was. A file's tables are said with the file, their
    /// warnings after; a database's with the connection, a warning as the toast's body (what is not drawn and why is
    /// not left to the log, docs/adr/0745 §3).
    fn bound(&mut self, ask: &Ask, tally: Tally) {
        let Some(doc) = &self.document else {
            return;
        };
        let last = doc.model.next_slot();
        let slots: Vec<Slot> = (tally.first..last)
            .filter_map(|n| u32::try_from(n).ok())
            .map(Slot)
            .filter(|s| doc.model.get(*s).is_some())
            .collect();
        self.zoom_to(&slots);
        let level = if tally.warnings.is_empty() {
            Level::Success
        } else {
            Level::Warn
        };
        if ask.file {
            self.announce(
                level,
                format!(
                    "“{}” bağlandı: {} tablo bağlı katman oldu, {} nesne. {}",
                    ask.shown,
                    tally.bound.len(),
                    tally.objects,
                    ask.then
                ),
            );
            for w in tally.warnings {
                self.say(Level::Warn, w);
            }
            return;
        }
        let body = if tally.warnings.is_empty() {
            ask.then.clone()
        } else {
            tally.warnings.join(" ")
        };
        let names = if tally.bound.len() == 1 {
            format!("“{}”", tally.bound[0])
        } else {
            tally.bound.join(", ")
        };
        self.announce(
            level,
            format!(
                "{names} ({}) bağlı katman oldu, {} nesne. {body}",
                ask.shown, tally.objects
            ),
        );
    }

    /// The panel of a bind going on: the table, its step and the bar, Durdur.
    pub(crate) fn binding_view(&self) -> Option<Element<'_, Message>> {
        let b = self.binding.as_ref()?;
        if b.began.elapsed() < SHOWN_AFTER {
            return None;
        }
        let (step, _) = bind_step(b.said);
        let mut detail = format!("{}…", BIND_STEPS[step].1);
        if let Some(rows) = b.table.rows {
            detail.push_str(&format!(
                " · yaklaşık {} satır",
                super::postgis::grouped(rows)
            ));
        }
        if b.ask.tables.len() > 1 {
            detail.push_str(&format!(" · tablo {} / {}", b.place, b.ask.tables.len()));
        }
        let list = TaskList::new().push(
            Panel::new(format!("Bağlanıyor: {}", shown_name(&b.table.name)))
                .detail(detail)
                .running(Some(b.shown as f32))
                .on_cancel(event(Event::Stop)),
        );
        let panel = container(list)
            .width(380)
            .padding(10)
            .style(kentos_ui::style::container::popover);
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
#[path = "binding_tests.rs"]
mod tests;
