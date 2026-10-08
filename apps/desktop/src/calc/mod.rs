//! Hesap: the surveying windows (the web's `ui/calc/`, docs/adr/0070,
//! 0071): Poligon hesabı ([`traverse`]), Kutupsal alım ([`polar`]), Önden
//! and Geriden kestirme ([`intersection`]), Aplikasyon ([`stakeout`]),
//! Vektör oturtma ([`fit`], docs/adr/0156), Koordinat dönüştür
//! ([`convert`], docs/adr/0167 §4), Karne editörü ([`fieldbook`],
//! docs/adr/0169 §6), Yatay ağ dengelemesi and Kot ağı dengelemesi
//! ([`network`], docs/adr/0203).
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

pub mod convert;
pub mod edgematch;
pub mod fieldbook;
pub mod fit;
pub mod grid;
pub mod intersection;
pub mod network;
mod parts;
pub mod polar;
pub mod raster_fit;
pub mod read;
pub mod stakeout;
pub mod traverse;

use iced::widget::column;
use iced::{Element, Task};
use kentos_contracts::{CreateOperation, Entity};
use kentos_interaction::calc::SurveyPoint;
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
    "transform.fit",
    "transform.edgematch",
    "crs.transform",
    "calc.fieldbook",
    "calc.network",
    "calc.levelNetwork",
    // docs/adr/0204 §6: Raster oturt (raster_fit.rs).
    "raster.georef",
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
    Fit,
    Edgematch,
    Convert,
    FieldBook,
    /// Yatay ağ dengelemesi and Kot ağı dengelemesi (docs/adr/0203).
    Network,
    Level,
    /// Raster oturt (docs/adr/0204 §6).
    RasterFit,
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
    /// Vektör oturtma's base point (Parametrelerle).
    Base,
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
    /// Aplikasyon's Cihaza gönder: its table's points as an instrument's file (docs/adr/0169 §4).
    SendToDevice,
    /// A table's cell (row, column) typed, pasted, Enter in it; rows added and removed.
    Cell(usize, usize, String),
    Paste(usize, usize, String),
    Pasted(usize, usize, Option<String>),
    Submit(usize, usize),
    /// ↑ (true) or ↓ in the focused field, found by `arrow`.
    Arrow(bool, iced::widget::Id),
    AddRow,
    RemoveRow(usize),
    /// Aplikasyon: the selected points into the table.
    FromSelection,
    /// Vektör oturtma's own controls.
    Fit(fit::Event),
    /// Kenar eşleme's own controls.
    Edgematch(edgematch::Event),
    /// Koordinat dönüştür's own controls.
    Convert(convert::Event),
    /// Karne editörü's own controls.
    FieldBook(fieldbook::Event),
    /// Yatay ağ dengelemesi's and Kot ağı dengelemesi's own controls.
    Network(network::Event),
    /// Raster oturt's own controls.
    RasterFit(raster_fit::Event),
}

/// The windows' state while the app runs (the web's module state).
#[derive(Debug, Default)]
pub struct Calc {
    pub open: Option<Window>,
    pub traverse: traverse::Form,
    pub polar: polar::Form,
    pub intersection: intersection::Form,
    pub stakeout: stakeout::Form,
    pub fit: fit::Form,
    pub edgematch: edgematch::Form,
    pub convert: convert::Form,
    pub fieldbook: fieldbook::Form,
    pub network: network::NetworkForm,
    pub level: network::LevelForm,
    pub raster_fit: raster_fit::Form,
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
            Window::Fit => Some(&mut self.fit),
            Window::Edgematch => Some(&mut self.edgematch),
            Window::Convert => Some(&mut self.convert),
            Window::FieldBook => Some(&mut self.fieldbook),
            Window::RasterFit => Some(&mut self.raster_fit),
            Window::Intersection | Window::Network | Window::Level => None,
        }
    }

    /// Where a window's new points go; Aplikasyon writes nothing.
    fn layer(&mut self, window: Window) -> Option<&mut Option<String>> {
        match window {
            Window::Traverse => Some(&mut self.traverse.layer),
            Window::Polar => Some(&mut self.polar.layer),
            Window::Intersection => Some(&mut self.intersection.layer),
            Window::Network => Some(&mut self.network.layer),
            Window::Stakeout
            | Window::Fit
            | Window::Edgematch
            | Window::Convert
            | Window::FieldBook
            | Window::RasterFit
            | Window::Level => None,
        }
    }
}

fn event(e: Event) -> Message {
    Message::Calc(e)
}

/// ↑ (`up`) or ↓ with a Hesap window open: the focused field is found,
/// then its table moves the keyboard (`grid::arrow`).
pub(crate) fn arrow(up: bool) -> Task<Message> {
    use iced::advanced::widget::{operate, operation::focusable::find_focused};
    operate(find_focused()).map(move |from| event(Event::Arrow(up, from)))
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
            "transform.fit" => self.calc_show(Window::Fit),
            "transform.edgematch" => self.calc_show(Window::Edgematch),
            "crs.transform" => self.calc_show(Window::Convert),
            "calc.fieldbook" => self.calc_show(Window::FieldBook),
            "calc.network" => self.calc_show(Window::Network),
            "calc.levelNetwork" => self.calc_show(Window::Level),
            "raster.georef" => self.calc_show(Window::RasterFit),
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
        if window == Window::Fit {
            self.calc.fit.sync(&doc.model, self.selection.len());
        }
        if window == Window::RasterFit {
            self.calc.raster_fit.sync(&doc.model, self.selection.ids());
        }
        if window == Window::Edgematch {
            self.calc.edgematch.sync(&doc.model, self.selection.len());
            self.calc.edgematch.solve(&doc.model, self.selection.ids());
        }
        // The project's k and tolerances may have changed since it was last shown.
        if window == Window::FieldBook {
            self.calc.fieldbook.sync(doc.settings());
        }
        self.calc.open = Some(window);
        self.dialog = Some(Asking::Calc);
        // The adjustments are solved again on the drawing as it is now.
        self.network_solve();
    }

    pub(crate) fn calc_event(&mut self, e: Event) -> Task<Message> {
        // Raster oturt's resampling ends after its window closed (rasters/jobs.rs).
        if let Event::RasterFit(e @ raster_fit::Event::Warped(_)) = e {
            return self.raster_fit_event(e);
        }
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
                self.calc.picking = Some((window, field));
                self.calc_pick(title, field_label(window, field).to_owned());
                Task::none()
            }
            Event::Known(field, text) => {
                if let Some(known) = self.calc_known_text(window, field) {
                    *known = text;
                }
                Task::none()
            }
            Event::Fit(e) => {
                self.fit_event(e);
                Task::none()
            }
            Event::Edgematch(e) => {
                self.edgematch_event(e);
                Task::none()
            }
            Event::Convert(e) => self.convert_event(e),
            Event::FieldBook(e) => self.fieldbook_event(e),
            Event::Network(e) => self.network_event(e),
            Event::RasterFit(e) => self.raster_fit_event(e),
            Event::CopyReport => self.calc_copy_report(window),
            Event::SendToDevice => self.calc_send_stakeout(),
            Event::AddPoints => {
                match window {
                    Window::Network => self.network_write(),
                    Window::Level => self.level_write(),
                    _ => self.calc_add_points(window),
                }
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
            Event::Arrow(up, from) => match self.calc.table(window) {
                Some(table) => grid::arrow(table, window, &from, up),
                None if matches!(window, Window::Network | Window::Level) => {
                    self.network_arrow(up, &from)
                }
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
        // table is on every change; Vektör oturtma solves again.
        if window == Window::Traverse
            && let Some(doc) = &self.document
        {
            self.calc.traverse.sync(&doc.model);
        }
        if window == Window::Fit {
            self.calc.fit.solve();
        }
        if window == Window::RasterFit {
            self.calc.raster_fit.solve();
        }
        // Karne editörü reduces the station again (Kullan, a name).
        if window == Window::FieldBook
            && let Some(doc) = &self.document
        {
            self.calc.fieldbook.sync(doc.settings());
        }
        // Kenar eşleme finds its links again (the window may have closed: Göster, Sınır's pick).
        if window == Window::Edgematch
            && self.calc.open == Some(Window::Edgematch)
            && let Some(doc) = &self.document
        {
            self.calc.edgematch.solve(&doc.model, self.selection.ids());
        }
        task
    }

    /// Çizimden: the window closes and the pick tool asks for `label`'s point.
    fn calc_pick(&mut self, title: &'static str, label: String) {
        self.dialog = None;
        self.field = None;
        self.snap = None;
        let said = format!("{title}: {label}");
        self.session.run(Box::new(PickPoint::new(title, label)));
        self.with_tool(|s, cx| s.activate(cx));
        self.say(Level::Command, said);
    }

    fn calc_title(&self, window: Window) -> &'static str {
        match window {
            Window::Traverse => traverse::TITLE,
            Window::Polar => polar::TITLE,
            Window::Intersection => self.calc.intersection.kind.title(),
            Window::Stakeout => stakeout::TITLE,
            Window::Fit => fit::TITLE,
            Window::Edgematch => edgematch::TITLE,
            Window::Convert => convert::TITLE,
            Window::FieldBook => fieldbook::TITLE,
            Window::Network => network::NETWORK_TITLE,
            Window::Level => network::LEVEL_TITLE,
            Window::RasterFit => raster_fit::TITLE,
        }
    }

    /// A known point field's text; Vektör oturtma has none.
    fn calc_known_text(&mut self, window: Window, field: Field) -> Option<&mut String> {
        let calc = &mut self.calc;
        Some(match (window, field) {
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
            (Window::Fit, Field::Base) => &mut calc.fit.params.base,
            (Window::Fit, _) => return None,
            (Window::Edgematch, _) => return None,
            (Window::Convert, _) => return None,
            (Window::FieldBook | Window::Network | Window::Level | Window::RasterFit, _) => {
                return None;
            }
        })
    }

    /// Çizimden's answer: the field takes the point's name when it lies on a
    /// named point, else its coordinates; the window opens again either way.
    pub(crate) fn calc_picked(&mut self, p: Option<Vec2>) {
        // Kenar eşleme's Göster: the look is over.
        if self.edgematch_looked() {
            return;
        }
        // Koordinat dönüştür's point: its coordinates in the two fields.
        if self.calc.convert.picking {
            self.convert_picked(p);
            self.calc_show(Window::Convert);
            return;
        }
        // Raster oturt's raster or row: the window opens again with it.
        if self.raster_fit_picked(p) {
            self.calc_show(Window::RasterFit);
            return;
        }
        // Vektör oturtma's row: its source or target, and the name it snapped to.
        if let Some((row, side)) = self.calc.fit.picking.take() {
            if let (Some(p), Some(doc)) = (p, &self.document) {
                let name = read::name_at(&doc.model, p);
                self.calc.fit.picked(row, side, p, name);
            }
            self.calc_show(Window::Fit);
            return;
        }
        let Some((window, field)) = self.calc.picking.take() else {
            return;
        };
        if let (Some(p), Some(doc)) = (p, &self.document) {
            let text = read::name_at(&doc.model, p).unwrap_or_else(|| format!("{},{}", p.x, p.y));
            if let Some(known) = self.calc_known_text(window, field) {
                *known = text;
            }
        }
        self.calc_show(window);
    }

    /// The report, tab-separated lines, to the system clipboard (the web's `copyReport`).
    /// Cihaza gönder (docs/adr/0169 §4): Aplikasyon's table as an instrument's
    /// coordinate file, the drawing's points with their codes and elevations;
    /// a point typed as Y,X has no name, and the window says so.
    fn calc_send_stakeout(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let mut points = Vec::new();
        for text in &self.calc.stakeout.rows {
            if let read::Known::Point { p, name } = read::resolve_point(&doc.model, text) {
                let of_drawing = (!name.is_empty())
                    .then(|| read::point_named(&doc.model, text))
                    .flatten()
                    .and_then(crate::exchange::field_send::field_point);
                points.push(of_drawing.unwrap_or(kentos_contracts::FieldPoint {
                    name,
                    east: p.x,
                    north: p.y,
                    elevation: None,
                    code: None,
                }));
            }
        }
        if points.is_empty() {
            self.warn(
                "Tabloda gönderilecek nokta yok: noktaları adlarıyla ya da Y,X olarak yazın.",
            );
            return Task::none();
        }
        let from = format!("Aplikasyon tablosunun {} noktası", points.len());
        self.open_field_send(points, from)
    }

    fn calc_copy_report(&mut self, window: Window) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        // The Hesap windows are in metres, as typed (docs/adr/0165 §2).
        let format = Format::of(doc.settings()).metric();
        let title = self.calc_title(window);
        let model = &doc.model;
        let lines = match window {
            Window::Traverse => self.calc.traverse.report(model, &format),
            Window::Polar => self.calc.polar.report(model, &format),
            Window::Intersection => self.calc.intersection.report(model, &format),
            Window::Stakeout => self.calc.stakeout.report(model, &format),
            Window::Fit => self.calc.fit.report(model, &format),
            Window::Edgematch => self.calc.edgematch.report(model, self.selection.len()),
            // Koordinat dönüştür copies its values itself (Panoya kopyala).
            Window::Convert => None,
            Window::FieldBook => self.calc.fieldbook.report(),
            Window::Network => self.calc.network.report(model, &format),
            Window::Level => self.calc.level.report(model, &format),
            Window::RasterFit => None,
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
        let (points, kind, operation, layer) = match window {
            Window::Traverse => (
                calc.traverse.points(model),
                "Poligon noktası",
                CreateOperation::Traverse,
                &calc.traverse.layer,
            ),
            Window::Polar => (
                calc.polar.points(model),
                "Alım noktası",
                CreateOperation::PolarSurvey,
                &calc.polar.layer,
            ),
            Window::Intersection => {
                let Some(found) = calc.intersection.compute(model).result else {
                    return;
                };
                let point = NewPoint {
                    name: found.name,
                    p: found.p,
                    z: None,
                };
                let operation = match calc.intersection.kind {
                    intersection::Kind::Forward => CreateOperation::ForwardIntersection,
                    intersection::Kind::Resection => CreateOperation::Resection,
                };
                (
                    vec![point],
                    "Kestirme noktası",
                    operation,
                    &calc.intersection.layer,
                )
            }
            Window::Stakeout
            | Window::Fit
            | Window::Edgematch
            | Window::Convert
            | Window::FieldBook
            | Window::Network
            | Window::Level
            | Window::RasterFit => return,
        };
        let (Some(layer), false) = (layer.clone(), points.is_empty()) else {
            return;
        };
        let step = self.calc_title(window);
        if !self.add_points(&layer, &points, kind, operation) {
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

    /// Adds points to `layer` through `cad.entities.create`, one undo step
    /// named after the window (`operation`), with their names as labels and
    /// Ad, Tür and Z (m) as attributes, and selects them (the web's
    /// `addPoints`). A locked or hidden layer is said in the window's own
    /// words. False, with the reason said, when the layer cannot take them.
    pub(crate) fn add_points(
        &mut self,
        layer: &str,
        points: &[NewPoint],
        kind: &str,
        operation: CreateOperation,
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
        let points: Vec<SurveyPoint> = points
            .iter()
            .map(|pt| SurveyPoint {
                name: pt.name.clone(),
                p: pt.p,
                z: pt.z,
            })
            .collect();
        let written = kentos_interaction::calc::add_points(model, layer, &points, kind, operation);
        let hidden = !model.layers().is_visible(layer);
        let (slots, warnings) = match written {
            Ok(done) => done,
            Err(reason) => {
                self.warn(reason);
                return false;
            }
        };
        for warning in warnings {
            self.warn(warning);
        }
        self.selection.set(slots);
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
        // The Hesap windows are in metres, as typed (docs/adr/0165 §2).
        let format = Format::of(doc.settings()).metric();
        let model = &doc.model;
        let dialog = match window {
            Window::Traverse => self.calc.traverse.view(model, &format),
            Window::Polar => self.calc.polar.view(model, &format),
            Window::Intersection => self.calc.intersection.view(model, &format),
            Window::Stakeout => self.calc.stakeout.view(model, &format),
            Window::Fit => self.calc.fit.view(model, &format, self.selection.len()),
            Window::Edgematch => self.calc.edgematch.view(model, self.selection.len()),
            // Coordinates as the project writes them: its axes and digits, the user's notation.
            Window::Convert => self.calc.convert.view(
                doc.settings(),
                &convert::ConvertFormat::of(doc.settings(), self.draft.geographic),
            ),
            Window::FieldBook => self.calc.fieldbook.view(model, &format),
            Window::Network => self.calc.network.view(model, &format),
            Window::Level => self.calc.level.view(model, &format),
            Window::RasterFit => self.calc.raster_fit.view(model),
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
        (Window::Fit, _) => "Taban noktası",
        (Window::Edgematch, _) => "Sınır",
        (Window::Convert, _) => "Nokta",
        (Window::FieldBook | Window::Network | Window::Level, _) => "Nokta",
        (Window::RasterFit, _) => "Nokta",
    }
}

#[cfg(test)]
mod tests;
