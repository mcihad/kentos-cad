//! Bölge (docs/adr/8085): the parcel's area's use in the plan, chosen in Proje ayarları › Mimari › İmar durumu, reads
//! the storeys' heights by PAİY 28/1's a, b, c or ç. The two-storey house of `dated_sizes_tests` made an office (its unit
//! Büro), Zemin kat 4.80 m and 1. kat 4.00 m, applied for on 2.7.2026: with no zone written its limit turns on the plan's
//! use (eksik veri); Ticaret alanı asks 4.50 m of the ground storey (it fails) and 4.00 m of the others (it holds); with
//! Sivas's 5.00 m the ground storey is a conflict the administration decides.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib zone_tests::screens -- --ignored --nocapture
//! ```

use crate::arch::ArchState;
use crate::arch_ext::*;
use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S, Zone};
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::project::settings::Section;
use crate::project::{SettingsEvent, settings_message};

fn run(app: &mut App, name: &str, input: Value) {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
}

/// The project's Mimari settings changed by `edit` and written back whole.
fn set_arch(app: &mut App, edit: impl FnOnce(&mut kentos_commands::ArchSettings)) {
    let mut arch = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .arch
        .clone()
        .unwrap_or_default();
    edit(&mut arch);
    run(
        app,
        "cad.settings.set",
        json!({ "arch": serde_json::to_value(&arch).expect("serialises") }),
    );
}

/// The house as an office: Büro, Zemin kat 4.80 m, 1. kat at +4.80 of 4.00 m; applied for on 2.7.2026.
fn office() -> App {
    let (mut app, _, _) = crate::dated_sizes_tests::house();
    run(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [{"id": "d1", "number": "1", "nature": "Büro"}]}}),
    );
    set_arch(&mut app, |a| {
        for l in &mut a.levels {
            if l.id == "k00" {
                l.height = Some(4.8);
            } else {
                l.elevation = 4.8;
                l.height = Some(4.0);
            }
        }
    });
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
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

/// The storeys' findings (code, state, message) and their rows (rule, version, outcome).
type Found = Vec<(String, Option<S>, String)>;
type Rows = Vec<(String, Option<String>, O)>;
fn storeys(app: &App) -> (Found, Rows) {
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
        .filter(|f| f.rule.as_deref().is_some_and(|r| r.contains("28/1")))
        .map(|f| (f.code.clone(), f.state, f.message.clone()))
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
        .filter(|r| r.rule.contains("28/1"))
        .map(|r| (r.rule.clone(), r.version.clone(), r.state))
        .collect();
    (found, rows)
}

#[test]
fn the_zone_chosen_in_the_settings_reads_the_storeys_by_its_article() {
    let mut app = office();
    checked(&mut app);
    let (found, rows) = storeys(&app);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(
        (found[0].0.as_str(), found[0].1),
        ("paiy_storey_height_over", Some(S::MissingData))
    );
    assert!(found[0].2.starts_with("Bölge yazılmamış"), "{found:#?}");
    assert_eq!(rows, [("PAİY 28/1".to_owned(), None, O::MissingData)]);
    // Ticaret alanı chosen in İmar durumu and saved.
    let _ = app.update(Message::Run("file.settings"));
    let _ = app.update(settings_message(SettingsEvent::Section(Section::Arch)));
    let _ = app.update(settings_message(SettingsEvent::ArchZone(Some(
        Zone::Commercial,
    ))));
    let _ = app.update(settings_message(SettingsEvent::Save));
    assert!(app.project.is_none(), "saved");
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(
        doc.settings().arch.as_ref().and_then(|a| a.zone),
        Some(Zone::Commercial)
    );
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    assert_eq!(facts.zone.as_deref(), Some("commercial"));
    checked(&mut app);
    let (found, rows) = storeys(&app);
    assert_eq!(
        found,
        [(
            "paiy_storey_height_over".to_owned(),
            None,
            "“Zemin kat” kat yüksekliği 4.80 m > 4.50 m (ticaret alanı, zemin kat) — PAİY 28/1-a (1.10.2017)".to_owned()
        )]
    );
    assert_eq!(
        rows,
        [(
            "PAİY 28/1-a".to_owned(),
            Some("1.10.2017".to_owned()),
            O::Failed
        )]
    );
    // Sivas over PAİY: its 5.00 m keeps the ground storey, PAİY's 4.50 m does not.
    set_arch(&mut app, |a| {
        a.local_rules = Some(kentos_commands::LocalRules::Sivas)
    });
    checked(&mut app);
    let (found, rows) = storeys(&app);
    assert_eq!(
        found,
        [(
            "regulation_conflict".to_owned(),
            None,
            "“Zemin kat” kat yüksekliği 4.80 m (ticaret alanı, zemin kat): Sivas 28/1-a en çok 5.00 m ister, uyuyor; \
             PAİY 28/1-a (1.10.2017) en çok 4.50 m ister, uymuyor — çelişkide idare karar verir"
                .to_owned()
        )]
    );
    assert_eq!(
        rows,
        [
            (
                "PAİY 28/1-a".to_owned(),
                Some("1.10.2017".to_owned()),
                O::Failed
            ),
            (
                "Sivas 28/1-a".to_owned(),
                Some("10.11.2022".to_owned()),
                O::Passed
            ),
        ]
    );
}

/// İmar durumu with Bölge Ticaret alanı chosen, and Denetim with the ground storey's finding, light and dark;
/// `.run/gui-evidence/bolge-8085/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/bolge-8085");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(1280.0, 800.0), "koyu"),
    ] {
        let mut app = office();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("file.settings"));
        let _ = app.update(settings_message(SettingsEvent::Section(Section::Arch)));
        let _ = app.update(settings_message(SettingsEvent::ArchZone(Some(
            Zone::Commercial,
        ))));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // Down the Mimari page to İmar durumu's Bölge: to its end, then up to the group.
        let at = iced::Point::new(size.width * 0.55, size.height * 0.5);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 4.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-bolge-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(settings_message(SettingsEvent::Save));
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(
            kentos_interaction::level_view::LevelView::Active,
        ));
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
            .position(|f| f.code == "paiy_storey_height_over");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-bolge-denetim-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}
