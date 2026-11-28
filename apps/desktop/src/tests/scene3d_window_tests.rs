//! 3B görünüm (scene3d_window.rs, plan/road.md Y12.3d): the window opens on the road project's scene cut into tiles,
//! its corridors' triangles by role; the camera follows the view buttons, a drag, the wheel, the exaggeration and the
//! north arrow; it always opens beside the plan in a window of its own and never comes over the drawing
//! (scene3d_view.rs, docs/adr/8060 §9, 8122); and its pictures.

use iced::keyboard::key::{Named, NativeCode, Physical};
use iced::keyboard::{Key, Modifiers};

use crate::app::{App, Message};
use crate::scene3d_window::{Event as W, TITLE, VIEWS};
use crate::windows::{Event as Windows, Surface};

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
    // Beside the plan, in a window of its own: nothing over the drawing.
    assert_eq!(app.dialog, None);
    assert!(app.surface_wanted(Surface::Scene3d) && app.windows.of(Surface::Scene3d).is_some());
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
    let before = (w.orbit.azimuth, w.orbit.elevation, w.orbit.distance);
    send(&mut app, W::Orbit(50.0, 20.0));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert!((w.orbit.azimuth - (before.0 - 20.0)).abs() < 1e-9 && (w.orbit.elevation - (before.1 + 6.0)).abs() < 1e-9);
    send(&mut app, W::Dolly(2.0));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert!(w.orbit.distance < before.2);
    send(&mut app, W::View(3));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!((w.orbit.azimuth, w.orbit.elevation), (VIEWS[3].1, VIEWS[3].2));
    // The exaggeration is the view's: the camera's target is shown higher, its real height the same.
    let real = w.orbit.target;
    let low = w.camera().target[2];
    send(&mut app, W::Exaggeration("6".into()));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!(w.orbit.target, real);
    assert!(w.camera().target[2] > low);
    send(&mut app, W::Exaggeration("yok".into()));
    assert_eq!(app.scene3d_window.as_ref().expect("the window").exaggeration().factor, 1.0);
    // The north arrow turns the camera north, its height kept; the legend folds and opens.
    send(&mut app, W::View(0));
    send(&mut app, W::North);
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!((w.orbit.azimuth, w.orbit.elevation), (-90.0, VIEWS[0].2));
    send(&mut app, W::Legend);
    assert!(!app.scene3d_window.as_ref().expect("the window").legend);
    send(&mut app, W::Legend);
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

/// Esc, as its own window's router hands it over.
fn escape() -> crate::keys::KeyPress {
    crate::keys::KeyPress {
        key: Key::Named(Named::Escape),
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers: Modifiers::empty(),
        text: None,
        repeat: false,
    }
}

/// Its own window, the platform having made it.
fn opened(app: &mut App) -> iced::window::Id {
    let id = app.windows.of(Surface::Scene3d).expect("its own window").id;
    let _ = app.update(Message::Windows(Windows::Opened(id)));
    id
}

#[test]
fn the_window_always_opens_beside_the_plan_never_over_the_drawing() {
    let mut app = app();
    let main = iced::window::Id::unique();
    app.windows.set_main(main);
    // A choice kept from before (over the drawing) is not followed: a 3D view is always apart (docs/adr/8122).
    app.windows.memory.set_apart(Surface::Scene3d, false);
    let _ = app.update(Message::Run("scene3d.view"));
    let id = opened(&mut app);
    // Its own window, named after it; the drawing free; a trace finds it.
    assert_eq!(app.dialog, None);
    assert!(app.is_apart(Surface::Scene3d));
    assert_eq!(app.window_title(id), "3B görünüm — KentOS CAD");
    assert_eq!(app.dialog_title().as_deref(), Some(TITLE));
    let settings = app.window_settings(Surface::Scene3d);
    assert!(!settings.exit_on_close_request, "its close is the window's Kapat");
    let _ = app.window_view(id);
    // Nothing offers to bring it over the drawing, and asking does nothing.
    assert!(app.dialog_control(crate::traces::Control::Press("Ana pencereye al")).is_err());
    let _ = app.update(Message::Windows(Windows::Apart(Surface::Scene3d, false)));
    assert_eq!(app.dialog, None);
    assert!(app.is_apart(Surface::Scene3d) && app.windows.of(Surface::Scene3d).is_some());
    // Opened again: beside the plan again.
    let _ = app.update(Message::Windows(Windows::Key(id, escape())));
    assert!(app.scene3d_window.is_none());
    let _ = app.update(Message::Run("scene3d.view"));
    assert_eq!(app.dialog, None);
    let id = opened(&mut app);
    // Esc in its own window closes it; so does its close button.
    let _ = app.update(Message::Windows(Windows::Key(id, escape())));
    assert!(app.scene3d_window.is_none());
    assert!(app.windows.of(Surface::Scene3d).is_none());
    let _ = app.update(Message::Run("scene3d.view"));
    let id = opened(&mut app);
    let _ = app.update(Message::Windows(Windows::CloseRequested(id)));
    assert!(app.scene3d_window.is_none());
    // The drawing closed while it is open beside the plan: it closes too.
    let _ = app.update(Message::Run("scene3d.view"));
    let _ = opened(&mut app);
    app.document = None;
    send(&mut app, W::Fit);
    assert!(app.scene3d_window.is_none());
    assert!(app.windows.of(Surface::Scene3d).is_none());
}

#[test]
fn a_map_project_has_no_3d_view() {
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Run("scene3d.view"));
    assert!(app.scene3d_window.is_none());
}

/// The window beside the plan, wide and light, narrow and dark, from four sides, the terrain see-through, a lane
/// clicked; then brought over the drawing, and made full screen there. The pictures are the renderer's own
/// (KENTOS_SNAPSHOT_BACKEND=wgpu); `.run/gui-evidence/3b-pencere/yol-*`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/3b-pencere");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    // Its own window's size, and the main window's behind it.
    for (theme, size, main, name) in [
        ("light", Size::new(1280.0, 800.0), Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(960.0, 600.0), Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(main));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::scene3d_shown, &mut update);
            // A second frame: the first uploads, the second draws what is resident (the same frame selects again).
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("yol-{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("corridor"));
        let _ = app.update(Message::Run("scene3d.view"));
        shot(&mut app, "1-kuzeydogu");
        let seen = *app.scene3d_window.as_ref().expect("the window").seen.lock().expect("seen");
        assert!(seen.tiles > 0 && seen.triangles > 0 && seen.waiting == 0 && seen.resident > 0, "{seen:?}");
        if name == "acik" {
            send(&mut app, W::View(1));
            shot(&mut app, "2-guneybati");
            send(&mut app, W::View(3));
            shot(&mut app, "3-ust");
            send(&mut app, W::View(0));
            send(&mut app, W::Dolly(4.0));
            send(&mut app, W::SeeThrough);
            shot(&mut app, "4-yakin-saydam");
            send(&mut app, W::SeeThrough);
            send(&mut app, W::Cut);
            send(&mut app, W::Flat);
            send(&mut app, W::Legend);
            shot(&mut app, "5-kesilmemis-duz-yuzler-lejant-kapali");
            send(&mut app, W::Legend);
            send(&mut app, W::Flat);
            send(&mut app, W::Cut);
        }
        send(&mut app, W::View(0));
        // A click on a lane: its point and what it is on (the ray is the same viewport's, so it meets the lane unless
        // the ground in a cut hides it: the first lane seen).
        let vp = kentos_scene3d::Viewport { width: 1240.0, height: 560.0 };
        let mut on_lane = false;
        for n in (30..400).step_by(11) {
            let w = app.scene3d_window.as_ref().expect("the window");
            let (cam, ex) = (w.camera(), w.exaggeration());
            let Some(lane) = w.point_on(1, n) else { break };
            let Some((x, y, _)) = cam.project(ex.shown(lane), &vp) else { continue };
            send(&mut app, W::Pick(x as f32, y as f32, 1240.0, 560.0));
            let p = app.scene3d_window.as_ref().expect("the window").picked.clone();
            if p.is_some_and(|p| p.what.contains("Şerit") && kentos_num::hypot(p.point[0] - lane[0], p.point[1] - lane[1]) < 1e-3) {
                on_lane = true;
                break;
            }
        }
        assert!(on_lane, "a lane is seen and picked to the millimetre");
        shot(&mut app, "6-secilen-nokta");
    }
}
