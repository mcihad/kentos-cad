//! Binanın kendi plan notları (docs/adr/8163; kademe hattı, PAİY 4/1-mm): the two-storey house of `dated_sizes_tests`
//! (Zemin kat ±0.00 and 1. kat +3.80, each 3.80 m floor to floor) as A Blok and three storeys beside it (B zemin kat
//! ±0.00, B 1. kat +3.00, B 2. kat +6.00, each 3.00 m) as B Blok, applied for on 2.7.2026; the project's Plan notları
//! gives Bina yüksekliği 9.50 m. A Blok's own note of 7.00 m, typed in Bina programı › Binalar and written by Kaydet,
//! holds A Blok's eaves level of 7.60 m to it: A fails its own note, B holds the project's 9.50 m with its 9.00 m. With
//! A's note at 8.00 m both pass; a note with no source keeps Kaydet waiting.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib building_plan_notes_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{ArchAuditOutcome as O, PlanNoteKey};

use crate::app::{App, Message};
use crate::arch;
use crate::program_window::{Event, Tab};
use crate::project::plan_notes::Edit;

const A_SOURCE: &str = "UİP 12.3.2019, kademe hattı A alanı";

fn ev(app: &mut App, e: Event) {
    let _ = app.update(Message::Arch(arch::Event::Program(e)));
}

/// The house as A Blok and three storeys beside it as B Blok, the project's Bina yüksekliği note 9.50 m; Bina programı
/// open on Binalar with the two blocks typed, not yet written.
fn blocks() -> App {
    let (mut app, _, _) = crate::dated_sizes_tests::house();
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut settings = doc.settings().clone();
    let mimari = settings.arch.as_mut().expect("the storeys");
    for (id, name, elevation) in [("m00", "B zemin kat", 0.0), ("m01", "B 1. kat", 3.0), ("m02", "B 2. kat", 6.0)] {
        mimari.levels.push(kentos_commands::ArchLevel { id: id.into(), name: name.into(), elevation, height: Some(3.0) });
    }
    mimari.plan_notes = vec![kentos_commands::PlanNote {
        key: PlanNoteKey::BuildingHeight,
        value: 9.5,
        source: "UİP 12.3.2019, plan notu 3".into(),
    }];
    doc.set_settings(settings);
    let _ = app.update(Message::Run("arch.program"));
    ev(&mut app, Event::Tab(Tab::Buildings));
    for (i, name, levels) in [(0, "A Blok", &["k00", "k01"][..]), (1, "B Blok", &["m00", "m01", "m02"][..])] {
        ev(&mut app, Event::AddBuilding);
        ev(&mut app, Event::BuildingName(i, name.into()));
        for l in levels {
            ev(&mut app, Event::BuildingLevel(i, (*l).into()));
        }
    }
    app
}

/// A Blok's own Bina yüksekliği note typed in Binalar: `value` m, from `source`.
fn a_note(app: &mut App, value: &str, source: &str) {
    ev(app, Event::AddBuildingPlanNote(0));
    for e in [Edit::Key(PlanNoteKey::BuildingHeight), Edit::Value(value.into()), Edit::Source(source.into())] {
        ev(app, Event::BuildingPlanNote(0, 0, e));
    }
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The building height's findings (message) and the rows of its notes (rule, outcome).
fn heights(app: &App) -> (Vec<String>, Vec<(String, O)>) {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let found =
        audited.findings.iter().filter(|f| f.code == "paiy_building_height_over").map(|f| f.message.clone()).collect();
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    let rows = set
        .rules
        .iter()
        .filter(|r| r.rule.starts_with("Plan notu: bina yüksekliği"))
        .map(|r| (r.rule.clone(), r.state))
        .collect();
    (found, rows)
}

#[test]
fn a_building_s_own_plan_note_comes_before_the_project_s_on_its_storeys() {
    let mut app = blocks();
    // A note without its source waits: Kaydet says why and writes nothing.
    a_note(&mut app, "7", " ");
    ev(&mut app, Event::Save);
    assert!(app.document.as_ref().expect("a drawing").model.program().buildings.is_empty(), "nothing written");
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("“A Blok” binasının plan notları: “Bina yüksekliği (en çok)” plan notunun kaynağını yazın"), "{said}");
    ev(&mut app, Event::BuildingPlanNote(0, 0, Edit::Source(A_SOURCE.into())));
    ev(&mut app, Event::Save);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let program = doc.program();
    let own: Vec<(PlanNoteKey, f64, &str)> =
        program.buildings[0].plan_notes.iter().map(|n| (n.key, n.value, n.source.as_str())).collect();
    assert_eq!(own, [(PlanNoteKey::BuildingHeight, 7.0, A_SOURCE)]);
    assert!(program.buildings[1].plan_notes.is_empty());
    // A Blok's storeys read its own note, B Blok's the project's.
    let mimari = doc.settings().arch.clone().expect("Mimari");
    let on = |level: &str| -> Vec<f64> { mimari.plan_notes_on(level).iter().map(|n| n.value).collect() };
    assert_eq!((on("k01"), on("m02")), (vec![7.0], vec![9.5]));
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let storey = |id: &str| facts.storeys.iter().find(|s| s.level == id).expect("the storey");
    assert_eq!(storey("k01").plan_notes.iter().map(|n| n.key.as_str()).collect::<Vec<_>>(), ["buildingHeight"]);
    assert_eq!(storey("k01").building_name.as_deref(), Some("A Blok"));
    assert!(storey("m02").plan_notes.is_empty());
    assert_eq!(storey("m02").building_name.as_deref(), Some("B Blok"));

    ev(&mut app, Event::Close);
    checked(&mut app);
    let (found, rows) = heights(&app);
    assert_eq!(
        found,
        ["A Blok: Bina yüksekliği 7.60 m > 7.00 m (±0,00'dan “1. kat” katının tavan döşemesinin üstüne) — A Blok plan \
          notu (UİP 12.3.2019, kademe hattı A alanı); PAİY 5/11"]
    );
    assert_eq!(
        rows,
        [
            ("Plan notu: bina yüksekliği (A Blok)".to_owned(), O::Failed),
            ("Plan notu: bina yüksekliği".to_owned(), O::Passed)
        ]
    );

    // A Blok's note at 8.00 m: both blocks hold.
    let _ = app.update(Message::Run("arch.program"));
    ev(&mut app, Event::Tab(Tab::Buildings));
    ev(&mut app, Event::BuildingPlanNote(0, 0, Edit::Value("8,00".into())));
    ev(&mut app, Event::Save);
    ev(&mut app, Event::Close);
    checked(&mut app);
    let (found, rows) = heights(&app);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        rows,
        [
            ("Plan notu: bina yüksekliği (A Blok)".to_owned(), O::Passed),
            ("Plan notu: bina yüksekliği".to_owned(), O::Passed)
        ]
    );
}

/// Binalar with A Blok's own plan note, and Denetim with its finding chosen, light and dark;
/// `.run/gui-evidence/bina-plan-notlari-8163/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bina-plan-notlari-8163");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sizes = [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")];
    for (theme, size, name) in sizes {
        let mut app = blocks();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        a_note(&mut app, "7,00", A_SOURCE);
        ev(&mut app, Event::Save);
        ev(&mut app, Event::ChooseBuilding(0));
        app.toasts.clear();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // Down the building's form to Plan notları.
        let at = iced::Point::new(size.width * 0.62, size.height * 0.55);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -6.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-binanin-plan-notlari-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        ev(&mut app, Event::Close);
        app.toasts.clear();
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_building_height_over");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
