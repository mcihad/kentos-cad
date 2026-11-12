//! Kamyon filosu (haul_fleet_window.rs, plan/road.md Y32.3): opened from Taşıma planı it reads `road.hauls.trucks` as
//! the shared cases do; the values are kept when it is closed; and its pictures.

use crate::app::{App, Message};
use crate::haul_fleet_window::{Event as F, Listed, Road};
use crate::haul_plan_window::Event as P;
use crate::road;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/road.hauls.trucks.json");

fn case(name: &str) -> serde_json::Value {
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone()
}

/// The shared cases' drawing: the side diagram with a borrow and a waste site.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: F) {
    let _ = app.update(Message::Road(road::Event::HaulPlan(P::Fleet(e))));
}

/// The case's truck and times typed in the window, its fleet and fuel too.
fn typed(app: &mut App, input: &serde_json::Value) {
    let _ = app.update(Message::Run("haul.plan"));
    let _ = app.update(Message::Road(road::Event::HaulPlan(P::Haul("h-2".into()))));
    let _ = app.update(Message::Road(road::Event::HaulPlan(P::OpenFleet)));
    let t = &input["truck"];
    let text = |v: &serde_json::Value| v.as_f64().map(|n| format!("{n}").replace('.', ",")).unwrap_or_default();
    let values = [
        &t["empty"], &t["gross"], &t["power"], &t["efficiency"], &t["top"], &t["downhill"], &t["traction"], &t["drive"],
        &t["brake"], &input["rolling"], &input["load"], &input["loading"], &input["dumping"], &input["spotting"],
        &input["trucks"], &input["hour"], &input["fuel"], &input["idle"], &input["co2"],
    ];
    for (k, v) in values.into_iter().enumerate() {
        send(app, F::Field(k, text(v)));
    }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 2e-3 * b.abs().max(1.0)
}

#[test]
fn the_window_reads_the_fleet_as_the_cases_do_and_keeps_its_values() {
    let mut app = app();
    let c = case("Yakıt, rölanti ve karbon");
    typed(&mut app, &c["steps"][0]["input"]);
    let want = &c["steps"][0]["result"]["output"];
    let w = app.road.haul_plan_window.as_ref().expect("open");
    let x = w.fleet.as_ref().expect("the fleet window");
    let out = x.result.clone().expect("read").expect("reads");
    assert_eq!(out.hauls.len(), want["hauls"].as_array().unwrap().len());
    assert!(close(out.trips, want["trips"].as_f64().unwrap()));
    assert!(close(out.cycle.unwrap(), want["cycle"].as_f64().unwrap()));
    let fleet = out.fleet.expect("a fleet");
    assert!(close(fleet.queued, want["fleet"]["queued"].as_f64().unwrap()), "{fleet:?}");
    assert!(close(out.fuel.unwrap(), want["fuel"].as_f64().unwrap()));
    // The speeds of the first haul are drawn.
    assert!(out.trace.as_ref().is_some_and(|t| t.loaded.len() > 2 && t.empty.len() > 2));
    // Another haul traced, the ground, the fleet table: read again.
    send(&mut app, F::Trace(2));
    send(&mut app, F::List(Listed::Fleet));
    let x = app
        .road
        .haul_plan_window
        .as_ref()
        .unwrap()
        .fleet
        .as_ref()
        .unwrap();
    assert_eq!(
        x.result
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .trace
            .as_ref()
            .unwrap()
            .haul,
        2
    );
    send(&mut app, F::Road(Road::Ground));
    let x = app
        .road
        .haul_plan_window
        .as_ref()
        .unwrap()
        .fleet
        .as_ref()
        .unwrap();
    let why = x
        .result
        .clone()
        .unwrap()
        .expect_err("the side alignment's start is off the surface");
    assert!(why.contains("Arazi km"), "{why}");
    // Geri keeps the values; opened again they are there.
    send(&mut app, F::Back);
    assert!(app.road.haul_plan_window.as_ref().unwrap().fleet.is_none());
    let _ = app.update(Message::Road(road::Event::HaulPlan(P::OpenFleet)));
    let x = app
        .road
        .haul_plan_window
        .as_ref()
        .unwrap()
        .fleet
        .as_ref()
        .unwrap();
    assert_eq!((x.fields[2].as_str(), x.road), ("300", Road::Ground));
}

#[test]
fn a_missing_value_says_what_to_type() {
    let mut app = app();
    let _ = app.update(Message::Run("haul.plan"));
    let _ = app.update(Message::Road(road::Event::HaulPlan(P::Haul("h-2".into()))));
    let _ = app.update(Message::Road(road::Event::HaulPlan(P::OpenFleet)));
    let x = app
        .road
        .haul_plan_window
        .as_ref()
        .unwrap()
        .fleet
        .as_ref()
        .unwrap();
    let why = x.result.clone().unwrap().expect_err("nothing typed");
    assert!(why.contains("Boş kütle"), "{why}");
    send(&mut app, F::Field(14, "2,5".into()));
    for (k, v) in [(0, "15"), (1, "40"), (2, "300"), (3, "0,8"), (4, "50"), (6, "0,6"), (7, "0,7"), (8, "2"), (9, "3"), (10, "15"), (11, "2,5"), (12, "1")] {
        send(&mut app, F::Field(k, v.into()));
    }
    let x = app
        .road
        .haul_plan_window
        .as_ref()
        .unwrap()
        .fleet
        .as_ref()
        .unwrap();
    let why = x.result.clone().unwrap().expect_err("2,5 trucks");
    assert!(why.contains("tamsayı"), "{why}");
}

/// The window, wide and narrow, light and dark, full screen with its chart zoomed; `.run/gui-evidence/yol-y32/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib haul_fleet_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};
    use kentos_ui::widget::sizable::sizes;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y32");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let input = case("Yakıt, rölanti ve karbon")["steps"][0]["input"].clone();
    for (theme, size, name, full) in [
        ("light", Size::new(1440.0, 900.0), "acik", false),
        ("dark", Size::new(1440.0, 900.0), "koyu", false),
        ("dark", Size::new(1920.0, 1080.0), "tam-ekran", true),
    ] {
        for step in ["bos", "filo", "tablo"] {
            if full {
                sizes::set(crate::haul_fleet_window::TITLE, sizes::Kept { width: 800.0, height: 600.0, maximized: true });
            } else {
                sizes::forget(crate::haul_fleet_window::TITLE);
            }
            let mut app = app();
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("alignment"));
            match step {
                "bos" => {
                    let _ = app.update(Message::Run("haul.plan"));
                    let _ = app.update(Message::Road(road::Event::HaulPlan(P::Haul("h-2".into()))));
                    let _ = app.update(Message::Road(road::Event::HaulPlan(P::OpenFleet)));
                }
                _ => typed(&mut app, &input),
            }
            send(&mut app, F::Trace(1));
            if step == "tablo" {
                send(&mut app, F::List(Listed::Fleet));
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            if full && step == "filo" {
                // The speeds zoomed on the haul's start: the run up to speed.
                let at = Point::new(size.width * 0.45, size.height * 0.3);
                snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, 5.0));
            }
            let file = out.join(format!("y32-3-{step}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        }
    }
    sizes::forget(crate::haul_fleet_window::TITLE);
}
