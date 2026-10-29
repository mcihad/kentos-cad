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

/// The fields of the owner's `kbs.ilceler` table bound from PostGIS (docs/adr/0629's kinds), in its order, with the
/// table's rules (docs/adr/0750): `id` is the key and no field; `il_id` and `ad` NOT NULL without a default, required;
/// the literal defaults the fields'; `genel_id` and the times the database's (`uuidv7()`, `now()`).
fn district_fields() -> serde_json::Value {
    serde_json::json!([
        { "name": "genel_id", "kind": "uuid" },
        { "name": "ek_veri", "kind": "json", "default": "{}" },
        { "name": "olusturma_tarihi", "kind": "instant" },
        { "name": "guncelleme_tarihi", "kind": "instant" },
        { "name": "il_id", "kind": "bigint", "required": true },
        { "name": "ust_id", "kind": "bigint" },
        { "name": "ad", "kind": "text", "required": true },
        { "name": "kisa_ad", "kind": "text" },
        { "name": "tur", "kind": "text", "default": "İlçe" },
        { "name": "mernis_kodu", "kind": "text" },
        { "name": "uavt_kodu", "kind": "text" },
        { "name": "merkez_mi", "kind": "boolean", "default": "false" },
        { "name": "kurulus_karar_no", "kind": "text" },
        { "name": "kurulus_tarihi", "kind": "date" },
        { "name": "kapanis_karar_no", "kind": "text" },
        { "name": "kapanis_tarihi", "kind": "date" },
        { "name": "maks_id", "kind": "text" },
        { "name": "aktif", "kind": "boolean", "default": "true" }
    ])
}

/// A district like the owner's, its values as PostGIS gives them.
fn districts(app: &mut App) {
    let mut o = crate::tools_scenes::Objects::new();
    let id = o.path(
        "ilceler",
        &[[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]],
        true,
    );
    o.data(
        id,
        &[
            ("genel_id", "6f1c2a7e-3b4d-4c5e-8f9a-0b1c2d3e4f5a"),
            ("ek_veri", "{\"nufus\": 31245, \"kaynak\": \"TÜİK\"}"),
            ("olusturma_tarihi", "2026-03-14T09:30:00+03:00"),
            ("guncelleme_tarihi", "2026-10-28T16:05:12+03:00"),
            ("il_id", "58"),
            ("ust_id", "58"),
            ("ad", "Suşehri"),
            ("kisa_ad", "Suşehri"),
            ("tur", "ilçe"),
            ("mernis_kodu", "1779"),
            ("uavt_kodu", "1779"),
            ("merkez_mi", "false"),
            ("kurulus_tarihi", "1869-01-01"),
            ("aktif", "true"),
        ],
        None,
    );
    let fields = district_fields();
    crate::tools_scenes::open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut layer = crate::tools_scenes::layer("ilceler", "İlçeler", "#3E63DD", 0.35);
        layer["fields"] = fields;
        layers.push(layer);
        d["layers"] = serde_json::Value::Array(layers);
        d["activeLayer"] = serde_json::json!("ilceler");
    });
}

/// Where the text `words` is drawn first.
fn text_at(snapshot: &mut Snapshot, app: &App, words: &str) -> Option<iced::Rectangle> {
    use iced::Rectangle;
    use iced::advanced::widget::{Id, Operation};
    use std::sync::{Arc, Mutex};
    struct Find(String, Arc<Mutex<Option<Rectangle>>>);
    impl Operation for Find {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
            let mut found = self.1.lock().expect("a lock");
            if found.is_none() && text == self.0 {
                *found = Some(bounds);
            }
        }
    }
    let found = Arc::new(Mutex::new(None));
    snapshot.operate(app.view(), Box::new(Find(words.to_owned(), found.clone())));
    *found.lock().expect("a lock")
}

/// Where the text input of `id` is.
fn input_at(snapshot: &mut Snapshot, app: &App, id: &str) -> Option<iced::Rectangle> {
    use iced::Rectangle;
    use iced::advanced::widget::operation::TextInput;
    use iced::advanced::widget::{Id, Operation};
    use std::sync::{Arc, Mutex};
    struct Find(Id, Arc<Mutex<Option<Rectangle>>>);
    impl Operation for Find {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, _: &mut dyn TextInput) {
            if id == Some(&self.0) {
                *self.1.lock().expect("a lock") = Some(bounds);
            }
        }
    }
    let found = Arc::new(Mutex::new(None));
    snapshot.operate(
        app.view(),
        Box::new(Find(Id::from(id.to_owned()), found.clone())),
    );
    *found.lock().expect("a lock")
}

/// A form made from the owner's district layer with Yeni form: the designer, the widget's choices, then the form
/// on the district in its own size, made large and with its instant's picker open (docs/adr/0750, 0751). Pictures
/// in `.run/shots/form-ilce-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn district_form_screens() {
    use kentos_ui::snapshot::Input;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder");
    for (w, h) in [(1100.0, 650.0), (1440.0, 900.0)] {
        sizes::load(Default::default());
        let (mut app, _) = App::boot(None);
        owners_look(&mut app);
        let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        districts(&mut app);
        run(&mut app, "data.forms");
        designer(&mut app, Event::NewForm);
        designer(&mut app, Event::NewLayer("ilceler".into()));
        designer(&mut app, Event::NewName("İlçe".into()));
        designer(&mut app, Event::Create);
        designer(&mut app, Event::SelectItem("olusturma-tarihi".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let shot = |snapshot: &mut Snapshot, app: &App, step: &str| {
            let file = out.join(format!("form-ilce-{step}-{w}x{h}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("a picture");
            println!("{}", file.display());
        };
        shot(&mut snapshot, &app, "1-tasarim");
        // The widgets a yes-or-no field takes, each with its icon (docs/adr/0751).
        designer(&mut app, Event::SelectItem("merkez-mi".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        if let Some(caption) = text_at(&mut snapshot, &app, "Öğe türü") {
            let at = iced::Point::new(
                caption.x + 60.0,
                caption.y + caption.height + 4.0 + typography_control() / 2.0,
            );
            snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
            shot(&mut snapshot, &app, "1b-oge-turu");
            snapshot.input(
                &mut app,
                App::view,
                &mut update,
                Input::Key(iced::keyboard::key::Named::Escape),
            );
        }
        designer(&mut app, Event::Preview(true));
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "2-onizleme");
        designer(&mut app, Event::Save);
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Forms(Event::Cancel));
        app.selection.set([Slot(1)]);
        run(&mut app, "data.formEdit");
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "3-formla-duzenle");
        let _ = app.update(Message::FormEntry(crate::forms::entry::Event::Cancel));
        snapshot.settle(&mut app, App::view, &mut update);
        // The same form made full screen (docs/adr/0746): the fields in two columns (docs/adr/0751).
        sizes::set(
            "Formla düzenle · İlçe",
            sizes::Kept {
                width: 1040.0,
                height: 600.0,
                maximized: true,
            },
        );
        run(&mut app, "data.formEdit");
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "4-tam-ekran");
        // The instant's picker, opened by its button beside the field (docs/adr/0751).
        if let Some(field) = input_at(&mut snapshot, &app, "form-entry-olusturma_tarihi") {
            let at = iced::Point::new(field.x + field.width + 14.0, field.y + field.height / 2.0);
            snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
            shot(&mut snapshot, &app, "5-zaman-secici");
        }
    }
    sizes::load(Default::default());
}

/// A control's height at the owner's text size (a select, a field).
fn typography_control() -> f32 {
    kentos_ui::theme::metrics::control()
}
