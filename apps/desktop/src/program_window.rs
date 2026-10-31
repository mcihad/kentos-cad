//! Bina programı (docs/adr/0773, `arch.program`): the building's groups of spaces and the design brief, and the
//! program read against the drawing.
//!
//! ```text
//! ┌ Bina programı ───────────────────────────────────────────────────────────────────────────┐
//! │ [Bölümler | Hedefler | Durum]                         Hedeflerin sürümü [Ofis 2026-1   ] │
//! │ No [1 ] Ad [Daire 1     ] Tür [Bağımsız bölüm ▾] Kayıtlı alan [60   ] 2 kat · 5 mahal … │
//! │ … [Seçilenleri ver] [Sil]                                                                │
//! │ [Bölüm ekle]                                                                             │
//! │ Hedefler: Kullanım, En az, En çok, Net en, Geçiş, Komşu, Ulaşım, Dış                     │
//! │ Durum: Toplamlar; mahaller (alan, hedef, fark, net en, geçiş, kapı); bulgular             │
//! │ Denetleniyor… %40  [Durdur]                                        [Vazgeç] [Kaydet]    │
//! └──────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the program; Kaydet writes it through `cad.program.edit`, Seçilenleri ver gives the
//! selected spaces to a group at once (one undo step each). Denetle reads the program against the drawing through
//! `cad.program.check` on a reading copy on a thread of its own: its progress is said, Durdur stops it at once. The
//! targets are the designer's brief, never a legal limit.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::futures::channel::mpsc;
use iced::widget::{Column, column, container, row, scrollable, space, text};
use iced::{Center, Element, Fill, Length, Task};
use kentos_commands::{
    BuildingProgram, CommandResult, ProgramAssign, ProgramCheck, ProgramChecked, ProgramEdit, ProgramUnit, SpaceBrief,
    UnitKind,
};
use kentos_native_application::{ExecutionContext, program_check, program_edit};
use kentos_ui::label;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::segmented::Segmented;

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Bina programı";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const CHECK: &str = "Denetle";
const STOP: &str = "Durdur";
const ADD_UNIT: &str = "Bölüm ekle";
const ADD_BRIEF: &str = "Hedef ekle";
const GIVE: &str = "Seçilenleri ver";

/// The part shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Units,
    Briefs,
    Status,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Units => "Bölümler",
            Tab::Briefs => "Hedefler",
            Tab::Status => "Durum",
        })
    }
}

/// A group being edited: its number and recorded area as typed.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitRow {
    unit: ProgramUnit,
    name: String,
    area: String,
}

/// A brief being edited: its numbers and lists as typed.
#[derive(Debug, Clone, PartialEq)]
pub struct BriefRow {
    brief: SpaceBrief,
    texts: [String; 6],
}

fn number_text(v: Option<f64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

impl BriefRow {
    fn of(brief: SpaceBrief) -> Self {
        let texts = [
            number_text(brief.area_min),
            number_text(brief.area_max),
            number_text(brief.min_width),
            number_text(brief.min_passage),
            brief.adjacent.join(", "),
            brief.reach.join(", "),
        ];
        Self { brief, texts }
    }
}

/// A check running on its own thread.
#[derive(Debug, Clone)]
pub struct Running {
    id: u64,
    stop: Arc<AtomicBool>,
    done: u64,
    total: u64,
}

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    tab: Tab,
    units: Vec<UnitRow>,
    briefs: Vec<BriefRow>,
    version: String,
    /// The rest of the program as the project has it (buildings, clearances).
    kept: BuildingProgram,
    checked: Option<Arc<ProgramChecked>>,
    running: Option<Running>,
    next: u64,
    said: Option<(Kind, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Tab(Tab),
    Version(String),
    UnitNumber(usize, String),
    UnitName(usize, String),
    UnitKind(usize, UnitKind),
    UnitArea(usize, String),
    AddUnit,
    RemoveUnit(usize),
    Give(usize),
    BriefUsage(usize, String),
    BriefText(usize, usize, String),
    BriefOutdoor(usize),
    AddBrief,
    RemoveBrief(usize),
    Check,
    Stop,
    Progress(u64, u64, u64),
    Checked(u64, Arc<CommandResult<ProgramChecked>>),
    Cancel,
    Save,
}

fn msg(event: Event) -> Message {
    Message::Program(event)
}

fn number(t: &str, what: &str) -> Result<Option<f64>, String> {
    let t = t.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| format!("{what} bir sayı olmalı: “{t}”."))
}

fn usages(t: &str) -> Vec<String> {
    t.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect()
}

impl Window {
    fn of(program: &BuildingProgram) -> Self {
        Self {
            units: program
                .units
                .iter()
                .map(|u| UnitRow {
                    unit: u.clone(),
                    name: u.name.clone().unwrap_or_default(),
                    area: number_text(u.recorded_area),
                })
                .collect(),
            briefs: program.briefs.iter().cloned().map(BriefRow::of).collect(),
            version: program.version.clone().unwrap_or_default(),
            kept: program.clone(),
            ..Self::default()
        }
    }

    /// The program as typed; the words of what does not read.
    fn program(&self) -> Result<BuildingProgram, String> {
        let mut p = self.kept.clone();
        p.units = Vec::with_capacity(self.units.len());
        for r in &self.units {
            let mut u = r.unit.clone();
            let name = r.name.trim();
            u.name = (!name.is_empty()).then(|| name.to_owned());
            u.recorded_area = number(&r.area, "Kayıtlı alan")?;
            p.units.push(u);
        }
        p.briefs = Vec::with_capacity(self.briefs.len());
        for r in &self.briefs {
            let mut b = r.brief.clone();
            b.area_min = number(&r.texts[0], "En az alan")?;
            b.area_max = number(&r.texts[1], "En çok alan")?;
            b.min_width = number(&r.texts[2], "Net en")?;
            b.min_passage = number(&r.texts[3], "Net geçiş")?;
            b.adjacent = usages(&r.texts[4]);
            b.reach = usages(&r.texts[5]);
            p.briefs.push(b);
        }
        let v = self.version.trim();
        p.version = (!v.is_empty()).then(|| v.to_owned());
        Ok(p)
    }
}

/// A new id not among `taken`.
fn fresh_id(base: &str, taken: &[&str]) -> String {
    (1..)
        .map(|n| format!("{base}{n}"))
        .find(|id| !taken.contains(&id.as_str()))
        .unwrap_or_else(|| base.to_owned())
}

impl App {
    /// Bina programı: the window on the project's program.
    pub(crate) fn open_program(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        self.program_window = Some(Window::of(&doc.model.program()));
        self.dialog = Some(Dialog::Program);
    }

    pub(crate) fn program_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.program_window.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Tab(t) => w.tab = t,
            Event::Version(t) => w.version = t,
            Event::UnitNumber(i, t) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.unit.number = t;
                }
            }
            Event::UnitName(i, t) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.name = t;
                }
            }
            Event::UnitKind(i, k) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.unit.kind = k;
                }
            }
            Event::UnitArea(i, t) => {
                if let Some(r) = w.units.get_mut(i) {
                    r.area = t;
                }
            }
            Event::AddUnit => {
                let taken: Vec<&str> = w.units.iter().map(|r| r.unit.id.as_str()).collect();
                let id = fresh_id("bolum-", &taken);
                let numbers: Vec<&str> = w.units.iter().map(|r| r.unit.number.as_str()).collect();
                let number = (1..).map(|n| n.to_string()).find(|n| !numbers.contains(&n.as_str())).unwrap_or_default();
                w.units.push(UnitRow {
                    unit: ProgramUnit { id, number, ..Default::default() },
                    name: String::new(),
                    area: String::new(),
                });
            }
            Event::RemoveUnit(i) => {
                if i < w.units.len() {
                    w.units.remove(i);
                }
            }
            Event::Give(i) => return self.give_to_unit(i),
            Event::BriefUsage(i, t) => {
                if let Some(r) = w.briefs.get_mut(i) {
                    r.brief.usage = t;
                }
            }
            Event::BriefText(i, k, t) => {
                if let Some(slot) = w.briefs.get_mut(i).and_then(|r| r.texts.get_mut(k)) {
                    *slot = t;
                }
            }
            Event::BriefOutdoor(i) => {
                if let Some(r) = w.briefs.get_mut(i) {
                    r.brief.outdoor = !r.brief.outdoor;
                }
            }
            Event::AddBrief => {
                w.briefs.push(BriefRow::of(SpaceBrief { usage: "Yeni kullanım".into(), ..Default::default() }))
            }
            Event::RemoveBrief(i) => {
                if i < w.briefs.len() {
                    w.briefs.remove(i);
                }
            }
            Event::Check => return self.check_program(),
            Event::Stop => {
                if let Some(r) = w.running.take() {
                    r.stop.store(true, Ordering::Relaxed);
                    w.said = Some((Kind::Info, "Denetim durduruldu; sonuç yok.".into()));
                }
            }
            Event::Progress(id, done, total) => {
                if let Some(r) = w.running.as_mut().filter(|r| r.id == id) {
                    (r.done, r.total) = (done, total);
                }
            }
            Event::Checked(id, result) => {
                if w.running.as_ref().is_none_or(|r| r.id != id) {
                    return Task::none();
                }
                w.running = None;
                match &*result {
                    CommandResult::Completed { output, .. } => {
                        let n = output.findings.len();
                        w.said = Some(if n == 0 {
                            (Kind::Ok, "Tasarım hedeflerinden sapma yok.".into())
                        } else {
                            (Kind::Warn, format!("{n} bulgu: tasarım hedefi, yasal sınır değil."))
                        });
                        w.checked = Some(Arc::new(output.clone()));
                        w.tab = Tab::Status;
                    }
                    CommandResult::Cancelled => w.said = Some((Kind::Info, "Denetim durduruldu.".into())),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        w.said = Some((Kind::Error, error.message.clone()))
                    }
                    CommandResult::Queued { .. } => {}
                }
            }
            Event::Cancel => {
                if let Some(r) = w.running.take() {
                    r.stop.store(true, Ordering::Relaxed);
                }
                self.program_window = None;
                self.dialog = None;
            }
            Event::Save => return self.save_program(),
        }
        Task::none()
    }

    /// Kaydet: the program as typed through `cad.program.edit`; the window stays open on what was written.
    fn save_program(&mut self) -> Task<Message> {
        let Some(w) = &self.program_window else {
            return Task::none();
        };
        let program = match w.program() {
            Ok(p) => p,
            Err(why) => {
                self.warn(why);
                return Task::none();
            }
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let input = ProgramEdit { program: Some(program), ..Default::default() };
        let result = program_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.output("Bina programı yazıldı.");
            let tab = self.program_window.as_ref().map(|w| w.tab).unwrap_or_default();
            let mut fresh = Window::of(&out.program);
            fresh.tab = tab;
            self.program_window = Some(fresh);
        }
        Task::none()
    }

    /// Seçilenleri ver: the selected spaces given to the group of row `i` (the program written first).
    fn give_to_unit(&mut self, i: usize) -> Task<Message> {
        let Some(w) = &self.program_window else {
            return Task::none();
        };
        let program = match w.program() {
            Ok(p) => p,
            Err(why) => {
                self.warn(why);
                return Task::none();
            }
        };
        let Some(unit) = program.units.get(i).map(|u| u.id.clone()) else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let uids: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter(|s| matches!(doc.model.get(**s), Some(kentos_commands::Entity::Space(_))))
            .filter_map(|s| doc.model.uid(*s).map(|u| u.to_string()))
            .collect();
        if uids.is_empty() {
            self.warn("Seçili mahal yok: önce mahalleri seçin, sonra Seçilenleri ver'e basın.");
            return Task::none();
        }
        let n = uids.len();
        let input = ProgramEdit { program: Some(program), assign: vec![ProgramAssign { uids, unit: Some(unit) }], ..Default::default() };
        let result = program_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if let Some(out) = self.answered(result) {
            self.output(format!("{n} mahal bölüme verildi."));
            let tab = self.program_window.as_ref().map(|w| w.tab).unwrap_or_default();
            let mut fresh = Window::of(&out.program);
            fresh.tab = tab;
            self.program_window = Some(fresh);
        }
        Task::none()
    }

    /// Denetle: `cad.program.check` on the drawing's reading copy, on a thread of its own.
    fn check_program(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let copy = doc.model.reading_copy();
        let Some(w) = self.program_window.as_mut() else {
            return Task::none();
        };
        if let Some(r) = w.running.take() {
            r.stop.store(true, Ordering::Relaxed);
        }
        w.next += 1;
        let id = w.next;
        let stop = Arc::new(AtomicBool::new(false));
        w.running = Some(Running { id, stop: stop.clone(), done: 0, total: 0 });
        w.said = None;
        let (out, replies) = mpsc::unbounded();
        let spawned = std::thread::Builder::new().name("kentos-program".into()).spawn(move || {
            let tell = out.clone();
            let stopped = move |done: u64, total: u64| {
                if done.is_multiple_of(8) || done == total {
                    let _ = tell.unbounded_send(Event::Progress(id, done, total));
                }
                stop.load(Ordering::Relaxed)
            };
            let result = program_check::check_watched(&copy, &ProgramCheck::default(), &stopped);
            let _ = out.unbounded_send(Event::Checked(id, Arc::new(result)));
        });
        match spawned {
            Ok(_) => Task::run(replies, msg),
            Err(e) => Task::done(msg(Event::Checked(
                id,
                Arc::new(CommandResult::Failed {
                    error: kentos_commands::CommandError {
                        code: "host".into(),
                        message: format!("Denetim arka plana gönderilemedi: {e}"),
                        ..Default::default()
                    },
                }),
            ))),
        }
    }

    pub(crate) fn program_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.program_window else {
            return space::horizontal().into();
        };
        let tabs = Segmented::new([Tab::Units, Tab::Briefs, Tab::Status], w.tab, |t| msg(Event::Tab(t)));
        let version = kentos_ui::widget::text_field("Hedeflerin sürümü", &w.version)
            .on_input(|t| msg(Event::Version(t)))
            .width(260);
        let head = row![
            words::field("Gösterilen", tabs, None),
            space::horizontal().width(Fill),
            words::field("Hedeflerin sürümü", version, None),
        ]
        .spacing(16)
        .align_y(iced::Bottom);
        let body: Element<'_, Message> = match w.tab {
            Tab::Units => self.program_units_view(w),
            Tab::Briefs => program_briefs_view(w),
            Tab::Status => program_status_view(w, self),
        };
        let mut foot = row![].spacing(8).align_y(Center);
        match &w.running {
            Some(r) => {
                let share = (r.done * 100).checked_div(r.total).unwrap_or(0);
                foot = foot
                    .push(words::text_line(Kind::Info, format!("Denetleniyor… %{share}")))
                    .push(words::secondary(STOP, Some(msg(Event::Stop))));
            }
            None => {
                foot = foot.push(words::secondary(CHECK, Some(msg(Event::Check))));
                if let Some((kind, said)) = &w.said {
                    foot = foot.push(container(words::text_line(*kind, said.clone())).width(Length::Shrink));
                }
            }
        }
        let foot = foot
            .push(space::horizontal().width(Fill))
            .push(words::secondary(CANCEL, Some(msg(Event::Cancel))))
            .push(words::primary(SAVE, Some(msg(Event::Save))));
        let content = column![head, scrollable(body).height(Length::Fixed(380.0)), foot].spacing(14);
        Frame::new(TITLE).push(content).width(1000.0).modal(msg(Event::Cancel))
    }

    fn program_units_view<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let checked = w.checked.as_deref();
        let level_name = |id: &str| -> String {
            self.document
                .as_ref()
                .and_then(|d| d.model.settings().arch.as_ref())
                .and_then(|a| a.levels.iter().find(|l| l.id == id).map(|l| l.name.clone()))
                .unwrap_or_else(|| id.to_owned())
        };
        let mut list = Column::new().spacing(10);
        for (i, r) in w.units.iter().enumerate() {
            let kinds = UnitKind::ALL;
            let kind = Select::new(
                kinds.iter().map(|k| Choice::new(k.label())),
                kinds.iter().position(|k| *k == r.unit.kind),
                move |k| msg(Event::UnitKind(i, kinds[k])),
            )
            .searchable(false);
            let fields = row![
                words::field(
                    "No",
                    kentos_ui::widget::text_field("1", &r.unit.number).on_input(move |t| msg(Event::UnitNumber(i, t))).width(70),
                    None
                ),
                words::field(
                    "Ad",
                    kentos_ui::widget::text_field("Daire 1", &r.name).on_input(move |t| msg(Event::UnitName(i, t))).width(200),
                    None
                ),
                words::field("Tür", container(kind).width(170), None),
                words::field(
                    "Kayıtlı alan (m²)",
                    kentos_ui::widget::text_field("Tapu, ruhsat", &r.area).on_input(move |t| msg(Event::UnitArea(i, t))).width(130),
                    None
                ),
                space::horizontal().width(Fill),
                words::secondary(GIVE, Some(msg(Event::Give(i)))),
                words::secondary("Sil", Some(msg(Event::RemoveUnit(i)))),
            ]
            .spacing(10)
            .align_y(iced::Bottom);
            let mut block = column![fields].spacing(4);
            if let Some(u) = checked.and_then(|c| c.units.iter().find(|u| u.id == r.unit.id)) {
                let levels: Vec<String> = u.levels.iter().map(|l| level_name(l)).collect();
                let methods: Vec<&str> = u.methods.iter().map(|m| m.label()).collect();
                let mut words_ = format!(
                    "{} mahal · odalar {} m² · dış {} m² · şaft {} m²",
                    u.spaces,
                    fixed2(u.rooms),
                    fixed2(u.outdoor),
                    fixed2(u.voids)
                );
                if !levels.is_empty() {
                    words_.push_str(&format!(" · {}", levels.join(" + ")));
                }
                if !methods.is_empty() {
                    words_.push_str(&format!(" · {}", methods.join(", ")));
                }
                if let (Some(a), Some(d)) = (u.recorded_area, u.recorded_difference) {
                    words_.push_str(&format!(" · kayıtlı {} m², fark {} m²", fixed2(a), fixed2(d)));
                }
                block = block.push(label::caption(words_));
            }
            list = list.push(block);
        }
        list = list.push(row![words::secondary(ADD_UNIT, Some(msg(Event::AddUnit)))]);
        if let Some(c) = checked {
            let t = &c.totals;
            list = list.push(words::summary(vec![words::text_line(
                Kind::Info,
                format!(
                    "Bağımsız bölümler {} m² (dış {} m²) · ortak {} m² · dış alan {} m² · şaft {} m² · bölümsüz {} m² · hepsi {} m²; her mahal bir kez sayılır.",
                    fixed2(t.units),
                    fixed2(t.unit_outdoor),
                    fixed2(t.common),
                    fixed2(t.outdoor),
                    fixed2(t.voids),
                    fixed2(t.unassigned),
                    fixed2(t.all)
                ),
            )]));
        }
        list.into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn program_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.program_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(CHECK) => Some(msg(Event::Check)),
            Control::Press(STOP) => Some(msg(Event::Stop)),
            Control::Press(ADD_UNIT) => Some(msg(Event::AddUnit)),
            Control::Press(ADD_BRIEF) => Some(msg(Event::AddBrief)),
            Control::Press("Bölümler") => Some(msg(Event::Tab(Tab::Units))),
            Control::Press("Hedefler") => Some(msg(Event::Tab(Tab::Briefs))),
            Control::Press("Durum") => Some(msg(Event::Tab(Tab::Status))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

fn fixed2(v: f64) -> String {
    kentos_geometry_core::display::fixed(v, 2)
}

fn program_briefs_view(w: &Window) -> Element<'_, Message> {
    let mut list = Column::new().spacing(10);
    for (i, r) in w.briefs.iter().enumerate() {
        let field = |title: &'static str, k: usize, place: &'static str, width: f32| {
            words::field(
                title,
                kentos_ui::widget::text_field(place, &r.texts[k]).on_input(move |t| msg(Event::BriefText(i, k, t))).width(width),
                None,
            )
        };
        let outdoor = words::check(r.brief.outdoor, "Dış mahal (balkon, teras)", Some(msg(Event::BriefOutdoor(i))));
        let top = row![
            words::field(
                "Kullanım",
                kentos_ui::widget::text_field("Yatak odası", &r.brief.usage).on_input(move |t| msg(Event::BriefUsage(i, t))).width(170),
                None
            ),
            field("En az alan (m²)", 0, "12", 110.0),
            field("En çok alan (m²)", 1, "16", 110.0),
            field("Net en (m)", 2, "2.8", 90.0),
            field("Net geçiş (m)", 3, "1.2", 90.0),
            space::horizontal().width(Fill),
            words::secondary("Sil", Some(msg(Event::RemoveBrief(i)))),
        ]
        .spacing(10)
        .align_y(iced::Bottom);
        let bottom = row![
            field("Komşu kullanımlar (virgülle)", 4, "Mutfak", 260.0),
            field("Geçilen kullanımlar (virgülle)", 5, "Salon", 260.0),
            container(outdoor).padding([6, 0]),
        ]
        .spacing(10)
        .align_y(iced::Bottom);
        list = list.push(column![top, bottom].spacing(6));
    }
    list = list.push(row![words::secondary(ADD_BRIEF, Some(msg(Event::AddBrief)))]);
    list = list.push(label::caption(
        "Hedefler tasarımcının kendi brief'idir; yönetmelik ya da yasal sınır değildir. Bulgular hedeflerin sürümünü ve ölçüm kuralını söyler.",
    ));
    list.into()
}

fn cell<'a>(t: impl Into<String>, width: f32) -> Element<'a, Message> {
    container(text(t.into()).size(kentos_ui::theme::typography::body())).width(width).into()
}

fn program_status_view<'a>(w: &'a Window, app: &'a App) -> Element<'a, Message> {
    let Some(c) = w.checked.as_deref() else {
        return label::body("Henüz denetlenmedi: Denetle'ye basın.").into();
    };
    let unit_no = |id: &Option<String>| -> String {
        id.as_deref()
            .map(|u| c.units.iter().find(|x| x.id == u).map_or_else(|| format!("{u}?"), |x| x.number.clone()))
            .unwrap_or_else(|| "—".into())
    };
    let level = |id: &Option<String>| -> String {
        id.as_deref()
            .and_then(|l| {
                app.document
                    .as_ref()
                    .and_then(|d| d.model.settings().arch.as_ref())
                    .and_then(|a| a.levels.iter().find(|x| x.id == l).map(|x| x.name.clone()))
            })
            .unwrap_or_else(|| "—".into())
    };
    let heads = [
        ("No", 60.0),
        ("Ad", 140.0),
        ("Bölüm", 60.0),
        ("Kat", 90.0),
        ("Yöntem", 120.0),
        ("Alan m²", 80.0),
        ("Hedef m²", 110.0),
        ("Fark m²", 80.0),
        ("Net en", 70.0),
        ("Geçiş", 70.0),
        ("Kapı", 50.0),
    ];
    let mut list = Column::new().spacing(4);
    list = list.push(
        heads
            .iter()
            .fold(row![].spacing(6), |r, (h, wd)| r.push(container(label::caption(*h)).width(*wd))),
    );
    for s in &c.spaces {
        let target = match (s.target_min, s.target_max) {
            (Some(a), Some(b)) => format!("{}–{}", fixed2(a), fixed2(b)),
            (Some(a), None) => format!("≥ {}", fixed2(a)),
            (None, Some(b)) => format!("≤ {}", fixed2(b)),
            (None, None) => "—".into(),
        };
        let diff = s.target_difference.map_or_else(|| "—".into(), |d| if d > 0.0 { format!("+{}", fixed2(d)) } else { fixed2(d) });
        let method = if s.void { "Şaft (boşluk)".to_owned() } else { s.method.label().to_owned() };
        let r = row![
            cell(s.number.clone().unwrap_or_default(), 60.0),
            cell(s.name.clone().or(s.usage.clone()).unwrap_or_default(), 140.0),
            cell(unit_no(&s.unit), 60.0),
            cell(level(&s.level), 90.0),
            cell(method, 120.0),
            cell(fixed2(s.area), 80.0),
            cell(target, 110.0),
            cell(diff, 80.0),
            cell(fixed2(s.width), 70.0),
            cell(s.passage.map_or_else(|| "—".into(), fixed2), 70.0),
            cell(s.doors.to_string(), 50.0),
        ]
        .spacing(6);
        list = list.push(r);
    }
    let mut found = Column::new().spacing(4);
    for f in &c.findings {
        found = found.push(words::text_line(Kind::Warn, f.message.clone()));
    }
    column![
        list,
        label::caption(format!("Kural: {}", c.rule)),
        found,
    ]
    .spacing(12)
    .into()
}
