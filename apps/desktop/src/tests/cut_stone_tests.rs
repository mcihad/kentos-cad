//! Taşıyıcı sistem (docs/adr/8094): the building's structural system, chosen in Proje ayarları › Mimari. The two-storey
//! house of `dated_sizes_tests` (10 × 5 m outside, 25 cm walls; Zemin kat a Taşlık of the flat and its WC, 1. kat a
//! bedroom), each storey 4.40 m floor to floor, applied for on 2.7.2026, on a 200 m² parcel of emsal 1.00. Not declared,
//! PAİY 28/1-c's 12.8.2023 text asks 4.00 m of a building of up to three storeys (both fail) and the Taşlık counts in.
//! Declared a cut stone masonry building with traditional elements, 28/1-d's 4.50 m holds both storeys and the Taşlık's
//! 40 m² is out (22/1-m, within the 30 %): katlar alanı 100 → 60 m². Its garden's reach is not in the drawing, so Denetim
//! leaves it to an expert.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib cut_stone_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{ArchAuditOutcome as O, ArchAuditState as S, BuildingStructure, StructuralSystem};
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
    let mut arch = app.document.as_ref().expect("a drawing").model.settings().arch.clone().unwrap_or_default();
    edit(&mut arch);
    run(app, "cad.settings.set", json!({ "arch": serde_json::to_value(&arch).expect("serialises") }));
}

/// The persistent id of the room named `name`.
fn room(app: &App, name: &str) -> String {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities_with_uids()
        .find_map(|(uid, e)| match e {
            kentos_commands::Entity::Space(s) if s.space.name.as_deref() == Some(name) => Some(uid.to_string()),
            _ => None,
        })
        .expect("the room")
}

/// The house with its Hol a Taşlık, both storeys 4.40 m, on a 200 m² parcel of emsal 1.00, applied for on 2.7.2026.
fn stone_house() -> App {
    let (mut app, _, _) = crate::dated_sizes_tests::house();
    let hol = room(&app, "Hol");
    run(&mut app, "cad.spaces.edit", json!({"uids": [hol], "operation": {"kind": "set", "name": "Taşlık"}}));
    set_arch(&mut app, |a| {
        for l in &mut a.levels {
            if l.id == "k01" {
                l.elevation = 4.4;
            }
            l.height = Some(4.4);
        }
        a.zoning = Some(kentos_commands::Zoning { parcel_area: 200.0, far: Some(1.0), coverage: None, ..Default::default() });
    });
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    app
}

/// Taşıyıcı sistem chosen in the settings (and its traditional elements switched), then saved.
fn declare(app: &mut App, system: Option<StructuralSystem>, traditional: bool) {
    let _ = app.update(Message::Run("file.settings"));
    let _ = app.update(settings_message(SettingsEvent::Section(Section::Arch)));
    let _ = app.update(settings_message(SettingsEvent::ArchStructure(system)));
    let _ = app.update(settings_message(SettingsEvent::ArchTraditional(traditional)));
    let _ = app.update(settings_message(SettingsEvent::Save));
    assert!(app.project.is_none(), "saved");
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The findings and rows of the articles whose rule says `article` (code, state, message; rule, version, outcome).
type Found = Vec<(String, Option<S>, String)>;
type Rows = Vec<(String, Option<String>, O)>;
fn of(app: &App, article: &str) -> (Found, Rows) {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let found = audited
        .findings
        .iter()
        .filter(|f| f.rule.as_deref().is_some_and(|r| r.contains(article)))
        .map(|f| (f.code.clone(), f.state, f.message.clone()))
        .collect();
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    let rows = set
        .rules
        .iter()
        .filter(|r| r.rule.contains(article))
        .map(|r| (r.rule.clone(), r.version.clone(), r.state))
        .collect();
    (found, rows)
}

/// Katlar alanı counted in and out (m²): the table's included, exempt and floors.
fn counted(app: &App) -> (f64, f64, f64) {
    let r = app.document.as_ref().expect("a drawing").model.zoning_report().expect("a zoning");
    let mm = |x: f64| (x * 1e4).round() / 1e4;
    (mm(r.included), mm(r.exempt), mm(r.floors))
}

#[test]
fn a_traditional_cut_stone_house_reads_its_storeys_by_28_1_d_and_lets_its_taslik_out() {
    let mut app = stone_house();
    // Not declared: 28/1-c's 4.00 m (12.8.2023: up to three storeys but the basements), the Taşlık counted in.
    checked(&mut app);
    let (found, rows) = of(&app, "28/1");
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found.iter().all(|f| f.2.contains("4.40 m > 4.00 m (konut bölgesi, 2 kat) — PAİY 28/1-c (12.8.2023)")));
    assert_eq!(rows, [("PAİY 28/1-c".to_owned(), Some("12.8.2023".to_owned()), O::Failed)]);
    assert_eq!(of(&app, "22/1-m"), (Vec::new(), Vec::new()), "no row for a building not declared");
    // The flat's three rooms: 40 + 10 on Zemin kat (the 10 cm partition's façade corners halved), 50 upstairs.
    assert_eq!(counted(&app), (100.0, 0.0, 100.0));

    // Kesme taş yığma, with traditional elements, chosen and saved.
    declare(&mut app, Some(StructuralSystem::CutStoneMasonry), true);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(
        doc.settings().arch.as_ref().and_then(|a| a.structure),
        Some(BuildingStructure { system: StructuralSystem::CutStoneMasonry, traditional: true })
    );
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    assert_eq!((facts.structural_system.as_deref(), facts.traditional), (Some("cutStoneMasonry"), true));
    let taslik = room(&app, "Taşlık");
    let traditional: Vec<(&[String], f64)> = facts
        .limits
        .iter()
        .filter(|l| l.limit == "traditional")
        .map(|l| (l.rooms.as_slice(), l.area))
        .collect();
    assert_eq!(traditional, [(std::slice::from_ref(&taslik), 40.0)]);
    // The Taşlık's 40 m² out within the 30 % (60 m² of the parcel's 200 m² at emsal 1.00).
    assert_eq!(counted(&app), (60.0, 40.0, 60.0));
    checked(&mut app);
    let (found, rows) = of(&app, "28/1");
    assert_eq!(found, Vec::new());
    assert_eq!(rows, [("PAİY 28/1-d".to_owned(), Some("9.10.2021".to_owned()), O::Passed)]);
    let (found, rows) = of(&app, "22/1-m");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!((found[0].0.as_str(), found[0].1), ("paiy_traditional_not_exempt", Some(S::ExpertReview)));
    assert!(found[0].2.contains("40.00 m²; zemin dahil 2 katlı geleneksel kesme taş yığma bina"), "{found:#?}");
    assert_eq!(rows, [("PAİY 22/1-m".to_owned(), Some("31.12.2022".to_owned()), O::ExpertReview)]);

    // Betonarme chosen: the traditional elements go with the masonry, the Taşlık counts in again.
    declare(&mut app, Some(StructuralSystem::ReinforcedConcrete), true);
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(
        doc.settings().arch.as_ref().and_then(|a| a.structure),
        Some(BuildingStructure { system: StructuralSystem::ReinforcedConcrete, traditional: false })
    );
    assert_eq!(counted(&app), (100.0, 0.0, 100.0));
}

/// Proje ayarları › Mimari's Taşıyıcı sistem with Kesme taş yığma and its traditional elements, Emsal ve TAKS tablosu
/// with the Taşlık out, and Denetim with 22/1-m's finding and the rule set's rows, light and dark;
/// `.run/gui-evidence/kesme-tas-8094/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kesme-tas-8094");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut app = stone_house();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("file.settings"));
        let _ = app.update(settings_message(SettingsEvent::Section(Section::Arch)));
        let _ = app.update(settings_message(SettingsEvent::ArchStructure(Some(StructuralSystem::CutStoneMasonry))));
        let _ = app.update(settings_message(SettingsEvent::ArchTraditional(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // Down the Mimari page to Taşıyıcı sistem: to its end, then up past the room lists.
        let at = iced::Point::new(size.width * 0.55, size.height * 0.5);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -60.0));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 2.0));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-tasiyici-sistem-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(settings_message(SettingsEvent::Save));
        app.toasts.clear();
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));

        // Emsal ve TAKS tablosu placed beside the house.
        let _ = app.update(Message::Run("arch.zoningTable"));
        let _ = app.submit_line("12,5");
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-emsal-tablosu-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());

        // Denetim: 22/1-m's finding chosen, the rule set over the list.
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_traditional_not_exempt");
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("3-taslik-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
