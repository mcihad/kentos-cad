//! Nesne ekleri on the desktop (docs/adr/0225): the attachments' views and
//! writes (Adres ekle, Eki düzenle, Kaldır, Göm, Dosya ekle), Bilgi kartı ve
//! eylemler, the card and the actions filled for an object, Eylemler ▸, and
//! Konumlu fotoğraflar over the EXIF fixtures. Bilgi's clicks are the shared
//! trace's (fixtures/interaction/v1/identify.json). Test code only.

use std::path::PathBuf;

use kentos_contracts::{AttachmentKind, Entity};
use kentos_domain::Slot;

use crate::app::{App, Dialog, Message, Picker};
use crate::attachment_scenes::opened;
use crate::attachments::Event;
use crate::files_testing::{app_with_drawing, drive, last_said, scratch};

fn scene() -> App {
    let mut app = app_with_drawing();
    opened(&mut app);
    app
}

fn attachments(app: &App, slot: Slot) -> Vec<kentos_contracts::Attachment> {
    app.document
        .as_ref()
        .and_then(|d| d.model.get(slot))
        .map(|e| e.base().attachments.clone())
        .unwrap_or_default()
}

fn undo(app: &mut App) -> Option<String> {
    app.document.as_mut().and_then(|d| d.model.undo())
}

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path)
}

#[test]
fn the_views_say_where_each_attachment_is() {
    let app = scene();
    let views = app.attachment_views(Slot(1));
    let seen: Vec<(AttachmentKind, &str, Option<&str>)> = views
        .iter()
        .map(|v| (v.kind, v.source.as_str(), v.key.as_deref()))
        .collect();
    assert_eq!(seen.len(), 4);
    assert_eq!(seen[0].0, AttachmentKind::Photo);
    assert_eq!(seen[0].1, "Gömülü fotoğraf (320×240 piksel)");
    assert!(seen[0].2.is_some_and(|k| k.starts_with("asset:resim-")));
    // The deed beside the drawing is found; a document has no photo key.
    assert_eq!(
        seen[2],
        (
            AttachmentKind::Document,
            "Bağlı dosya: tapu-101-5.pdf",
            None
        )
    );
    assert_eq!(
        seen[3],
        (
            AttachmentKind::Link,
            "https://imar.example.org/ada/101/parsel/5",
            None
        )
    );
}

#[test]
fn an_address_is_added_edited_and_removed_one_step_each() {
    let mut app = scene();
    app.selection.set([Slot(4)]);
    let _ = app.update(Message::Run("attachment.addLink"));
    assert_eq!(app.dialog, Some(Dialog::Attachment));
    assert_eq!(app.attachment_title(), crate::attachments::LINK_TITLE);
    // No address yet: Kaydet writes nothing.
    let _ = app.attachment_event(Event::FormSave);
    assert!(attachments(&app, Slot(4)).is_empty());
    assert_eq!(app.dialog, Some(Dialog::Attachment));
    // Not http or https: still nothing.
    let _ = app.attachment_event(Event::FormUrl("ftp://imar.example.org/x".to_owned()));
    let _ = app.attachment_event(Event::FormSave);
    assert!(attachments(&app, Slot(4)).is_empty());
    // An address without a name takes its host.
    let _ = app.attachment_event(Event::FormUrl(
        "https://imar.example.org/ada/101/parsel/8".to_owned(),
    ));
    let _ = app.attachment_event(Event::FormSave);
    let list = attachments(&app, Slot(4));
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "imar.example.org");
    assert_eq!(
        list[0].url.as_deref(),
        Some("https://imar.example.org/ada/101/parsel/8")
    );
    assert_eq!(app.dialog, None);
    // Düzenle: its name and a note of two lines.
    let _ = app.attachment_event(Event::Edit(Slot(4), 0));
    assert_eq!(app.attachment_title(), crate::attachments::EDIT_TITLE);
    let _ = app.attachment_event(Event::FormName("İmar durumu".to_owned()));
    let _ = app.attachment_event(Event::FormNote(iced::widget::text_editor::Action::Edit(
        iced::widget::text_editor::Edit::Paste(std::sync::Arc::new(
            "Satır bir\nsatır iki".to_owned(),
        )),
    )));
    let _ = app.attachment_event(Event::FormSave);
    let list = attachments(&app, Slot(4));
    assert_eq!(list[0].name, "İmar durumu");
    assert_eq!(list[0].note.as_deref(), Some("Satır bir\nsatır iki"));
    let _ = app.attachment_event(Event::Remove(Slot(4), 0));
    assert!(attachments(&app, Slot(4)).is_empty());
    // Three steps, each “Ekler”, back to none.
    assert_eq!(undo(&mut app).as_deref(), Some("Ekler"));
    assert_eq!(attachments(&app, Slot(4))[0].name, "İmar durumu");
    assert_eq!(undo(&mut app).as_deref(), Some("Ekler"));
    assert_eq!(attachments(&app, Slot(4))[0].name, "imar.example.org");
    assert_eq!(undo(&mut app).as_deref(), Some("Ekler"));
    assert!(attachments(&app, Slot(4)).is_empty());
}

#[test]
fn an_attachment_wants_one_object_on_an_open_layer() {
    let mut app = scene();
    assert!(!app.available("attachment.add"));
    app.selection.set([Slot(1), Slot(2)]);
    assert!(!app.available("attachment.addLink"));
    let _ = app.attachment_command("attachment.addLink");
    assert_eq!(
        last_said(&app),
        "Ek tek bir nesneye eklenir: bir nesne seçin."
    );
    assert_eq!(app.dialog, None);
    // A locked layer's object takes none.
    let doc = app.document.as_mut().expect("open");
    doc.model.toggle_layer_locked("parsel");
    app.selection.set([Slot(4)]);
    let _ = app.attachment_command("attachment.add");
    assert_eq!(
        last_said(&app),
        "“Parsel” katmanı kilitli; nesnesine ek eklenemez. Kilidi Katmanlar panelinden açın."
    );
}

#[test]
fn files_are_linked_beside_the_drawing_and_a_photo_is_embedded() {
    let dir = scratch("ekler");
    let drawing = dir.join("cizim.kcad");
    let photos = dir.join("foto");
    std::fs::create_dir_all(&photos).expect("a folder");
    let photo = photos.join("cephe.jpg");
    std::fs::copy(fixture("exif/v1/konum.jpg"), &photo).expect("the photo");
    let mut app = scene();
    app.document.as_mut().expect("open").path = Some(drawing);
    app.selection.set([Slot(4)]);
    app.picker = Picker::File(photo.clone());
    let task = app.attachment_command("attachment.add");
    drive(&mut app, task);
    let list = attachments(&app, Slot(4));
    assert_eq!(list.len(), 1);
    // Under the drawing's folder: relative, so the folder moves with it.
    assert_eq!(list[0].file.as_deref(), Some("foto/cephe.jpg"));
    assert_eq!(list[0].name, "cephe.jpg");
    assert_eq!(list[0].kind(), AttachmentKind::Photo);
    assert_eq!(last_said(&app), "“cephe.jpg” eklendi.");
    // Göm: the photo in the project's library under Ekler, the attachment embedded, one step.
    let _ = app.attachment_event(Event::Embed(Slot(4), 0));
    let list = attachments(&app, Slot(4));
    let asset = list[0].asset.clone().expect("embedded");
    assert!(list[0].file.is_none());
    let doc = app.document.as_ref().expect("open");
    let item = doc
        .model
        .styles()
        .items
        .iter()
        .find(|it| it.get("id").and_then(|v| v.as_str()) == Some(asset.as_str()))
        .expect("in the library");
    assert_eq!(item["path"], serde_json::json!(["Ekler"]));
    assert_eq!(item["format"], "jpeg");
    assert_eq!(undo(&mut app).as_deref(), Some("Ekler"));
    assert_eq!(
        attachments(&app, Slot(4))[0].file.as_deref(),
        Some("foto/cephe.jpg")
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_card_and_the_actions_are_filled_for_the_object() {
    let app = scene();
    let doc = app.document.as_ref().expect("open");
    let card = app.slot_card(Slot(1)).expect("a card");
    assert_eq!(card.title, "Parsel 101/5");
    assert_eq!(
        card.rows,
        vec![
            ("Mahalle".to_owned(), "Kızılay".to_owned()),
            ("Nitelik".to_owned(), "Kargir ev ve bahçesi".to_owned()),
            ("Malik".to_owned(), "Özel".to_owned()),
        ]
    );
    assert_eq!(
        card.text.as_deref(),
        Some("Hesaplanan alan: 320 m²\nÇevre: 72 m")
    );
    // The layer without a card: the kind and the label, every attribute by name.
    let road = app.slot_card(Slot(8)).expect("a card");
    assert_eq!(road.title, "Çoklu çizgi");
    assert_eq!(road.rows, vec![("Ad".to_owned(), "Gül Sokak".to_owned())]);
    // The place is WGS 84's: the address encodes nothing it does not have to.
    let actions = app.slot_actions(Slot(1));
    assert_eq!(actions.len(), 2);
    let e = doc.model.get(Slot(1)).expect("the parcel");
    let (lat, lon) = app.place_of(e).expect("a place");
    assert!(
        (39.9..40.0).contains(&lat) && (32.8..32.9).contains(&lon),
        "{lat} {lon}"
    );
    let lat = kentos_geometry_core::display::fixed(lat, 8);
    let lon = kentos_geometry_core::display::fixed(lon, 8);
    assert_eq!(
        actions[0].target.as_deref(),
        Some(
            format!("https://www.openstreetmap.org/?mlat={lat}&mlon={lon}#map=19/{lat}/{lon}")
                .as_str()
        )
    );
    assert_eq!(
        actions[0].kind,
        kentos_expression::template::TargetKind::Address
    );
    assert_eq!(
        actions[1].target.as_deref(),
        Some("belgeler/tapu-101-5.pdf")
    );
    assert_eq!(
        actions[1].kind,
        kentos_expression::template::TargetKind::File
    );
    // Documents and pictures open; a program does not.
    assert!(crate::object_info::opens(std::path::Path::new(
        "belgeler/tapu-101-5.pdf"
    )));
    assert!(crate::object_info::opens(std::path::Path::new("FOTO.JPG")));
    assert!(!crate::object_info::opens(std::path::Path::new("kur.sh")));
    assert!(!crate::object_info::opens(std::path::Path::new(
        "setup.exe"
    )));
}

#[test]
fn a_target_that_is_no_address_and_no_file_is_not_opened() {
    let mut app = scene();
    let action = crate::object_info::ObjectAction {
        name: "Betik".to_owned(),
        target: Some("javascript:alert(1)".to_owned()),
        error: None,
        kind: kentos_expression::template::target_kind("javascript:alert(1)"),
    };
    app.run_action(&action);
    assert_eq!(
        last_said(&app),
        "“Betik” eyleminin hedefi açılmadı: “javascript:alert(1)” http, https ya da mailto adresi ya da dosya yolu değil."
    );
    app.open_document("Kurulum", "kur.exe");
    assert!(
        last_said(&app).starts_with("“Kurulum” açılmadı: "),
        "{}",
        last_said(&app)
    );
}

#[test]
fn the_layer_card_window_writes_through_its_command() {
    let mut app = scene();
    app.selection.set([Slot(10)]);
    let _ = app.open_layer_info(Some("foto".to_owned()));
    assert_eq!(app.dialog, Some(Dialog::LayerInfo));
    assert_eq!(
        app.dialog_title().as_deref(),
        Some(crate::layer_info::TITLE)
    );
    let fill = |app: &mut App, label: &'static str, words: &'static str| {
        let message = app
            .dialog_control(crate::traces::Control::Fill(label, words))
            .expect("the field")
            .expect("a message");
        let _ = app.update(message);
    };
    // A broken template: Kaydet does nothing.
    fill(&mut app, "Başlık", "{Ad");
    assert!(
        app.dialog_control(crate::traces::Control::Press("Kaydet"))
            .expect("the button")
            .is_none()
    );
    fill(&mut app, "Başlık", "Fotoğraf {Ad}");
    fill(&mut app, "Alanlar", "Tarih, Kamera");
    let save = app
        .dialog_control(crate::traces::Control::Press("Kaydet"))
        .expect("the button")
        .expect("on");
    let _ = app.update(save);
    assert_eq!(app.dialog, None);
    let card = app.slot_card(Slot(10)).expect("a card");
    assert_eq!(card.title, "Fotoğraf IMG_0412");
    assert_eq!(
        card.rows,
        vec![
            ("Tarih".to_owned(), "2024-05-17T10:32:05+03:00".to_owned()),
            ("Kamera".to_owned(), "Canon EOS R5".to_owned()),
        ]
    );
    assert_eq!(
        last_said(&app),
        "“Fotoğraflar” katmanının bilgi kartı ve eylemleri yazıldı."
    );
    assert_eq!(undo(&mut app).as_deref(), Some("Bilgi kartı"));
    assert_eq!(app.slot_card(Slot(10)).expect("a card").title, "IMG_0412");
}

#[test]
fn the_drawing_menu_offers_the_layers_actions() {
    let mut app = scene();
    let hover = |app: &mut App, p: [f64; 2]| crate::tools_scenes::hover(app, p);
    hover(&mut app, [-6.0, 2.0]);
    assert_eq!(app.selection.hover(), Some(Slot(1)));
    assert!(app.menu_actions().is_some());
    // The building's layer has none.
    hover(&mut app, [-14.0, 8.0]);
    assert_eq!(app.selection.hover(), Some(Slot(5)));
    assert!(app.menu_actions().is_none());
}

#[test]
fn geotagged_photos_become_points_with_their_photos() {
    let mut app = scene();
    let files = ["konum.jpg", "gps-saati.jpg", "gpssiz.jpg", "yarim-gps.jpg"]
        .into_iter()
        .map(|name| {
            let path = fixture(&format!("exif/v1/{name}"));
            crate::exchange::photo_import::PhotoFile {
                name: name.to_owned(),
                bytes: std::fs::read(&path).expect("the fixture").into(),
                path,
            }
        })
        .collect();
    app.photo_import_now(files);
    let plan = app
        .photo_import_state()
        .and_then(|s| s.plan())
        .expect("placed")
        .clone();
    assert_eq!(plan.placed.len(), 2);
    assert_eq!(plan.skipped.len(), 2);
    assert_eq!(
        plan.skipped[0].message,
        "gpssiz.jpg: Fotoğrafta GPS konumu yok. Eklenmedi."
    );
    // Linked by default: outside the drawing's folder, the whole path.
    assert!(
        plan.placed[0]
            .attachment
            .file
            .as_deref()
            .is_some_and(|f| f.ends_with("exif/v1/konum.jpg"))
    );
    // Gömülü: the photos' images instead, into the library when they are written.
    let _ = app.update(Message::Exchange(Box::new(
        crate::exchange::Event::PhotoImport(crate::exchange::photo_import::Event::Embed),
    )));
    let plan = app
        .photo_import_state()
        .and_then(|s| s.plan())
        .expect("placed")
        .clone();
    let asset = plan.placed[0].attachment.asset.clone().expect("embedded");
    let before = app
        .document
        .as_ref()
        .map_or(0, |d| d.model.entities().count());
    let _ = app.update(Message::Exchange(Box::new(
        crate::exchange::Event::PhotoImport(crate::exchange::photo_import::Event::Run),
    )));
    let doc = app.document.as_ref().expect("open");
    assert_eq!(doc.model.entities().count(), before + 2);
    let made: Vec<&Entity> = doc.model.entities().skip(before).collect();
    let Entity::Point(p) = made[0] else {
        panic!("a point");
    };
    assert_eq!(p.base.label.as_deref(), Some("konum"));
    assert_eq!(p.base.attrs.get("Yön").map(String::as_str), Some("135.25"));
    assert_eq!(p.base.attachments[0].asset.as_deref(), Some(asset.as_str()));
    assert!(
        doc.model
            .styles()
            .items
            .iter()
            .any(|it| it.get("id").and_then(|v| v.as_str()) == Some(asset.as_str()))
    );
    assert!(app.exchange.is_none());
    assert_eq!(undo(&mut app).as_deref(), Some("Konumlu fotoğraflar"));
}

/// One card opened: its title, rows and text, its actions and its attachments
/// (docs/adr/0225 §7: ≤ 16 ms; release, by hand).
#[test]
#[ignore = "timing; run with --ignored --nocapture in release"]
fn a_card_opens_in_a_frame() {
    let app = scene();
    let mut best = f64::INFINITY;
    for _ in 0..200 {
        let start = std::time::Instant::now();
        let card = app.slot_card(Slot(1)).expect("a card");
        let actions = app.slot_actions(Slot(1));
        let views = app.attachment_views(Slot(1));
        std::hint::black_box((card, actions, views));
        best = best.min(start.elapsed().as_secs_f64() * 1e3);
    }
    println!("Bilgi kartı: bir kartın açılması {best:.3} ms");
    assert!(best <= 16.0, "{best} ms");
}
