//! Güzergâh (alignment_window.rs, plan/road.md Y3): the window writes through `road.alignments`, each write one undo
//! step; and its pictures.

use kentos_domain::Slot;

use crate::alignment_window::Event as W;
use crate::app::{App, Dialog, Message};

/// A road project whose drawing holds the PI line of the alignment commands' main alignment (its first object), with
/// `alignments` as the project's.
fn app(alignments: serde_json::Value) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.alignments.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    if alignments.as_array().is_some_and(|a| a.is_empty()) {
        setup["settings"].as_object_mut().expect("settings").remove("road");
    } else {
        setup["settings"]["road"] = serde_json::json!({ "alignments": alignments });
    }
    let main = &cases["setup"]["settings"]["road"]["alignments"][0]["pis"];
    let pts: Vec<serde_json::Value> =
        main.as_array().expect("pis").iter().map(|p| serde_json::json!({"x": p["x"], "y": p["y"]})).collect();
    setup["entities"] = serde_json::json!([{"kind": "polyline", "id": 1, "layerId": "0", "attrs": {}, "pts": pts}]);
    setup["uids"] = serde_json::json!(["0192f5a0-7c3e-7d4a-9b1e-000000000001"]);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Alignments(e));
}

fn alignments(app: &App) -> Vec<kentos_commands::AlignmentDef> {
    app.document.as_ref().expect("a drawing").model.road_alignments().to_vec()
}

/// Yeni aks with the PI line selected: its vertices are the PIs; the middle one gets its curve.
fn made(app: &mut App) {
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("alignment.new"));
    assert_eq!(app.dialog, Some(Dialog::Alignments));
    send(app, W::Start("2000".into()));
    send(app, W::Pi(1, 2, "400".into()));
    send(app, W::Pi(1, 3, "80".into()));
    send(app, W::Pi(1, 4, "80".into()));
    send(app, W::Save);
}

#[test]
fn the_window_makes_an_alignment_and_its_parallel_each_one_undo_step() {
    let mut app = app(serde_json::json!([]));
    made(&mut app);
    assert!(app.alignment_window.is_none(), "Kaydet closes the window");
    let list = alignments(&app);
    assert_eq!(list.len(), 1);
    let a = &list[0];
    assert_eq!((a.id.as_str(), a.name.as_str(), a.revision), ("g-1", "Ana aks", 1));
    assert_eq!(a.pis.len(), 3);
    assert_eq!((a.pis[1].id.as_str(), a.pis[1].radius, a.pis[1].spiral_in), ("p1", 400.0, 80.0));
    assert_eq!(a.stationing.as_ref().map(|s| s.start), Some(2000.0));
    // A parallel axis of the chosen alignment, 7,5 m right unless told.
    let _ = app.update(Message::Run("alignment.editor"));
    send(&mut app, W::Parallel);
    send(&mut app, W::Offset("-7,5".into()));
    send(&mut app, W::Save);
    let list = alignments(&app);
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].offset_of.as_ref().map(|o| o.offset), Some(-7.5));
    // The window asks a point's km on the saved alignment.
    let _ = app.update(Message::Run("alignment.editor"));
    send(&mut app, W::Point("485520; 4420030".into()));
    let answer = app.alignment_window.as_ref().expect("open").answer.clone();
    assert!(answer.starts_with("km 2+525,205 · sol 19,496 m"), "{answer}");
    // Km'yi çizime yaz: the km's ticks and words on the layer, one undo step.
    let before = app.document.as_ref().expect("doc").model.entities().count();
    send(&mut app, W::Choose(0));
    send(&mut app, W::LabelLayer("0".into()));
    send(&mut app, W::Label);
    let doc = &mut app.document.as_mut().expect("doc").model;
    assert!(doc.entities().count() > before + 20, "the ticks and words are drawn");
    assert_eq!(doc.undo().as_deref(), Some("Aks km yazısı"));
    // Removing the source while the parallel follows it is refused in the window.
    send(&mut app, W::Choose(0));
    send(&mut app, W::Remove);
    assert_eq!(app.alignment_window.as_ref().map(|w| w.rows.len()), Some(2));
    // Ters çevir: one step; the parallel changes sides.
    send(&mut app, W::Reverse);
    let list = alignments(&app);
    assert_eq!(list[0].pis[0].id, "b");
    assert_eq!(list[1].offset_of.as_ref().map(|o| o.offset), Some(7.5));
    send(&mut app, W::Cancel);
    let doc = &mut app.document.as_mut().expect("doc").model;
    assert_eq!(doc.undo().as_deref(), Some("Aksı düzenle"));
    assert_eq!(doc.undo().as_deref(), Some("Aks oluştur"));
    assert_eq!(doc.road_alignments().len(), 1);
}

#[test]
fn a_curve_that_overlaps_says_so_before_it_is_saved() {
    let mut app = app(serde_json::json!([]));
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("alignment.new"));
    send(&mut app, W::Pi(1, 2, "4000".into()));
    send(&mut app, W::Save);
    assert!(app.alignment_window.is_some(), "a refused write leaves the window open");
    assert!(alignments(&app).is_empty());
}

/// The alignment over the drawing and the window, wide and narrow, light and dark; `.run/gui-evidence/y3-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib alignment_window_tests::screens -- --ignored --nocapture
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
        let mut app = app(serde_json::json!([]));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        made(&mut app);
        let _ = app.update(Message::Run("alignment.editor"));
        send(&mut app, W::Parallel);
        send(&mut app, W::Offset("-7,5".into()));
        send(&mut app, W::Save);
        let _ = app.update(Message::Run("alignment.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::EquationAdd);
        send(&mut app, W::Back(0, "2400".into()));
        send(&mut app, W::Ahead(0, "2450".into()));
        send(&mut app, W::Save);
        app.selection.clear();
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y3-aks-cizimde-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::Run("alignment.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Point("485520; 4420030".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y3-guzergah-penceresi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        // The km written on the drawing, then the window closed over it.
        send(&mut app, W::Point(String::new()));
        send(&mut app, W::LabelLayer("km".into()));
        send(&mut app, W::Label);
        send(&mut app, W::Cancel);
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y3-km-yazisi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::Run("alignment.editor"));
        send(&mut app, W::Choose(0));
        // A radius too large: the window says why before it is saved.
        send(&mut app, W::Pi(1, 2, "4000".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y3-guzergah-binme-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
