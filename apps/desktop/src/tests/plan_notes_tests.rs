//! Plan notları (docs/adr/8084): the parcel's plan notes typed in Proje ayarları › Mimari, kept in the project, carried
//! in the audit facts and read by the standards package over the municipality's and the national regulation (PAİY
//! 69/3; 28/1 leaves the storey heights to the plan). On the two-storey house of `dated_sizes_tests` (Zemin kat and
//! 1. kat, 3.80 m each, one flat) applied for on 1.8.2023, when 28/1-c asked 3.60 m: a note of 3.50 m for the ground
//! storey binds it (3.80 m over it, 1.10.2017's 3.60 m said beside), a note of 4.00 m for the storeys lets the 1. kat
//! pass; with the notes taken away 28/1-c's 3.60 m holds both storeys again.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib plan_notes_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{ArchAuditOutcome as O, PlanNote, PlanNoteKey};

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

/// The house applied for on 1.8.2023, Proje ayarları open on its Mimari page with the two notes typed (the source of
/// the first left empty when `whole` is false).
fn typed(whole: bool) -> App {
    let (mut app, _, _) = crate::dated_sizes_tests::house();
    crate::regulation_dates_tests::dated(&mut app, Some("01.08.2023"));
    let _ = app.update(Message::Run("file.settings"));
    send(&mut app, SettingsEvent::Section(Section::Arch));
    send(&mut app, SettingsEvent::AddPlanNote);
    note(&mut app, 0, Edit::Value("3,50".into()));
    if whole {
        note(&mut app, 0, Edit::Source(" UİP 12.3.2019, plan notu 7 ".into()));
    }
    send(&mut app, SettingsEvent::AddPlanNote);
    note(&mut app, 1, Edit::Value("4".into()));
    note(&mut app, 1, Edit::Source("UİP 12.3.2019, plan notu 8".into()));
    app
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The storeys' findings and the storey rows of the rule set.
fn storeys(app: &App) -> (Vec<String>, Vec<(String, Option<String>, O)>) {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.code == "paiy_storey_height_over")
        .map(|f| f.message.clone())
        .collect();
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    let rows = set
        .rules
        .iter()
        .filter(|r| r.rule.contains("28/1-c") || r.rule.starts_with("Plan notu"))
        .map(|r| (r.rule.clone(), r.version.clone(), r.state))
        .collect();
    (found, rows)
}

#[test]
fn plan_notes_typed_in_the_settings_bind_the_storeys_over_the_regulation() {
    // A note without its source keeps Kaydet waiting.
    let mut app = typed(false);
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_some(), "Kaydet waits for the note's source");
    note(&mut app, 0, Edit::Source(" UİP 12.3.2019, plan notu 7 ".into()));
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_none(), "saved");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let notes = doc.settings().arch.as_ref().map(|a| a.plan_notes.clone()).unwrap_or_default();
    assert_eq!(
        notes,
        [
            PlanNote { key: PlanNoteKey::GroundStoreyHeight, value: 3.5, source: "UİP 12.3.2019, plan notu 7".into() },
            PlanNote { key: PlanNoteKey::StoreyHeight, value: 4.0, source: "UİP 12.3.2019, plan notu 8".into() },
        ]
    );
    // The facts carry them as written.
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let keys: Vec<(&str, f64)> = facts.plan_notes.iter().map(|n| (n.key.as_str(), n.value)).collect();
    assert_eq!(keys, [("groundStoreyHeight", 3.5), ("storeyHeight", 4.0)]);
    // Denetim: the ground storey over its note, 28/1-c's 3.60 m beside; the 1. kat within the storeys' 4.00 m.
    checked(&mut app);
    let (found, rows) = storeys(&app);
    assert_eq!(
        found,
        ["“Zemin kat” kat yüksekliği 3.80 m > 3.50 m — Plan notu (UİP 12.3.2019, plan notu 7); PAİY 28/1-c \
          (1.10.2017): 3.60 m"]
    );
    assert_eq!(
        rows,
        [
            ("Plan notu: zemin kat".to_owned(), None, O::Failed),
            ("PAİY 28/1-c".to_owned(), Some("1.10.2017".to_owned()), O::OutOfScope),
            ("Plan notu: kat yüksekliği".to_owned(), None, O::Passed),
        ]
    );
    let rule = app.arch.arch_audit.audited.as_ref().expect("checked").findings.iter()
        .find(|f| f.code == "paiy_storey_height_over").and_then(|f| f.rule.clone());
    assert_eq!(rule.as_deref(), Some("Plan notu · UİP 12.3.2019, plan notu 7"));
    // The notes taken away: 28/1-c's 3.60 m of 1.10.2017 holds both storeys.
    let _ = app.update(Message::Run("file.settings"));
    send(&mut app, SettingsEvent::Section(Section::Arch));
    note(&mut app, 1, Edit::Delete);
    note(&mut app, 0, Edit::Delete);
    send(&mut app, SettingsEvent::Save);
    assert!(app.project.is_none(), "saved");
    checked(&mut app);
    let (found, rows) = storeys(&app);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found.iter().all(|m| m.ends_with("> 3.60 m (konut bölgesi, 2 kat) — PAİY 28/1-c (1.10.2017)")), "{found:#?}");
    assert_eq!(rows, [("PAİY 28/1-c".to_owned(), Some("1.10.2017".to_owned()), O::Failed)]);
}

/// Proje ayarları › Mimari's Plan notları with the two notes, and Denetim with the ground storey's finding, light and
/// dark; `.run/gui-evidence/plan-notlari-8084/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/plan-notlari-8084");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut app = typed(true);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // Down the Mimari page to Plan notları: to its end, then up to the group.
        let at = iced::Point::new(size.width * 0.55, size.height * 0.5);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 2.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-plan-notlari-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        // Kaydet, then Denetim on the ground storey with its finding chosen.
        send(&mut app, SettingsEvent::Save);
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_storey_height_over");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-plan-notu-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
