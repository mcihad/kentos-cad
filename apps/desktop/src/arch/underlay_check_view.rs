//! Denetim of Altlık modeller (docs/adr/8102; plan/bim-plan.md BIM-17, B19): the chosen underlay set beside the
//! drawing's own model by `cad.underlay.check`, off the window's thread, and its answer one table at a time:
//!
//! - **Katlar:** the project's levels and the file's storeys paired by elevation, and the difference; a difference past
//!   the tolerance is marked.
//! - **Akslar:** by label, how far the file's axis stands from the project's and how much it is turned.
//! - **Çakışmalar:** the drawing's object, the file's, how far they overlap and whether one holds the other; a row chosen
//!   selects the drawing's object and shows the file's under Seçilen nesne.
//! - **Yalnız birinde:** what only one model has.
//!
//! A refusal says why (another system or height datum stops it before any geometry); a height datum one side does not
//! say is said above the tables. The check writes nothing.

use std::sync::Arc;

use iced::futures::channel::oneshot;
use iced::widget::{Column, column, row, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CommandResult, ModelUnderlay, UnderlayCheck, UnderlayChecked};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::{ExecutionContext, underlay_check};
use kentos_ui::label;
use kentos_ui::theme::typography::scaled;
use kentos_ui::widget::table::{self, Column as Col, Row, Table};
use kentos_ui::widget::{Form, Segmented, text_field};

use crate::app::{App, Message};
use crate::arch;
use crate::exchange::words::{self, Kind};
use crate::underlays_window::{Event, Window};

/// The button that checks the chosen underlay.
pub const CHECK: &str = "Denetle";
/// The most rows a table shows before it scrolls.
const ROWS: usize = 8;

/// The table shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Levels,
    Axes,
    Clashes,
    Unmatched,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Levels => "Katlar",
            Tab::Axes => "Akslar",
            Tab::Clashes => "Çakışmalar",
            Tab::Unmatched => "Yalnız birinde",
        })
    }
}

/// Where a check stands.
#[derive(Clone, Debug)]
pub enum Outcome {
    Running,
    Done { checked: Arc<UnderlayChecked>, warnings: Vec<String> },
    Refused(String),
}

/// The check's boxes and its last answer.
#[derive(Clone, Debug)]
pub struct Check {
    /// The tolerance as typed, metres.
    pub tolerance: String,
    /// The underlay checked and its answer.
    pub outcome: Option<(String, Outcome)>,
    pub tab: Tab,
    /// The clash chosen in its table.
    pub clash: Option<usize>,
}

impl Default for Check {
    fn default() -> Self {
        Check { tolerance: "0.001".into(), outcome: None, tab: Tab::Levels, clash: None }
    }
}

fn msg(e: Event) -> Message {
    Message::Arch(arch::Event::Underlays(e))
}

/// A length as the tables say it: three decimals, a comma.
fn metres(v: f64) -> String {
    format!("{v:.3}").replace('.', ",")
}

/// A difference with its sign.
fn signed(v: f64) -> String {
    let s = metres(v.abs());
    if v > 0.0005 {
        format!("+{s}")
    } else if v < -0.0005 {
        format!("−{s}")
    } else {
        s
    }
}

/// What one answer says, in a line: how many stand where they should, clash, or only one model has.
pub(crate) fn summary(c: &UnderlayChecked) -> String {
    let levels = c.levels.iter().filter(|l| l.difference.is_some_and(|d| d.abs() > c.tolerance)).count();
    let axes = c
        .axes
        .iter()
        .filter(|a| a.offset.is_some_and(|o| o > c.tolerance) || a.turn.is_some_and(|d| d > 0.0005))
        .count();
    format!(
        "{} eleman iki modelde aynı yerde · {} çakışma · {} eleman yalnız bir modelde · {} kat ve {} aks farklı",
        c.matched,
        c.clashes.len(),
        c.unmatched.len(),
        levels,
        axes
    )
}

impl App {
    /// Denetle: the chosen underlay checked off the window's thread with the tolerance typed.
    pub(crate) fn underlay_check_start(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else { return Task::none() };
        let mut model = doc.model.clone();
        let Some(w) = self.arch.underlays_window.as_mut() else { return Task::none() };
        let Some(id) = w.chosen.clone() else { return Task::none() };
        let typed = w.check.tolerance.trim().replace(',', ".");
        let tolerance = if typed.is_empty() {
            None
        } else {
            match typed.parse::<f64>() {
                Ok(t) if t.is_finite() => Some(t),
                _ => {
                    let why = format!(
                        "Tolerans “{}” bir sayı değil: metre olarak yazın (0.001 bir milimetredir).",
                        w.check.tolerance.trim()
                    );
                    w.check.outcome = Some((id, Outcome::Refused(why)));
                    return Task::none();
                }
            }
        };
        w.check.outcome = Some((id.clone(), Outcome::Running));
        w.check.clash = None;
        let (done, wait) = oneshot::channel();
        let input = UnderlayCheck { id: id.clone(), tolerance };
        // The drawing's model is built as its IFC export builds it: deep, as the export's own thread is.
        let spawned = std::thread::Builder::new().name("altlik-denetimi".into()).stack_size(64 << 20).spawn(move || {
            let _ = done.send(underlay_check::execute(&mut ExecutionContext::new(&mut model), input));
        });
        if spawned.is_err() {
            w.check.outcome = Some((id, Outcome::Refused("Denetim başlatılamadı: iş parçacığı açılmadı.".into())));
            return Task::none();
        }
        Task::perform(wait, move |answer| {
            let outcome = match answer {
                Ok(CommandResult::Completed { output, warnings }) => Outcome::Done {
                    checked: Arc::new(output),
                    warnings: warnings.into_iter().map(|w| w.message).collect(),
                },
                Ok(CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error }) => {
                    Outcome::Refused(error.message)
                }
                Ok(CommandResult::Queued { .. } | CommandResult::Cancelled) | Err(_) => {
                    Outcome::Refused("Denetim yarıda kaldı.".into())
                }
            };
            msg(Event::Checked(id.clone(), Box::new(outcome)))
        })
    }

    /// A check's answer came: kept while the window waits for it.
    pub(crate) fn underlay_checked(&mut self, id: String, outcome: Outcome) {
        if let Some(w) = self.arch.underlays_window.as_mut()
            && w.check.outcome.as_ref().is_some_and(|(i, o)| *i == id && matches!(o, Outcome::Running))
        {
            if let Outcome::Done { checked, .. } = &outcome {
                // The table worth looking at first: the clashes when there are some.
                w.check.tab = if checked.clashes.is_empty() { Tab::Levels } else { Tab::Clashes };
            }
            w.check.outcome = Some((id, outcome));
        }
    }

    /// A clash chosen: the drawing's object selected and shown, the file's object under Seçilen nesne.
    pub(crate) fn underlay_clash_chosen(&mut self, k: usize) {
        let Some(w) = self.arch.underlays_window.as_mut() else { return };
        let Some((id, Outcome::Done { checked, .. })) = w.check.outcome.clone() else { return };
        let Some(c) = checked.clashes.get(k) else { return };
        w.check.clash = Some(k);
        let object = match self.arch.underlays.states.get(&id).map(|s| &s.1) {
            Some(crate::underlays::State::Ready(l)) => l.model.objects.iter().position(|o| o.global_id == c.object),
            _ => None,
        };
        if let Some(w) = self.arch.underlays_window.as_mut() {
            w.object = object.map(|o| (id.clone(), o));
        }
        let Some(doc) = self.document.as_ref() else { return };
        let slots: Vec<Slot> = Uuid::parse_str(&c.uid).ok().and_then(|u| doc.model.slot_of(u)).into_iter().collect();
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if !ids.is_empty() {
            self.navigating(|app| app.zoom_to_objects(&ids));
        }
    }

    /// The Denetim section of the chosen underlay: the tolerance, Denetle, and the answer.
    pub(crate) fn underlay_check_section<'a>(&'a self, w: &'a Window, u: &ModelUnderlay) -> Element<'a, Message> {
        let mut form = Form::new()
            .label_width(scaled(110.0))
            .section("Denetim")
            .field(
                "Tolerans",
                row![
                    text_field("0.001", &w.check.tolerance).on_input(|t| msg(Event::Tolerance(t))).width(scaled(90.0)),
                    label::body("m"),
                    space::horizontal().width(scaled(8.0)),
                    words::primary(CHECK, Some(msg(Event::Check))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        let answer = w.check.outcome.as_ref().filter(|(id, _)| *id == u.id).map(|(_, o)| o);
        match answer {
            None => {
                form = form.wide(label::caption(
                    "Katları, aksları ve elemanları çizimin kendi modeliyle karşılaştırır; toleranstan az örtüşme ve \
                     kayma bulgu sayılmaz. Çizime bir şey yazmaz.",
                ));
            }
            Some(Outcome::Running) => form = form.wide(words::text_line(Kind::Info, "Denetleniyor…")),
            Some(Outcome::Refused(why)) => form = form.wide(words::text_line(Kind::Warn, why.clone())),
            Some(Outcome::Done { checked, warnings }) => {
                let kind = if checked.clashes.is_empty() && checked.unmatched.is_empty() { Kind::Ok } else { Kind::Warn };
                form = form.wide(words::text_line(kind, summary(checked)));
                for why in warnings {
                    form = form.wide(words::text_line(Kind::Warn, why.clone()));
                }
                form = form.wide(Segmented::new(
                    [Tab::Levels, Tab::Axes, Tab::Clashes, Tab::Unmatched],
                    w.check.tab,
                    |t| msg(Event::Tab(t)),
                ));
                form = form.wide(match w.check.tab {
                    Tab::Levels => levels_table(checked),
                    Tab::Axes => axes_table(checked),
                    Tab::Clashes => clashes_table(checked, w.check.clash),
                    Tab::Unmatched => unmatched_table(checked),
                });
            }
        }
        Column::new().push(form).into()
    }
}

/// A table's height for `rows` rows, at most [`ROWS`] before it scrolls.
fn height(rows: usize) -> f32 {
    (rows.clamp(1, ROWS) as f32 + 1.0) * table::row_height() + 2.0
}

fn levels_table(c: &UnderlayChecked) -> Element<'_, Message> {
    let mut t = Table::new([
        Col::new("Projenin katı").width(Fill),
        Col::new("Kotu").width(scaled(80.0)).align_right(),
        Col::new("Dosyanın katı").width(Fill),
        Col::new("Kotu").width(scaled(80.0)).align_right(),
        Col::new("Fark").width(scaled(80.0)).align_right(),
        Col::new("Durum").width(scaled(130.0)),
    ])
    .empty("Ne projede ne dosyada kat var.");
    let dash = || "—".to_owned();
    for l in &c.levels {
        let state = match l.difference {
            Some(d) if d.abs() > c.tolerance => words::text_line(Kind::Warn, "farklı"),
            Some(_) => words::text_line(Kind::Ok, "aynı"),
            None if l.storey.is_some() && l.file.is_none() => words::text_line(Kind::Info, "kotu yazılmamış"),
            None => words::text_line(Kind::Info, "eşi yok"),
        };
        t = t.push(Row::new([
            table::text(l.level.clone().unwrap_or_else(dash)).into(),
            table::number(l.project.map_or_else(dash, metres)).into(),
            table::text(l.storey.clone().unwrap_or_else(dash)).into(),
            table::number(l.file.map_or_else(dash, metres)).into(),
            table::number(l.difference.map_or_else(dash, signed)).into(),
            state,
        ]));
    }
    t.height(height(c.levels.len())).into()
}

fn axes_table(c: &UnderlayChecked) -> Element<'_, Message> {
    let mut t = Table::new([
        Col::new("Aks").width(Fill),
        Col::new("Projede").width(scaled(80.0)),
        Col::new("Dosyada").width(scaled(80.0)),
        Col::new("Uzaklık").width(scaled(90.0)).align_right(),
        Col::new("Dönme").width(scaled(90.0)).align_right(),
        Col::new("Durum").width(scaled(130.0)),
    ])
    .empty("Ne projede ne dosyada aks var.");
    let yes = |b: bool| if b { "var" } else { "yok" };
    for a in &c.axes {
        let turn = a.turn.map_or_else(|| "—".to_owned(), |d| format!("{d:.3}°").replace('.', ","));
        // An axis turned is off along its length even where its middle is on the line.
        let off = a.offset.is_some_and(|o| o > c.tolerance) || a.turn.is_some_and(|d| d > 0.0005);
        let state = match a.offset {
            Some(_) if off => words::text_line(Kind::Warn, "farklı"),
            Some(_) => words::text_line(Kind::Ok, "aynı"),
            None => words::text_line(Kind::Info, "eşi yok"),
        };
        t = t.push(Row::new([
            table::text(a.label.clone()).into(),
            table::text(yes(a.in_project)).into(),
            table::text(yes(a.in_file)).into(),
            table::number(a.offset.map_or_else(|| "—".to_owned(), metres)).into(),
            table::number(turn).into(),
            state,
        ]));
    }
    t.height(height(c.axes.len())).into()
}

/// An object as a row says it: its class and its name.
fn object(class: &str, name: Option<&str>) -> String {
    match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => format!("{class} {n}"),
        None => class.to_owned(),
    }
}

fn clashes_table(c: &UnderlayChecked, chosen: Option<usize>) -> Element<'_, Message> {
    let mut t = Table::new([
        Col::new("Çizimdeki nesne").width(Fill),
        Col::new("Dosyadaki nesne").width(Fill),
        Col::new("Örtüşme (m)").width(scaled(150.0)),
        Col::new("İçinde").width(scaled(60.0)),
    ])
    .empty("Çakışma yok.");
    for (k, x) in c.clashes.iter().enumerate() {
        let o = &x.overlap;
        let size = if o.len() == 6 {
            format!("{} × {} × {}", metres(o[3] - o[0]), metres(o[4] - o[1]), metres(o[5] - o[2]))
        } else {
            "—".to_owned()
        };
        t = t.push(
            Row::new([
                table::text(object(&x.class, Some(&x.name))).into(),
                table::text(object(&x.object_class, x.object_name.as_deref())).into(),
                table::number(size).into(),
                table::text(if x.inside { "evet" } else { "" }).into(),
            ])
            .selected(chosen == Some(k))
            .on_press(msg(Event::Clash(k))),
        );
    }
    let lead = label::caption("Bir satır çizimdeki nesneyi seçer, dosyadakini Seçilen nesne'de gösterir.");
    column![lead, t.height(height(c.clashes.len()))].spacing(6).into()
}

fn unmatched_table(c: &UnderlayChecked) -> Element<'_, Message> {
    let mut t = Table::new([
        Col::new("Model").width(scaled(70.0)),
        Col::new("Sınıf").width(scaled(150.0)),
        Col::new("Ad").width(Fill),
    ])
    .empty("Dosyanın sınıflarından yalnız bir modelde olan eleman yok.");
    for x in &c.unmatched {
        let side = if x.uid.is_some() { "Çizim" } else { "Dosya" };
        t = t.push(Row::new([
            table::text(side).into(),
            table::text(x.class.clone()).into(),
            table::text(x.name.clone()).into(),
        ]));
    }
    t.height(height(c.unmatched.len())).into()
}
