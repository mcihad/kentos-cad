//! Dever (plan/road.md Y6, `superelevation.editor`): a road project's superelevation schedules, as Civil 3D's
//! superelevation view and OpenRoads' superelevation editor show them: on an alignment, the normal cross slopes, then
//! each curve's rate, runoff, share on the tangent and inside widening (rotating about the centreline), or rows of
//! stations; the diagram of both sides' slopes with the critical stations (NC, LC, RC, FS) and the widening, built as
//! it is typed (`kentos.superelevation/1`), so why it does not build shows before it is saved.
//!
//! ```text
//! ┌ Dever ──────────────────────────────────────────────────────────────────────────────────────┐
//! │ Dever çizelgeleri   │ Ad [Ana aks deveri] Aks [Ana aks ▾] Normal sol [-2,5] sağ [-2,5] [Kurplardan ▾]│
//! │ ● Ana aks deveri    │ ┌───────────────────────────────────────────────────────────────────┐ │
//! │ [Yeni dever]        │ │  %6 ─────/‾‾‾‾‾\──── left blue, right red, NC LC RC FS, widening │ │
//! │                     │ └───────────────────────────────────────────────────────────────────┘ │
//! │                     │ PI   Oran (%)  Geçiş boyu (m)  Teğet payı  Genişleme (m)               │
//! │                     │ [Kurp ekle] [Kurbu sil] [Bütün kurplar]                                 │
//! │ [Sil] [Aksa yeniden bağla]                                               [Vazgeç] [Kaydet] │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the schedules; Kaydet writes what changed through `road.superelevations.edit` (removes,
//! then sets), each one undo step, and closes. The rates, runoffs and widths come from the standard or the project:
//! nothing is filled in for the user.

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Center, Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    RoadEditOperation, SuperCurveDef, SuperRowDef, SuperelevationDef, SuperelevationMethod, SuperelevationReport,
    SuperelevationsEdit,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_superelevations, road_superelevations_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Dever";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni dever";
const REMOVE: &str = "Sil";
const REBIND: &str = "Aksa yeniden bağla";
const ADD: &str = "Satır ekle";
const REMOVE_ROW: &str = "Satırı sil";
const ALL_CURVES: &str = "Bütün kurplar";
const METHODS: [&str; 2] = ["Kurplardan", "Km satırlarından"];

fn comma(v: f64, digits: usize) -> String {
    format!("{v:.digits$}").replace('.', ",")
}

fn trimmed(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn number(text: &str) -> Result<Option<f64>, ()> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| ())
}

fn need(text: &str, what: &str) -> Result<f64, String> {
    match number(text) {
        Ok(Some(v)) => Ok(v),
        _ => Err(format!("{what} bir sayı olmalı.")),
    }
}

/// A schedule as typed: its curves' rows (PI, rate, runoff, share, widening) or its station rows (station, left,
/// right), as text.
#[derive(Debug, Clone, Default)]
pub struct Row {
    pub(crate) id: String,
    pub(crate) name: String,
    revision: u32,
    alignment: String,
    alignment_revision: u32,
    normal: [String; 2],
    /// 0 from the curves, 1 from rows.
    method: usize,
    curves: Vec<[String; 5]>,
    rows: Vec<[String; 3]>,
}

impl Row {
    fn of(d: &SuperelevationDef) -> Self {
        let mut r = Self {
            id: d.id.clone(),
            name: d.name.clone(),
            revision: d.revision,
            alignment: d.alignment.clone(),
            alignment_revision: d.alignment_revision,
            normal: [trimmed(d.normal_left), trimmed(d.normal_right)],
            ..Self::default()
        };
        match &d.method {
            SuperelevationMethod::Curves { curves } => {
                r.curves = curves
                    .iter()
                    .map(|c| {
                        [c.pi.clone(), trimmed(c.rate), trimmed(c.runoff), trimmed(c.on_tangent), if c.widening == 0.0 { String::new() } else { trimmed(c.widening) }]
                    })
                    .collect();
            }
            SuperelevationMethod::Stations { rows } => {
                r.method = 1;
                r.rows = rows.iter().map(|x| [trimmed(x.station), trimmed(x.left), trimmed(x.right)]).collect();
            }
        }
        r
    }

    fn def(&self) -> Result<SuperelevationDef, String> {
        let method = if self.method == 0 {
            let mut curves = Vec::new();
            for (i, c) in self.curves.iter().enumerate() {
                let n = i + 1;
                curves.push(SuperCurveDef {
                    pi: c[0].trim().to_owned(),
                    rate: need(&c[1], &format!("{n}. kurbun oranı"))?,
                    runoff: need(&c[2], &format!("{n}. kurbun geçiş boyu"))?,
                    on_tangent: need(&c[3], &format!("{n}. kurbun teğet payı"))?,
                    widening: number(&c[4]).map_err(|_| format!("{n}. kurbun genişlemesi bir sayı olmalı."))?.unwrap_or(0.0),
                });
            }
            SuperelevationMethod::Curves { curves }
        } else {
            let mut rows = Vec::new();
            for (i, x) in self.rows.iter().enumerate() {
                let n = i + 1;
                rows.push(SuperRowDef {
                    station: need(&x[0], &format!("{n}. satırın km'si"))?,
                    left: need(&x[1], &format!("{n}. satırın sol eğimi"))?,
                    right: need(&x[2], &format!("{n}. satırın sağ eğimi"))?,
                });
            }
            SuperelevationMethod::Stations { rows }
        };
        Ok(SuperelevationDef {
            id: self.id.clone(),
            name: self.name.clone(),
            alignment: self.alignment.clone(),
            alignment_revision: self.alignment_revision,
            revision: self.revision,
            normal_left: need(&self.normal[0], "Sol normal eğim")?,
            normal_right: need(&self.normal[1], "Sağ normal eğim")?,
            method,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) rows: Vec<Row>,
    kept: Vec<SuperelevationDef>,
    pub(crate) chosen: Option<usize>,
    /// The table row chosen.
    line: Option<usize>,
    /// The chosen schedule as last built, or why not.
    pub(crate) report: Option<Result<SuperelevationReport, String>>,
    /// The alignment's length and its curves' PIs (id, TS, ST).
    length: f64,
    curves: Vec<(String, f64, f64)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    Remove,
    Rebind,
    Name(String),
    Alignment(String),
    /// 0 left, 1 right.
    Normal(usize, String),
    Method(usize),
    /// A table cell: the row and the column.
    Cell(usize, usize, String),
    Line(usize),
    Add,
    RemoveRow,
    AllCurves,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Superelevations(event)
}

impl App {
    /// Dever: the window on the road project's superelevation schedules.
    pub(crate) fn open_superelevations(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Dever yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let kept = doc.model.road_superelevations().to_vec();
        self.superelevation_window = Some(Box::new(Window {
            chosen: (!kept.is_empty()).then_some(0),
            rows: kept.iter().map(Row::of).collect(),
            kept,
            ..Window::default()
        }));
        self.dialog = Some(Dialog::Superelevations);
        self.refresh_superelevation();
    }

    /// Dever oluştur from the ribbon: the window with a new schedule on the first alignment.
    pub(crate) fn new_superelevation(&mut self) {
        self.open_superelevations();
        if self.superelevation_window.is_some() {
            let _ = self.superelevation_event(Event::New);
        }
    }

    /// The chosen schedule built again as typed, and its alignment's curves.
    fn refresh_superelevation(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.superelevation_window.as_mut() else { return };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            w.report = None;
            return;
        };
        let made = kentos_native_application::road_alignments::made_alignments(&doc.model);
        match made.iter().find(|(d, _)| d.id == r.alignment).map(|(_, b)| b) {
            Some(Ok(b)) => {
                w.length = b.core.length;
                w.curves = b.core.curves.iter().map(|c| (c.pi.clone(), c.stations[0], c.stations[3])).collect();
            }
            _ => {
                w.length = 0.0;
                w.curves.clear();
            }
        }
        w.report = Some(match r.def() {
            Err(e) => Err(e),
            Ok(def) => {
                let report = road_superelevations::preview(&doc.model, &def);
                match report.problem.clone() {
                    Some(p) => Err(p),
                    None => Ok(report),
                }
            }
        });
    }

    fn superelevation_alignments(&self) -> Vec<(String, String, u32)> {
        self.document.as_ref().map_or_else(Vec::new, |d| {
            d.model
                .road_alignments()
                .iter()
                .filter(|a| a.offset_of.is_none())
                .map(|a| (a.id.clone(), a.name.clone(), a.revision))
                .collect()
        })
    }

    pub(crate) fn superelevation_event(&mut self, event: Event) -> Task<Message> {
        let alignments = self.superelevation_alignments();
        let Some(w) = self.superelevation_window.as_mut() else { return Task::none() };
        let mut warning = None;
        match event {
            Event::Choose(i) => {
                w.chosen = Some(i);
                w.line = None;
            }
            Event::New => match alignments.first() {
                None => warning = Some("Dever bir aksın üstünde kurulur: önce Güzergâh'ta bir aks oluşturun."),
                Some((id, _, rev)) => {
                    let taken: Vec<String> = w.rows.iter().map(|r| r.id.clone()).collect();
                    let new_id = (1..).map(|n| format!("d-{n}")).find(|x| !taken.contains(x)).unwrap_or_default();
                    let n = w.rows.len() + 1;
                    // Nothing filled in for the user: the normal slopes and the rates come from the standard.
                    w.rows.push(Row {
                        id: new_id,
                        name: if n == 1 { "Dever".to_owned() } else { format!("Dever {n}") },
                        alignment: id.clone(),
                        alignment_revision: *rev,
                        ..Row::default()
                    });
                    w.chosen = Some(w.rows.len() - 1);
                    w.line = None;
                }
            },
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|i| *i < w.rows.len()) {
                    w.rows.remove(i);
                    w.chosen = (!w.rows.is_empty()).then_some(i.min(w.rows.len().saturating_sub(1)));
                }
            }
            Event::Rebind => return self.rebind_superelevation(),
            Event::Name(t) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.name = t;
                }
            }
            Event::Alignment(id) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    if w.kept.iter().any(|k| k.id == r.id) {
                        warning = Some("Kaydedilmiş çizelgenin aksı değişmez; yeni bir çizelge oluşturun.");
                    } else {
                        r.alignment_revision = alignments.iter().find(|a| a.0 == id).map_or(0, |a| a.2);
                        r.alignment = id;
                    }
                }
            }
            Event::Normal(k, t) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.normal[k.min(1)] = t;
                }
            }
            Event::Method(m) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.method = m.min(1);
                }
                w.line = None;
            }
            Event::Cell(j, k, t) => {
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    if r.method == 0 {
                        if let Some(c) = r.curves.get_mut(j) {
                            c[k.min(4)] = t;
                        }
                    } else if let Some(x) = r.rows.get_mut(j) {
                        x[k.min(2)] = t;
                    }
                }
                w.line = Some(j);
            }
            Event::Line(j) => w.line = Some(j),
            Event::Add => {
                let free = w.curves.iter().map(|c| c.0.clone()).find(|pi| {
                    w.chosen.and_then(|i| w.rows.get(i)).is_some_and(|r| r.curves.iter().all(|c| c[0] != *pi))
                });
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    if r.method == 0 {
                        r.curves.push([free.unwrap_or_default(), String::new(), String::new(), String::new(), String::new()]);
                        w.line = Some(r.curves.len() - 1);
                    } else {
                        r.rows.push(Default::default());
                        w.line = Some(r.rows.len() - 1);
                    }
                }
            }
            Event::RemoveRow => {
                if let (Some(r), Some(j)) = (w.chosen.and_then(|i| w.rows.get_mut(i)), w.line) {
                    if r.method == 0 && j < r.curves.len() {
                        r.curves.remove(j);
                    } else if r.method == 1 && j < r.rows.len() {
                        r.rows.remove(j);
                    }
                    w.line = None;
                }
            }
            Event::AllCurves => {
                let pis: Vec<String> = w.curves.iter().map(|c| c.0.clone()).collect();
                if let Some(r) = w.chosen.and_then(|i| w.rows.get_mut(i)) {
                    r.method = 0;
                    for pi in pis {
                        if r.curves.iter().all(|c| c[0] != pi) {
                            r.curves.push([pi, String::new(), String::new(), String::new(), String::new()]);
                        }
                    }
                }
            }
            Event::Cancel => {
                self.superelevation_window = None;
                self.dialog = None;
                return Task::none();
            }
            Event::Save => return self.save_superelevations(),
        }
        if let Some(words) = warning {
            self.warn(words.to_owned());
        }
        self.refresh_superelevation();
        Task::none()
    }

    fn superelevation_write(
        &mut self,
        edit: SuperelevationsEdit,
    ) -> bool {
        let Some(doc) = self.document.as_mut() else { return false };
        let result = road_superelevations_edit::execute(&mut ExecutionContext::new(&mut doc.model), edit);
        self.answered(result).is_some()
    }

    fn blank_edit(operation: RoadEditOperation, id: &str) -> SuperelevationsEdit {
        SuperelevationsEdit {
            operation,
            superelevation: Some(id.to_owned()),
            name: None,
            alignment: None,
            normal_left: None,
            normal_right: None,
            method: None,
            expected_revision: None,
        }
    }

    /// Aksa yeniden bağla: the chosen saved schedule takes its alignment's present revision, one undo step.
    fn rebind_superelevation(&mut self) -> Task<Message> {
        let Some(w) = &self.superelevation_window else { return Task::none() };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else { return Task::none() };
        if !w.kept.iter().any(|k| r.def().ok().as_ref() == Some(k)) {
            self.warn("Önce çizelgeyi kaydedin; kaydedilmiş çizelge yeniden bağlanır.".to_owned());
            return Task::none();
        }
        if self.superelevation_write(Self::blank_edit(RoadEditOperation::Rebind, &r.id.clone())) {
            let Some(doc) = &self.document else { return Task::none() };
            let kept = doc.model.road_superelevations().to_vec();
            if let Some(w) = self.superelevation_window.as_mut() {
                w.rows = kept.iter().map(Row::of).collect();
                w.kept = kept;
            }
            self.refresh_superelevation();
            self.say(Level::Success, "Dever çizelgesi aksın bugünkü hâline bağlandı.");
        }
        Task::none()
    }

    /// Kaydet: the removed first, then the changed and the new.
    fn save_superelevations(&mut self) -> Task<Message> {
        let Some(w) = self.superelevation_window.clone() else { return Task::none() };
        let mut defs = Vec::new();
        for r in &w.rows {
            match r.def() {
                Ok(d) => defs.push(d),
                Err(e) => {
                    self.warn(format!("“{}”: {e}", r.name));
                    return Task::none();
                }
            }
        }
        for k in w.kept.iter().filter(|k| defs.iter().all(|d| d.id != k.id)) {
            if !self.superelevation_write(Self::blank_edit(RoadEditOperation::Remove, &k.id)) {
                return Task::none();
            }
        }
        for d in &defs {
            let kept = w.kept.iter().find(|k| k.id == d.id);
            if kept == Some(d) {
                continue;
            }
            let edit = SuperelevationsEdit {
                name: Some(d.name.clone()),
                alignment: kept.is_none_or(|k| k.alignment != d.alignment).then(|| d.alignment.clone()),
                normal_left: Some(d.normal_left),
                normal_right: Some(d.normal_right),
                method: Some(d.method.clone()),
                ..Self::blank_edit(RoadEditOperation::Set, &d.id)
            };
            if !self.superelevation_write(edit) {
                return Task::none();
            }
        }
        self.superelevation_window = None;
        self.dialog = None;
        self.say(Level::Success, "Dever çizelgeleri kaydedildi.");
        Task::none()
    }

    pub(crate) fn road_superelevation_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.superelevation_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let alignments = self.superelevation_alignments();
        let stale_of = |r: &Row| {
            w.kept.iter().any(|k| k.id == r.id)
                && alignments.iter().find(|a| a.0 == r.alignment).is_some_and(|a| a.2 != r.alignment_revision)
        };
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let on = alignments.iter().find(|a| a.0 == r.alignment).map_or(r.alignment.clone(), |a| a.1.clone());
            let kind = if stale_of(r) { format!("{on} · eski") } else { on };
            let face = column![
                Elided::new(r.name.clone()).size(typography::body()).font(typography::ui()).width(Fill),
                label::caption(kind).style(muted),
            ]
            .spacing(1);
            list = list.push(
                iced::widget::button(face)
                    .on_press(msg(Event::Choose(i)))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(w.chosen == Some(i))),
            );
        }
        if w.rows.is_empty() {
            list = list.push(label::caption("Dever çizelgesi yok. Yeni dever ile bir aksın kurplarına dever verin.").style(muted));
        }
        let left = column![
            label::strong("Dever çizelgeleri"),
            container(scrollable(list).height(typography::scaled(300.0))).style(style::container::field_box).width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        let chosen = w.chosen.and_then(|i| w.rows.get(i));
        let right: Element<'_, Message> = match chosen {
            None => container(label::caption("Bir dever çizelgesi seçin ya da yeni bir çizelge oluşturun.")).padding(12).into(),
            Some(r) => scrollable(self.superelevation_editor(w, r, stale_of(r))).height(typography::scaled(420.0)).into(),
        };
        let saved = chosen.is_some_and(|r| w.kept.iter().any(|k| r.def().ok().as_ref() == Some(k)));
        let body = column![
            row![container(left).width(220), container(right).width(Fill)].spacing(16),
            row![
                words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))),
                words::secondary(REBIND, (saved && chosen.is_some_and(stale_of)).then(|| msg(Event::Rebind))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, Some(msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center),
        ]
        .spacing(14);
        Frame::new(TITLE).push(body).width(1080.0).modal(msg(Event::Cancel))
    }

    fn superelevation_editor<'a>(&'a self, w: &'a Window, r: &'a Row, stale: bool) -> Element<'a, Message> {
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let alignments = self.superelevation_alignments();
        let ids: Vec<String> = alignments.iter().map(|a| a.0.clone()).collect();
        let on_alignment = Select::new(
            alignments.iter().map(|a| Choice::new(a.1.clone())),
            ids.iter().position(|id| *id == r.alignment),
            move |i| msg(Event::Alignment(ids[i].clone())),
        )
        .placeholder("Aks");
        let method = Select::new(METHODS.iter().map(|m| Choice::new((*m).to_owned())), Some(r.method), |m| msg(Event::Method(m)));
        let mut col = Column::new().spacing(10).padding(iced::Padding::default().right(14));
        col = col.push(
            row![
                words::field("Ad", kentos_ui::widget::text_field("Çizelgenin adı", &r.name).on_input(|t| msg(Event::Name(t))), None),
                words::field("Aks", on_alignment, None),
                words::field("Normal sol (%)", kentos_ui::widget::text_field("-2,5", &r.normal[0]).on_input(|t| msg(Event::Normal(0, t))).width(90), None),
                words::field("Normal sağ (%)", kentos_ui::widget::text_field("-2,5", &r.normal[1]).on_input(|t| msg(Event::Normal(1, t))).width(90), None),
                words::field("Yöntem", container(method).width(180), None),
            ]
            .spacing(10),
        );
        let normal = r.def().ok().map(|d| (d.normal_left, d.normal_right));
        let built = w.report.as_ref().and_then(|x| x.as_ref().ok());
        col = col.push(
            container(
                iced::widget::canvas(Chart { report: built, normal, length: w.length, curves: &w.curves })
                    .width(Fill)
                    .height(typography::scaled(220.0)),
            )
            .style(style::container::field_box)
            .width(Fill),
        );
        match &w.report {
            Some(Err(e)) => col = col.push(label::caption(e.clone()).style(danger)),
            Some(Ok(rep)) => {
                let what = if r.method == 0 {
                    let full = rep.critical.iter().filter(|c| c.mark == "full").count() / 2;
                    format!("{full} kurp · {} kritik km", rep.critical.len())
                } else {
                    format!("{} km satırı", r.rows.len())
                };
                col = col.push(label::caption(format!("{what} · sol mavi, sağ kırmızı, iç yanın genişlemesi yeşil")));
            }
            None => {}
        }
        if stale {
            col = col.push(label::caption("Aks değişti: km'leri denetleyip aksa yeniden bağlayın.").style(warn));
        }
        let (head, widths): (Vec<&str>, Vec<f32>) = if r.method == 0 {
            (vec!["PI", "Oran (%)", "Geçiş boyu (m)", "Teğet payı (0–1)", "Genişleme (m)"], vec![80.0, 90.0, 110.0, 110.0, 110.0])
        } else {
            (vec!["Km (m)", "Sol (%)", "Sağ (%)"], vec![120.0, 100.0, 100.0])
        };
        let mut table = Column::new().spacing(3);
        table = table.push(head.iter().zip(&widths).fold(row![].spacing(4), |row, (h, wd)| row.push(label::caption(*h).style(muted).width(*wd))));
        let cells: Vec<Vec<&String>> = if r.method == 0 {
            r.curves.iter().map(|c| c.iter().collect()).collect()
        } else {
            r.rows.iter().map(|x| x.iter().collect()).collect()
        };
        for (j, line) in cells.into_iter().enumerate() {
            let mut cells_row = row![].spacing(4).align_y(Center);
            for (k, v) in line.into_iter().enumerate() {
                cells_row = cells_row.push(
                    kentos_ui::widget::text_field("", v).on_input(move |t| msg(Event::Cell(j, k, t))).width(widths[k]),
                );
            }
            cells_row = cells_row.push(
                iced::widget::button(label::caption(if w.line == Some(j) { "●" } else { "○" }))
                    .on_press(msg(Event::Line(j)))
                    .padding([2, 6])
                    .style(style::button::row(w.line == Some(j))),
            );
            table = table.push(cells_row);
        }
        let title = if r.method == 0 { "Kurplar" } else { "Km satırları" };
        col = col.push(words::field(title, table, None)).push(
            row![
                words::secondary(ADD, Some(msg(Event::Add))),
                words::ghost(REMOVE_ROW, w.line.map(|_| msg(Event::RemoveRow))),
                words::secondary(ALL_CURVES, (!w.curves.is_empty()).then(|| msg(Event::AllCurves))),
            ]
            .spacing(8),
        );
        col = col.push(
            label::caption("Oran, geçiş boyu, teğet payı ve genişleme KGM'nin standardından ya da projeden yazılır; KentOS öntanımlı sayı vermez.")
                .style(muted),
        );
        col.into()
    }

    pub(crate) fn superelevation_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.superelevation_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(REBIND) => Some(msg(Event::Rebind)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE_ROW) => Some(msg(Event::RemoveRow)),
            Control::Press(ALL_CURVES) => Some(msg(Event::AllCurves)),
            Control::Press(words) => match w.rows.iter().position(|r| r.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill("Normal sol (%)", text) => Some(msg(Event::Normal(0, text.to_owned()))),
            Control::Fill("Normal sağ (%)", text) => Some(msg(Event::Normal(1, text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The diagram: both sides' slopes along the alignment, the curves' extents, the critical stations, the widening.
struct Chart<'a> {
    report: Option<&'a SuperelevationReport>,
    normal: Option<(f64, f64)>,
    length: f64,
    curves: &'a [(String, f64, f64)],
}

impl canvas::Program<Message> for Chart<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let word = |frame: &mut canvas::Frame, s: String, at: Point, color: Color, centred: bool| {
            frame.fill_text(Text {
                content: s,
                position: at,
                color,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                align_x: if centred { iced::widget::text::Alignment::Center } else { iced::widget::text::Alignment::Default },
                ..Text::default()
            });
        };
        let (Some(rep), Some((nl, nr))) = (self.report, self.normal) else {
            word(&mut frame, "Normal eğimleri ve kurpların değerlerini yazınca dever burada çizilir.".into(), Point::new(16.0, 20.0), t.muted, false);
            return vec![frame.into_geometry()];
        };
        let length = self.length.max(rep.left.iter().chain(&rep.right).map(|p| p[0]).fold(0.0, f64::max));
        if length <= 0.0 {
            return vec![frame.into_geometry()];
        }
        let slopes = rep.left.iter().chain(&rep.right).map(|p| p[1]).chain([nl, nr]);
        let step = if slopes.clone().fold(0.0_f64, |a, v| a.max(v.abs())) > 8.0 { 4.0 } else { 2.0 };
        let top = (slopes.fold(0.0_f64, |a, v| a.max(v.abs())) / step).ceil().max(1.0) * step + step / 2.0;
        let widened = !(rep.left_widening.is_empty() && rep.right_widening.is_empty());
        // The slopes above, the widening in its own strip under them, the critical stations' names at the bottom.
        let strip = if widened { 40.0 } else { 0.0 };
        let plot = Rectangle { x: 52.0, y: 12.0, width: bounds.width - 64.0, height: bounds.height - 34.0 - strip };
        let x = |s: f64| plot.x + (s / length * plot.width as f64) as f32;
        let y = |v: f64| plot.y + plot.height / 2.0 - (v / top * plot.height as f64 / 2.0) as f32;
        let bottom = plot.y + plot.height + strip;
        let thin = |c: Color| Stroke::default().with_color(c).with_width(1.0);
        // The curves' extents, TS to ST.
        for (pi, a, b) in self.curves {
            frame.fill_rectangle(Point::new(x(*a), plot.y), iced::Size::new(x(*b) - x(*a), bottom - plot.y), Color { a: 0.06, ..t.accent });
            word(&mut frame, pi.clone(), Point::new(0.5 * (x(*a) + x(*b)), plot.y + 2.0), t.muted, true);
        }
        let mut v = -(top / step).floor() * step;
        while v <= top + 1e-9 {
            let c = if v == 0.0 { t.muted } else { t.border };
            frame.stroke(&Path::line(Point::new(plot.x, y(v)), Point::new(plot.x + plot.width, y(v))), thin(c));
            let shown = if v < 0.0 { format!("−%{}", comma(-v, 0)) } else { format!("%{}", comma(v, 0)) };
            word(&mut frame, shown, Point::new(4.0, y(v) - 7.0), t.muted, false);
            v += step;
        }
        for c in &rep.critical {
            let at = x(c.station);
            frame.stroke(
                &Path::line(Point::new(at, plot.y), Point::new(at, bottom)),
                Stroke { line_dash: LineDash { segments: &[3.0, 3.0], offset: 0 }, ..thin(t.border) },
            );
            let mark = match c.mark.as_str() {
                "normalCrown" => "NC",
                "levelCrown" => "LC",
                "reverseCrown" => "RC",
                _ => "FS",
            };
            word(&mut frame, mark.into(), Point::new(at, bottom + 4.0), t.muted, true);
        }
        let side = |points: &[[f64; 2]], normal: f64| -> Vec<[f64; 2]> {
            let mut out = vec![[0.0, points.first().map_or(normal, |p| p[1])]];
            out.extend_from_slice(points);
            out.push([length, points.last().map_or(normal, |p| p[1])]);
            out
        };
        for (points, color) in [(side(&rep.left, nl), t.accent), (side(&rep.right, nr), t.danger)] {
            let line = Path::new(|b| {
                for (i, p) in points.iter().enumerate() {
                    let at = Point::new(x(p[0]), y(p[1]));
                    if i == 0 { b.move_to(at) } else { b.line_to(at) }
                }
            });
            frame.stroke(&line, Stroke::default().with_color(color).with_width(2.0));
        }
        if widened {
            let green = Color::from_rgb8(0x16, 0xa3, 0x4a);
            let base = bottom - 6.0;
            frame.stroke(&Path::line(Point::new(plot.x, base), Point::new(plot.x + plot.width, base)), thin(t.border));
            let most = rep.left_widening.iter().chain(&rep.right_widening).map(|p| p[1]).fold(0.0, f64::max).max(1e-9);
            word(&mut frame, format!("{} m", comma(most, 2)), Point::new(4.0, base - 26.0), green, false);
            for points in [&rep.left_widening, &rep.right_widening] {
                if points.is_empty() {
                    continue;
                }
                let full = side(points, 0.0);
                let line = Path::new(|b| {
                    for (i, p) in full.iter().enumerate() {
                        let at = Point::new(x(p[0]), base - (p[1] / most * 24.0) as f32);
                        if i == 0 { b.move_to(at) } else { b.line_to(at) }
                    }
                });
                frame.stroke(&line, Stroke::default().with_color(green).with_width(1.6));
            }
        }
        vec![frame.into_geometry()]
    }
}
