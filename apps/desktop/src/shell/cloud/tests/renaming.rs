//! Cloud tests: saving as a file, renaming, the trash, the last revision, and the pictures for the owner.

use super::*;

// ── The open project's actions (actions.rs) ──────────────────────────────

/// The catalog's answer to a lifecycle command on the open project.
pub(super) fn catalog_change(
    name: &str,
    purge_after: Option<&str>,
) -> kentos_commands::ProjectCatalogChange {
    let mut project = summary(PROJECT, name, ProjectStorage::File);
    project.purge_after = purge_after.map(str::to_owned);
    kentos_commands::ProjectCatalogChange {
        project,
        changed: true,
        event_seq: None,
        replayed: false,
    }
}

#[test]
fn buluta_dosya_olarak_kaydet_opens_the_upload_on_file_storage() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.uploadFile");
    assert_eq!(app.dialog, Some(Dialog::Upload));
    assert_eq!(
        app.cloud.upload.as_ref().expect("the window").storage,
        ProjectStorage::File
    );
    cloud(&mut app, Event::Close);
    let _ = app.run("cloud.upload");
    assert_eq!(
        app.cloud.upload.as_ref().expect("the window").storage,
        ProjectStorage::Database
    );
}

/// A file project's name is the catalog's: the drawing takes it quietly,
/// not as an edit; a refusal stays in the window in its words.
#[test]
fn a_file_project_is_renamed_through_the_catalog() {
    let mut app = signed_in();
    file_project(&mut app);
    assert!(app.cloud_available("cloud.rename"));
    let _ = app.run("cloud.rename");
    assert_eq!(app.dialog, Some(Dialog::CloudRename));
    cloud(&mut app, Event::RenameSubmit);
    assert!(
        !app.cloud.rename.as_ref().expect("the window").busy,
        "the same name is not sent"
    );
    cloud(&mut app, Event::RenameInput("  Ada 102 ".into()));
    cloud(&mut app, Event::RenameSubmit);
    let id = {
        let r = app.cloud.rename.as_ref().expect("the window");
        assert!(r.busy);
        r_id(r)
    };
    cloud(
        &mut app,
        Event::Renamed {
            id,
            result: Err(ApiFailure::new(409, "conflict", "Çakışma")),
        },
    );
    let r = app.cloud.rename.as_ref().expect("the window stays");
    assert!(!r.busy);
    assert_eq!(
        r.status,
        Some(crate::cloud::actions::Status::Error(
            "Proje bilgileri bu arada başka biri tarafından değiştirildi. Listeyi yenileyip yeniden deneyin.".into()
        ))
    );
    cloud(&mut app, Event::RenameSubmit);
    let id = r_id(app.cloud.rename.as_ref().expect("the window"));
    cloud(
        &mut app,
        Event::Renamed {
            id,
            result: Ok(catalog_change("Ada 102", None)),
        },
    );
    assert_eq!(app.dialog, None);
    let doc = app.document.as_ref().expect("open");
    assert_eq!(doc.name(), "Ada 102");
    assert!(!doc.dirty(), "taken quietly, not an edit");
    assert_eq!(
        last_said(&app),
        "Proje “Ada 102” olarak yeniden adlandırıldı."
    );
}

pub(super) fn r_id(r: &crate::cloud::actions::Rename) -> u64 {
    r.id_for_tests()
}

/// A database project's new name goes through its autosave with whatever
/// else waits; said once the server has it, or that it waits on the device.
#[test]
fn a_database_project_is_renamed_through_its_autosave() {
    let mut app = signed_in();
    database(&mut app);
    let _ = app.run("cloud.rename");
    cloud(&mut app, Event::RenameInput("Ada 103".into()));
    cloud(&mut app, Event::RenameSubmit);
    let sent = app
        .cloud
        .live
        .as_ref()
        .and_then(|l| l.sent.clone())
        .expect("the name goes at once");
    let input: ProjectChanges = serde_json::from_value(sent.input).expect("its input");
    assert_eq!(
        input.project.and_then(|p| p.name).as_deref(),
        Some("Ada 103")
    );
    assert_eq!(
        app.dialog,
        Some(Dialog::CloudRename),
        "until the server has it"
    );
    let answered = answer(&app, "8");
    commit(&mut app, Ok(answered));
    assert_eq!(app.dialog, None);
    assert_eq!(
        last_said(&app),
        "Proje “Ada 103” olarak yeniden adlandırıldı."
    );

    // No connection: the name waits on this device.
    let _ = app.run("cloud.rename");
    cloud(&mut app, Event::RenameInput("Ada 104".into()));
    cloud(&mut app, Event::RenameSubmit);
    commit(
        &mut app,
        Err(ApiFailure::new(0, "network", "Sunucuya ulaşılamadı.")),
    );
    assert_eq!(app.dialog, None);
    assert!(
        said(&app).iter().any(
            |t| t == "Yeni ad (“Ada 104”) bu cihazda bekliyor; sunucuya ulaşılınca kaydedilir."
        ),
        "{:?}",
        said(&app)
    );
}

/// Çöpe taşı: asked, then the drawing stays as a local one and the
/// message says until when the project can come back.
#[test]
fn the_open_project_goes_to_the_trash_and_the_drawing_stays() {
    let mut app = signed_in();
    file_project(&mut app);
    // An editor may not move it to the trash; its owner may.
    assert!(!app.cloud_available("cloud.delete"));
    if let Some(source) = app.document.as_mut().and_then(|d| d.cloud_source_mut()) {
        source
            .info
            .access
            .permissions
            .push(ProjectPermission::Delete);
    }
    assert!(app.cloud_available("cloud.delete"));
    let _ = app.run("cloud.delete");
    assert_eq!(app.dialog, Some(Dialog::CloudTrash));
    cloud(&mut app, Event::TrashConfirm);
    let id = app.cloud.trashing.expect("the request on its way");
    cloud(
        &mut app,
        Event::Trashed {
            id,
            result: Ok(catalog_change("Ada 101", Some("2026-10-27T10:00:00Z"))),
        },
    );
    let doc = app.document.as_ref().expect("the drawing stays");
    assert!(doc.cloud_source().is_none(), "no longer a cloud project");
    assert_eq!(
        last_said(&app),
        "“Ada 101” bulut projesi çöp kutusuna taşındı (27.10.2026 tarihine kadar geri yüklenebilir). Çizim ekranda kaldı; saklamak için Dosya → Farklı kaydet ile yerel bir dosyaya kaydedin."
    );
}

/// Son revizyonu aç: a plain question over a clean drawing; over unsaved
/// work the web's question without “Kaydet ve aç” (4b393ae); a viewer of a
/// database project has no such command.
#[test]
fn son_revizyonu_ac_asks_first() {
    use crate::cloud::revisions::{Answer, QuestionId};
    let mut app = signed_in();
    file_project(&mut app);
    assert!(app.cloud_available("cloud.openNewest"));
    let _ = app.run("cloud.openNewest");
    assert_eq!(app.dialog, Some(Dialog::Revision));
    assert_eq!(
        app.cloud.question.as_ref().map(|q| q.id),
        Some(QuestionId::Newest)
    );
    cloud(&mut app, Event::Close);
    assert!(app.cloud.question.is_none(), "Vazgeç leaves nothing");
    edit(&mut app, 1.0);
    let _ = app.run("cloud.openNewest");
    assert_eq!(
        app.cloud.question.as_ref().map(|q| q.id),
        Some(QuestionId::Unsaved)
    );
    cloud(&mut app, Event::RevisionAnswer(Answer::Discard));
    assert!(
        app.cloud.opening.as_ref().is_some_and(|o| o.newest),
        "the newest revision opens from the server"
    );

    let mut db = signed_in();
    database(&mut db);
    assert!(
        !db.cloud_available("cloud.openNewest"),
        "a file project's command"
    );
}

/// The open project's windows, for the owner. Not run by default:
/// `cargo test -p kentos-desktop cloud::tests::action_screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn action_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in [
                "bulut-yeniden-adlandir",
                "bulut-cope-tasi",
                "bulut-son-revizyon",
                "bulut-son-revizyon-kaydedilmemis",
            ] {
                let mut app = signed_in();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                file_project(&mut app);
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                match name {
                    "bulut-yeniden-adlandir" => {
                        let _ = app.run("cloud.rename");
                        cloud(
                            &mut app,
                            Event::RenameInput("Ada 101 (tevhit sonrası)".into()),
                        );
                    }
                    "bulut-cope-tasi" => {
                        if let Some(source) =
                            app.document.as_mut().and_then(|d| d.cloud_source_mut())
                        {
                            source
                                .info
                                .access
                                .permissions
                                .push(ProjectPermission::Delete);
                        }
                        let _ = app.run("cloud.delete");
                    }
                    "bulut-son-revizyon" => {
                        let _ = app.run("cloud.openNewest");
                    }
                    _ => {
                        edit(&mut app, 1.0);
                        let _ = app.run("cloud.openNewest");
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// Pictures for the owner: an editor's database project, the pointer
/// resting on Katmanlar's Yeni katman (off, its tip says why; docs/adr/0078);
/// `.run/shots/bulut-agac-kilitli-*`.
/// `cargo test -p kentos-desktop cloud::tests::tree_locked_screens -- --ignored --nocapture`
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn tree_locked_screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            let mut app = signed_in();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
            app.apply_settings();
            let mut project = info(ProjectStorage::Database, true, ProjectState::Active);
            project.access.permissions =
                vec![ProjectPermission::Read, ProjectPermission::FeatureWrite];
            open(&mut app, project);
            let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            // Katmanlar's header: Yeni katman left of Yeni grup.
            let at = Point::new(width - 91.0, 160.0);
            snapshot.input(&mut app, App::view, &mut update, Input::Move(at));
            std::thread::sleep(std::time::Duration::from_millis(500));
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("bulut-agac-kilitli-{width}x{height}{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
    }
}

/// Buluta yükle for the owner (the web's UploadDialog, parity-audit Y1–Y4):
/// a database project, a file project with the catalog's fields filled, and
/// a file going up; `.run/shots/bulut-yukle-*`.
/// `cargo test -p kentos-desktop cloud::tests::upload_screens -- --ignored --nocapture`
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn upload_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["veritabani", "dosya", "yukleniyor"] {
                let mut app = signed_in();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let _ = app.update(Message::Opened(Some(Ok(Box::new(
                    crate::files_testing::drawing(0),
                )))));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let _ = app.run("cloud.upload");
                if name != "veritabani" {
                    cloud(&mut app, Event::UploadStorage(ProjectStorage::File));
                    cloud(&mut app, Event::UploadName("Kadıköy 1244 ada".into()));
                    cloud(
                        &mut app,
                        Event::UploadType(kentos_commands::ProjectType::Subdivision),
                    );
                    cloud(&mut app, Event::UploadTags("Kadıköy, 2026".into()));
                }
                if name == "yukleniyor" {
                    let _ = app.update(crate::cloud::msg(Event::UploadSubmit));
                    let (id, _) = app
                        .cloud
                        .upload
                        .as_ref()
                        .and_then(|u| u.request())
                        .expect("working");
                    cloud(
                        &mut app,
                        Event::UploadEncoded {
                            id,
                            result: Ok(Once::new(vec![1, 2, 3])),
                        },
                    );
                    cloud(
                        &mut app,
                        Event::UploadProgress {
                            id,
                            done: 3 * 1024 * 1024,
                            total: 7 * 1024 * 1024,
                        },
                    );
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("bulut-yukle-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
