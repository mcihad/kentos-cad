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
