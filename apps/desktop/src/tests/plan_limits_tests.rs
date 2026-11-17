//! Planın bina yüksekliği ve kat adedi (docs/adr/8098; PAİY 5/11): the plan's notes typed in Proje ayarları › Mimari ›
//! Plan notları, kept in the project (Mimari data 16) and read by the standards package. On the two-storey house of
//! `dated_sizes_tests` (Zemin kat ±0.00 and 1. kat +3.80, each 3.80 m floor to floor) applied for on 2.7.2026: its eaves
//! level is the top of the 1. kat's ceiling slab, 7.60 m over ±0.00, past a note of 7.00 m and within one of 8.00 m;
//! its two storeys pass a note of one storey and hold one of two.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib plan_limits_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{ArchAuditOutcome as O, PlanNoteKey};

use crate::app::{App, Message};
use crate::project::plan_notes::Edit;
use crate::project::settings::Section;
use crate::project::{SettingsEvent, settings_message};

fn send(app: &mut App, e: SettingsEvent) {
    let _ = app.update(settings_message(e));
}

fn note(app: &mut App, i: usize, e: Edit) {
    send(app, SettingsEvent::ArchPlanNote(i, e));
}

/// The house applied for on 2.7.2026, Proje ayarları open on its Mimari page with the plan's building height and storey
/// count typed: `height` m and `storeys`.
fn typed(height: &str, storeys: &str) -> App {
    let (mut app, _, _) = crate::dated_sizes_tests::house();
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    let _ = app.update(Message::Run("file.settings"));
    send(&mut app, SettingsEvent::Section(Section::Arch));
    for (i, (key, value)) in [(PlanNoteKey::BuildingHeight, height), (PlanNoteKey::Storeys, storeys)].into_iter().enumerate()
    {
        send(&mut app, SettingsEvent::AddPlanNote);
        note(&mut app, i, Edit::Key(key));
        note(&mut app, i, Edit::Value(value.into()));
        note(&mut app, i, Edit::Source("UİP 12.3.2019, plan notu 3".into()));
    }
    app
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The plan limits' findings (code, message) and rows (rule, outcome).
fn limits(app: &App) -> (Vec<(String, String)>, Vec<(String, O)>) {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code == "paiy_building_height_over" || f.code == "paiy_storey_count_over")
        .map(|f| (f.code.clone(), f.message.clone()))
        .collect();
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    let rows = set
        .rules
        .iter()
        .filter(|r| r.rule == "Plan notu: bina yüksekliği" || r.rule == "Plan notu: kat adedi")
        .map(|r| (r.rule.clone(), r.state))
        .collect();
    (found, rows)
}

#[test]
fn the_plan_s_building_height_and_storey_count_typed_in_the_settings_are_not_passed() {
    // A storey count is a whole number: Kaydet waits.
    let mut app = typed("7", "1,5");
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_some(), "Kaydet waits for a whole number of storeys");
    note(&mut app, 1, Edit::Value("1".into()));
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_none(), "saved");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let notes: Vec<(PlanNoteKey, f64)> =
        doc.settings().arch.as_ref().expect("arch").plan_notes.iter().map(|n| (n.key, n.value)).collect();
    assert_eq!(notes, [(PlanNoteKey::BuildingHeight, 7.0), (PlanNoteKey::Storeys, 1.0)]);
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let keys: Vec<&str> = facts.plan_notes.iter().map(|n| n.key.as_str()).collect();
    assert_eq!(keys, ["buildingHeight", "storeys"]);

    checked(&mut app);
    let (found, rows) = limits(&app);
    assert_eq!(
        found,
        [
            (
                "paiy_building_height_over".to_owned(),
                "Bina yüksekliği 7.60 m > 7.00 m (±0,00'dan “1. kat” katının tavan döşemesinin üstüne) — Plan notu \
                 (UİP 12.3.2019, plan notu 3); PAİY 5/11"
                    .to_owned()
            ),
            (
                "paiy_storey_count_over".to_owned(),
                "Kat adedi 2 > 1 (±0,00 ve üstündeki zemin ve normal katlar) — Plan notu (UİP 12.3.2019, plan notu 3); \
                 PAİY 5/11"
                    .to_owned()
            ),
        ]
    );
    assert_eq!(
        rows,
        [("Plan notu: bina yüksekliği".to_owned(), O::Failed), ("Plan notu: kat adedi".to_owned(), O::Failed)]
    );

    // 8.00 m and two storeys: both hold.
    let _ = app.update(Message::Run("file.settings"));
    send(&mut app, SettingsEvent::Section(Section::Arch));
    note(&mut app, 0, Edit::Value("8".into()));
    note(&mut app, 1, Edit::Value("2".into()));
    send(&mut app, SettingsEvent::Save);
    checked(&mut app);
    let (found, rows) = limits(&app);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        rows,
        [("Plan notu: bina yüksekliği".to_owned(), O::Passed), ("Plan notu: kat adedi".to_owned(), O::Passed)]
    );
}

/// Plan notları with the plan's building height and storey count, and Denetim with their findings, light and dark;
/// `.run/gui-evidence/plan-yukseklik-8098/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/plan-yukseklik-8098");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut app = typed("7", "1");
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // Down the Mimari page to Plan notları: to its end, then up past the groups under it.
        let at = iced::Point::new(size.width * 0.55, size.height * 0.5);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 5.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-plan-notlari-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        send(&mut app, SettingsEvent::Save);
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
