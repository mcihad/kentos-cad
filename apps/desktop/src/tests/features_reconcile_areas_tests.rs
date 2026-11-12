//! Detay eşleştirme with area candidates (features_reconcile_window.rs, plan/map-cad.md MAP-04, docs/adr/2140): on the
//! shared case's drawing of building footprints and green areas, the window's plan is the case's (the footprints that
//! overlap above 0.5 agree, the overlap ratio in the table), the overlap box joins the two footprints overlapping 0.25
//! when set to 0.2, and Eşleştir writes the agreed building as its most trusted footprint with Örtüşme, one undo step.
//! And the pictures.

use kentos_commands::Entity;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::features_reconcile_window::Event as R;

const PLAN: &str = "Alanlar (plan): bina izi iki kaynakta örtüşür (yineleneniyle), 0,25 örtüşen iki iz ayrı tekler, var \
                    olan binanın üstündeki aday, yeşil alan/bina çelişkisi, nokta çizilmiş bina, alan çizilmiş direk";

fn case() -> Value {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.features.reconcile.json")).expect("cases");
    cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == PLAN).expect("the case").clone()
}

fn opened() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let _ = app.update(Message::Run("map.featuresReconcile"));
    assert_eq!(app.dialog, Some(Dialog::FeaturesReconcile));
    app
}

fn answer(app: &App) -> kentos_commands::FeaturesReconciled {
    app.features_reconcile.answer.clone().expect("an answer").expect("reconciled")
}

#[test]
fn footprints_agree_by_their_overlap_and_uzlastir_writes_the_trusted_one() {
    let mut app = opened();
    let a = answer(&app);
    let want = &case()["steps"][0]["result"]["output"]["groups"];
    let statuses: Vec<&str> = a.groups.iter().map(|g| g.status.as_str()).collect();
    let wanted: Vec<&str> = want.as_array().expect("groups").iter().map(|g| g["status"].as_str().expect("status")).collect();
    assert_eq!(statuses, wanted);
    let building = a.groups.iter().find(|g| g.status == "agreed").expect("the agreed building");
    let ratio = building.overlap.expect("an area group's ratio");
    assert!((ratio - want[0]["overlap"].as_f64().expect("ratio")).abs() < 1e-9 && ratio > 0.85, "{ratio}");
    // 0.2: the two footprints overlapping 0.25 are one group.
    let _ = app.update(Message::FeaturesReconcile(R::Overlap(0.2)));
    assert_eq!(answer(&app).groups.iter().filter(|g| g.status == "agreed").count(), 2);
    let _ = app.update(Message::FeaturesReconcile(R::Overlap(0.5)));
    // Eşleştir: the agreed footprint on Bina izleri, with its overlap, one undo step.
    let before = app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0);
    let _ = app.update(Message::FeaturesReconcile(R::Run));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let written: Vec<&Entity> = doc
        .entities()
        .filter(|e| e.base().layer_id == "hv-bna" && e.base().attrs.contains_key("Örtüşme"))
        .collect();
    assert_eq!(written.len(), 1);
    let Entity::Polygon(p) = written[0] else { panic!("a footprint") };
    assert_eq!(p.base.attrs.get("Kaynaklar").map(String::as_str), Some("LiDAR, Ortofoto"));
    assert_eq!(p.base.attrs.get("Örtüşme").map(String::as_str), Some(format!("{ratio:.3}").as_str()));
    assert_eq!(p.base.attrs.get("KatSayisi").map(String::as_str), Some("3"), "the trusted footprint's fields");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(app.document.as_ref().map(|d| d.model.entities().count()).unwrap_or(0), before, "one undo step");
}

/// The window on the footprints: the plan with the overlap column, and after Eşleştir; light wide and dark narrow,
/// `.run/gui-evidence/aday-alanlar-2140/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib features_reconcile_areas_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/aday-alanlar-2140");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-plan-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::FeaturesReconcile(R::Run));
        let _ = app.update(Message::FeaturesReconcile(R::Close));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-yazildi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
