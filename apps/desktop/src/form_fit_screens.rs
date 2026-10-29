//! The forms' windows at the owner's look (14 px text in Plus Jakarta Sans, the light theme) in three window sizes:
//! nothing runs past its pane or the window (docs/adr/0749). Pictures in `.run/shots/form-sigar-*`.
//!
//! ```text
//! cargo test -p kentos-desktop --bin kentos-cad form_fit_screens -- --ignored --nocapture --test-threads=1
//! ```

use std::path::PathBuf;

use iced::Size;
use kentos_domain::Slot;
use kentos_ui::snapshot::Snapshot;
use kentos_ui::widget::sizable::sizes;

use crate::app::{App, Message};
use crate::form_scenes::{buildings, designer};
use crate::forms::designer::Event;
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

fn update(app: &mut App, message: Message) {
    let _ = app.update(message);
}

/// The owner's look: the text size, the face and the theme.
fn owners_look(app: &mut App) {
    let _ = app.settings.choose(&[
        ("appearance.textSize", serde_json::Value::from(14)),
        ("appearance.uiFont", serde_json::Value::from("jakarta")),
        ("appearance.theme", serde_json::Value::from("light")),
    ]);
    app.apply_settings();
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn form_fit_screens() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder");
    let scenes: [Scene; 6] = [
        ("tasarim-ozellikler", |app| {
            buildings(app, true);
            run(app, "data.forms");
            designer(app, Event::SelectItem("aciklama".into()));
        }),
        ("tasarim-onizleme", |app| {
            buildings(app, true);
            run(app, "data.forms");
            designer(app, Event::Preview(true));
        }),
        ("tasarim-degisiklikler", |app| {
            buildings(app, true);
            run(app, "data.forms");
            designer(app, Event::Changes);
        }),
        ("tasarim-akis", |app| {
            buildings(app, true);
            run(app, "data.forms");
            designer(app, Event::Flow);
        }),
        ("tasarim-iliski", |app| {
            buildings(app, true);
            run(app, "data.forms");
            designer(
                app,
                Event::Palette(crate::forms::designer::Palette::Relations),
            );
        }),
        ("formla-duzenle", |app| {
            buildings(app, true);
            app.selection.set([Slot(1)]);
            run(app, "data.formEdit");
        }),
    ];
    // The forms' other windows, from their own scenes (docs/adr/0405 … 0438), and the designer full screen.
    let shared = crate::form_scenes::scenes();
    let others = [
        "form-iliskiler",
        "form-kayit-denetle",
        "form-kosullu-bicim",
        "form-alt-turler",
        "form-bilesimler",
        "form-kayit-kurallari",
        "deger-listeleri",
        "form-iliski-yigini",
    ]
    .map(|name| {
        let open = shared
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, f)| *f)
            .expect("a scene of its name");
        (name, open)
    });
    let full: Scene = ("tasarim-tam-ekran", |app| {
        buildings(app, true);
        run(app, "data.forms");
        designer(app, Event::SelectItem("aciklama".into()));
        sizes::set(
            "Formlar",
            sizes::Kept {
                width: 1040.0,
                height: 600.0,
                maximized: true,
            },
        );
    });
    let only = std::env::var("KENTOS_SHOTS_ONLY").ok();
    for (name, open) in scenes.into_iter().chain(others).chain([full]) {
        if only
            .as_deref()
            .is_some_and(|o| !o.split(',').any(|n| n == name))
        {
            continue;
        }
        for (w, h) in [(1100.0, 650.0), (1440.0, 900.0), (1920.0, 1080.0)] {
            sizes::load(Default::default());
            let (mut app, _) = App::boot(None);
            owners_look(&mut app);
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            snapshot.settle(&mut app, App::view, &mut update);
            open(&mut app);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("form-sigar-{name}-{w}x{h}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("a picture");
            println!("{}", file.display());
        }
    }
    sizes::load(Default::default());
}

/// Every forms scene in a low window (1100 × 650) at the owner's look: the window's own buttons (Vazgeç,
/// Kaydet, Kapat, Tamam, Uygula) within the window, none pushed out under its edge (docs/adr/0749).
#[test]
#[ignore = "every forms scene rendered, run by hand"]
fn every_forms_window_keeps_its_buttons_in_a_low_window() {
    use iced::Rectangle;
    use iced::advanced::widget::{Id, Operation};
    use std::sync::{Arc, Mutex};
    struct Find(Arc<Mutex<Vec<(String, Rectangle)>>>);
    impl Operation for Find {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
            if matches!(text, "Vazgeç" | "Kaydet" | "Kapat" | "Tamam" | "Uygula") {
                self.0
                    .lock()
                    .expect("a lock")
                    .push((text.to_owned(), bounds));
            }
        }
    }
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder");
    let (w, h) = (1100.0, 650.0);
    let mut cut = Vec::new();
    let mut seen = 0;
    for (name, open) in crate::form_scenes::scenes() {
        sizes::load(Default::default());
        let (mut app, _) = App::boot(None);
        owners_look(&mut app);
        let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        open(&mut app);
        snapshot.settle(&mut app, App::view, &mut update);
        let found = Arc::new(Mutex::new(Vec::new()));
        snapshot.operate(app.view(), Box::new(Find(found.clone())));
        let buttons = found.lock().map_or(Vec::new(), |f| f.clone());
        if buttons.is_empty() {
            continue;
        }
        seen += 1;
        let out_of = buttons
            .iter()
            .filter(|(_, b)| b.y + b.height > h + 0.5 || b.x + b.width > w + 0.5)
            .map(|(t, b)| format!("{t} at {:.0},{:.0}", b.x, b.y))
            .collect::<Vec<_>>();
        if !out_of.is_empty() {
            let file = out.join(format!("form-sigar-kesik-{name}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("a picture");
            cut.push(format!("{name}: {}", out_of.join(", ")));
        }
    }
    sizes::load(Default::default());
    println!("{seen} windows with buttons");
    assert!(
        cut.is_empty(),
        "buttons out of the window:\n{}",
        cut.join("\n")
    );
}
