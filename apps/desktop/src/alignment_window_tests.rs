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

/// `road.alignments.impact`'s drawing: profiles, section sets, corridors and superelevation on the main alignment, the
/// main volume and haul plan approved (docs/adr/0963).
fn impact_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.alignments.impact.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Aks değişikliğinin etkisi (Y57, docs/adr/0963; ROAD-ALIGN-10): the curve's radius typed 180, the window reads what
/// saving it would do as the shared case does; typed back, nothing.
#[test]
fn typing_a_radius_tells_what_moves_on_the_stations() {
    let mut app = impact_app();
    let _ = app.update(Message::Run("alignment.editor"));
    send(&mut app, W::Choose(0));
    assert!(app.alignment_window.as_ref().expect("open").impact.is_none(), "nothing typed yet");
    send(&mut app, W::Pi(1, 2, "180".into()));
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.alignments.impact.json")).expect("cases");
    let want = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "Yarıçap 150'den 180'e").expect("the case")["steps"][0]
        ["result"]["output"]
        .clone();
    let Some(Ok((imp, warnings))) = app.alignment_window.as_ref().expect("open").impact.clone() else { panic!("the impact reads") };
    assert_eq!(imp.removed.len(), want["removed"].as_array().expect("removed").len());
    assert!((imp.removed[0].from - want["removed"][0]["from"].as_f64().expect("from")).abs() < 1e-6);
    assert_eq!(imp.refs.len(), want["refs"].as_array().expect("refs").len());
    assert!(warnings.iter().any(|w| w.code == "alignment_impact_approved"));
    send(&mut app, W::Pi(1, 2, "150".into()));
    assert!(app.alignment_window.as_ref().expect("open").impact.is_none());
}

/// The impact under the alignment's form, wide and narrow, light and dark; `.run/gui-evidence/yol-y57/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib alignment_window_tests::screens_impact -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_impact() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y57");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = impact_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y57-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("alignment.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Pi(1, 2, "180".into()));
        shot(&mut app, "yaricap");
        send(&mut app, W::Pi(1, 2, "150".into()));
        send(&mut app, W::Pi(0, 0, "0".into()));
        send(&mut app, W::Pi(0, 1, "49,2857142857143".into()));
        shot(&mut app, "baslangic");
    }
}

/// `road.alignments.edit`'s drawing with superelevation schedules on the main alignment (docs/adr/0964).
fn reverse_app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.alignments.edit.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "Ters çevrilir: dever çizelgeleri de döner").expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Ters çevir carries the schedules (Y58, docs/adr/0964; ROAD-DEVER-03): one undo step, said by name; the rows read
/// from the new start, their sides swapped, the pivot on the other edge.
#[test]
fn reversing_carries_the_superelevation_schedules() {
    let mut app = reverse_app();
    let _ = app.update(Message::Run("alignment.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Reverse);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let d1 = doc.road_superelevations().iter().find(|d| d.id == "d-1").expect("d-1");
    assert_eq!((d1.normal_left, d1.normal_right), (-3.0, -2.5));
    assert_eq!(d1.pivot, Some(kentos_commands::SuperelevationPivot::RightEdge));
    let kentos_commands::SuperelevationMethod::Stations { rows } = &d1.method else { panic!("rows") };
    assert_eq!((rows[0].left, rows[0].right), (-6.0, 6.0));
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    assert_eq!(doc.undo().as_deref(), Some("Aksı düzenle"), "one step undoes both");
    let d1 = doc.road_superelevations().iter().find(|d| d.id == "d-1").expect("d-1");
    assert_eq!((d1.normal_left, d1.normal_right), (-2.5, -3.0), "undone together");
}

/// The schedule's diagram before and after the alignment is turned round, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y58/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib alignment_window_tests::screens_reverse -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_reverse() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y58");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = reverse_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y58-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        for (k, label) in [(0usize, "satirli"), (1, "kurplu")] {
            let _ = app.update(Message::Run("superelevation.editor"));
            let _ = app.update(Message::Superelevations(crate::superelevation_window::Event::Choose(k)));
            shot(&mut app, &format!("{label}-once"));
            let _ = app.update(Message::Superelevations(crate::superelevation_window::Event::Cancel));
        }
        let _ = app.update(Message::Run("alignment.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Reverse);
        shot(&mut app, "ters-cevrildi");
        let _ = app.update(Message::Alignments(crate::alignment_window::Event::Cancel));
        for (k, label) in [(0usize, "satirli"), (1, "kurplu")] {
            let _ = app.update(Message::Run("superelevation.editor"));
            let _ = app.update(Message::Superelevations(crate::superelevation_window::Event::Choose(k)));
            shot(&mut app, &format!("{label}-sonra"));
            let _ = app.update(Message::Superelevations(crate::superelevation_window::Event::Cancel));
        }
    }
}

/// Ters çevir with profiles, corridors, a culvert, a wall and a sheet set on the alignment (Y59, docs/adr/0965): the
/// profiles and schedules turn with it in one step, what does not turn is told by name.
#[test]
fn reversing_turns_the_profiles_and_tells_what_goes_stale() {
    let mut app = impact_app();
    let before: Vec<kentos_commands::ProfileDef> =
        app.document.as_ref().expect("a drawing").model.road_profiles().iter().filter(|p| p.alignment == "g-1").cloned().collect();
    let _ = app.update(Message::Run("alignment.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::Reverse);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let length = kentos_native_application::road_alignments::made_alignments(doc)
        .into_iter()
        .find(|(d, _)| d.id == "g-1")
        .and_then(|(_, b)| b.ok())
        .map(|b| b.core.length)
        .expect("builds");
    for p0 in &before {
        let p1 = doc.road_profiles().iter().find(|p| p.id == p0.id).expect("kept");
        let (first, last) = (&p0.pvis[0], &p1.pvis[p1.pvis.len() - 1]);
        assert!((last.station - (length - first.station)).abs() < 1e-9 && last.elevation == first.elevation, "{}", p0.id);
    }
    let said: String = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains("boykesiti") && said.contains("dever çizelgesi"), "{said}");
    assert!(said.contains("Ana koridor") && said.contains("Km 1+380 menfezi") && said.contains("Sağ istinat"), "{said}");
}

/// The reversal told, and the profile read from its new start, wide and narrow, light and dark;
/// `.run/gui-evidence/yol-y59/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib alignment_window_tests::screens_reverse_profile -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_reverse_profile() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yol-y59");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = impact_app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("y59-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("profile.editor"));
        shot(&mut app, "boykesit-once");
        let _ = app.update(Message::Profiles(crate::profile_window::Event::Cancel));
        let _ = app.update(Message::Run("alignment.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Reverse);
        shot(&mut app, "ters-cevrildi");
        let _ = app.update(Message::Alignments(crate::alignment_window::Event::Cancel));
        let _ = app.update(Message::Run("profile.editor"));
        shot(&mut app, "boykesit-sonra");
    }
}
