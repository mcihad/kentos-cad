//! Hesap: the surveying windows (the web's `ui/calc/`, docs/adr/0070,
//! 0071): Poligon hesabı ([`traverse`]), Kutupsal alım ([`polar`]), Önden
//! and Geriden kestirme ([`intersection`]) and Aplikasyon ([`stakeout`]).
//! What they share, as the web's `common.ts`:
//!
//! - a known point: a point object's name in the drawing or “Y,X”, or shown
//!   on the drawing (Çizimden: the window closes, the pick tool runs, the
//!   window opens again as it was, the point's name when it snapped to a
//!   named point, else its coordinates);
//! - numbers read as the web reads them ([`read`]);
//! - the measurements table ([`grid`]): Enter goes down, rows are added and
//!   removed, lines pasted from a spreadsheet fill down and right;
//! - a summary of at most six problems, then the results;
//! - the report copied as tab-separated lines, for a spreadsheet;
//! - new points added as one undo step named after the window, with their
//!   names as labels and Ad, Tür, Z (m) as attributes, then selected.
//!
//! What is typed stays while the app runs, whatever drawing is open, as on
//! the web; nothing of it is saved.

pub mod grid;
pub mod intersection;
mod parts;
pub mod polar;
pub mod read;
pub mod stakeout;
pub mod traverse;

use std::collections::BTreeMap;

use iced::widget::column;
use iced::{Element, Task};
use kentos_contracts::{Entity, EntityBase, PointEntity, Vec2 as Wire};
use kentos_domain::Slot;
use kentos_interaction::pick::PickPoint;
use kentos_interaction::{Format, Level, Vec2, fixed};

use crate::app::{App, Dialog as Asking, Message};

use grid::Table;
pub(crate) use parts::{
    footer, footer_button, known_field, knowns, leaves, number_field, result_table, summary,
    text_field,
};

/// The commands the windows answer.
pub const COMMANDS: &[&str] = &[
    "calc.traverse",
    "calc.polar",
    "calc.forward",
    "calc.resection",
    "calc.stakeout",
];

/// The windows' greatest height: the web's body of at most 760 px with the
/// heading and the buttons; in a lower window the body scrolls.
pub(crate) const MAX_HEIGHT: f32 = 880.0;

/// A Hesap window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    Traverse,
    Polar,
    Intersection,
    Stakeout,
}

/// A known point field of the window it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// Kestirme's known points.
    A,
    B,
    C,
    /// Kutupsal alım's and Aplikasyon's station.
    Station,
    /// The point looked at: from the station, or from a traverse's start.
    Back,
    /// Poligon hesabı's start, end and the point looked at from the end.
    Start,
    End,
    Fore,
}

/// What the windows ask for.
#[derive(Clone, Debug)]
pub enum Event {
    Close,
    /// A known point's text.
    Known(Field, String),
    /// Çizimden: the window closes and the point is shown on the drawing.
    Pick(Field),
    /// Kestirme's kind, angles and new point's name.
    Kind(intersection::Kind),
    Alpha(String),
    Beta(String),
    Name(String),
    /// Poligon hesabı's kind, and whether the angle at the end was measured.
    TraverseKind(traverse::Kind),
    EndOriented(bool),
    /// Kutupsal alım's reading to the back point and heights.
    BackReading(String),
    StationZ(String),
    InstrumentHeight(String),
    /// Where new points go: a layer's index in the leaves.
    Layer(usize),
    AddPoints,
    CopyReport,
    /// A table's cell (row, column) typed, pasted, Enter in it; rows added and removed.
    Cell(usize, usize, String),
    Paste(usize, usize, String),
    Pasted(usize, usize, Option<String>),
    Submit(usize, usize),
    AddRow,
    RemoveRow(usize),
    /// Aplikasyon: the selected points into the table.
    FromSelection,
}

/// The windows' state while the app runs (the web's module state).
#[derive(Debug, Default)]
pub struct Calc {
    pub open: Option<Window>,
    pub traverse: traverse::Form,
    pub polar: polar::Form,
    pub intersection: intersection::Form,
    pub stakeout: stakeout::Form,
    /// The field Çizimden picks for, while its window is closed.
    picking: Option<(Window, Field)>,
}

impl Calc {
    /// A window's measurements table; Kestirme has none.
    fn table(&mut self, window: Window) -> Option<&mut dyn Table> {
        match window {
            Window::Traverse => Some(&mut self.traverse),
            Window::Polar => Some(&mut self.polar),
            Window::Stakeout => Some(&mut self.stakeout),
            Window::Intersection => None,
        }
    }

    /// Where a window's new points go; Aplikasyon writes nothing.
    fn layer(&mut self, window: Window) -> Option<&mut Option<String>> {
        match window {
            Window::Traverse => Some(&mut self.traverse.layer),
            Window::Polar => Some(&mut self.polar.layer),
            Window::Intersection => Some(&mut self.intersection.layer),
            Window::Stakeout => None,
        }
    }
}

fn event(e: Event) -> Message {
    Message::Calc(e)
}

/// An angle already in the project's unit, with its mark (the web's `angleText`).
pub fn angle_text(format: &Format, v: f64) -> String {
    let mark = if format.angle_unit_label() == "°" {
        "°"
    } else {
        " g"
    };
    format!("{}{mark}", fixed(v, 4))
}

impl App {
    /// `calc.*`: the window opens as it was left.
    pub(crate) fn calc_command(&mut self, id: &'static str) -> Task<Message> {
        if self.document.is_none() {
            self.warn("Hesap için önce bir çizim açın.");
            return Task::none();
        }
        match id {
            "calc.traverse" => self.calc_show(Window::Traverse),
            "calc.polar" => self.calc_show(Window::Polar),
            "calc.forward" | "calc.resection" => {
                self.calc.intersection.kind = if id == "calc.forward" {
                    intersection::Kind::Forward
                } else {
                    intersection::Kind::Resection
                };
                self.calc_show(Window::Intersection);
            }
            "calc.stakeout" => self.calc_show(Window::Stakeout),
            _ => {}
        }
        Task::none()
    }

    /// The window shown, as the web builds it on opening: the layer chosen
    /// stays while the drawing has it, else the window's own (`poligon`
    /// where there is one; the active layer); a traverse's station rows
    /// take the known points' names.
    fn calc_show(&mut self, window: Window) {
        let Some(doc) = &self.document else {
            return;
        };
        let layers = doc.model.layers();
        let preferred = match window {
            Window::Polar => None,
            _ => Some("poligon"),
        };
        if let Some(layer) = self.calc.layer(window)
            && layer.as_deref().is_none_or(|id| layers.get(id).is_none())
        {
            *layer = Some(
                preferred
                    .filter(|id| layers.get(id).is_some())
                    .unwrap_or(layers.active())
                    .to_owned(),
            );
        }
        if window == Window::Traverse {
            self.calc.traverse.sync(&doc.model);
        }
        self.calc.open = Some(window);
        self.dialog = Some(Asking::Calc);
    }

    pub(crate) fn calc_event(&mut self, e: Event) -> Task<Message> {
        let Some(window) = self.calc.open else {
            return Task::none();
        };
        let task = match e {
            Event::Close => {
                self.calc.open = None;
                self.dialog = None;
                Task::none()
            }
            Event::Pick(field) => {
                let title = self.calc_title(window);
                let label = field_label(window, field);
                self.calc.picking = Some((window, field));
                self.dialog = None;
                self.field = None;
                self.snap = None;
                self.session.run(Box::new(PickPoint::new(title, label)));
                self.with_tool(|s, cx| s.activate(cx));
                self.say(Level::Command, format!("{title}: {label}"));
                Task::none()
            }
            Event::Known(field, text) => {
                *self.calc_known_text(window, field) = text;
                Task::none()
            }
            Event::CopyReport => self.calc_copy_report(window),
            Event::AddPoints => {
                self.calc_add_points(window);
                Task::none()
            }
            Event::Layer(i) => {
                if let Some(doc) = &self.document
                    && let Some((id, _, false)) = leaves(&doc.model).get(i)
                    && let Some(layer) = self.calc.layer(window)
                {
                    *layer = Some(id.clone());
                }
                Task::none()
            }
            Event::Kind(kind) => {
                self.calc.intersection.kind = kind;
                Task::none()
            }
            Event::Alpha(t) => {
                self.calc.intersection.alpha = t;
                Task::none()
            }
            Event::Beta(t) => {
                self.calc.intersection.beta = t;
                Task::none()
            }
            Event::Name(t) => {
                self.calc.intersection.name = t;
                Task::none()
            }
            Event::TraverseKind(kind) => {
                self.calc.traverse.kind = kind;
                Task::none()
            }
            Event::EndOriented(on) => {
                self.calc.traverse.end_oriented = on;
                Task::none()
            }
            Event::BackReading(t) => {
                self.calc.polar.back_reading = t;
                Task::none()
            }
            Event::StationZ(t) => {
                self.calc.polar.station_z = t;
                Task::none()
            }
            Event::InstrumentHeight(t) => {
                self.calc.polar.instrument_height = t;
                Task::none()
            }
            Event::Cell(row, col, text) => {
                if let Some(table) = self.calc.table(window) {
                    table.set(row, col, text);
                }
                Task::none()
            }
            Event::Paste(row, col, contents) => {
                // The field's own paste stands until the clipboard says it held a table.
                if let Some(table) = self.calc.table(window) {
                    table.set(row, col, contents);
                }
                iced::clipboard::read().map(move |t| event(Event::Pasted(row, col, t)))
            }
            Event::Pasted(row, col, raw) => match (raw, self.calc.table(window)) {
                (Some(raw), Some(table)) => match grid::paste(table, row, col, &raw) {
                    Some(at) => iced::widget::operation::focus(grid::cell_id(window, at, col)),
                    None => Task::none(),
                },
                _ => Task::none(),
            },
            Event::Submit(row, col) => match self.calc.table(window) {
                Some(table) => grid::submit(table, window, row, col),
                None => Task::none(),
            },
            Event::AddRow => match self.calc.table(window) {
                Some(table) => grid::add(table, window),
                None => Task::none(),
            },
            Event::RemoveRow(row) => {
                if let Some(table) = self.calc.table(window)
                    && table.can_remove(row)
                {
                    table.remove(row);
                }
                Task::none()
            }
            Event::FromSelection => {
                self.calc_from_selection();
                Task::none()
            }
        };
        // The station rows are named after the known points, as the web's
        // table is on every change.
        if window == Window::Traverse
            && let Some(doc) = &self.document
        {
            self.calc.traverse.sync(&doc.model);
        }
        task
    }

    fn calc_title(&self, window: Window) -> &'static str {
        match window {
            Window::Traverse => traverse::TITLE,
            Window::Polar => polar::TITLE,
            Window::Intersection => self.calc.intersection.kind.title(),
            Window::Stakeout => stakeout::TITLE,
        }
    }

    fn calc_known_text(&mut self, window: Window, field: Field) -> &mut String {
        let calc = &mut self.calc;
        match (window, field) {
            (Window::Traverse, Field::Back) => &mut calc.traverse.back,
            (Window::Traverse, Field::End) => &mut calc.traverse.end,
            (Window::Traverse, Field::Fore) => &mut calc.traverse.fore,
            (Window::Traverse, _) => &mut calc.traverse.start,
            (Window::Polar, Field::Back) => &mut calc.polar.back,
            (Window::Polar, _) => &mut calc.polar.station,
            (Window::Intersection, Field::B) => &mut calc.intersection.b,
            (Window::Intersection, Field::C) => &mut calc.intersection.c,
            (Window::Intersection, _) => &mut calc.intersection.a,
            (Window::Stakeout, Field::Back) => &mut calc.stakeout.back,
            (Window::Stakeout, _) => &mut calc.stakeout.station,
        }
    }

    /// Çizimden's answer: the field takes the point's name when it lies on a
    /// named point, else its coordinates; the window opens again either way.
    pub(crate) fn calc_picked(&mut self, p: Option<Vec2>) {
        let Some((window, field)) = self.calc.picking.take() else {
            return;
        };
        if let (Some(p), Some(doc)) = (p, &self.document) {
            let text = read::name_at(&doc.model, p).unwrap_or_else(|| format!("{},{}", p.x, p.y));
            *self.calc_known_text(window, field) = text;
        }
        self.calc_show(window);
    }

    /// The report, tab-separated lines, to the system clipboard (the web's `copyReport`).
    fn calc_copy_report(&mut self, window: Window) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let format = Format::of(doc.settings());
        let title = self.calc_title(window);
        let model = &doc.model;
        let lines = match window {
            Window::Traverse => self.calc.traverse.report(model, &format),
            Window::Polar => self.calc.polar.report(model, &format),
            Window::Intersection => self.calc.intersection.report(model, &format),
            Window::Stakeout => self.calc.stakeout.report(model, &format),
        };
        let Some(lines) = lines else {
            return Task::none();
        };
        let text = lines
            .iter()
            .map(|l| l.join("\t"))
            .collect::<Vec<_>>()
            .join("\n");
        self.say(
            Level::Success,
            format!(
                "{title} raporu panoya kopyalandı ({} satır; elektronik tabloya yapıştırılabilir).",
                lines.len()
            ),
        );
        iced::clipboard::write(text)
    }

    /// Çizime ekle: the window's new points in one step, named after it; the
    /// window closes (the web's `addToDrawing`).
    fn calc_add_points(&mut self, window: Window) {
        let Some(doc) = &self.document else {
            return;
        };
        let model = &doc.model;
        let calc = &self.calc;
        let (points, kind, layer) = match window {
            Window::Traverse => (
                calc.traverse.points(model),
                "Poligon noktası",
                &calc.traverse.layer,
            ),
            Window::Polar => (calc.polar.points(model), "Alım noktası", &calc.polar.layer),
            Window::Intersection => {
                let Some(found) = calc.intersection.compute(model).result else {
                    return;
                };
                let point = NewPoint {
                    name: found.name,
                    p: found.p,
                    z: None,
                };
                (vec![point], "Kestirme noktası", &calc.intersection.layer)
            }
            Window::Stakeout => return,
        };
        let (Some(layer), false) = (layer.clone(), points.is_empty()) else {
            return;
        };
        let step = self.calc_title(window);
        if !self.add_points(&layer, &points, kind, step) {
            return;
        }
        let n = points.len();
        let said = match window {
            Window::Traverse => {
                format!("{step}: {n} poligon noktası çizime eklendi (Ctrl+Z geri alır).")
            }
            Window::Polar => format!("{step}: {n} nokta çizime eklendi (Ctrl+Z geri alır)."),
            _ => format!(
                "{step}: {} noktası çizime eklendi (Ctrl+Z geri alır).",
                points[0].name
            ),
        };
        self.say(Level::Success, said);
        self.calc.open = None;
        self.dialog = None;
    }

    /// Aplikasyon's “Seçili noktaları ekle”: the selected points, by name
    /// or coordinates, after the rows already filled.
    fn calc_from_selection(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        let picked: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter_map(|&s| match doc.model.get(s) {
                Some(Entity::Point(p)) => Some(
                    p.base
                        .label
                        .clone()
                        .unwrap_or_else(|| format!("{},{}", p.p.x, p.p.y)),
                ),
                _ => None,
            })
            .collect();
        if picked.is_empty() {
            self.warn("Seçili nokta yok: aplike edilecek noktaları seçip pencereyi yeniden açın.");
            return;
        }
        self.calc.stakeout.append(picked);
    }

    /// Adds points to `layer` as one undo step named `step`, with their names
    /// as labels and Ad, Tür and Z (m) as attributes, and selects them (the
    /// web's `addPoints`). False, with the reason said, when the layer cannot
    /// take them.
    pub(crate) fn add_points(
        &mut self,
        layer: &str,
        points: &[NewPoint],
        kind: &str,
        step: &str,
    ) -> bool {
        let Some(doc) = &mut self.document else {
            return false;
        };
        let model = &mut doc.model;
        let Some(name) = model.layers().get(layer).map(|n| n.name.clone()) else {
            return false;
        };
        if model.layers().is_locked(layer) {
            self.warn(format!(
                "“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katman seçin."
            ));
            return false;
        }
        let entities: Vec<Entity> = points
            .iter()
            .map(|pt| {
                let mut attrs = BTreeMap::from([
                    ("Ad".to_owned(), pt.name.clone()),
                    ("Tür".to_owned(), kind.to_owned()),
                ]);
                if let Some(z) = pt.z {
                    attrs.insert("Z (m)".to_owned(), fixed(z, 3));
                }
                Entity::Point(PointEntity {
                    base: EntityBase {
                        id: 0,
                        layer_id: layer.to_owned(),
                        color: None,
                        attrs,
                        label: Some(pt.name.clone()),
                        symbol: None,
                    },
                    p: Wire {
                        x: pt.p.x,
                        y: pt.p.y,
                    },
                    z: pt.z,
                })
            })
            .collect();
        let added = model.transact(step, |doc| {
            entities
                .into_iter()
                .map(|e| doc.add(e))
                .collect::<Result<Vec<Slot>, _>>()
        });
        let hidden = !model.layers().is_visible(layer);
        match added {
            Ok(slots) => self.selection.set(slots),
            Err(e) => {
                self.error(e.to_string());
                return false;
            }
        }
        if hidden {
            self.warn(format!(
                "“{name}” katmanı gizli; eklenen noktalar görünmüyor."
            ));
        }
        true
    }

    /// The open window.
    pub(crate) fn calc_view(&self) -> Element<'_, Message> {
        let (Some(window), Some(doc)) = (self.calc.open, &self.document) else {
            return column![].into();
        };
        let format = Format::of(doc.settings());
        let model = &doc.model;
        let dialog = match window {
            Window::Traverse => self.calc.traverse.view(model, &format),
            Window::Polar => self.calc.polar.view(model, &format),
            Window::Intersection => self.calc.intersection.view(model, &format),
            Window::Stakeout => self.calc.stakeout.view(model, &format),
        };
        kentos_ui::widget::overlay::modal(dialog, event(Event::Close))
    }
}

/// A point the windows add to the drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct NewPoint {
    pub name: String,
    pub p: Vec2,
    pub z: Option<f64>,
}

/// The label of a known field, as the prompt of Çizimden names it.
fn field_label(window: Window, field: Field) -> &'static str {
    match (window, field) {
        (Window::Traverse, Field::Back) => "Başlangıçta bakılan nokta",
        (Window::Traverse, Field::End) => "Bitiş noktası (B)",
        (Window::Traverse, Field::Fore) => "Bitişte bakılan nokta",
        (Window::Traverse, _) => "Başlangıç noktası (A)",
        (Window::Polar | Window::Stakeout, Field::Back) => "Bakılan nokta",
        (Window::Polar | Window::Stakeout, _) => "Durulan nokta (istasyon)",
        (Window::Intersection, Field::B) => "B noktası",
        (Window::Intersection, Field::C) => "C noktası",
        (Window::Intersection, _) => "A noktası",
    }
}

#[cfg(test)]
mod tests;
