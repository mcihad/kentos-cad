//! Çatının ölçüleri pakette (docs/adr/8159; PAİY 40/4, 40/5, 42/1, BYKHY 58/2, Sivas 40/5-e, 40/5-g, 42/5), checked
//! in Denetim with the standards package on the facade building (fixtures/arch/v1/facade-building.kcad): a hip roof
//! drawn from its walls with 1.20 m eaves at 75 %, 10 cm over its storey, no snow guard; a vent 0.90 m over the roof, a
//! chimney 0.50 m over the ridge; a 1.40 × 1.60 m window on the south face 0.66 m from the facade line and a
//! 0.78 × 1.18 m one 1.50 m east of it. PAİY alone says the eaves, the vent and the chimney (the ridge, 4.47 m over the
//! saçak seviyesi, is within its 5.00 m); Sivas's rules read over it the ridge (4.00 m), the slope, the snow guard and
//! the windows too.
//! Düzelt on the snow guard gives the roof one, on the chimney raises it to 0.80 m: their findings go. The plan's roof
//! notes (docs/adr/8160) come before them: eaves of 1.20 m pass a note of 1.20 m, a note of 33 % fails the 75 % slope
//! and its Düzelt brings every sloping edge down to 33 %. On 8152's dwelling a roof over 1. kat reads the parcel
//! (42/2, docs/adr/8161) and the Balkon under its south eaves (42/3, docs/adr/8164).
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib roof_rules_tests::screens -- --ignored --nocapture
//! ```

use crate::arch_ext::*;
use serde_json::json;
use crate::arch::ArchState;

use crate::app::{App, Message};

const ROOF_CODES: [&str; 9] = [
    "paiy_ridge_high",
    "paiy_eaves_wide",
    "paiy_roof_vent_high",
    "paiy_roof_slope_steep",
    "bykhy_chimney_low",
    "local_roof_slope_steep",
    "local_snow_guard_missing",
    "local_roof_window_large",
    "local_roof_window_close",
];

/// The facade building with the roof over its 1. kat, applied for on 2.7.2026.
fn roofed() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot =
        kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/facade-building.kcad")).expect("decodes");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    let doc = &mut app.document.as_mut().expect("the building").model;
    let eaves = kentos_native_application::roofs::eaves_from_walls(
        doc,
        kentos_commands::Vec2 { x: 6.0, y: 4.5 },
        Some("k1"),
        1.2,
    )
    .expect("the walls enclose the point");
    let x0 = eaves.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let y0 = eaves.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let at = |x: f64, y: f64| json!({"x": x0 + x, "y": y0 + y});
    let slopes = vec![75.0; eaves.len()];
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        "cad.roofs.create",
        kentos_commands::CallOp::Execute,
        json!({"layerId": "0", "roofs": [{"pts": eaves, "slopes": slopes, "overhang": 1.2, "offset": 0.1,
            "arch": {"level": "k1"}, "parts": [
                {"kind": "vent", "at": at(10.0, 8.0), "width": 0.4, "depth": 0.4, "rise": 0.9},
                {"kind": "chimney", "at": at(7.0, 8.5), "width": 0.4, "depth": 0.4, "rise": 0.5},
                {"kind": "window", "at": at(4.0, 2.5), "width": 1.4, "depth": 1.6},
                {"kind": "window", "at": at(6.59, 3.5), "width": 0.78, "depth": 1.18}
            ]}]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    app
}

fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.modules.arch().arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The roof's findings' messages, in Denetim's order.
fn said(app: &App) -> Vec<String> {
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    audited.findings.iter().filter(|f| ROOF_CODES.contains(&f.code.as_str())).map(|f| f.message.clone()).collect()
}

fn sivas(app: &mut App) {
    let doc = &mut app.document.as_mut().expect("the building").model;
    let mut s = doc.settings().clone();
    s.arch.as_mut().expect("arch settings").local_rules = Some(kentos_commands::LocalRules::Sivas);
    doc.set_settings(s);
}

/// Düzelt on the first finding of `code`.
fn fix(app: &mut App, code: &str) {
    let at = {
        let a = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        a.findings.iter().position(|f| f.code == code).unwrap_or_else(|| panic!("{code}"))
    };
    let f = &app.modules.arch().arch_audit.audited.as_ref().expect("checked").findings[at];
    assert_eq!(f.fix.as_ref().map(|x| x.step.command.as_str()), Some("cad.arch.edit"), "{f:?}");
    app.modules.arch_mut().arch_audit.selected = Some(at);
    let _ = app.update(Message::from(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Fix)));
    app.arch_audit_standards_settle();
}

#[test]
fn the_roof_rules_come_from_the_standards_package_with_their_fixes() {
    let mut app = roofed();
    checked(&mut app);
    assert_eq!(
        said(&app),
        [
            "Saçak genişliği 1.20 m > 1.00 m — PAİY 42/1 (1.10.2017)",
            "Hava bacasının çatı örtüsünü aşan yüksekliği 0.90 m > 0.60 m — PAİY 40/5 (1.10.2017)",
            "Duman bacasının mahyayı aşan yüksekliği 0.50 m < 0.80 m — BYKHY 58/2",
        ]
    );
    sivas(&mut app);
    checked(&mut app);
    assert_eq!(
        said(&app),
        [
            "Mahya yüksekliği (saçak seviyesinden) 4.47 m > 4.00 m — Sivas 40/5-g; PAİY 40/4 (1.10.2017): 5.00 m",
            "Saçak genişliği 1.20 m > 1.00 m — PAİY 42/1 (1.10.2017)",
            "Hava bacasının çatı örtüsünü aşan yüksekliği 0.90 m > 0.60 m — PAİY 40/5 (1.10.2017)",
            "Duman bacasının mahyayı aşan yüksekliği 0.50 m < 0.80 m — BYKHY 58/2",
            "Çatının eğimi %75 > %40 — Sivas 40/5-g",
            "Eğimli çatının saçağında kar siperliği yok — Sivas 42/5",
            "Çatı penceresi 1.40 × 1.60 m, en çok 1.20 × 1.50 m (en/boy) — Sivas 40/5-e-2",
            "Çatı penceresinin bina cephe hattına uzaklığı 0.66 m < 1.00 m — Sivas 40/5-e-3",
            "İki çatı penceresi arası 1.50 m < 2.00 m — Sivas 40/5-e-3",
        ]
    );
    let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
    let guard = audited.findings.iter().find(|f| f.code == "local_snow_guard_missing").expect("the snow guard");
    assert_eq!(guard.rule.as_deref(), Some("Sivas İmar Yön. 42/5 · RG 10.11.2022/32009"));
    // Düzelt: the snow guard given, the chimney raised; the other parts as drawn.
    fix(&mut app, "local_snow_guard_missing");
    assert!(!said(&app).iter().any(|m| m.contains("kar siperliği")), "{:?}", said(&app));
    fix(&mut app, "bykhy_chimney_low");
    assert!(!said(&app).iter().any(|m| m.contains("Duman bacası")), "{:?}", said(&app));
    let doc = &app.document.as_ref().expect("the building").model;
    let roof = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Roof(r) => Some(r.roof.clone()),
            _ => None,
        })
        .expect("the roof");
    assert_eq!(roof.snow_guard, Some(kentos_commands::DEFAULT_SNOW_GUARD));
    let rises: Vec<Option<f64>> = roof.parts.iter().map(|p| p.rise).collect();
    assert_eq!(rises, [Some(0.9), Some(0.8), None, None]);
    assert_eq!((roof.parts[2].width, roof.parts[3].depth), (1.4, 1.18));
}

/// The plan's roof notes typed in Plan notları (docs/adr/8160): `notes` as (measure, value), then Kaydet.
fn plan_notes(app: &mut App, notes: &[(kentos_commands::PlanNoteKey, &str)]) {
    use crate::project::plan_notes::Edit;
    use crate::project::settings::Section;
    use crate::project::{SettingsEvent, settings_message};
    let _ = app.update(Message::Run("file.settings"));
    let _ = app.update(settings_message(SettingsEvent::Section(Section::Arch)));
    for (i, (key, value)) in notes.iter().enumerate() {
        let _ = app.update(settings_message(SettingsEvent::AddPlanNote));
        for e in [Edit::Key(*key), Edit::Value((*value).into()), Edit::Source("UİP 12.3.2019, plan notu 12".into())] {
            let _ = app.update(settings_message(SettingsEvent::ArchPlanNote(i, e)));
        }
    }
    let _ = app.update(settings_message(SettingsEvent::Save));
    assert!(app.project.is_none(), "saved");
}

#[test]
fn the_plan_s_roof_notes_come_before_the_regulations() {
    use kentos_commands::PlanNoteKey;
    let mut app = roofed();
    // The plan lets the eaves be 1.20 m and asks the roof's slope at most 33 % (40 and 42 may be changed by a plan).
    plan_notes(&mut app, &[(PlanNoteKey::EavesMost, "1,20"), (PlanNoteKey::RoofSlopeMost, "33")]);
    checked(&mut app);
    let found = said(&app);
    assert!(!found.iter().any(|m| m.starts_with("Saçak genişliği")), "{found:?}");
    assert!(
        found.contains(&"Çatının eğimi %75 > %33 — Plan notu (UİP 12.3.2019, plan notu 12)".to_owned()),
        "{found:?}"
    );
    let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
    let row = |rule: &str| set.rules.iter().find(|r| r.rule == rule).map(|r| r.state);
    use kentos_commands::ArchAuditOutcome as O;
    assert_eq!(row("Plan notu: saçak genişliği"), Some(O::Passed));
    assert_eq!(row("Plan notu: çatı eğimi"), Some(O::Failed));
    assert_eq!(row("PAİY 42/1"), Some(O::OutOfScope));
    // Düzelt: every sloping edge down to the plan's 33 %.
    fix(&mut app, "paiy_roof_slope_steep");
    assert!(!said(&app).iter().any(|m| m.starts_with("Çatının eğimi")), "{:?}", said(&app));
    let doc = &app.document.as_ref().expect("the building").model;
    let slopes = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Roof(r) => Some(r.roof.slopes.clone()),
            _ => None,
        })
        .expect("the roof");
    assert!(slopes.iter().all(|s| *s == Some(33.0)), "{slopes:?}");
}

/// PAİY 42/2 (docs/adr/8161): on 8152's dwelling (the parcel x −3–13, y −5–11), a roof over 1. kat whose eaves run
/// from x −4: 1.00 m past the parcel's west side, "Saçaklar parsel sınırlarını aşamaz" fails; from x −2 it holds.
#[test]
fn the_eaves_stay_within_the_parcel() {
    use kentos_commands::ArchAuditOutcome as O;
    for (west, outcome) in [(-4.0, O::Failed), (-2.0, O::Passed)] {
        let (mut app, _) = crate::projection_tests::scene(1.2);
        crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
        let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
        let p = |x: f64, y: f64| json!({"x": x, "y": y});
        crate::use_class_tests::run(
            &mut app,
            "cad.roofs.create",
            json!({"layerId": layer, "roofs": [{"pts": [p(west, -1.0), p(11.0, -1.0), p(11.0, 9.0), p(west, 9.0)],
                "slopes": [33.0, 33.0, 33.0, 33.0], "arch": {"level": "k01"}}]}),
        );
        checked(&mut app);
        let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
        let row = set.rules.iter().find(|r| r.rule == "PAİY 42/2").map(|r| r.state);
        assert_eq!(row, Some(outcome), "{:#?}", set.rules);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        let over: Vec<&str> = audited
            .findings
            .iter()
            .filter(|f| f.code == "paiy_eaves_over_parcel")
            .map(|f| f.message.as_str())
            .collect();
        match outcome {
            O::Failed => assert_eq!(over, ["Saçak parsel sınırını 1.00 m aşıyor — PAİY 42/2 (1.10.2017)"]),
            _ => assert!(over.is_empty(), "{over:?}"),
        }
    }
}

/// Denetim's roof findings with Sivas read, the chimney's selected, light and dark;
/// `.run/gui-evidence/cati-olculeri-8159/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cati-olculeri-8159");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sizes = [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")];
    for (theme, size, name) in sizes {
        let mut app = roofed();
        sivas(&mut app);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::ActiveLevel(Some("k1".into())));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_chimney_low");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-cati-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Plan notları with the plan's three roof notes, light and dark; `.run/gui-evidence/cati-notlari-8160/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn notes_screens() {
    use crate::project::plan_notes::Edit;
    use crate::project::settings::Section;
    use crate::project::{SettingsEvent, settings_message};
    use iced::Size;
    use kentos_commands::PlanNoteKey;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cati-notlari-8160");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sizes = [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")];
    for (theme, size, name) in sizes {
        let mut app = roofed();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("file.settings"));
        let _ = app.update(settings_message(SettingsEvent::Section(Section::Arch)));
        let notes =
            [(PlanNoteKey::RidgeMost, "3,50"), (PlanNoteKey::RoofSlopeMost, "33"), (PlanNoteKey::EavesMost, "1,20")];
        for (i, (key, value)) in notes.into_iter().enumerate() {
            let _ = app.update(settings_message(SettingsEvent::AddPlanNote));
            for e in [Edit::Key(key), Edit::Value(value.into()), Edit::Source("UİP 12.3.2019, plan notu 12".into())] {
                let _ = app.update(settings_message(SettingsEvent::ArchPlanNote(i, e)));
            }
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
        let file = out.join(format!("1-cati-notlari-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// Denetim's 42/2 finding beside 1. kat's plan (the parcel and the roof past its west side), light and dark;
/// `.run/gui-evidence/sacak-parsel-8161/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn parcel_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/sacak-parsel-8161");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sizes = [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")];
    for (theme, size, name) in sizes {
        let (mut app, _) = crate::projection_tests::scene(1.2);
        crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
        let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
        let p = |x: f64, y: f64| json!({"x": x, "y": y});
        crate::use_class_tests::run(
            &mut app,
            "cad.roofs.create",
            json!({"layerId": layer, "roofs": [{"pts": [p(-4.0, -1.0), p(11.0, -1.0), p(11.0, 9.0), p(-4.0, 9.0)],
                "slopes": [33.0, 33.0, 33.0, 33.0], "arch": {"level": "k01"}}]}),
        );
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_eaves_over_parcel");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-sacak-parsel-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// 8152's dwelling with a roof over 1. kat whose south eaves run along y = `south`: over the 1.20 m deep Balkon
/// (y −1.20 to 0, x 2–8) on 1. kat.
fn over_the_balcony(south: f64) -> App {
    let (mut app, _) = crate::projection_tests::scene(1.2);
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    crate::use_class_tests::run(
        &mut app,
        "cad.roofs.create",
        json!({"layerId": layer, "roofs": [{"pts": [p(-1.0, south), p(11.0, south), p(11.0, 9.0), p(-1.0, 9.0)],
            "slopes": [33.0, 33.0, 33.0, 33.0], "arch": {"level": "k01"}}]}),
    );
    app
}

/// PAİY 42/3 (docs/adr/8164): "Binada çıkma yapılması halinde saçak genişliği çıkmadan itibaren 0.50 metreyi aşamaz."
/// On 8152's dwelling the roof over 1. kat from y −2.00 reaches 0.80 m past the Balkon's front: 42/3 fails; from
/// y −1.60, 0.40 m: it holds. The Balkon's sides are not its front: the eaves west and east of it are the main
/// facade's.
#[test]
fn the_eaves_stay_within_half_a_metre_of_a_projection() {
    use kentos_commands::ArchAuditOutcome as O;
    for (south, outcome, past) in [(-2.0, O::Failed, 0.8), (-1.6, O::Passed, 0.4)] {
        let mut app = over_the_balcony(south);
        let doc = &app.document.as_ref().expect("a drawing").model;
        let measured: Vec<Option<f64>> =
            kentos_native_application::roof_audit::measures(doc).iter().map(|m| m.over_projection).collect();
        assert_eq!(measured.len(), 1);
        assert!(measured[0].is_some_and(|v| (v - past).abs() < 1e-6), "{measured:?}");
        checked(&mut app);
        let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
        let row = set.rules.iter().find(|r| r.rule == "PAİY 42/3").map(|r| r.state);
        assert_eq!(row, Some(outcome), "{:#?}", set.rules);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        let said: Vec<&str> = audited
            .findings
            .iter()
            .filter(|f| f.code == "paiy_eaves_past_projection")
            .map(|f| f.message.as_str())
            .collect();
        match outcome {
            O::Failed => {
                assert_eq!(said, ["Saçağın çıkmadan genişliği 0.80 m > 0.50 m — PAİY 42/3 (1.10.2017)"]);
            }
            _ => assert!(said.is_empty(), "{said:?}"),
        }
    }
}

/// Denetim's 42/3 finding beside 1. kat's plan (the Balkon under the roof's south eaves), light and dark;
/// `.run/gui-evidence/cikma-sacak-8164/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn projection_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cikma-sacak-8164");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sizes = [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")];
    for (theme, size, name) in sizes {
        let mut app = over_the_balcony(-2.0);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_eaves_past_projection");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-cikma-sacak-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// A two-storey dwelling (Zemin kat ±0.00, 1. kat +3.00, each 3.00 m) under a hip roof over 1. kat (x 0–10, y 0–8, 50 %
/// on every edge, its eaves at 1. kat's top, +6.00) and a stair housing's room on 1. kat (Merdiven evi, x 6–8, y 5–6.5,
/// away from the slope arrows) whose south wall (x 6–8 along y 5, 20 cm) is `height` m: its foot +3.00.
fn stair_housing(height: f64) -> App {
    use crate::use_class_tests::{building, p, rect, run, spaces};
    let mut app = building(2, "Konut");
    let room = rect(6.0, 5.0, 8.0, 6.5);
    spaces(&mut app, vec![json!({"parts": room, "name": "Merdiven evi", "arch": {"level": "k01"}})]);
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    run(
        &mut app,
        "cad.roofs.create",
        json!({"layerId": layer, "roofs": [{"pts": [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 8.0), p(0.0, 8.0)],
            "slopes": [50.0, 50.0, 50.0, 50.0], "arch": {"level": "k01"}}]}),
    );
    run(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [{"pts": [p(6.0, 5.0), p(8.0, 5.0)], "width": 0.2, "height": height,
            "arch": {"level": "k01"}}]}),
    );
    app
}

/// PAİY 40/5 (docs/adr/8165): the stair housing's wall 5.00 m tall tops out at +8.00; the covering under its east end
/// (2.00 m in from the east eaves at 50 %) is +7.00: 1.00 m over it, past 0.60 m: 40/5 fails. 4.50 m tall, 0.50 m
/// over: it holds. With an elevator in the building the 1.00 m is an expert's to review: the stair housing arranged
/// with the elevator tower may pass the covering in its least sizes.
#[test]
fn a_stair_housing_passes_the_roof_covering_by_at_most_0_60_m() {
    use kentos_commands::{ArchAuditOutcome as O, ArchAuditState};
    let row = |app: &App| {
        let set = app.modules.arch().arch_audit.rule_set.as_ref().expect("the rule set");
        set.rules.iter().find(|r| r.rule == "PAİY 40/5").map(|r| r.state)
    };
    let found = |app: &App| -> Vec<(String, Option<ArchAuditState>)> {
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        audited
            .findings
            .iter()
            .filter(|f| f.code == "paiy_roof_wall_high")
            .map(|f| (f.message.clone(), f.state))
            .collect()
    };
    use kentos_native_application::roof_walls::WallOverKind;
    let mut app = stair_housing(5.0);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let walls: Vec<(WallOverKind, f64)> = kentos_native_application::roof_audit::measures(doc)
        .into_iter()
        .flat_map(|m| m.walls)
        .map(|w| (w.kind, w.over))
        .collect();
    assert_eq!(walls.len(), 1, "{walls:?}");
    assert_eq!(walls[0].0, WallOverKind::Stair);
    assert!((walls[0].1 - 1.0).abs() < 1e-6, "{walls:?}");
    checked(&mut app);
    assert_eq!(row(&app), Some(O::Failed));
    let message = "Merdiven evi duvarının çatı örtüsünü aşan yüksekliği 1.00 m > 0.60 m — PAİY 40/5 (1.10.2017)";
    assert_eq!(found(&app), [(message.to_owned(), None)]);

    let mut low = stair_housing(4.5);
    checked(&mut low);
    assert_eq!(row(&low), Some(O::Passed));
    assert!(found(&low).is_empty());

    // An elevator's shaft on 1. kat: the building has an elevator.
    let shaft = crate::use_class_tests::rect(1.5, 2.0, 3.0, 4.0);
    let elevator = json!({"parts": shaft, "name": "Asansör", "arch": {"level": "k01"}});
    crate::use_class_tests::spaces(&mut app, vec![elevator]);
    checked(&mut app);
    assert_eq!(row(&app), Some(O::ExpertReview));
    let said = found(&app);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].0.starts_with(message) && said[0].0.contains("asansörlü binada"), "{said:?}");
    assert_eq!(said[0].1, Some(ArchAuditState::ExpertReview));
}

/// Denetim's 40/5 finding on the stair housing's wall, light and dark; `.run/gui-evidence/cati-duvar-8165/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn wall_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cati-duvar-8165");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let sizes = [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")];
    for (theme, size, name) in sizes {
        let mut app = stair_housing(5.0);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::ActiveLevel(Some("k01".into())));
        checked(&mut app);
        let audited = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        app.modules.arch_mut().arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_roof_wall_high");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-cati-duvar-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
