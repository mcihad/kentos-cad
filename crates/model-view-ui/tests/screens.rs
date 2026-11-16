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
use kentos_model_view_ui::scene::Gesture;
use kentos_model_view_ui::{Host, Message, Out, Panel, Viewer, look};
use kentos_ui::theme::{self, Accent, Mode, Tokens};

struct Window {
    viewer: Viewer,
    file: Option<(Vec<u8>, Model)>,
    mode: Mode,
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
            coordinate: Box::new(|v| format!("{v:.3}")),
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
    let name = Path::new(path).file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let mut viewer = Viewer::new(name.clone());
    viewer.load(ifc::view::input(&name, &model, &bodies), vec![(model.summary.schema.clone(), "Dosyanın şeması.".to_owned())]);
    Window { viewer, file: Some((bytes, model)), mode }
}

fn shot(w: &mut Window, name: &str) {
    let out = root().join(".run/gui-evidence/model-view-ui");
    std::fs::create_dir_all(&out).expect("the folder");
    let mut snapshot = kentos_ui::snapshot::Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
    let mut update = |w: &mut Window, m: Message| w.update(m);
    snapshot.settle(w, Window::view, &mut update);
    for _ in 0..3 {
        let _ = snapshot.render(w.view(), &w.theme());
    }
    let file = out.join(format!("{name}.png"));
    snapshot.render(w.view(), &w.theme()).save(&file).expect("writes the picture");
    println!("{} ({})", file.display(), snapshot.renderer_name());
}

/// The first object of `class` whose label holds `words`.
fn find(w: &Window, class: &str, words: &str) -> usize {
    let v = w.viewer.view().expect("loaded");
    v.input().objects.iter().position(|o| o.class == class && o.label.contains(words)).unwrap_or_else(|| panic!("a {class} “{words}”"))
}

#[test]
#[ignore = "pictures for the owner: KENTOS_SNAPSHOT_BACKEND=wgpu … -- --ignored"]
fn screens() {
    let building = "fixtures/arch/v1/ifc/kesit-binasi.ifc";
    for (mode, tag) in [(Mode::Light, "acik"), (Mode::Dark, "koyu")] {
        let mut w = opened(building, mode);
        // A click in the scene's middle: the scene is what the two panels leave of the window under the strip.
        let scene = Size::new(1400.0 - 280.0 - 300.0 - 2.0, 860.0 - 62.0);
        w.update(Message::Gesture(Gesture::Pick(scene.width * 0.62, scene.height * 0.55, scene.width, scene.height)));
        assert!(w.viewer.view().and_then(|v| v.selected()).is_some(), "the click chose something");
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

    let mut w = opened("fixtures/ifc/v1/disaridan/bsi-ifc4x3-building-architecture.ifc", Mode::Light);
    w.update(Message::Spaces);
    shot(&mut w, "disaridan-ifc4x3-bina");

    let mut w = Window { viewer: Viewer::new("buyuk-model.ifc"), file: None, mode: Mode::Light };
    w.viewer.progress("Gövdeler üçgenleniyor", 1200, 49000);
    shot(&mut w, "okunuyor");
    w.viewer.failed("Dosya bir IFC dosyası değil: başında ISO-10303-21 satırı yok. Dosyanın uzantısı .ifc olsa da içeriği başka bir biçimde; dosyayı yazan programdan IFC olarak yeniden alın.");
    shot(&mut w, "reddedildi");
}
