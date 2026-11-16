//! BIM ruhsat paketi (docs/adr/8075; plan/bim-plan.md BIM-14) in Paftaları toplu yayınla, opened from Ruhsat takımı: the
//! switch adds the IFC model and the model quality control form; Denetle writes the model in memory, reads it back and
//! answers the form; the five results show apart and the questions left to the author take Evet or Hayır with a note.
//! The section building's model still lacks mandatory fields, so the package's errors stop the publication.

use crate::app::{App, Message};
use crate::exchange::Window;
use crate::exchange::sheet_publish::Event as Publish;
use crate::permit_package_tests::{check, permit_publication, section_building};

fn publication(app: &App) -> &crate::exchange::sheet_publish::State {
    match &app.exchange {
        Some(Window::SheetPublish(s)) => s,
        _ => panic!("Paftaları toplu yayınla is open"),
    }
}

fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-bim-ruhsat-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn the_bim_package_is_checked_from_its_file_and_waits_for_the_author_s_answers() {
    let mut app = section_building();
    let path = folder("denetim");
    permit_publication(&mut app, &path);
    let _ = app.sheet_publish_event(Publish::Bim(true));
    assert!(publication(&app).input.bim.is_some());
    check(&mut app);
    let s = publication(&app);
    let report = s.report.as_ref().expect("the check's report");
    let bim = report.bim.as_ref().expect("the package's part");
    assert_eq!(bim.rows.len(), 21);
    let open: Vec<u8> = bim.rows.iter().filter(|r| r.evidence == "inceleme").map(|r| r.no).collect();
    assert_eq!(open, [1, 7, 10], "the questions left to the author");
    assert!(!report.ready);
    let unanswered = report.issues.iter().filter(|i| i.code == "digital_form_unanswered").count();
    assert_eq!(unanswered, 3);
    // The form's 20th question holds: the building's quantities are the template's (FootPrintArea, docs/adr/8075).
    assert!(!report.issues.iter().any(|i| i.code == "digital_pset_wrong"), "{:?}", report.issues);
    // The author answers; Denetle again: the questions are no finding, the model's missing fields still are.
    for no in open {
        let _ = app.sheet_publish_event(Publish::BimAnswer(no, Some(true)));
    }
    let _ = app.sheet_publish_event(Publish::BimNote(1, "Bakanlığın sınırı yayımlanmadı".into()));
    assert!(!publication(&app).report.as_ref().is_some_and(|r| r.ready), "an answer asks Denetle again");
    check(&mut app);
    let report = publication(&app).report.as_ref().expect("the check's report");
    assert!(!report.issues.iter().any(|i| i.code == "digital_form_unanswered"));
    let row1 = report.bim.as_ref().expect("the part").rows.iter().find(|r| r.no == 1).expect("row 1");
    assert_eq!((row1.answer.as_str(), row1.note.as_str()), ("Evet", "Bakanlığın sınırı yayımlanmadı (müellif)"));
    assert!(report.issues.iter().any(|i| i.error && i.code == "digital_required_field"));
    assert!(!report.ready, "the model's missing fields stop it");
    let _ = std::fs::remove_dir_all(&path);
}

/// The window's BIM part, light and dark: the switch, the five results and the author's questions after Denetle;
/// `.run/gui-evidence/bim-ruhsat-paketi-8075/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib bim_permit_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bim-ruhsat-paketi-8075");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1500.0, 1100.0), "acik"), ("dark", Size::new(1300.0, 1000.0), "koyu")] {
        let mut app = section_building();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let path = folder(&format!("resim-{name}"));
        permit_publication(&mut app, &path);
        let _ = app.sheet_publish_event(Publish::Bim(true));
        check(&mut app);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let middle = Point::new(size.width / 2.0, size.height / 2.0);
        // The window scrolled to its end, where the package's part is.
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(middle, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-bim-paketi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        for no in [1, 7, 10] {
            let _ = app.sheet_publish_event(Publish::BimAnswer(no, Some(true)));
        }
        let _ = app.sheet_publish_event(Publish::BimNote(1, "Bakanlığın sınırı yayımlanmadı".into()));
        check(&mut app);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(middle, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-cevaplar-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = std::fs::remove_dir_all(&path);
    }
}
