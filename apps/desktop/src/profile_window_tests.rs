//! Boykesit (profile_window.rs, plan/road.md Y4): the window writes through `road.profiles`, each write one undo step;
//! and its pictures.

use crate::app::{App, Dialog, Message};
use crate::profile_window::Event as W;

/// A road project with `road.profiles.query`'s drawing: 120 points with heights, two surfaces, the main alignment, a
/// side one and a parallel axis; its profiles kept, or none.
fn app(profiles: bool) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.profiles.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    if !profiles {
        setup["settings"]["road"].as_object_mut().expect("road").remove("profiles");
    }
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Profiles(e));
}

fn profiles(app: &App) -> Vec<kentos_commands::ProfileDef> {
    app.document.as_ref().expect("a drawing").model.road_profiles().to_vec()
}

/// Boykesit oluştur: on the main alignment, its ends on the terrain, a parabola and a circle between.
fn made(app: &mut App) {
    let _ = app.update(Message::Run("profile.new"));
    assert_eq!(app.dialog, Some(Dialog::Profiles));
    send(app, W::FromGround);
    send(app, W::Add);
    send(app, W::Pvi(1, 1, "200".into()));
    send(app, W::Pvi(1, 2, "100".into()));
    send(app, W::Curve(1, 1));
    send(app, W::Pvi(1, 3, "120".into()));
    send(app, W::Add);
    send(app, W::Pvi(2, 1, "420".into()));
    send(app, W::Pvi(2, 2, "112".into()));
    send(app, W::Curve(2, 3));
    send(app, W::Pvi(2, 3, "1500".into()));
}

#[test]
fn the_window_makes_a_profile_on_the_terrain_and_rebinds_a_stale_one() {
    let mut app = app(false);
    made(&mut app);
    let w = app.profile_window.as_ref().expect("open");
    assert!(w.ground.length > 600.0, "the alignment's terrain is read");
    send(&mut app, W::Save);
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(app.profile_window.is_none(), "Kaydet closes the window: {said:?}");
    let list = profiles(&app);
    assert_eq!(list.len(), 1);
    let p = &list[0];
    assert_eq!((p.id.as_str(), p.name.as_str(), p.alignment.as_str(), p.surface.as_deref()), ("b-1", "Proje kotu", "g-1", Some("t-1")));
    assert_eq!(p.pvis.len(), 4);
    assert_eq!((p.pvis[0].station, p.pvis[1].length, p.pvis[2].radius), (0.0, 120.0, 1500.0));
    assert!(p.pvis[3].station > 669.0, "the last PVI at the alignment's end");
    let doc = &mut app.document.as_mut().expect("doc").model;
    assert_eq!(doc.undo().as_deref(), Some("Boykesit oluştur"));

    // A stale profile: Aksa yeniden bağla takes the alignment's present revision, one undo step.
    let mut app = self::app(true);
    let _ = app.update(Message::Run("profile.editor"));
    send(&mut app, W::Choose(1));
    send(&mut app, W::Rebind);
    assert_eq!(profiles(&app)[1].alignment_revision, 3);
    send(&mut app, W::Cancel);
    let doc = &mut app.document.as_mut().expect("doc").model;
    assert_eq!(doc.undo().as_deref(), Some("Boykesiti düzenle"));

    // Çizime yaz: the chosen profile drawn as objects on the layer, one undo step.
    let mut app = self::app(true);
    let before = app.document.as_ref().expect("doc").model.entities().count();
    let _ = app.update(Message::Run("profile.editor"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::DrawLayer("0".into()));
    send(&mut app, W::Place("0; -300".into()));
    send(&mut app, W::Draw);
    let doc = &mut app.document.as_mut().expect("doc").model;
    assert!(doc.entities().count() > before + 50, "the frame, grid, lines and bands are drawn");
    assert_eq!(doc.undo().as_deref(), Some("Boykesit çiz"));
}

#[test]
fn curves_that_overlap_are_refused_and_the_window_stays() {
    let mut app = app(false);
    made(&mut app);
    send(&mut app, W::Pvi(1, 3, "400".into()));
    send(&mut app, W::Save);
    assert!(app.profile_window.is_some(), "a refused write leaves the window open");
    assert!(profiles(&app).is_empty());
}

/// The window over the drawing, wide and narrow, light and dark; `.run/gui-evidence/yol-y4-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib profile_window_tests::screens -- --ignored --nocapture
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
        let mut app = app(true);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("alignment"));
        let _ = app.update(Message::Run("profile.editor"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::PviChoose(2));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("yol-y4-boykesit-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        // The stale profile on the side alignment.
        send(&mut app, W::Choose(1));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("yol-y4-boykesit-eski-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        // Çizime yaz: the profile drawn under the alignments, the window closed over it.
        if name == "acik-genis" {
            send(&mut app, W::Choose(0));
            send(&mut app, W::DrawLayer("0".into()));
            send(&mut app, W::Draw);
            send(&mut app, W::Cancel);
            let _ = app.update(Message::Run("view.zoomExtents"));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join("yol-y4-boykesit-cizimde-acik-genis.png");
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
            // Closer: the first 230 m, its bands and the first curve.
            app.zoom_to_bounds(&kentos_commands::Bounds { min_x: 0.0, min_y: -320.0, max_x: 250.0, max_y: -120.0 });
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join("yol-y4-boykesit-bantlar-acik-genis.png");
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
            let _ = app.update(Message::Run("profile.editor"));
        }
        // A curve too long: the window says why before it is saved.
        send(&mut app, W::Choose(0));
        send(&mut app, W::Pvi(1, 3, "400".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("yol-y4-boykesit-binme-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
