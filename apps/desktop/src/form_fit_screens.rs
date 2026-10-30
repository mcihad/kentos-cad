//! The forms' windows at the owner's look (14 px text in Plus Jakarta Sans, the light theme) in three window sizes:
//! nothing runs past its pane or the window (docs/adr/0749). Pictures in `.run/shots/form-sigar-*`.
//!
//! ```text
//! cargo test -p kentos-desktop --lib form_fit_screens -- --ignored --nocapture --test-threads=1
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
/// the literal defaults the fields'; `genel_id` and the times the database's (`uuidv7()`, `now()`, docs/adr/0753's
/// `column.made`); `il_id` and `ust_id` the provinces' and the districts' rows by their foreign keys (docs/adr/0753).
fn district_fields() -> serde_json::Value {
    let list = |rows: &[(&str, &str)]| {
        rows.iter()
            .map(|(code, label)| serde_json::json!({ "code": code, "label": label }))
            .collect::<Vec<_>>()
    };
    serde_json::json!([
        { "name": "genel_id", "kind": "uuid", "column": { "made": "uuidv7()" } },
        { "name": "ek_veri", "kind": "json", "default": "{}" },
        { "name": "olusturma_tarihi", "kind": "instant", "column": { "made": "now()" } },
        { "name": "guncelleme_tarihi", "kind": "instant", "column": { "made": "now()" } },
        { "name": "il_id", "kind": "bigint", "required": true,
          "values": list(&[("1", "Adana"), ("6", "Ankara"), ("34", "İstanbul"), ("35", "İzmir"), ("58", "Sivas"),
                           ("60", "Tokat")]),
          "column": { "refers": { "table": "kbs.iller", "column": "id", "label": "ad" } } },
        { "name": "ust_id", "kind": "bigint",
          "values": list(&[("1", "Merkez (1)"), ("12", "Merkez (12)"), ("2", "Suşehri")]),
          "column": { "refers": { "table": "kbs.ilceler", "column": "id", "label": "ad" } } },
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
            ("ust_id", "1"),
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

/// Where the text `words` is drawn last (on top: a window's over the panels behind it).
fn text_last_at(snapshot: &mut Snapshot, app: &App, words: &str) -> Option<iced::Rectangle> {
    use iced::Rectangle;
    use iced::advanced::widget::{Id, Operation};
    use std::sync::{Arc, Mutex};
    struct Find(String, Arc<Mutex<Option<Rectangle>>>);
    impl Operation for Find {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn text(&mut self, _: Option<&Id>, bounds: Rectangle, text: &str) {
            if text == self.0 {
                *self.1.lock().expect("a lock") = Some(bounds);
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
        // Further down the new record: the layer's defaults where the form gives none (docs/adr/0752).
        if let Some(caption) = text_at(&mut snapshot, &app, "Yeni kayıt · İlçeler") {
            let at = iced::Point::new(caption.x + 150.0, caption.y + 200.0);
            snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -6.0));
            shot(&mut snapshot, &app, "2b-onizleme-asagi");
        }
        designer(&mut app, Event::Save);
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Forms(Event::Cancel));
        app.selection.set([Slot(1)]);
        run(&mut app, "data.formEdit");
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "3-formla-duzenle");
        // The province field's list: the provinces' rows by their names (docs/adr/0753).
        if let Some(at) = text_last_at(&mut snapshot, &app, "Sivas") {
            let at = iced::Point::new(at.x + at.width / 2.0, at.y + at.height / 2.0);
            snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
            shot(&mut snapshot, &app, "3b-il-listesi");
            snapshot.input(
                &mut app,
                App::view,
                &mut update,
                Input::Key(iced::keyboard::key::Named::Escape),
            );
        }
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
        let _ = app.update(Message::FormEntry(crate::forms::entry::Event::Cancel));
        snapshot.settle(&mut app, App::view, &mut update);
        // Alanlar on the province field and on a made one: what their columns say (docs/adr/0753).
        run(&mut app, "layer.fields");
        let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Choose(4)));
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "6-alanlar-baglanti");
        let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Choose(0)));
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "6b-alanlar-veritabani");
        let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Cancel));
    }
    sizes::load(Default::default());
}

/// A control's height at the owner's text size (a select, a field).
fn typography_control() -> f32 {
    kentos_ui::theme::metrics::control()
}

/// The fields of the owner's `muhasebe.hesap_plani` bound from PostGIS (docs/adr/0754): `id` is the key and no field;
/// the database's defaults (`uuidv7()`, `now()`) and its generated columns (`ana_hesap`, `d1`…`d4`, `birlesik_kod`,
/// worked out from `kod`) in `column.made`; `ust_id` refers to the table itself, of more than 10 000 rows, so no list
/// (docs/adr/0753).
fn ledger_fields() -> serde_json::Value {
    let part = |n: u32| {
        format!(
            "GENERATED ALWAYS AS (NULLIF(split_part((kod)::text, '.'::text, {n}), ''::text)) STORED"
        )
    };
    serde_json::json!([
        { "name": "genel_id", "kind": "uuid", "column": { "made": "uuidv7()" } },
        { "name": "ek_veri", "kind": "json", "default": "{}" },
        { "name": "olusturma_tarihi", "kind": "instant", "column": { "made": "now()" } },
        { "name": "guncelleme_tarihi", "kind": "instant", "column": { "made": "now()" } },
        { "name": "kod", "kind": "text", "length": 32, "required": true },
        { "name": "ad", "kind": "text", "length": 200, "required": true },
        { "name": "ust_id", "kind": "bigint",
          "column": { "refers": { "table": "muhasebe.hesap_plani", "column": "id", "label": "ad" } } },
        { "name": "duzey", "kind": "integer", "default": "1" },
        { "name": "tur", "kind": "text", "length": 16, "required": true },
        { "name": "calisma_sekli", "kind": "text", "length": 16, "required": true },
        { "name": "yil", "kind": "integer", "required": true },
        { "name": "kayit_atilabilir", "kind": "boolean", "default": "true" },
        { "name": "yasal_dayanak", "kind": "text" },
        { "name": "aciklama", "kind": "text" },
        { "name": "aktif", "kind": "boolean", "default": "true" },
        { "name": "sahip_turu", "kind": "text", "length": 16 },
        { "name": "sahip_id", "kind": "bigint" },
        { "name": "ana_hesap", "kind": "text", "length": 3,
          "column": { "made": "GENERATED ALWAYS AS (split_part((kod)::text, '.'::text, 1)) STORED" } },
        { "name": "d1", "kind": "text", "length": 2, "column": { "made": part(2) } },
        { "name": "d2", "kind": "text", "length": 2, "column": { "made": part(3) } },
        { "name": "d3", "kind": "text", "length": 2, "column": { "made": part(4) } },
        { "name": "d4", "kind": "text", "length": 2, "column": { "made": part(5) } },
        { "name": "birlesik_kod", "kind": "text", "length": 11,
          "column": { "made": "GENERATED ALWAYS AS (replace((kod)::text, '.'::text, ''::text)) STORED" } }
    ])
}

/// An account like the owner's, its values as PostGIS gives them (the generated ones worked out from its code).
fn ledger(app: &mut App) {
    let mut o = crate::tools_scenes::Objects::new();
    let id = o.path(
        "hesap_plani",
        &[[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]],
        true,
    );
    o.data(
        id,
        &[
            ("genel_id", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0aa1"),
            ("ek_veri", "{}"),
            ("olusturma_tarihi", "2026-01-05T09:00:00+03:00"),
            ("guncelleme_tarihi", "2026-10-30T08:15:00+03:00"),
            ("kod", "104.02.01"),
            ("ad", "AB Hibeleri"),
            ("ust_id", "13"),
            ("duzey", "5"),
            ("tur", "aktif"),
            ("calisma_sekli", "borc"),
            ("yil", "2026"),
            ("kayit_atilabilir", "true"),
            ("aktif", "true"),
            ("ana_hesap", "104"),
            ("d1", "02"),
            ("d2", "01"),
            ("birlesik_kod", "1040201"),
        ],
        None,
    );
    let fields = ledger_fields();
    crate::tools_scenes::open_edited(app, o, move |d| {
        let mut layers = d["layers"].as_array().cloned().unwrap_or_default();
        let mut layer = crate::tools_scenes::layer("hesap_plani", "Hesap planı", "#2F7D5B", 0.35);
        layer["fields"] = fields;
        layers.push(layer);
        d["layers"] = serde_json::Value::Array(layers);
        d["activeLayer"] = serde_json::json!("hesap_plani");
    });
}

/// docs/adr/0754 §3: a value the bound table works out itself is shown and never edited, in Öznitelikler and in the
/// attribute table; a plain field beside it is; Alanlar and an empty form field say what makes it.
#[test]
fn a_generated_field_is_shown_and_never_edited() {
    let (mut app, _) = App::boot(None);
    ledger(&mut app);
    app.selection.set([Slot(1)]);
    let doc = app.document.as_ref().expect("open");
    let rows: Vec<_> = app
        .properties_panel(doc)
        .sections
        .into_iter()
        .flat_map(|s| s.rows)
        .collect();
    let editable = |label: &str| {
        rows.iter()
            .find(|r| r.label == label)
            .map(|r| r.editor.is_some())
    };
    assert_eq!(editable("ana_hesap"), Some(false), "a generated value");
    assert_eq!(editable("birlesik_kod"), Some(false));
    assert_eq!(editable("aciklama"), Some(true), "a plain field");
    let attrs = |app: &App| {
        app.document
            .as_ref()
            .and_then(|d| d.model.get(Slot(1)))
            .map(|e| e.base().attrs.clone())
            .unwrap_or_default()
    };
    let _ = app.update(Message::Run("data.featureTable"));
    for m in app.features_edit(1, "ana_hesap", "999").expect("the cell") {
        let _ = app.update(m);
    }
    assert!(!app.features.editing(), "the generated cell does not open");
    assert_eq!(
        attrs(&app).get("ana_hesap").map(String::as_str),
        Some("104")
    );
    for m in app
        .features_edit(1, "aciklama", "Hibe hesabı")
        .expect("the cell")
    {
        let _ = app.update(m);
    }
    assert_eq!(
        attrs(&app).get("aciklama").map(String::as_str),
        Some("Hibe hesabı")
    );
    let fields: Vec<kentos_commands::LayerField> =
        serde_json::from_value(ledger_fields()).expect("fields");
    let column = |name: &str| {
        fields
            .iter()
            .find(|f| f.name == name)
            .and_then(|f| f.column.as_ref())
    };
    assert_eq!(
        crate::layer_fields::made_said(column("ana_hesap")).as_deref(),
        Some("Veritabanı hesaplar: split_part((kod)::text, '.'::text, 1)")
    );
    assert!(
        crate::layer_fields::column_said(column("d4"))
            .is_some_and(|s| s.starts_with(
                "Değerini veritabanı satırın diğer değerlerinden hesaplar: NULLIF(split_part("
            ) && s.ends_with("Düzenlenmez; kaynağa her yazmada tablodan gelir.")),
        "{:?}",
        crate::layer_fields::column_said(column("d4"))
    );
    assert_eq!(
        crate::layer_fields::made_said(column("genel_id")).as_deref(),
        Some("Veritabanı verir: uuidv7()")
    );
    let identity = |always: bool| kentos_commands::FieldColumn {
        made: Some(if always {
            "GENERATED ALWAYS AS IDENTITY".to_owned()
        } else {
            "GENERATED BY DEFAULT AS IDENTITY".to_owned()
        }),
        refers: None,
        check: None,
    };
    assert_eq!(
        crate::layer_fields::made_said(Some(&identity(true))).as_deref(),
        Some("Veritabanı verir: sıradaki numara")
    );
    assert!(
        crate::layer_fields::column_said(Some(&identity(false)))
            .is_some_and(|s| s.contains("boş bırakılırsa veritabanı sıradaki numarayı verir"))
    );
    assert!(
        crate::layer_fields::column_said(Some(&identity(true))).is_some_and(|s| s
            .contains("her zaman kendisi numaralar")
            && s.ends_with("düzenlenmez."))
    );
}

/// The owner's ledger table with a form made from it (docs/adr/0754): the designer on a generated field, the new
/// record's preview saying what the database works out, the form on an account (its generated values shown, not
/// written), full screen, Öznitelikler, Alanlar on a generated field, and the attribute table, whose generated cell
/// stays shut where a plain one opens. Pictures in `.run/shots/form-hesap-*`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn ledger_form_screens() {
    use kentos_ui::snapshot::Input;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder");
    for (w, h) in [(1100.0, 650.0), (1440.0, 900.0)] {
        sizes::load(Default::default());
        let (mut app, _) = App::boot(None);
        owners_look(&mut app);
        let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        ledger(&mut app);
        let shot = |snapshot: &mut Snapshot, app: &App, step: &str| {
            let file = out.join(format!("form-hesap-{step}-{w}x{h}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("a picture");
            println!("{}", file.display());
        };
        run(&mut app, "data.forms");
        designer(&mut app, Event::NewForm);
        designer(&mut app, Event::NewLayer("hesap_plani".into()));
        designer(&mut app, Event::NewName("Hesap".into()));
        designer(&mut app, Event::Create);
        designer(&mut app, Event::SelectItem("ana-hesap".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "1-tasarim");
        designer(&mut app, Event::Preview(true));
        snapshot.settle(&mut app, App::view, &mut update);
        // Down the new record to its generated fields: what the database works out, read-only.
        if let Some(caption) = text_at(&mut snapshot, &app, "Yeni kayıt · Hesap planı") {
            let at = iced::Point::new(caption.x + 150.0, caption.y + 200.0);
            snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -30.0));
            shot(&mut snapshot, &app, "2-onizleme-hesaplanan");
        }
        designer(&mut app, Event::Save);
        snapshot.settle(&mut app, App::view, &mut update);
        let _ = app.update(Message::Forms(Event::Cancel));
        app.selection.set([Slot(1)]);
        run(&mut app, "data.formEdit");
        snapshot.settle(&mut app, App::view, &mut update);
        if let Some(field) = input_at(&mut snapshot, &app, "form-entry-ad") {
            let at = iced::Point::new(field.x + field.width / 2.0, field.y + field.height + 40.0);
            snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -30.0));
        }
        shot(&mut snapshot, &app, "3-formla-duzenle");
        let _ = app.update(Message::FormEntry(crate::forms::entry::Event::Cancel));
        snapshot.settle(&mut app, App::view, &mut update);
        // Full screen: the fields in two columns, the generated ones at the end.
        sizes::set(
            "Formla düzenle · Hesap",
            sizes::Kept {
                width: 1040.0,
                height: 600.0,
                maximized: true,
            },
        );
        run(&mut app, "data.formEdit");
        snapshot.settle(&mut app, App::view, &mut update);
        if let Some(field) = input_at(&mut snapshot, &app, "form-entry-ad") {
            let at = iced::Point::new(field.x + field.width / 2.0, field.y + field.height + 40.0);
            snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -30.0));
        }
        shot(&mut snapshot, &app, "4-tam-ekran");
        let _ = app.update(Message::FormEntry(crate::forms::entry::Event::Cancel));
        snapshot.settle(&mut app, App::view, &mut update);
        // Öznitelikler on the account, down to its generated rows.
        app.props_closed.insert("general");
        app.props_closed.insert("geometry");
        snapshot.settle(&mut app, App::view, &mut update);
        if let Some(row) = text_at(&mut snapshot, &app, "ek_veri") {
            let at = iced::Point::new(row.x + 40.0, row.y + row.height / 2.0);
            snapshot.input(&mut app, App::view, &mut update, Input::Scroll(at, -30.0));
        }
        shot(&mut snapshot, &app, "5-oznitelikler");
        // Alanlar on a generated field.
        run(&mut app, "layer.fields");
        let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Choose(17)));
        snapshot.settle(&mut app, App::view, &mut update);
        shot(&mut snapshot, &app, "6-alanlar-hesaplanan");
        let _ = app.update(Message::LayerFields(crate::layer_fields::Event::Cancel));
        snapshot.settle(&mut app, App::view, &mut update);
        // The attribute table, the account's code, name, note and generated columns shown (Sütunlar ▾): the generated
        // cell pressed to edit stays shut, a plain one opens.
        app.features.hidden.insert(
            "hesap_plani".into(),
            [
                "genel_id",
                "ek_veri",
                "olusturma_tarihi",
                "guncelleme_tarihi",
                "ust_id",
                "duzey",
                "tur",
                "calisma_sekli",
                "yil",
                "kayit_atilabilir",
                "yasal_dayanak",
                "aktif",
                "sahip_turu",
                "sahip_id",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        );
        let _ = app.update(Message::Run("data.featureTable"));
        snapshot.settle(&mut app, App::view, &mut update);
        if let Ok(m) = app.features_edit(1, "ana_hesap", "999") {
            for m in m.into_iter().take(2) {
                let _ = app.update(m);
            }
            snapshot.settle(&mut app, App::view, &mut update);
            shot(&mut snapshot, &app, "7-tablo-hesaplanan");
        }
        if let Ok(m) = app.features_edit(1, "aciklama", "Hibe") {
            for m in m.into_iter().take(2) {
                let _ = app.update(m);
            }
            snapshot.settle(&mut app, App::view, &mut update);
            shot(&mut snapshot, &app, "7b-tablo-duz");
        }
    }
    sizes::load(Default::default());
}
