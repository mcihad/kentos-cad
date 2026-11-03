//! Kübaj (volume_window.rs, plan/road.md Y8): the window computes the volume as it is typed, writes through
//! `road.volumes.edit` (one undo step) and writes the sheet as a file; and its pictures.

use crate::app::{App, Dialog, Message};
use crate::volume_window::Event as W;

/// A road project with `road.volumes.query`'s drawing: two corridors and two earthwork volumes.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.volumes.query.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Volumes(e));
}

fn model(app: &mut App) -> &mut kentos_domain::Document {
    &mut app.document.as_mut().expect("a drawing").model
}

#[test]
fn the_window_makes_a_volume_and_shows_it_computed() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.new"));
    assert_eq!(app.dialog, Some(Dialog::Volumes));
    send(&mut app, W::Name("Prizmoidal kübaj".into()));
    send(&mut app, W::Method(kentos_commands::VolumeMethod::Prismoidal));
    send(&mut app, W::Add);
    send(&mut app, W::Factor(0, 1, "600".into()));
    send(&mut app, W::Factor(0, 2, "1,25".into()));
    send(&mut app, W::Factor(0, 3, "0,9".into()));
    let w = app.volume_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("computes: {:?}", w.preview.why));
    assert!(rep.rows.len() > 20 && rep.cut > 1000.0, "{} rows, {} m³", rep.rows.len(), rep.cut);
    assert!((rep.factored_cut - 1.25 * rep.cut).abs() < 1e-6, "{rep:?}");
    send(&mut app, W::Save);
    assert!(app.volume_window.is_none(), "Kaydet closes the window");
    let doc = model(&mut app);
    assert_eq!(doc.road_volumes().len(), 3);
    assert_eq!(doc.road_volumes()[2].factors[0].cut, Some(1.25));
    assert_eq!(doc.undo().as_deref(), Some("Kübaj"));
    assert_eq!(doc.road_volumes().len(), 2);
}

#[test]
fn a_saved_volume_keeps_its_corridor_and_a_bad_factor_is_refused() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Corridor("c-2".into()));
    let w = app.volume_window.as_ref().expect("open");
    assert_eq!(w.preview.report.as_ref().map(|r| r.corridor.as_str()), Some("c-1"), "the corridor stays");
    send(&mut app, W::Add);
    send(&mut app, W::Factor(0, 2, "x".into()));
    assert!(app.volume_window.as_ref().is_some_and(|w| w.preview.why.is_some()));
    send(&mut app, W::Save);
    assert!(app.volume_window.is_some(), "the window stays");
    assert_eq!(model(&mut app).road_volumes()[0].factors.len(), 2);
}

#[test]
fn stripping_and_benching_are_typed_and_saved() {
    let mut app = app();
    let _ = app.update(Message::Run("volume.editor"));
    send(&mut app, W::Choose(0));
    let before = app.volume_window.as_ref().and_then(|w| w.preview.report.as_ref()).map(|r| r.cut).unwrap_or_default();
    send(&mut app, W::AddStrip);
    send(&mut app, W::Strip(0, 1, "600".into()));
    send(&mut app, W::Strip(0, 2, "0,3".into()));
    send(&mut app, W::Bench(0, "3".into()));
    send(&mut app, W::Bench(1, "0,5".into()));
    let w = app.volume_window.as_ref().expect("open");
    let rep = w.preview.report.as_ref().unwrap_or_else(|| panic!("computes: {:?}", w.preview.why));
    assert!(rep.strip.is_some_and(|v| v > 100.0) && rep.bench.is_some_and(|v| v > 10.0), "{:?} {:?}", rep.strip, rep.bench);
    assert!(rep.cut < before, "the stripped ground leaves less cut: {} → {}", before, rep.cut);
    send(&mut app, W::Save);
    let doc = model(&mut app);
    assert_eq!(doc.road_volumes()[0].strip[0].depth, 0.3);
    assert_eq!(doc.road_volumes()[0].bench.as_ref().map(|b| b.height), Some(0.5));
    assert_eq!(doc.undo().as_deref(), Some("Kübaj"));
}

#[test]
fn the_sheet_is_written_as_a_workbook_and_a_csv() {
    let mut app = app();
    let doc = model(&mut app);
    let def = doc.road_volumes()[0].clone();
    let (report, _) = kentos_native_application::road_volumes::preview(doc, &def).expect("computes");
    let sheet = kentos_native_application::road_volumes::sheet(&report);
    assert_eq!(sheet.rows.len(), report.rows.len() + 1, "a row a stretch and the totals");
    assert_eq!(sheet.rows.last().map(|r| r[0].as_str()), Some("Toplam"));
    let folder = std::env::temp_dir().join(format!("kentos-kubaj-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    for (format, name) in [(kentos_commands::WorkbookFormat::Xlsx, "kubaj.xlsx"), (kentos_commands::WorkbookFormat::Csv, "kubaj.csv")] {
        let path = folder.join(name);
        match crate::table_write::write(vec![sheet.clone()], format, &path) {
            crate::table_write::Outcome::Written { rows, .. } => assert_eq!(rows, report.rows.len() + 1),
            crate::table_write::Outcome::Failed(why) => panic!("{why}"),
        }
        assert!(std::fs::metadata(&path).is_ok_and(|m| m.len() > 200), "{name} written");
    }
    let csv = std::fs::read_to_string(folder.join("kubaj.csv")).expect("reads");
    // The writer's CSV opens with a byte order mark, for Excel (docs/adr/0223 §2).
    assert!(csv.trim_start_matches('\u{feff}').starts_with("Başlangıç km;Bitiş km;Uzunluk (m);Kazı (m³)"), "{csv}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// The window, wide and narrow, light and dark; `.run/gui-evidence/yol-y8-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib volume_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-y8-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("volume.editor"));
        shot(&mut app, "kubaj");
        // Between the surfaces of the side corridor, the stale one.
        send(&mut app, W::Show(true));
        shot(&mut app, "kubaj-cetvel");
        // Stripped and benched: the topsoil and the benches in the sheet.
        send(&mut app, W::AddStrip);
        send(&mut app, W::Strip(0, 1, "600".into()));
        send(&mut app, W::Strip(0, 2, "0,3".into()));
        send(&mut app, W::Bench(0, "3".into()));
        send(&mut app, W::Bench(1, "0,5".into()));
        shot(&mut app, "kubaj-siyirma");
        send(&mut app, W::Choose(1));
        send(&mut app, W::Show(false));
        shot(&mut app, "kubaj-yuzey");
        send(&mut app, W::Cancel);
    }
}
