//! Emsalin 22. madde sınırları (docs/adr/8079; plan/bim-plan.md BIM-13) on zones_tests' 15 × 6 m plan built on two
//! storeys: a basement (−3.00, 3.00 m high: its top at ±0.00, under the ground) and the ground storey. Each bay is 5 × 6
//! m: its regulation gross 30 m², its net 4.675 × 5.5 = 25.7125 m² at the ends (paiy_area_tests checks them by hand).
//! On the ground: flat 1 (west), the stair hall (middle, named Merdiven, the common area's), flat 2 (east); in the
//! basement the bays each test names. The parcel 400 m², emsal 1.50: the 30 % is 180 m², never reached here.
//!
//! Worked by hand from Planlı Alanlar İmar Yönetmeliği 22/1 (RG 25.7.2019/30842, 23.1.2021/31373, 11.7.2021/31538):
//! - 22/1-l: the stair hall, "kat ve ara sahanlıkları dâhil açık veya kapalı merdiven evi", out within the 30 %.
//! - 22/1-k: flat 1's storage annex under the ground, 30 m², out up to 0.50 × 25.7125 = 12.85625 m²; 17.14375 m² in.
//! - 22/1-ç: a caretaker flat of three 30 m² rooms, 70 m² of its 90 out, 20 m² in.
//! - 22/1-f: two library rooms, 50 m² of their 60 out, 10 m² in.

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

fn command(app: &mut App, name: &str, input: Value) {
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

/// The two-storey plan: the basement's floor at `floor`, 3.00 m high, the ground storey's floor at its top (a raised
/// ground storey over a basement half out of the ground when `floor` is over −3.00); the basement's three bays named
/// and grouped as `basement` says (`e1` flat 1's storage annex, `o` the common area, `k` the caretaker flat), the ground
/// storey's flat 1, stair hall and flat 2; the parcel's zoning (400 m², emsal 1.50, TAKS 0.40).
fn building(basement: [(&str, &str); 3], floor: f64) -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(Some(Module::Architecture), None, Settings::memory(), Recovery::off());
    app.db_store = Box::leak(Box::new(kentos_postgis::connections::ConnectionStore::open(None)));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    let mut arch = app.document.as_ref().expect("a drawing").model.settings().arch.clone().expect("arch");
    let level = |id: &str, name: &str, z: f64| kentos_commands::ArchLevel {
        id: id.into(),
        name: name.into(),
        elevation: z,
        height: Some(3.0),
    };
    arch.levels = vec![level("b01", "Bodrum kat", floor), level("k00", "Zemin kat", floor + 3.0)];
    arch.zoning = Some(kentos_commands::Zoning { parcel_area: 400.0, far: Some(1.5), coverage: Some(0.4), ..Default::default() });
    let mut settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    settings.arch = Some(arch);
    app.document.as_mut().expect("a drawing").model.set_settings(settings);
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    for level in ["b01", "k00"] {
        let wall = |a: (f64, f64), b: (f64, f64), w: f64| {
            json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": w, "arch": {"level": level}})
        };
        command(&mut app, "cad.walls.create", json!({"layerId": layer, "walls": [
            wall((0.125, 0.125), (14.875, 0.125), 0.25), wall((14.875, 0.125), (14.875, 5.875), 0.25),
            wall((14.875, 5.875), (0.125, 5.875), 0.25), wall((0.125, 5.875), (0.125, 0.125), 0.25),
            wall((5.0, 0.125), (5.0, 5.875), 0.15), wall((10.0, 0.125), (10.0, 5.875), 0.15)
        ]}));
    }
    command(&mut app, "cad.program.edit", json!({"program": {"units": [
        {"id": "d1", "number": "1", "nature": "Daire"},
        {"id": "d2", "number": "2", "nature": "Daire"},
        {"id": "e1", "number": "E1", "kind": "annex", "annexOf": "d1", "nature": "Depo"},
        {"id": "o", "number": "O", "name": "Ortak alan", "kind": "common"},
        {"id": "k", "number": "K", "name": "Kapıcı dairesi", "kind": "common"}
    ]}}));
    let room = |x: f64, level: &str, unit: &str, name: Option<&str>| {
        let mut r = json!({"seed": p(x, 3.0), "arch": {"level": level}, "unit": unit});
        if let Some(n) = name {
            r["name"] = json!(n);
        }
        r
    };
    let mut spaces = vec![room(2.5, "k00", "d1", None), room(7.5, "k00", "o", Some("Merdiven")), room(12.5, "k00", "d2", None)];
    for (x, (name, unit)) in [2.5, 7.5, 12.5].into_iter().zip(basement) {
        spaces.push(room(x, "b01", unit, Some(name)));
    }
    command(&mut app, "cad.spaces.create", json!({"layerId": layer, "spaces": spaces}));
    app
}

/// Flat 1's storage annex, the basement's stair and the common boiler room.
fn storage(floor: f64) -> App {
    building([("Depo", "e1"), ("Merdiven", "o"), ("Kazan dairesi", "o")], floor)
}

/// The caretaker flat's three rooms in the basement, by their own names.
fn caretaker() -> App {
    building([("Salon", "k"), ("Yatak odası", "k"), ("Mutfak", "k")], -3.0)
}

/// Two library rooms of the common area and the boiler room.
fn library() -> App {
    building([("Kütüphane", "o"), ("Kütüphane", "o"), ("Kazan dairesi", "o")], -3.0)
}

fn report(app: &App) -> kentos_domain::zoning::ZoningReport {
    app.document.as_ref().expect("a drawing").model.zoning_report().expect("a zoning")
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// Emsal ve TAKS tablosu's row starting with `words`: its drawing's value.
fn row(app: &App, words: &str) -> Option<String> {
    let t = app.document.as_ref().expect("a drawing").model.zoning_table();
    t.rows.iter().find(|r| r[0].starts_with(words)).map(|r| r[2].clone())
}

#[test]
fn a_flat_s_storage_annex_under_the_ground_is_out_up_to_half_its_net_area_and_the_stair_hall_is_out() {
    let app = storage(-3.0);
    let r = report(&app);
    assert!(near(r.storage_over, 30.0 - 0.5 * 25.7125), "{r:?}");
    // Katlar alanı: the two flats and the annex's part past the 50 %; out: the annex's rest, two stair halls, the boiler.
    assert!(near(r.included, 60.0 + 17.14375), "{r:?}");
    assert!(near(r.exempt, 12.85625 + 30.0 + 30.0 + 30.0), "{r:?}");
    assert!(near(r.floors, 77.14375), "{r:?}");
    assert_eq!(row(&app, "Depo eklentilerinin %50'yi aşan kısmı").as_deref(), Some("17.14"));
    let limits: Vec<_> = r.limits.iter().map(|l| (l.limit, l.unit.as_deref(), l.rooms.len())).collect();
    assert_eq!(limits, [(kentos_domain::zoning::ZoningLimit::Storage, Some("d1"), 1)]);
    // The basement's top a metre over ±0.00 (the ground storey raised on it): not under the ground, the store counts
    // in whole.
    let raised = storage(-2.0);
    let r = report(&raised);
    assert!(near(r.storage_over, 0.0) && r.limits.is_empty(), "{r:?}");
    assert!(near(r.included, 90.0), "the store in: {r:?}");
    assert!(near(r.exempt, 90.0), "two stair halls and the boiler: {r:?}");
    assert_eq!(row(&raised, "Depo eklentilerinin"), None);
    // A coal room in flat 1's storage annex is a coal room (22/1-i, its own name), not a store under the 50 %.
    let coal = building([("Kömürlük", "e1"), ("Merdiven", "o"), ("Kazan dairesi", "o")], -3.0);
    let r = report(&coal);
    assert!(r.limits.is_empty() && near(r.storage_over, 0.0), "{r:?}");
    assert!(near(r.included, 60.0) && near(r.exempt, 120.0), "{r:?}");
}

#[test]
fn the_caretaker_flat_s_rooms_are_out_up_to_70_m2_together() {
    let app = caretaker();
    let r = report(&app);
    assert!(near(r.caretaker_over, 20.0), "{r:?}");
    assert!(near(r.included, 60.0 + 20.0), "{r:?}");
    assert!(near(r.exempt, 70.0 + 30.0), "70 m² of the flat and the stair hall: {r:?}");
    assert_eq!(row(&app, "Kapıcı dairelerinin 70 m²'yi aşan kısmı").as_deref(), Some("20.00"));
    // The excess shared by the rooms' areas: a third each, not all on the last room.
    let moved: Vec<f64> = r.rooms.iter().filter(|x| x.moved > 0.0).map(|x| x.moved).collect();
    assert_eq!(moved.len(), 3, "{:?}", r.rooms);
    assert!(moved.iter().all(|m| near(*m, 20.0 / 3.0)), "{moved:?}");
}

#[test]
fn the_library_rooms_are_out_up_to_50_m2_together() {
    let app = library();
    let r = report(&app);
    assert!(near(r.library_over, 10.0), "{r:?}");
    assert!(near(r.included, 60.0 + 10.0), "{r:?}");
    assert_eq!(row(&app, "Kütüphanenin 50 m²'yi aşan kısmı").as_deref(), Some("10.00"));
}

/// A polygon's area by the shoelace formula, its rings even-odd.
fn area(rings: &[Vec<[f64; 2]>]) -> f64 {
    let signed = |r: &[[f64; 2]]| {
        let n = r.len();
        (0..n).map(|i| r[i][0] * r[(i + 1) % n][1] - r[(i + 1) % n][0] * r[i][1]).sum::<f64>() / 2.0
    };
    rings.iter().map(|r| signed(r)).sum::<f64>().abs()
}

/// The model's emsal zones (EK-6 Tablo 6.60): the basement holds no room counted in, so its counted-out zone names the
/// annex's part past the 50 % as counted in. The ground's counted-in zone is the two flats' shapes: each contour reaches
/// the interior wall's far face along the façade wall's outer half at both ends, and the corner it shares with the stair
/// hall is split at the wall's axis (docs/adr/8097), so the zone is the flats' 60 m² gross; it and the named part make
/// the table's Emsale konu alan (60 + 17.14375).
#[test]
fn a_storey_with_no_room_counted_in_names_the_part_past_the_limit_in_its_zone_counted_out() {
    let app = storage(-3.0);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let (b, _, _) = kentos_native_application::arch_ifc::model(doc);
    let state = |z: &kentos_ifc::building::Zone| {
        z.psets.iter().flat_map(|p| p.props.iter()).find_map(|(k, v)| match (k.as_str(), v) {
            ("EmsalDurumu", kentos_ifc::building::Value::Label(s)) => Some(s.clone()),
            _ => None,
        })
    };
    let dahil: Vec<_> = b.zones.iter().filter(|z| state(z).as_deref() == Some("DAHIL")).collect();
    assert_eq!(dahil.len(), 1, "the ground storey's: {:?}", b.zones);
    assert!(near(area(&dahil[0].rings), 60.0), "the two flats: {} {:?}", area(&dahil[0].rings), dahil[0].rings);
    let named: Vec<&str> = b
        .zones
        .iter()
        .filter(|z| state(z).as_deref() == Some("HARIC"))
        .filter_map(|z| z.description.as_deref())
        .filter(|d| d.contains("22/1-k"))
        .collect();
    assert_eq!(named.len(), 1, "{:?}", b.zones);
    assert!(named[0].contains("17.14 m² depo eklentisi emsale dahil"), "{}", named[0]);
    assert!(near(area(&dahil[0].rings) + 17.14375, report(&app).floors));
}

/// The whole drawing checked in Denetim, the standards packages asked and answered.
fn checked(app: &mut App) {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    assert_eq!(app.arch.arch_audit.regulation_failed, None, "the national regulation's rule answers");
}

/// Two flats with a caretaker flat counted out: PAİY 44/1 makes none obligatory under 40 dwellings and 60 units, so
/// the standards package says 22/1-ç's flat may not be counted out; the rule set's rows say each article of 22/1.
#[test]
fn a_caretaker_flat_no_rule_makes_obligatory_is_a_finding_of_the_standards_package() {
    let mut app = caretaker();
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    checked(&mut app);
    let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
    let f = audited.findings.iter().find(|f| f.code == "paiy_caretaker_not_required").expect("the finding");
    assert_eq!(f.state, None, "failed: {f:?}");
    assert!(f.message.contains("zorunlu değil") && f.message.contains("2 konut bölümü"), "{}", f.message);
    assert_eq!(f.uids.len(), 3, "the flat's three rooms: {f:?}");
    let set = app.arch.arch_audit.rule_set.as_ref().expect("the rule set");
    let rows: Vec<(&str, kentos_commands::ArchAuditOutcome)> =
        set.rules.iter().filter(|r| r.rule.contains(" 22/1-")).map(|r| (r.rule.as_str(), r.state)).collect();
    use kentos_commands::ArchAuditOutcome as O;
    assert_eq!(
        rows,
        [("PAİY 22/1-ç", O::Failed), ("PAİY 22/1-f", O::OutOfScope), ("PAİY 22/1-k", O::OutOfScope), ("PAİY 22/1-l", O::OutOfScope)]
    );
}

/// The basement the active storey, drawn alone (Yalnız etkin kat): the pictures show the bays the test names.
fn basement_alone(app: &mut App) {
    let _ = app.update(Message::ActiveLevel(Some("b01".into())));
    let _ = app.update(Message::LevelView(kentos_interaction::level_view::LevelView::Active));
}

/// Emsal ve TAKS tablosu beside the two-storey plan with flat 1's storage annex in the basement (the part past the 50 %
/// on its own row), and Denetim's finding of the caretaker flat, light and dark; `.run/gui-evidence/emsal-22-8079/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib emsal_limits_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/emsal-22-8079");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1280.0, 800.0), "koyu")] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = storage(-3.0);
        basement_alone(&mut app);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("arch.zoningTable"));
        let _ = app.submit_line("17,6");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-depo-eklentisi-ve-emsal-tablosu-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());

        let mut app = caretaker();
        basement_alone(&mut app);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
        checked(&mut app);
        let audited = app.arch.arch_audit.audited.as_ref().expect("checked");
        app.arch.arch_audit.selected = audited.findings.iter().position(|f| f.code == "paiy_caretaker_not_required");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-kapici-dairesi-denetim-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
