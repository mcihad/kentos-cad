//! Brükner (plan/road.md Y9, `haul.editor`): a road project's mass haul diagrams, as Civil 3D's mass haul view with
//! its balancing options and OpenRoads' mass haul diagram make them: an earthwork volume's mass ordinate against a
//! balance line, a free haul distance, borrow and waste sites at their stations. The diagram is read as it is typed
//! (`road.hauls.query`'s rule): the curve with its loops shaded (forward, backward, unbalanced), the balance points and
//! the sites' steps; the loops' table; the totals and the warnings.
//!
//! ```text
//! ┌ Brükner ────────────────────────────────────────────────────────────────────────────────────┐
//! │ Brüknerler          │ Ad [Ana Brükner] Kübaj [▾] Denge çizgisi (m³) [0] Serbest taşıma (m) [ ] │
//! │ ● Ana Brükner       │ Para birimi [TL] Fazla taşıma Döküm Ödünç (birim fiyat) Kamyon yükü  │
//! │                     │ Alanlar: Ad  Tür [Ödünç ▾]  Km  Hacim  ○   [Alan ekle] [Alanı sil]    │
//! │ [Yeni Brükner]      │ ┌───────────────────────────────────────────────────────────────────┐ │
//! │                     │ │  the diagram: curve, loops, balance line and points, sites         │ │
//! │                     │ └───────────────────────────────────────────────────────────────────┘ │
//! │                     │ totals · warnings · loops                                             │
//! │ [Sil]                                                                    [Vazgeç] [Kaydet] │
//! └─────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy of the diagrams; Kaydet writes what changed through `road.hauls.edit` (removes, then sets),
//! each one undo step, and closes. The free haul, the sites, the unit prices and the truck's payload are the user's:
//! nothing is filled in; the cost is the priced items' sum, the trips the hauled volume over the payload (ADR 0907 §5).

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Center, Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{CommandWarning, HaulCostDef, HaulDef, HaulReport, HaulSiteDef, HaulSiteKind, HaulsEdit, RoadEditOperation};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, road_hauls, road_hauls_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, Elided};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Brükner";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni Brükner";
const REMOVE: &str = "Sil";
const ADD: &str = "Alan ekle";
const REMOVE_ROW: &str = "Alanı sil";

fn trimmed(v: f64) -> String {
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// A number in Turkish: thousands with points, the fraction after a comma.
fn grouped(v: f64, digits: usize) -> String {
    let text = format!("{:.digits$}", v.abs());
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let mut out = String::new();
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    let sign = if v < 0.0 && text.chars().any(|c| c.is_ascii_digit() && c != '0') { "-" } else { "" };
    if fraction.is_empty() { format!("{sign}{out}") } else { format!("{sign}{out},{fraction}") }
}

fn number(text: &str) -> Result<Option<f64>, ()> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>().map(Some).map_err(|_| ())
}

/// A diagram as typed.
#[derive(Debug, Clone)]
pub struct Row {
    pub(crate) id: String,
    pub(crate) name: String,
    revision: u32,
    volume: Option<String>,
    /// The balance line, the free haul, the currency, the overhaul, waste and borrow prices, the truck's payload.
    fields: [String; 7],
    /// A site's name, kind, station and volume.
    sites: Vec<(String, HaulSiteKind, String, String)>,
}

impl Row {
    fn of(d: &HaulDef) -> Self {
        Self {
            id: d.id.clone(),
            name: d.name.clone(),
            revision: d.revision,
            volume: Some(d.volume.clone()),
            fields: {
                let price = |f: fn(&HaulCostDef) -> Option<f64>| d.cost.as_ref().and_then(f).map(trimmed).unwrap_or_default();
                [
                    if d.balance == 0.0 { String::new() } else { trimmed(d.balance) },
                    d.free.map(trimmed).unwrap_or_default(),
                    d.cost.as_ref().map(|c| c.currency.clone()).unwrap_or_default(),
                    price(|c| c.overhaul),
                    price(|c| c.waste),
                    price(|c| c.borrow),
                    price(|c| c.payload),
                ]
            },
            sites: d.sites.iter().map(|s| (s.name.clone(), s.kind, trimmed(s.station), trimmed(s.volume))).collect(),
        }
    }

    fn def(&self) -> Result<HaulDef, String> {
        let volume = self.volume.clone().ok_or("Bir kübaj seçin: Brükner onun kütle ordinatıdır.")?;
        let balance = number(&self.fields[0]).map_err(|()| "Denge çizgisi bir sayı olmalı.".to_owned())?.unwrap_or(0.0);
        let free = number(&self.fields[1]).map_err(|()| "Serbest taşıma bir sayı olmalı.".to_owned())?;
        let mut prices = [None; 4];
        for (k, what) in ["Fazla taşıma fiyatı", "Döküm fiyatı", "Ödünç fiyatı", "Kamyon yükü"].into_iter().enumerate() {
            prices[k] = number(&self.fields[3 + k]).map_err(|()| format!("{what} bir sayı olmalı."))?;
        }
        let currency = self.fields[2].trim();
        let cost = match (currency.is_empty(), prices.iter().any(Option::is_some)) {
            (true, false) => None,
            (true, true) => return Err("Birim fiyat ya da kamyon yükü için para birimini yazın.".to_owned()),
            (false, _) => Some(HaulCostDef {
                currency: currency.to_owned(),
                overhaul: prices[0],
                waste: prices[1],
                borrow: prices[2],
                payload: prices[3],
            }),
        };
        let mut sites = Vec::new();
        for (i, (name, kind, station, volume)) in self.sites.iter().enumerate() {
            let n = i + 1;
            let need = |t: &str, what: &str| match number(t) {
                Ok(Some(v)) => Ok(v),
                _ => Err(format!("{n}. alanın {what} bir sayı olmalı.")),
            };
            sites.push(HaulSiteDef { name: name.clone(), kind: *kind, station: need(station, "km'si")?, volume: need(volume, "hacmi")? });
        }
        Ok(HaulDef { id: self.id.clone(), name: self.name.clone(), volume, revision: self.revision, balance, free, sites, cost })
    }
}

/// The chosen diagram as last read: its report and warnings, or why it does not read.
#[derive(Debug, Clone, Default)]
pub(crate) struct Preview {
    pub(crate) report: Option<HaulReport>,
    pub(crate) warnings: Vec<CommandWarning>,
    pub(crate) why: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Window {
    pub(crate) rows: Vec<Row>,
    kept: Vec<HaulDef>,
    pub(crate) chosen: Option<usize>,
    line: Option<usize>,
    pub(crate) preview: Preview,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(usize),
    New,
    Remove,
    Name(String),
    Volume(String),
    /// 0 the balance line, 1 the free haul, 2 the currency, 3–5 the overhaul, waste and borrow prices, 6 the payload.
    Field(usize, String),
    /// A site's name (0), station (2) or volume (3).
    Site(usize, usize, String),
    Kind(usize, HaulSiteKind),
    Line(usize),
    Add,
    RemoveRow,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Hauls(event)
}

fn chosen_row(w: &mut Window) -> Option<&mut Row> {
    w.chosen.and_then(|i| w.rows.get_mut(i))
}

impl App {
    /// Brükner: the window on the road project's mass haul diagrams.
    pub(crate) fn open_hauls(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        if doc.model.settings().cad_work != Some(kentos_commands::CadWork::Road) {
            self.warn("Brükner yalnız yol projesinde (KentOS Yol) olur.".to_owned());
            return;
        }
        let kept = doc.model.road_hauls().to_vec();
        self.haul_window = Some(Box::new(Window {
            chosen: (!kept.is_empty()).then_some(0),
            rows: kept.iter().map(Row::of).collect(),
            kept,
            ..Window::default()
        }));
        self.dialog = Some(Dialog::Hauls);
        self.refresh_haul();
    }

    /// Brükner oluştur from the ribbon: the window with a new diagram of the first volume.
    pub(crate) fn new_haul(&mut self) {
        self.open_hauls();
        if self.haul_window.is_some() {
            let _ = self.haul_event(Event::New);
        }
    }

    fn refresh_haul(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(w) = self.haul_window.as_mut() else { return };
        let Some(r) = w.chosen.and_then(|i| w.rows.get(i)) else {
            w.preview = Preview::default();
            return;
        };
        w.preview = match r.def().and_then(|d| road_hauls::preview(&doc.model, &d)) {
            Err(e) => Preview { why: Some(e), ..Preview::default() },
            Ok((report, warnings)) => Preview { report: Some(report), warnings, why: None },
        };
    }

    /// The project's earthwork volumes: id and name.
    fn haul_volumes(&self) -> Vec<(String, String)> {
        self.document
            .as_ref()
            .map_or_else(Vec::new, |d| d.model.road_volumes().iter().map(|v| (v.id.clone(), v.name.clone())).collect())
    }

    pub(crate) fn haul_event(&mut self, event: Event) -> Task<Message> {
        let volumes = self.haul_volumes();
        let Some(w) = self.haul_window.as_mut() else { return Task::none() };
        let mut warning = None;
        match event {
            Event::Choose(i) => {
                w.chosen = Some(i);
                w.line = None;
            }
            Event::New => match volumes.first() {
                None => warning = Some("Brükner bir kübajın kütle ordinatıdır: önce Kübaj'da bir kübaj hesabı oluşturun."),
                Some((id, _)) => {
                    let taken: Vec<String> = w.rows.iter().map(|r| r.id.clone()).collect();
                    let new_id = (1..).map(|n| format!("h-{n}")).find(|x| !taken.contains(x)).unwrap_or_default();
                    let n = w.rows.len() + 1;
                    // The first volume, the balance line at 0; the free haul and the sites are the user's.
                    w.rows.push(Row {
                        id: new_id,
                        name: if n == 1 { "Brükner".to_owned() } else { format!("Brükner {n}") },
                        revision: 0,
                        volume: Some(id.clone()),
                        fields: Default::default(),
                        sites: Vec::new(),
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
            Event::Name(t) => {
                if let Some(r) = chosen_row(w) {
                    r.name = t;
                }
            }
            Event::Volume(id) => {
                let saved = w.chosen.and_then(|i| w.rows.get(i)).is_some_and(|r| w.kept.iter().any(|k| k.id == r.id));
                if saved {
                    warning = Some("Kaydedilmiş Brüknerin kübajı değişmez; başka kübaj için yeni bir Brükner oluşturun.");
                } else if let Some(r) = chosen_row(w) {
                    r.volume = Some(id);
                }
            }
            Event::Field(k, t) => {
                if let Some(r) = chosen_row(w) {
                    r.fields[k.min(6)] = t;
                }
            }
            Event::Site(j, k, t) => {
                if let Some(x) = chosen_row(w).and_then(|r| r.sites.get_mut(j)) {
                    match k {
                        0 => x.0 = t,
                        2 => x.2 = t,
                        _ => x.3 = t,
                    }
                }
                w.line = Some(j);
            }
            Event::Kind(j, kind) => {
                if let Some(x) = chosen_row(w).and_then(|r| r.sites.get_mut(j)) {
                    x.1 = kind;
                }
                w.line = Some(j);
            }
            Event::Line(j) => w.line = Some(j),
            Event::Add => {
                if let Some(r) = chosen_row(w) {
                    // A new site's name follows the count; its station and volume are the user's.
                    let n = r.sites.len() + 1;
                    r.sites.push((format!("Alan {n}"), HaulSiteKind::Waste, String::new(), String::new()));
                    w.line = Some(n - 1);
                }
            }
            Event::RemoveRow => {
                if let Some(j) = w.line {
                    if let Some(r) = chosen_row(w).filter(|r| j < r.sites.len()) {
                        r.sites.remove(j);
                    }
                    w.line = None;
                }
            }
            Event::Cancel => {
                self.haul_window = None;
                self.dialog = None;
                return Task::none();
            }
            Event::Save => return self.save_hauls(),
        }
        if let Some(words) = warning {
            self.warn(words.to_owned());
        }
        self.refresh_haul();
        Task::none()
    }

    fn blank_haul_edit(operation: RoadEditOperation, id: &str) -> HaulsEdit {
        HaulsEdit {
            operation,
            haul: Some(id.to_owned()),
            name: None,
            volume: None,
            balance: None,
            free: None,
            sites: None,
            cost: None,
            expected_revision: None,
        }
    }

    fn haul_write(&mut self, edit: HaulsEdit) -> bool {
        let Some(doc) = self.document.as_mut() else { return false };
        let result = road_hauls_edit::execute(&mut ExecutionContext::new(&mut doc.model), edit);
        self.answered(result).is_some()
    }

    fn save_hauls(&mut self) -> Task<Message> {
        let Some(w) = self.haul_window.clone() else { return Task::none() };
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
            if !self.haul_write(Self::blank_haul_edit(RoadEditOperation::Remove, &k.id)) {
                return Task::none();
            }
        }
        for d in &defs {
            let kept = w.kept.iter().find(|k| k.id == d.id);
            if kept == Some(d) {
                continue;
            }
            let edit = HaulsEdit {
                name: Some(d.name.clone()),
                volume: kept.is_none().then(|| d.volume.clone()),
                balance: Some(d.balance),
                free: Some(d.free),
                sites: Some(d.sites.clone()),
                cost: Some(d.cost.clone()),
                ..Self::blank_haul_edit(RoadEditOperation::Set, &d.id)
            };
            if !self.haul_write(edit) {
                return Task::none();
            }
        }
        self.haul_window = None;
        self.dialog = None;
        self.say(Level::Success, "Brüknerler kaydedildi.");
        Task::none()
    }

    pub(crate) fn road_haul_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.haul_window else {
            return iced::widget::text("").into();
        };
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let volumes = self.haul_volumes();
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rows.iter().enumerate() {
            let on = volumes.iter().find(|v| Some(&v.0) == r.volume.as_ref()).map_or_else(String::new, |v| v.1.clone());
            let face = column![
                Elided::new(r.name.clone()).size(typography::body()).font(typography::ui()).width(Fill),
                label::caption(format!("{on} · {} alan", r.sites.len())).style(muted),
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
            list = list.push(label::caption("Brükner yok. Yeni Brükner ile bir kübajın kütle diyagramını okuyun.").style(muted));
        }
        let left = column![
            label::strong("Brüknerler"),
            container(scrollable(list).height(typography::scaled(300.0))).style(style::container::field_box).width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        let chosen = w.chosen.and_then(|i| w.rows.get(i));
        let right: Element<'_, Message> = match chosen {
            None => container(label::caption("Bir Brükner seçin ya da yeni bir Brükner oluşturun.")).padding(12).into(),
            Some(r) => self.haul_editor(w, r, &volumes),
        };
        // The editor scrolls with the body: resized or full screen it grows.
        let body = row![container(left).width(220), container(right).width(Fill)].spacing(16);
        // The buttons are the dialog's footer: resized or full screen they sit on its bottom edge (ADR 0746 eki).
        let footer = row![
                words::ghost(REMOVE, w.chosen.map(|_| msg(Event::Remove))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, Some(msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center);
        Frame::new(TITLE).scroll(body).footer(footer).width(1120.0).modal(msg(Event::Cancel))
    }

    fn haul_editor<'a>(&'a self, w: &'a Window, r: &'a Row, volumes: &[(String, String)]) -> Element<'a, Message> {
        let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
        let warn = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).warning) };
        let danger = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).danger) };
        let ids: Vec<String> = volumes.iter().map(|v| v.0.clone()).collect();
        let on_volume = Select::new(
            volumes.iter().map(|v| Choice::new(v.1.clone())),
            r.volume.as_ref().and_then(|v| ids.iter().position(|i| i == v)),
            move |i| msg(Event::Volume(ids[i].clone())),
        )
        .placeholder("Kübaj");
        let field = |hint: &'a str, k: usize, width: f32| {
            kentos_ui::widget::text_field(hint, &r.fields[k]).on_input(move |t| msg(Event::Field(k, t))).width(width)
        };
        // The picture takes what a resized or full screen window has over (docs/adr/0937 §2).
        let mut col = kentos_ui::widget::grow::Grows::new().spacing(10.0);
        col = col.push(
            row![
                words::field("Ad", kentos_ui::widget::text_field("Brüknerin adı", &r.name).on_input(|t| msg(Event::Name(t))), None),
                words::field("Kübaj", container(on_volume).width(200), None),
                words::field("Denge çizgisi (m³)", field("0", 0, 110.0), None),
                words::field("Serbest taşıma (m)", field("", 1, 110.0), None),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom),
        );
        // The price fields name the currency as it is typed.
        let currency = if r.fields[2].trim().is_empty() { "para".to_owned() } else { r.fields[2].trim().to_owned() };
        let priced = |title: String, k: usize, width: f32| -> Element<'a, Message> {
            column![label::caption(title), field("", k, width)].spacing(4).into()
        };
        col = col.push(
            row![
                words::field("Para birimi", field("", 2, 80.0), None),
                priced(format!("Fazla taşıma ({currency}/m³·km)"), 3, 150.0),
                priced(format!("Döküm ({currency}/m³)"), 4, 110.0),
                priced(format!("Ödünç ({currency}/m³)"), 5, 110.0),
                words::field("Kamyon yükü (m³)", field("", 6, 110.0), None),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Bottom),
        );
        let widths = [150.0, 110.0, 80.0, 90.0];
        let mut table = Column::new().spacing(3);
        table = table.push(
            ["Ad", "Tür", "Km (m)", "Hacim (m³)"]
                .iter()
                .zip(widths)
                .fold(row![].spacing(4), |row, (h, wd)| row.push(label::caption(*h).style(muted).width(wd))),
        );
        let kinds = [(HaulSiteKind::Borrow, "Ödünç"), (HaulSiteKind::Waste, "Döküm")];
        for (j, (name, kind, station, volume)) in r.sites.iter().enumerate() {
            let pick = Select::new(
                kinds.iter().map(|k| Choice::new(k.1.to_owned())),
                kinds.iter().position(|k| k.0 == *kind),
                move |i| msg(Event::Kind(j, kinds[i].0)),
            );
            table = table.push(
                row![
                    kentos_ui::widget::text_field("Ad", name).on_input(move |v| msg(Event::Site(j, 0, v))).width(widths[0]),
                    container(pick).width(widths[1]),
                    kentos_ui::widget::text_field("", station).on_input(move |v| msg(Event::Site(j, 2, v))).width(widths[2]),
                    kentos_ui::widget::text_field("", volume).on_input(move |v| msg(Event::Site(j, 3, v))).width(widths[3]),
                    iced::widget::button(label::caption(if w.line == Some(j) { "●" } else { "○" }))
                        .on_press(msg(Event::Line(j)))
                        .padding([2, 6])
                        .style(style::button::row(w.line == Some(j))),
                ]
                .spacing(4)
                .align_y(Center),
            );
        }
        col = col.push(
            row![
                words::field("Ödünç ve döküm alanları", table, None),
                column![
                    words::secondary(ADD, Some(msg(Event::Add))),
                    words::ghost(REMOVE_ROW, w.line.map(|_| msg(Event::RemoveRow))),
                    label::caption("Serbest taşıma ve birim fiyatlar\nsözleşmeden ve tarifeden yazılır;\nKentOS öntanımlı sayı vermez.")
                        .style(muted),
                ]
                .spacing(6),
            ]
            .spacing(16),
        );
        match (&w.preview.why, &w.preview.report) {
            // The totals and the cost over the diagram, the loops under it.
            (Some(e), _) => col = col.push(label::caption(e.clone()).style(danger)),
            (None, Some(rep)) => {
                let avg = rep.average.map_or_else(|| "—".to_owned(), |a| format!("{} m", grouped(a, 0)));
                col = col.push(label::caption(format!(
                    "Dengelenen {} m³ · moment {} m³·m · ortalama taşıma {avg} · fazla taşıma {} m³ ({} m³·m) · döküm {} m³ · ödünç {} m³",
                    grouped(rep.hauled, 0),
                    grouped(rep.moment, 0),
                    grouped(rep.over_volume, 0),
                    grouped(rep.over_moment, 0),
                    grouped(rep.waste, 0),
                    grouped(rep.borrow, 0),
                )));
                if let Some(c) = &rep.cost {
                    let part = |what: &str, v: Option<f64>| v.map(|v| format!(" · {what} {} {}", grouped(v, 2), c.currency));
                    let parts: String = [part("fazla taşıma", c.overhaul), part("döküm", c.waste), part("ödünç", c.borrow)]
                        .into_iter()
                        .flatten()
                        .collect();
                    let trips = c.trips.map_or_else(String::new, |t| format!(" · {} kamyon seferi", grouped(t, 0)));
                    col = col.push(label::strong(format!("Maliyet {} {}{parts}{trips}", grouped(c.total, 2), c.currency)));
                }
                for wn in &w.preview.warnings {
                    col = col.push(label::caption(wn.message.clone()).style(warn));
                }
                col = col.push_grow(crate::picture::picture(iced::widget::canvas(Diagram { preview: &w.preview }).width(Fill).height(Fill), "Brükner"), typography::scaled(160.0));
                col = col.push(loops_table(rep));
            }
            _ => {}
        }
        container(col).padding(iced::Padding::default().right(14)).into()
    }

    pub(crate) fn haul_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        let Some(w) = &self.haul_window else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE_ROW) => Some(msg(Event::RemoveRow)),
            Control::Press(words) => match w.rows.iter().position(|r| r.name == words) {
                Some(i) => Some(msg(Event::Choose(i))),
                None => return Err(format!("“{TITLE}” penceresinde “{words}” düğmesi yok")),
            },
            Control::Fill("Ad", text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill("Denge çizgisi (m³)", text) => Some(msg(Event::Field(0, text.to_owned()))),
            Control::Fill("Serbest taşıma (m)", text) => Some(msg(Event::Field(1, text.to_owned()))),
            Control::Fill("Para birimi", text) => Some(msg(Event::Field(2, text.to_owned()))),
            Control::Fill("Kamyon yükü (m³)", text) => Some(msg(Event::Field(6, text.to_owned()))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The loops as the window lists them.
fn loops_table(rep: &HaulReport) -> Element<'_, Message> {
    let muted = |t: &Theme| iced::widget::text::Style { color: Some(Tokens::of(t).muted) };
    let widths = [84.0, 84.0, 44.0, 90.0, 110.0, 60.0, 90.0, 110.0, 90.0];
    let head = ["Baş km", "Son km", "Yön", "Dengelenen", "Moment", "Ort.", "Fazla taş.", "Fazla mom.", "Dengesiz"];
    let header = head.iter().zip(widths).fold(row![].spacing(6), |row, (h, wd)| row.push(label::caption(*h).style(muted).width(wd)));
    let mut rows = Column::new().spacing(2);
    for l in &rep.loops {
        let cells = [
            l.from_text.clone(),
            l.to_text.clone(),
            if l.forward { "ileri" } else { "geri" }.to_owned(),
            grouped(l.volume, 0),
            grouped(l.moment, 0),
            l.average.map_or_else(|| "—".to_owned(), |a| grouped(a, 0)),
            grouped(l.over_volume, 0),
            grouped(l.over_moment, 0),
            grouped(l.unbalanced, 0),
        ];
        rows = rows.push(cells.into_iter().zip(widths).fold(row![].spacing(6), |row, (c, wd)| row.push(label::caption(c).width(wd))));
    }
    column![label::strong("Döngüler"), header, rows].spacing(4).into()
}

/// The mass haul diagram: the curve, the loops shaded by direction (unbalanced apart), the balance line and points,
/// the sites' steps, fitted to the canvas.
struct Diagram<'a> {
    preview: &'a Preview,
}

impl canvas::Program<Message> for Diagram<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let caption = |frame: &mut canvas::Frame, words: String, at: Point, color: Color| {
            frame.fill_text(Text {
                content: words,
                position: at,
                color,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                ..Text::default()
            });
        };
        let Some(rep) = self.preview.report.as_ref().filter(|r| r.points.len() > 1) else {
            caption(&mut frame, "Kübajı seçince Brükner burada çizilir.".into(), Point::new(16.0, 20.0), t.muted);
            return vec![frame.into_geometry()];
        };
        let pts = &rep.points;
        let (s0, s1) = (pts[0][0], pts[pts.len() - 1][0]);
        let (mut lo, mut hi) = (rep.balance, rep.balance);
        for p in pts {
            lo = lo.min(p[1]);
            hi = hi.max(p[1]);
        }
        if hi - lo < 1e-9 {
            hi = lo + 1.0;
        }
        let pad = (hi - lo) * 0.06;
        let (lo, hi) = (lo - pad, hi + pad);
        let (left, gap) = (64.0, 10.0);
        let (wd, ht) = (bounds.width as f64 - left - gap, bounds.height as f64 - 2.0 * gap);
        let at = |s: f64, v: f64| {
            Point::new((left + (s - s0) / (s1 - s0).max(1e-9) * wd) as f32, (gap + (hi - v) / (hi - lo) * ht) as f32)
        };
        // The ordinate at a station: after a step there, or before it.
        let value = |s: f64, after: bool| -> f64 {
            let same: Vec<&[f64; 2]> = pts.iter().filter(|p| p[0] == s).collect();
            if let (Some(a), Some(b)) = (same.first(), same.last()) {
                return if after { b[1] } else { a[1] };
            }
            let i = pts.partition_point(|p| p[0] < s).clamp(1, pts.len() - 1);
            let (p, q) = (pts[i - 1], pts[i]);
            p[1] + (q[1] - p[1]) * (s - p[0]) / (q[0] - p[0])
        };
        let orange = Color::from_rgb8(0xf5, 0xa6, 0x23);
        for l in &rep.loops {
            let color = if l.unbalanced > 0.0 {
                Color { a: 0.35, ..orange }
            } else if l.forward {
                Color { a: 0.28, ..t.danger }
            } else {
                Color { a: 0.28, ..t.accent }
            };
            let shape = Path::new(|b| {
                b.move_to(at(l.from, rep.balance));
                b.line_to(at(l.from, value(l.from, true)));
                for p in pts.iter().filter(|p| p[0] > l.from && p[0] < l.to) {
                    b.line_to(at(p[0], p[1]));
                }
                b.line_to(at(l.to, value(l.to, false)));
                b.line_to(at(l.to, rep.balance));
                b.close();
            });
            frame.fill(&shape, color);
        }
        // Zoomed in a picture (docs/adr/0937 §3): round volumes at least 40 px apart, finer as the diagram is laid out
        // larger, their labels on the part shown.
        let part = crate::picture::Shown::of(bounds.size());
        let step = crate::picture::step(40.0 * (hi - lo) / ht.max(1.0));
        let label_x = part.left(4.0);
        let mut v = (lo / step).ceil() * step;
        while v <= hi + 1e-9 {
            let a = at(s0, v);
            if part.has_y(a.y) {
                frame.stroke(&Path::line(a, at(s1, v)), Stroke::default().with_color(Color { a: 0.3, ..t.border }).with_width(1.0));
                crate::picture::label(&mut frame, grouped(v, 0), Point::new(label_x, a.y - 7.0), t.muted, false, (label_x > 4.5).then_some(t.field.scale_alpha(0.9)));
            }
            v += step;
        }
        frame.stroke(
            &Path::line(at(s0, rep.balance), at(s1, rep.balance)),
            Stroke::default().with_color(t.muted).with_width(1.0),
        );
        let curve = Path::new(|b| {
            for (i, p) in pts.iter().enumerate() {
                if i == 0 { b.move_to(at(p[0], p[1])) } else { b.line_to(at(p[0], p[1])) }
            }
        });
        frame.stroke(&curve, Stroke::default().with_color(t.text).with_width(1.8));
        let green = Color::from_rgb8(0x16, 0xa3, 0x4a);
        for s in &rep.balance_points {
            frame.fill(&Path::circle(at(*s, rep.balance), 3.5), green);
        }
        // The sites: their steps marked.
        for w in pts.windows(2).filter(|w| w[0][0] == w[1][0]) {
            frame.stroke(
                &Path::line(at(w[0][0], w[0][1]), at(w[1][0], w[1][1])),
                Stroke::default().with_color(orange).with_width(3.0),
            );
        }
        vec![frame.into_geometry()]
    }
}
