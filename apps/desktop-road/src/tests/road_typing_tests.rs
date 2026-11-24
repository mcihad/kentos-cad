//! Typing in Yol's Hakediş grid draws each key at once (the core's typing_tests.rs keeps the rest): it needs the module
//! registered (docs/adr/9199 §4.6).

use crate::road::RoadState;
use std::sync::{Arc, Mutex};
use iced::advanced::widget::operation::{Scrollable, TextInput};
use iced::advanced::widget::{Id, Operation};
use iced::{Event, Point, Rectangle, Size, Vector, keyboard};
use kentos_ui::snapshot::{Input, Snapshot};
use crate::app::{App, Message};
use crate::files_testing::offscreen;


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
        self.offset += std::mem::take(&mut self.pending);
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

fn update(app: &mut App, m: Message) {
    let _ = app.update(m);
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
                let pending = std::mem::take(&mut f.pending);
                f.offset += pending;
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

/// Poz metraj's grid: a new item's number, digit by digit.
#[test]
fn the_pay_grid_draws_each_key_at_once() {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.pay.query.json")).expect("cases");
    let snapshot_v2: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot_v2, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("payItems.editor"));
    let mut snapshot = offscreen(Size::new(1440.0, 900.0));
    snapshot.settle(&mut app, App::view, &mut update);
    let items = app.modules.road().pay_window.as_ref().map_or(0, |w| w.items.len());
    let add = *find(&mut snapshot, &app, "Poz ekle")
        .1
        .first()
        .expect("Poz ekle is shown");
    snapshot.input(&mut app, App::view, &mut update, Input::Click(add.center()));
    assert_eq!(
        app.modules.road().pay_window.as_ref().map_or(0, |w| w.items.len()),
        items + 1,
        "Poz ekle added a row"
    );
    // The items' grid: its cells (a row's height, not the form's taller fields) left of the rules' table; the new
    // item is its first column's last row. Its window grew and moved, so it is found again, not by where it was.
    let cells: Vec<Rectangle> = inputs(&mut snapshot, &app)
        .into_iter()
        .filter(|f| f.height < 20.0 && f.x < 1000.0)
        .collect();
    let first = cells.iter().map(|f| f.x).fold(f32::INFINITY, f32::min);
    let new = cells
        .into_iter()
        .filter(|f| (f.x - first).abs() < 0.5)
        .max_by(|a, b| a.y.total_cmp(&b.y))
        .expect("the new item's number");
    assert_eq!(
        keys(&mut app, &mut snapshot, new.center(), "15"),
        [true, true]
    );
    assert_eq!(
        app.modules.road()
            .pay_window
            .as_ref()
            .map(|w| w.items[items][0].clone()),
        Some(String::new()),
        "the cell keeps it until Enter"
    );
}
