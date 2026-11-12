//! 3B görünüm (scene3d_window.rs, plan/road.md Y12.3d): the window opens on the road project's scene cut into tiles,
//! its corridors' triangles by role; the camera follows the view buttons, a drag, the wheel and the exaggeration; and
//! its pictures.

use crate::app::{App, Dialog, Message};
use crate::scene3d_window::{Event as W, VIEWS};

/// A road project with `file.export.gltf`'s drawing: a terrain and two corridors.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/file.export.gltf.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Scene3d(e));
}

#[test]
fn the_window_opens_on_the_scene_and_its_camera_follows() {
    let mut app = app();
    let _ = app.update(Message::Run("scene3d.view"));
    assert_eq!(app.dialog, Some(Dialog::Scene3d));
    let w = app.scene3d_window.as_ref().expect("the window");
    // The terrain cut by the two corridors (610 + 97 triangles).
    assert!(w.scene_triangles() > 707);
    assert!(w.scene_parts().contains(&1) && w.scene_parts().contains(&4), "lanes and fill slopes: {:?}", w.scene_parts());
    // The terrain is cut by the corridors; uncut it is the 928 triangles file.export.gltf writes.
    assert!(w.cut);
    send(&mut app, W::Cut);
    assert_eq!(app.scene3d_window.as_ref().expect("the window").scene_triangles(), 928);
    send(&mut app, W::Cut);
    let w = app.scene3d_window.as_ref().expect("the window");
    let before = (w.azimuth, w.elevation, w.distance);
    send(&mut app, W::Orbit(50.0, 20.0));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert!((w.azimuth - (before.0 - 20.0)).abs() < 1e-9 && (w.elevation - (before.1 + 6.0)).abs() < 1e-9);
    send(&mut app, W::Dolly(2.0));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert!(w.distance < before.2);
    send(&mut app, W::View(3));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!((w.azimuth, w.elevation), (VIEWS[3].1, VIEWS[3].2));
    // The exaggeration is the view's: the camera's target is shown higher, its real height the same.
    let real = w.target;
    let low = w.camera().target[2];
    send(&mut app, W::Exaggeration("6".into()));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!(w.target, real);
    assert!(w.camera().target[2] > low);
    send(&mut app, W::Exaggeration("yok".into()));
    assert_eq!(app.scene3d_window.as_ref().expect("the window").exaggeration().factor, 1.0);
    // A click in the middle of the view from above picks the terrain or a corridor, a point inside the scene.
    send(&mut app, W::View(3));
    send(&mut app, W::Pick(600.0, 300.0, 1200.0, 600.0));
    let w = app.scene3d_window.as_ref().expect("the window");
    let p = w.picked.clone().expect("a point");
    assert!(p.point[0] > 0.0 && p.point[0] < 600.0 && p.point[1] > 0.0 && p.point[1] < 400.0, "{p:?}");
    assert!(p.point[2] > 90.0 && p.point[2] < 130.0, "{p:?}");
    send(&mut app, W::Pick(2.0, 2.0, 1200.0, 600.0));
    assert!(app.scene3d_window.as_ref().expect("the window").picked.is_none(), "the sky picks nothing");
    send(&mut app, W::Close);
    assert!(app.scene3d_window.is_none());
}

#[test]
fn a_map_project_has_no_3d_view() {
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Run("scene3d.view"));
    assert!(app.scene3d_window.is_none());
}

/// The window, wide and light, narrow and dark, from four sides, the terrain see-through; the pictures are the
/// renderer's own (KENTOS_SNAPSHOT_BACKEND=wgpu); `.run/gui-evidence/yol-3b-gorunum-*`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-desktop --lib scene3d_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            // A second frame: the first uploads, the second draws what is resident (the same frame selects again).
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("yol-3b-gorunum-{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("scene3d.view"));
        shot(&mut app, "kuzeydogu");
        let seen = *app.scene3d_window.as_ref().expect("the window").seen.lock().expect("seen");
        assert!(seen.tiles > 0 && seen.triangles > 0 && seen.waiting == 0 && seen.resident > 0, "{seen:?}");
        if name == "acik-genis" {
            send(&mut app, W::View(1));
            shot(&mut app, "guneybati");
            send(&mut app, W::View(3));
            shot(&mut app, "ust");
            send(&mut app, W::View(0));
            send(&mut app, W::Dolly(4.0));
            send(&mut app, W::SeeThrough);
            shot(&mut app, "yakin-saydam");
            send(&mut app, W::SeeThrough);
            send(&mut app, W::Cut);
            shot(&mut app, "yakin-kesilmemis");
            send(&mut app, W::Cut);
            send(&mut app, W::Flat);
            shot(&mut app, "yakin-duz-yuzler");
            send(&mut app, W::Flat);
            send(&mut app, W::View(0));
            // A click on a lane: its point and what it is on (the ray is the same viewport's, so it meets the lane
            // unless the ground in a cut hides it: the first lane seen).
            let vp = kentos_scene3d::Viewport { width: 1240.0, height: 560.0 };
            let mut on_lane = false;
            for n in (30..400).step_by(11) {
                let w = app.scene3d_window.as_ref().expect("the window");
                let (cam, ex) = (w.camera(), w.exaggeration());
                let Some(lane) = w.point_on(1, n) else { break };
                let Some((x, y, _)) = cam.project(ex.shown(lane), &vp) else { continue };
                send(&mut app, W::Pick(x as f32, y as f32, 1240.0, 560.0));
                let p = app.scene3d_window.as_ref().expect("the window").picked.clone();
                if p.is_some_and(|p| p.what.contains("Şerit") && (p.point[0] - lane[0]).hypot(p.point[1] - lane[1]) < 1e-3) {
                    on_lane = true;
                    break;
                }
            }
            assert!(on_lane, "a lane is seen and picked to the millimetre");
            shot(&mut app, "secilen-nokta");
            // Full screen: the scene takes the room, the buttons stay at the foot.
            use kentos_ui::widget::sizable::sizes;
            sizes::set(crate::scene3d_window::TITLE, sizes::Kept { width: 1180.0, height: 700.0, maximized: true });
            shot(&mut app, "tam-ekran");
            sizes::forget(crate::scene3d_window::TITLE);
        }
    }
}
