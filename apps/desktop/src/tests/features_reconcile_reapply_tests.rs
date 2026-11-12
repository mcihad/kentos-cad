//! Detay eşleştirme's Önceki kararlar (features_reconcile_window.rs, docs/adr/4540) on the shared case's drawing of
//! `map.features.reconcile` after a reviewed run and a model's new one: four decisions recorded as resolved review notes
//! (a manhole accepted, a tree corrected to a pole, a door given its number, an unknown rejected), the assets written
//! then, and new candidates near them. Without the switch they are read afresh (the tree a tree); with it the new
//! candidates take the recorded decisions (the tree the pole written then), Eşleştir marks them, their notes naming the
//! earlier ones, in one undo step. And the pictures.

use crate::gis;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::features_reconcile_window::Event as R;

const CASE: &str = "Yeniden çıkarım, plan";

fn opened() -> App {
    let cases: Value = serde_json::from_str(include_str!("../../../../fixtures/commands/v1/map.features.reconcile.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(CASE)))
        .cloned()
        .expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The candidate's class and status in the window's answer, by the end of its uid.
fn candidate(app: &App, end: &str) -> (Option<String>, String) {
    let a = app
        .gis
        .features_reconcile
        .answer
        .clone()
        .expect("an answer")
        .expect("reconciled");
    let c = a
        .candidates
        .iter()
        .find(|c| c.uid.ends_with(end))
        .expect("the candidate");
    (c.class.clone(), c.status.clone())
}

#[test]
fn the_recorded_decisions_are_taken_again_with_the_switch() {
    let mut app = opened();
    let _ = app.update(Message::Run("map.featuresReconcile"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Gis(gis::GisDialogKind::FeaturesReconcile))
    );
    // Without the switch the new candidates are read afresh.
    let a = app
        .gis
        .features_reconcile
        .answer
        .clone()
        .expect("an answer")
        .expect("reconciled");
    assert!(a.reapplied.is_empty());
    assert_eq!(candidate(&app, "12"), (Some("agac".into()), "member".into()));
    assert_eq!(candidate(&app, "13").1, "unknown");
    let _ = app.update(Message::Gis(gis::Event::FeaturesReconcile(R::Reapply(
        true,
    ))));
    let a = app
        .gis
        .features_reconcile
        .answer
        .clone()
        .expect("an answer")
        .expect("reconciled");
    let taken: Vec<(String, u32)> = a
        .reapplied
        .iter()
        .map(|r| (r.candidate[r.candidate.len() - 2..].to_owned(), r.note))
        .collect();
    assert_eq!(
        taken,
        [
            ("12".into(), 2),
            ("13".into(), 4),
            ("14".into(), 3),
            ("15".into(), 1)
        ]
    );
    assert_eq!(
        candidate(&app, "12"),
        (Some("direk".into()), "member".into()),
        "corrected to a pole again"
    );
    assert_eq!(candidate(&app, "13").1, "rejected", "rejected again");
    assert_eq!(candidate(&app, "16").1, "unknown", "the far one is read afresh");
    let pole = a.groups.iter().find(|g| g.candidates.iter().any(|c| c.ends_with("12"))).expect("the pole");
    assert_eq!(pole.status, "existing", "the pole written then");
    let door = a.groups.iter().find(|g| g.class.as_deref() == Some("kapi")).expect("the door");
    assert!(door.missing.is_empty(), "its number given again");
    // Eşleştir: the notes name the earlier ones; one undo step.
    let notes = |app: &App| app.document.as_ref().map(|d| d.model.settings().review_notes.clone()).unwrap_or_default();
    let before = notes(&app).len();
    let _ = app.update(Message::Gis(gis::Event::FeaturesReconcile(R::Run)));
    let after = notes(&app);
    let said: Vec<&str> = after[before..].iter().map(|n| n.text.as_str()).collect();
    assert_eq!(
        said,
        [
            "Detay eşleştirme, Ortofoto adayı: sınıfı “Direk” olarak düzeltildi. Önceki karar (not 2) yeniden uygulandı.",
            "Detay eşleştirme, LiDAR adayı: reddedildi (Gölge; nesne yok). Önceki karar (not 4) yeniden uygulandı.",
            "Detay eşleştirme, LiDAR adayı: alanları verildi (KapiNo = 14). Önceki karar (not 3) yeniden uygulandı.",
            "Detay eşleştirme, Ortofoto adayı: kabul edildi. Önceki karar (not 1) yeniden uygulandı."
        ]
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(notes(&app).len(), before, "one undo step");
}

/// `.run/gui-evidence/onceki-kararlar-4540/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib features_reconcile_reapply_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/onceki-kararlar-4540");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("map.featuresReconcile"));
        let _ = app.update(Message::Gis(gis::Event::FeaturesReconcile(R::Reviewer(
            "Ayşe Demir".into(),
        ))));
        let _ = app.update(Message::Gis(gis::Event::FeaturesReconcile(R::Reapply(
            true,
        ))));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-onceki-kararlar-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
