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

fn write(app: &mut App, e: super::write::WriteEvent) {
    ev(app, Event::Write(e));
}

#[test]
fn alan_hesapla_previews_then_writes_in_one_step_and_a_paste_reports_what_does_not_fit() {
    // docs/adr/0507: the window over cad.fields.write; plan first, the write one undo step.
    use super::write::{Scope, WriteEvent as W};
    let mut app = app_with_table();
    let _ = app.update(Message::Run("data.featureTable"));
    write(&mut app, W::Open);
    assert_eq!(app.dialog, Some(crate::app::Dialog::FieldsWrite));
    write(&mut app, W::Field("Alan".into()));
    write(&mut app, W::Expression("Alan * 2".into()));
    write(&mut app, W::Scope(Scope::All));
    write(&mut app, W::Preview);
    let plan = app
        .features
        .write
        .as_ref()
        .and_then(|w| w.plan.clone())
        .expect("planned")
        .expect("a plan");
    // “12,5” is no number to the language: its result is empty and the old value stays.
    assert_eq!((plan.changed, plan.kept), (4, 1), "{plan:?}");
    write(&mut app, W::Run(kentos_contracts::WritePolicy::All));
    assert!(app.features.write.is_none());
    assert!(rows(&app)[0].contains("| 801 |"), "{:?}", rows(&app));
    let _ = app.update(Message::Run("edit.undo"));
    assert!(rows(&app)[0].contains("| 400.50 |"), "{:?}", rows(&app));
    // A paste: a header row names the fields; the second row's Parsel is refused.
    app.selection.set([kentos_domain::Slot(1)]);
    write(
        &mut app,
        W::Pasted(Some("Parsel\tKullanım\n8\tTicaret\nx\tKonut\n".into())),
    );
    let plan = app
        .features
        .write
        .as_ref()
        .and_then(|w| w.plan.clone())
        .expect("planned")
        .expect("a plan");
    assert_eq!((plan.changed, plan.invalid), (1, 1), "{plan:?}");
    write(&mut app, W::Run(kentos_contracts::WritePolicy::Valid));
    assert!(rows(&app)[0].contains("| 8 |"), "{:?}", rows(&app));
    assert!(rows(&app)[1].contains("| 12 |"), "{:?}", rows(&app));
}

/// Alan hesapla's preview and a paste's problems, dark and light (`.run/shots/alan-hesapla-*`):
///
/// ```text
/// cargo test -p kentos-desktop features::tests::write_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn write_screens() {
    use super::write::{Scope, WriteEvent as W};
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for mode in ["dark", "light"] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in ["hesap", "yapistir"] {
                let mut app = app_with_table();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::Run("data.featureTable"));
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                if scene == "hesap" {
                    write(&mut app, W::Open);
                    write(&mut app, W::Field("Alan".into()));
                    write(&mut app, W::Expression("Alan / 3".into()));
                    write(&mut app, W::Scope(Scope::All));
                } else {
                    app.selection.set([kentos_domain::Slot(1)]);
                    write(
                        &mut app,
                        W::Pasted(Some("Parsel\tKullanım\n8\tTicaret\nx\tKonut\n".into())),
                    );
                }
                write(&mut app, W::Preview);
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("alan-hesapla-{mode}-{scene}-{width}x{height}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
