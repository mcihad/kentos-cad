//! Toplu çalıştır in İşlemler's window (docs/adr/0529 §4, plan/gis.md CBS-24): the open tool or model over a table of
//! rows of parameter values. A column is a parameter, a row a run; an empty cell takes the window's value. The
//! window does not run the rows itself: it makes the `cad.processing.batch` input of its table and starts the
//! command as a job (docs/adr/0253 §9), so the rows are the runs `cad.processing.run` makes, one undo step, computed
//! on another thread; Durdur is heard between the rows and inside the running tool. A row that stops takes the whole
//! batch back (Hata olursa: Dur) or only itself (Devam et, docs/adr/0551 §1). What the command answers fills the rows'
//! states, the stopped rows' reasons and the record panel (steps, counts, durations, what running it again would meet).

use std::time::Duration;

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream};
use iced::widget::{Row as UiRow, button, column, container, row, scrollable, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_commands::{
    BatchOnError, CAD_PROCESSING_BATCH, CallAnswer, CallEnvelope, CallOp, CommandResult, JobPhase,
    ProcessingBatched, ProcessingRecord,
};
use kentos_native_application::jobs::Job;
use kentos_processing::batch_table::{self, Table};
use kentos_processing::command::command_values;
use kentos_processing::record::rerun_kind;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::progress::{self, Tint};
use kentos_ui::widget::table::{self as ui_table, Table as UiTable};
use kentos_ui::widget::{Dialog, Menu, MenuButton, Segmented, horizontal_divider, overlay};

use super::{Event as ProcessingEvent, Processing};
use crate::app::{App, Message};

/// How often a running batch is looked at.
const TICK: Duration = Duration::from_millis(120);
/// The window's size, as the tool's (window.rs).
const WIDTH: f32 = 940.0;
const HEIGHT: f32 = 660.0;
/// A parameter column's and the name column's width.
const CELL: f32 = 150.0;
const NAME: f32 = 130.0;
/// The progress bar of a running batch (as the form's).
const PROGRESS: f32 = 140.0;

/// What a batch does when a row stops (the table's Hata olursa choice).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnError {
    /// The whole batch is taken back.
    Stop,
    /// Only the row is taken back; the others are written.
    Continue,
}

impl OnError {
    const ALL: [OnError; 2] = [OnError::Stop, OnError::Continue];
}

impl std::fmt::Display for OnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OnError::Stop => "Dur",
            OnError::Continue => "Devam et",
        })
    }
}

impl From<BatchOnError> for OnError {
    fn from(m: BatchOnError) -> Self {
        match m {
            BatchOnError::Stop => OnError::Stop,
            BatchOnError::Continue => OnError::Continue,
        }
    }
}

impl From<OnError> for BatchOnError {
    fn from(m: OnError) -> Self {
        match m {
            OnError::Stop => BatchOnError::Stop,
            OnError::Continue => BatchOnError::Continue,
        }
    }
}

/// What the batch window asks for.
#[derive(Clone, Debug)]
pub enum Event {
    /// Toplu çalıştır in the tool's footer: the table opens.
    Open,
    /// Forma dön.
    Back,
    AddRow,
    RemoveRow(usize),
    /// A parameter made a column.
    AddColumn(String),
    RemoveColumn(usize),
    /// A row's name as typed.
    Label(usize, String),
    /// A cell's text as typed: row, column, text.
    Cell(usize, usize, String),
    /// Hata olursa: what a row that stops does.
    OnError(OnError),
    Run,
    /// Durdur: the job is asked to stop between rows.
    Stop,
    /// The running job looked at.
    Tick,
}

fn ev(e: Event) -> Message {
    Message::Processing(ProcessingEvent::Batch(e))
}

/// One row as the last run left it.
#[derive(Clone, Debug, PartialEq)]
pub enum RowState {
    Idle,
    Ran {
        records_in: u64,
        records_out: u64,
        ms: u64,
    },
    /// The row that stopped, or whose values were refused, and why.
    Failed(String),
    /// A row that ran before another stopped: its results were taken back.
    BackOut,
    /// A row after the one that stopped.
    NotRun,
}

/// The footer's line.
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    Idle,
    Running { share: Option<f64>, stopping: bool },
    Done(String),
    /// Finished with rows that stopped and were taken back (Devam et): the summary says which.
    Partial(String),
    Failed(String),
    Cancelled,
}

/// The table, what the last run did to each row, and the job while one runs.
pub struct BatchState {
    pub table: Table,
    pub rows: Vec<RowState>,
    pub line: Line,
    job: Option<Job>,
    /// The drawing the job was started on: its answer is written to no other.
    session: u64,
    /// The last finished batch's record, and what running it again would meet (said once, when it finished).
    pub record: Option<ProcessingRecord>,
    pub rerun: Option<String>,
    /// The rows that stopped and were taken back in the last run, with their reasons (Devam et).
    pub failed: Vec<(String, String)>,
    started: u64,
}

impl BatchState {
    /// The table of a tool: two rows and a column of its first number parameter (else its first offered one).
    pub fn new(tool: &kentos_processing::Tool) -> Self {
        let offered = batch_table::offered(tool);
        let first = offered
            .iter()
            .find(|p| matches!(p.kind, kentos_processing::ParamKind::Number { .. }))
            .or_else(|| offered.first());
        let mut table = Table::with_rows(2);
        if let Some(p) = first {
            table.add_column(&p.name);
        }
        let rows = vec![RowState::Idle; table.rows.len()];
        BatchState {
            table,
            rows,
            line: Line::Idle,
            job: None,
            session: 0,
            record: None,
            rerun: None,
            failed: Vec::new(),
            started: 0,
        }
    }

    pub fn running(&self) -> bool {
        self.job.is_some()
    }

    /// An edit: what the last run said no longer fits the table.
    fn edited(&mut self) {
        self.rows = vec![RowState::Idle; self.table.rows.len()];
        self.failed.clear();
        if !self.running() {
            self.line = Line::Idle;
        }
    }

    /// What a finished job's answer did to the rows and the footer.
    fn answered(&mut self, answer: CallAnswer, rerun: impl FnOnce(&ProcessingRecord) -> String) {
        self.rows = vec![RowState::Idle; self.table.rows.len()];
        self.record = None;
        self.rerun = None;
        self.failed.clear();
        match answer.result {
            CommandResult::Completed { output, .. } => {
                match serde_json::from_value::<ProcessingBatched>(output) {
                    Ok(done) => {
                        for r in &done.rows {
                            if let Some(slot) = self.rows.get_mut(r.index as usize) {
                                *slot = RowState::Ran {
                                    records_in: r.ran.record.total.records_in,
                                    records_out: r.ran.record.total.records_out,
                                    ms: r.ran.record.ms,
                                };
                            }
                        }
                        for f in &done.failed {
                            if let Some(slot) = self.rows.get_mut(f.index as usize) {
                                *slot = RowState::Failed(f.message.clone());
                            }
                        }
                        self.failed = done
                            .failed
                            .iter()
                            .map(|f| (f.label.clone(), f.message.clone()))
                            .collect();
                        self.rerun = Some(rerun(&done.record));
                        self.record = Some(done.record);
                        self.line = if done.failed.is_empty() {
                            Line::Done(done.summary)
                        } else {
                            Line::Partial(done.summary)
                        };
                    }
                    Err(e) => self.line = Line::Failed(format!("Cevap okunamadı: {e}")),
                }
            }
            CommandResult::Cancelled => {
                self.line = Line::Cancelled;
            }
            CommandResult::Failed { error }
            | CommandResult::NeedsInput { error }
            | CommandResult::Conflict { error } => {
                if let Some(i) = error.path.as_deref().and_then(row_of) {
                    let failed = matches!(error.code.as_str(), "batch_row_failed");
                    for (n, slot) in self.rows.iter_mut().enumerate() {
                        *slot = if n == i {
                            RowState::Failed(short(&error.message))
                        } else if failed && n < i {
                            RowState::BackOut
                        } else if failed {
                            RowState::NotRun
                        } else {
                            RowState::Idle
                        };
                    }
                }
                self.line = Line::Failed(error.message);
            }
            CommandResult::Queued { .. } => {
                self.line = Line::Failed("Beklenmeyen cevap: iş sıraya alındı.".into());
            }
        }
    }
}

/// The row an error path is about: `rows[2]` or `rows[2].parameters.distance`.
fn row_of(path: &str) -> Option<usize> {
    let rest = path.strip_prefix("rows[")?;
    rest.split(']').next()?.parse().ok()
}

/// The row's own words of a stopped row's message: after “çalışmadı: ”, before the batch's closing sentence.
fn short(message: &str) -> String {
    let at = message
        .find("çalışmadı: ")
        .map_or(0, |i| i + "çalışmadı: ".len());
    let rest = &message[at..];
    rest.split(" Toplu işin hiçbir")
        .next()
        .unwrap_or(rest)
        .to_owned()
}

/// Every `TICK` while a batch runs.
pub fn ticks() -> impl Stream<Item = Message> {
    let (mut out, ticks) = mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(TICK);
            if iced::futures::executor::block_on(out.send(ev(Event::Tick))).is_err() {
                break;
            }
        }
    });
    ticks
}

impl Processing {
    /// Whether a batch is running (the app listens to the clock then).
    pub fn batch_running(&self) -> bool {
        self.batch.as_ref().is_some_and(BatchState::running)
    }
}

impl App {
    /// The batch window's messages.
    pub(crate) fn processing_batch_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Open => {
                if let Some(window) = &self.processing.dialog
                    && !window.running()
                {
                    self.processing.batch = Some(BatchState::new(&window.tool));
                }
            }
            Event::Back => {
                if self.processing.batch.as_ref().is_some_and(|b| !b.running()) {
                    self.processing.batch = None;
                }
            }
            Event::Run => self.batch_run(),
            Event::Stop => {
                if let Some(b) = &mut self.processing.batch
                    && let Some(job) = &mut b.job
                {
                    job.cancel();
                    b.line = Line::Running {
                        share: None,
                        stopping: true,
                    };
                }
            }
            Event::Tick => self.batch_tick(),
            edit => {
                let Some(b) = &mut self.processing.batch else {
                    return Task::none();
                };
                if b.running() {
                    return Task::none();
                }
                match edit {
                    Event::AddRow => {
                        b.table.add_row();
                    }
                    Event::RemoveRow(i) => b.table.remove_row(i),
                    Event::AddColumn(name) => b.table.add_column(&name),
                    Event::RemoveColumn(i) => b.table.remove_column(i),
                    Event::Label(i, text) => {
                        if let Some(r) = b.table.rows.get_mut(i) {
                            r.label = text;
                        }
                    }
                    Event::Cell(r, c, text) => b.table.set_cell(r, c, text),
                    Event::OnError(m) => b.table.on_error = m.into(),
                    _ => {}
                }
                b.edited();
            }
        }
        Task::none()
    }

    /// Toplu çalıştır: the table's input started as the command's job.
    fn batch_run(&mut self) {
        let (Some(doc), Some(window), Some(b)) = (
            self.document.as_ref(),
            self.processing.dialog.as_ref(),
            self.processing.batch.as_mut(),
        ) else {
            return;
        };
        if b.running() {
            return;
        }
        let common = command_values(&doc.model, &window.tool, &window.values);
        let selection: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        let v = self.viewport.camera.visible_bounds();
        let view = Some([v.min_x, v.min_y, v.max_x, v.max_y]);
        let mut input =
            match batch_table::input(&doc.model, &window.tool, common, selection, view, &b.table) {
                Ok(input) => input,
                Err(refusal) => {
                    b.rows = vec![RowState::Idle; b.table.rows.len()];
                    if let Some(slot) = b.rows.get_mut(refusal.row) {
                        *slot = RowState::Failed(refusal.message.clone());
                    }
                    b.line =
                        Line::Failed(format!("Satır {}: {}", refusal.row + 1, refusal.message));
                    return;
                }
            };
        // A raster's relative path is found from the drawing's folder (docs/adr/0548 §3).
        input.drawing = doc.path.as_ref().map(|p| p.display().to_string());
        let envelope = CallEnvelope {
            command: CAD_PROCESSING_BATCH.into(),
            version: None,
            op: CallOp::Execute,
            input: serde_json::to_value(&input).unwrap_or_default(),
            request_id: None,
            expected: None,
            relax_constraints: false,
        };
        b.started += 1;
        b.session = doc.session;
        b.rows = vec![RowState::Idle; b.table.rows.len()];
        b.record = None;
        b.rerun = None;
        b.failed.clear();
        b.line = Line::Running {
            share: Some(0.0),
            stopping: false,
        };
        b.job = Some(Job::start(
            format!("toplu-{}", b.started),
            &doc.model,
            envelope,
        ));
    }

    /// The running job looked at: its progress, and when it is over its answer (written to the drawing by the job).
    fn batch_tick(&mut self) {
        let (Some(doc), Some(b)) = (self.document.as_mut(), self.processing.batch.as_mut()) else {
            return;
        };
        let Some(job) = &mut b.job else {
            return;
        };
        if doc.session != b.session {
            // The drawing it was started on is gone: nothing to write to.
            job.cancel();
            b.job = None;
            b.line = Line::Cancelled;
            return;
        }
        let status = job.status(&mut doc.model);
        if !status.phase.finished() {
            if let Line::Running { share, .. } = &mut b.line {
                *share = status.progress;
            }
            return;
        }
        b.job = None;
        let answer = status
            .answer
            .unwrap_or_else(|| CallAnswer::of(CommandResult::Cancelled));
        let phase = status.phase;
        let model = &doc.model;
        b.answered(answer, |record| rerun_kind(record, model).sentence());
        if phase == JobPhase::Cancelled {
            b.line = Line::Cancelled;
        }
    }
}

impl App {
    /// The batch window: the tool's own window with the table in the form's place.
    pub(crate) fn batch_view(&self) -> Element<'_, Message> {
        let (Some(b), Some(window)) = (&self.processing.batch, &self.processing.dialog) else {
            return iced::widget::space().into();
        };
        let tool = &window.tool;
        let body = column![
            intro(),
            toolbar(b, tool),
            on_error(b),
            grid(b, tool),
            failed_panel(b),
            record_panel(b)
        ]
        .spacing(12)
        .padding([4, 8]);
        overlay::modal(
            Dialog::new(format!("Toplu çalıştır: {}", tool.label))
                .push(
                    scrollable(body)
                        .direction(style::field::body_scrollbar())
                        .width(Fill)
                        .height(Fill),
                )
                .push(horizontal_divider())
                .push(footer(b))
                .width(WIDTH)
                .max_height(HEIGHT),
            ev(Event::Back),
        )
    }
}

fn intro<'a>() -> Element<'a, Message> {
    label::body(
        "Her satır aracı bir kez çalıştırır. Sütunlar satıra göre değişen parametrelerdir; boş hücre penceredeki değeri alır. Satırlar sırayla çalışır, hepsi tek geri alma adımıdır. Hata olursa: Dur seçiliyken biri çalışmazsa hiçbir şey yazılmaz; Devam et seçiliyken yalnız o satır geri alınır, öteki satırlar yazılır.",
    )
    .style(style::text::muted)
    .into()
}

/// Hata olursa: Dur ya da Devam et.
fn on_error<'a>(b: &'a BatchState) -> Element<'a, Message> {
    let idle = !b.running();
    row![
        label::body("Hata olursa"),
        Segmented::new_with(
            OnError::ALL,
            OnError::from(b.table.on_error),
            |m| ev(Event::OnError(m)),
            move |_| idle,
        )
        .compact()
        .hints([
            "Bir satır çalışıp dururlarsa bütün toplu iş geri alınır, hiçbir şey yazılmaz.",
            "Duran satır tek başına geri alınır; öteki satırlar yazılır, hepsi yine tek adımdır.",
        ]),
    ]
    .spacing(10)
    .align_y(Center)
    .into()
}

/// The rows that stopped and were taken back, with their reasons: the table's cell says which, this says why.
fn failed_panel<'a>(b: &'a BatchState) -> Element<'a, Message> {
    if b.failed.is_empty() {
        return iced::widget::space().into();
    }
    let mut lines = column![
        label::caption("Duran satırlar (geri alındı)")
            .font(typography::ui_strong())
            .style(style::text::muted)
    ]
    .spacing(3);
    for (name, why) in &b.failed {
        lines = lines.push(
            row![
                icon(Icon::Error).size(14.0).tone(Tone::Danger),
                label::caption(name.clone()).width(Length::Fixed(typography::scaled(110.0))),
                label::caption(why.clone()).width(Fill),
            ]
            .spacing(8)
            .align_y(Center),
        );
    }
    container(lines)
        .padding(12)
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// Sütun ekle ▾, the columns as chips, and Satır ekle.
fn toolbar<'a>(b: &'a BatchState, tool: &'a kentos_processing::Tool) -> Element<'a, Message> {
    let taken = b.table.columns.clone();
    let free: Vec<(String, String)> = batch_table::offered(tool)
        .into_iter()
        .filter(|p| !taken.contains(&p.name))
        .map(|p| (p.name.clone(), p.label.clone()))
        .collect();
    let idle = !b.running();
    let add_column: Element<'a, Message> = if idle && !free.is_empty() {
        MenuButton::new(
            container(
                row![
                    icon(crate::icons::from_web(Some("tableColumnRight"))).size(14.0),
                    label::body("Sütun ekle"),
                    icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .padding([4, 10])
            .style(style::container::field_box),
            move || {
                free.iter().fold(Menu::new(), |m, (name, label)| {
                    m.item(label.clone(), ev(Event::AddColumn(name.clone())))
                })
            },
        )
        .into()
    } else {
        container(
            row![
                icon(crate::icons::from_web(Some("tableColumnRight")))
                    .size(14.0)
                    .tone(Tone::Muted),
                label::body("Sütun ekle").style(style::text::muted),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([4, 10])
        .into()
    };
    let mut chips = UiRow::new().spacing(6).align_y(Center);
    for (i, name) in b.table.columns.iter().enumerate() {
        let text = tool
            .parameters
            .iter()
            .find(|p| &p.name == name)
            .map_or_else(|| name.clone(), |p| p.label.clone());
        chips = chips.push(
            container(
                row![
                    label::caption(text),
                    button(label::caption("×"))
                        .padding([0, 4])
                        .style(style::button::ghost)
                        .on_press_maybe(idle.then(|| ev(Event::RemoveColumn(i)))),
                ]
                .spacing(2)
                .align_y(Center),
            )
            .padding([2, 8])
            .style(style::container::token),
        );
    }
    let add_row = button(
        row![
            icon(crate::icons::from_web(Some("tableRowBelow"))).size(14.0),
            label::body("Satır ekle")
        ]
        .spacing(6)
        .align_y(Center),
    )
    .padding([4, 10])
    .style(style::button::secondary)
    .on_press_maybe(
        (idle && b.table.rows.len() < batch_table::MAX_BATCH_ROWS).then(|| ev(Event::AddRow)),
    );
    row![
        add_column,
        chips.wrap(),
        iced::widget::space().width(Fill),
        add_row
    ]
    .spacing(10)
    .align_y(Center)
    .into()
}

fn input<'a>(
    placeholder: &str,
    value: &str,
    on: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .padding([3, 6])
        .font(typography::ui())
        .size(typography::body())
        .style(style::field::validated(false))
        .on_input(on)
        .width(Fill)
        .into()
}

/// The table: a row's number, its name, a cell for each column, how it went, and its Sil.
fn grid<'a>(b: &'a BatchState, tool: &'a kentos_processing::Tool) -> Element<'a, Message> {
    let idle = !b.running();
    let mut columns = vec![
        ui_table::Column::new("#").width(28.0),
        ui_table::Column::new("Ad").width(NAME),
    ];
    for name in &b.table.columns {
        let text = tool
            .parameters
            .iter()
            .find(|p| &p.name == name)
            .map_or_else(|| name.clone(), |p| p.label.clone());
        columns.push(ui_table::Column::new(text).width(CELL));
    }
    columns.push(ui_table::Column::new("Durum").width(Length::Fill));
    columns.push(ui_table::Column::new("").width(34.0));
    let body: Vec<ui_table::Row<'a, Message>> = b
        .table
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut cells: Vec<Element<'a, Message>> = vec![
                label::mono_caption((i + 1).to_string())
                    .style(style::text::muted)
                    .into(),
                if idle {
                    input(&format!("Satır {}", i + 1), &r.label, move |t| {
                        ev(Event::Label(i, t))
                    })
                } else {
                    label::caption(if r.label.is_empty() {
                        format!("Satır {}", i + 1)
                    } else {
                        r.label.clone()
                    })
                    .into()
                },
            ];
            for (c, cell) in r.cells.iter().enumerate() {
                cells.push(if idle {
                    input("", cell, move |t| ev(Event::Cell(i, c, t)))
                } else {
                    label::caption(cell.clone()).into()
                });
            }
            cells.push(state_cell(b.rows.get(i)));
            cells.push(
                button(icon(crate::icons::from_web(Some("tableRowDelete"))).size(14.0))
                    .padding([2, 6])
                    .style(style::button::ghost)
                    .on_press_maybe(
                        (idle && b.table.rows.len() > 1).then(|| ev(Event::RemoveRow(i))),
                    )
                    .into(),
            );
            ui_table::Row::new(cells)
        })
        .collect();
    let height = (34.0 * (b.table.rows.len() as f32 + 1.0) + 8.0).min(300.0);
    // Few columns fill the window's width; many scroll sideways.
    let wide = b.table.columns.len() >= 4;
    let mut table = UiTable::new(columns).extend(body).height(height);
    if wide {
        table = table.horizontal();
    }
    container(table)
        .style(style::container::field_box)
        .width(Fill)
        .into()
}

/// How a row went, in its words.
fn state_cell<'a>(state: Option<&'a RowState>) -> Element<'a, Message> {
    match state {
        Some(RowState::Ran {
            records_in,
            records_out,
            ms,
        }) => row![
            icon(Icon::Success).size(14.0).tone(Tone::Success),
            label::caption(format!(
                "{records_in} → {records_out} kayıt · {}",
                took(*ms)
            )),
        ]
        .spacing(6)
        .align_y(Center)
        .into(),
        // The reason is long: it is in the footer's line; the cell says which row it is about.
        Some(RowState::Failed(_)) => row![
            icon(Icon::Error).size(14.0).tone(Tone::Danger),
            label::caption("Durdu; nedeni aşağıda"),
        ]
        .spacing(6)
        .align_y(Center)
        .into(),
        Some(RowState::BackOut) => label::caption("Geri alındı")
            .style(style::text::muted)
            .into(),
        Some(RowState::NotRun) => label::caption("Çalışmadı").style(style::text::muted).into(),
        _ => iced::widget::space().into(),
    }
}

/// A duration as the history writes it.
fn took(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{:.1} sn", ms as f64 / 1000.0)
    }
}

/// The record of the last finished batch (docs/adr/0529 §2): what ran, on what, its steps, counts and times, and
/// what running it again would meet.
fn record_panel<'a>(b: &'a BatchState) -> Element<'a, Message> {
    let Some(r) = &b.record else {
        return iced::widget::space().into();
    };
    let title = |t: &'static str| {
        label::caption(t)
            .font(typography::ui_strong())
            .style(style::text::muted)
    };
    let mut lines = column![
        title("Kayıt"),
        label::caption(format!(
            "{} · {} · sürüm {} · {} · {} · çizim sürümü {}{}",
            match r.kind.as_str() {
                "batch" => "toplu çalıştırma",
                "model" => "model",
                _ => "araç",
            },
            r.tool,
            r.tool_version,
            r.engine,
            r.host,
            r.document_revision,
            r.srid.map_or_else(String::new, |s| format!(" · EPSG:{s}")),
        )),
    ]
    .spacing(3);
    for s in &r.steps {
        lines = lines.push(
            row![
                label::caption(s.label.clone()).width(Length::Fixed(typography::scaled(180.0))),
                label::mono_caption(format!(
                    "{} → {} kayıt · {}",
                    s.records_in,
                    s.records_out,
                    took(s.ms)
                ))
                .style(style::text::muted),
            ]
            .spacing(10),
        );
    }
    lines = lines.push(label::caption(format!(
        "Toplam: {} kayıt okundu, {} nesne yazıldı ya da seçildi · {}",
        r.total.records_in,
        r.total.records_out,
        took(r.ms)
    )));
    if let Some(sentence) = &b.rerun {
        lines = lines.push(label::caption(sentence.clone()).style(style::text::muted));
    }
    container(lines)
        .padding(12)
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// Forma dön, the line (how it goes, ended, or why not), Kapat or Durdur, and Toplu çalıştır.
fn footer<'a>(b: &'a BatchState) -> Element<'a, Message> {
    let running = b.running();
    let back = button(label::body("Forma dön"))
        .padding([5, 10])
        .style(style::button::ghost)
        .on_press_maybe((!running).then(|| ev(Event::Back)));
    let mut status = UiRow::new().spacing(8).align_y(Center).width(Fill);
    match &b.line {
        Line::Idle => {}
        Line::Running { share, stopping } => {
            status = status
                .push(
                    progress::bar(share.map(|s| s as f32))
                        .width(Length::Fixed(typography::scaled(PROGRESS)))
                        .tint(Tint::Muted),
                )
                .push(label::body(if *stopping {
                    "Durduruluyor…"
                } else {
                    "Satırlar çalışıyor…"
                }));
        }
        Line::Done(text) => {
            status = status
                .push(icon(Icon::Success).size(16.0).tone(Tone::Success))
                .push(label::body(text.clone()).width(Fill));
        }
        Line::Partial(text) => {
            status = status
                .push(icon(Icon::Warning).size(16.0).tone(Tone::Warning))
                .push(label::body(text.clone()).width(Fill));
        }
        Line::Failed(text) => {
            status = status
                .push(icon(Icon::Error).size(16.0).tone(Tone::Danger))
                .push(label::body(text.clone()).width(Fill));
        }
        Line::Cancelled => {
            status = status
                .push(icon(Icon::Warning).size(16.0).tone(Tone::Warning))
                .push(label::body("Durduruldu; çizim değişmedi.").width(Fill));
        }
    }
    let close = button(label::body(if running { "Durdur" } else { "Kapat" }))
        .padding([5, 14])
        .style(style::button::secondary)
        .on_press(if running {
            ev(Event::Stop)
        } else {
            Message::Processing(ProcessingEvent::Close)
        });
    let run = button(
        row![icon(Icon::Play).size(14.0), label::body("Toplu çalıştır")]
            .spacing(6)
            .align_y(Center),
    )
    .padding([5, 14])
    .style(style::button::primary)
    .on_press_maybe((!running).then(|| ev(Event::Run)));
    row![back, status, close, run]
        .spacing(10)
        .align_y(Center)
        .into()
}
