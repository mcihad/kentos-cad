//! İfade oluşturucu on the desktop over İfadeyle seç: it opens on the
//! field's text with the input's fields and objects; typing a name opens
//! the list, whose entry goes in with its parentheses; the tree inserts on
//! a double press, a function around the selection; the preview steps
//! through the objects and follows one picked on the drawing; Vazgeç leaves
//! the field, Tamam writes the text back. The drawing is the processing
//! cases' parcels (fixtures/processing/v1/parcels.kcad).

use iced::widget::text_editor::{Action, Edit, Motion};
use kentos_domain::Slot;
use serde_json::Value;

use super::{Event, Preview};
use crate::app::{App, Dialog, Message};
use crate::document::Document;

const PARCELS: &str = include_str!("../../../../fixtures/processing/v1/parcels.kcad");

/// The app with the parcels open.
fn app_with_parcels() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(PARCELS).expect("the parcels read");
    let doc = Document::new(snapshot, None).expect("the parcels open");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Builder(e));
}

/// İfadeyle seç's window and its condition's builder.
fn opened() -> App {
    let mut app = app_with_parcels();
    let _ = app.update(Message::Run("processing.run.selection.byExpression"));
    assert_eq!(app.dialog, Some(Dialog::Processing));
    event(&mut app, Event::OpenProcessing("condition".into()));
    assert!(app.builder.is_some(), "the builder opens");
    app
}

fn typed(app: &mut App, text: &str) {
    for c in text.chars() {
        event(app, Event::Edit(Action::Edit(Edit::Insert(c))));
    }
}

/// Everything in the editor replaced by `text`.
fn written(app: &mut App, text: &str) {
    event(app, Event::Edit(Action::SelectAll));
    event(app, Event::Edit(Action::Edit(Edit::Backspace)));
    typed(app, text);
}

fn source(app: &App) -> String {
    app.builder
        .as_ref()
        .map(|b| b.source.clone())
        .unwrap_or_default()
}

fn condition(app: &App) -> String {
    app.processing
        .dialog
        .as_ref()
        .and_then(|w| w.values.get("condition"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn it_opens_on_the_fields_text_fields_and_objects() {
    let app = opened();
    let b = app.builder.as_ref().expect("open");
    assert_eq!(b.source, condition(&app));
    assert_eq!(
        b.cursor,
        b.source.encode_utf16().count(),
        "the cursor at the end"
    );
    let names: Vec<&str> = b.schema.fields.iter().map(|f| f.name.as_str()).collect();
    for name in ["Ada", "Parsel", "Nitelik"] {
        assert!(names.contains(&name), "{names:?}");
    }
    // Tümü: every object the tool takes, in the drawing's order.
    assert!(b.objects.len() >= 5, "{}", b.objects.len());
    assert!(matches!(b.preview, Preview::Value(_)), "{:?}", b.preview);
}

#[test]
fn a_typed_name_opens_the_list_and_its_entry_goes_in_with_parentheses() {
    let mut app = opened();
    written(&mut app, "yuv");
    let b = app.builder.as_ref().expect("open");
    let list = b.completion.as_ref().expect("the list opens");
    assert_eq!(list.items[0].label, "yuvarla");
    event(&mut app, Event::AcceptActive);
    let b = app.builder.as_ref().expect("open");
    assert_eq!(b.source, "yuvarla()");
    assert_eq!(b.cursor, 8, "between the parentheses");
    assert!(b.completion.is_none());
    let s = b.signature.as_ref().expect("the call's signature");
    assert_eq!((s.name, s.active), ("yuvarla", Some(0)));
    // Ctrl+Space where nothing is being written lists everything.
    event(&mut app, Event::Complete);
    let b = app.builder.as_ref().expect("open");
    assert!(b.completion.as_ref().is_some_and(|c| c.items.len() > 40));
    event(&mut app, Event::CloseList);
    assert!(app.builder.as_ref().is_some_and(|b| b.completion.is_none()));
}

#[test]
fn the_list_moves_with_the_keys_and_its_help_follows() {
    let mut app = opened();
    written(&mut app, "$m");
    let first = app
        .builder
        .as_ref()
        .and_then(|b| b.help.as_ref())
        .map(|h| h.key.clone());
    event(&mut app, Event::Step(1));
    let b = app.builder.as_ref().expect("open");
    assert_eq!(b.active, 1);
    let second = b.help.as_ref().map(|h| h.key.clone());
    assert_ne!(first, second, "the help shows the highlighted entry");
    event(&mut app, Event::Step(-1));
    assert_eq!(app.builder.as_ref().map(|b| b.active), Some(0));
}

#[test]
fn the_tree_inserts_on_a_double_press_a_function_around_the_selection() {
    let mut app = opened();
    written(&mut app, "$alan");
    event(&mut app, Event::Edit(Action::SelectAll));
    event(&mut app, Event::Row("func:mutlak".into()));
    assert_eq!(source(&app), "$alan", "one press shows the help");
    assert_eq!(
        app.builder
            .as_ref()
            .and_then(|b| b.help.as_ref())
            .map(|h| h.title.clone()),
        Some("mutlak".into())
    );
    event(&mut app, Event::Row("func:mutlak".into()));
    assert_eq!(source(&app), "mutlak($alan)");
    // An operator keeps single spaces; a field goes in as the language writes it.
    event(&mut app, Event::Edit(Action::Move(Motion::DocumentEnd)));
    event(
        &mut app,
        Event::Operator(">", " > ", kentos_expression::editor::Kind::Operator),
    );
    assert_eq!(source(&app), "mutlak($alan) > ");
}

#[test]
fn the_keys_choose_in_the_tree_and_enter_puts_the_entry_in() {
    let mut app = opened();
    written(&mut app, "");
    // Alanlar ve değerler is open: ↓ goes through its fields, Enter puts the chosen one in.
    event(&mut app, Event::TreeStep(1));
    event(&mut app, Event::TreeStep(1));
    let chosen = app.builder.as_ref().and_then(|b| b.chosen.clone());
    assert_eq!(
        chosen.as_deref(),
        Some("field:Nitelik"),
        "Ada, then Nitelik"
    );
    event(&mut app, Event::QuerySubmit);
    assert_eq!(source(&app), "Nitelik");
    // A search: Enter puts the first found in when nothing found is chosen.
    written(&mut app, "");
    event(&mut app, Event::Query("round".into()));
    event(&mut app, Event::QuerySubmit);
    assert_eq!(source(&app), "yuvarla()");
}

#[test]
fn a_fields_values_go_in_quoted() {
    let mut app = opened();
    written(&mut app, "Nitelik = ");
    event(&mut app, Event::Row("field:Nitelik".into()));
    event(&mut app, Event::Values(true));
    let listed = app
        .builder
        .as_ref()
        .and_then(|b| b.listed.clone())
        .expect("the values are listed");
    let texts: Vec<&str> = listed.items.iter().map(|v| v.text.as_str()).collect();
    assert_eq!(texts, ["Arsa", "Tarla"]);
    event(&mut app, Event::Value(1));
    event(&mut app, Event::Value(1));
    assert_eq!(source(&app), "Nitelik = 'Tarla'");
}

#[test]
fn the_preview_steps_through_the_objects_and_follows_a_picked_one() {
    let mut app = opened();
    written(&mut app, "$sıra");
    let shown = |app: &App| app.builder.as_ref().map(|b| b.preview.clone());
    assert_eq!(shown(&app), Some(Preview::Value("1".into())));
    event(&mut app, Event::Object(1));
    assert_eq!(shown(&app), Some(Preview::Value("2".into())));
    event(&mut app, Event::Object(-2));
    let n = app.builder.as_ref().map_or(0, |b| b.objects.len());
    assert_eq!(
        shown(&app),
        Some(Preview::Value(n.to_string())),
        "round to the last"
    );
    // Sahneden seç: the windows step aside and the pick tool runs; the picked object is previewed.
    event(&mut app, Event::Pick);
    assert_eq!(app.dialog, None, "the windows step aside");
    assert!(app.builder.as_ref().is_some_and(super::Builder::is_picking));
    let third = app
        .builder
        .as_ref()
        .map(|b| b.objects.slots[2])
        .expect("objects");
    app.selection.set([third]);
    assert!(app.builder_picked(true));
    assert_eq!(app.dialog, Some(Dialog::Processing), "they come back");
    assert_eq!(shown(&app), Some(Preview::Value("3".into())));
    assert!(app.selection.ids().is_empty(), "the selection as it was");
    // An object the expression does not run on leaves the preview where it was.
    event(&mut app, Event::Pick);
    app.selection.set([Slot(999)]);
    assert!(app.builder_picked(true));
    assert_eq!(shown(&app), Some(Preview::Value("3".into())));
}

#[test]
fn the_view_follows_the_cursor_past_the_box() {
    let mut app = opened();
    event(&mut app, Event::Room(300.0, 120.0));
    written(
        &mut app,
        "eğer(Nitelik = 'Arsa' ve $alan > 500, 'büyük arsa', 'küçük')",
    );
    let b = app.builder.as_ref().expect("open");
    let follow = b.follow().is_some();
    assert!(follow, "the end of a long line is out of a 300 px box");
    // Scrolled there, the cursor is in view: nothing more to do.
    let m = super::editor::Metrics::now();
    let (line, column) = super::editor::line_column(&b.source, b.cursor);
    let x = m.cell(line, column).x + 2.0 * m.advance - 300.0;
    event(&mut app, Event::Scrolled(x, 0.0));
    assert!(app.builder.as_ref().is_some_and(|b| b.follow().is_none()));
    // Back at the start, the view goes back too.
    event(&mut app, Event::Edit(Action::Move(Motion::DocumentStart)));
    assert!(app.builder.as_ref().is_some_and(|b| b.follow().is_some()));
}

#[test]
fn vazgec_leaves_the_field_and_tamam_writes_it_back() {
    let mut app = opened();
    let before = condition(&app);
    written(&mut app, "Nitelik = 'Arsa'");
    event(&mut app, Event::Cancel);
    assert!(app.builder.is_none());
    assert_eq!(condition(&app), before);
    event(&mut app, Event::OpenProcessing("condition".into()));
    written(&mut app, "Nitelik = 'Arsa' ve $alan >");
    event(&mut app, Event::Ok);
    assert!(app.builder.is_some(), "an error keeps Tamam off");
    typed(&mut app, " 100");
    event(&mut app, Event::Ok);
    assert!(app.builder.is_none());
    assert_eq!(condition(&app), "Nitelik = 'Arsa' ve $alan > 100");
    assert_eq!(
        app.dialog,
        Some(Dialog::Processing),
        "the window it came from stays"
    );
}

/// Pictures for the owner: the builder over İfadeyle seç with a call being
/// written, with the list open, and with an error and a field's values; both
/// themes at 1440×900 and 1100×650 (`.run/shots/ifade-olusturucu-*`).
/// `cargo test -p kentos-desktop expression::tests::screens -- --ignored --nocapture`
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["cagri", "oneri", "hata", "uzun"] {
                let mut app = opened();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "cagri" => written(&mut app, "Nitelik = 'Arsa' ve yuvarla($alan, "),
                    "oneri" => written(&mut app, "Nitelik = 'Arsa' ve Pa"),
                    // A line longer than the box: the view follows the cursor to its end.
                    "uzun" => {
                        written(
                            &mut app,
                            "eğer(Nitelik = 'Arsa' ve $alan > 500, 'büyük arsa', eğer(Nitelik = 'Tarla', 'tarla', 'başka'))",
                        );
                        snapshot.settle(&mut app, App::view, &mut update);
                        event(&mut app, Event::Edit(Action::Move(Motion::DocumentEnd)));
                    }
                    _ => {
                        written(&mut app, "eğer(Nitelik = , 1, 2)");
                        event(&mut app, Event::Row("field:Nitelik".into()));
                        event(&mut app, Event::Values(false));
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!(
                    "ifade-olusturucu-{name}-{width}x{height}{suffix}.png"
                ));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// Akış (docs/adr/0101): the same text as nodes; a value written in the
/// inspector, a connection taken off and put back, a node removed and
/// brought back, a palette entry carried onto an empty input; each change
/// is the text's at once, and Tamam writes it back.
#[test]
fn akis_changes_the_text_and_tamam_writes_it_back() {
    use super::ViewMode;
    use super::flow::FlowEvent as F;
    let mut app = opened();
    written(&mut app, "yuvarla($alan, 2)");
    event(&mut app, Event::Mode(ViewMode::Flow));
    let ids = |app: &App| -> Vec<String> {
        app.builder
            .as_ref()
            .and_then(|b| b.flow.flow.as_ref())
            .map(|f| f.nodes.iter().map(|n| n.id.clone()).collect())
            .unwrap_or_default()
    };
    assert_eq!(ids(&app), ["r", "0", "0.0", "0.1"]);
    let flow = |app: &mut App, e: F| event(app, Event::Flow(e));

    // A value written in the inspector.
    flow(&mut app, F::Select(Some("0.1".into())));
    assert_eq!(
        app.builder.as_ref().map(|b| b.flow.draft.clone()),
        Some("2".into())
    );
    flow(&mut app, F::Draft("3".into()));
    flow(&mut app, F::Commit);
    assert_eq!(source(&app), "yuvarla($alan, 3)");
    flow(&mut app, F::Draft("x".into()));
    flow(&mut app, F::Commit);
    assert!(
        app.builder
            .as_ref()
            .is_some_and(|b| b.flow.draft_error.is_some()),
        "not a number"
    );

    // A connection taken off stands apart; put back, the text is as it was.
    flow(
        &mut app,
        F::Disconnect {
            to: "0".into(),
            port: 1,
        },
    );
    assert_eq!(source(&app), "yuvarla($alan)");
    assert!(ids(&app).contains(&"1".to_owned()));
    flow(
        &mut app,
        F::Connect {
            from: "1".into(),
            to: "0".into(),
            port: 1,
        },
    );
    assert_eq!(source(&app), "yuvarla($alan, 3)");

    // Removed, the node's input is empty (the text's error); Ctrl+Z brings it back.
    flow(&mut app, F::Remove("0.0".into()));
    assert_eq!(source(&app), "yuvarla(?, 3)");
    assert!(
        app.builder
            .as_ref()
            .is_some_and(|b| b.check.error.is_some())
    );
    flow(&mut app, F::Undo);
    assert_eq!(source(&app), "yuvarla($alan, 3)");
    flow(&mut app, F::Redo);
    flow(&mut app, F::Undo);

    // A palette entry carried out of the tree onto the empty input.
    flow(
        &mut app,
        F::Disconnect {
            to: "0".into(),
            port: 0,
        },
    );
    let place = app
        .builder
        .as_ref()
        .and_then(|b| b.palette().iter().position(|k| k == "field:Nitelik"))
        .expect("Nitelik is in the palette");
    event(&mut app, Event::Carry(place));
    assert_eq!(
        app.builder.as_ref().and_then(|b| b.flow.carrying.clone()),
        Some("field:Nitelik".into())
    );
    flow(
        &mut app,
        F::Drop {
            at: (-400.0, 0.0),
            to: Some(("0".into(), 0)),
        },
    );
    assert_eq!(source(&app), "yuvarla(Nitelik, 3)");

    // Metin shows the same text; Tamam writes it back.
    event(&mut app, Event::Mode(ViewMode::Text));
    event(&mut app, Event::Ok);
    assert_eq!(condition(&app), "yuvarla(Nitelik, 3)");
}

/// Pictures for the owner: Akış over İfadeyle seç, a whole expression and a
/// node selected with its inspector; both themes at 1440×900 and 1100×650
/// (`.run/shots/ifade-akisi-*`).
/// `cargo test -p kentos-desktop expression::tests::flow_screens -- --ignored --nocapture`
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn flow_screens() {
    use super::ViewMode;
    use super::flow::FlowEvent as F;
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["akis", "dugum"] {
                let mut app = opened();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "akis" => written(
                        &mut app,
                        "durum eğer Nitelik = 'Arsa' ve $alan > 500 ise yuvarla($alan / 1000, 2) yoksa 0 son",
                    ),
                    _ => written(&mut app, "yuvarla(Nitelik || ' ' || ?, 2)"),
                }
                event(&mut app, Event::Mode(ViewMode::Flow));
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "dugum" {
                    event(&mut app, Event::Flow(F::Select(Some("0".into()))));
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("ifade-akisi-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
