//! Planın bina yüksekliği ve kat adedi (docs/adr/8098; PAİY 5/11): the plan's notes typed in Proje ayarları › Mimari ›
//! Plan notları, kept in the project (Mimari data 16) and read by the standards package. On the two-storey house of
//! `dated_sizes_tests` (Zemin kat ±0.00 and 1. kat +3.80, each 3.80 m floor to floor) applied for on 2.7.2026: its eaves
//! level is the top of the 1. kat's ceiling slab, 7.60 m over ±0.00, past a note of 7.00 m and within one of 8.00 m;
//! its two storeys pass a note of one storey and hold one of two. With one of the two notes alone the other is read by
//! PAİY 9/1-c's conversion (docs/adr/8157): 8.00 m holds one storey (1.20 m + 4.00 m = 5.20 m; two storeys 9.20 m),
//! the house's two are an expert's to review; 10.00 m holds two; two storeys convert to 9.20 m, the house's 7.60 m
//! within it. Raised by 1.50 m, the house's ground storey fails 4/1-yyy's +1.20 and holds the plan's own subasman note
//! of +2.40 (docs/adr/8158); a least above the most keeps Kaydet waiting.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib plan_limits_tests::screens -- --ignored --nocapture
//! ```

use crate::arch::ArchState;
use crate::arch_ext::*;
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
    for (i, (key, value)) in [
        (PlanNoteKey::BuildingHeight, height),
        (PlanNoteKey::Storeys, storeys),
    ]
    .into_iter()
    .enumerate()
    {
        send(&mut app, SettingsEvent::AddPlanNote);
        note(&mut app, i, Edit::Key(key));
        note(&mut app, i, Edit::Value(value.into()));
        note(
            &mut app,
            i,
            Edit::Source("UİP 12.3.2019, plan notu 3".into()),
        );
    }
    app
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(
        app.modules.arch().arch_audit.regulation_failed,
        None,
        "the national regulation's rule answers"
    );
}

/// The plan limits' findings (code, message) and rows (rule, outcome).
#[allow(clippy::type_complexity)]
fn limits(app: &App) -> (Vec<(String, String)>, Vec<(String, O)>) {
    let audited = app
        .modules
        .arch()
        .arch_audit
        .audited
        .as_ref()
        .expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code == "paiy_building_height_over" || f.code == "paiy_storey_count_over")
        .map(|f| (f.code.clone(), f.message.clone()))
        .collect();
    let set = app
        .modules
        .arch()
        .arch_audit
        .rule_set
        .as_ref()
        .expect("the rule set");
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
    assert!(
        app.project.is_some(),
        "Kaydet waits for a whole number of storeys"
    );
    note(&mut app, 1, Edit::Value("1".into()));
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_none(), "saved");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let notes: Vec<(PlanNoteKey, f64)> = doc
        .settings()
        .arch
        .as_ref()
        .expect("arch")
        .plan_notes
        .iter()
        .map(|n| (n.key, n.value))
        .collect();
    assert_eq!(
        notes,
        [
            (PlanNoteKey::BuildingHeight, 7.0),
            (PlanNoteKey::Storeys, 1.0)
        ]
    );
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
        [
            ("Plan notu: bina yüksekliği".to_owned(), O::Failed),
            ("Plan notu: kat adedi".to_owned(), O::Failed)
        ]
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
        [
            ("Plan notu: bina yüksekliği".to_owned(), O::Passed),
            ("Plan notu: kat adedi".to_owned(), O::Passed)
        ]
    );
}

/// The house with one of the plan's values alone typed and saved: `key` = `value`.
fn typed_alone(key: PlanNoteKey, value: &str) -> App {
    let (mut app, _, _) = crate::dated_sizes_tests::house();
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    let _ = app.update(Message::Run("file.settings"));
    send(&mut app, SettingsEvent::Section(Section::Arch));
    send(&mut app, SettingsEvent::AddPlanNote);
    note(&mut app, 0, Edit::Key(key));
    note(&mut app, 0, Edit::Value(value.into()));
    note(
        &mut app,
        0,
        Edit::Source("UİP 12.3.2019, plan notu 3".into()),
    );
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_none(), "saved");
    app
}

/// The conversion's findings (message, state) and its row's outcome.
fn conversion(app: &App) -> (Vec<(String, Option<kentos_commands::ArchAuditState>)>, O) {
    let audited = app
        .modules
        .arch()
        .arch_audit
        .audited
        .as_ref()
        .expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code == "paiy_height_storeys_conversion")
        .map(|f| (f.message.clone(), f.state))
        .collect();
    let set = app
        .modules
        .arch()
        .arch_audit
        .rule_set
        .as_ref()
        .expect("the rule set");
    let row = set
        .rules
        .iter()
        .find(|r| r.rule == "PAİY 9/1-c")
        .unwrap_or_else(|| panic!("9/1-c: {:#?}", set.rules));
    (found, row.state)
}

#[test]
fn the_plan_s_building_height_alone_converts_to_a_storey_count() {
    // 8.00 m: one storey (5.20 m), two 9.20 m; the house's two storeys are for an expert, its 7.60 m hold 5/11.
    let mut app = typed_alone(PlanNoteKey::BuildingHeight, "8");
    checked(&mut app);
    let (found, row) = conversion(&app);
    assert_eq!(row, O::ExpertReview);
    assert_eq!(found.len(), 1, "{found:?}");
    let (message, state) = &found[0];
    assert_eq!(*state, Some(kentos_commands::ArchAuditState::ExpertReview));
    assert!(
        message.contains("Planın bina yüksekliği 8.00 m")
            && message.contains("1 kat 5.20 m, 2 kat 9.20 m")
            && message.contains("1 kata karşılık gelir; binada 2 kat"),
        "{message}"
    );
    let (limits_found, rows) = limits(&app);
    assert!(limits_found.is_empty(), "{limits_found:?}");
    assert_eq!(rows, [("Plan notu: bina yüksekliği".to_owned(), O::Passed)]);
    // 10.00 m: two storeys; the house's two hold it.
    let _ = app.update(Message::Run("file.settings"));
    send(&mut app, SettingsEvent::Section(Section::Arch));
    note(&mut app, 0, Edit::Value("10".into()));
    send(&mut app, SettingsEvent::Save);
    checked(&mut app);
    assert_eq!(conversion(&app), (Vec::new(), O::Passed));
    // The storey count alone: two storeys convert to 9.20 m, the house is 7.60 m.
    let mut app = typed_alone(PlanNoteKey::Storeys, "2");
    checked(&mut app);
    assert_eq!(conversion(&app), (Vec::new(), O::Passed));
}

/// Plan notları with the plan's building height and storey count, and Denetim with their findings, light and dark;
/// `.run/gui-evidence/plan-yukseklik-8098/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/plan-yukseklik-8098");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(1280.0, 800.0), "koyu"),
    ] {
        let mut app = typed("7", "1");
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
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
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
        send(&mut app, SettingsEvent::Save);
        app.toasts.clear();
        checked(&mut app);
        let audited = app
            .modules
            .arch()
            .arch_audit
            .audited
            .as_ref()
            .expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited
            .findings
            .iter()
            .position(|f| f.code == "paiy_building_height_over");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-denetim-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Denetim with the conversion's expert review (8.00 m alone: one storey, the house has two), light and dark;
/// `.run/gui-evidence/tahvil-8157/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn conversion_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/tahvil-8157");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sizes = [
        ("light", Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(1280.0, 800.0), "koyu"),
    ];
    for (theme, size, name) in sizes {
        let mut app = typed_alone(PlanNoteKey::BuildingHeight, "8");
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.toasts.clear();
        checked(&mut app);
        let audited = app
            .modules
            .arch()
            .arch_audit
            .audited
            .as_ref()
            .expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited
            .findings
            .iter()
            .position(|f| f.code == "paiy_height_storeys_conversion");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-tahvil-denetim-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}

/// The house applied for on 2.7.2026 with its storeys raised by `by` m: its ground storey's floor `by` over ±0.00.
fn raised(by: f64) -> App {
    let (mut app, _, _) = crate::dated_sizes_tests::house();
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut settings = doc.settings().clone();
    for l in &mut settings.arch.as_mut().expect("the storeys").levels {
        l.elevation += by;
    }
    doc.set_settings(settings);
    app
}

/// The subasman's findings and its rows (rule, outcome): 4/1-yyy's and the plan's notes'.
fn plinth(app: &App) -> (Vec<String>, Vec<(String, O)>) {
    let audited = app
        .modules
        .arch()
        .arch_audit
        .audited
        .as_ref()
        .expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code == "paiy_plinth_level")
        .map(|f| f.message.clone())
        .collect();
    let set = app
        .modules
        .arch()
        .arch_audit
        .rule_set
        .as_ref()
        .expect("the rule set");
    let rows = set
        .rules
        .iter()
        .filter(|r| r.rule == "PAİY 4/1-yyy" || r.rule.starts_with("Plan notu: subasman"))
        .map(|r| (r.rule.clone(), r.state))
        .collect();
    (found, rows)
}

/// The plan's subasman notes typed in Plan notları (docs/adr/8158): `notes` as (measure, value), then Kaydet.
fn plinth_notes(app: &mut App, notes: &[(PlanNoteKey, &str)]) {
    let _ = app.update(Message::Run("file.settings"));
    send(app, SettingsEvent::Section(Section::Arch));
    for (i, (key, value)) in notes.iter().enumerate() {
        send(app, SettingsEvent::AddPlanNote);
        note(app, i, Edit::Key(*key));
        note(app, i, Edit::Value((*value).into()));
        note(app, i, Edit::Source("UİP 12.3.2019, plan notu 56".into()));
    }
    send(app, SettingsEvent::Save);
}

#[test]
fn the_plan_s_subasman_notes_come_before_4_1_yyy() {
    // The ground storey's floor at +1.50: past 4/1-yyy's +1.20.
    let mut app = raised(1.5);
    checked(&mut app);
    let (found, rows) = plinth(&app);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("+1.50 m, en çok +1.20 m") && found[0].contains("PAİY 4/1-yyy"),
        "{}",
        found[0]
    );
    assert_eq!(rows, [("PAİY 4/1-yyy".to_owned(), O::Failed)]);
    // The plan lets it stand at +2.40 at most: its note binds, 4/1-yyy reads the least end only.
    plinth_notes(&mut app, &[(PlanNoteKey::PlinthMost, "2,40")]);
    assert!(app.project.is_none(), "saved");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let keys: Vec<(&str, f64)> = facts
        .plan_notes
        .iter()
        .map(|n| (n.key.as_str(), n.value))
        .collect();
    assert_eq!(keys, [("plinthMost", 2.4)]);
    checked(&mut app);
    let (found, rows) = plinth(&app);
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(
        rows,
        [
            ("PAİY 4/1-yyy".to_owned(), O::Passed),
            ("Plan notu: subasman kotu (en çok)".to_owned(), O::Passed)
        ]
    );
}

#[test]
fn a_subasman_least_above_its_most_waits() {
    let mut app = raised(0.0);
    plinth_notes(
        &mut app,
        &[
            (PlanNoteKey::PlinthMost, "0,50"),
            (PlanNoteKey::PlinthLeast, "1"),
        ],
    );
    assert!(
        app.project.is_some(),
        "Kaydet waits for a least not above the most"
    );
    note(&mut app, 1, Edit::Value("-0,50".into()));
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_none(), "saved");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let notes: Vec<(PlanNoteKey, f64)> = doc
        .settings()
        .arch
        .as_ref()
        .expect("arch")
        .plan_notes
        .iter()
        .map(|n| (n.key, n.value))
        .collect();
    assert_eq!(
        notes,
        [
            (PlanNoteKey::PlinthMost, 0.5),
            (PlanNoteKey::PlinthLeast, -0.5)
        ]
    );
}

/// Plan notları with the plan's subasman notes, light and dark; `.run/gui-evidence/subasman-plan-8158/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn plinth_screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/subasman-plan-8158");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sizes = [
        ("light", Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(1280.0, 800.0), "koyu"),
    ];
    for (theme, size, name) in sizes {
        let mut app = raised(1.5);
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("file.settings"));
        send(&mut app, SettingsEvent::Section(Section::Arch));
        let notes = [
            (PlanNoteKey::PlinthMost, "2,40"),
            (PlanNoteKey::PlinthLeast, "-0,50"),
        ];
        for (i, (key, value)) in notes.into_iter().enumerate() {
            send(&mut app, SettingsEvent::AddPlanNote);
            note(&mut app, i, Edit::Key(key));
            note(&mut app, i, Edit::Value(value.into()));
            note(
                &mut app,
                i,
                Edit::Source("UİP 12.3.2019, plan notu 56".into()),
            );
        }
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
        let file = out.join(format!("1-subasman-notlari-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}

/// The plan's basement note (docs/adr/8162): the house with two basements under it (−3.80 and −7.60) fails a note of
/// one and holds one of two.
#[test]
fn the_plan_s_basement_note_counts_the_storeys_under_the_ground_storey() {
    for (most, outcome) in [("1", O::Failed), ("2", O::Passed)] {
        let mut app = raised(0.0);
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        let mut settings = doc.settings().clone();
        let levels = &mut settings.arch.as_mut().expect("the storeys").levels;
        for (id, name, elevation) in [("b1", "1. bodrum kat", -3.8), ("b2", "2. bodrum kat", -7.6)]
        {
            levels.push(kentos_commands::ArchLevel {
                id: id.into(),
                name: name.into(),
                elevation,
                height: Some(3.8),
            });
        }
        doc.set_settings(settings);
        crate::projection_tests::plan_notes(&mut app, &[(PlanNoteKey::BasementsMost, most)]);
        checked(&mut app);
        let set = app
            .modules
            .arch()
            .arch_audit
            .rule_set
            .as_ref()
            .expect("the rule set");
        let row = set
            .rules
            .iter()
            .find(|r| r.rule == "Plan notu: bodrum kat adedi")
            .map(|r| r.state);
        assert_eq!(row, Some(outcome), "{:#?}", set.rules);
        let audited = app
            .modules
            .arch()
            .arch_audit
            .audited
            .as_ref()
            .expect("checked");
        let said = audited
            .findings
            .iter()
            .any(|f| f.code == "paiy_basement_count_over" && f.message.contains("2 > 1"));
        assert_eq!(said, outcome == O::Failed);
    }
}
