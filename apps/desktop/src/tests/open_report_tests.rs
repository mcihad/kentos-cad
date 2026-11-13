//! Açılış raporu (usage review UR-014): a DWG, DXF or NCZ opened as a drawing of its own keeps its reader's report
//! with it; the window comes up by itself when something was left out or changed, comes back with Dosya › Açılış
//! raporu, and a clean file brings no window. `open_report_screens` draws it:
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib open_report_tests::open_report_screens -- --ignored --nocapture
//! ```

use std::path::PathBuf;

use crate::app::{App, Dialog, Message};
use crate::files_testing::{drive, scratch};
use crate::open_report_window::{Event, Fate};

/// A metre DXF with a line, and an IMAGE too when `image`.
fn dxf(name: &str, image: bool) -> PathBuf {
    let mut text = String::from("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n");
    text.push_str("0\nLINE\n8\n0\n10\n0\n20\n0\n11\n10\n21\n0\n");
    if image {
        text.push_str("0\nIMAGE\n8\n0\n10\n0\n20\n0\n11\n1\n21\n0\n12\n0\n22\n1\n13\n100\n23\n100\n");
    }
    text.push_str("0\nENDSEC\n0\nEOF\n");
    let path = scratch("acilis-raporu").join(name);
    std::fs::write(&path, text).expect("writes the DXF");
    path
}

fn opened(path: PathBuf) -> App {
    let (mut app, _) = App::boot(None);
    let task = app.start_opening(path, crate::opening::Purpose::File);
    drive(&mut app, task);
    assert!(app.document.is_some(), "opened: {:?}", app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>());
    app
}

#[test]
fn a_lossy_file_brings_its_report_and_it_comes_back() {
    let path = dxf("resimli.dxf", true);
    let before = std::fs::read(&path).expect("the source");
    let mut app = opened(path.clone());
    assert_eq!(app.dialog, Some(Dialog::OpenReport), "up by itself: something was left out");
    let report = app.open_report().expect("kept with the drawing");
    assert_eq!(report.file, "resimli.dxf");
    assert!(
        report.items.iter().any(|(fate, item)| *fate == Fate::Left && item.what.contains("IMAGE")),
        "{:?}",
        report.items
    );
    let _ = app.update(Message::OpenReport(Event::Close));
    assert_eq!(app.dialog, None);
    assert!(app.available("file.openReport"));
    let _ = app.update(Message::Run("file.openReport"));
    assert_eq!(app.dialog, Some(Dialog::OpenReport), "Dosya › Açılış raporu brings it back");
    assert_eq!(std::fs::read(&path).expect("the source"), before, "the source file is not changed");
}

#[test]
fn a_clean_file_brings_no_window_and_a_kcad_has_no_report() {
    let app = opened(dxf("temiz.dxf", false));
    assert_eq!(app.dialog, None, "nothing left out: no window");
    assert!(app.open_report().is_some_and(|r| !r.lossy()));

    let mut app = crate::files_testing::app_with_drawing();
    assert!(!app.available("file.openReport"), "a KentOS drawing has no open report");
    let _ = app.update(Message::Run("file.openReport"));
    assert_eq!(app.dialog, None);
    assert!(crate::files_testing::last_said(&app).contains("açılış raporu yalnız onlarda"));
}

/// UR-014's pictures (`.run/gui-evidence/kullanim-UR-014/`), light and dark.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn open_report_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kullanim-UR-014");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = opened(dxf("resimli.dxf", true));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("acilis-raporu-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // Its place on the ribbon: Dosya › Proje.
        let _ = app.update(Message::OpenReport(Event::Close));
        let _ = app.update(Message::RibbonTab("file"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("serit-yeri-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
