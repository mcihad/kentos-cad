//! Stil yöneticisi as the user drives it: browsing and searching the
//! library, copies kept in Kitaplığım's file, fields written when they are
//! left, deleting after the question, symbols given to the selection in one
//! undo step, picking for Katman stili's slots, the project's library kept
//! in the drawing, imports with their conflicts, categories in the tree,
//! pictures taken in, file names.

use iced::keyboard::key::Named;
use iced::widget::text_editor::{Action, Edit};
use kentos_native_style::file::{ConflictMode, export_styles};
use kentos_native_style::library::{ItemKind, Source};
use kentos_native_style::renderer::GeometryClass;
use serde_json::json;

use super::files::{base64, file_slug, raster_asset};
use super::{Event, Field, KindFilter, PickTarget, node_key};
use crate::app::{App, Dialog, Message};
use crate::files_testing::{app_with_drawing, last_said, scratch};
use crate::style::layer_style::{Event as LayerEvent, SetAt};

const DOUBLE: &str = "temel.cizgi.cift";
const FILL: &str = "temel.alan.dolu";

fn sm(app: &mut App, e: Event) {
    let _ = app.update(Message::StyleManager(Box::new(e)));
}

fn manager(app: &App) -> &super::Manager {
    app.styles.manager.as_ref().expect("the window is open")
}

fn listed(app: &App) -> Vec<String> {
    manager(app)
        .listed(&app.styles.library)
        .iter()
        .map(|(id, _)| id.clone())
        .collect()
}

fn said(app: &App) -> String {
    manager(app)
        .said()
        .map(|(t, _)| t.clone())
        .unwrap_or_default()
}

/// The app with the sample drawing and Kitaplığım kept in a scratch folder.
fn app_with_library(name: &str) -> (App, std::path::PathBuf) {
    let mut app = app_with_drawing();
    let dir = scratch(name);
    assert_eq!(app.styles.open_user_library(&dir), None);
    (app, dir)
}

fn user_file(dir: &std::path::Path) -> serde_json::Value {
    let text = std::fs::read_to_string(dir.join(crate::style::user_library::FILE))
        .expect("Kitaplığım's file is written");
    serde_json::from_str(&text).expect("it is JSON")
}

#[test]
fn opens_on_the_system_symbols_and_searches_across_the_library() {
    let (mut app, _dir) = app_with_library("ara");
    let _ = app.update(Message::Run("style.manager"));
    assert_eq!(app.dialog, Some(Dialog::StyleManager));
    let m = manager(&app);
    assert_eq!(m.at, (Source::System, Vec::new()));
    assert!(m.pick.is_none());
    let system = app.styles.library.items(Some(Source::System)).len();
    assert_eq!(listed(&app).len(), system);

    // A category shows everything under it.
    sm(
        &mut app,
        Event::Go(Source::System, vec!["Temel".into()], true),
    );
    assert_eq!(listed(&app).len(), 22);
    sm(&mut app, Event::Kind(KindFilter::Fill));
    assert_eq!(listed(&app).len(), 5);
    sm(&mut app, Event::Kind(KindFilter::All));

    // The search spans the sources, folds Turkish letters and leaves the kinds as chosen.
    sm(&mut app, Event::Search("CIFT".into()));
    let found = listed(&app);
    assert!(found.contains(&DOUBLE.to_owned()), "{found:?}");
    sm(&mut app, Event::Kind(KindFilter::Marker));
    assert!(!listed(&app).contains(&DOUBLE.to_owned()));
    sm(&mut app, Event::Search("proje sembolü".into()));
    assert!(listed(&app).contains(&"proje.sembol".to_owned()));
    sm(&mut app, Event::Search("yokböylebirsembol".into()));
    assert!(listed(&app).is_empty());

    // Esc closes it.
    app.close_dialog();
    assert_eq!(app.dialog, None);
    assert!(app.styles.manager.is_none());
}

#[test]
fn a_press_on_a_category_opens_it_and_a_second_closes_it() {
    let mut app = app_with_drawing();
    let _ = app.update(Message::Run("style.manager"));
    let key = node_key(Source::System, &["Temel".to_owned()]);
    sm(
        &mut app,
        Event::Go(Source::System, vec!["Temel".into()], true),
    );
    assert!(manager(&app).expanded.contains(&key));
    sm(
        &mut app,
        Event::Go(Source::System, vec!["Temel".into()], true),
    );
    assert!(
        !manager(&app).expanded.contains(&key),
        "a second press closes it"
    );
    assert_eq!(manager(&app).at.1, vec!["Temel".to_owned()]);
    // Its arrow opens it without choosing it.
    sm(&mut app, Event::Go(Source::System, Vec::new(), true));
    sm(&mut app, Event::Toggle(key.clone()));
    assert!(manager(&app).expanded.contains(&key));
    assert!(manager(&app).at.1.is_empty());
}

#[test]
fn a_system_symbol_is_copied_to_kitapligim_and_kept_in_its_file() {
    let (mut app, dir) = app_with_library("kopya");
    let _ = app.update(Message::Run("style.manager"));
    sm(&mut app, Event::Press(DOUBLE.into()));
    assert_eq!(manager(&app).selected.as_deref(), Some(DOUBLE));
    // A system item cannot be deleted: Delete asks nothing.
    let _ = app.style_manager_key(Named::Delete);
    assert!(manager(&app).deleting.is_none());

    sm(&mut app, Event::Copy(DOUBLE.into(), Source::User));
    let copy = manager(&app).selected.clone().expect("the copy is chosen");
    let (item, source) = app.styles.library.get(&copy).expect("in the library");
    assert_eq!(source, Source::User);
    assert_eq!(item.name(), "Çift çizgi (kopya)");
    assert!(copy.starts_with('u'), "{copy}");
    assert_eq!(
        said(&app),
        "“Çift çizgi” Kitaplığım kitaplığına kopyalandı."
    );
    let file = user_file(&dir);
    assert_eq!(file["format"], "kentos-style");
    assert_eq!(file["items"][0]["id"], json!(copy));

    // Opened again, the window starts at Kitaplığım.
    app.close_dialog();
    let _ = app.update(Message::Run("style.manager"));
    assert_eq!(manager(&app).at, (Source::User, Vec::new()));
    assert_eq!(listed(&app), vec![copy]);
}

#[test]
fn typed_fields_are_written_when_they_are_left() {
    let (mut app, dir) = app_with_library("alanlar");
    let _ = app.update(Message::Run("style.manager"));
    sm(&mut app, Event::Copy(DOUBLE.into(), Source::User));
    let copy = manager(&app).selected.clone().expect("chosen");

    sm(
        &mut app,
        Event::Field(Field::Name, "  Benim çizgim ".into()),
    );
    sm(&mut app, Event::Field(Field::Tags, "yol, , sınır".into()));
    sm(
        &mut app,
        Event::Field(Field::Reference, "Yönetmelik s. 3".into()),
    );
    // Açıklama is a box of several lines.
    for c in "İki\nsatır".chars() {
        let edit = if c == '\n' {
            Edit::Enter
        } else {
            Edit::Insert(c)
        };
        sm(&mut app, Event::Describe(Action::Edit(edit)));
    }
    // Nothing is written while typing…
    assert_eq!(
        app.styles
            .library
            .get(&copy)
            .map(|(i, _)| i.name().to_owned()),
        Some("Çift çizgi (kopya)".into())
    );
    // …and all of it when another item is chosen.
    sm(&mut app, Event::Press(FILL.into()));
    let (item, _) = app.styles.library.get(&copy).expect("still there");
    assert_eq!(item.name(), "Benim çizgim");
    assert_eq!(item.tags(), vec!["yol", "sınır"]);
    assert_eq!(item.text("reference"), Some("Yönetmelik s. 3"));
    assert_eq!(item.text("description"), Some("İki\nsatır"));
    assert_eq!(user_file(&dir)["items"][0]["name"], "Benim çizgim");

    // Cleared, a field goes (the web kept the old text).
    sm(&mut app, Event::Press(copy.clone()));
    sm(&mut app, Event::Field(Field::Reference, " ".into()));
    sm(&mut app, Event::Commit(Field::Reference));
    let (item, _) = app.styles.library.get(&copy).expect("there");
    assert_eq!(item.text("reference"), None);
    assert!(user_file(&dir)["items"][0].get("reference").is_none());

    // Kategori's Enter moves it, and the list goes with it.
    sm(
        &mut app,
        Event::Field(Field::Path, "Sembollerim / Yollar".into()),
    );
    sm(&mut app, Event::Commit(Field::Path));
    let (item, _) = app.styles.library.get(&copy).expect("there");
    assert_eq!(item.path(), vec!["Sembollerim", "Yollar"]);
    let m = manager(&app);
    assert_eq!(
        m.at,
        (
            Source::User,
            vec!["Sembollerim".to_owned(), "Yollar".to_owned()]
        )
    );
    assert!(
        m.expanded
            .contains(&node_key(Source::User, &["Sembollerim".to_owned()]))
    );

    // Closing writes what was typed too; a name left blank is “Adsız”.
    sm(&mut app, Event::Field(Field::Name, "Son ad".into()));
    app.close_dialog();
    assert_eq!(
        app.styles
            .library
            .get(&copy)
            .map(|(i, _)| i.name().to_owned()),
        Some("Son ad".into())
    );
    let _ = app.update(Message::Run("style.manager"));
    sm(&mut app, Event::Press(copy.clone()));
    sm(&mut app, Event::Field(Field::Name, "   ".into()));
    sm(&mut app, Event::Commit(Field::Name));
    assert_eq!(
        app.styles
            .library
            .get(&copy)
            .map(|(i, _)| i.name().to_owned()),
        Some("Adsız".into())
    );
}

#[test]
fn deleting_asks_first_and_the_answer_decides() {
    let (mut app, dir) = app_with_library("sil");
    let _ = app.update(Message::Run("style.manager"));
    sm(&mut app, Event::Copy(FILL.into(), Source::User));
    let copy = manager(&app).selected.clone().expect("chosen");

    // Delete asks; Esc answers the question, the window stays.
    let _ = app.style_manager_key(Named::Delete);
    assert_eq!(manager(&app).deleting.as_deref(), Some(copy.as_str()));
    app.close_dialog();
    assert_eq!(app.dialog, Some(Dialog::StyleManager));
    assert!(manager(&app).deleting.is_none());
    assert!(app.styles.library.get(&copy).is_some());

    // Sil and Enter delete it.
    sm(&mut app, Event::Delete(copy.clone()));
    let _ = app.style_manager_key(Named::Enter);
    assert!(app.styles.library.get(&copy).is_none());
    assert_eq!(manager(&app).selected, None);
    assert_eq!(said(&app), "“Düz dolgu (kopya)” silindi.");
    assert_eq!(user_file(&dir)["items"], json!([]));
}

#[test]
fn style_assign_gives_the_selection_its_symbol_in_one_undo_step() {
    let mut app = app_with_drawing();
    let slots: Vec<kentos_domain::Slot> = {
        let model = &app.document.as_ref().expect("open").model;
        model
            .entities()
            .filter(|e| e.base().layer_id == "cizim" && e.kind() == "line")
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect()
    };
    assert!(!slots.is_empty());
    assert!(!app.available("style.assign"));
    app.selection.set(slots.clone());
    assert!(app.available("style.assign"));
    assert!(!app.available("style.clearSymbol"));

    let _ = app.update(Message::Run("style.assign"));
    assert_eq!(app.dialog, Some(Dialog::StyleManager));
    let m = manager(&app);
    let pick = m.pick.as_ref().expect("pick mode");
    assert_eq!(pick.target, PickTarget::Selection);
    assert_eq!(pick.kind, Some(KindFilter::Line));
    assert_eq!(m.kind, KindFilter::Line);
    // A drawing cannot be picked for objects; a symbol can.
    assert!(!m.pickable(&app.styles.library, "missing"));
    sm(&mut app, Event::Press(DOUBLE.into()));
    let _ = app.style_manager_key(Named::Enter);
    assert_eq!(app.dialog, None);
    let symbols = |app: &App| -> Vec<Option<String>> {
        let model = &app.document.as_ref().expect("open").model;
        slots
            .iter()
            .map(|&s| model.get(s).and_then(|e| e.base().symbol.clone()))
            .collect()
    };
    assert!(symbols(&app).iter().all(|s| s.as_deref() == Some(DOUBLE)));
    assert_eq!(
        last_said(&app),
        format!("{} nesneye “Çift çizgi” verildi.", slots.len())
    );

    // Taken away again, then both undone step by step.
    assert!(app.available("style.clearSymbol"));
    let _ = app.update(Message::Run("style.clearSymbol"));
    assert!(symbols(&app).iter().all(Option::is_none));
    assert_eq!(
        last_said(&app),
        format!("{} nesnenin sembolü kaldırıldı.", slots.len())
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert!(symbols(&app).iter().all(|s| s.as_deref() == Some(DOUBLE)));
    let _ = app.update(Message::Run("edit.undo"));
    assert!(symbols(&app).iter().all(Option::is_none));

    // On a locked layer nothing changes, and the reason is said.
    app.document
        .as_mut()
        .expect("open")
        .model
        .toggle_layer_locked("cizim");
    let said = app.assign_symbol(Some(DOUBLE));
    assert_eq!(
        said,
        format!(
            "0 nesneye “Çift çizgi” verildi; kilitli katmandaki {} nesne atlandı.",
            slots.len()
        )
    );
    assert!(symbols(&app).iter().all(Option::is_none));
}

#[test]
fn katman_stili_picks_from_the_library_and_keeps_its_own_symbols() {
    let (mut app, dir) = app_with_library("katman");
    let _ = app.update(Message::LayerStyle(LayerEvent::Open(Some("parsel".into()))));
    let _ = app.update(Message::LayerStyle(LayerEvent::Kind(
        crate::style::layer_style::Kind::Single,
    )));
    let _ = app.update(Message::LayerStyle(LayerEvent::Pick(
        SetAt::Single,
        GeometryClass::Fill,
        "Tek sembol (alan)".into(),
        None,
    )));
    assert_eq!(app.dialog, Some(Dialog::StyleManager));
    let m = manager(&app);
    let pick = m.pick.as_ref().expect("pick mode");
    assert_eq!(pick.title, "Tek sembol (alan): sembol seçin");
    assert_eq!(
        pick.target,
        PickTarget::Slot(SetAt::Single, GeometryClass::Fill)
    );
    assert_eq!(m.kind, KindFilter::Fill);

    // Esc goes back to Katman stili, which stays open.
    app.close_dialog();
    assert_eq!(app.dialog, Some(Dialog::LayerStyle));
    assert!(app.styles.layer_style.is_some());

    // A double press picks.
    let _ = app.update(Message::LayerStyle(LayerEvent::Pick(
        SetAt::Single,
        GeometryClass::Fill,
        "Tek sembol (alan)".into(),
        None,
    )));
    sm(&mut app, Event::Press(FILL.into()));
    sm(&mut app, Event::Press(FILL.into()));
    assert_eq!(app.dialog, Some(Dialog::LayerStyle));
    let window = app.styles.layer_style.as_ref().expect("open");
    assert_eq!(
        window.single.get(GeometryClass::Fill),
        Some(&json!({ "ref": FILL }))
    );

    // Opened again from the slot, the manager starts at the slot's symbol.
    let _ = app.update(Message::LayerStyle(LayerEvent::Pick(
        SetAt::Single,
        GeometryClass::Fill,
        "Tek sembol (alan)".into(),
        Some(FILL.into()),
    )));
    let m = manager(&app);
    assert_eq!(m.selected.as_deref(), Some(FILL));
    assert_eq!(
        m.at,
        (
            Source::System,
            vec!["Temel".to_owned(), "Alanlar".to_owned()]
        )
    );
    app.close_dialog();

    // A symbol written into the style goes to Kitaplığım, and the slot refers to it.
    let own = json!({ "type": "fill", "layers": [{ "id": "0", "type": "simpleFill", "color": "#C9D6E3" }] });
    let _ = app.update(Message::LayerStyle(LayerEvent::Keep(
        SetAt::Single,
        GeometryClass::Fill,
        "Tek sembol (alan)".into(),
        own.clone(),
    )));
    let kept = user_file(&dir)["items"][0].clone();
    assert_eq!(kept["name"], "Tek sembol (alan)");
    assert_eq!(kept["path"], json!(["Sembollerim"]));
    assert_eq!(kept["symbol"], own);
    let window = app.styles.layer_style.as_ref().expect("open");
    assert_eq!(
        window.single.get(GeometryClass::Fill),
        Some(&json!({ "ref": kept["id"] }))
    );
    assert_eq!(
        window.status(),
        Some(("Değişiklikler henüz uygulanmadı.".into(), true))
    );
    assert_eq!(
        last_said(&app),
        "“Tek sembol (alan)” Kitaplığım'a kaydedildi."
    );
}

#[test]
fn the_project_s_library_lives_in_the_drawing() {
    let mut app = app_with_drawing();
    let styles = |app: &App| app.document.as_ref().expect("open").model.styles().clone();
    assert_eq!(styles(&app).items.len(), 1);
    assert!(!app.document.as_ref().expect("open").model.is_dirty());
    let _ = app.update(Message::Run("style.manager"));
    sm(&mut app, Event::Copy(DOUBLE.into(), Source::Project));
    let copy = manager(&app).selected.clone().expect("chosen");
    assert!(copy.starts_with('p'), "{copy}");
    assert_eq!(said(&app), "“Çift çizgi” Proje kitaplığına kopyalandı.");
    let model = &app.document.as_ref().expect("open").model;
    assert_eq!(model.styles().items.len(), 2);
    // An edit of the drawing, not an undo step: the library keeps no undo.
    assert!(model.is_dirty());
    assert!(!model.can_undo());

    sm(&mut app, Event::Field(Field::Name, "Proje çizgisi".into()));
    sm(&mut app, Event::Commit(Field::Name));
    assert_eq!(styles(&app).items[1]["name"], "Proje çizgisi");
    sm(&mut app, Event::Delete(copy.clone()));
    sm(&mut app, Event::DeleteConfirmed);
    assert_eq!(styles(&app).items.len(), 1);
}

#[test]
fn a_style_file_is_offered_then_taken_in_with_its_conflicts() {
    let (mut app, dir) = app_with_library("ice");
    let _ = app.update(Message::Run("style.manager"));
    let text = export_styles(&app.styles.library, &[DOUBLE, FILL]).to_string();

    app.offer_import("paylasilan.kstil", &text, false);
    let draft = manager(&app).import.as_ref().expect("the panel");
    assert_eq!(draft.file.items.len(), 2);
    assert_eq!((draft.to, draft.mode), (Source::User, ConflictMode::Copy));
    // Vazgeç takes nothing in.
    sm(&mut app, Event::ImportCancel);
    assert!(manager(&app).import.is_none());

    // Üzerine yaz writes over the target library's items only: the system's are left out.
    app.offer_import("paylasilan.kstil", &text, false);
    sm(&mut app, Event::ImportMode(ConflictMode::Replace));
    sm(&mut app, Event::ImportRun);
    assert_eq!(said(&app), "0 öğe eklendi, 2 atlandı.");
    assert!(
        app.styles
            .library
            .get(DOUBLE)
            .is_some_and(|(_, s)| s == Source::System)
    );
    // Kopya olarak al takes them in under new ids.
    app.offer_import("paylasilan.kstil", &text, false);
    sm(&mut app, Event::ImportRun);
    assert_eq!(said(&app), "2 öğe eklendi.");
    assert_eq!(manager(&app).at, (Source::User, Vec::new()));
    let items = user_file(&dir)["items"].clone();
    assert_eq!(items.as_array().map(Vec::len), Some(2));
    assert!(
        items[0]["id"]
            .as_str()
            .is_some_and(|id| id.starts_with('u'))
    );
    assert_eq!(items[0]["name"], "Çift çizgi");

    // Text that is not a style says why.
    app.offer_import("bozuk.kstil", "{ bozuk", false);
    assert_eq!(
        said(&app),
        "“bozuk.kstil” okunamadı: Dosya okunamadı: JSON değil."
    );
    app.offer_import("Pano", "merhaba", true);
    assert_eq!(
        said(&app),
        "Panodaki metin bir KentOS stili değil: Dosya okunamadı: JSON değil."
    );
    assert!(manager(&app).import.is_none());
}

#[test]
fn categories_are_made_and_renamed_in_the_tree() {
    let (mut app, _dir) = app_with_library("kategori");
    let _ = app.update(Message::Run("style.manager"));
    sm(&mut app, Event::Copy(DOUBLE.into(), Source::User));
    let copy = manager(&app).selected.clone().expect("chosen");
    sm(&mut app, Event::Field(Field::Path, "Sembollerim".into()));
    sm(&mut app, Event::Commit(Field::Path));

    // Yeni alt kategori: named at once, in place.
    let parent = vec!["Sembollerim".to_owned()];
    sm(&mut app, Event::NewCategory(Source::User, parent.clone()));
    let (source, path, name) = manager(&app).renaming.clone().expect("renaming");
    assert_eq!((source, name.as_str()), (Source::User, "Yeni kategori"));
    assert_eq!(
        path,
        vec!["Sembollerim".to_owned(), "Yeni kategori".to_owned()]
    );
    sm(&mut app, Event::RenameInput("Yollar".into()));
    sm(&mut app, Event::RenameDone);
    let trees = manager(&app).trees(&app.styles.library);
    let user = &trees
        .iter()
        .find(|(s, _)| *s == Source::User)
        .expect("user")
        .1;
    assert_eq!(user[0].name, "Sembollerim");
    assert_eq!(user[0].children[0].name, "Yollar");

    // F2 renames the chosen category (Kategori's Enter chose it); the list
    // stays on it under its new name, open as it was.
    assert_eq!(manager(&app).at, (Source::User, parent.clone()));
    let _ = app.style_manager_key(Named::F2);
    assert_eq!(
        manager(&app).renaming.as_ref().map(|r| r.2.as_str()),
        Some("Sembollerim")
    );
    sm(&mut app, Event::RenameInput("Benimkiler".into()));
    sm(&mut app, Event::RenameDone);
    assert_eq!(
        manager(&app).at,
        (Source::User, vec!["Benimkiler".to_owned()])
    );
    assert_eq!(
        app.styles
            .library
            .get(&copy)
            .map(|(i, _)| i.path().join("/")),
        Some("Benimkiler".into())
    );
    assert!(
        manager(&app)
            .expanded
            .contains(&node_key(Source::User, &["Benimkiler".to_owned()]))
    );
    // Esc leaves a rename, not the window.
    let _ = app.style_manager_key(Named::F2);
    app.close_dialog();
    assert!(manager(&app).renaming.is_none());
    assert_eq!(app.dialog, Some(Dialog::StyleManager));
    // System categories are not renamed.
    sm(
        &mut app,
        Event::Go(Source::System, vec!["Temel".into()], true),
    );
    let _ = app.style_manager_key(Named::F2);
    assert!(manager(&app).renaming.is_none());
}

#[test]
fn pictures_come_in_as_images_of_kitapligim() {
    let (mut app, _dir) = app_with_library("resim");
    let _ = app.update(Message::Run("style.manager"));
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 2, 3);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(&[200; 24]).expect("pixels");
    }
    app.take_file("Logo.PNG", &png);
    let id = manager(&app).selected.clone().expect("the image is chosen");
    let (item, source) = app.styles.library.get(&id).expect("taken in");
    assert_eq!(source, Source::User);
    assert_eq!(item.kind(), ItemKind::Asset);
    assert_eq!(item.name(), "Logo");
    assert_eq!(item.format(), Some("png"));
    assert_eq!(item.size(), Some((2.0, 3.0)));
    assert!(
        item.data()
            .is_some_and(|d| d.starts_with("data:image/png;base64,iVBORw0KGgo"))
    );
    assert_eq!(item.path(), vec!["Görüntülerim"]);
    assert_eq!(
        manager(&app).at,
        (Source::User, vec!["Görüntülerim".to_owned()])
    );

    // A broken picture says so; a JPEG is taken, and said not to draw here yet.
    app.take_file("bozuk.png", b"\x89PNG\r\n\x1a\nbozuk");
    assert_eq!(
        said(&app),
        "“bozuk.png” okunamadı: PNG dosyası bozuk görünüyor."
    );
    let jpeg = [
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x20,
        0x00, 0x40, 0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xD9,
    ];
    let asset = raster_asset("foto.jpg", &jpeg)
        .expect("a picture")
        .expect("read");
    assert_eq!(
        (asset["width"].clone(), asset["height"].clone()),
        (json!(64), json!(32))
    );
    assert_eq!(asset["name"], "foto");
    app.take_file("foto.jpg", &jpeg);
    assert!(
        said(&app).contains("JPEG görüntüleri masaüstünde henüz çizilmez"),
        "{}",
        said(&app)
    );
    assert!(raster_asset("notlar.txt", b"merhaba").is_none());
}

#[test]
fn file_names_and_data_addresses_as_the_web_writes_them() {
    assert_eq!(file_slug("Çizgi tipleri"), "cizgi-tipleri");
    assert_eq!(file_slug("İşaret 2"), "isaret-2");
    assert_eq!(file_slug("--Yol--"), "yol");
    assert_eq!(file_slug("  "), "stiller");
    assert_eq!(file_slug("“ara” araması"), "ara-aramasi");
    assert_eq!(base64(b"Man"), "TWFu");
    assert_eq!(base64(b"Ma"), "TWE=");
    assert_eq!(base64(b"M"), "TQ==");
    assert_eq!(base64(b""), "");
}

#[test]
fn every_state_of_the_window_builds() {
    let (mut app, _dir) = app_with_library("gorunum");
    let _ = app.update(Message::Run("style.manager"));
    let _ = app.view();
    sm(&mut app, Event::Press(DOUBLE.into()));
    let _ = app.view();
    sm(&mut app, Event::Copy(DOUBLE.into(), Source::User));
    let copy = manager(&app).selected.clone().expect("chosen");
    let _ = app.view();
    sm(&mut app, Event::Delete(copy));
    let _ = app.view();
    sm(&mut app, Event::DeleteCancelled);
    let text = export_styles(&app.styles.library, &[DOUBLE]).to_string();
    app.offer_import("x.kstil", &text, false);
    let _ = app.view();
    sm(&mut app, Event::ImportCancel);
    sm(&mut app, Event::NewCategory(Source::User, Vec::new()));
    let _ = app.view();
    app.close_dialog();
    app.close_dialog();
    // Pick mode over Katman stili.
    let _ = app.update(Message::LayerStyle(LayerEvent::Open(Some("parsel".into()))));
    let _ = app.update(Message::LayerStyle(LayerEvent::Pick(
        SetAt::Single,
        GeometryClass::Fill,
        "Tek sembol (alan)".into(),
        None,
    )));
    let _ = app.view();
}
