//! Pictures of the model viewer on its own, with no application around it (docs/adr/8061 §3.3 "Tek pencere örneği",
//! docs/adr/8073 §4): the section building's file as the example shows it, a wall chosen and its details read; the
//! same in the dark theme; cut by a level plane; a box about a storey; a real-world IFC 4.3 file; the reading and a
//! file refused. Drawn on the GPU (the scene is a shader); not run by default; the PNGs go to
//! `.run/gui-evidence/model-view-ui/` for the owner to look at:
//!
//! ```text
//! KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-model-view-ui --test screens -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};

use iced::{Element, Size, Theme};
use kentos_ifc::read::{self as ifc, Limits, Model};
use kentos_model_view::Cut;
use kentos_model_view_ui::scene::{Gesture, Quality};
use kentos_model_view_ui::{Host, Message, Out, Panel, Viewer, look};
use kentos_ui::theme::{self, Accent, Mode, Tokens};

struct Window {
    viewer: Viewer,
    file: Option<(Vec<u8>, Model)>,
    mode: Mode,
    quality: Quality,
}

impl Window {
    fn theme(&self) -> Theme {
        theme::theme(self.mode, Accent::Blue)
    }

    fn view(&self) -> Element<'_, Message> {
        let t = Tokens::of(&self.theme());
        self.viewer.layout(Host {
            to: |m| m,
            end: Vec::new(),
            sky: look::sky(t.window, Some(t.accent)),
            quality: self.quality,
            coordinate: Box::new(|v| format!("{v:.3}")),
            measure: Box::new(|v, unit| match look::digits(unit, 3, 2) {
                Some(d) => format!("{v:.d$}"),
                None => look::plain(v),
            }),
            east: "Y",
            north: "X",
        })
    }

    fn update(&mut self, m: Message) {
        if let Some(Out::Chosen(Some((o, _)))) = self.viewer.update(m)
            && let Some((bytes, model)) = &self.file
            && let Ok(d) = model.details(bytes, o)
        {
            self.viewer.set_details(o, d);
        }
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn opened(path: &str, mode: Mode) -> Window {
    let never = || false;
    let bytes = std::fs::read(root().join(path)).expect("the fixture");
    let model = ifc::read(&bytes, &Limits::default(), &never).expect("it reads");
    let mut bodies = model.bodies(&bytes, &never).expect("the bodies");
    let openings = model.opening_bodies(&bytes, &never).expect("the openings");
    bodies.bodies.extend(openings.bodies);
    bodies.not_shown.extend(openings.not_shown);
    let name = Path::new(path)
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let mut viewer = Viewer::new(name.clone());
    viewer.load(
        ifc::view::input(&name, &model, &bodies),
        vec![(model.summary.schema.clone(), "Dosyanın şeması.".to_owned())],
    );
    Window {
        viewer,
        file: Some((bytes, model)),
        mode,
        quality: Quality::High,
    }
}

fn shot(w: &mut Window, name: &str) {
    let out = root().join(".run/gui-evidence/model-view-ui");
    std::fs::create_dir_all(&out).expect("the folder");
    let mut snapshot =
        kentos_ui::snapshot::Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
    let mut update = |w: &mut Window, m: Message| w.update(m);
    snapshot.settle(w, Window::view, &mut update);
    for _ in 0..3 {
        let _ = snapshot.render(w.view(), &w.theme());
    }
    let file = out.join(format!("{name}.png"));
    snapshot
        .render(w.view(), &w.theme())
        .save(&file)
        .expect("writes the picture");
    println!("{} ({})", file.display(), snapshot.renderer_name());
}

/// Ölç on the section building: the highest face that looks up (the roof's top) measured whole, the length of one of
/// its sides and the angle at that side's end, and the pointer on the roof's farthest corner from it.
fn measured_roof(w: &mut Window) {
    use kentos_model_view::measure;
    use kentos_model_view_ui::Measured;
    w.update(Message::Measure);
    let scene = w.viewer.scene().clone();
    let mut top: Option<(f64, usize, usize)> = None;
    for (m, s) in scene.meshes().iter().enumerate() {
        for (t, tri) in s.triangles.iter().enumerate() {
            let [a, b, c] = tri.map(|k| s.vertices[k as usize]);
            let n = [
                (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]),
                (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]),
                (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]),
            ];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 0.0 && (n[2] / len).abs() > 0.999 && top.is_none_or(|(z, _, _)| a[2] > z) {
                top = Some((a[2], m, t));
            }
        }
    }
    let (_, mesh, triangle) = top.expect("a face looking up");
    let s = &scene.meshes()[mesh];
    let face = measure::face(&s.vertices, &s.triangles, triangle).expect("the roof's face");
    let [a, b] = face.outline[0];
    let next = face
        .outline
        .iter()
        .find(|[p, q]| (*p == b || *q == b) && !(*p == a || *q == a))
        .expect("the next side");
    let c = if next[0] == b { next[1] } else { next[0] };
    let at = face.outline.iter().flatten().fold([0.0; 3], |acc, p| {
        [acc[0] + p[0], acc[1] + p[1], acc[2] + p[2]]
    });
    let at = at.map(|v| v / (2 * face.outline.len()) as f64);
    let far = *face
        .outline
        .iter()
        .flatten()
        .max_by(|p, q| {
            let d = |r: &[f64; 3]| (r[0] - a[0]).hypot(r[1] - a[1]);
            d(p).total_cmp(&d(q))
        })
        .expect("a corner");
    let m = &mut w.viewer.measuring;
    m.done.push(Measured::Area {
        face,
        what: s.name.clone(),
        at,
    });
    m.done.push(Measured::Distance { a, b });
    m.done.push(Measured::Angle { a, at: b, b: c });
    // The pointer on the farthest corner, as the scene sees it.
    let scene_size = Size::new(
        1400.0 - 300.0 - 300.0 - 2.0 * kentos_ui::widget::sash::THICKNESS,
        860.0 - 62.0,
    );
    let vp = kentos_scene3d::Viewport {
        width: f64::from(scene_size.width),
        height: f64::from(scene_size.height),
    };
    if let Some((x, y, _)) = w.viewer.camera().project(far, &vp) {
        w.update(Message::Gesture(Gesture::Hover(
            Some((x as f32 + 2.0, y as f32 + 2.0)),
            scene_size.width,
            scene_size.height,
        )));
    }
}

/// The first object of `class` whose label holds `words`.
fn find(w: &Window, class: &str, words: &str) -> usize {
    let v = w.viewer.view().expect("loaded");
    v.input()
        .objects
        .iter()
        .position(|o| o.class == class && o.label.contains(words))
        .unwrap_or_else(|| panic!("a {class} “{words}”"))
}

#[test]
#[ignore = "pictures for the owner: KENTOS_SNAPSHOT_BACKEND=wgpu … -- --ignored"]
fn screens() {
    let building = "fixtures/arch/v1/ifc/kesit-binasi.ifc";
    for (mode, tag) in [(Mode::Light, "acik"), (Mode::Dark, "koyu")] {
        let mut w = opened(building, mode);
        // A click in the scene's middle: the scene is what the two panels leave of the window under the strip.
        let scene = Size::new(
            1400.0 - 300.0 - 300.0 - 2.0 * kentos_ui::widget::sash::THICKNESS,
            860.0 - 62.0,
        );
        w.update(Message::Gesture(Gesture::Pick(
            scene.width * 0.62,
            scene.height * 0.55,
            scene.width,
            scene.height,
        )));
        assert!(
            w.viewer.view().and_then(|v| v.selected()).is_some(),
            "the click chose something"
        );
        shot(&mut w, &format!("kesit-binasi-tiklanan-{tag}"));
    }

    let mut w = opened(building, Mode::Light);
    w.update(Message::Cut(Cut::Level));
    let (lo, hi) = w.viewer.span(Cut::Level).expect("a span");
    w.update(Message::At(lo + (hi - lo) * 0.3));
    w.update(Message::View(1));
    shot(&mut w, "kesit-binasi-yatay-kesit");

    let mut w = opened(building, Mode::Light);
    let storey = find(&w, "IfcBuildingStorey", "Zemin");
    w.update(Message::Choose(Some(storey)));
    w.update(Message::Boxed);
    w.update(Message::Panel(Panel::Class));
    shot(&mut w, "kesit-binasi-kat-kutusu");

    // The tree's search (docs/adr/8118): the columns, their class beside them; the tree widened by its edge; one of
    // them chosen, the section's card for a north–south plane. Light and dark.
    for (mode, tag) in [(Mode::Light, "acik"), (Mode::Dark, "koyu")] {
        let mut w = opened(building, mode);
        w.update(Message::Search("kolon".into()));
        let found = w.viewer.found();
        assert_eq!(found.len(), 16, "the section building's columns");
        w.update(Message::TreeWidth(Some(360.0)));
        w.update(Message::Choose(Some(found[0])));
        w.update(Message::Cut(Cut::NorthSouth));
        shot(&mut w, &format!("kesit-binasi-arama-{tag}"));
    }
    // Turkish letters fold: “iç” finds the interior walls written “İç_duvar”.
    let mut w = opened(building, Mode::Light);
    w.update(Message::Search("iç duvar".into()));
    assert!(!w.viewer.found().is_empty(), "İç_duvar by “iç duvar”");

    // Ölç (docs/adr/8121): the roof's top face measured (its area), its south edge's length and the angle at its
    // corner, the pointer on another corner; light and dark. Then the side panes folded to their rails.
    for (mode, tag) in [(Mode::Light, "acik"), (Mode::Dark, "koyu")] {
        let mut w = opened(building, mode);
        measured_roof(&mut w);
        shot(&mut w, &format!("kesit-binasi-olcum-{tag}"));
    }
    let mut w = opened(building, Mode::Light);
    measured_roof(&mut w);
    w.update(Message::MeasureFold);
    w.update(Message::Pane(kentos_model_view_ui::Side::Tree));
    w.update(Message::Pane(kentos_model_view_ui::Side::Details));
    shot(&mut w, "kesit-binasi-paneller-kapali");

    let mut w = opened(
        "fixtures/ifc/v1/disaridan/bsi-ifc4x3-building-architecture.ifc",
        Mode::Light,
    );
    w.update(Message::Spaces);
    shot(&mut w, "disaridan-ifc4x3-bina");

    let mut w = Window {
        viewer: Viewer::new("buyuk-model.ifc"),
        file: None,
        mode: Mode::Light,
        quality: Quality::High,
    };
    w.viewer.progress("Gövdeler üçgenleniyor", 1200, 49000);
    shot(&mut w, "okunuyor");
    w.viewer.failed("Dosya bir IFC dosyası değil: başında ISO-10303-21 satırı yok. Dosyanın uzantısı .ifc olsa da içeriği başka bir biçimde; dosyayı yazan programdan IFC olarak yeniden alın.");
    shot(&mut w, "reddedildi");
}
