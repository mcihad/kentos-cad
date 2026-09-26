//! The application menu's flows (app_menu/): opening and closing, rows
//! running their commands, what the desktop does not run yet, the
//! keyboard, the recent files and the cloud's recent projects.

use iced::Task;
use iced::keyboard::Modifiers;
use iced::keyboard::key::{Key, Named, NativeCode, Physical};
use kentos_cloud::ApiFailure;
use kentos_contracts::ProjectStorage;

use super::{Event, Focus, Pane, Projects, State};
use crate::app::{App, Dialog, Message, Picker, Then};
use crate::cloud::tests::{PROJECT, TENANT, page, signed_in, summary};
use crate::files_testing::{app_with_drawing, drive, saved, scratch};
use crate::keys::KeyPress;

fn menu(app: &mut App, e: Event) -> Task<Message> {
    app.update(Message::AppMenu(e))
}

fn key(app: &mut App, named: Named) {
    let _ = app.update(Message::Key(KeyPress {
        key: Key::Named(named),
        physical: Physical::Unidentified(NativeCode::Unidentified),
        modifiers: Modifiers::default(),
        text: None,
        repeat: false,
    }));
}

fn state(app: &App) -> &State {
    app.app_menu.as_ref().expect("the menu is open")
}

#[test]
fn the_mark_opens_and_closes_the_menu() {
    let mut app = app_with_drawing();
    let _ = menu(&mut app, Event::Toggle);
    assert_eq!(state(&app).pane, Pane::Overview, "the drawing first");
    assert_eq!(state(&app).focus, None);
    let _ = menu(&mut app, Event::Toggle);
    assert!(app.app_menu.is_none(), "the mark again closes it");
    let _ = menu(&mut app, Event::Toggle);
    let _ = menu(&mut app, Event::Dismiss);
    assert!(app.app_menu.is_none(), "a click beside it closes it");
    let _ = menu(&mut app, Event::Toggle);
    key(&mut app, Named::Escape);
    assert!(app.app_menu.is_none(), "Esc closes it");
    assert!(app.dialog.is_none());
    // A command from anywhere (a shortcut, the ribbon) closes it too, as on the web.
    let _ = menu(&mut app, Event::Toggle);
    let _ = app.update(Message::Run("view.zoomExtents"));
    assert!(app.app_menu.is_none());
}

#[test]
fn rows_run_their_commands_once_the_menu_closes() {
    let mut app = app_with_drawing();
    let _ = menu(&mut app, Event::Toggle);
    let _ = menu(&mut app, Event::Run("file.new"));
    assert!(app.app_menu.is_none());
    assert_eq!(app.dialog, Some(Dialog::Project), "Yeni proje's window");
    key(&mut app, Named::Escape);
    let _ = menu(&mut app, Event::Toggle);
    let _ = menu(&mut app, Event::Run("tools.options"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Settings),
        "the footer's Uygulama ayarları"
    );
    key(&mut app, Named::Escape);
    let _ = menu(&mut app, Event::Toggle);
    let _ = menu(&mut app, Event::Run("file.start"));
    assert_eq!(app.dialog, Some(Dialog::Start));
}

#[test]
fn what_the_desktop_does_not_run_yet_is_dimmed_and_says_why() {
    let app = app_with_drawing();
    assert!(!app.menu_runs("file.print"));
    assert_eq!(app.menu_why("file.print"), Some("Geliştirme aşamasında"));
    assert!(!app.menu_runs("cloud.rename"));
    assert_eq!(
        app.menu_why("cloud.rename"),
        Some("Web'de var; masaüstüne henüz taşınmadı")
    );
    for id in ["file.import.dxf", "file.import.shp", "file.import.geojson"] {
        assert!(app.menu_runs(id), "{id}");
        assert_eq!(app.menu_why(id), None, "{id}");
    }
    // Without a drawing there is nothing to save, set or export; opening is.
    let (empty, _) = App::boot(None);
    assert!(!empty.menu_runs("file.save"));
    assert_eq!(empty.menu_why("file.save"), Some("Açık çizim yok"));
    assert!(!empty.menu_runs("file.export.dxf"));
    assert!(empty.menu_runs("file.open"));
    assert!(empty.menu_runs("file.new"));
}

#[test]
fn the_lines_say_where_the_drawing_is_kept_and_who_is_signed_in() {
    let mut app = app_with_drawing();
    assert_eq!(app.save_where(), "İlk kayıtta yer sorulur");
    assert_eq!(
        app.cloud_line(),
        "Giriş yapın, projelerinizi açın ve paylaşın"
    );
    let dir = scratch("menu-lines");
    let path = saved(&dir, "Ada 1244.kcad", 0);
    let task = app.start_opening(path, crate::opening::Purpose::File);
    drive(&mut app, task);
    assert_eq!(app.save_where(), "Ada 1244.kcad");
    let mut app = signed_in();
    assert_eq!(app.cloud_line(), "Ayşe Yılmaz olarak giriş yapıldı");
    app.cloud.link = crate::cloud::copy::Link::Offline;
    assert_eq!(app.cloud_line(), "Sunucuya ulaşılamıyor");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_keyboard_walks_the_rows_and_enter_runs_one() {
    let mut app = app_with_drawing();
    let _ = menu(&mut app, Event::Toggle);
    key(&mut app, Named::ArrowDown);
    assert_eq!(state(&app).focus, Some(Focus::Nav(0)), "Yeni");
    key(&mut app, Named::ArrowUp);
    assert_eq!(
        state(&app).focus,
        Some(Focus::Nav(8)),
        "round to Proje ayarları"
    );
    // Down to İçe aktar: its formats show on the right.
    for _ in 0..5 {
        key(&mut app, Named::ArrowDown);
    }
    assert_eq!(state(&app).focus, Some(Focus::Nav(4)));
    assert_eq!(state(&app).pane, Pane::Import);
    key(&mut app, Named::ArrowRight);
    assert_eq!(state(&app).focus, Some(Focus::Pane(0)), "DXF");
    key(&mut app, Named::ArrowDown);
    assert_eq!(
        state(&app).focus,
        Some(Focus::Pane(1)),
        "the coordinate list"
    );
    // NCZ does not run here yet: the keyboard passes it.
    key(&mut app, Named::ArrowDown);
    assert_eq!(state(&app).focus, Some(Focus::Pane(3)), "Shapefile");
    key(&mut app, Named::ArrowDown);
    assert_eq!(state(&app).focus, Some(Focus::Pane(4)), "GeoJSON");
    key(&mut app, Named::ArrowDown);
    assert_eq!(state(&app).focus, Some(Focus::Pane(0)), "round to DXF");
    key(&mut app, Named::ArrowLeft);
    assert_eq!(state(&app).focus, Some(Focus::Nav(4)), "back to İçe aktar");
    // Enter on a command row runs it.
    key(&mut app, Named::ArrowUp);
    key(&mut app, Named::ArrowUp);
    key(&mut app, Named::ArrowUp);
    key(&mut app, Named::ArrowUp);
    assert_eq!(state(&app).focus, Some(Focus::Nav(0)));
    key(&mut app, Named::Enter);
    assert!(app.app_menu.is_none());
    assert_eq!(app.dialog, Some(Dialog::Project));
}

#[test]
fn a_recent_file_opens_again_and_a_missing_one_leaves_the_list() {
    let dir = scratch("menu-recent");
    let path = saved(&dir, "Ada 1244.kcad", 0);
    let gone = dir.join("taşındı.kcad");
    let (mut app, _) = App::boot(None);
    app.recent.add(&path, "13 nesne · TUREF / TM36".into());
    app.recent.add(&gone, "2 nesne · TUREF / TM36".into());
    let _ = menu(&mut app, Event::Toggle);
    let task = menu(&mut app, Event::OpenRecent(gone.clone()));
    drive(&mut app, task);
    assert!(app.app_menu.is_none());
    assert!(app.document.is_none(), "nothing opened");
    assert_eq!(app.recent.list().len(), 1, "the missing file left the list");
    assert!(crate::files_testing::last_said(&app).contains("bulunamadı"));
    let _ = menu(&mut app, Event::Toggle);
    let task = menu(&mut app, Event::OpenRecent(path.clone()));
    drive(&mut app, task);
    assert_eq!(
        app.document.as_ref().and_then(|d| d.path.clone()),
        Some(path.clone())
    );
    assert_eq!(app.recent.list()[0].path, path);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_recent_file_asks_about_unsaved_work_first() {
    let dir = scratch("menu-unsaved");
    let path = saved(&dir, "Ada 1244.kcad", 2);
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    doc.model.set_name("Değişen çizim");
    assert!(doc.dirty());
    let _ = menu(&mut app, Event::OpenRecent(path.clone()));
    assert_eq!(app.dialog, Some(Dialog::Unsaved(Then::OpenRecent)));
    assert_eq!(
        app.unsaved_question(Then::OpenRecent).confirm,
        "Kaydetmeden aç"
    );
    // Vazgeç: the drawing stays, the file is not opened later by surprise.
    key(&mut app, Named::Escape);
    assert_eq!(app.dialog, None);
    assert!(app.opening_recent.is_none());
    assert_eq!(
        app.document.as_ref().map(|d| d.name()),
        Some("Değişen çizim")
    );
    // Again, and Kaydetmeden aç this time.
    let _ = menu(&mut app, Event::OpenRecent(path.clone()));
    let task = app.update(Message::DialogConfirmed);
    drive(&mut app, task);
    assert_eq!(
        app.document.as_ref().and_then(|d| d.path.clone()),
        Some(path)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn opened_and_saved_files_go_first_in_the_recent_files() {
    let dir = scratch("menu-remember");
    let a = saved(&dir, "a.kcad", 0);
    let b = saved(&dir, "b.kcad", 2);
    let (mut app, _) = App::boot(None);
    let task = app.start_opening(a.clone(), crate::opening::Purpose::File);
    drive(&mut app, task);
    let task = app.start_opening(b.clone(), crate::opening::Purpose::File);
    drive(&mut app, task);
    let list = app.recent.list();
    assert_eq!(list[0].path, b);
    assert_eq!(list[0].info, "15 nesne · TUREF / TM36");
    assert_eq!(list[1].path, a);
    assert_eq!(list[1].info, "13 nesne · TUREF / TM36");
    // Farklı kaydet writes another file: it comes first.
    let c = dir.join("c.kcad");
    app.picker = Picker::File(c.clone());
    let task = app.update(Message::Run("file.saveAs"));
    drive(&mut app, task);
    assert_eq!(app.recent.list()[0].path, c);
    assert_eq!(app.recent.list().len(), 3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_cloud_pane_lists_the_accounts_recent_projects() {
    // Signed out: signing in and this device's projects, and nothing asked of a server.
    let (mut app, _) = App::boot(None);
    let _ = menu(&mut app, Event::Toggle);
    let _ = menu(&mut app, Event::Show(Pane::Cloud));
    assert!(matches!(state(&app).projects, Projects::NotAsked));
    let ids: Vec<&str> = app.cloud_actions().iter().map(|(id, ..)| *id).collect();
    assert_eq!(ids, ["cloud.signIn", "cloud.open"]);

    let mut app = signed_in();
    let _ = menu(&mut app, Event::Toggle);
    let _ = menu(&mut app, Event::Show(Pane::Cloud));
    let id = state(&app).request;
    assert!(matches!(state(&app).projects, Projects::Loading));
    // A late answer to an earlier request is dropped.
    let _ = menu(
        &mut app,
        Event::Projects {
            id: id + 100,
            result: Box::new(Ok(page(
                vec![summary(PROJECT, "Eski", ProjectStorage::Database)],
                1,
                None,
            ))),
        },
    );
    assert!(matches!(state(&app).projects, Projects::Loading));
    let _ = menu(
        &mut app,
        Event::Projects {
            id,
            result: Box::new(Ok(page(
                vec![summary(PROJECT, "Ada 1244", ProjectStorage::File)],
                1,
                None,
            ))),
        },
    );
    assert!(
        matches!(&state(&app).projects, Projects::Loaded(p) if p.len() == 1 && p[0].name == "Ada 1244")
    );
    // Showing the pane again does not ask again.
    let _ = menu(&mut app, Event::Show(Pane::Overview));
    let _ = menu(&mut app, Event::Show(Pane::Cloud));
    assert!(matches!(state(&app).projects, Projects::Loaded(_)));
    // A row opens the project, named in its opening window.
    let _ = menu(
        &mut app,
        Event::OpenProject {
            tenant: TENANT.into(),
            project: PROJECT.into(),
        },
    );
    assert!(app.app_menu.is_none());
    let opening = app.cloud.opening.as_ref().expect("the project opens");
    assert_eq!(opening.name, "Ada 1244");

    // A failed request says so in the pane.
    let mut app = signed_in();
    let _ = menu(&mut app, Event::Toggle);
    let _ = menu(&mut app, Event::Show(Pane::Cloud));
    let id = state(&app).request;
    let _ = menu(
        &mut app,
        Event::Projects {
            id,
            result: Box::new(Err(ApiFailure::new(0, "offline", "Sunucuya ulaşılamıyor."))),
        },
    );
    assert!(matches!(state(&app).projects, Projects::Failed));
}

/// Pictures of the menu for the owner, in `.run/shots`:
///
/// ```text
/// cargo test -p kentos-desktop app_menu::tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let dir = scratch("menu-screens");
    let picture = |app: &mut App, name: &str| {
        let mut snapshot = Snapshot::new(Size::new(1440.0, 900.0)).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(app, App::view, &mut update);
        let file = out.join(format!("{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes the picture");
        println!("{}", file.display());
    };
    for mode in ["dark", "light"] {
        let mut app = signed_in();
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
        app.apply_settings();
        let path = saved(&dir, "Ornek_1244-1249_Ada.kcad", 0);
        let task = app.start_opening(path, crate::opening::Purpose::File);
        drive(&mut app, task);
        for (name, info) in [
            ("Kadıköy imar.kcad", "412 nesne · TUREF / TM30"),
            ("Tevhit 118 ada.kcad", "86 nesne · TUREF / TM36"),
        ] {
            app.recent.add(&dir.join(name), info.into());
        }
        let current = app.document.as_ref().and_then(|d| d.path.clone());
        if let Some(path) = current {
            app.recent.add(&path, "13 nesne · TUREF / TM36".into());
        }
        if let Some(doc) = app.document.as_mut() {
            doc.model.set_name("Ornek_1244-1249_Ada");
        }
        let _ = menu(&mut app, Event::Toggle);
        picture(&mut app, &format!("menu-{mode}-1-cizim"));
        let _ = menu(&mut app, Event::Show(Pane::Import));
        key(&mut app, Named::ArrowRight);
        picture(&mut app, &format!("menu-{mode}-2-ice-aktar"));
        let _ = menu(&mut app, Event::Show(Pane::Cloud));
        let id = state(&app).request;
        let mut first = summary(PROJECT, "Ada 1244 ifraz", ProjectStorage::File);
        first.opened_at = Some("2026-09-26T09:00:00Z".into());
        let mut second = summary(
            "0199aaaa-0000-7000-8000-00000000000b",
            "Moda imar planı",
            ProjectStorage::Database,
        );
        second.opened_at = Some("2026-09-24T12:00:00Z".into());
        let _ = menu(
            &mut app,
            Event::Projects {
                id,
                result: Box::new(Ok(page(vec![first, second], 2, None))),
            },
        );
        picture(&mut app, &format!("menu-{mode}-3-bulut"));
        let _ = menu(&mut app, Event::Dismiss);
        app.open_start();
        picture(&mut app, &format!("baslangic-{mode}"));
    }
    let _ = std::fs::remove_dir_all(&dir);
}
