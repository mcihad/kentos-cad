//! Öznitelik tablosu's rules and the tab over `fixtures/interaction/v1/feature-table.kcad`
//! (docs/adr/0199 §4); the shared trace `feature-table.json` plays it through the shell.

use super::{Event, Show, next_sort};
use crate::app::{App, Message};
use crate::bottom::BottomTab;

fn app_with_table() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(include_str!(
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

/// Sütunlar ▾ with a column hidden and the rows' copy and file, and İstatistik and Grafik over the table's rows,
/// dark and light (`.run/shots/oznitelik-sutun-*`, `oznitelik-istatistik-*`, `oznitelik-grafik-*`):
///
/// ```text
/// cargo test -p kentos-desktop features::tests::outgoing_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn outgoing_screens() {
    use super::outgoing::{Analyse, ColumnsEvent};
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for scene in ["sutun", "istatistik", "grafik"] {
                let mut app = app_with_secret();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::WindowResized(Size::new(width, height)));
                ev(&mut app, Event::Columns(ColumnsEvent::Toggle("Ruhsat".into())));
                ev(&mut app, Event::Search("102".into()));
                match scene {
                    "istatistik" => {
                        ev(&mut app, Event::Analyse(Analyse::Statistics));
                        // The summaries are counted as frames come: count them all for the picture.
                        let _ = app.statistics_seen();
                    }
                    "grafik" => ev(&mut app, Event::Analyse(Analyse::Chart)),
                    _ => {}
                }
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("oznitelik-{scene}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
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
    write(&mut app, W::Run(kentos_commands::WritePolicy::All));
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
    write(&mut app, W::Run(kentos_commands::WritePolicy::Valid));
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

/// The table's drawing with a form that marks the Tapu alanı field (`Alan`) `redact` (docs/adr/0443 §2).
fn app_with_secret() -> App {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../fixtures/interaction/v1/feature-table.kcad"
    ))
    .expect("the drawing is JSON");
    value["settings"]["forms"] = serde_json::json!([{
        "id": "parsel-formu", "name": "Parsel formu", "layer": "parsel",
        "draft": {"revision": 1, "items": [{"type": "field", "id": "alan", "field": "Alan", "redact": true}]}
    }]);
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&value.to_string()).expect("the drawing reads");
    let (mut app, _) = App::boot(None);
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.features.reset(None);
    let _ = app.update(Message::Run("data.featureTable"));
    app
}

fn headers(app: &App) -> Vec<String> {
    app.features_seen().map(|(_, h, _)| h).unwrap_or_default()
}

#[test]
fn a_hidden_column_is_out_of_the_table_the_copy_and_the_file_and_a_redacted_field_is_out_of_the_last_two() {
    use super::outgoing::ColumnsEvent as C;
    let mut app = app_with_secret();
    // The table shows the redacted field; what leaves it does not carry it (docs/adr/0546 §1).
    assert!(headers(&app).contains(&"Tapu alanı".to_owned()));
    let (text, count, withheld) = app.features_copy_text().expect("rows");
    assert_eq!(count, 5);
    assert_eq!(withheld, ["Tapu alanı"]);
    assert_eq!(
        text.lines().next(),
        Some("Ada\tParsel\tKullanım\tRuhsat\tTescil\tNot")
    );
    assert!(!text.contains("400.50") && !text.contains("1200"), "{text}");
    let (sheet, left_out) = app.features_sheet().expect("rows");
    assert_eq!(left_out, ["Tapu alanı"]);
    let names: Vec<&str> = sheet.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Tür", "Ada", "Parsel", "Kullanım", "Ruhsat", "Tescil", "Not"]);
    // A column the reader hides leaves the table, the copy and the file.
    ev(&mut app, Event::Columns(C::Toggle("Parsel".into())));
    assert!(!headers(&app).contains(&"Parsel".to_owned()));
    let (text, _, _) = app.features_copy_text().expect("rows");
    assert_eq!(text.lines().next(), Some("Ada\tKullanım\tRuhsat\tTescil\tNot"));
    let (sheet, _) = app.features_sheet().expect("rows");
    assert!(sheet.columns.iter().all(|c| c.name != "Parsel"));
    // The copy is what the table shows: a search narrows its rows (and the lines under the header).
    ev(&mut app, Event::Search("102".into()));
    let (text, count, _) = app.features_copy_text().expect("rows");
    assert_eq!((count, text.lines().count()), (2, 3));
    // A hidden column is no column to paste into: Yapıştır reads the shown ones.
    let rows = app.feature_rows(app.document.as_ref().expect("a drawing"));
    assert!(rows.visible.iter().all(|&j| rows.table.columns[j].key.as_deref() != Some("Parsel")));
    ev(&mut app, Event::Columns(C::ShowAll));
    assert!(headers(&app).contains(&"Parsel".to_owned()));
}

#[test]
fn statistics_and_charts_over_the_tables_rows_follow_its_filter_and_leave_redacted_fields_out() {
    use super::outgoing::Analyse;
    let mut app = app_with_secret();
    ev(&mut app, Event::Search("102".into()));
    ev(&mut app, Event::Analyse(Analyse::Statistics));
    assert_eq!(app.charts.show, Show::Table);
    let (objects, summaries) = app.statistics_seen().expect("the tab is open");
    assert_eq!(objects, "2 nesne");
    // No summary of the redacted field, the others are there.
    assert!(summaries.iter().all(|s| !s.starts_with("Tapu alanı")), "{summaries:?}");
    assert!(summaries.iter().any(|s| s.starts_with("Kullanım")), "{summaries:?}");
    // The table's filter changes: the tab follows; its own Göster takes the whole layer.
    ev(&mut app, Event::Search(String::new()));
    assert_eq!(app.statistics_seen().expect("open").0, "5 nesne");
    ev(&mut app, Event::Search("102".into()));
    assert_eq!(app.statistics_seen().expect("open").0, "2 nesne");
    let _ = app.update(Message::Statistics(crate::statistics::Event::Show(Show::All)));
    assert_eq!(app.statistics_seen().expect("open").0, "5 nesne");
    // A chart over the same rows: Çubuk by Kullanım counts only them.
    let _ = app.update(Message::Statistics(crate::statistics::Event::Show(Show::Table)));
    ev(&mut app, Event::Analyse(Analyse::Chart));
    let seen = app.chart_seen().expect("a chart");
    let total: u32 = seen
        .rows
        .iter()
        .filter_map(|r| r.split(" | ").nth(1)?.parse::<u32>().ok())
        .sum();
    assert_eq!(total, 2, "{seen:?}");
}

/// Parcels with fields of docs/adr/0629's kinds: a big integer, a time stamp, a UUID, JSON and binary values.
fn app_with_kinds() -> App {
    kinds_app(true)
}

/// [`app_with_kinds`]; `keep`: the parcels' own fields too (else only Ada beside the new ones, for the pictures).
fn kinds_app(keep: bool) -> App {
    use kentos_commands::{LayerField, LayerFieldKind};
    let mut app = app_with_table();
    let model = &mut app.document.as_mut().expect("a drawing").model;
    let slots: Vec<kentos_domain::Slot> = model
        .entities()
        .filter(|e| e.base().layer_id == "parsel")
        .map(|e| kentos_domain::Slot(e.base().id))
        .collect();
    let values = [
        ["9007199254740993", "2024-03-01T10:20:30.5+03:00", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001", r#"{"kot":[1.25,12345678901234567890]}"#, "QUJD"],
        ["-9223372036854775808", "2024-03-01T07:20:30Z", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0002", "[]", ""],
        ["12", "2023-12-31T23:59:59.999999999-05:00", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0003", r#""ç""#, "a2VudG9zIGNhZA=="],
    ];
    for (slot, row) in slots.iter().zip(values) {
        let mut e = model.get(*slot).cloned().expect("a parcel");
        for (name, v) in ["Kimlik", "Ölçüm anı", "Cihaz", "Ek bilgi", "İmza"].into_iter().zip(row) {
            e.base_mut().attrs.insert(name.into(), v.into());
        }
        model.update(*slot, e);
    }
    let mut fields = model
        .layers()
        .get("parsel")
        .map(|n| n.fields.clone())
        .unwrap_or_default();
    if !keep {
        fields.retain(|f| f.name == "Ada");
    }
    fields.extend([
        LayerField::new("Kimlik", LayerFieldKind::BigInt),
        LayerField::new("Ölçüm anı", LayerFieldKind::Instant),
        LayerField::new("Cihaz", LayerFieldKind::Uuid),
        LayerField::new("Ek bilgi", LayerFieldKind::Json),
        LayerField::new("İmza", LayerFieldKind::Binary),
    ]);
    assert_eq!(model.set_layer_fields("parsel", fields, &[]), Ok(true));
    app.features.reset(None);
    app
}

/// Sorts by the column headed `name` (no column hidden: the headers are the table's columns).
fn sort_by(app: &mut App, name: &str) {
    let (_, headers, _) = app.features_seen().expect("the tab is open");
    let j = headers.iter().position(|h| h == name).expect(name);
    ev(app, Event::Sort(j + 1));
}

/// docs/adr/0629 §3: a time stamp shows its day, time and offset and sorts by its moment in UTC, binary its size, a big
/// integer every digit sorted exactly.
#[test]
fn the_table_shows_and_sorts_the_new_kinds() {
    let mut app = app_with_kinds();
    let _ = app.update(Message::Run("data.featureTable"));
    let all = rows(&app).join("\n");
    assert!(all.contains("01.03.2024 10:20:30.5 +03:00"), "{all}");
    assert!(all.contains("01.03.2024 07:20:30 UTC"), "{all}");
    assert!(all.contains("ikili, 3 bayt"), "{all}");
    assert!(all.contains("9007199254740993"), "{all}");
    sort_by(&mut app, "Ölçüm anı");
    let sorted = rows(&app);
    let at = |s: &str| sorted.iter().position(|r| r.contains(s)).expect(s);
    // 2023-12-31 23:59:59.999999999 −05:00 is 2024-01-01 04:59:59.999999999 UTC: before both of 1 March.
    assert!(at("31.12.2023") < at("07:20:30 UTC"), "{sorted:?}");
    sort_by(&mut app, "Kimlik");
    let sorted = rows(&app);
    let at = |s: &str| sorted.iter().position(|r| r.contains(s)).expect(s);
    assert!(at("-9223372036854775808") < at("| 12 | 31.12.2023") && at("| 12 | 31.12.2023") < at("9007199254740993"), "{sorted:?}");
}

/// The new kinds in the table and in Öznitelikler, dark and light, at 1440×900 and 1100×650 (`.run/shots/alan-turleri-*`):
///
/// ```text
/// cargo test -p kentos-desktop features::tests::kinds_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn kinds_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let mut app = kinds_app(false);
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::Run("data.featureTable"));
            sort_by(&mut app, "Ölçüm anı");
            let first = app
                .document
                .as_ref()
                .and_then(|d| {
                    d.model
                        .entities()
                        .find(|e| e.base().attrs.get("Kimlik").map(String::as_str) == Some("9007199254740993"))
                })
                .map(|e| kentos_domain::Slot(e.base().id))
                .expect("the first parcel");
            app.selection.set([first]);
            app.props_closed.insert("general");
            app.props_closed.insert("geometry");
            let _ = app.update(Message::WindowResized(Size::new(width, height)));
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("alan-turleri-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
