//! Çatının ölçüleri pakette (docs/adr/8159; PAİY 40/4, 40/5, 42/1, BYKHY 58/2, Sivas 40/5-e, 40/5-g, 42/5), checked
//! in Denetim with the standards package on the facade building (fixtures/arch/v1/facade-building.kcad): a hip roof
//! drawn from its walls with 1.20 m eaves at 75 %, 10 cm over its storey, no snow guard; a vent 0.90 m over the roof, a
//! chimney 0.50 m over the ridge; a 1.40 × 1.60 m window on the south face 0.66 m from the facade line and a
//! 0.78 × 1.18 m one 1.50 m east of it. PAİY alone says the eaves, the vent and the chimney (the ridge, 4.47 m over the
//! saçak seviyesi, is within its 5.00 m); Sivas's rules read over it the ridge (4.00 m), the slope, the snow guard and
//! the windows too.
//! Düzelt on the snow guard gives the roof one, on the chimney raises it to 0.80 m: their findings go.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib roof_rules_tests::screens -- --ignored --nocapture
//! ```

use serde_json::json;

use crate::app::{App, Message};

const ROOF_CODES: [&str; 8] = [
    "paiy_ridge_high",
    "paiy_eaves_wide",
    "paiy_roof_vent_high",
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
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// The roof's findings' messages, in Denetim's order.
fn said(app: &App) -> Vec<String> {
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
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
        let a = app.arch.arch_audit.audited.as_ref().expect("checked");
        a.findings.iter().position(|f| f.code == code).unwrap_or_else(|| panic!("{code}"))
    };
    let f = &app.arch.arch_audit.audited.as_ref().expect("checked").findings[at];
    assert_eq!(f.fix.as_ref().map(|x| x.step.command.as_str()), Some("cad.arch.edit"), "{f:?}");
    app.arch.arch_audit.selected = Some(at);
    let _ = app.update(Message::Arch(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Fix)));
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
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
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
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "bykhy_chimney_low");
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
