//! The behaviour table's keyboard column measured on the drawn panel (docs/adr/8057 §71): each component type in a
//! panel KentOS draws, beside the components it names; Tab reaches it when its drawing holds a focusable widget, and
//! typing writes in it when that is a text input.

use std::sync::{Arc, Mutex};

use iced::Rectangle;
use iced::Size;
use iced::advanced::widget::Id;
use iced::advanced::widget::operation::{Focusable, Operation, TextInput};
use kentos_extensions::ui_panel;
use kentos_extensions::ui_panel::behaviour::{self, Keys, Prop};
use kentos_ui::snapshot::Snapshot;
use serde_json::{Value, json};

use super::super::panels::OpenPanel;
use crate::app::{App, Message};

const RECORDS: &str = include_str!("../../../../../../fixtures/kcad/v2/records.json");

/// The focusable widgets and text inputs an operation met, shown ones only (as Tab counts them).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Reach {
    focusables: usize,
    inputs: usize,
}

struct Survey(Arc<Mutex<Reach>>);

fn shown(bounds: Rectangle) -> bool {
    bounds.width > 0.5 && bounds.height > 0.5
}

impl Operation for Survey {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, _state: &mut dyn Focusable) {
        if shown(bounds)
            && let Ok(mut reach) = self.0.lock()
        {
            reach.focusables += 1;
        }
    }

    fn text_input(&mut self, _id: Option<&Id>, bounds: Rectangle, _state: &mut dyn TextInput) {
        if shown(bounds)
            && let Ok(mut reach) = self.0.lock()
        {
            reach.inputs += 1;
        }
    }
}

/// The building the measured components hold.
const BINA: &str = "0192f5a1-25a0-7000-8000-000000000301";

/// The records drawing with its owners' relation and an attachment on the building (as the relation and attachment
/// tests have them): every component draws its content, rows and buttons included.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let mut snapshot: Value = serde_json::from_str(RECORDS).expect("reads");
    snapshot["settings"]["relations"] = json!([{ "id": "malikler", "name": "Malikler", "parent": "bina",
        "child": "malik", "field": "Bina" }]);
    snapshot["entities"][0]["attachments"] =
        json!([{ "name": "Kat planı", "url": "https://harita.ornek.bel.tr/a-blok" }]);
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(snapshot).expect("a drawing");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// What the panel of `body` holds for Tab.
fn reach(app: &mut App, body: Vec<Value>) -> Reach {
    let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Deneme",
        "body": body });
    app.extensions.panels.clear();
    app.extensions.panels.push(OpenPanel::new(
        "tr.ornek.deneme",
        "deneme",
        ui_panel::read(&value).expect("a panel"),
    ));
    app.extensions.active_panel = 0;
    // Its pickers hold the building and its tables two rows: what a row's click or a record's buttons do is drawn.
    let p = &mut app.extensions.panels[0];
    for id in ["nesne", "x"] {
        p.picks.insert(id.into(), vec![BINA.into()]);
    }
    p.choices.insert("x".into(), BINA.into());
    let rows = vec![json!({ "a": "1", "uid": BINA }), json!({ "a": "2" })];
    for id in ["satirlar", "x"] {
        p.tables.insert(id.into(), (vec!["a".into()], rows.clone()));
    }
    let found = Arc::new(Mutex::new(Reach::default()));
    let mut snapshot = Snapshot::software(Size::new(420.0, 2400.0)).expect("a renderer");
    snapshot.operate(
        app.extension_panel_view(),
        Box::new(Survey(Arc::clone(&found))),
    );
    let reach = *found.lock().expect("counted");
    reach
}

/// The components the measured ones name: a layer picker, a feature picker and a table.
fn context() -> Vec<Value> {
    vec![
        json!({ "type": "layerPicker", "id": "katman", "label": "Katman" }),
        json!({ "type": "featurePicker", "id": "nesne", "label": "Nesne", "multiple": false }),
        json!({ "type": "table", "id": "satirlar", "label": "Satırlar" }),
    ]
}

/// Each type alone beside the context.
fn samples() -> Vec<(&'static str, Value)> {
    let choices = json!([{ "value": "a", "label": "A" }, { "value": "b", "label": "B" }]);
    vec![
        (
            "text",
            json!({ "type": "text", "id": "x", "text": "Yazı." }),
        ),
        (
            "section",
            json!({ "type": "section", "id": "x", "label": "Bölüm" }),
        ),
        ("divider", json!({ "type": "divider" })),
        (
            "textField",
            json!({ "type": "textField", "id": "x", "label": "Ad" }),
        ),
        (
            "numberField",
            json!({ "type": "numberField", "id": "x", "label": "Sayı" }),
        ),
        (
            "dateField",
            json!({ "type": "dateField", "id": "x", "label": "Tarih" }),
        ),
        (
            "timeField",
            json!({ "type": "timeField", "id": "x", "label": "Saat" }),
        ),
        (
            "dateTimeField",
            json!({ "type": "dateTimeField", "id": "x", "label": "An" }),
        ),
        (
            "select",
            json!({ "type": "select", "id": "x", "label": "Seçim", "options": choices }),
        ),
        (
            "multiSelect",
            json!({ "type": "multiSelect", "id": "x", "label": "Çoklu", "options": choices }),
        ),
        (
            "layerPicker",
            json!({ "type": "layerPicker", "id": "x", "label": "Katman 2" }),
        ),
        (
            "fieldPicker",
            json!({ "type": "fieldPicker", "id": "x", "label": "Alan", "layer": "katman" }),
        ),
        (
            "featurePicker",
            json!({ "type": "featurePicker", "id": "x", "label": "Nesne 2" }),
        ),
        (
            "filePicker",
            json!({ "type": "filePicker", "id": "x", "label": "Dosya" }),
        ),
        (
            "crsPicker",
            json!({ "type": "crsPicker", "id": "x", "label": "Sistem" }),
        ),
        (
            "pointPicker",
            json!({ "type": "pointPicker", "id": "x", "label": "Nokta" }),
        ),
        (
            "shapePicker",
            json!({ "type": "shapePicker", "id": "x", "label": "Alan", "shape": "polygon" }),
        ),
        (
            "relationPicker",
            json!({ "type": "relationPicker", "id": "x", "label": "Tesis", "relation": "malikler" }),
        ),
        (
            "checkbox",
            json!({ "type": "checkbox", "id": "x", "label": "Onay" }),
        ),
        (
            "button",
            json!({ "type": "button", "id": "x", "label": "Ara", "command": "ara" }),
        ),
        (
            "table",
            json!({ "type": "table", "id": "x", "label": "Tablo" }),
        ),
        (
            "tree",
            json!({ "type": "tree", "id": "x", "label": "Ağaç", "group": ["a"] }),
        ),
        (
            "card",
            json!({ "type": "card", "id": "x", "label": "Kart", "title": "a" }),
        ),
        (
            "stats",
            json!({ "type": "stats", "id": "x", "label": "Özet" }),
        ),
        (
            "chart",
            json!({ "type": "chart", "id": "x", "label": "Grafik", "of": "satirlar", "category": "a" }),
        ),
        (
            "image",
            json!({ "type": "image", "id": "x", "label": "Görsel", "height": 80 }),
        ),
        (
            "relationList",
            json!({ "type": "relationList", "id": "x", "label": "Kayıtlar", "relation": "malikler",
            "of": "nesne" }),
        ),
        (
            "attachments",
            json!({ "type": "attachments", "id": "x", "label": "Ekler", "of": "nesne" }),
        ),
    ]
}

/// The keyboard column is what KentOS draws: a component Tab reaches holds a focusable widget (a text input when
/// typing writes in it); one only the mouse uses, or only shown, holds none.
#[test]
fn the_keyboard_column_is_the_drawn_panels() {
    let mut app = app();
    let base = reach(&mut app, context());
    let kinds: Vec<&str> = samples().iter().map(|(k, _)| *k).collect();
    assert_eq!(kinds, ui_panel::component_types().collect::<Vec<_>>());
    for (kind, sample) in samples() {
        let mut body = context();
        body.push(sample);
        let with = reach(&mut app, body);
        let added = (with.focusables - base.focusables, with.inputs - base.inputs);
        let keys = behaviour::of(kind).expect("in the table").keys;
        let (reached, typed) = (added.0 > 0, added.1 > 0);
        let measured = match keys {
            Keys::Type => reached && typed,
            Keys::Choose | Keys::Toggle | Keys::Press | Keys::Rows => reached && !typed,
            Keys::Pointer | Keys::Shown => added == (0, 0),
        };
        assert!(
            measured,
            "{kind}: {keys:?}, focusables and inputs added {added:?}"
        );
    }
}

/// What starts the code is what the definition takes: a type whose table names `onChange` reads one, another refuses
/// it; the button runs its `command`.
#[test]
fn the_events_column_is_the_definitions() {
    for (kind, mut sample) in samples() {
        sample["onChange"] = json!("degisti");
        let body = [context(), vec![sample]].concat();
        let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Deneme",
            "body": body });
        let changes = ui_panel::read(&value)
            .and_then(|p| p.body.last().and_then(|c| c.on_change().map(str::to_owned)));
        let events = behaviour::of(kind).expect("in the table").events;
        assert_eq!(
            changes.is_some(),
            events.contains(&"onChange"),
            "{kind}: {events:?}"
        );
        assert_eq!(events.contains(&"command"), kind == "button", "{kind}");
    }
}

/// `ctx.panel.set` changes of a component what the table gives and refuses the rest, each property alone; `rows` and
/// `focus` likewise.
#[test]
fn the_session_takes_what_the_table_gives() {
    for (kind, sample) in samples() {
        let b = behaviour::of(kind).expect("in the table");
        let given = [
            (
                Prop::Value,
                b.example
                    .map_or(json!("x"), |e| serde_json::from_str(e).expect("json")),
            ),
            (Prop::Options, json!([{ "value": "c", "label": "C" }])),
            (Prop::Text, json!("Yeni.")),
            (Prop::Image, Value::Null),
            (Prop::Enabled, json!(false)),
        ];
        for (prop, value) in given {
            let mut p = panel(sample.clone());
            let mut params = json!({ "id": "x" });
            params[prop.name()] = value;
            let said = super::set(&mut p, &params, None);
            assert_eq!(
                said.is_ok(),
                b.takes(prop),
                "{kind} {}: {:?}",
                prop.name(),
                said.err().map(|e| e.message)
            );
        }
        let mut p = panel(sample.clone());
        let rows = super::rows(&mut p, &json!({ "id": "x", "rows": [{ "a": "1" }] }));
        assert_eq!(rows.is_ok(), b.rows, "{kind} rows");
    }
}

/// A panel of the context and `sample`.
fn panel(sample: Value) -> OpenPanel {
    let body = [context(), vec![sample]].concat();
    let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Deneme",
        "body": body });
    OpenPanel::new(
        "tr.ornek.deneme",
        "deneme",
        ui_panel::read(&value).expect("a panel"),
    )
}

/// The rows' cursor (docs/adr/8057 §71): the first ↓ takes the first row, the first ↑ the last, a step at a time to
/// the ends; the cursor's event shows its page; new rows from a command let it go.
#[test]
fn the_rows_cursor_steps_shows_its_page_and_goes_with_the_rows() {
    use super::super::panel_view::next_row;
    use super::super::panels::{PanelEvent, PanelOrigin};
    assert_eq!(next_row(None, 1, 5), Some(0));
    assert_eq!(next_row(None, -1, 5), Some(4));
    assert_eq!(next_row(Some(2), -1, 5), Some(1));
    assert_eq!(next_row(Some(4), 1, 5), None, "the last row stays");
    assert_eq!(next_row(None, 1, 0), None, "no rows, no cursor");
    let mut app = app();
    let body = vec![
        json!({ "type": "button", "id": "ara", "label": "Ara", "command": "ara", "table": "liste" }),
        json!({ "type": "table", "id": "liste" }),
    ];
    let value = json!({ "format": ui_panel::FORMAT, "schemaVersion": ui_panel::VERSION, "title": "Liste",
        "body": body });
    let mut p = OpenPanel::new(
        "tr.ornek.deneme",
        "liste",
        ui_panel::read(&value).expect("a panel"),
    );
    let rows: Vec<Value> = (0..450).map(|i| json!({ "no": i, "uid": BINA })).collect();
    p.tables.insert("liste".into(), (vec!["no".into()], rows));
    app.extensions.panels.clear();
    app.extensions.panels.push(p);
    app.extensions.active_panel = 0;
    let _ = app.ext_panel_event(PanelEvent::Cursor("liste".into(), 250, 1));
    let p = &app.extensions.panels[0];
    assert_eq!((p.cursor["liste"], p.pages["liste"]), (250, 1));
    let _ = app.extension_panel_view();
    app.extensions.panels[0].running = Some("ara".into());
    let origin = PanelOrigin {
        ext: "tr.ornek.deneme".into(),
        panel: "liste".into(),
        button: "ara".into(),
        table: Some("liste".into()),
    };
    app.ext_panel_answer(
        &origin,
        Ok(json!({ "rows": [{ "no": 1 }], "columns": ["no"] })),
    );
    assert!(
        !app.extensions.panels[0].cursor.contains_key("liste"),
        "new rows, no cursor"
    );
}
