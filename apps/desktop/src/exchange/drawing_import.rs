//! DXF ve Netcad NCZ içe aktar (the web's `ui/io/DrawingImportDialog.ts`).
//!
//! The shared reader reads the whole file once on a thread of its own and
//! says how far it is (the window's bar), and Vazgeç stops it: a DXF with its
//! blocks exploded and object coordinate systems applied; an NCZ with Netcad
//! 8's smart objects drawn as their symbols and its declared coordinate
//! system (docs/adr/0138). The window shows the source's layers with their
//! object counts and where each goes (a project layer with the same name, or
//! a new layer in a group named after the file), the report, and the
//! coordinate system question.
//!
//! Everything chosen goes in as one undo step. A small file goes in at once;
//! a large one a slice of time at a time, one slice per frame
//! (`apply::Progressive`): the window closes, the drawing fills in as the
//! objects arrive, a panel counts them and Durdur takes everything back.
//! Meanwhile the drawing takes no edit (`while_importing`); the view moves.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc;
use iced::widget::{Column, column, container, row};
use iced::{Bottom, Center, Element, Fill, Length, Right, Task};
use kentos_contracts::{DxfReadOptions, ImportLayer, ImportResult, LayerStyle, NczReadOptions, ReportItem};
use kentos_domain::Slot;
use kentos_formats::import;
use kentos_formats::watch::{STOPPED, Steps};
use kentos_interaction::{Format, Level};
use kentos_ui::label;
use kentos_ui::widget::progress::{self, Task as Job, TaskList};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Dialog, overlay, swatch};

use super::apply::{self, ImportPlan, LayerTarget, Progressive, layer_named};
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Kind, Picked, Window, message};
use crate::app::{App, Message};
use crate::crs::{CrsQuestion, Statement, grouped};
use crate::view::hex_color;

/// Reads told apart: a late answer to an older read is dropped.
static READS: AtomicU64 = AtomicU64::new(0);

/// Objects that go in at once; a larger import goes in a frame at a time.
pub(crate) const AT_ONCE: usize = 20_000;
/// Time a frame gives the import on the UI thread: a third of what the
/// frame took apart from it (drawing what came in so far is the most of it,
/// and grows as the drawing fills), between these two. A cheap frame keeps
/// the window at its rate; an expensive one writes more, so fewer frames
/// draw the growing drawing again.
const SLICE_LEAST: Duration = Duration::from_millis(10);
const SLICE_MOST: Duration = Duration::from_millis(50);
/// Least time between two progress messages of a read.
const PROGRESS_EVERY: Duration = Duration::from_millis(50);

/// The file's format: what it is read with and what the window says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Dxf,
    Ncz,
}

impl Source {
    fn title(self) -> &'static str {
        match self {
            Source::Dxf => "DXF içe aktar",
            Source::Ncz => "NCZ içe aktar",
        }
    }

    fn column(self) -> &'static str {
        match self {
            Source::Dxf => "DXF katmanı",
            Source::Ncz => "NCZ katmanı",
        }
    }

    /// The undo step's first word (“DXF: plan.dxf”).
    fn prefix(self) -> &'static str {
        match self {
            Source::Dxf => "DXF",
            Source::Ncz => "NCZ",
        }
    }

    fn kind(self) -> Kind {
        match self {
            Source::Dxf => Kind::Dxf,
            Source::Ncz => Kind::Ncz,
        }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    source: Source,
    file: Picked,
    read: u64,
    /// The read under way: how far (0..1), and the flag that stops it.
    reading: Option<(f32, Arc<AtomicBool>)>,
    result: Option<Arc<ImportResult>>,
    failed: Option<String>,
    /// The first object with a number that is not finite: nothing is imported.
    unusable: Option<String>,
    /// Source layer names left out by the user.
    excluded: BTreeSet<String>,
    crs: CrsQuestion,
    /// What the footer says, and whether it is an error.
    status: Option<(bool, String)>,
}

impl State {
    /// Asks the read under way to stop (the window closes, or another file is picked).
    pub(super) fn stop_reading(&self) {
        if let Some((_, stop)) = &self.reading {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    /// How far the read is.
    Progress { read: u64, share: f32 },
    Read {
        read: u64,
        result: Result<Arc<ImportResult>, String>,
        unusable: Option<String>,
    },
    Toggle(String),
    ToggleAll,
    Crs(u32),
    Another,
    Run,
    /// Vazgeç: a read under way stops, and the window closes.
    Cancel,
    /// A frame: the next objects go in.
    Frame,
    /// Durdur while the objects go in: everything the import made is taken back.
    Stop,
}

fn event(e: Event) -> Message {
    message(Exchange::DrawingImport(e))
}

/// An import going into the drawing a frame at a time (`App::importing`).
#[derive(Debug)]
pub struct Importing {
    name: String,
    layers: usize,
    work: Progressive,
    skipped: Vec<ReportItem>,
    /// When the last slice ended: the frame's own time is what passed since.
    last: Option<Instant>,
}

/// Where a source layer's objects go: the project layer of the same name
/// (and whether it is locked), or a new one.
struct Target {
    existing: Option<String>,
    locked: bool,
}

impl App {
    fn target(&self, layer: &ImportLayer) -> Target {
        let Some(doc) = &self.document else {
            return Target {
                existing: None,
                locked: false,
            };
        };
        match layer_named(&doc.model, &layer.name) {
            Some(node) => Target {
                locked: doc.model.layers().is_locked(&node.id),
                existing: Some(node.id.clone()),
            },
            None => Target {
                existing: None,
                locked: false,
            },
        }
    }

    /// The layers that will be imported: chosen, and not onto a locked layer.
    fn drawing_included<'a>(&self, state: &'a State) -> Vec<&'a ImportLayer> {
        state
            .result
            .as_ref()
            .map(|r| {
                r.layers
                    .iter()
                    .filter(|l| !state.excluded.contains(&l.name) && !self.target(l).locked)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The drawing's typeface, by its id (what an NCZ's symbol texts are centred in).
    fn drawing_font_id(&self) -> String {
        self.document
            .as_ref()
            .and_then(|d| d.settings().drawing_font)
            .and_then(|f| serde_json::to_value(f).ok())
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default()
    }

    /// Opens the window on `file` and reads it on a thread of its own.
    pub(super) fn drawing_import_picked(&mut self, source: Source, file: Picked) -> Task<Message> {
        let srid = self.project_srid();
        // “Başka dosya…” keeps the chosen coordinate system of a file that says nothing.
        let crs = match &self.exchange {
            Some(Window::DrawingImport(s)) if s.source == Source::Dxf && source == Source::Dxf => s.crs.clone(),
            _ => CrsQuestion::new(srid),
        };
        if let Some(Window::DrawingImport(s)) = &self.exchange {
            s.stop_reading();
        }
        let read = READS.fetch_add(1, Ordering::Relaxed) + 1;
        let stop = Arc::new(AtomicBool::new(false));
        let bytes = file.bytes.clone();
        let font = self.drawing_font_id();
        self.open_window(Window::DrawingImport(State {
            source,
            file,
            read,
            reading: Some((0.0, stop.clone())),
            result: None,
            failed: None,
            unusable: None,
            excluded: BTreeSet::new(),
            crs,
            status: None,
        }));
        let (out, replies) = mpsc::unbounded::<Event>();
        let spawned = std::thread::Builder::new()
            .name("kentos-ice-aktar".into())
            .spawn(move || {
                let told = out.clone();
                let mut last: Option<Instant> = None;
                let mut watch = Steps(|done: u64, total: u64| {
                    if stop.load(Ordering::Relaxed) {
                        return false;
                    }
                    let now = Instant::now();
                    if last.is_none_or(|t| now.duration_since(t) >= PROGRESS_EVERY) {
                        last = Some(now);
                        let share = done as f32 / total.max(1) as f32;
                        let _ = told.unbounded_send(Event::Progress { read, share });
                    }
                    true
                });
                let result = match source {
                    Source::Dxf => kentos_formats::dxf::read_watched(
                        &bytes,
                        &DxfReadOptions { max_entities: 0 },
                        &mut watch,
                    ),
                    Source::Ncz => kentos_ncz::read(
                        &bytes,
                        &NczReadOptions {
                            max_entities: 0,
                            drawing_font: font,
                        },
                        &mut watch,
                    ),
                };
                let unusable = result
                    .as_ref()
                    .ok()
                    .and_then(|r| apply::unusable(&r.entities))
                    .map(|(place, kind)| apply::unusable_text(place, kind));
                let _ = out.unbounded_send(Event::Read {
                    read,
                    result: result.map(Arc::new),
                    unusable,
                });
            });
        match spawned {
            Ok(_) => Task::run(replies, event),
            Err(e) => Task::done(event(Event::Read {
                read,
                result: Err(format!("Dosya okunamadı: iş parçacığı başlatılamadı ({e}).")),
                unusable: None,
            })),
        }
    }

    pub(super) fn drawing_import_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Another => {
                let kind = match &self.exchange {
                    Some(Window::DrawingImport(s)) => s.source.kind(),
                    _ => Kind::Dxf,
                };
                return self.pick(kind);
            }
            Event::Run => return self.run_drawing_import(),
            Event::Cancel => {
                self.close_exchange();
                return Task::none();
            }
            Event::Frame => return self.importing_frame(),
            Event::Stop => {
                self.stop_importing();
                return Task::none();
            }
            _ => {}
        }
        let project = self.project_srid();
        let Some(Window::DrawingImport(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Progress { read, share } => {
                if read == s.read
                    && let Some((at, _)) = &mut s.reading
                {
                    *at = share;
                }
            }
            Event::Read {
                read,
                result,
                unusable,
            } => {
                if read != s.read {
                    return Task::none();
                }
                s.reading = None;
                match result {
                    Ok(r) => {
                        // An NCZ says what its coordinates are in (its MPROJ); a DXF says nothing.
                        if s.source == Source::Ncz {
                            let statement = r
                                .declared_crs
                                .as_ref()
                                .map(|d| Statement::of(Some(d), false));
                            s.crs.declare(project, statement, r.bounds);
                        }
                        s.result = Some(r);
                    }
                    // A read stopped by “Başka dosya…” says nothing.
                    Err(e) if e == STOPPED => {}
                    Err(e) => s.failed = Some(e),
                }
                s.unusable = unusable;
            }
            Event::Toggle(name) => {
                if !s.excluded.remove(&name) {
                    s.excluded.insert(name);
                }
            }
            Event::ToggleAll => {
                if s.excluded.is_empty() {
                    if let Some(r) = &s.result {
                        s.excluded = r.layers.iter().map(|l| l.name.clone()).collect();
                    }
                } else {
                    s.excluded.clear();
                }
            }
            Event::Crs(srid) => s.crs.pick(srid),
            Event::Another | Event::Run | Event::Cancel | Event::Frame | Event::Stop => {}
        }
        Task::none()
    }

    fn drawing_can_import(&self, s: &State) -> bool {
        s.reading.is_none()
            && self.importing.is_none()
            && s.result.is_some()
            && s.unusable.is_none()
            && !self.drawing_included(s).is_empty()
            && s.crs.matches(self.project_srid())
    }

    fn run_drawing_import(&mut self) -> Task<Message> {
        let Some(Window::DrawingImport(s)) = &self.exchange else {
            return Task::none();
        };
        if !self.drawing_can_import(s) {
            return Task::none();
        }
        let Some(result) = s.result.clone() else {
            return Task::none();
        };
        let (name, source) = (s.file.name.clone(), s.source);
        let layers: Vec<(String, LayerTarget)> = self
            .drawing_included(s)
            .into_iter()
            .map(|l| {
                let target = match self.target(l).existing {
                    Some(id) => LayerTarget::Existing(id),
                    None => LayerTarget::New {
                        name: l.name.clone(),
                        style: Box::new(LayerStyle {
                            color: l.color.clone(),
                            line_type: l.line_type,
                            // The web's `...(l.lineWeight ? { lineWeight } : {})`: none or 0 is the default.
                            line_weight: l
                                .line_weight
                                .filter(|w| *w != 0.0)
                                .unwrap_or(kentos_domain::default_style().line_weight),
                            ..kentos_domain::default_style()
                        }),
                        visible: l.visible,
                        locked: l.locked,
                    },
                };
                (l.name.clone(), target)
            })
            .collect();
        let chosen = layers.len();
        let plan = ImportPlan {
            label: format!("{}: {name}", source.prefix()),
            layers,
            group: Some(name.clone()),
        };
        let skipped = result.report.skipped.clone();
        let Some(doc) = &mut self.document else {
            return Task::none();
        };

        if result.entities.len() <= AT_ONCE {
            match apply::apply_import(&mut doc.model, result.entities.clone(), &plan) {
                Err(error) => {
                    if let Some(Window::DrawingImport(s)) = &mut self.exchange {
                        s.status = Some((true, error));
                    }
                }
                Ok(applied) => {
                    self.close_exchange();
                    self.import_done(&name, chosen, &applied.slots, &applied.created, &skipped);
                }
            }
            return Task::none();
        }

        // A large file: the layers now, the objects a frame at a time, one undo step.
        match Progressive::start(&mut doc.model, &plan) {
            Err(error) => {
                if let Some(Window::DrawingImport(s)) = &mut self.exchange {
                    s.status = Some((true, error));
                }
                Task::none()
            }
            Ok(mut work) => {
                // The objects leave the window's result, its last holder by now.
                if let Some(Window::DrawingImport(s)) = &mut self.exchange {
                    s.result = None;
                }
                // The view goes where the file is first, so the drawing fills in before the
                // user's eyes and its first drawing is spread over the frames, not the last one.
                let chosen_layers: BTreeSet<&str> = plan.layers.iter().map(|(s, _)| s.as_str()).collect();
                let view = import::view_of(
                    result.view.as_ref(),
                    result
                        .layers
                        .iter()
                        .filter(|l| chosen_layers.contains(l.name.as_str()))
                        .filter_map(|l| l.bounds.as_ref()),
                );
                if let Some(b) = view {
                    self.zoom_to_bounds(&b);
                }
                let entities = Arc::try_unwrap(result).map_or_else(|shared| shared.entities.clone(), |r| r.entities);
                work.feed(entities);
                self.close_exchange();
                self.importing = Some(Importing {
                    name,
                    layers: chosen,
                    work,
                    skipped,
                    last: None,
                });
                Task::none()
            }
        }
    }

    /// One frame's slice of a large import.
    fn importing_frame(&mut self) -> Task<Message> {
        let (Some(doc), Some(job)) = (&mut self.document, &mut self.importing) else {
            return Task::none();
        };
        let budget = job
            .last
            .map_or(SLICE_LEAST, |t| (t.elapsed() / 3).clamp(SLICE_LEAST, SLICE_MOST));
        let stepped = job.work.step(&mut doc.model, budget);
        job.last = Some(Instant::now());
        match stepped {
            Ok(false) => {}
            Ok(true) => {
                if let Some(job) = self.importing.take() {
                    // The view went there when the writing began (and the user may have moved it since).
                    self.import_said(&job.name, job.layers, job.work.slots.len(), &job.work.created, &job.skipped);
                }
            }
            Err(error) => {
                self.importing = None;
                self.error(error);
            }
        }
        Task::none()
    }

    /// Durdur: what the import made goes, and nothing is recorded.
    pub(crate) fn stop_importing(&mut self) {
        let (Some(doc), Some(job)) = (&mut self.document, self.importing.take()) else {
            return;
        };
        job.work.stop(&mut doc.model);
        self.say(
            Level::Warn,
            format!("“{}” içe aktarılması durduruldu; çizim olduğu gibi kaldı.", job.name),
        );
    }

    fn import_done(
        &mut self,
        name: &str,
        chosen: usize,
        slots: &[Slot],
        created: &[String],
        skipped: &[ReportItem],
    ) {
        self.zoom_to(slots);
        self.import_said(name, chosen, slots.len(), created, skipped);
    }

    fn import_said(&mut self, name: &str, chosen: usize, count: usize, created: &[String], skipped: &[ReportItem]) {
        let into = if created.is_empty() {
            String::new()
        } else {
            format!("; {} yeni katman “{name}” grubunda", created.len())
        };
        self.say(
            Level::Success,
            format!(
                "“{name}”: {} nesne {chosen} katmana alındı{into}. Tek adımda geri alınabilir.",
                grouped(count as f64)
            ),
        );
        if !skipped.is_empty() {
            let skipped: Vec<String> = skipped.iter().map(words::report_text).collect();
            self.warn(format!("“{name}” içinde alınmayanlar: {}", skipped.join(" ")));
        }
    }

    /// While an import goes in, the drawing takes no edit, command, key or
    /// click: they would join its undo step. The view moves; Esc stops it.
    pub(crate) fn while_importing(&mut self, message: &Message) -> Option<Task<Message>> {
        self.importing.as_ref()?;
        use crate::viewport::Event as V;
        match message {
            Message::Exchange(e) => match &**e {
                Exchange::DrawingImport(Event::Frame | Event::Stop) => None,
                _ => Some(Task::none()),
            },
            Message::Key(press) => {
                if press.named() == Some(iced::keyboard::key::Named::Escape) {
                    self.stop_importing();
                }
                Some(Task::none())
            }
            Message::CloseRequested(_) => {
                self.stop_importing();
                None
            }
            Message::Viewport(V::Pressed(_) | V::RightClick(_)) => Some(Task::none()),
            Message::Viewport(_)
            | Message::WindowResized(_)
            | Message::LogFrame
            | Message::LogScrolled(_)
            | Message::LayoutSave
            | Message::Modifiers(_)
            | Message::HoverCard(_)
            | Message::TrackDwell(_)
            | Message::Dock(_)
            | Message::BottomTab(_)
            | Message::BottomResized(_)
            | Message::BottomReset
            | Message::RibbonTab(_)
            | Message::LayerExpanded(_)
            | Message::Opened(_)
            | Message::Saved(_)
            | Message::Opening(_)
            | Message::Saving(_)
            | Message::Recovery(_)
            | Message::Cloud(_)
            | Message::ServerChecked(_)
            | Message::Swallowed => None,
            _ => Some(Task::none()),
        }
    }

    /// The panel of an import going in: the file, the objects so far, Durdur.
    pub(crate) fn importing_view(&self) -> Option<Element<'_, Message>> {
        let job = self.importing.as_ref()?;
        let (done, total) = job.work.counts();
        let list = TaskList::new().push(
            Job::new(format!("İçe aktarılıyor: {}", job.name))
                .detail(format!("{} / {} nesne çizime yazıldı", grouped(done as f64), grouped(total as f64)))
                .running(Some(job.work.share()))
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

    pub(super) fn drawing_import_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let srid = self.project_srid();
        let r = s.result.as_deref();
        let meta = match (r, &s.failed) {
            (Some(r), _) => r
                .report
                .source
                .iter()
                .map(|f| format!("{}: {}", f.label, f.value))
                .collect::<Vec<_>>()
                .join(", "),
            (None, Some(_)) => "okunamadı".to_owned(),
            (None, None) => "okunuyor…".to_owned(),
        };
        let mut body = Column::new()
            .spacing(12)
            .push(words::file_line(&s.file.name, meta))
            .push(self.drawing_layers(s))
            .push(self.drawing_summary(s))
            .push(
                s.crs
                    .view(srid, &self.number_format(), |srid| event(Event::Crs(srid))),
            );
        if let Some((error, words)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                words.clone(),
            ));
        }
        let can = self.drawing_can_import(s);
        overlay::blocking(
            Dialog::new(s.source.title())
                // As tall as its content; a long body (a file with many layers) scrolls, the buttons stay in view.
                .scroll(body)
                .action(words::ghost("Başka dosya…", Some(event(Event::Another))))
                .action(words::secondary("Vazgeç", Some(event(Event::Cancel))))
                .action(words::primary("İçe aktar", can.then(|| event(Event::Run))))
                .width(900.0)
                .max_height(820.0),
        )
    }

    fn drawing_layers<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = &s.result else {
            if s.failed.is_some() {
                return words::empty("Katman yok.");
            }
            // The read's bar: a large file takes a moment.
            let share = s.reading.as_ref().map_or(0.0, |(at, _)| *at);
            return column![
                label::body(format!("Dosya okunuyor… %{:.0}", share * 100.0)),
                progress::bar(Some(share)),
            ]
            .spacing(8)
            .width(Fill)
            .into();
        };
        if r.layers.is_empty() {
            return words::empty("Dosyada alınacak nesne yok.");
        }
        let all = match s.excluded.len() {
            0 => Check::Checked,
            n if n == r.layers.len() => Check::Unchecked,
            _ => Check::Mixed,
        };
        let head = row![
            check_box(all, Some(event(Event::ToggleAll))),
            label::caption(format!("Bütün katmanlar ({})", r.layers.len())),
        ]
        .spacing(8)
        .align_y(Center);
        let rows = r.layers.iter().map(|l| {
            let t = self.target(l);
            let checked = !s.excluded.contains(&l.name) && !t.locked;
            let toggle = (!t.locked).then(|| event(Event::Toggle(l.name.clone())));
            let where_to = match (&t.existing, t.locked) {
                (Some(id), true) => format!(
                    "“{}” katmanı kilitli; alınmaz. Kilidini Katmanlar panelinden açın.",
                    self.layer_name(id)
                ),
                (Some(id), false) => format!("“{}” katmanına eklenir", self.layer_path(id)),
                (None, _) => format!(
                    "yeni katman{}{}",
                    if l.visible { "" } else { ", gizli" },
                    if l.locked { ", kilitli" } else { "" }
                ),
            };
            TableRow::new([
                check_box(
                    if checked {
                        Check::Checked
                    } else {
                        Check::Unchecked
                    },
                    toggle,
                ),
                row![swatch(hex_color(&l.color)), label::body(l.name.clone())]
                    .spacing(6)
                    .align_y(Center)
                    .into(),
                label::body(grouped(f64::from(l.count))).into(),
                if t.locked {
                    label::caption(where_to)
                        .style(kentos_ui::style::text::danger)
                        .into()
                } else {
                    label::caption(where_to).into()
                },
            ])
        });
        let table = words::fitted(
            Table::new([
                TableColumn::new("").width(22),
                TableColumn::new(s.source.column()).width(Length::FillPortion(2)),
                TableColumn::new("Nesne").width(60).align_right(),
                TableColumn::new("Nereye").width(Length::FillPortion(3)),
            ])
            .extend(rows),
            r.layers.len(),
        );
        column![head, table].spacing(6).width(Fill).into()
    }

    fn drawing_summary<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(r) = &s.result else {
            return words::summary(vec![match &s.failed {
                Some(e) => words::text_line(Line::Error, e.clone()),
                None => words::text_line(
                    Line::Info,
                    "Dosya okunuyor; büyük dosyalar biraz sürebilir. Vazgeç okumayı durdurur.",
                ),
            }]);
        };
        let included = self.drawing_included(s);
        // Counted from the layers, not the objects: this is drawn every frame.
        let mut counts: Vec<(&str, u32)> = Vec::new();
        for (kind, n) in included.iter().flat_map(|l| &l.kinds) {
            match counts.iter_mut().find(|(k, _)| k == kind) {
                Some((_, m)) => *m += n,
                None => counts.push((kind.as_str(), *n)),
            }
        }
        let total: u32 = counts.iter().map(|(_, n)| n).sum();
        let created = included
            .iter()
            .filter(|l| self.target(l).existing.is_none())
            .count();
        let mut lines = vec![if total > 0 {
            let group = if created > 0 {
                format!(
                    " {created} yeni katman “{}” grubunda kurulacak.",
                    s.file.name
                )
            } else {
                String::new()
            };
            words::text_line(
                Line::Ok,
                format!(
                    "{} nesne alınacak: {}.{group}",
                    grouped(f64::from(total)),
                    words::kind_counts(&counts)
                ),
            )
        } else {
            words::text_line(Line::Warn, "Alınacak nesne yok; en az bir katman seçin.")
        }];
        if total as usize > AT_ONCE {
            lines.push(words::text_line(
                Line::Info,
                "Nesneler çizime parça parça yazılır: pencere kapanır, çizim doldukça görünür, sağ alttaki panel sayar ve Durdur hepsini geri alır.",
            ));
        }
        if let Some(unusable) = &s.unusable {
            lines.push(words::text_line(Line::Error, unusable.clone()));
        }
        lines.extend(words::report_lines(&r.report.notes, Line::Info, 12));
        lines.extend(words::report_lines(&r.report.skipped, Line::Warn, 12));
        if let (Some(b), Some(doc)) = (&r.bounds, &self.document) {
            lines.push(words::text_line(
                Line::Info,
                words::extent_text(&Format::of(doc.settings()), b),
            ));
        }
        words::summary(lines)
    }

    /// The open drawing's coordinate system.
    pub(super) fn project_srid(&self) -> u32 {
        self.document.as_ref().map_or(5256, |d| d.settings().srid)
    }

    pub(super) fn layer_name(&self, id: &str) -> String {
        self.document
            .as_ref()
            .and_then(|d| d.model.layers().get(id))
            .map_or_else(|| id.to_owned(), |n| n.name.clone())
    }

    /// “Grup / Katman” (web `layers.path`).
    pub(super) fn layer_path(&self, id: &str) -> String {
        let Some(doc) = &self.document else {
            return id.to_owned();
        };
        let layers = doc.model.layers();
        let mut names = Vec::new();
        let mut at = layers.get(id);
        let mut current = id.to_owned();
        while let Some(node) = at {
            names.push(node.name.clone());
            let parent = layers.parent(&current).map(|p| p.id.clone());
            at = parent.as_deref().and_then(|p| layers.get(p));
            current = parent.unwrap_or_default();
        }
        names.reverse();
        names.join(" / ")
    }
}
