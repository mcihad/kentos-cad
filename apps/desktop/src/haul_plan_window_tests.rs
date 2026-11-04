//! Taşıma planı (haul_plan_window.rs, plan/road.md Y23): the window reads `road.hauls.plan` as the shared cases do.

use crate::app::{App, Dialog, Message};
use crate::haul_plan_window::Event as W;

const CASES: &str = include_str!("../../../fixtures/commands/v1/road.hauls.plan.json");

fn case(name: &str) -> serde_json::Value {
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case").clone()
}

/// The app on a case's drawing (its own setup, else the file's).
fn plan_app(name: Option<&str>) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("cases");
    let setup = name.map(case).and_then(|c| c.get("setup").cloned()).unwrap_or_else(|| cases["setup"].clone());
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::HaulPlan(e));
}

#[test]
fn the_window_reads_a_haul_plan() {
    const NAME: &str = "Yan Brükner: ödünç alanı dolguları besler";
    let mut app = plan_app(Some(NAME));
    let before = app.document.as_ref().expect("a drawing").model.revision().to_string();
    let _ = app.update(Message::Run("haul.plan"));
    assert!(matches!(app.dialog, Some(Dialog::HaulPlan)));
    send(&mut app, W::Haul("h-2".into()));
    send(&mut app, W::Preview);
    let w = app.haul_plan_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    let want = &case(NAME)["steps"][0]["result"]["output"];
    assert!(r.feasible);
    assert!((r.moment - want["moment"].as_f64().expect("a moment")).abs() < 1e-3);
    assert_eq!(r.sites.len(), 2);
    send(&mut app, W::Close);
    assert_eq!(app.document.as_ref().expect("a drawing").model.revision().to_string(), before);
}

#[test]
fn an_unclosed_plan_says_why() {
    let mut app = plan_app(None);
    let _ = app.update(Message::Run("haul.plan"));
    send(&mut app, W::Haul("h-3".into()));
    send(&mut app, W::Preview);
    let w = app.haul_plan_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    assert!(!r.feasible && r.shortage > 0.0);
    assert!(w.warnings.iter().any(|x| x.contains("karşılanmayan dolgu")));
}

#[test]
fn the_window_reads_an_economic_plan() {
    const NAME: &str = "Ekonomik: dolgu fazlası, ödünç ve döküm";
    let mut app = plan_app(Some(NAME));
    let _ = app.update(Message::Run("haul.plan"));
    send(&mut app, W::Haul("h-3".into()));
    send(&mut app, W::Aim(crate::haul_plan_window::Aim::Cost));
    send(&mut app, W::Preview);
    let w = app.haul_plan_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    let want = &case(NAME)["steps"][0]["result"]["output"];
    let c = r.cost.as_ref().expect("a cost");
    assert!((c.total - want["cost"]["total"].as_f64().expect("a total")).abs() < 1e-3);
}

/// Taşıma planı, wide and narrow, light and dark; `.run/gui-evidence/yol-y23-tasima-*` (the economic plan, Y24, too):
///
/// ```text
/// cargo test -p kentos-desktop --lib haul_plan_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        for (case_name, haul, file, cost) in [
            ("Yan Brükner: ödünç alanı dolguları besler", "h-2", "odunc", false),
            ("Dolgu fazlası: ödünç ve döküm birlikte", "h-3", "dolgu", false),
            ("Dolgu fazlası: ödünç yetmez", "h-3", "yetmez", false),
            ("Ekonomik: dolgu fazlası, ödünç ve döküm", "h-3", "ekonomik", true),
            ("Ekonomik: serbest taşımalı yan Brükner", "h-2", "serbest", true),
        ] {
            let mut app = plan_app(Some(case_name));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            let _ = app.update(Message::RibbonTab("corridor"));
            let _ = app.update(Message::Run("haul.plan"));
            send(&mut app, W::Haul(haul.into()));
            if cost {
                send(&mut app, W::Aim(crate::haul_plan_window::Aim::Cost));
            }
            send(&mut app, W::Preview);
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("yol-y23-tasima-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
            println!("{}", path.display());
        }
    }
}

/// Y38.3 (docs/adr/0944): a diagram with unsuitable cut and a barrier is planned as `road.hauls.plan`'s shared case
/// does: the unsuitable cut and its waste, the barrier's crossing.
#[test]
fn the_window_reads_a_sorted_plan() {
    const NAME: &str = "Uygunsuz yarma ve engel birlikte";
    let mut app = plan_app(Some(NAME));
    let _ = app.update(Message::Run("haul.plan"));
    send(&mut app, W::Haul("h-1".into()));
    send(&mut app, W::Preview);
    let w = app.haul_plan_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    let want = &case(NAME)["steps"][0]["result"]["output"];
    let close = |a: f64, v: &serde_json::Value| (a - v.as_f64().expect("a number")).abs() <= 1e-5 * v.as_f64().unwrap().abs().max(1.0);
    assert!(close(r.moment, &want["moment"]) && close(r.unsuitable.expect("sorted"), &want["unsuitable"]));
    assert_eq!(r.barriers.len(), 1);
    assert!(close(r.barriers[0].crossed, &want["barriers"][0]["crossed"]));
    assert!(r.segments.iter().any(|s| s.unsuitable_flow.is_some_and(|u| u.abs() > 0.0)), "the unsuitable cut moves");
}

/// The sorted plans, light and dark; `.run/gui-evidence/yol-y38/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib haul_plan_window_tests::screens_sorted -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_sorted() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y38");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        for (case_name, file) in [
            ("Uygunsuz yarma ve engel birlikte", "uygunsuz-engel"),
            ("Kapalı kavşak taşımayı ikiye böler", "kapali"),
            ("Uygunsuz için döküm yetmez", "dokum-yetmez"),
        ] {
            let mut app = plan_app(Some(case_name));
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            let _ = app.update(Message::RibbonTab("corridor"));
            let _ = app.update(Message::Run("haul.plan"));
            send(&mut app, W::Haul("h-1".into()));
            send(&mut app, W::Preview);
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("y38-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
            println!("{}", path.display());
        }
    }
}
