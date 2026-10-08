//! Öznitelik tablosu's rules and the tab over `fixtures/interaction/v1/feature-table.kcad`
//! (docs/adr/0199 §4); the shared trace `feature-table.json` plays it through the shell.

use super::{Event, Show, next_sort};
use crate::app::{App, Message};
use crate::bottom::BottomTab;

fn app_with_table() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
        "../../../../fixtures/interaction/v1/feature-table.kcad"
    ))
    .expect("the drawing reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.features.reset(None);
    app
}

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Features(e));
}

fn rows(app: &App) -> Vec<String> {
    app.features_seen()
        .map(|(_, _, rows)| rows)
        .unwrap_or_default()
}

#[test]
fn a_header_sorts_ascending_then_descending_then_the_drawings_order() {
    let (mut sort, mut desc) = (None, false);
    (sort, desc) = next_sort(&sort, desc, "Parsel");
    assert_eq!((sort.as_deref(), desc), (Some("Parsel"), false));
    (sort, desc) = next_sort(&sort, desc, "Parsel");
    assert_eq!((sort.as_deref(), desc), (Some("Parsel"), true));
    (sort, desc) = next_sort(&sort, desc, "Parsel");
    assert_eq!((sort.as_deref(), desc), (None, false));
}

#[test]
fn the_tab_shows_the_active_layers_objects_by_its_fields() {
    let mut app = app_with_table();
    let _ = app.update(Message::Run("data.featureTable"));
    assert_eq!(app.bottom_tab, BottomTab::Table);
    let (count, columns, _) = app.features_seen().expect("the tab is open");
    assert_eq!(count, "5 / 5");
    assert_eq!(
        columns,
        [
            "Tür",
            "Ada *",
            "Parsel",
            "Tapu alanı",
            "Kullanım",
            "Ruhsat",
            "Tescil tarihi",
            "Not"
        ]
    );
    assert_eq!(
        rows(&app)[4],
        "Kapalı alan |  | 1 | 12.5 | Konut |  |  | köşe"
    );
    // Göster: the selected ones (none yet).
    ev(&mut app, Event::Show(Show::Selected));
    assert!(rows(&app).is_empty());
    ev(&mut app, Event::Show(Show::All));
    // A refused value keeps the cell open with what was typed.
    for m in app.features_edit(2, "Parsel", "x").expect("the cell") {
        let _ = app.update(m);
    }
    assert!(app.features.editing());
    ev(&mut app, Event::Cancel);
    for m in app.features_edit(2, "Parsel", "015").expect("the cell") {
        let _ = app.update(m);
    }
    assert!(!app.features.editing());
    assert!(rows(&app)[1].contains("| 15 |"), "{:?}", rows(&app));
}

/// Öznitelikler's fields beside the Tablo tab, dark and light, at 1440×900
/// and 1100×650 (`.run/shots/oznitelik-alanlari-*`): the parcel whose Parsel
/// is “7a” selected, Genel and Geometri folded, its fields by their kinds
/// (a value list and yes or no as lists, a date and numbers in boxes, the
/// refused value in the warning colour):
///
/// ```text
/// cargo test -p kentos-desktop features::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let mut app = app_with_table();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::Run("data.featureTable"));
            app.selection.set([kentos_domain::Slot(4)]);
            app.props_closed.insert("general");
            app.props_closed.insert("geometry");
            let _ = app.update(Message::WindowResized(Size::new(width, height)));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("oznitelik-alanlari-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
