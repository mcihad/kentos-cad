//! Taşıma planı (haul_plan_window.rs, plan/road.md Y23): the window reads `road.hauls.plan` as the shared cases do.

use crate::app::{App, Dialog, Message};
use crate::haul_plan_window::Event as W;

const CASES: &str = include_str!("../../../../fixtures/commands/v1/road.hauls.plan.json");

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

/// ROAD-HAUL-12 (docs/adr/0953): Planı yaz's sheets carry the plan as read — its summary with units and the volume's
/// revision, the flows whose volume times length adds up to the moment, the sites, the barriers.
#[test]
fn the_plan_sheets_carry_the_plan() {
    const NAME: &str = "Uygunsuz yarma ve engel birlikte";
    let mut app = plan_app(Some(NAME));
    let _ = app.update(Message::Run("haul.plan"));
    send(&mut app, W::Haul("h-1".into()));
    send(&mut app, W::Preview);
    let w = app.haul_plan_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    let sheets = kentos_native_application::road_hauls_plan::sheets(&app.document.as_ref().expect("a drawing").model, r);
    let names: Vec<&str> = sheets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Özet", "Taşımalar", "Alanlar", "Engeller"]);
    let summary = &sheets[0].rows;
    let value = |k: &str| summary.iter().find(|row| row[0] == k).unwrap_or_else(|| panic!("{k}")).clone();
    assert_eq!(value("Taşıma momenti")[2], "m³·m");
    assert_eq!(value("Taşıma momenti")[1], format!("{:.3}", r.moment));
    assert!(value("Kübaj sürümü")[1].parse::<u32>().is_ok(), "the volume's revision is written");
    assert!(value("Hacimler")[1].contains("düzeltilmiş"), "what the volumes are is said");
    // Σ (|suitable| + |unsuitable|) × haul length over the flows is the moment (the sheet's three decimals per row).
    let flows = &sheets[1];
    let at = |name: &str| flows.columns.iter().position(|c| c.name == name).unwrap_or_else(|| panic!("{name}"));
    let (len_at, good_at, bad_at) = (at("Taşıma boyu (m)"), at("Uygun (m³, + ileri)"), at("Uygunsuz (m³, + ileri)"));
    let num = |v: &String| v.parse::<f64>().unwrap();
    let moment: f64 = flows.rows.iter().map(|row| num(&row[len_at]) * (num(&row[good_at]).abs() + num(&row[bad_at]).abs())).sum();
    // Each figure is written to the half millimetre (or half litre): a row's product is off by at most
    // L·(δg + δb) + (|g| + |b|)·δL + δL·(δg + δb), δ = 0.0005.
    let d = 0.0005;
    let bound: f64 = flows.rows.iter().map(|row| num(&row[len_at]) * 2.0 * d + (num(&row[good_at]).abs() + num(&row[bad_at]).abs()) * d + 2.0 * d * d).sum();
    assert!((moment - r.moment).abs() <= bound, "{moment} vs {} (bound {bound})", r.moment);
    assert_eq!(sheets[2].rows.len(), r.sites.len());
    assert_eq!(sheets[3].rows[0][3], format!("{:.3}", r.barriers[0].crossed));
}

/// Planı yaz's row under the plan, light; `.run/gui-evidence/yol-y47/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib haul_plan_window_tests::screens_write -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_write() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y47");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = plan_app(Some("Uygunsuz yarma ve engel birlikte"));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("haul.plan"));
        send(&mut app, W::Haul("h-1".into()));
        send(&mut app, W::Preview);
        snapshot.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("y47-plan-yaz-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
        println!("{}", path.display());
        if theme == "light" {
            // The workbook Planı yaz writes, for the owner to open.
            let w = app.haul_plan_window.as_ref().expect("open");
            let Some(Ok(r)) = &w.preview else { panic!("a read") };
            let sheets = kentos_native_application::road_hauls_plan::sheets(&app.document.as_ref().expect("a drawing").model, r);
            let book = out.join("y47-tasima-plani.xlsx");
            let _ = crate::table_write::write(sheets, kentos_commands::WorkbookFormat::Xlsx, &book);
            println!("{}", book.display());
        }
    }
}

/// ROAD-HAUL-08 (docs/adr/0954): Onayla writes the plan as read with its note; the plan then reads against it, the
/// same; Onayı kaldır sends it to the history.
#[test]
fn the_window_approves_and_takes_the_approval_off() {
    let mut app = plan_app(Some("İki döküm alanı: yakın olan dolar"));
    let _ = app.update(Message::Run("haul.plan"));
    send(&mut app, W::Haul("h-1".into()));
    send(&mut app, W::ApprovalNote("  Şantiye şefiyle görüşüldü ".into()));
    send(&mut app, W::Approve(true));
    let haul = |app: &App| app.document.as_ref().expect("a drawing").model.road_hauls()[0].clone();
    let a = haul(&app).approved.expect("approved");
    assert_eq!((a.number, a.note.as_deref(), a.cost), (1, Some("Şantiye şefiyle görüşüldü"), false));
    let w = app.haul_plan_window.as_ref().expect("open");
    assert!(w.approval_note.is_empty(), "the note is written once");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    let approval = r.approval.as_ref().expect("read against its approval");
    assert!(!approval.stale && approval.reasons.is_empty());
    assert!((a.moment - r.moment).abs() < 1e-9);
    send(&mut app, W::Approve(false));
    let h = haul(&app);
    assert!(h.approved.is_none());
    assert_eq!(h.history.len(), 1);
    assert!(h.history[0].ended.as_ref().is_some_and(|e| e.revoked));
    let w = app.haul_plan_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    assert!(r.approval.is_none());
}

/// A plan whose site grew after it was approved says what changed, in the warnings and against each site.
#[test]
fn a_stale_approval_says_why() {
    const NAME: &str = "Onaydan sonra alanın kapasitesi büyüdü";
    let mut app = plan_app(Some(NAME));
    let _ = app.update(Message::Run("haul.plan"));
    send(&mut app, W::Haul("h-1".into()));
    send(&mut app, W::List(crate::haul_plan_window::Listed::Approval));
    let w = app.haul_plan_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    let a = r.approval.as_ref().expect("an approval");
    let want = &case(NAME)["steps"][0]["result"]["output"]["approval"];
    assert!(a.stale);
    assert_eq!(serde_json::to_value(&a.reasons).expect("reasons"), want["reasons"]);
    assert!(w.warnings.iter().any(|x| x.contains("eskidi") && x.contains("D1 alanının kapasitesi değişti")));
    assert!(a.sites.iter().any(|s| s.name == "D1" && s.capacity == Some(14000.0)));
    // Planı yaz carries the approval and why it went stale.
    let sheets = kentos_native_application::road_hauls_plan::sheets(&app.document.as_ref().expect("a drawing").model, r);
    let value = |k: &str| sheets[0].rows.iter().find(|row| row[0] == k).unwrap_or_else(|| panic!("{k}"))[1].clone();
    assert_eq!(value("Taşıma planı onayı"), "1. onay · 2026-11-04");
    assert_eq!(value("Onaylı taşıma momenti"), format!("{:.3}", a.approved.moment));
    assert!(value("Onay").starts_with("eskidi: Brüknerin tanımı değişti"));
}

/// The approval in the window: stale with why and the sites against today's, light and wide; still the same, dark and
/// narrow. `.run/gui-evidence/yol-y48/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib haul_plan_window_tests::screens_approval -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_approval() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y48");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (case_name, theme, size, list, name) in [
        ("Onaydan sonra alan kalktı, yenisi eklendi", "light", Size::new(1440.0, 900.0), true, "eskidi-acik-genis"),
        ("Onaydan sonra alanın kapasitesi büyüdü", "light", Size::new(1440.0, 900.0), false, "eskidi-alanlar-acik"),
        ("Ekonomik onay, fiyat değişti", "dark", Size::new(1440.0, 900.0), true, "tutar-koyu-genis"),
        ("Onaylı plan değişmedi", "dark", Size::new(1100.0, 650.0), true, "ayni-koyu-dar"),
        ("İki döküm alanı: yakın olan dolar", "light", Size::new(1440.0, 900.0), false, "onaysiz-acik"),
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
        send(&mut app, W::Haul("h-1".into()));
        if case_name.starts_with("Ekonomik") {
            send(&mut app, W::Aim(crate::haul_plan_window::Aim::Cost));
        }
        if list {
            send(&mut app, W::List(crate::haul_plan_window::Listed::Approval));
        }
        if name == "onaysiz-acik" {
            send(&mut app, W::ApprovalNote("Şantiye şefiyle görüşüldü".into()));
        }
        snapshot.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("y48-onay-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
        println!("{}", path.display());
    }
}

/// ROAD-HAUL-06 (docs/adr/0955): a site off the road stands where its access meets the alignment and the access is
/// part of the moment; the network's view reads `road.hauls.network` as its cases do.
#[test]
fn the_window_reads_the_haul_road_network() {
    const NAME: &str = "Ağdan döküm ve yol üstü döküm birlikte";
    let mut app = plan_app(Some(NAME));
    let _ = app.update(Message::Run("haul.plan"));
    send(&mut app, W::Haul("h-1".into()));
    send(&mut app, W::Show(crate::haul_plan_window::Shown::Network));
    let w = app.haul_plan_window.as_ref().expect("open");
    let Some(Ok(r)) = &w.preview else { panic!("a read") };
    let want = &case(NAME)["steps"][0]["result"]["output"];
    assert!((r.moment - want["moment"].as_f64().expect("a moment")).abs() < 1e-5 * r.moment);
    let access = r.sites[0].access.expect("an access");
    assert!((access - want["sites"][0]["access"].as_f64().expect("an access")).abs() < 1e-6);
    assert!(r.access_moment.is_some_and(|a| (a - access * r.sites[0].used).abs() < 1e-6 * a.max(1.0)));
    let Some(Ok((n, warnings))) = &w.network else { panic!("the network reads") };
    assert_eq!(n.sites.len(), 1);
    assert!(n.sites[0].joined && n.sites[0].roads == [0]);
    assert!(n.problems.iter().any(|p| p.kind == kentos_commands::HaulNetworkIssue::Crossing));
    assert!(warnings.iter().any(|x| x.contains("birleşmeyen")));
    let drawn = w.drawn.as_ref().expect("drawn");
    assert_eq!(drawn.roads.len(), n.roads.len());
    assert!(drawn.axis.len() > 100);
}

/// The network in plan and the flow with a site off the road; a cut off site; `.run/gui-evidence/yol-y49/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib haul_plan_window_tests::screens_network -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_network() {
    use crate::haul_plan_window::Shown;
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y49");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (case_name, haul, theme, size, shown, name) in [
        ("Ağdan döküm ve yol üstü döküm birlikte", "h-1", "light", Size::new(1440.0, 900.0), Shown::Network, "ag-acik-genis"),
        ("Ağdan döküm ve yol üstü döküm birlikte", "h-1", "light", Size::new(1440.0, 900.0), Shown::Flow, "akis-acik-genis"),
        ("Kopuk alan hesaba katılmaz", "h-1", "dark", Size::new(1440.0, 900.0), Shown::Network, "kopuk-koyu-genis"),
        ("Ağdan ödünç alanı km 520'den dolguları besler", "h-3", "dark", Size::new(1100.0, 650.0), Shown::Network, "odunc-koyu-dar"),
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
        send(&mut app, W::Show(shown));
        snapshot.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("y49-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&path).expect("writes the picture");
        println!("{}", path.display());
    }
}
