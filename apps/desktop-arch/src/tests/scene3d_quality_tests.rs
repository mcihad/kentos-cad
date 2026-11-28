//! 3B görünüm kalitesi (docs/adr/8120; Uygulama ayarları › Çizim motoru › 3B görünüm): the setting's three levels reach
//! the 3D views (the building's viewer, IFC modeli, the road's scene), Kaliteli by default. `screens` draws the section
//! building at each level and the setting in its section, for the owner (`.run/gui-evidence/3b-kalite-8120/`):
//!
//! ```text
//! KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_quality_tests::screens -- --ignored --nocapture
//! ```

use kentos_model_view_ui::scene::Quality;
use serde_json::Value;

use crate::app::{App, Message};
use crate::scene3d_building_tests::{building, open};

fn choose(app: &mut App, value: &str) {
    let _ = app
        .settings
        .choose(&[("graphics.scene3dQuality", Value::from(value))]);
    app.apply_settings();
}

#[test]
fn the_quality_follows_the_setting_kaliteli_by_default() {
    let (mut app, _) = App::boot(None);
    assert_eq!(
        app.scene3d_quality(),
        Quality::High,
        "Kaliteli unless asked"
    );
    for (value, quality) in [
        ("fast", Quality::Fast),
        ("balanced", Quality::Balanced),
        ("high", Quality::High),
    ] {
        choose(&mut app, value);
        assert_eq!(app.scene3d_quality(), quality, "{value}");
    }
    // The schema offers the three by their Turkish names.
    let d = crate::settings::schema()
        .get("graphics.scene3dQuality")
        .expect("the setting");
    let names: Vec<&str> = d.choices.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(names, ["Hızlı", "Dengeli", "Kaliteli"]);
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/3b-kalite-8120");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    for (theme, size, tag) in [
        ("light", Size::new(1440.0, 900.0), "acik"),
        ("dark", Size::new(1100.0, 700.0), "koyu"),
    ] {
        for (value, name) in [
            ("fast", "hizli"),
            ("balanced", "dengeli"),
            ("high", "kaliteli"),
        ] {
            let mut app = open(building());
            let _ = app
                .settings
                .choose(&[("appearance.theme", Value::from(theme))]);
            choose(&mut app, value);
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("scene3d.view"));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            snapshot.settle(&mut app, App::scene3d_shown, &mut update);
            snapshot.settle(&mut app, App::scene3d_shown, &mut update);
            let file = out.join(format!("bina-{name}-{tag}.png"));
            snapshot
                .render(app.scene3d_shown(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
    // Ölç in 3B görünüm (docs/adr/8121): the roof's top face, one of its sides and the angle at its end; then both side
    // panes folded, the card to its head.
    let size = Size::new(1440.0, 900.0);
    let mut app = open(building());
    let _ = app
        .settings
        .choose(&[("appearance.theme", Value::from("light"))]);
    app.apply_settings();
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("scene3d.view"));
    measured_roof(&mut app);
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    snapshot.settle(&mut app, App::scene3d_shown, &mut update);
    snapshot.settle(&mut app, App::scene3d_shown, &mut update);
    let file = out.join("bina-olcum-acik.png");
    snapshot
        .render(app.scene3d_shown(), &app.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());
    use kentos_model_view_ui::{Message as V, Side};
    for m in [V::Pane(Side::Tree), V::Pane(Side::Details), V::MeasureFold] {
        let _ = app.update(Message::Scene3d(crate::scene3d_window::Event::Viewer(m)));
    }
    snapshot.settle(&mut app, App::scene3d_shown, &mut update);
    let file = out.join("bina-paneller-kapali-acik.png");
    snapshot
        .render(app.scene3d_shown(), &app.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());

    // The setting in Uygulama ayarları › Çizim motoru.
    let size = Size::new(1440.0, 900.0);
    let mut app = open(building());
    let _ = app
        .settings
        .choose(&[("appearance.theme", Value::from("light"))]);
    app.apply_settings();
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let _ = app.update(Message::WindowResized(size));
    let _ = app.run("tools.options");
    let _ = app.update(Message::Settings(crate::settings_view::Edit::Section(
        crate::settings_sections::Section::Engine,
    )));
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("ayar-3b-kalite-acik.png");
    snapshot
        .render(app.view(), &app.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{}", file.display());
}

/// Ölç on in 3B görünüm's viewer and the roof measured: the highest face that looks up whole, one of its sides and the
/// angle at that side's end.
fn measured_roof(app: &mut App) {
    use kentos_model_view::measure;
    use kentos_model_view_ui::{Measured, Message as V};
    let _ = app.update(Message::Scene3d(crate::scene3d_window::Event::Viewer(
        V::Measure,
    )));
    let b = app
        .scene3d_window
        .as_mut()
        .and_then(|w| w.building.as_mut())
        .expect("a building");
    let scene = b.viewer.scene().clone();
    let mut top: Option<(f64, usize, usize)> = None;
    for (m, s) in scene.meshes().iter().enumerate() {
        for (t, tri) in s.triangles.iter().enumerate() {
            let [p, q, r] = tri.map(|k| s.vertices[k as usize]);
            let (u, v) = (
                [q[0] - p[0], q[1] - p[1], q[2] - p[2]],
                [r[0] - p[0], r[1] - p[1], r[2] - p[2]],
            );
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 0.0 && (n[2] / len).abs() > 0.999 && top.is_none_or(|(z, _, _)| p[2] > z) {
                top = Some((p[2], m, t));
            }
        }
    }
    let (_, mesh, triangle) = top.expect("a face looking up");
    let s = &scene.meshes()[mesh];
    let face = measure::face(&s.vertices, &s.triangles, triangle).expect("the roof's face");
    let [a, c] = face.outline[0];
    let next = face
        .outline
        .iter()
        .find(|[p, q]| (*p == c || *q == c) && !(*p == a || *q == a))
        .expect("the next side");
    let d = if next[0] == c { next[1] } else { next[0] };
    let at = face
        .outline
        .iter()
        .flatten()
        .fold([0.0; 3], |acc, p| {
            [acc[0] + p[0], acc[1] + p[1], acc[2] + p[2]]
        })
        .map(|v| v / (2 * face.outline.len()) as f64);
    let m = &mut b.viewer.measuring;
    m.done.push(Measured::Area {
        face,
        what: s.name.clone(),
        at,
    });
    m.done.push(Measured::Distance { a, b: c });
    m.done.push(Measured::Angle { a, at: c, b: d });
}

/// A drawing's 3D bodies by size (a probe run by hand: `KENTOS_PROBE_KCAD=<file>`), with the IFC written beside it.
#[test]
#[ignore = "a probe of a drawing given by hand"]
fn probe_bodies() {
    let Ok(path) = std::env::var("KENTOS_PROBE_KCAD") else {
        return;
    };
    let snapshot = kentos_kcad::decode(&std::fs::read(&path).expect("the file")).expect("decodes");
    let mut app = open(snapshot);
    let doc = &app.document.as_ref().expect("the drawing").model;
    let (building, _) = kentos_native_application::arch_ifc::building(doc, "", "KentOS Mimari");
    let out = std::env::temp_dir().join("kentos-probe.ifc");
    std::fs::write(&out, kentos_ifc::building::write(&building)).expect("writes");
    println!("IFC: {}", out.display());
    let _ = app.update(Message::Run("scene3d.view"));
    let b = app
        .scene3d_window
        .as_ref()
        .and_then(|w| w.building.as_ref())
        .expect("a building");
    let v = b.viewer.view().expect("the model");
    let input = v.input();
    let mut rows: Vec<(f64, String)> = input
        .bodies
        .iter()
        .map(|body| {
            let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
            for p in body.positions.iter() {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
            let size = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
            let o = &input.objects[body.object];
            let storey = o
                .storey
                .map_or(String::new(), |s| input.objects[s].label.clone());
            (
                size[0] * size[1] * size[2],
                format!(
                    "{:>10.2} m³  {:?} {} “{}” [{}] z {:.2}–{:.2}  {:.2} × {:.2} × {:.2}  {} üçgen",
                    size[0] * size[1] * size[2],
                    o.category,
                    o.class,
                    o.label,
                    storey,
                    lo[2],
                    hi[2],
                    size[0],
                    size[1],
                    size[2],
                    body.triangles.len()
                ),
            )
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (_, r) in rows.iter().take(12) {
        println!("{r}");
    }
    // The see-through volumes' flat faces: the sum of their triangles' areas against the face's own (shoelace of its
    // outline, the outline from the edges only one triangle has): more means triangles over each other.
    for body in &input.bodies {
        let o = &input.objects[body.object];
        if o.category != kentos_model_view::Category::Space {
            continue;
        }
        let p = &body.positions;
        let (mut top, mut zmax, mut zmin) = (0.0f64, f64::MIN, f64::MAX);
        for v in p.iter() {
            zmax = zmax.max(v[2]);
            zmin = zmin.min(v[2]);
        }
        let mut bottom = 0.0f64;
        for t in &body.triangles {
            let [a, b, c] = t.map(|k| p[k as usize]);
            let area = 0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs();
            if [a, b, c].iter().all(|v| (v[2] - zmax).abs() < 1e-6) {
                top += area;
            }
            if [a, b, c].iter().all(|v| (v[2] - zmin).abs() < 1e-6) {
                bottom += area;
            }
        }
        println!(
            "{} “{}”: alt yüz üçgenleri {bottom:.3} m², üst yüz üçgenleri {top:.3} m², {} üçgen",
            o.class,
            o.label,
            body.triangles.len()
        );
    }
    // Pictures: the north-east view, a level cut at 2 m and at 3.6 m.
    use iced::Size;
    use kentos_model_view_ui::Message as V;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(
        std::env::var("KENTOS_PROBE_OUT")
            .unwrap_or_else(|_| std::env::temp_dir().display().to_string()),
    );
    let size = Size::new(1440.0, 900.0);
    let _ = app.update(Message::WindowResized(size));
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut shot = |app: &mut App, name: &str| {
        snapshot.settle(app, App::scene3d_shown, &mut update);
        snapshot.settle(app, App::scene3d_shown, &mut update);
        let file = out.join(format!("{name}.png"));
        snapshot
            .render(app.scene3d_shown(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    };
    shot(&mut app, "probe-kuzeydogu");
    for (at, name) in [(2.0, "probe-kesit-2m"), (3.6, "probe-kesit-3-6m")] {
        let _ = app.update(Message::Scene3d(crate::scene3d_window::Event::Viewer(
            V::Cut(kentos_model_view::Cut::Level),
        )));
        let _ = app.update(Message::Scene3d(crate::scene3d_window::Event::Viewer(
            V::At(at),
        )));
        shot(&mut app, name);
    }
}
