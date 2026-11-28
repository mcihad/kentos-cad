//! Merdiven altı (docs/adr/8080; Planlı Alanlar İmar Yönetmeliği 4/1-ğ-3, 31/6) on a duplex flat, worked by hand:
//!
//! - Zemin kat at ±0.00 and 1. kat at +3.06, each 3.06 m; on each a 10 × 5 m plan inside 25 cm walls, one room of the
//!   flat (net 9.50 × 4.50 = 42.75 m²); 1. kat's slab (15 cm structure, its top +3.06) over the whole plan.
//! - A straight stair on Zemin kat from (2.00, 2.50) along x: 18 risers of 0.17 m (3.06 m), treads of 0.28 m, 1.00 m
//!   wide. Its waist is the slab it arrives at: 0.15 m, measured upright under the steps' roots 0.15·√(1 + (0.17/0.28)²)
//!   = 0.175 48 m.
//! - Its underside leaves the floor 0.175 48/(0.17/0.28) = 0.289 0 m along it and reaches 1.80 m at
//!   (1.80 + 0.175 48)/(0.17/0.28) = 3.253 7 m; the tread over that point stands at 1.87 m (from 2.80 m), so the
//!   underside binds. Lower than 1.80 m: 3.253 7 m² (out of the flat's net, 4/1-ğ-3); of it a space from 0.289 0 m on:
//!   1.80 × 0.28/0.17 = 2.964 7 m², open into the room it stands in (to be closed, 31/6).

use crate::arch::ArchState;
use crate::arch_ext::*;
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

const SLOPE: f64 = 0.17 / 0.28;

/// The stair's places under 1.80 m and the space among them, by hand (see the module's words).
fn by_hand() -> (f64, f64) {
    let upright = 0.15 * (1.0 + SLOPE * SLOPE).sqrt();
    ((1.80 + upright) / SLOPE, 1.80 / SLOPE)
}

/// The duplex flat: its ground storey's room named `name`, its upper one Yatak odası.
fn duplex(name: &str) -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(
        Some(Module::Architecture),
        None,
        Settings::memory(),
        Recovery::off(),
    );
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    let mut settings = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .settings()
        .clone();
    let mut arch = settings.arch.clone().expect("arch");
    let level = |id: &str, name: &str, z: f64| kentos_commands::ArchLevel {
        id: id.into(),
        name: name.into(),
        elevation: z,
        height: Some(3.06),
    };
    arch.levels = vec![level("k00", "Zemin kat", 0.0), level("k01", "1. kat", 3.06)];
    settings.arch = Some(arch);
    app.document
        .as_mut()
        .expect("a drawing")
        .model
        .set_settings(settings);
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    for level in ["k00", "k01"] {
        let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.25, "arch": {"level": level}});
        command(
            &mut app,
            "cad.walls.create",
            json!({"layerId": layer, "walls": [
                wall((0.125, 0.125), (9.875, 0.125)), wall((9.875, 0.125), (9.875, 4.875)),
                wall((9.875, 4.875), (0.125, 4.875)), wall((0.125, 4.875), (0.125, 0.125))
            ]}),
        );
    }
    command(
        &mut app,
        "cad.slabs.create",
        json!({"layerId": layer, "slabs": [{
            "parts": [{"outer": {"pts": [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 5.0), p(0.0, 5.0)]}}],
            "thickness": 0.15, "arch": {"level": "k01"}
        }]}),
    );
    command(
        &mut app,
        "cad.stairs.create",
        json!({"layerId": layer, "stairs": [{
            "pts": [p(2.0, 2.5), p(2.0 + 17.0 * 0.28, 2.5)], "width": 1.0, "risers": 18, "riserHeight": 0.17,
            "tread": 0.28, "arch": {"level": "k00"}
        }]}),
    );
    command(
        &mut app,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "nature": "Daire"}
        ]}}),
    );
    command(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"seed": p(8.5, 1.0), "arch": {"level": "k00"}, "unit": "d1", "name": name},
            {"seed": p(8.5, 1.0), "arch": {"level": "k01"}, "unit": "d1", "name": "Yatak odası"}
        ]}),
    );
    app
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn the_place_under_a_stair_lower_than_1_80_is_out_of_the_flat_s_net_area() {
    let app = duplex("Salon");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let (low, open) = by_hand();
    let unders = doc.under_stairs();
    assert_eq!(unders.len(), 1, "{unders:?}");
    let u = &unders[0];
    assert!(u.waist, "the waist is the slab it arrives at");
    let area = |a: &[kentos_geometry_core::geom::arrangement::Area]| {
        kentos_geometry_core::geom::space::area(a)
    };
    assert!(near(area(&u.low), low), "{} ≠ {low}", area(&u.low));
    assert!(near(area(&u.open), open), "{} ≠ {open}", area(&u.open));
    // The flat's net: its two rooms (42.75 m² each) less the place under the stair.
    let groups = doc.paiy_areas();
    let (_, g) = groups.iter().find(|(id, _)| id == "d1").expect("the flat");
    assert!(near(g.under_stair, low), "{g:?}");
    assert!(near(g.net, 2.0 * 42.75 - low), "{g:?}");
    // The storeys' parts: only Zemin kat's room lies over it.
    let nets: Vec<f64> = g.levels.iter().map(|(_, _, n)| *n).collect();
    assert!(
        near(nets[0], 42.75 - low) && near(nets[1], 42.75),
        "{nets:?}"
    );
    // The facts name the room its space opens into.
    let facts = kentos_processing::audit_facts::building::measure(doc).normalized();
    let under = &facts.under_stairs[0];
    assert_eq!(under.rooms.len(), 1, "{under:?}");
    assert!(
        under.rooms[0].name.starts_with("Salon") && (under.rooms[0].area - open).abs() < 1e-4,
        "{under:?}"
    );
}

#[test]
fn the_metrekare_cetveli_writes_the_flat_s_net_without_it() {
    let mut app = duplex("Salon");
    let _ = app.update(Message::Run("tool.scheduleAreas"));
    let _ = app.submit_line("14,0");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let cells = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Table(t) => Some(t.cells.clone()),
            _ => None,
        })
        .expect("the metrekare cetveli");
    let (low, _) = by_hand();
    let net = |v: f64| format!("{v:.2}");
    let rows: Vec<(&str, &str)> = cells
        .iter()
        .map(|r| (r[0].as_str(), r[5].as_str()))
        .collect();
    assert_eq!(
        rows[1],
        ("Zemin kat", net(42.75 - low).as_str()),
        "{cells:?}"
    );
    assert_eq!(rows[3], ("1. kat", "42.75"), "{cells:?}");
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

/// 31/6: the space under the stair opens into the living room: a finding naming the stair and the room; the same
/// stair standing in a store is closed off, and the article passes.
#[test]
fn a_low_space_under_a_stair_open_into_a_room_is_a_finding_of_31_6() {
    let mut app = duplex("Salon");
    crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
    checked(&mut app);
    let audited = app
        .modules
        .arch()
        .arch_audit
        .audited
        .as_ref()
        .expect("checked");
    let f = audited
        .findings
        .iter()
        .find(|f| f.code == "paiy_under_stair_open")
        .expect("the finding");
    assert!(
        f.message.contains("“Salon”") && f.message.contains("2.96 m²"),
        "{}",
        f.message
    );
    assert_eq!((f.state, f.uids.len()), (None, 2), "{f:?}");
    let row = |app: &App| {
        let set = app
            .modules
            .arch()
            .arch_audit
            .rule_set
            .as_ref()
            .expect("the rule set");
        set.rules
            .iter()
            .find(|r| r.rule == "PAİY 31/6")
            .map(|r| r.state)
    };
    assert_eq!(row(&app), Some(kentos_commands::ArchAuditOutcome::Failed));
    let mut closed = duplex("Depo");
    crate::regulation_dates_tests::dated(&mut closed, Some("02.07.2026"));
    checked(&mut closed);
    let audited = closed
        .modules
        .arch()
        .arch_audit
        .audited
        .as_ref()
        .expect("checked");
    assert!(
        !audited
            .findings
            .iter()
            .any(|f| f.code == "paiy_under_stair_open")
    );
    assert_eq!(
        row(&closed),
        Some(kentos_commands::ArchAuditOutcome::Passed)
    );
}

/// The duplex's ground storey with Metrekare cetveli beside it and Denetim's 31/6 finding, light and dark;
/// `.run/gui-evidence/merdiven-alti-8080/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib under_stair_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/merdiven-alti-8080");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(1280.0, 800.0), "koyu"),
    ] {
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut app = duplex("Salon");
        let _ = app.update(Message::ActiveLevel(Some("k00".into())));
        let _ = app.update(Message::LevelView(
            kentos_interaction::level_view::LevelView::Active,
        ));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("tool.scheduleAreas"));
        let _ = app.submit_line("12,0");
        crate::regulation_dates_tests::dated(&mut app, Some("02.07.2026"));
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
            .position(|f| f.code == "paiy_under_stair_open");
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        app.selection.set(Vec::new());
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-merdiven-alti-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    }
}
