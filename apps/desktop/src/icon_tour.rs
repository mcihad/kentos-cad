//! The icon tour (the owner's, 3 October 2026): the ribbon's lists and
//! menus, the status bar's mode menu, a layer's menu and the sheet mode's
//! menus, opened as a person opens them, so every entry's icon can be looked
//! at. Not a test of correctness and not run by default; the pictures go to
//! `.run/shots/ikon-turu`:
//!
//! ```text
//! cargo test -p kentos-desktop icon_tour -- --ignored --nocapture
//! ```

use iced::{Point, Rectangle, Size, mouse};
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::files_testing::{app_with_drawing, find_texts};

/// How a scene's list opens.
#[derive(Clone, Copy)]
enum Open {
    /// A press on the caption: a large split's list, a menu button.
    Caption,
    /// A press on the arrow right of the caption: a small split's list.
    Arrow,
    /// A right press on the caption: a row's menu.
    Menu,
}

struct Scene {
    name: &'static str,
    /// The ribbon tab to show first.
    tab: Option<&'static str>,
    caption: &'static str,
    open: Open,
    /// Over a sheet (the sheet mode's ribbon and panels).
    sheet: bool,
}

const fn scene(
    name: &'static str,
    tab: Option<&'static str>,
    caption: &'static str,
    open: Open,
) -> Scene {
    Scene {
        name,
        tab,
        caption,
        open,
        sheet: false,
    }
}

const fn on_sheet(name: &'static str, caption: &'static str, open: Open) -> Scene {
    Scene {
        name,
        tab: None,
        caption,
        open,
        sheet: true,
    }
}

const SCENES: [Scene; 33] = [
    scene("daire", Some("Giriş"), "Daire", Open::Caption),
    scene("yay", Some("Giriş"), "Yay", Open::Caption),
    scene("buda", Some("Giriş"), "Buda", Open::Arrow),
    scene("uzat", Some("Giriş"), "Uzat", Open::Arrow),
    scene("kose", Some("Değiştir"), "Köşe yuvarla", Open::Caption),
    scene("parcala", Some("Değiştir"), "Parçala", Open::Arrow),
    scene("kot", Some("Değiştir"), "Kot ver", Open::Caption),
    scene("dizi", Some("Değiştir"), "Dizi", Open::Caption),
    // Hizala ve dağıt's eight methods under Dönüştür's ▾ (docs/adr/0194).
    scene("hizala-dagit", Some("Değiştir"), "Dönüştür", Open::Caption),
    scene("ara-nokta", Some("Çizim"), "Ara nokta", Open::Arrow),
    scene("kesisim", Some("Çizim"), "Kesişim noktası", Open::Arrow),
    scene("yardimci", Some("Çizim"), "Yardımcı çizgi", Open::Caption),
    scene("olcu", Some("Çizim"), "Ölçülendirme", Open::Caption),
    scene("ice-aktar", Some("Dosya"), "İçe aktar", Open::Caption),
    scene("disa-aktar", Some("Dosya"), "Dışa aktar", Open::Caption),
    scene("proje-turu", Some("Görünüm"), "Proje türü", Open::Caption),
    scene(
        "sembol-boyutu",
        Some("Görünüm"),
        "Sembol boyutu",
        Open::Caption,
    ),
    scene("katman", None, "Bina", Open::Menu),
    // Katmanlar's ▾: the layer actions by an object (docs/adr/0177 §7).
    scene("katman-araclari", Some("Giriş"), "Katmanlar", Open::Caption),
    // The status bar's project type.
    scene("mod", None, "CAD", Open::Caption),
    on_sheet("pafta-yeni", "Yeni", Open::Caption),
    on_sheet("pafta-sekil", "Şekil", Open::Arrow),
    on_sheet("pafta-harita", "Harita", Open::Arrow),
    on_sheet("pafta-lejant", "Lejant", Open::Arrow),
    on_sheet("pafta-olcek", "Ölçek çubuğu", Open::Arrow),
    on_sheet("pafta-kuzey", "Kuzey oku", Open::Arrow),
    on_sheet("pafta-cerceve", "Pafta çerçevesi", Open::Arrow),
    on_sheet("pafta-tablo", "Tablo", Open::Arrow),
    on_sheet("pafta-hizala", "Hizala", Open::Caption),
    on_sheet("pafta-dagit", "Dağıt", Open::Caption),
    on_sheet("pafta-sira", "Sıra", Open::Caption),
    on_sheet("pafta-disa", "Dışa aktar", Open::Caption),
    on_sheet("pafta-oge", "Harita", Open::Menu),
];

fn press(snapshot: &mut Snapshot, app: &mut App, at: Point, button: mouse::Button) {
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    snapshot.step(
        app,
        App::view,
        &mut update,
        &[
            iced::Event::Mouse(mouse::Event::CursorMoved { position: at }),
            iced::Event::Mouse(mouse::Event::ButtonPressed(button)),
            iced::Event::Mouse(mouse::Event::ButtonReleased(button)),
        ],
    );
    snapshot.settle(app, App::view, &mut update);
}

/// Where a caption is: the ribbon's (the highest below `top`: the tab row
/// has a Harita of its own) for a list, the first below the ribbon for a row.
fn caption_at(
    snapshot: &mut Snapshot,
    app: &App,
    caption: &str,
    row: bool,
    top: f32,
) -> Option<Rectangle> {
    let found = find_texts(snapshot, app, caption);
    let found = found.into_iter().filter(|at| at.y > top);
    if row {
        found
            .filter(|at| at.y > 160.0)
            .min_by(|a, b| a.y.total_cmp(&b.y))
    } else {
        found.min_by(|a, b| a.y.total_cmp(&b.y))
    }
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn icon_tour() {
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    let out =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots/ikon-turu");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    for (mode, suffix) in [("light", ""), ("dark", "-koyu")] {
        for scene in &SCENES {
            if only
                .as_ref()
                .is_some_and(|o| !o.split(',').any(|s| s == scene.name))
            {
                continue;
            }
            // The sheet's Çıktı panel keeps its labels only in a wide window.
            let (width, height) = if scene.name == "pafta-disa" {
                (1920.0, 1080.0)
            } else {
                (1440.0, 900.0)
            };
            let mut app = app_with_drawing();
            // The drawing's lists in CAD's ribbon, which has them all (CBS leaves dimensions,
            // arrays and helpers out); the sheet's in CBS's profile (docs/adr/0165).
            if !scene.sheet {
                let _ = app.update(Message::Run("workspace.cad"));
            }
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(Size::new(width, height)));
            if scene.sheet {
                let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::NewSheet));
                if app.sheets.questions().is_some() {
                    let _ = app.update(Message::Sheet(kentos_sheet_ui::Message::Questions(
                        kentos_sheet_ui::questions::QuestionsMessage::Make,
                    )));
                }
            }
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            if let Some(tab) = scene.tab {
                let at = caption_at(&mut snapshot, &app, tab, false, 0.0)
                    .unwrap_or_else(|| panic!("{}: the tab {tab}", scene.name));
                press(&mut snapshot, &mut app, at.center(), mouse::Button::Left);
            }
            let row = matches!(scene.open, Open::Menu);
            let top = if scene.sheet { 40.0 } else { 0.0 };
            let Some(at) = caption_at(&mut snapshot, &app, scene.caption, row, top) else {
                println!("{}: “{}” is not shown", scene.name, scene.caption);
                continue;
            };
            match scene.open {
                Open::Caption => press(&mut snapshot, &mut app, at.center(), mouse::Button::Left),
                Open::Arrow => press(
                    &mut snapshot,
                    &mut app,
                    Point::new(at.x + at.width + 10.0, at.center_y()),
                    mouse::Button::Left,
                ),
                Open::Menu => press(&mut snapshot, &mut app, at.center(), mouse::Button::Right),
            }
            for _ in 0..3 {
                let _ = snapshot.render(app.view(), &app.theme());
            }
            let file = out.join(format!("{}{suffix}.png", scene.name));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}
