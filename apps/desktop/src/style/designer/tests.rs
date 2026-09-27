//! Sembol tasarımcısı as the user drives it: a new symbol from Stil
//! yöneticisi, the layer list's edits and their undo, the form's fields as
//! typed (numbers, dashes, colours, expressions), ↑ and ↓, saving to
//! Kitaplığım and to the project, a system symbol's copy, the question on
//! closing, a layer style's symbol handed back with Uygula, pictures taken in.

use iced::widget::Id;
use kentos_native_style::designer::{LayerPath, Patch, set};
use kentos_native_style::library::Source;
use kentos_native_style::preview::Geometry;
use kentos_native_style::renderer::GeometryClass;
use serde_json::{Value, json};

use super::{Designer, Edit, Event, Focus, Target, field_id};
use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, scratch};
use crate::style::layer_style::{Event as LayerEvent, Kind, SetAt};
use crate::style::manager::Event as ManagerEvent;

const DOUBLE: &str = "temel.cizgi.cift";

fn de(app: &mut App, e: Event) {
    let _ = app.update(Message::Designer(Box::new(e)));
}

fn sm(app: &mut App, e: ManagerEvent) {
    let _ = app.update(Message::StyleManager(Box::new(e)));
}

fn designer(app: &App) -> &Designer {
    app.styles.designer.as_ref().expect("the designer is open")
}

fn layers(app: &App) -> Vec<Value> {
    designer(app).draft.symbol["layers"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn types(app: &App) -> Vec<String> {
    layers(app)
        .iter()
        .map(|l| l["type"].as_str().unwrap_or("").to_owned())
        .collect()
}

fn said(app: &App) -> String {
    designer(app)
        .said()
        .map(|(t, _)| t.clone())
        .unwrap_or_default()
}

/// A field typed into on the chosen layer.
fn typed(app: &mut App, key: &str, text: &str, patch: Option<Patch>) {
    let at = designer(app).selected;
    de(
        app,
        Event::Edit(Edit {
            at,
            key: key.into(),
            text: Some(text.into()),
            patch,
            focus: None,
        }),
    );
}

/// The app with the sample drawing, Kitaplığım in a scratch folder and Stil yöneticisi open.
fn with_manager(name: &str) -> (App, std::path::PathBuf) {
    let mut app = app_with_drawing();
    let dir = scratch(name);
    assert_eq!(app.styles.open_user_library(&dir), None);
    let _ = app.update(Message::Run("style.manager"));
    (app, dir)
}

#[test]
fn a_new_symbol_is_made_saved_and_shown_in_the_manager() {
    let (mut app, dir) = with_manager("yeni");
    sm(&mut app, ManagerEvent::NewSymbol("fill"));
    assert_eq!(app.dialog, Some(Dialog::SymbolDesigner));
    let d = designer(&app);
    assert_eq!(d.draft.name, "Yeni alan sembolü");
    assert_eq!(d.draft.path, vec!["Sembollerim".to_owned()]);
    assert_eq!(d.title(), "Alan sembolü tasarımcısı");
    assert_eq!(types(&app), ["simpleFill"]);
    assert_eq!(d.sample, Geometry::Area);
    // Unchanged, Vazgeç closes without a question, back to the manager.
    de(&mut app, Event::Close);
    assert_eq!(app.dialog, Some(Dialog::StyleManager));
    assert!(app.styles.designer.is_none());

    sm(&mut app, ManagerEvent::NewSymbol("fill"));
    de(&mut app, Event::Add("hatchFill", None));
    de(&mut app, Event::Add("simpleLine", None));
    assert_eq!(types(&app), ["simpleFill", "hatchFill", "simpleLine"]);
    assert_eq!(designer(&app).selected, LayerPath::Top(2));
    assert_eq!(designer(&app).title(), "Alan sembolü tasarımcısı •");
    // An area's line starts at the area's width.
    assert_eq!(layers(&app)[2]["width"], json!(0.25));
    de(&mut app, Event::Name("Bahçe alanı".into()));
    de(&mut app, Event::Path("Sembollerim / Alanlar".into()));
    de(&mut app, Event::Save);
    assert_eq!(said(&app), "“Bahçe alanı” kaydedildi.");
    assert!(!designer(&app).dirty());
    let Target::Library {
        id: Some(id),
        source: Source::User,
    } = designer(&app).target.clone()
    else {
        panic!("saved into Kitaplığım");
    };
    let (item, _) = app.styles.library.get(&id).expect("in the library");
    assert_eq!(item.name(), "Bahçe alanı");
    assert_eq!(item.path(), ["Sembollerim", "Alanlar"]);
    assert_eq!(item.symbol().expect("a symbol")["layers"][1]["type"], "hatchFill");
    // Kept in Kitaplığım's file at once.
    let text = std::fs::read_to_string(dir.join(crate::style::user_library::FILE))
        .expect("Kitaplığım's file");
    assert!(text.contains("Bahçe alanı"));
    // The manager shows it where it now is.
    let m = app.styles.manager.as_ref().expect("the manager stays");
    assert_eq!(m.selected.as_deref(), Some(id.as_str()));
    assert_eq!(
        m.at,
        (
            Source::User,
            vec!["Sembollerim".to_owned(), "Alanlar".to_owned()]
        )
    );
    // A second Kaydet updates the same item.
    de(&mut app, Event::Add("patternFill", None));
    de(&mut app, Event::Save);
    let (item, _) = app.styles.library.get(&id).expect("still there");
    assert_eq!(
        item.symbol().expect("a symbol")["layers"]
            .as_array()
            .map(Vec::len),
        Some(4)
    );
    de(&mut app, Event::Close);
    assert_eq!(app.dialog, Some(Dialog::StyleManager));
}

#[test]
fn the_list_s_edits_and_their_undo() {
    let (mut app, _dir) = with_manager("liste");
    sm(&mut app, ManagerEvent::NewSymbol("line"));
    assert_eq!(designer(&app).draft.name, "Yeni çizgi sembolü");
    de(&mut app, Event::Add("markerLine", None));
    assert_eq!(designer(&app).selected, LayerPath::Top(1));
    // A layer that places markers takes marker layers too.
    de(&mut app, Event::Add("text", Some(1)));
    assert_eq!(designer(&app).selected, LayerPath::Child(1, 1));
    assert_eq!(layers(&app)[1]["marker"]["layers"][1]["type"], "text");
    de(&mut app, Event::Up);
    assert_eq!(designer(&app).selected, LayerPath::Child(1, 0));
    assert_eq!(layers(&app)[1]["marker"]["layers"][0]["type"], "text");
    de(&mut app, Event::Duplicate);
    assert_eq!(designer(&app).selected, LayerPath::Child(1, 1));
    assert_eq!(layers(&app)[1]["marker"]["layers"][1]["id"], "2");
    de(&mut app, Event::Select(LayerPath::Top(0)));
    de(&mut app, Event::Down);
    assert_eq!(types(&app), ["markerLine", "simpleLine"]);
    de(&mut app, Event::Remove);
    assert_eq!(types(&app), ["markerLine"]);
    // The last layer stays.
    de(&mut app, Event::Select(LayerPath::Top(0)));
    de(&mut app, Event::Remove);
    assert_eq!(types(&app), ["markerLine"]);
    assert_eq!(said(&app), "Sembolün en az bir katmanı olmalı.");
    // Switched off and on.
    de(&mut app, Event::Enabled(LayerPath::Top(0), false));
    assert_eq!(layers(&app)[0]["enabled"], json!(false));
    de(&mut app, Event::Enabled(LayerPath::Top(0), true));
    assert!(layers(&app)[0].get("enabled").is_none());
    // Undo goes back step by step, redo forward.
    de(&mut app, Event::Undo);
    assert_eq!(layers(&app)[0]["enabled"], json!(false));
    de(&mut app, Event::Undo);
    de(&mut app, Event::Undo);
    assert_eq!(types(&app), ["markerLine", "simpleLine"]);
    de(&mut app, Event::Redo);
    assert_eq!(types(&app), ["markerLine"]);
    // ↑ and ↓ with no field holding the keyboard move along the rows.
    de(&mut app, Event::Undo);
    de(&mut app, Event::Select(LayerPath::Top(0)));
    de(
        &mut app,
        Event::Arrow {
            up: false,
            shift: false,
            focus: Focus::default(),
        },
    );
    assert_eq!(designer(&app).selected, LayerPath::Child(0, 0));
    // A field holding the keyboard keeps ↑ ↓ from the list.
    de(
        &mut app,
        Event::Arrow {
            up: false,
            shift: false,
            focus: Focus { id: None, any: true },
        },
    );
    assert_eq!(designer(&app).selected, LayerPath::Child(0, 0));
}

#[test]
fn fields_as_typed() {
    let (mut app, _dir) = with_manager("alanlar");
    sm(&mut app, ManagerEvent::NewSymbol("line"));
    let width = |app: &App| layers(app)[0]["width"].clone();
    // A comma is the point; the text stays as typed.
    typed(&mut app, "width", "0,5", Some(set("width", json!(0.5))));
    assert_eq!(width(&app), json!(0.5));
    assert_eq!(designer(&app).typed.get("width").map(String::as_str), Some("0,5"));
    // Enter gives the value's own text back.
    de(&mut app, Event::Settle("width".into()));
    assert!(designer(&app).typed.get("width").is_none());
    // ↑ with the field holding the keyboard: a tenth up, ten with Shift.
    de(
        &mut app,
        Event::Arrow {
            up: true,
            shift: false,
            focus: Focus {
                id: Some(field_id("width")),
                any: true,
            },
        },
    );
    assert_eq!(width(&app), json!(0.6));
    de(
        &mut app,
        Event::Arrow {
            up: false,
            shift: true,
            focus: Focus {
                id: Some(field_id("width")),
                any: true,
            },
        },
    );
    // Not below the field's bound.
    assert_eq!(width(&app), json!(0));
    // Typing within a second is one undo step; Enter ends it.
    de(&mut app, Event::Settle("width".into()));
    typed(&mut app, "width", "1", Some(set("width", json!(1))));
    typed(&mut app, "width", "1.2", Some(set("width", json!(1.2))));
    de(&mut app, Event::Undo);
    assert_eq!(width(&app), json!(0));
    // An unknown field is left alone.
    de(
        &mut app,
        Event::Arrow {
            up: true,
            shift: false,
            focus: Focus {
                id: Some(Id::from("başka")),
                any: true,
            },
        },
    );
    assert_eq!(width(&app), json!(0));
}

#[test]
fn a_system_symbol_is_copied_and_the_copy_opens() {
    let (mut app, _dir) = with_manager("kopya");
    sm(&mut app, ManagerEvent::Press(DOUBLE.into()));
    sm(&mut app, ManagerEvent::Edit(DOUBLE.into()));
    assert_eq!(app.dialog, Some(Dialog::SymbolDesigner));
    let Target::Library {
        id: Some(id),
        source: Source::User,
    } = designer(&app).target.clone()
    else {
        panic!("a copy in Kitaplığım");
    };
    assert_ne!(id, DOUBLE);
    let (item, source) = app.styles.library.get(&id).expect("the copy");
    assert_eq!(source, Source::User);
    assert_eq!(item.path().first(), Some(&"Sembollerim"));
    assert_eq!(designer(&app).draft.name, item.name());
    assert_eq!(designer(&app).title(), "Çizgi sembolü tasarımcısı");
    assert!(!designer(&app).dirty());
    // The system symbol itself stays as it was.
    assert!(!app.styles.library.can_edit(DOUBLE));
}

#[test]
fn closing_with_changes_asks_first() {
    let (mut app, _dir) = with_manager("soru");
    sm(&mut app, ManagerEvent::NewSymbol("marker"));
    de(&mut app, Event::Add("text", None));
    // Esc asks; Esc again answers “stay”.
    app.close_dialog();
    assert_eq!(app.dialog, Some(Dialog::SymbolDesigner));
    assert!(designer(&app).asking);
    app.close_dialog();
    assert!(!designer(&app).asking);
    assert_eq!(app.dialog, Some(Dialog::SymbolDesigner));
    // Kaydetmeden kapat.
    de(&mut app, Event::Close);
    de(&mut app, Event::Discard);
    assert!(app.styles.designer.is_none());
    assert_eq!(app.dialog, Some(Dialog::StyleManager));
    let before = app.styles.library.items(Some(Source::User)).len();
    // Kaydet ve kapat.
    sm(&mut app, ManagerEvent::NewSymbol("marker"));
    de(&mut app, Event::Add("text", None));
    de(&mut app, Event::Close);
    de(&mut app, Event::SaveAndClose);
    assert!(app.styles.designer.is_none());
    assert_eq!(
        app.styles.library.items(Some(Source::User)).len(),
        before + 1
    );
}

#[test]
fn a_symbol_that_is_not_valid_is_not_saved() {
    let (mut app, _dir) = with_manager("gecersiz");
    sm(&mut app, ManagerEvent::NewSymbol("fill"));
    let at = designer(&app).selected;
    de(
        &mut app,
        Event::Edit(Edit {
            at,
            key: "color".into(),
            text: None,
            patch: Some(set("color", Value::Null)),
            focus: None,
        }),
    );
    de(&mut app, Event::Save);
    assert_eq!(
        said(&app),
        "Kaydedilemedi: sembol › katman 1: renk geçerli bir renk değil (#RRGGBB, #RRGGBBAA, ink, paper, fg, fg-dim)"
    );
    assert!(designer(&app).dirty());
}

#[test]
fn a_layer_style_s_symbol_is_handed_back_with_uygula() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::LayerStyle(LayerEvent::Open(Some("parsel".into()))));
    let _ = app.update(Message::LayerStyle(LayerEvent::Kind(Kind::Single)));
    let start = super::slot_symbol(
        &app.styles.library,
        None,
        None,
        GeometryClass::Fill,
    )
    .expect("the default");
    let _ = app.update(Message::LayerStyle(LayerEvent::Design(
        SetAt::Single,
        GeometryClass::Fill,
        "Tek sembol (alan)".into(),
        start,
    )));
    assert_eq!(app.dialog, Some(Dialog::SymbolDesigner));
    assert_eq!(designer(&app).title(), "Tek sembol (alan): alan sembolü");
    de(&mut app, Event::Add("hatchFill", None));
    de(&mut app, Event::Save);
    assert!(app.styles.designer.is_none());
    assert_eq!(app.dialog, Some(Dialog::LayerStyle));
    let window = app.styles.layer_style.as_ref().expect("open");
    let got = window
        .single
        .get(GeometryClass::Fill)
        .expect("the slot's symbol");
    assert_eq!(got["layers"][2]["type"], "hatchFill");
    // Nothing reached the library.
    assert!(app.styles.library.items(Some(Source::User)).is_empty());
}

#[test]
fn a_slot_starts_from_what_it_shows() {
    let app = app_with_drawing();
    let lib = &app.styles.library;
    let simple = json!({ "type": "line", "layers": [{ "id": "l", "type": "simpleLine", "color": "#FF0000", "width": 0.5 }] });
    // The plain look the slot shows, not a default.
    assert_eq!(
        super::slot_symbol(lib, None, Some(&simple), GeometryClass::Line),
        Some(simple.clone())
    );
    // A library symbol is copied.
    let linked = json!({ "ref": DOUBLE });
    assert_eq!(
        super::slot_symbol(lib, Some(&linked), Some(&simple), GeometryClass::Line),
        lib.symbol(DOUBLE).cloned()
    );
    // A missing library symbol cannot be edited.
    let gone = json!({ "ref": "silinmis" });
    assert_eq!(
        super::slot_symbol(lib, Some(&gone), Some(&simple), GeometryClass::Line),
        None
    );
}

#[test]
fn a_picture_comes_in_for_an_image_field() {
    let (mut app, _dir) = with_manager("resim");
    sm(&mut app, ManagerEvent::NewSymbol("marker"));
    de(&mut app, Event::Add("svg", None));
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><script>x()</script><circle cx="5" cy="5" r="4"/></svg>"#;
    de(
        &mut app,
        Event::AssetPicked(
            "asset".into(),
            Some(("agac.svg".into(), svg.to_vec())),
        ),
    );
    assert_eq!(said(&app), "“agac” Kitaplığım'a eklendi.");
    let asset = layers(&app)[1]["asset"].as_str().unwrap_or("").to_owned();
    let item = app.styles.library.asset(&asset).expect("in the library");
    assert_eq!(item.path(), ["Çizimlerim"]);
    assert!(!item.data().unwrap_or("").contains("script"));
    // Text that is not a drawing is refused.
    de(
        &mut app,
        Event::AssetPicked("asset".into(), Some(("not.txt".into(), b"hello".to_vec()))),
    );
    assert_eq!(said(&app), "“not.txt” bir SVG çizimi değil.");
}

#[test]
fn the_preview_s_scale() {
    let (mut app, _dir) = with_manager("olcek");
    sm(&mut app, ManagerEvent::NewSymbol("line"));
    assert_eq!(designer(&app).px_per_mm, 4.0);
    de(&mut app, Event::Wheel(1.0));
    assert_eq!(designer(&app).px_per_mm, 5.0);
    de(&mut app, Event::Zoom(1.0 / 1.25));
    assert_eq!(designer(&app).px_per_mm, 4.0);
    for _ in 0..40 {
        de(&mut app, Event::Wheel(-1.0));
    }
    assert_eq!(designer(&app).px_per_mm, 1.0);
    de(&mut app, Event::RealSize);
    assert!((designer(&app).px_per_mm - 96.0 / 25.4).abs() < 1e-12);
    de(&mut app, Event::Sample(Geometry::Bent));
    assert_eq!(designer(&app).sample, Geometry::Bent);
}

#[test]
fn every_layer_type_s_form_builds() {
    let (mut app, _dir) = with_manager("formlar");
    for (kind, types) in [
        ("fill", &["hatchFill", "patternFill", "imageFill", "simpleLine", "markerLine", "centroidMarker"][..]),
        ("marker", &["shape", "text", "svg", "raster"][..]),
    ] {
        sm(&mut app, ManagerEvent::NewSymbol(kind));
        let _ = app.view();
        for t in types {
            de(&mut app, Event::Add(t, None));
            let _ = app.view();
        }
        // The expressions' fields, a wave and a rectangle.
        de(&mut app, Event::Select(LayerPath::Top(1)));
        let at = designer(&app).selected;
        de(
            &mut app,
            Event::Edit(Edit {
                at,
                key: "color".into(),
                text: None,
                patch: Some(set("color", json!({ "expr": "", "fallback": "ink" }))),
                focus: Some("color:expr".into()),
            }),
        );
        let _ = app.view();
        de(&mut app, Event::Close);
        de(&mut app, Event::Discard);
    }
    sm(&mut app, ManagerEvent::NewSymbol("line"));
    let at = designer(&app).selected;
    de(
        &mut app,
        Event::Edit(Edit {
            at,
            key: "wave".into(),
            text: None,
            patch: Some(set("wave", json!({ "shape": "sine", "length": 5, "amplitude": 0.8, "offsetAlong": 0 }))),
            focus: None,
        }),
    );
    let _ = app.view();
    de(&mut app, Event::Close);
    de(&mut app, Event::Discard);
    // Over Katman stili too.
    let _ = app.update(Message::LayerStyle(LayerEvent::Open(Some("parsel".into()))));
    let _ = app.update(Message::LayerStyle(LayerEvent::Design(
        SetAt::Single,
        GeometryClass::Marker,
        "Tek sembol (nokta)".into(),
        json!({ "type": "marker", "layers": [{ "id": "0", "type": "shape", "shape": "rectangle", "size": 3 }] }),
    )));
    let _ = app.view();
    de(&mut app, Event::Close);
    let _ = app.view();
}

/// A key press as the window gives it.
fn press(app: &mut App, key: iced::keyboard::Key, modifiers: iced::keyboard::Modifiers, text: Option<&str>) {
    use iced::keyboard::key::{NativeCode, Physical};
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key,
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers,
        text: text.map(str::to_owned),
        repeat: false,
    }));
}

#[test]
fn ctrl_z_and_ctrl_y_undo_and_redo_in_the_designer() {
    use iced::keyboard::{Key, Modifiers};
    let (mut app, _dir) = with_manager("tuslar");
    sm(&mut app, ManagerEvent::NewSymbol("fill"));
    de(&mut app, Event::Add("hatchFill", None));
    assert_eq!(types(&app), ["simpleFill", "hatchFill"]);
    press(&mut app, Key::Character("z".into()), Modifiers::CTRL, Some("\u{1a}"));
    assert_eq!(types(&app), ["simpleFill"]);
    press(&mut app, Key::Character("y".into()), Modifiers::CTRL, Some("\u{19}"));
    assert_eq!(types(&app), ["simpleFill", "hatchFill"]);
    press(&mut app, Key::Character("Z".into()), Modifiers::CTRL | Modifiers::SHIFT, Some("\u{1a}"));
    assert_eq!(types(&app), ["simpleFill", "hatchFill"], "nothing more to redo");
    // The drawing's own undo is not reached.
    assert_eq!(app.dialog, Some(Dialog::SymbolDesigner));
    // Esc asks about the changes rather than closing.
    press(&mut app, Key::Named(iced::keyboard::key::Named::Escape), Modifiers::default(), None);
    assert!(designer(&app).asking);
}
