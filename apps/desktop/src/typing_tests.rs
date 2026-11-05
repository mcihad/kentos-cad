//! A typed key shows at once (docs/adr/0976; the owner's “bir saniye bir buçuk saniye sonra yazılıyor”): in Proje
//! ayarları › Birim ve ölçek's number fields, in Nesne denetçisi's text cells and in Poz metraj's grid. Iced 0.14 draws
//! the window only for a message or a frame a widget asks for; the fields that keep what is typed sent neither, and
//! the letter waited for the caret's next blink or a timer. Each key here must be drawn at its own frame.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use iced::advanced::widget::operation::{Scrollable, TextInput};
use iced::advanced::widget::{Id, Operation};
use iced::{Event, Point, Rectangle, Size, Vector, keyboard};
use kentos_ui::snapshot::{Input, Snapshot};

use crate::app::{App, Message};
use crate::files_testing::offscreen;
use crate::project::{Event as ProjectEvent, SettingsEvent, settings::Section};
use crate::tools_scenes::{Objects, open};

fn update(app: &mut App, m: Message) {
    let _ = app.update(m);
}

/// Where the window shows each text field and each text (`caption`'s), on the screen: a scrolled body's offset taken
/// off what it holds (an operation reports where the layout puts a widget, before its scrollable moves it).
struct Find {
    caption: String,
    inputs: Vec<Rectangle>,
    texts: Vec<Rectangle>,
    offset: Vector,
    pending: Vector,
}

impl Find {
    fn shown(&self, bounds: Rectangle) -> Rectangle {
        Rectangle { x: bounds.x - self.offset.x, y: bounds.y - self.offset.y, ..bounds }
    }
}

impl Operation for Find {
    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        _content: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.pending = translation;
    }

    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        let saved = self.offset;
        self.offset = self.offset + std::mem::take(&mut self.pending);
        operate(self);
        self.offset = saved;
    }

    fn text_input(&mut self, _id: Option<&Id>, bounds: Rectangle, _state: &mut dyn TextInput) {
        let shown = self.shown(bounds);
        self.inputs.push(shown);
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, text: &str) {
        if text == self.caption {
            let shown = self.shown(bounds);
            self.texts.push(shown);
        }
    }
}

/// The window's text fields and `caption`'s texts, where they show.
fn find(snapshot: &mut Snapshot, app: &App, caption: &str) -> (Vec<Rectangle>, Vec<Rectangle>) {
    struct Shared(Arc<Mutex<Find>>);
    impl Operation for Shared {
        fn scrollable(
            &mut self,
            id: Option<&Id>,
            bounds: Rectangle,
            content: Rectangle,
            translation: Vector,
            state: &mut dyn Scrollable,
        ) {
            if let Ok(mut f) = self.0.lock() {
                f.scrollable(id, bounds, content, translation, state);
            }
        }
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            let saved = {
                let mut f = self.0.lock().expect("the finder");
                let saved = f.offset;
                f.offset = f.offset + std::mem::take(&mut f.pending);
                saved
            };
            operate(self);
            self.0.lock().expect("the finder").offset = saved;
        }
        fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
            if let Ok(mut f) = self.0.lock() {
                f.text_input(id, bounds, state);
            }
        }
        fn text(&mut self, id: Option<&Id>, bounds: Rectangle, text: &str) {
            if let Ok(mut f) = self.0.lock() {
                f.text(id, bounds, text);
            }
        }
    }
    let found = Arc::new(Mutex::new(Find {
        caption: caption.to_owned(),
        inputs: Vec::new(),
        texts: Vec::new(),
        offset: Vector::ZERO,
        pending: Vector::ZERO,
    }));
    snapshot.operate(app.view(), Box::new(Shared(found.clone())));
    let f = found.lock().expect("the operation's answer");
    (f.inputs.clone(), f.texts.clone())
}

/// Every text field the window shows: where it is.
fn inputs(snapshot: &mut Snapshot, app: &App) -> Vec<Rectangle> {
    find(snapshot, app, "").0
}

/// The text field on `caption`'s row, right of it.
fn field_beside(snapshot: &mut Snapshot, app: &App, caption: &str) -> Point {
    let (inputs, texts) = find(snapshot, app, caption);
    let label = *texts.first().unwrap_or_else(|| panic!("“{caption}” is shown"));
    let middle = label.center_y();
    inputs
        .into_iter()
        .filter(|f| f.x > label.x + label.width && (f.center_y() - middle).abs() < 40.0)
        .min_by(|a, b| (a.center_y() - middle).abs().total_cmp(&(b.center_y() - middle).abs()))
        .map(|f| f.center())
        .unwrap_or_else(|| panic!("a field beside “{caption}”"))
}

/// Pressed at `at`, then `typed` key by key: whether each key is drawn at its own frame.
fn keys(app: &mut App, snapshot: &mut Snapshot, at: Point, typed: &str) -> Vec<bool> {
    snapshot.input(app, App::view, &mut update, Input::Click(at));
    snapshot
        .drawn(app, App::view, &mut update, Input::Type(typed.into()))
        .into_iter()
        .filter(|(e, _)| matches!(e, Event::Keyboard(keyboard::Event::KeyPressed { .. })))
        .map(|(_, drawn)| drawn)
        .collect()
}

fn settings_units() -> (App, Snapshot) {
    let mut app = crate::files_testing::app_with_drawing();
    let _ = app.update(Message::Run("file.settings"));
    let _ = app.update(Message::Project(Box::new(ProjectEvent::Settings(SettingsEvent::Section(Section::Units)))));
    let mut snapshot = offscreen(Size::new(1440.0, 900.0));
    snapshot.settle(&mut app, App::view, &mut update);
    (app, snapshot)
}

/// Ondalık basamak (a number field): each digit at once.
#[test]
fn project_settings_decimals_draw_each_key_at_once() {
    let (mut app, mut snapshot) = settings_units();
    let at = field_beside(&mut snapshot, &app, "Ondalık basamak");
    assert_eq!(keys(&mut app, &mut snapshot, at, "2"), [true]);
}

/// Ölçek's denominator (a text field sending each change) and a kind's height on paper (a number field).
#[test]
fn project_settings_scale_and_heights_draw_each_key_at_once() {
    let (mut app, mut snapshot) = settings_units();
    let at = field_beside(&mut snapshot, &app, "Ölçek");
    assert_eq!(keys(&mut app, &mut snapshot, at, "50"), [true, true]);
    let (mut app, mut snapshot) = settings_units();
    // Yazı yükseklikleri at the page's foot: the page scrolled there first.
    snapshot.operate(app.view(), Box::new(crate::files_testing::SnapAll));
    snapshot.settle(&mut app, App::view, &mut update);
    let label = kentos_commands::AnnotationKind::Measure.label();
    let at = field_beside(&mut snapshot, &app, label);
    assert_eq!(keys(&mut app, &mut snapshot, at, "35"), [true, true]);
}

/// Nesne denetçisi's Metin (a text cell): each letter at once.
#[test]
fn the_inspector_s_text_cell_draws_each_key_at_once() {
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = Objects::new();
    let id = o.text("cizim", [2.0, 2.0], "Yol", 1.0, 0.0);
    open(&mut app, o);
    app.selection.set([kentos_domain::Slot(id)]);
    let mut snapshot = offscreen(Size::new(1440.0, 900.0));
    snapshot.settle(&mut app, App::view, &mut update);
    let at = field_beside(&mut snapshot, &app, "Metin");
    assert_eq!(keys(&mut app, &mut snapshot, at, "ab"), [true, true]);
}

/// Poz metraj's grid: a new item's number, digit by digit.
#[test]
fn the_pay_grid_draws_each_key_at_once() {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.pay.query.json")).expect("cases");
    let snapshot_v2: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot_v2, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("payItems.editor"));
    let mut snapshot = offscreen(Size::new(1440.0, 900.0));
    snapshot.settle(&mut app, App::view, &mut update);
    let items = app.pay_window.as_ref().map_or(0, |w| w.items.len());
    let add = *find(&mut snapshot, &app, "Poz ekle").1.first().expect("Poz ekle is shown");
    snapshot.input(&mut app, App::view, &mut update, Input::Click(add.center()));
    assert_eq!(app.pay_window.as_ref().map_or(0, |w| w.items.len()), items + 1, "Poz ekle added a row");
    // The items' grid: its cells (a row's height, not the form's taller fields) left of the rules' table; the new
    // item is its first column's last row. Its window grew and moved, so it is found again, not by where it was.
    let cells: Vec<Rectangle> = inputs(&mut snapshot, &app).into_iter().filter(|f| f.height < 20.0 && f.x < 1000.0).collect();
    let first = cells.iter().map(|f| f.x).fold(f32::INFINITY, f32::min);
    let new = cells
        .into_iter()
        .filter(|f| (f.x - first).abs() < 0.5)
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .expect("the new item's number");
    assert_eq!(keys(&mut app, &mut snapshot, new.center(), "15"), [true, true]);
    assert_eq!(app.pay_window.as_ref().map(|w| w.items[items][0].clone()), Some(String::new()), "the cell keeps it until Enter");
}

fn crowded() -> App {
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = Objects::new();
    for i in 0..20_000 {
        let (x, y) = (f64::from(i % 141), f64::from(i / 141));
        o.line("cizim", [x, y], [x + 0.5, y]);
    }
    for i in 0..2_000 {
        let (x, y) = (f64::from(i % 50) * 3.0, f64::from(i / 50) * 3.0);
        o.text("cizim", [x, y], "Yazı 12,5", 0.5, 0.0);
    }
    open(&mut app, o);
    app
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// How long a key's work takes on a crowded drawing (20 000 lines, 2 000 texts): the update, the view, the drawing.
/// The delay was not this work (it is milliseconds) but the frame not asked for.
#[test]
#[ignore = "a measure, run by hand with --nocapture"]
fn typing_in_project_settings_on_a_crowded_drawing() {
    let mut app = crowded();
    let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
    let _ = app.update(Message::Run("file.settings"));
    let _ = app.update(Message::Project(Box::new(ProjectEvent::Settings(SettingsEvent::Section(Section::Units)))));
    snapshot.settle(&mut app, App::view, &mut update);
    let theme = app.theme();
    let _ = snapshot.render(app.view(), &theme);
    for typed in ["1", "10", "100", "1000", "10000"] {
        let t = Instant::now();
        let _ = app.update(Message::Project(Box::new(ProjectEvent::Settings(SettingsEvent::ScaleTyped(typed.into())))));
        let u = ms(t);
        let t = Instant::now();
        let view = app.view();
        let v = ms(t);
        let t = Instant::now();
        let _ = snapshot.render(view, &theme);
        let r = ms(t);
        println!("ölçek {typed:>6}: update {u:7.1} ms · view {v:7.1} ms · çizim {r:7.1} ms");
    }
}
