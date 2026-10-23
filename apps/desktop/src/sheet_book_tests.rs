//! The project's sheet book on the desktop (docs/adr/0251; sheet_book.rs):
//! opened from the drawing's file, written into the drawing as it changes,
//! saved and opened again; a book this device kept before moved into its
//! project and set aside once saved; a book the core does not read kept as
//! it is.

use kentos_interaction::Level;
use kentos_sheet_ui::Message as Sheet;

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

const SAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/kcad/v2/sheets.kcad"
);

fn open(app: &mut App, path: &std::path::Path) {
    let doc = Document::read(path).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(app, task);
}

fn app_with_store(name: &str) -> (App, kentos_sheet_ui::Store) {
    let (mut app, _) = App::boot(None);
    let store = kentos_sheet_ui::Store::open(&scratch(name)).expect("a store");
    app.sheet_store = Some(store.clone());
    (app, store)
}

/// The key an older build kept the drawing's book under (its project's id, else its file's name).
fn key_of(doc: &kentos_commands::DocumentSnapshotV2, file: &str) -> String {
    let id = doc
        .project_id
        .map(|p| kentos_domain::Uuid::from_bytes(p.0).to_string());
    kentos_sheet_ui::project_key(None, id.as_deref(), Some(file), "test").id
}

fn sheet(app: &mut App, m: Sheet) {
    let task = app.update(Message::Sheet(m));
    drive(app, task);
}

fn names(app: &App) -> Vec<String> {
    app.sheets
        .book()
        .sheets
        .iter()
        .map(|s| s.name.clone())
        .collect()
}

/// The drawing's book as the drawing holds it, read by the core.
fn kept(app: &App) -> Option<kentos_sheet::model::SheetBook> {
    let doc = app.document.as_ref()?;
    let sheets = doc.model.sheets()?;
    Some(
        kentos_sheet::project::read_project_sheets(sheets)
            .expect("reads")
            .0,
    )
}

fn history_key(app: &mut App, undo: bool) {
    use iced::keyboard::{
        Key, Modifiers,
        key::{Code, Physical},
    };
    let task = app.update(Message::Key(crate::keys::KeyPress {
        key: Key::Character(if undo { "z" } else { "y" }.into()),
        physical: Physical::Code(if undo { Code::KeyZ } else { Code::KeyY }),
        modifiers: if app.sheets.is_active() {
            Modifiers::COMMAND
        } else {
            Modifiers::CTRL
        },
        text: None,
        repeat: false,
    }));
    drive(app, task);
    app.follow_sheet_book();
}

fn combine(app: &mut App) {
    {
        let doc = &mut app.document.as_mut().unwrap().model;
        let answer = kentos_native_application::call::call(
            doc,
            None,
            kentos_commands::CallEnvelope {
                command: "cad.changeset.apply".into(),
                version: None,
                op: kentos_commands::CallOp::Execute,
                request_id: None,
                expected: None,
                relax_constraints: false,
                input: serde_json::json!({"label":"Model ve pafta", "steps":[
                    {"command":"cad.point.create","input":{"layerId":"0","p":{"x":500002,"y":4400003}}},
                    {"command":"cad.sheets.edit","input":{"ops":[{"op":"renameSheet","id":"pafta-1","name":"Birleşik"}]}}
                ]}),
            },
        );
        assert!(
            matches!(
                answer.result,
                kentos_commands::CommandResult::Completed { .. }
            ),
            "{answer:?}"
        );
    }
}

#[test]
fn combined_model_and_paper_undo_follows_in_the_designer_and_preserves_later_edits() {
    let (mut app, _store) = app_with_store("pafta-birlesik-geri-al");
    open(&mut app, std::path::Path::new(SAMPLE));
    let original = app
        .document
        .as_ref()
        .unwrap()
        .model
        .sheets()
        .unwrap()
        .clone();
    let count = app.document.as_ref().unwrap().model.len();
    combine(&mut app);
    app.follow_sheet_book();
    assert_eq!(names(&app), ["Birleşik"]);
    assert_eq!(app.document.as_ref().unwrap().model.len(), count + 1);
    sheet(
        &mut app,
        Sheet::RenameSheet("pafta-1".into(), "Sonraki".into()),
    );
    history_key(&mut app, true);
    app.follow_sheet_book();
    assert!(app.log.said(Level::Warn, "ayrıca değiştirildi"));
    assert_eq!(names(&app), ["Sonraki"]);
    assert_eq!(app.document.as_ref().unwrap().model.len(), count + 1);
    sheet(&mut app, Sheet::Tab(1));
    history_key(&mut app, true);
    sheet(&mut app, Sheet::Model);
    assert_eq!(names(&app), ["Birleşik"]);
    history_key(&mut app, true);
    app.follow_sheet_book();
    assert_eq!(names(&app), ["Büro paftası"]);
    assert_eq!(app.document.as_ref().unwrap().model.len(), count);
    assert_eq!(
        app.document.as_ref().unwrap().model.sheets(),
        Some(&original)
    );
    history_key(&mut app, false);
    app.follow_sheet_book();
    assert_eq!(names(&app), ["Birleşik"]);
    assert_eq!(app.document.as_ref().unwrap().model.len(), count + 1);
    assert_eq!(
        app.document
            .as_ref()
            .unwrap()
            .model
            .sheets()
            .unwrap()
            .pictures,
        original.pictures
    );
}

#[test]
fn the_book_opens_from_the_file_changes_in_the_drawing_and_is_saved() {
    let (mut app, _store) = app_with_store("pafta-dosyadan");
    let dir = scratch("pafta-dosyadan-cizim");
    let path = dir.join("pafta.kcad");
    std::fs::copy(SAMPLE, &path).expect("copied");
    open(&mut app, &path);
    assert_eq!(names(&app), ["Büro paftası"]);
    let logo = app.sheets.book().assets[0].sha256.clone();
    let doc = app.document.as_ref().expect("a drawing");
    assert!(!doc.model.is_dirty(), "opening is no edit");
    // A change in the sheet mode is in the drawing at once: an edit, not an undo step of the drawing.
    let id = app.sheets.book().sheets[0].id.clone();
    let undo_before = app.document.as_ref().expect("a drawing").model.can_undo();
    sheet(
        &mut app,
        Sheet::RenameSheet(id.clone(), "Ada 101 paftası".into()),
    );
    let doc = app.document.as_ref().expect("a drawing");
    assert!(doc.model.is_dirty());
    assert_eq!(doc.model.can_undo(), undo_before);
    assert_eq!(
        kept(&app).expect("a book").sheets[0].name,
        "Ada 101 paftası"
    );
    // Its picture's bytes go with it.
    let pictures = &app
        .document
        .as_ref()
        .and_then(|d| d.model.sheets())
        .expect("a book")
        .pictures;
    assert_eq!(
        pictures
            .iter()
            .map(|p| p.sha256.as_str())
            .collect::<Vec<_>>(),
        [logo.as_str()]
    );
    // The sheet mode's own undo takes it back, in the drawing too.
    sheet(&mut app, Sheet::Undo);
    assert_eq!(kept(&app).expect("a book").sheets[0].name, "Büro paftası");
    sheet(&mut app, Sheet::Redo);
    // Saved, and opened again in another window: the book is the file's.
    let snapshot = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .to_snapshot_v2();
    crate::document::write(&snapshot, &path).expect("saved");
    let (mut again, _) = app_with_store("pafta-dosyadan-2");
    open(&mut again, &path);
    assert_eq!(names(&again), ["Ada 101 paftası"]);
    // A drawing without a book: an empty one; a new sheet makes the drawing's book.
    let empty = dir.join("bos.kcad");
    let mut bare = snapshot.clone();
    bare.sheets = None;
    crate::document::write(&bare, &empty).expect("saved");
    open(&mut again, &empty);
    assert!(again.sheets.book().sheets.is_empty());
    assert!(kept(&again).is_none());
    let _ = again.update(Message::Sheet(Sheet::NewSheet));
    if again.sheets.questions().is_some() {
        let _ = again.update(Message::Sheet(Sheet::Questions(
            kentos_sheet_ui::questions::QuestionsMessage::Make,
        )));
    }
    assert_eq!(kept(&again).expect("a book").sheets.len(), 1);
}

#[test]
fn a_book_this_device_kept_moves_into_its_project_and_is_set_aside_once_saved() {
    let (mut app, store) = app_with_store("pafta-goc");
    let dir = scratch("pafta-goc-cizim");
    let path = dir.join("ada.kcad");
    // The sample's drawing without its book, and its book where an older build kept it.
    let mut doc = kentos_kcad::decode(&std::fs::read(SAMPLE).expect("sample")).expect("reads");
    let sheets = doc.sheets.take().expect("a book");
    crate::document::write(&doc, &path).expect("saved");
    let (book, pictures) = kentos_sheet::project::read_project_sheets(&sheets).expect("reads");
    for (sha, bytes) in &pictures {
        store.put_asset(sha, bytes);
    }
    let key = key_of(&doc, "ada.kcad");
    store.save_book(&key, &book).expect("kept");
    open(&mut app, &path);
    assert_eq!(names(&app), ["Büro paftası"]);
    assert_eq!(kept(&app), Some(book.clone()));
    let model = &app.document.as_ref().expect("a drawing").model;
    assert!(
        model.is_dirty(),
        "the drawing changed: it is saved with its book"
    );
    assert_eq!(model.sheets().expect("a book").pictures.len(), 1);
    assert!(app.log.said(Level::Info, "bu bilgisayardan çizime alındı"));
    // Not saved yet: the device's copy stays where it was.
    assert_eq!(store.load_book(&key), Some(book.clone()));
    // Saved: set aside, never deleted.
    let revision = app.document.as_ref().expect("a drawing").model.revision();
    let snapshot = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .to_snapshot_v2();
    crate::document::write(&snapshot, &path).expect("saved");
    app.document
        .as_mut()
        .expect("a drawing")
        .model
        .mark_saved(revision);
    let _ = app.update(Message::Swallowed);
    assert_eq!(store.load_book(&key), None);
    let aside = std::fs::read_dir(store.dir().join("kitaplar").join("tasindi"))
        .expect("set aside")
        .count();
    assert_eq!(aside, 1);
    // Opened again: the file's book, nothing moved, nothing said of it.
    let moved = |app: &App| {
        app.log
            .lines()
            .filter(|l| l.text.contains("çizime alındı") || l.text.contains("silinmedi"))
            .count()
    };
    open(&mut app, &path);
    assert_eq!(names(&app), ["Büro paftası"]);
    assert!(!app.document.as_ref().expect("a drawing").model.is_dirty());
    assert_eq!(moved(&app), 1);
}

#[test]
fn a_project_with_a_book_sets_the_devices_other_copy_aside_and_says_so() {
    let (mut app, store) = app_with_store("pafta-iki-kitap");
    let doc = kentos_kcad::decode(&std::fs::read(SAMPLE).expect("sample")).expect("reads");
    let key = key_of(&doc, "sheets.kcad");
    let mut other = kentos_sheet::model::SheetBook::default();
    other.variables.push(kentos_sheet::model::Variable {
        name: "eski".into(),
        label: "Eski".into(),
        kind: kentos_sheet::model::VarKind::Text,
        value: kentos_sheet::model::VarValue::Text("1".into()),
    });
    store.save_book(&key, &other).expect("kept");
    open(&mut app, std::path::Path::new(SAMPLE));
    assert_eq!(names(&app), ["Büro paftası"], "the file's book");
    assert!(!app.document.as_ref().expect("a drawing").model.is_dirty());
    assert_eq!(store.load_book(&key), None);
    assert!(app.log.said(Level::Info, "cihazdaki silinmedi"));
}

#[test]
fn a_book_the_core_does_not_read_is_kept_as_it_is() {
    let (mut app, _store) = app_with_store("pafta-okunamayan");
    let dir = scratch("pafta-okunamayan-cizim");
    let path = dir.join("yeni.kcad");
    let mut doc = kentos_kcad::decode(&std::fs::read(SAMPLE).expect("sample")).expect("reads");
    let mut sheets = doc.sheets.take().expect("a book");
    sheets.book["schema"] = "kentos.sheet/9".into();
    doc.sheets = Some(sheets.clone());
    crate::document::write(&doc, &path).expect("saved");
    open(&mut app, &path);
    assert!(app.log.said(Level::Warn, "pafta kitabı okunamadı"));
    assert!(app.sheets.book().sheets.is_empty());
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    if app.sheets.questions().is_some() {
        let _ = app.update(Message::Sheet(Sheet::Questions(
            kentos_sheet_ui::questions::QuestionsMessage::Make,
        )));
    }
    let model = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(model.sheets(), Some(&sheets), "kept as it is");
    assert!(!model.is_dirty());
}

/// Pictures of the book in the project's file for the owner (docs/adr/0251):
/// a sheet saved in the drawing and opened in another window, the book this
/// device kept moved into its project (the line in the history and the
/// drawing's unsaved mark), and a book the core does not read (the warning).
/// `cargo test -p kentos-desktop sheet_book::tests::screens -- --ignored --nocapture`
/// writes `.run/shots/pafta-kitabi-*.png`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_render_wgpu::Vec2;

    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let shoot = |app: &mut App, name: &str| {
        // The GPU's renderer: the drawing area too (run by hand, alone).
        let mut snapshot =
            kentos_ui::snapshot::Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(format!("pafta-kitabi-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    };
    let look_at_drawing = |app: &mut App| {
        if let Some(b) =
            kentos_render_wgpu::scene::extents(app.document.as_ref().expect("a drawing"))
        {
            app.viewport.camera.center =
                Vec2::new((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
        }
    };

    // A sheet made over the demo drawing and saved in its file.
    let dir = scratch("pafta-kitabi-resim");
    let path = dir.join("ada-101.kcad");
    let mut app = crate::files_testing::app_with_drawing();
    app.sheet_store =
        Some(kentos_sheet_ui::Store::open(&scratch("pafta-kitabi-depo")).expect("a store"));
    look_at_drawing(&mut app);
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    // The template's questions answered as an office would answer them.
    let answers: Vec<(usize, &str)> = app
        .sheets
        .questions()
        .map(|q| {
            q.template_variables()
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    let answer = match v.name.as_str() {
                        "harita_basligi" => "Ada 101 Parsel 7 aplikasyon krokisi",
                        "alt_baslik" => "Örnek Mahallesi, Merkez / Sivas",
                        "kaynak" => "Sivas Belediyesi Harita Müdürlüğü",
                        _ => return None,
                    };
                    Some((i, answer))
                })
                .collect()
        })
        .unwrap_or_default();
    for (i, answer) in answers {
        let _ = app.update(Message::Sheet(Sheet::Questions(
            kentos_sheet_ui::questions::QuestionsMessage::Text(i, answer.into()),
        )));
    }
    if app.sheets.questions().is_some() {
        let _ = app.update(Message::Sheet(Sheet::Questions(
            kentos_sheet_ui::questions::QuestionsMessage::Make,
        )));
    }
    let id = app.sheets.book().sheets[0].id.clone();
    for v in app
        .sheets
        .questions()
        .into_iter()
        .flat_map(|q| q.template_variables())
    {
        println!("soru: {}", v.name);
    }
    sheet(
        &mut app,
        Sheet::RenameSheet(id, "Ada 101 aplikasyon".into()),
    );
    let snapshot = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .to_snapshot_v2();
    crate::document::write(&snapshot, &path).expect("saved");
    let book = kept(&app).expect("a book");

    // Opened in another window: the file's sheet.
    let (mut again, _) = app_with_store("pafta-kitabi-resim-2");
    open(&mut again, &path);
    look_at_drawing(&mut again);
    sheet(&mut again, Sheet::Tab(1));
    shoot(&mut again, "dosyadan");

    // The same drawing without its book, the book where an older build kept it: moved in.
    let (mut moved, store) = app_with_store("pafta-kitabi-resim-goc");
    let bare_path = dir.join("ada-102.kcad");
    let mut bare = snapshot.clone();
    bare.sheets = None;
    bare.project_id = None;
    crate::document::write(&bare, &bare_path).expect("saved");
    store
        .save_book(&key_of(&bare, "ada-102.kcad"), &book)
        .expect("kept");
    open(&mut moved, &bare_path);
    look_at_drawing(&mut moved);
    let _ = moved.update(Message::BottomTab(crate::bottom::BottomTab::History));
    shoot(&mut moved, "goc");

    // A book of a newer build: kept as it is, said once.
    let (mut newer, _) = app_with_store("pafta-kitabi-resim-yeni");
    let newer_path = dir.join("ada-103.kcad");
    let mut later = snapshot;
    if let Some(s) = later.sheets.as_mut() {
        s.book["schema"] = "kentos.sheet/9".into();
    }
    crate::document::write(&later, &newer_path).expect("saved");
    open(&mut newer, &newer_path);
    look_at_drawing(&mut newer);
    let _ = newer.update(Message::BottomTab(crate::bottom::BottomTab::Messages));
    shoot(&mut newer, "okunamayan");
}

/// GPU evidence for the ownership warning and both undo histories in light/dark and large text.
#[test]
#[ignore = "GPU pictures, run alone"]
fn transaction_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    for (theme, text, width, height) in [
        ("dark", 13, 1440., 900.),
        ("light", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store(&format!("pafta-islem-resim-{theme}-{text}"));
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        open(&mut app, std::path::Path::new(SAMPLE));
        combine(&mut app);
        app.follow_sheet_book();
        sheet(
            &mut app,
            Sheet::RenameSheet("pafta-1".into(), "Sonraki".into()),
        );
        history_key(&mut app, true);
        app.follow_sheet_book();
        let _ = app.update(Message::BottomTab(crate::bottom::BottomTab::Messages));
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(width, height)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("pafta-islem-koruma-{theme}-{text}.png"));
        snap.render(app.view(), &app.theme()).save(&path).unwrap();
        println!("{}", path.display());
        let _ = app.update(Message::CommandHistoryToggled);
        sheet(&mut app, Sheet::Undo);
        history_key(&mut app, true);
        app.follow_sheet_book();
        sheet(&mut app, Sheet::Tab(1));
        snap.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("pafta-islem-geri-{theme}-{text}.png"));
        let image = snap.render(app.view(), &app.theme());
        image.save(&path).unwrap();
        // The fixture has a red embedded logo. A blank paper must fail even
        // when the book's three items and the UI tree are still present.
        let red = image
            .rgba
            .chunks_exact(4)
            .filter(|p| p[0] > 150 && p[1] < 70 && p[2] < 70 && p[3] > 200)
            .count();
        assert!(
            red > 500,
            "{theme}-{text}: the restored embedded logo is not painted ({red} red pixels)"
        );
        println!("{}", path.display());
    }
}

/// Real model, template gallery creation and the common fit buttons, including large type on a narrow window.
#[test]
#[ignore = "GPU pictures, run alone"]
fn sheet07_tools_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(fixture["setup"].clone()).unwrap();
    for value in [
        serde_json::json!({"kind":"polygon","id":2,"layerId":"cizim","attrs":{},"pts":[{"x":0,"y":0},{"x":5,"y":0},{"x":5,"y":3},{"x":0,"y":3}]}),
        serde_json::json!({"kind":"text","id":3,"layerId":"cizim","attrs":{},"p":{"x":1.4,"y":1.5},"text":"ODA 15 m²","height":0.15,"rotation":0}),
    ] {
        snapshot
            .entities
            .push(serde_json::from_value(value).unwrap());
    }
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    for (theme, text, width, height) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet07-tools");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::new(snapshot.clone(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        app.selection.set([kentos_domain::Slot(2)]);
        sheet(&mut app, Sheet::NewSheet);
        if app.sheets.questions().is_some() {
            sheet(
                &mut app,
                Sheet::Questions(kentos_sheet_ui::questions::QuestionsMessage::Make),
            );
        }
        app.follow_sheets();
        assert_eq!(
            app.sheets.context().selection_bounds,
            Some([0., 0., 5., 3.])
        );
        let map = app
            .sheets
            .open_sheet()
            .unwrap()
            .items
            .iter()
            .find(|i| matches!(i.kind, kentos_sheet::ItemKind::Map(_)))
            .unwrap()
            .id
            .clone();
        sheet(&mut app, Sheet::Select(vec![map.clone()]));
        sheet(&mut app, Sheet::MapFitDrawing);
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        sheet(&mut app, Sheet::MapFitSelection);
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        let item = app
            .sheets
            .open_sheet()
            .unwrap()
            .items
            .iter()
            .find(|i| i.id == map)
            .unwrap();
        let kentos_sheet::ItemKind::Map(m) = &item.kind else {
            panic!()
        };
        let kentos_sheet::MapView::Fixed(v) = &m.view else {
            panic!()
        };
        assert_eq!(v.center, Some(kentos_sheet::GroundPoint { x: 2.5, y: 1.5 }));
        for section in ["frame", "constraints"] {
            sheet(&mut app, Sheet::Section(section));
        }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(width, height)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::ZoomPage);
        snap.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("sheet07-tools-{theme}-{text}.png"));
        let image = snap.render(app.view(), &app.theme());
        let (x0, y0, x1, y1, least) = if text == 13 {
            (550, 425, 700, 475, 100)
        } else {
            (440, 350, 550, 390, 20)
        };
        let mut ink = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                let at = (y * width as usize + x) * 4;
                if image.rgba[at..at + 3].iter().all(|c| *c < 150) {
                    ink += 1;
                }
            }
        }
        assert!(
            ink > least,
            "{theme}-{text}: room text missing ({ink} pixels)"
        );
        image.save(&path).unwrap();
        println!("{}", path.display());
    }
}

#[test]
#[ignore = "GPU pictures, run alone"]
fn sheet08_analytic_paper_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut model = kentos_domain::Document::from_snapshot(
        serde_json::from_value(fixture["setup"].clone()).unwrap(),
    )
    .unwrap();
    model.set_sheets(None).unwrap();
    let mut layout = kentos_commands::CadLayout {
        name: "Analitik pafta".into(),
        width_mm: 297.,
        height_mm: 210.,
        ..Default::default()
    };
    let curves = [
        serde_json::json!({"kind":"circle","c":{"x":0.06,"y":0.145},"r":0.025}),
        serde_json::json!({"kind":"arc","c":{"x":0.15,"y":0.145},"r":0.025,"a0":0,"a1":4.7}),
        serde_json::json!({"kind":"ellipse","c":{"x":0.24,"y":0.145},"major":{"x":0.025,"y":0.01},"ratio":0.4,"t0":0.3,"t1":5.2}),
        serde_json::json!({"kind":"nurbs","degree":3,"ctrl":[{"x":0.035,"y":0.055},{"x":0.04,"y":0.12},{"x":0.09,"y":0.0},{"x":0.105,"y":0.06}],"knots":[0,0,0,0,1,1,1,1]}),
        serde_json::json!({"kind":"polyline","pts":[{"x":0.14,"y":0.06},{"x":0.2,"y":0.06}],"bulges":[1,0],"widths":[[0.002,0.005],[0,0]],"closed":false}),
        serde_json::json!({"kind":"text","p":{"x":0.035,"y":0.19},"text":"ANALİTİK PAFTA · Yay / Elips / NURBS","height":0.005,"rotation":0}),
        serde_json::json!({"kind":"circle","c":{"x":0.24,"y":0.06},"r":0.0001}),
    ];
    for (mut raw, i) in curves.into_iter().zip(1..) {
        raw["id"] = serde_json::json!(i);
        raw["layerId"] = serde_json::json!("0");
        raw["attrs"] = serde_json::json!({});
        raw["color"] = serde_json::json!("#1747aa");
        layout.entities.push(serde_json::from_value(raw).unwrap());
    }
    let sheets = kentos_native_application::cad_layouts::merge_with_source(
        &model,
        &[layout],
        Some(&kentos_native_application::cad_layouts::Source {
            file: "analytic.dxf".into(),
            sha256: Some("a".repeat(64)),
        }),
    )
    .unwrap();
    model.set_sheets(sheets).unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    let pdf=kentos_plot::book::book_pdf(&model,&[],"Test","2026-10-22").unwrap();
    std::fs::write(out.join("sheet08-curves.pdf"),pdf.bytes).unwrap();
    for (theme, text, width, height) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet08-curves");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        sheet(&mut app, Sheet::Tab(1));
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(width, height)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::ZoomPage);
        snap.settle(&mut app, App::view, &mut update);
        let image = snap.render(app.view(), &app.theme());
        let(x0,y0,x1,y1,least)=if text==13{(301,222,1094,783,2000)}else{(328,250,734,537,400)};
        let mut blue=0;
        for y in y0..y1{for x in x0..x1{let at=(y*width as usize+x)*4;let p=&image.rgba[at..at+4];if p[2]>90&&p[0]<70&&p[1]<120{blue+=1;}}}
        let path=out.join(format!("sheet08-curves-{theme}-{text}.png"));image.save(&path).unwrap();
        assert!(blue>least,"{theme}-{text}: missing curves {blue}");
        println!("{}", path.display());
    }
}

#[test]
#[ignore = "GPU pictures, run alone"]
fn sheet09_viewport_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(fixture["setup"].clone()).unwrap();
    for value in [
        serde_json::json!({"kind":"polygon","id":2,"layerId":"cizim","attrs":{},"pts":[{"x":0,"y":0},{"x":5,"y":0},{"x":5,"y":3},{"x":0,"y":3}]}),
        serde_json::json!({"kind":"text","id":3,"layerId":"cizim","attrs":{},"p":{"x":1.4,"y":1.5},"text":"ODA 15 m²","height":0.15,"rotation":0}),
    ] {
        snapshot
            .entities
            .push(serde_json::from_value(value).unwrap());
    }
    let named = kentos_commands::NamedView {
        id: "view-room".into(),
        name: "Zemin kat".into(),
        center: kentos_commands::Vec2 { x: 2.5, y: 1.5 },
        width: 5.5,
        height: 3.3,
        twist: std::f64::consts::PI / 12.,
    };
    snapshot.settings.named_views = vec![named.clone()];
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    for (theme, text, width, height) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet09-viewport");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::new(snapshot.clone(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        app.selection.set([kentos_domain::Slot(2)]);
        sheet(&mut app, Sheet::NewSheet);
        if app.sheets.questions().is_some() {
            sheet(
                &mut app,
                Sheet::Questions(kentos_sheet_ui::questions::QuestionsMessage::Make),
            );
        }
        app.follow_sheets();
        assert_eq!(
            app.sheets.context().selection_bounds,
            Some([0., 0., 5., 3.])
        );
        let map = app
            .sheets
            .open_sheet()
            .unwrap()
            .items
            .iter()
            .find(|i| matches!(i.kind, kentos_sheet::ItemKind::Map(_)))
            .unwrap()
            .id
            .clone();
        sheet(&mut app, Sheet::Select(vec![map.clone()]));
        sheet(&mut app, Sheet::MapFitDrawing);
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        sheet(&mut app, Sheet::MapFitSelection);
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        sheet(
            &mut app,
            Sheet::MapConfigure(Box::new(kentos_sheet::viewport::SetViewport {
                id: map.clone(),
                source: Some(kentos_sheet::viewport::Source::ViewSnapshot {
                    view: named.clone(),
                }),
                ratio: Some(kentos_sheet::viewport::ScaleRatio {
                    paper: 1.,
                    model: 37.5,
                }),
                locked: Some(true),
                ..Default::default()
            })),
        );
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        let item = app
            .sheets
            .open_sheet()
            .unwrap()
            .items
            .iter()
            .find(|i| i.id == map)
            .unwrap();
        let kentos_sheet::ItemKind::Map(m) = &item.kind else {
            panic!()
        };
        let kentos_sheet::MapView::Fixed(v) = &m.view else {
            panic!()
        };
        assert_eq!(v.center, Some(kentos_sheet::GroundPoint { x: 2.5, y: 1.5 }));
        for section in ["frame", "constraints"] {
            sheet(&mut app, Sheet::Section(section));
        }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(width, height)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::ZoomPage);
        snap.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("sheet09-viewport-{theme}-{text}.png"));
        let image = snap.render(app.view(), &app.theme());
        let (x0,y0,x1,y1,least)=if text==13{(560,340,855,570,900)}else{(458,310,615,430,400)};
        let mut ink=0;
        for y in y0..y1{for x in x0..x1{let at=(y*width as usize+x)*4;if image.rgba[at..at+3].iter().all(|c|*c<220){ink+=1;}}}
        assert!(ink>least,"{theme}-{text}: viewport geometry missing ({ink})");
        image.save(&path).unwrap();
        println!("{}", path.display());
        if theme == "light" {
            let pdf = kentos_plot::book::book_pdf(
                &app.document.as_ref().unwrap().model,
                &[],
                "Test",
                "2026-10-22",
            )
            .unwrap();
            std::fs::write(out.join("sheet09-viewport.pdf"), pdf.bytes).unwrap();
        }
    }
}

#[test]
#[ignore = "GPU pictures, run alone"]
fn sheet10_precision_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(fixture["setup"].clone()).unwrap();
    snapshot.entities.clear();
    for (id, x, y, color) in [(1, 0., 0., "#e5484d"), (2, 487150., 4417820., "#1747aa")] {
        snapshot.entities.push(serde_json::from_value(serde_json::json!({"kind":"polyline","id":id,"layerId":"cizim","attrs":{},"color":color,"lineWeight":0.25,"closed":true,"pts":[{"x":x-0.025,"y":y-0.015},{"x":x+0.025,"y":y-0.015},{"x":x+0.025,"y":y+0.015},{"x":x-0.025,"y":y+0.015}]})).unwrap());
    }
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/sheet/v1/viewports.json")).unwrap();
    let mut page = fixtures["cases"][0]["sheet"].clone();
    page["name"] = serde_json::json!("2:1 · 5 cm model = 100 mm kâğıt");
    page["page"]["paper"] = serde_json::json!("a4");
    page["page"]["size"] = serde_json::json!({"width":297000,"height":210000});
    let mut items = Vec::new();
    for (id, left, x, y) in [("near", 20000, 0., 0.), ("far", 160000, 487150., 4417820.)] {
        let mut item = page["items"][0].clone();
        item["id"] = serde_json::json!(id);
        item["name"] = serde_json::json!(id);
        item["padding"] = serde_json::json!(0);
        item["kind"]["labelBand"] = serde_json::json!(0);
        item["frame"] = serde_json::json!({"left":left,"top":50000,"width":110000,"height":100000});
        item["kind"]["view"] = serde_json::json!({"type":"fixed","center":{"x":x,"y":y},"scale":1,"ratio":{"paper":2,"model":1},"rotation":0});
        items.push(item);
    }
    page["items"] = serde_json::json!(items);
    let mut book = kentos_sheet::model::SheetBook::default();
    book.sheets.push(serde_json::from_value(page).unwrap());
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let before = model.to_snapshot_v2();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    let pdf = kentos_plot::book::book_pdf(&model, &[], "Test", "2026-10-22").unwrap();
    std::fs::write(out.join("sheet10-precision.pdf"), pdf.bytes).unwrap();
    assert_eq!(
        model.to_snapshot_v2(),
        before,
        "paper export must not mutate the model or its origin"
    );
    for (theme, text, width, height) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet10-precision");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        sheet(&mut app, Sheet::Tab(1));
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(width, height)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::ZoomPage);
        snap.settle(&mut app, App::view, &mut update);
        let image = snap.render(app.view(), &app.theme());
        let path = out.join(format!("sheet10-precision-{theme}-{text}.png"));
        image.save(&path).unwrap();
        println!("{}", path.display());
        // Only each viewport's content: the page-margin guides are blue too.
        let regions = if text == 13 {
            [(360, 395, 645, 585), (735, 395, 1020, 585)]
        } else {
            [(350, 330, 510, 445), (545, 330, 705, 445)]
        };
        let mut boxes = [[usize::MAX, usize::MAX, 0, 0]; 2];
        let mut counts = [0; 2];
        for (i, (x0, y0, x1, y1)) in regions.into_iter().enumerate() {
            for y in y0..y1 {
                for x in x0..x1 {
                    let at = (y * width as usize + x) * 4;
                    let p = &image.rgba[at..at + 4];
                    // Anti-aliased thin strokes mix with white; compare colour dominance.
                    let colored = if i == 0 {
                        i16::from(p[0]) > i16::from(p[1]) + 25
                            && i16::from(p[0]) > i16::from(p[2]) + 25
                    } else {
                        i16::from(p[2]) > i16::from(p[0]) + 25
                            && i16::from(p[2]) > i16::from(p[1]) + 15
                    };
                    if colored {
                        counts[i] += 1;
                        boxes[i] = [
                            boxes[i][0].min(x),
                            boxes[i][1].min(y),
                            boxes[i][2].max(x),
                            boxes[i][3].max(y),
                        ];
                    }
                }
            }
        }
        println!("{theme}-{text}: {boxes:?} {counts:?}");
        for (b, n) in boxes.iter().zip(counts) {
            assert!(n > 100, "missing model detail");
            assert!(
                b[2] - b[0] > 100 && b[3] - b[1] > 60,
                "detail must be visible at the requested physical scale"
            );
        }
        for j in [0, 1] {
            assert!(
                (boxes[0][j + 2] - boxes[0][j]).abs_diff(boxes[1][j + 2] - boxes[1][j]) <= 1,
                "near and far details must have equal dimensions"
            );
        }
    }
}

#[test]
#[ignore = "GPU pictures, run alone"]
fn sheet11_exact_scale_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(fixture["setup"].clone()).unwrap();
    for value in [
        serde_json::json!({"kind":"polygon","id":2,"layerId":"cizim","attrs":{},"pts":[{"x":0,"y":0},{"x":5,"y":0},{"x":5,"y":3},{"x":0,"y":3}]}),
        serde_json::json!({"kind":"text","id":3,"layerId":"cizim","attrs":{},"p":{"x":1.4,"y":1.5},"text":"ODA 15 m²","height":0.15,"rotation":0}),
    ] {
        snapshot
            .entities
            .push(serde_json::from_value(value).unwrap());
    }
    let named = kentos_commands::NamedView {
        id: "view-room".into(),
        name: "Zemin kat".into(),
        center: kentos_commands::Vec2 { x: 2.5, y: 1.5 },
        width: 5.5,
        height: 3.3,
        twist: std::f64::consts::PI / 12.,
    };
    snapshot.settings.named_views = vec![named.clone()];
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    for (theme, text, width, height) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet11-exact-scale");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::new(snapshot.clone(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        app.selection.set([kentos_domain::Slot(2)]);
        sheet(&mut app, Sheet::NewSheet);
        if app.sheets.questions().is_some() {
            sheet(
                &mut app,
                Sheet::Questions(kentos_sheet_ui::questions::QuestionsMessage::Make),
            );
        }
        app.follow_sheets();
        assert_eq!(
            app.sheets.context().selection_bounds,
            Some([0., 0., 5., 3.])
        );
        let map = app
            .sheets
            .open_sheet()
            .unwrap()
            .items
            .iter()
            .find(|i| matches!(i.kind, kentos_sheet::ItemKind::Map(_)))
            .unwrap()
            .id
            .clone();
        sheet(&mut app, Sheet::Select(vec![map.clone()]));
        sheet(&mut app, Sheet::MapFitDrawing);
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        sheet(&mut app, Sheet::MapFitSelection);
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        sheet(
            &mut app,
            Sheet::MapConfigure(Box::new(kentos_sheet::viewport::SetViewport {
                id: map.clone(),
                source: Some(kentos_sheet::viewport::Source::ViewSnapshot {
                    view: named.clone(),
                }),
                ratio: Some(kentos_sheet::viewport::ScaleRatio {
                    paper: 1.,
                    model: 37.123456789123,
                }),
                locked: Some(false),
                ..Default::default()
            })),
        );
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        let item = app
            .sheets
            .open_sheet()
            .unwrap()
            .items
            .iter()
            .find(|i| i.id == map)
            .unwrap();
        let kentos_sheet::ItemKind::Map(m) = &item.kind else {
            panic!()
        };
        let kentos_sheet::MapView::Fixed(v) = &m.view else {
            panic!()
        };
        assert_eq!(v.center, Some(kentos_sheet::GroundPoint { x: 2.5, y: 1.5 }));
        for section in ["frame", "constraints"] {
            sheet(&mut app, Sheet::Section(section));
        }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(width, height)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::ZoomPage);
        snap.settle(&mut app, App::view, &mut update);
        let path = out.join(format!("sheet11-exact-scale-{theme}-{text}.png"));
        let image = snap.render(app.view(), &app.theme());
        let (x0, y0, x1, y1, least) = if text == 13 {
            (560, 340, 855, 570, 900)
        } else {
            (458, 310, 615, 430, 400)
        };
        let mut ink = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                let at = (y * width as usize + x) * 4;
                if image.rgba[at..at + 3].iter().all(|c| *c < 220) {
                    ink += 1;
                }
            }
        }
        assert!(
            ink > least,
            "{theme}-{text}: viewport geometry missing ({ink})"
        );
        image.save(&path).unwrap();
        println!("{}", path.display());
        if text==18 {
            snap.input(&mut app,App::view,&mut update,kentos_ui::snapshot::Input::Scroll(iced::Point::new(width-100.,height-140.),-9.));
            snap.render(app.view(),&app.theme()).save(out.join("sheet11-exact-scale-dark-18-scrolled.png")).unwrap();
        }
        if theme=="light" {
            let before=app.sheets.book().clone();
            snap.input(&mut app,App::view,&mut update,kentos_ui::snapshot::Input::Click(iced::Point::new(1150.,357.)));
            let item=app.sheets.book().item(&map).unwrap();
            let kentos_sheet::ItemKind::Map(m)=&item.kind else{panic!()};
            assert!(m.view_locked,"a real mouse press locks the viewport");
            let kentos_sheet::MapView::Fixed(v)=&m.view else{panic!()};assert_eq!(v.denominator(),37.123456789123);
            sheet(&mut app,Sheet::Undo);assert_eq!(app.sheets.book(),&before);
        }
        if theme == "light" {
            let pdf = kentos_plot::book::book_pdf(
                &app.document.as_ref().unwrap().model,
                &[],
                "Test",
                "2026-10-22",
            )
            .unwrap();
            std::fs::write(out.join("sheet11-exact-scale.pdf"), pdf.bytes).unwrap();
        }
    }
}

#[test]
#[ignore = "GPU pictures, run alone"]
fn sheet12_clipping_screens() {
    use kentos_sheet::{ItemKind, MapView};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(fixture["setup"].clone()).unwrap();
    snapshot.entities.clear();
    snapshot.layers[0].style.renderer = Some(
        serde_json::json!({"type":"single","symbols":{"fill":{"type":"fill","layers":[{"id":"blue","type":"simpleFill","color":"#1747aa"}]}}}),
    );
    for raw in [
        serde_json::json!({"kind":"polygon","id":1,"layerId":"cizim","attrs":{},"pts":[{"x":0,"y":0},{"x":5,"y":0},{"x":5,"y":3},{"x":0,"y":3}]}),
        serde_json::json!({"kind":"text","id":2,"layerId":"cizim","attrs":{},"p":{"x":3.1,"y":2.35},"text":"ETİKET","height":0.18,"rotation":0}),
    ] {
        snapshot.entities.push(serde_json::from_value(raw).unwrap());
    }
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sheet/v1/viewport-clips.json"
    ))
    .unwrap();
    let mut page: kentos_sheet::Sheet =
        serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap();
    let prototype = page.items[0].clone();
    page.items.clear();
    page.name = "Kırpma sınırları · 1:50".into();
    for (id, name, left, source) in [
        ("full", "Tam çerçeve", 30000, "Explicit frame"),
        ("tri", "Üçgen · çerçeve baskısız", 155000, "Triangle"),
        ("L", "İçbükey çokgen", 280000, "Concave L"),
    ] {
        let case = data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == source)
            .unwrap();
        let mut item = prototype.clone();
        item.id = id.into();
        item.name = name.into();
        item.frame = kentos_sheet::units::RectUm::new(left, 80000, 110000, 80000);
        let ItemKind::Map(map) = &mut item.kind else {
            panic!()
        };
        map.clip = serde_json::from_value(case["expected"]["clip"].clone()).unwrap();
        map.frame_plot = id != "tri";
        let MapView::Fixed(v) = &mut map.view else {
            panic!()
        };
        v.scale = 50;
        v.ratio = None;
        v.center = Some(kentos_sheet::GroundPoint { x: 2.5, y: 1.5 });
        page.items.push(item);
        let mut kind = kentos_sheet::kinds::default_kind("text").unwrap();
        let ItemKind::Text(t) = &mut kind else {
            panic!()
        };
        t.content = name.into();
        t.style.size = 3000;
        page.items.push(kentos_sheet::Item::new(
            &format!("label-{id}"),
            &format!("{name} etiketi"),
            kentos_sheet::units::RectUm::new(left, 170000, 110000, 15000),
            kind,
        ));
    }
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets.push(page);
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    let pdf = kentos_plot::book::book_pdf(&model, &[], "Test", "2026-10-22").unwrap();
    std::fs::write(out.join("sheet12-clips.pdf"), pdf.bytes).unwrap();
    for (theme, text, width, height) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet12-clips");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        sheet(&mut app, Sheet::Tab(1));
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(width, height)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::ZoomPage);
        snap.settle(&mut app, App::view, &mut update);
        let image = snap.render(app.view(), &app.theme());
        let path = out.join(format!("sheet12-clips-{theme}-{text}.png"));
        image.save(&path).unwrap();
        println!("{}", path.display());
        let (px, py, k) = if text == 13 {
            (300., 222., 795. / 420.)
        } else {
            (328., 250., 407. / 420.)
        };
        for (left, x, y, blue) in [
            (30., 0.25, 0.25, true),
            (155., 0.25, 0.25, true),
            (155., 0.75, 0.75, false),
            (280., 0.25, 0.25, true),
            (280., 0.75, 0.25, false),
            (280., 0.75, 0.75, true),
        ] {
            let x = (px + (left + x * 110.) * k) as usize;
            let y = (py + (80. + y * 80.) * k) as usize;
            let at = (y * width as usize + x) * 4;
            let p = &image.rgba[at..at + 4];
            assert_eq!(
                i16::from(p[2]) > i16::from(p[0]) + 40 && i16::from(p[2]) > i16::from(p[1]) + 20,
                blue,
                "{theme}-{text}: probe {left}/{x}/{y}: {p:?}"
            );
        }
        // The concave void must contain neither source fill nor source text.
        for y in ((py + 96. * k) as usize)..((py + 116. * k) as usize) {
            for x in ((px + 346. * k) as usize)..((px + 379. * k) as usize) {
                let at = (y * width as usize + x) * 4;
                assert!(
                    image.rgba[at..at + 3].iter().all(|c| *c >= 245),
                    "{theme}-{text}: leaked model content in polygon void"
                );
            }
        }
        sheet(
            &mut app,
            Sheet::ViewportClip(kentos_sheet_ui::viewport_clip::ClipMessage::Open(
                "tri".into(),
            )),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet12-boundary-dialog-{theme}-{text}.png")))
            .unwrap();
        if text == 18 {
            snap.input(
                &mut app,
                App::view,
                &mut update,
                kentos_ui::snapshot::Input::Click(iced::Point::new(881., 568.)),
            );
            assert!(
                app.sheets
                    .window(kentos_sheet_ui::lend(&kentos_sheet_ui::NoMaps))
                    .is_none(),
                "real mouse click closes an unchanged valid boundary"
            );
        } else {
            sheet(
                &mut app,
                Sheet::ViewportClip(kentos_sheet_ui::viewport_clip::ClipMessage::Apply),
            );
        }
        assert!(app.sheets.error().is_none());
    }
}

fn model_scope_app(theme: &str, text: u32) -> App {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(fixture["setup"].clone()).unwrap();
    snapshot.entities.clear();
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sheet/v1/viewport-navigation.json"
    ))
    .unwrap();
    let mut page: kentos_sheet::Sheet =
        serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap();
    page.name = "A-101 · Görünümden model düzenleme".into();
    let kentos_sheet::ItemKind::Map(map) = &mut page.items[0].kind else {
        panic!()
    };
    let kentos_sheet::MapView::Fixed(f) = &mut map.view else {
        panic!()
    };
    f.center = Some(kentos_sheet::GroundPoint { x: 2.5, y: 1.5 });
    f.rotation = 0;
    page.items[0].frame = kentos_sheet::units::RectUm::new(40000, 40000, 310000, 190000);
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets.push(page);
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let (mut app, _store) = app_with_store("sheet13-model");
    app.settings.choose(&[
        ("appearance.theme", serde_json::json!(theme)),
        ("appearance.textSize", serde_json::json!(text)),
    ]);
    app.apply_settings();
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    sheet(&mut app, Sheet::Tab(1));
    app
}
fn model_click(app: &mut App, world: [f64; 2], release: bool) {
    let at = app.sheets.active_viewport().unwrap().paper_point(world);
    let stage = if release {
        kentos_sheet_ui::StageEvent::Release {
            at,
            shift: false,
            ctrl: false,
            alt: false,
        }
    } else {
        kentos_sheet_ui::StageEvent::Press {
            at,
            px: iced::Point::ORIGIN,
            size: iced::Size::new(1000., 700.),
            shift: false,
            ctrl: false,
            alt: false,
        }
    };
    sheet(app, Sheet::Stage(stage));
}
#[test]
fn model_scope_uses_real_drawing_tools_and_model_history_then_restores_camera() {
    let mut app = model_scope_app("light", 13);
    let camera = app.viewport.camera;
    let before = app.sheets.book().clone();
    sheet(&mut app, Sheet::EnterModel("viewport".into()));
    assert!(app.sheets.model_scope().is_some());
    let _ = app.run("tool.line");
    model_click(&mut app, [0., 0.], false);
    model_click(&mut app, [0., 0.], true);
    model_click(&mut app, [5., 0.], false);
    model_click(&mut app, [5., 0.], true);
    let doc = app.document.as_ref().unwrap();
    assert_eq!(doc.model.entities().count(), 1);
    let e = doc.model.entities().next().unwrap();
    let kentos_commands::Entity::Line(l) = e else {
        panic!("{e:?}")
    };
    assert!((l.a.x).abs() < 1e-6);
    assert!((l.b.x - 5.).abs() < 1e-6);
    assert_eq!(app.sheets.book(), &before);
    app.cancel();
    app.undo();
    assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 0);
    sheet(&mut app, Sheet::ReturnPaper);
    assert_eq!(app.viewport.camera, camera);
    assert!(app.sheets.is_active());
}
#[test]
#[ignore="GPU images, run alone"]
fn sheet13_model_scope_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = model_scope_app(theme, text);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::EnterModel("viewport".into()));
        let _ = app.run("tool.line");
        model_click(&mut app, [0., 0.], false);
        model_click(&mut app, [0., 0.], true);
        model_click(&mut app, [5., 0.], false);
        model_click(&mut app, [5., 0.], true);
        let at = app.sheets.active_viewport().unwrap().paper_point([5., 3.]);
        sheet(
            &mut app,
            Sheet::Stage(kentos_sheet_ui::StageEvent::Move {
                at,
                size: iced::Size::new(1000., 700.),
                shift: false,
                ctrl: false,
                alt: false,
            }),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet13-model-scope-{theme}-{text}.png")))
            .unwrap();
        assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 1);
        assert!(app.session.preview(&app.format()).is_some());
        app.cancel();
        app.session.exit();
        model_click(&mut app, [2.5, 0.], false);
        model_click(&mut app, [2.5, 0.], true);
        assert_eq!(
            app.selection.len(),
            1,
            "a press through the viewport chooses the original model object"
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet13-model-properties-{theme}-{text}.png")))
            .unwrap();
        if text == 18 {
            snap.input(
                &mut app,
                App::view,
                &mut update,
                kentos_ui::snapshot::Input::Scroll(iced::Point::new(w - 100., h - 170.), -7.),
            );
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join("sheet13-model-properties-scrolled.png"))
                .unwrap();
            let task = app.update(Message::Run("tool.text"));
            drive(&mut app, task);
            model_click(&mut app, [1., 1.5], false);
            assert!(app.text_field.is_some());
            let task = app.update(Message::TextField(crate::text_field::Event::Input(
                "Model yazısı".into(),
            )));
            drive(&mut app, task);
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join("sheet13-model-text-field.png"))
                .unwrap();
            let task = app.update(Message::TextField(crate::text_field::Event::Keep));
            drive(&mut app, task);
            assert!(app.document.as_ref().unwrap().model.entities().any(
                |e| matches!(e,kentos_commands::Entity::Text(t) if t.text=="Model yazısı")
            ));
        }
    }
}

#[test]
#[ignore="GPU images and independent PDF, run alone"]
fn sheet14_named_view_screens() {
    use kentos_sheet::{ItemKind, MapView, viewport::Source};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sheet/v1/viewport-links.json"
    ))
    .unwrap();
    let mut page: kentos_sheet::Sheet =
        serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap();
    page.name = "A-101 · Canlı görünüm ve kopya".into();
    let mut items = Vec::new();
    let captured = kentos_commands::NamedView {
        id: "model-plan".into(),
        name: "Plan ilk kayıt".into(),
        center: kentos_commands::Vec2 { x: 2.5, y: 1.5 },
        width: 14.,
        height: 6.,
        twist: 0.,
    };
    let current = kentos_commands::NamedView {
        name: "Plan güncel".into(),
        width: 7.,
        height: 3.,
        ..captured.clone()
    };
    for (id, left, source) in [
        (
            "live",
            35000,
            Source::ViewLink {
                view: captured.clone(),
            },
        ),
        (
            "copy",
            235000,
            Source::ViewSnapshot {
                view: captured.clone(),
            },
        ),
    ] {
        let mut item = page.items[0].clone();
        item.id = id.into();
        item.name = if id == "live" {
            "Canlı · Plan güncel"
        } else {
            "Kopya · İlk kayıt"
        }
        .into();
        item.frame = kentos_sheet::units::RectUm::new(left, 60000, 150000, 120000);
        let ItemKind::Map(map) = &mut item.kind else {
            panic!()
        };
        map.source = source;
        let MapView::Fixed(view) = &mut map.view else {
            panic!()
        };
        view.center = Some(kentos_sheet::GroundPoint { x: 2.5, y: 1.5 });
        view.scale = 100;
        view.ratio = None;
        view.rotation = 0;
        items.push(item);
        let mut kind = kentos_sheet::kinds::default_kind("text").unwrap();
        let ItemKind::Text(t) = &mut kind else {
            panic!()
        };
        t.content = if id == "live" {
            "Canlı görünüm · [% @olcek %]"
        } else {
            "Kopya görünüm · [% @olcek %]"
        }
        .into();
        t.map = Some(id.into());
        t.style.size = 3000;
        items.push(kentos_sheet::Item::new(
            &format!("title-{id}"),
            &format!("Başlık {id}"),
            kentos_sheet::units::RectUm::new(left, 190000, 150000, 15000),
            kind,
        ));
    }
    page.items = items;
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets.push(page);
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.sheets.fit.json"
    ))
    .unwrap();
    let mut snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(fixture["setup"].clone()).unwrap();
    snapshot.settings.named_views = vec![current];
    snapshot.entities.clear();
    snapshot.entities.push(serde_json::from_value(serde_json::json!({"kind":"polyline","id":1,"layerId":"cizim","attrs":{},"closed":true,"color":"#1747aa","pts":[{"x":0.,"y":0.},{"x":5.,"y":0.},{"x":5.,"y":3.},{"x":0.,"y":3.}]})).unwrap());
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let answer=kentos_native_application::registry::run(&mut kentos_native_application::ExecutionContext::new(&mut model),"cad.viewmarks.create",kentos_commands::CallOp::Execute,serde_json::json!({"layerId":"cizim","marks":[{"form":"elevation","points":[{"x":2.5,"y":1.5}],"mark":"42","size":2.5}]})).unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    let pdf = kentos_plot::book::book_pdf(&model, &[], "Test", "2026-10-22").unwrap();
    std::fs::write(out.join("sheet14-view-links.pdf"), pdf.bytes).unwrap();
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet14-links");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        sheet(&mut app, Sheet::Tab(1));
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::Select(vec!["live".into()]));
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet14-view-links-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(
                iced::Point::new(w - 100., h - 140.),
                if text == 18 { -15. } else { -8. },
            ),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet14-view-source-{theme}-{text}.png")))
            .unwrap();
        let maps: Vec<_> = app
            .sheets
            .display_list()
            .unwrap()
            .prims
            .iter()
            .filter_map(|p| {
                if let kentos_sheet::display::Prim::Map(m) = p {
                    Some(m)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(maps.len(), 2);
        assert!((maps[0].view.scale - 50.).abs() < 1e-10);
        assert_eq!(maps[1].view.scale, 100.);
    }
}

#[test]
#[ignore = "GPU images and PDF, run alone"]
fn sheet15_annotation_layer_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sheet/v1/viewport-annotations.json"
    ))
    .unwrap();
    let cases = data["cases"].as_array().unwrap();
    let mut page: kentos_sheet::Sheet =
        serde_json::from_value(cases[12]["sheet"].clone()).unwrap();
    page.name = "A-101 · Katman ve açıklama temsilleri".into();
    page.items.clear();
    for (index, c) in cases[12..16].iter().enumerate() {
        let mut item: kentos_sheet::Item =
            serde_json::from_value(c["sheet"]["items"][0].clone()).unwrap();
        let patch: kentos_sheet::ops::Op =
            serde_json::from_value(c["operation"].clone()).unwrap();
        let mut one = kentos_sheet::SheetBook::default();
        let mut single = page.clone();
        single.items.clear();
        single.items.push(item);
        one.sheets.push(single);
        item = kentos_sheet::ops::apply(&one, &patch).unwrap().book.sheets[0]
            .items
            .remove(0);
        item.id = format!("v{index}");
        item.name = c["name"].as_str().unwrap().into();
        item.padding = 0;
        item.frame = kentos_sheet::units::RectUm::new(
            20000 + (index as i32 % 2) * 200000,
            30000 + (index as i32 / 2) * 125000,
            170000,
            85000,
        );
        if let kentos_sheet::ItemKind::Map(m) = &mut item.kind {
            m.label_band = 0;
        }
        let left = item.frame.left;
        let top = item.frame.top;
        page.items.push(item);
        let mut kind = kentos_sheet::kinds::default_kind("text").unwrap();
        if let kentos_sheet::ItemKind::Text(t) = &mut kind {
            t.content = c["name"].as_str().unwrap().into();
            t.style.size = 3000;
        }
        page.items.push(kentos_sheet::Item::new(
            &format!("title{index}"),
            &format!("Başlık {index}"),
            kentos_sheet::units::RectUm::new(left, top + 90000, 170000, 15000),
            kind,
        ));
    }
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets.push(page);
    let snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(cases[12]["model"].clone()).unwrap();
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(
        out.join("sheet15-annotations.pdf"),
        kentos_plot::book::book_pdf(&model, &[], "Test", "2026-10-22")
            .unwrap()
            .bytes,
    )
    .unwrap();
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let (mut app, _store) = app_with_store("sheet15");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
        let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        drive(&mut app, task);
        sheet(&mut app, Sheet::Tab(1));
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::Select(vec!["v1".into()]));
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet15-annotations-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(iced::Point::new(w - 100., h - 140.), -12.),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet15-controls-{theme}-{text}.png")))
            .unwrap();
    }
}

fn identity_scope_app(index: usize, theme: &str, text: u32) -> App {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sheet/v1/viewport-identities.json"
    ))
    .unwrap();
    let c = &data["cases"][index];
    let snapshot: kentos_commands::DocumentSnapshotV1 =
        serde_json::from_value(c["model"].clone()).unwrap();
    let mut model = kentos_domain::Document::from_snapshot(snapshot).unwrap();
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut model),
        "cad.sheets.edit",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"ops":[{"op":"addSheet","sheet":c["sheet"]},c["operation"]]}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed");
    let (mut app, _store) = app_with_store("sheet16-identities");
    app.settings.choose(&[
        ("appearance.theme", serde_json::json!(theme)),
        ("appearance.textSize", serde_json::json!(text)),
    ]);
    app.apply_settings();
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    sheet(&mut app, Sheet::Tab(1));
    app
}
#[test]
fn sheet_model_scope_uses_visible_source_ids_and_new_text_takes_the_physical_paper_scale() {
    let mut app = identity_scope_app(8, "light", 13);
    sheet(&mut app, Sheet::EnterModel("viewport".into()));
    assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 5);
    let at = app
        .document
        .as_ref()
        .unwrap()
        .model
        .get(kentos_domain::Slot(5))
        .unwrap();
    let kentos_commands::Entity::Point(p) = at else {
        panic!()
    };
    let p = [p.p.x, p.p.y];
    model_click(&mut app, p, false);
    model_click(&mut app, p, true);
    assert!(app.selection.contains(kentos_domain::Slot(5)));
    app.cancel();
    app.draft.snap = false;
    let task = app.update(Message::Run("tool.text"));
    drive(&mut app, task);
    model_click(&mut app, [-1., 0.], false);
    assert!(app.text_field.is_some());
    let task = app.update(Message::TextField(crate::text_field::Event::Input(
        "PAPER37".into(),
    )));
    drive(&mut app, task);
    let task = app.update(Message::TextField(crate::text_field::Event::Keep));
    drive(&mut app, task);
    let doc = app.document.as_ref().unwrap();
    let t = doc
        .model
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Text(t) if t.text == "PAPER37" => Some(t),
            _ => None,
        })
        .unwrap();
    assert!(
        (t.height
            - 37.5
                * doc
                    .model
                    .settings()
                    .annotation_mm(kentos_commands::AnnotationKind::Text)
                / 1000.)
            .abs()
            < 1e-12
    );
    app.undo();
    assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 5);
    sheet(&mut app, Sheet::ReturnPaper);
    app.sync_sheet_model_index();
    assert!(
        app.spatial
            .store()
            .shown(app.spatial.store().get(4.).unwrap())
    );
}
#[test]
#[ignore="GPU images, run alone"]
fn sheet16_original_identity_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = identity_scope_app(12, theme, text);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        sheet(&mut app, Sheet::EnterModel("viewport".into()));
        let point = app
            .document
            .as_ref()
            .unwrap()
            .model
            .get(kentos_domain::Slot(5))
            .unwrap();
        let kentos_commands::Entity::Point(p) = point else {
            panic!()
        };
        let at = [p.p.x, p.p.y];
        model_click(&mut app, at, false);
        model_click(&mut app, at, true);
        snap.settle(&mut app, App::view, &mut update);
        assert!(app.selection.contains(kentos_domain::Slot(5)));
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet16-source-selection-{theme}-{text}.png")))
            .unwrap();
        assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 5);
    }
}

#[test]
fn an_active_viewport_index_cannot_hide_source_objects_in_another_map_or_export() {
    use kentos_sheet_ui::MapPainter;
    let mut app = identity_scope_app(12, "light", 13);
    let mut map = app
        .sheets
        .export_list()
        .unwrap()
        .prims
        .into_iter()
        .find_map(|p| match p {
            kentos_sheet::display::Prim::Map(m) => Some(m),
            _ => None,
        })
        .unwrap();
    map.item = "other-unclipped-source-view".into();
    map.clip_shape = kentos_sheet::clip::Clip::Frame;
    map.layer_overrides.clear();
    map.layers = Default::default();
    let content = |app: &App| {
        let request =
            kentos_sheet_ui::MapRequest::of(&map, iced::Size::new(1000., 600.), 0.003, true);
        app.sheet_painter()
            .vector(&request)
            .expect("source vector content")
    };
    let before = content(&app);
    sheet(&mut app, Sheet::EnterModel("viewport".into()));
    assert!(!app.spatial.store().view_shown(4.));
    assert_eq!(
        content(&app),
        before,
        "the model interaction scope must not become another viewport's data source"
    );
    assert_eq!(app.document.as_ref().unwrap().model.entities().count(), 5);
}

#[test]
fn a_lost_active_source_returns_to_paper_instead_of_using_an_unscoped_model_camera() {
    let mut app=identity_scope_app(12,"light",13);let camera=app.viewport.camera;
    let view=kentos_commands::NamedView{id:"lost-view".into(),name:"Plan".into(),center:kentos_commands::Vec2{x:0.,y:0.},width:19.5,height:11.5,twist:0.};
    let mut ctx=app.sheets.context().clone();ctx.named_views=vec![view.clone()];app.sheets.set_context(ctx.clone());let mut book=app.sheets.book().clone();if let kentos_sheet::ItemKind::Map(m)=&mut book.sheets[0].items[0].kind {m.source=kentos_sheet::viewport::Source::ViewLink{view};}
    app.sheets.load(book);app.sheets.update(Sheet::Tab(1));app.sheets.update(Sheet::EnterModel("viewport".into()));app.sync_sheet_model_camera();assert!(app.spatial.has_sheet_view());
    ctx.named_views.clear();app.sheets.set_context(ctx);app.sync_sheet_model_camera();assert!(app.sheets.model_scope().is_none());assert!(!app.spatial.has_sheet_view());assert_eq!(app.viewport.camera,camera);
}

fn multipage_set_app(theme: &str, text: u32) -> App {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sheet/v1/multipage-sets.json"
    ))
    .unwrap();
    let c = &data["cases"][8];
    let mut model = kentos_domain::Document::from_snapshot(
        serde_json::from_value(c["model"].clone()).unwrap(),
    )
    .unwrap();
    let book: kentos_sheet::SheetBook = serde_json::from_value(c["book"].clone()).unwrap();
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let (mut app, _store) = app_with_store("sheet17-sets");
    app.settings.choose(&[
        ("appearance.theme", serde_json::json!(theme)),
        ("appearance.textSize", serde_json::json!(text)),
    ]);
    app.apply_settings();
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    sheet(&mut app, Sheet::Tab(1));
    sheet(
        &mut app,
        Sheet::Pdf(kentos_sheet_ui::message::PdfMessage::Scope(
            kentos_sheet_ui::message::PdfScope::All,
        )),
    );
    app
}
#[test]
fn desktop_set_pdf_resolves_each_master_map_and_the_four_named_architectural_views() {
    let app = multipage_set_app("light", 13);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let painter = app.sheet_painter();
    let (bytes, _) = app
        .sheets
        .pdf_and_findings(&painter, app.sheet_pdf_host())
        .unwrap();
    let dir = std::env::temp_dir().join(format!("kentos-sheet17-ui-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("set.pdf");
    std::fs::write(&path, bytes).unwrap();
    let read = std::process::Command::new("pdftotext")
        .args(["-layout"])
        .arg(&path)
        .arg("-")
        .output()
        .unwrap();
    assert!(read.status.success());
    let words = String::from_utf8(read.stdout).unwrap();
    let pages = words.split('\u{c}').collect::<Vec<_>>();
    assert!(!pages[0].contains("FAR"));
    for (n, word) in [
        (0, "CENTER"),
        (1, "FAR"),
        (2, "PLAN"),
        (3, "ELEV"),
        (4, "SECT"),
        (5, "DETAIL"),
    ] {
        assert!(pages[n].contains(word), "page {} {words}", n + 1);
    }
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
#[ignore="GPU images and PDF, run alone"]
fn sheet17_multipage_set_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = multipage_set_app(theme, text);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        for (tab, label) in [(1, "small"), (2, "large"), (3, "plan"), (5, "section")] {
            sheet(&mut app, Sheet::Tab(tab));
            snap.settle(&mut app, App::view, &mut update);
            snap.render(app.view(), &app.theme())
                .save(out.join(format!("sheet17-{label}-{theme}-{text}.png")))
                .unwrap();
        }
        if theme == "light" {
            let painter = app.sheet_painter();
            std::fs::write(
                out.join("sheet17-set.pdf"),
                app.sheets.pdf(&painter, app.sheet_pdf_host()).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
#[ignore="GPU page setup window, run alone"]
fn sheet18_page_setup_screens() {
    use kentos_sheet_ui::page_setup::{SetupMessage as P, Area};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = multipage_set_app(theme, text);
        sheet(&mut app, Sheet::Tab(3));
        for m in [
            P::Open,
            P::Name("Ofis · A3 baskı".into()),
            P::Area(Area::Window),
            P::Window(0, 10.),
            P::Window(1, 10.),
            P::Window(2, 390.),
            P::Window(3, 250.),
            P::Fit(true),
            P::Center(true),
        ] {
            sheet(&mut app, Sheet::PageSetup(m));
        }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet18-page-setup-{theme}-{text}.png")))
            .unwrap();
        sheet(&mut app, Sheet::PageSetup(P::Save));
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        assert!(app.sheets.open_sheet().unwrap().plot.is_some());
    }
}

#[test]
#[ignore="GPU profile choice window, run alone"]
fn sheet19_plot_profile_screens() {
    use kentos_sheet_ui::page_setup::SetupMessage as P;
    use kentos_sheet::{
        ops::Op,
        plot_profile::{SavePlotProfiles, PlotProfile},
    };
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sheet/v1/plot-profiles.json"
    ))
    .unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = multipage_set_app(theme, text);
        sheet(&mut app, Sheet::Tab(3));
        let profiles: Vec<PlotProfile> = [0, 10]
            .into_iter()
            .map(|i| {
                serde_json::from_value(data["cases"][i]["exchange"]["profile"].clone()).unwrap()
            })
            .collect();
        let id = profiles[1].id.clone();
        assert!(app.sheets.apply(
            vec![Op::SavePlotProfiles(SavePlotProfiles { profiles })],
            "Baskı profilleri"
        ));
        for m in [P::Open, P::Name("Profil ile baskı".into()), P::Profile(id)] {
            sheet(&mut app, Sheet::PageSetup(m));
        }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet19-profiles-{theme}-{text}.png")))
            .unwrap();
        if theme=="light" {
            use kentos_ui::snapshot::Input;
            snap.input(&mut app,App::view,&mut update,Input::Click(iced::Point::new(700.,566.)));
            snap.render(app.view(),&app.theme()).save(out.join("sheet19-profiles-menu.png")).unwrap();
            snap.input(&mut app,App::view,&mut update,Input::Key(iced::keyboard::key::Named::ArrowUp));
            snap.input(&mut app,App::view,&mut update,Input::Key(iced::keyboard::key::Named::Enter));
        }
        sheet(&mut app, Sheet::PageSetup(P::Save));
        if theme=="light" {assert_eq!(app.sheets.open_sheet().unwrap().plot.as_ref().unwrap().settings.profile.as_deref(),Some("profile-0"));}
        assert!(app.sheets.error().is_none());
        assert!(
            app.sheets
                .open_sheet()
                .unwrap()
                .plot
                .as_ref()
                .unwrap()
                .settings.profile_snapshot
                .is_some()
        );
    }
}

#[test]
#[ignore="GPU plot ink controls and real PDF, run alone"]
fn sheet20_plot_ink_screens() {
    use kentos_sheet_ui::plot_profiles::{PlotMessage as P, ColorMode};
    use kentos_sheet::{plot_profile::TableKind, LineCap, LineJoin};
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 650.),
    ] {
        let mut app = multipage_set_app(theme, text);
        sheet(&mut app, Sheet::Tab(3));
        for m in [
            P::Open,
            P::New(TableKind::Named),
            P::Name("Mimari · gri ve ince".into()),
            P::ColorMode(ColorMode::Gray),
            P::Screening(70.),
            P::Weight(true),
            P::WeightMm(0.35),
            P::Dash(true),
            P::DashText("2 1".into()),
            P::Opacity(true),
            P::Alpha(80.),
            P::Cap(Some(LineCap::Round)),
            P::Join(Some(LineJoin::Miter)),
        ] {
            sheet(&mut app, Sheet::PlotProfiles(m));
        }
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet20-ink-top-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Scroll(iced::Point::new(w / 2., h / 2.), -25.),
        );
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet20-ink-bottom-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Click(iced::Point::new(
                if text == 18 { 930. } else { 990. },
                h - 32.,
            )),
        );
        assert!(
            app.sheets.book().sheets[2]
                .plot
                .as_ref()
                .is_some_and(|p| p.settings.profile_snapshot.is_some())
        );
        assert!(app.sheets.error().is_none(), "{:?}", app.sheets.error());
        if theme == "light" {
            let painter = app.sheet_painter();
            std::fs::write(
                out.join("sheet20-ink.pdf"),
                app.sheets.pdf(&painter, app.sheet_pdf_host()).unwrap(),
            )
            .unwrap();
        }
    }
}

#[test]
fn desktop_pdf_refuses_an_impossible_model_curve_without_a_gray_raster_fallback() {
    let data: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/sheet/v1/plot-document.json"
    ))
    .unwrap();
    let mut source = data["model"].clone();
    source["entities"][1]["r"] = serde_json::json!(1e10);
    let mut model =
        kentos_domain::Document::from_snapshot(serde_json::from_value(source).unwrap())
            .unwrap();
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets
        .push(serde_json::from_value(data["cases"][0]["sheet"].clone()).unwrap());
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let (mut app, _store) = app_with_store("sheet22-quality");
    let doc = Document::from_v2(model.to_snapshot_v2(), None).unwrap();
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(&mut app, task);
    sheet(&mut app, Sheet::Tab(1));
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let painter = app.sheet_painter();
    let error = app
        .sheets
        .pdf_and_findings(&painter, app.sheet_pdf_host())
        .unwrap_err();
    assert!(error.contains("plot_curve_too_complex"), "{error}");
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}

#[test]
fn sheet23_real_save_and_print_use_the_same_native_pdf() {
    let mut app = multipage_set_app("light", 13);
    sheet(&mut app, Sheet::Tab(3));
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let doc = app.sheet_output_snapshot().unwrap();
    let options = app.sheets.output_pdf_options();
    let expected = kentos_plot::book::book_pdf_options(
        &doc,
        &options.sheets.clone(),
        "",
        &kentos_plot::book::today(),
        None,
        Some(options),
    )
    .unwrap();
    let path = scratch("sheet23-real-save").join("native.pdf");
    app.sheet_export_to(kentos_sheet_ui::ExportKind::Pdf, Some(path.clone()));
    assert_eq!(std::fs::read(path).unwrap(), expected.bytes);
    let _ = app.sheet_print();
    let path = crate::sheet_pdf::OPENED
        .with(|p| p.borrow().last().cloned())
        .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), expected.bytes);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}

#[test]
#[ignore = "GPU print preview controls, run alone"]
fn sheet23_print_preview_screens() {
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [
        ("light", 13, 1440., 900.),
        ("dark", 13, 1440., 900.),
        ("dark", 18, 1100., 720.),
    ] {
        let mut app = multipage_set_app(theme, text);
        sheet(&mut app, Sheet::Tab(3));
        sheet(&mut app, Sheet::Print);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet23-options-{theme}-{text}.png")))
            .unwrap();
        snap.input(
            &mut app,
            App::view,
            &mut update,
            kentos_ui::snapshot::Input::Click(iced::Point::new(
                if text == 18 { 910. } else { 975. },
                if text == 18 { 550. } else { 605. },
            )),
        );
        snap.settle(&mut app, App::view, &mut update);
        let shot = snap.render(app.view(), &app.theme());
        let white = shot
            .rgba
            .chunks_exact(4)
            .filter(|p| p[0] > 248 && p[1] > 248 && p[2] > 248)
            .count();
        shot.save(out.join(format!("sheet23-preview-{theme}-{text}.png")))
            .unwrap();
        assert!(
            white > 20_000,
            "A printed white page must be visible: {white}"
        );
        assert!(app.sheets.error().is_none());
    }
}

#[test]
#[ignore = "GPU calibration sheet and common native output, run alone"]
fn sheet24_calibration_screens() {
    use kentos_sheet::template::*;
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    let data: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/sheet/v1/calibration.json")).unwrap();
    let t = system_template("sys:baski-kalibrasyonu").unwrap();
    let ids = InstanceIds {
        sheet: "cal".into(),
        master: None,
        items: t.sheet.items.iter().map(|i| i.id.clone()).collect(),
        master_items: Vec::new(),
    };
    let inst = instantiate(
        t,
        &ids,
        &InstanceOptions {
            center: Some(kentos_sheet::GroundPoint { x: 0., y: 0. }),
            scale: Some(200),
            ..Default::default()
        },
    )
    .unwrap();
    let mut book = kentos_sheet::SheetBook::default();
    book.sheets.push(inst.sheet);
    let mut model = kentos_domain::Document::from_snapshot(
        serde_json::from_value(data["cases"][6]["model"].clone()).unwrap(),
    )
    .unwrap();
    model
        .set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    for (theme, text, w, h) in [("light", 13, 1440., 900.), ("dark", 18, 1100., 720.)] {
        let (mut app, _) = app_with_store("sheet24-calibration");
        app.settings.choose(&[
            ("appearance.theme", serde_json::json!(theme)),
            ("appearance.textSize", serde_json::json!(text)),
        ]);
        app.apply_settings();
        let task = app.update(Message::Opened(Some(Ok(Box::new(
            Document::from_v2(model.to_snapshot_v2(), None).unwrap(),
        )))));
        drive(&mut app, task);
        sheet(&mut app, Sheet::Tab(1));
        sheet(&mut app, Sheet::ZoomPage);
        let mut snap = kentos_ui::snapshot::Snapshot::new(iced::Size::new(w, h)).unwrap();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet24-calibration-{theme}-{text}.png")))
            .unwrap();
        sheet(&mut app, Sheet::Print);
        sheet(
            &mut app,
            Sheet::Pdf(kentos_sheet_ui::message::PdfMessage::Preview),
        );
        snap.settle(&mut app, App::view, &mut update);
        snap.render(app.view(), &app.theme())
            .save(out.join(format!("sheet24-preview-{theme}-{text}.png")))
            .unwrap();
        if theme == "light" {
            app.sheet_export_to(
                kentos_sheet_ui::ExportKind::Pdf,
                Some(out.join("sheet24-calibration.pdf")),
            );
        }
    }
}
