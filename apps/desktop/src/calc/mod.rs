//! Hesap: the surveying windows (the web's `ui/calc/`, docs/adr/0070),
//! Önden and Geriden kestirme ([`intersection`]) and Aplikasyon
//! ([`stakeout`]). What they share, as the web's `common.ts`:
//!
//! - a known point: a point object's name in the drawing or “Y,X”, or shown
//!   on the drawing (Çizimden: the window closes, the pick tool runs, the
//!   window opens again as it was, the point's name when it snapped to a
//!   named point, else its coordinates);
//! - numbers read as the web reads them ([`read`]);
//! - a summary of at most six problems, then the results;
//! - the report copied as tab-separated lines, for a spreadsheet;
//! - new points added as one undo step named after the window, with their
//!   names as labels and Ad, Tür, Z (m) as attributes, then selected.
//!
//! What is typed stays while the app runs, whatever drawing is open, as on
//! the web; nothing of it is saved.

pub mod intersection;
pub mod read;
pub mod stakeout;

use std::collections::BTreeMap;

use iced::widget::{Column, button, column, container, row, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{Entity, EntityBase, PointEntity, Vec2 as Wire};
use kentos_domain::Slot;
use kentos_interaction::pick::PickPoint;
use kentos_interaction::{Format, Level, Vec2, fixed};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::select::{Choice, Select};

use crate::app::{App, Dialog as Asking, Message};
use crate::exchange::words::{self, Kind as Line};

pub use read::Known;

/// The commands the windows answer.
pub const COMMANDS: &[&str] = &["calc.forward", "calc.resection", "calc.stakeout"];

/// A Hesap window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    Intersection,
    Stakeout,
}

/// A known point field of the window it is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    A,
    B,
    C,
    Station,
    Back,
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
    /// Where new points go: a layer's index in the leaves.
    Layer(usize),
    AddPoints,
    CopyReport,
    /// Aplikasyon's table: a cell typed, pasted, Enter in it; rows added and removed.
    Cell(usize, String),
    Paste(usize, String),
    Pasted(usize, Option<String>),
    Submit(usize),
    AddRow,
    RemoveRow(usize),
    FromSelection,
}

/// The windows' state while the app runs (the web's module state).
#[derive(Debug, Default)]
pub struct Calc {
    pub open: Option<Window>,
    pub intersection: intersection::Form,
    pub stakeout: stakeout::Form,
    /// The field Çizimden picks for, while its window is closed.
    picking: Option<(Window, Field)>,
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
        match id {
            "calc.forward" => {
                self.calc_open(Window::Intersection, Some(intersection::Kind::Forward))
            }
            "calc.resection" => {
                self.calc_open(Window::Intersection, Some(intersection::Kind::Resection))
            }
            "calc.stakeout" => self.calc_open(Window::Stakeout, None),
            _ => {}
        }
        Task::none()
    }

    fn calc_open(&mut self, window: Window, kind: Option<intersection::Kind>) {
        if self.document.is_none() {
            self.warn("Hesap için önce bir çizim açın.");
            return;
        }
        if let Some(kind) = kind {
            self.calc.intersection.kind = kind;
        }
        self.calc.open = Some(window);
        self.dialog = Some(Asking::Calc);
    }

    pub(crate) fn calc_event(&mut self, e: Event) -> Task<Message> {
        let Some(window) = self.calc.open else {
            return Task::none();
        };
        match e {
            Event::Close => {
                self.calc.open = None;
                self.dialog = None;
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
            }
            Event::Known(field, text) => *self.calc_known_text(window, field) = text,
            Event::CopyReport => return self.calc_copy_report(window),
            Event::AddPoints => self.calc_add_points(window),
            Event::Layer(i) => {
                if let Some(doc) = &self.document {
                    let leaves = leaves(&doc.model);
                    if let Some((id, _, locked)) = leaves.get(i)
                        && !locked
                    {
                        self.calc.intersection.layer = Some(id.clone());
                    }
                }
            }
            Event::Kind(kind) => self.calc.intersection.kind = kind,
            Event::Alpha(t) => self.calc.intersection.alpha = t,
            Event::Beta(t) => self.calc.intersection.beta = t,
            Event::Name(t) => self.calc.intersection.name = t,
            Event::Cell(row, text) => self.calc.stakeout.set(row, text),
            Event::Paste(row, contents) => {
                // The field's own paste stands until the clipboard says it held a table.
                self.calc.stakeout.set(row, contents);
                return iced::clipboard::read().map(move |t| event(Event::Pasted(row, t)));
            }
            Event::Pasted(row, raw) => {
                if let Some(raw) = raw {
                    self.calc.stakeout.paste(row, &raw);
                }
            }
            Event::Submit(row) => return self.calc.stakeout.submit(row),
            Event::AddRow => {
                let row = self.calc.stakeout.add_row();
                return iced::widget::operation::focus(stakeout::cell_id(row));
            }
            Event::RemoveRow(row) => self.calc.stakeout.remove_row(row),
            Event::FromSelection => self.calc_from_selection(),
        }
        Task::none()
    }

    fn calc_title(&self, window: Window) -> &'static str {
        match window {
            Window::Intersection => self.calc.intersection.kind.title(),
            Window::Stakeout => stakeout::TITLE,
        }
    }

    fn calc_known_text(&mut self, window: Window, field: Field) -> &mut String {
        match (window, field) {
            (Window::Intersection, Field::B) => &mut self.calc.intersection.b,
            (Window::Intersection, Field::C) => &mut self.calc.intersection.c,
            (Window::Intersection, _) => &mut self.calc.intersection.a,
            (Window::Stakeout, Field::Back) => &mut self.calc.stakeout.back,
            (Window::Stakeout, _) => &mut self.calc.stakeout.station,
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
        self.calc.open = Some(window);
        self.dialog = Some(Asking::Calc);
    }

    /// The report, tab-separated lines, to the system clipboard (the web's `copyReport`).
    fn calc_copy_report(&mut self, window: Window) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let format = Format::of(doc.settings());
        let title = self.calc_title(window);
        let lines = match window {
            Window::Intersection => self.calc.intersection.report(&doc.model, &format),
            Window::Stakeout => self.calc.stakeout.report(&doc.model, &format),
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

    fn calc_add_points(&mut self, window: Window) {
        let Window::Intersection = window else {
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let form = &self.calc.intersection;
        let Some(result) = form.compute(&doc.model).result else {
            return;
        };
        let layer = form.layer_or_default(&doc.model);
        let title = form.kind.title();
        let point = NewPoint {
            name: result.name.clone(),
            p: result.p,
            z: None,
        };
        if self.add_points(&layer, &[point], "Kestirme noktası", title) {
            self.say(
                Level::Success,
                format!(
                    "{title}: {} noktası çizime eklendi (Ctrl+Z geri alır).",
                    result.name
                ),
            );
            self.calc.open = None;
            self.dialog = None;
        }
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
        let dialog = match window {
            Window::Intersection => self.calc.intersection.view(&doc.model, &format),
            Window::Stakeout => self.calc.stakeout.view(&doc.model, &format),
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
        (Window::Stakeout, Field::Back) => "Bakılan nokta",
        (Window::Stakeout, _) => "Durulan nokta (istasyon)",
        (Window::Intersection, Field::B) => "B noktası",
        (Window::Intersection, Field::C) => "C noktası",
        (Window::Intersection, _) => "A noktası",
    }
}

/// The layers points may go to, in tree order, as the layer select lists
/// them: id, name, locked.
pub(crate) fn leaves(model: &kentos_domain::Document) -> Vec<(String, String, bool)> {
    let layers = model.layers();
    layers
        .leaves()
        .into_iter()
        .map(|node| {
            (
                node.id.clone(),
                node.name.clone(),
                layers.is_locked(&node.id),
            )
        })
        .collect()
}

/// A known point field (the web's `knownField`): the text, Çizimden, and
/// under them what it resolves to: the point, the error, or the hint.
pub(crate) fn known_field<'a>(
    model: &kentos_domain::Document,
    format: &Format,
    title: &'a str,
    field: Field,
    text: &'a str,
    hint: Option<&'a str>,
) -> Element<'a, Message> {
    let input = text_input("Nokta adı ya da Y,X", text)
        .on_input(move |t| event(Event::Known(field, t)))
        .padding([5, 8])
        .width(Fill)
        .font(kentos_ui::theme::typography::ui())
        .size(kentos_ui::theme::typography::body())
        .style(style::field::input);
    let pick = button(
        row![icon(Icon::Magnet).size(14.0), label::body("Çizimden")]
            .spacing(6)
            .align_y(Center),
    )
    .on_press(event(Event::Pick(field)))
    .padding([5, 10])
    .style(style::button::secondary);
    let resolved = match read::resolve_point(model, text) {
        Known::Empty => label::caption(hint.unwrap_or("Henüz verilmedi")),
        Known::Error(e) => label::caption(e).style(style::text::danger),
        Known::Point { p, .. } => label::caption(format.point(p)),
    };
    words::field(
        title,
        column![row![input, pick].spacing(8).align_y(Center), resolved].spacing(4),
        None,
    )
}

/// A text field with its label (the web's `textField`).
pub(crate) fn number_field<'a>(
    title: &'a str,
    value: &'a str,
    placeholder: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    words::field(
        title,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([5, 8])
            .width(Fill)
            .font(kentos_ui::theme::typography::ui())
            .size(kentos_ui::theme::typography::body())
            .style(style::field::input),
        None,
    )
}

/// The summary box: at most six problems, or what was computed.
pub(crate) fn summary<'a>(lines: Vec<(Line, String)>) -> Option<Element<'a, Message>> {
    (!lines.is_empty()).then(|| {
        words::summary(
            lines
                .into_iter()
                .map(|(kind, t)| words::text_line(kind, t))
                .collect(),
        )
    })
}

/// A results table: a header and rows of text, numbers right-aligned.
pub(crate) fn result_table<'a>(
    head: &[&'a str],
    rows: Vec<Vec<String>>,
    numeric: &[bool],
) -> Element<'a, Message> {
    let cell = |t: String, n: bool, strong: bool| -> Element<'a, Message> {
        let text = if n { label::mono(t) } else { label::body(t) };
        let text = if strong {
            text.font(kentos_ui::theme::typography::ui_strong())
        } else {
            text
        };
        let c = container(text).width(Fill).padding([4, 8]);
        if n {
            c.align_x(iced::Right).into()
        } else {
            c.into()
        }
    };
    let head_row = row(head
        .iter()
        .zip(numeric)
        .map(|(h, &n)| cell((*h).to_owned(), n, true)));
    let mut table = Column::new().push(
        container(head_row)
            .width(Fill)
            .style(style::container::header),
    );
    for r in rows {
        table = table.push(row(r
            .into_iter()
            .zip(numeric)
            .map(|(t, &n)| cell(t, n, false))));
    }
    container(table)
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// The layer select of the footer (the web's `layerChoice`): locked layers marked.
pub(crate) fn layer_select<'a>(
    model: &kentos_domain::Document,
    chosen: &str,
) -> Element<'a, Message> {
    let leaves = leaves(model);
    let selected = leaves.iter().position(|(id, _, _)| id == chosen);
    let choices: Vec<Choice> = leaves
        .iter()
        .map(|(_, name, locked)| {
            Choice::new(if *locked {
                format!("{name} (kilitli)")
            } else {
                name.clone()
            })
        })
        .collect();
    container(Select::new(choices, selected, |i| event(Event::Layer(i))).searchable(false))
        .width(Length::Fixed(200.0))
        .into()
}

/// The footer's buttons as the web lays them out.
pub(crate) fn footer_button<'a>(
    caption: &'a str,
    on: Option<Message>,
    primary: bool,
) -> Element<'a, Message> {
    if primary {
        words::primary(caption, on)
    } else {
        words::secondary(caption, on)
    }
}

#[cfg(test)]
mod tests {
    use kentos_contracts::Entity;

    use super::*;
    use crate::files_testing::{app_with_drawing, last_said};

    fn calc(app: &mut App, e: Event) {
        let _ = app.update(Message::Calc(e));
    }

    /// Önden kestirme from A (0, 0) and B (100, 0), 50 g at each: P lies
    /// right of A → B, at (50, −50). Worked out by hand.
    #[test]
    fn forward_intersection_adds_its_point_in_one_step() {
        let mut app = app_with_drawing();
        let _ = app.run("calc.forward");
        assert_eq!(app.dialog, Some(Asking::Calc));
        calc(&mut app, Event::Known(Field::A, "0,0".into()));
        calc(&mut app, Event::Known(Field::B, "100;0".into()));
        calc(&mut app, Event::Alpha("50".into()));
        calc(&mut app, Event::Beta("50,0".into()));
        let doc = app.document.as_ref().expect("open");
        let found = app
            .calc
            .intersection
            .compute(&doc.model)
            .result
            .expect("a point");
        assert!(
            (found.p.x - 50.0).abs() < 1e-9 && (found.p.y + 50.0).abs() < 1e-9,
            "{:?}",
            found.p
        );
        assert_eq!(found.name, "P");
        let count = doc.model.entities().count();
        calc(&mut app, Event::AddPoints);
        assert_eq!(app.dialog, None, "the window closes");
        let doc = app.document.as_mut().expect("open");
        assert_eq!(doc.model.entities().count(), count + 1);
        let Some(Entity::Point(p)) = app.selection.ids().first().and_then(|&s| doc.model.get(s))
        else {
            panic!("the new point, selected");
        };
        assert_eq!(p.base.label.as_deref(), Some("P"));
        assert_eq!(
            p.base.attrs.get("Tür").map(String::as_str),
            Some("Kestirme noktası")
        );
        assert_eq!(
            last_said(&app),
            "Önden kestirme: P noktası çizime eklendi (Ctrl+Z geri alır)."
        );
        let doc = app.document.as_mut().expect("open");
        assert_eq!(doc.model.undo().as_deref(), Some("Önden kestirme"));
    }

    #[test]
    fn missing_fields_are_said_in_the_webs_order() {
        let mut app = app_with_drawing();
        let _ = app.run("calc.resection");
        calc(&mut app, Event::Known(Field::B, "Q9".into()));
        calc(&mut app, Event::Alpha("x".into()));
        let doc = app.document.as_ref().expect("open");
        let computed = app.calc.intersection.compute(&doc.model);
        assert_eq!(
            computed.errors,
            [
                "A noktası verilmedi.",
                "B noktası: “Q9” adlı nokta çizimde yok. Adını denetleyin ya da Y,X yazın.",
                "C noktası verilmedi.",
                "α açısını yazın.",
                "β açısını yazın.",
            ]
        );
    }

    /// Çizimden: the window closes, the pick tool asks, a typed point comes
    /// back into the field and the window opens again as it was.
    #[test]
    fn a_point_shown_on_the_drawing_comes_back_to_its_field() {
        let mut app = app_with_drawing();
        let _ = app.run("calc.stakeout");
        calc(&mut app, Event::Known(Field::Back, "12,34".into()));
        calc(&mut app, Event::Pick(Field::Station));
        assert_eq!(app.dialog, None);
        assert_eq!(
            app.session.prompt().text(),
            "Aplikasyon: Durulan nokta (istasyon): haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]"
        );
        let _ = app.submit_line("486513.341,4420189.522");
        assert!(!app.session.is_running());
        assert_eq!(app.dialog, Some(Asking::Calc));
        // The demo drawing's point P1 lies exactly there: its name is taken.
        assert_eq!(app.calc.stakeout.station, "P1");
        assert_eq!(app.calc.stakeout.back, "12,34", "the rest as it was");
        // Esc: nothing picked, the window opens again.
        calc(&mut app, Event::Pick(Field::Back));
        app.cancel();
        assert_eq!(app.dialog, Some(Asking::Calc));
        assert_eq!(app.calc.stakeout.back, "12,34");
    }

    /// From (0, 0) oriented on (0, 100): the point (100, 0) is 100 g east,
    /// 100 m away, 100 g clockwise from the back point.
    #[test]
    fn stakeout_values_and_a_target_at_the_station() {
        let mut app = app_with_drawing();
        let _ = app.run("calc.stakeout");
        calc(&mut app, Event::Known(Field::Station, "0,0".into()));
        calc(&mut app, Event::Known(Field::Back, "0,100".into()));
        calc(&mut app, Event::Cell(0, "100,0".into()));
        let doc = app.document.as_ref().expect("open");
        let read = app.calc.stakeout.compute(&doc.model);
        let stakes = read.stakes.expect("values");
        assert!((stakes[0].bearing - 100.0).abs() < 1e-9);
        assert!((stakes[0].distance - 100.0).abs() < 1e-9);
        assert!(stakes[0].angle.is_some_and(|a| (a - 100.0).abs() < 1e-9));
        assert_eq!(read.names, ["100,0"]);
        calc(&mut app, Event::Cell(1, "0,0".into()));
        let doc = app.document.as_ref().expect("open");
        assert_eq!(
            app.calc.stakeout.compute(&doc.model).errors,
            ["2. satırdaki nokta durulan noktayla aynı yerde; semt tanımsız."]
        );
    }
}

/// The two windows filled in, for the owner. Not run by default:
/// `cargo test -p kentos-desktop calc::screens -- --ignored --nocapture`.
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["hesap-onden", "hesap-geriden", "hesap-aplikasyon"] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let calc = |app: &mut App, e: Event| {
                    let _ = app.update(Message::Calc(e));
                };
                match name {
                    "hesap-onden" => {
                        let _ = app.run("calc.forward");
                        calc(&mut app, Event::Known(Field::A, "P1".into()));
                        calc(
                            &mut app,
                            Event::Known(Field::B, "486535.757,4420188.723".into()),
                        );
                        calc(&mut app, Event::Alpha("62,5".into()));
                        calc(&mut app, Event::Beta("58.25".into()));
                        calc(&mut app, Event::Name("Y1".into()));
                    }
                    "hesap-geriden" => {
                        let _ = app.run("calc.resection");
                        calc(&mut app, Event::Known(Field::A, "P1".into()));
                        calc(
                            &mut app,
                            Event::Known(Field::B, "486535.757,4420188.723".into()),
                        );
                        calc(&mut app, Event::Alpha("42".into()));
                    }
                    _ => {
                        let _ = app.run("calc.stakeout");
                        calc(&mut app, Event::Known(Field::Station, "P1".into()));
                        calc(
                            &mut app,
                            Event::Known(Field::Back, "486535.757,4420188.723".into()),
                        );
                        calc(&mut app, Event::Cell(0, "486538.221,4420218.986".into()));
                        calc(&mut app, Event::Cell(1, "486514.344 4420220.532".into()));
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
