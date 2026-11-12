//! Cloud tests: the draft kept and put back, viewers, file projects' revisions, uploads, an editor's layer
//! tree and the status bar.

use super::*;

// ── Device drafts and leaving ───────────────────────────────────────────

pub(super) fn drafts(app: &mut App, dir: PathBuf) {
    app.cloud.drafts = Some(DraftStore::new(dir));
}

#[test]
fn signing_out_keeps_unsent_work_in_the_draft_and_the_next_opening_puts_it_back() {
    let dir = scratch("bulut-taslak");
    let mut app = signed_in();
    drafts(&mut app, dir.clone());
    database(&mut app);
    let uid = edit(&mut app, 7.0);
    app.cloud_after(Instant::now());
    let task = app.run("cloud.signOut");
    assert!(app.cloud.leaving.is_some(), "the draft first");
    drive(&mut app, task);
    assert!(said(&app).iter().any(|t| t
        == "Gönderilmemiş 1 değişiklik bu cihazda saklandı; proje yeniden açılınca gönderilecek."));
    assert_eq!(last_said(&app), "Bulut oturumu kapatıldı.");
    assert!(app.cloud.me.is_none());
    let doc = app.document.as_ref().expect("the drawing stays");
    assert!(doc.cloud_source().is_none(), "a local drawing now");
    assert!(app.cloud.live.is_none());

    // Signed in again and opened: the edit is back, unsent, and goes.
    app.cloud.client = Some(Client::new("http://127.0.0.1:9").expect("local"));
    app.cloud.me = Some(me());
    database(&mut app);
    assert!(
        said(&app)
            .iter()
            .any(|t| t == "Bu cihazda gönderilmemiş değişiklikler vardı; çizime geri kondu.")
    );
    let doc = app.document.as_ref().expect("open");
    let back = doc
        .model
        .get(doc.model.slot_of(uid).expect("there"))
        .expect("an object");
    let server = server_drawing();
    match (back, server.get(Slot(1)).expect("an object")) {
        (Entity::Point(back), Entity::Point(server)) => assert_eq!(back.p.x, server.p.x + 7.0),
        other => panic!("{other:?}"),
    }
    assert!(doc.dirty());
    let live = app.cloud.live.as_ref().expect("live");
    assert_eq!(live.sync.pending(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_command_on_its_way_is_in_the_draft_before_it_leaves() {
    let dir = scratch("bulut-yoldaki");
    let mut app = signed_in();
    drafts(&mut app, dir.clone());
    database(&mut app);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let task = app.run("file.save");
    let live = app.cloud.live.as_ref().expect("live");
    assert!(live.sending(), "waiting for its draft");
    let key = live.sent.clone().expect("planned").idempotency_key;
    // Only the draft is written here: the command itself would reach the network.
    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        panic!("a task");
    };
    use iced::futures::StreamExt as _;
    let written = loop {
        match iced::futures::executor::block_on(stream.next()) {
            Some(iced_runtime::Action::Output(Message::Cloud(e)))
                if matches!(*e, Event::DraftWritten { .. }) =>
            {
                break *e;
            }
            Some(_) => {}
            None => panic!("no draft was written"),
        }
    };
    let Event::DraftWritten { send, result, .. } = &written else {
        unreachable!()
    };
    assert!(*send && result.is_ok(), "{written:?}");
    let files: Vec<PathBuf> = walk(&dir);
    assert_eq!(files.len(), 1, "{files:?}");
    let text = std::fs::read_to_string(&files[0]).expect("reads");
    assert!(text.contains(&key), "the command on its way, with its key");
    assert!(!text.contains(SECRET) && !text.contains("kentos_session"));
    let _ = std::fs::remove_dir_all(dir);
}

pub(super) fn walk(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }
    out
}

#[test]
fn a_draft_that_cannot_be_written_asks_before_anything_is_lost() {
    let dir = scratch("bulut-yazilamaz");
    // A file where the draft folder should be: nothing can be written under it.
    let blocked = dir.join("dosya");
    std::fs::write(&blocked, b"x").expect("writes");
    let mut app = signed_in();
    drafts(&mut app, blocked);
    database(&mut app);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let task = app.run("cloud.signOut");
    drive(&mut app, task);
    assert_eq!(app.dialog, Some(Dialog::Unsaved(Then::SignOut)));
    assert!(app.cloud.me.is_some(), "not signed out before the answer");
    let q = app.unsaved_question(Then::SignOut);
    assert_eq!(
        q.message,
        "“Ada 101” projesinde buluta gönderilmemiş 1 değişiklik var."
    );
    assert!(
        q.detail.contains("Bu cihaza da yedeklenemediler"),
        "{}",
        q.detail
    );
    assert_eq!(q.confirm, "Göndermeden oturumu kapat");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_viewer_is_told_once_and_leaving_asks() {
    let dir = scratch("bulut-izleyici");
    let mut app = signed_in();
    drafts(&mut app, dir.clone());
    open(
        &mut app,
        info(ProjectStorage::Database, false, ProjectState::Active),
    );
    assert_eq!(state(&app), SaveState::ReadOnly);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let told = said(&app)
        .iter()
        .filter(|t| t.starts_with("Bu projeyi yalnız görüntüleyebilirsiniz"))
        .count();
    assert_eq!(told, 1);
    let _ = app.run("file.open");
    assert_eq!(app.dialog, Some(Dialog::Unsaved(Then::Open)));
    let q = app.unsaved_question(Then::Open);
    assert!(
        q.detail.contains("yalnız görüntüleme yetkiniz var"),
        "{}",
        q.detail
    );
    assert_eq!(q.confirm, "Göndermeden aç");
    let _ = std::fs::remove_dir_all(dir);
}

// ── A file project ──────────────────────────────────────────────────────

pub(crate) fn file_project(app: &mut App) {
    open(app, info(ProjectStorage::File, true, ProjectState::Active));
}

pub(super) fn save_file(app: &mut App) -> u64 {
    let _ = app.run("file.save");
    let id = app.saving.as_ref().expect("saving").id;
    let _ = app.update(Message::Saving(crate::saving::Event::Encoded {
        id,
        result: Ok(Once::new(vec![1, 2, 3])),
    }));
    id
}

#[test]
fn a_file_project_saves_its_next_revision() {
    let mut app = signed_in();
    file_project(&mut app);
    assert!(app.cloud.live.is_none(), "no autosave for a file project");
    assert_eq!(cell_text(&app), "Buluta kaydedildi · r3");
    edit(&mut app, 1.0);
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::default()));
    assert_eq!(cell_text(&app), "Kaydedilmedi · r3 üstüne");
    let id = save_file(&mut app);
    let s = app.saving.as_ref().expect("saving");
    assert!(s.stage.starts_with("Yükleniyor %"), "{}", s.stage);
    assert!(
        matches!(&s.target, crate::saving::Target::Cloud { based_on: 3, .. }),
        "{:?}",
        s.target
    );
    cloud(
        &mut app,
        Event::FileSaved {
            id,
            result: Ok(FileCommitted {
                revision: "4".into(),
                size: 3,
                sha256: "cd".repeat(32),
                objects: Some("13".into()),
                replayed: false,
            }),
        },
    );
    assert!(app.saving.is_none());
    let doc = app.document.as_ref().expect("open");
    assert!(!doc.dirty());
    assert_eq!(
        doc.cloud_source()
            .and_then(|s| s.revision.as_ref())
            .map(|r| r.number),
        Some(4)
    );
    assert_eq!(last_said(&app), "“Ada 101” buluta kaydedildi: revizyon 4.");
    assert_eq!(cell_text(&app), "Buluta kaydedildi · r4");
}

// ── Buluta yükle ────────────────────────────────────────────────────────

#[test]
fn the_upload_offers_only_workspaces_where_projects_can_be_opened() {
    let mut app = signed_in();
    if let Some(me) = app.cloud.me.as_mut() {
        me.memberships[0].capabilities.clear();
    }
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.upload");
    assert_eq!(app.dialog, Some(Dialog::Upload));
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.tenants.len(), 1);
    assert_eq!(u.tenants[0].tenant_id, PERSONAL);
    assert_eq!(u.storage, ProjectStorage::Database, "the web's default");

    cloud(&mut app, Event::UploadName("  ".into()));
    cloud(&mut app, Event::UploadSubmit);
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(
        u.error.as_deref(),
        Some("Proje adı boş olamaz ve en çok 200 karakter olabilir.")
    );
    assert!(!u.working());
}

/// The catalog's fields go with the new project (the web's `catalogFields`,
/// parity-audit Y1): a change to them is a new upload, with a new key.
#[test]
fn the_upload_takes_the_catalog_s_type_description_and_tags() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.upload");
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(
        u.project_type,
        kentos_commands::ProjectType::Cad,
        "the web's default"
    );
    let key = |app: &App| {
        app.cloud
            .upload
            .as_ref()
            .map(|u| format!("{:?}", u.key_for_tests()))
    };
    let before = key(&app);
    cloud(
        &mut app,
        Event::UploadType(kentos_commands::ProjectType::Subdivision),
    );
    assert_ne!(key(&app), before, "a new upload");
    let before = key(&app);
    cloud(&mut app, Event::UploadTags("Kadıköy, 2026".into()));
    assert_ne!(key(&app), before);
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.project_type, kentos_commands::ProjectType::Subdivision);
    assert_eq!(
        crate::cloud::forms_plan::parse_tags(&u.tags),
        ["Kadıköy", "2026"]
    );
}

#[test]
fn a_refused_upload_names_the_object_and_a_good_one_opens_the_project() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.upload");
    cloud(&mut app, Event::UploadStorage(ProjectStorage::File));
    cloud(&mut app, Event::UploadName("Pafta 12".into()));
    let _ = app.update(crate::cloud::msg(Event::UploadSubmit));
    let (id, key) = app
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
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.stage.as_deref(), Some("Proje oluşturuluyor…"));
    // How far the file is, then what the server does with it (the web's words).
    cloud(
        &mut app,
        Event::UploadProgress {
            id,
            done: 4 * 1024 * 1024,
            total: 10 * 1024 * 1024,
        },
    );
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.stage.as_deref(), Some("Yükleniyor: %40 (4 MB / 10 MB)"));
    assert_eq!(u.fraction, Some(0.4));
    cloud(
        &mut app,
        Event::UploadProgress {
            id,
            done: 10 * 1024 * 1024,
            total: 10 * 1024 * 1024,
        },
    );
    let u = app.cloud.upload.as_ref().expect("the window");
    assert_eq!(u.stage.as_deref(), Some("Sunucu dosyayı doğruluyor…"));
    let mut refused = ApiFailure::new(
        422,
        "invalid",
        "Dosyanın 1. nesnesi içe aktarılamadı: koordinat sınırın dışında.",
    );
    refused = refused.with_path("entities[0]");
    cloud(
        &mut app,
        Event::Uploaded {
            id,
            result: Err(refused),
        },
    );
    let u = app.cloud.upload.as_ref().expect("the window");
    let error = u.error.clone().unwrap_or_default();
    assert!(
        error.contains("Reddedilen nesne: 1. nesne: Nokta · Çizim · P1; çizimde seçildi."),
        "{error}"
    );
    assert_eq!(app.selection.len(), 1, "selected in the drawing");
    assert!(!u.working());

    // Refused for good, the empty project went to the trash: the next try is
    // a new upload, which a trashed project must not answer.
    cloud(&mut app, Event::UploadSubmit);
    let (id, again) = app
        .cloud
        .upload
        .as_ref()
        .and_then(|u| u.request())
        .expect("working");
    assert_ne!(again, key, "a new key after a refusal for good");
    // No answer: the project stays, and the same key finds it again.
    cloud(
        &mut app,
        Event::UploadEncoded {
            id,
            result: Ok(Once::new(vec![1, 2, 3])),
        },
    );
    cloud(
        &mut app,
        Event::Uploaded {
            id,
            result: Err(ApiFailure::new(0, "network", "Sunucuya ulaşılamadı.")),
        },
    );
    cloud(&mut app, Event::UploadSubmit);
    let (id, third) = app
        .cloud
        .upload
        .as_ref()
        .and_then(|u| u.request())
        .expect("working");
    assert_eq!(third, again, "the same key after no answer");
    cloud(
        &mut app,
        Event::UploadEncoded {
            id,
            result: Ok(Once::new(vec![1, 2, 3])),
        },
    );
    let mut made = info(ProjectStorage::File, true, ProjectState::Active);
    made.name = "Pafta 12".into();
    cloud(
        &mut app,
        Event::Uploaded {
            id,
            result: Ok((
                made,
                kentos_cloud::Uploaded::File(FileCommitted {
                    revision: "1".into(),
                    size: 3,
                    sha256: "ef".repeat(32),
                    objects: Some("13".into()),
                    replayed: false,
                }),
            )),
        },
    );
    assert_eq!(app.dialog, None);
    assert!(app.cloud.upload.is_none());
    assert!(said(&app).iter().any(|t| t.starts_with(
        "“Pafta 12” buluta yüklendi (Harita Bürosu, Dosya (KCAD revizyonları)): 13 nesne."
    )));
    assert_eq!(
        app.cloud.opening.as_ref().map(|o| o.name.as_str()),
        Some("Pafta 12")
    );
}

/// The drawing changed while it went up (the web's f7616e4): the project
/// holds it as it went up, the window closes, nothing opens over the change.
#[test]
fn a_drawing_changed_while_it_went_up_stays_as_it_is() {
    let mut app = signed_in();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(
        crate::files_testing::drawing(0),
    )))));
    let _ = app.run("cloud.upload");
    cloud(&mut app, Event::UploadStorage(ProjectStorage::Database));
    cloud(&mut app, Event::UploadName("Pafta 12".into()));
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
    let doc = app.document.as_mut().expect("open");
    doc.model.remove(&[Slot(4)]);
    let revision = doc.model.revision();
    let mut made = info(ProjectStorage::Database, true, ProjectState::Active);
    made.name = "Pafta 12".into();
    cloud(
        &mut app,
        Event::Uploaded {
            id,
            result: Ok((
                made,
                kentos_cloud::Uploaded::Database(kentos_commands::ProjectImported {
                    objects: "13".into(),
                    data_revision: "1".into(),
                    meta_version: "1".into(),
                    replayed: false,
                }),
            )),
        },
    );
    assert_eq!(app.dialog, None);
    assert!(app.cloud.upload.is_none());
    assert!(app.cloud.opening.is_none(), "nothing opens over the change");
    let doc = app.document.as_ref().expect("open");
    assert!(doc.cloud_source().is_none() && doc.model.revision() == revision);
    assert_eq!(
        last_said(&app),
        "“Pafta 12” bulut projesi oluşturuldu ve çizim içe aktarıldı (13 nesne), ama çizim yükleme sürerken değişti; ekrandaki çizim projeye bağlanmadı ve değişiklikleri yerinde duruyor. Projeyi Bulut projesi aç ile açın."
    );
}

/// An editor of a database project (objects, not the tree; the web's
/// b19ed6f): Yeni katman and Yeni grup are off with the reason; run anyway,
/// and the row's Yanına yeni katman, Yeniden adlandır and Sil, say it and
/// change nothing; the eye stays the user's own. A file project may.
#[test]
fn an_editor_may_not_change_the_layer_tree_of_a_database_project() {
    use crate::layering::{Event as LayerEvent, TREE_LOCKED};
    let mut app = signed_in();
    let mut project = info(ProjectStorage::Database, true, ProjectState::Active);
    project.access.permissions = vec![ProjectPermission::Read, ProjectPermission::FeatureWrite];
    open(&mut app, project);
    assert!(!app.available("layer.new") && !app.available("layer.newGroup"));
    assert_eq!(app.why_disabled("layer.new").as_deref(), Some(TREE_LOCKED));
    let nodes = |app: &App| {
        let doc = app.document.as_ref().expect("open");
        (
            doc.model.layers().leaves().len(),
            doc.model.layers().nodes().len(),
        )
    };
    let before = nodes(&app);
    let _ = app.run("layer.new");
    assert_eq!(last_said(&app), TREE_LOCKED);
    let _ = app.run("layer.newGroup");
    assert_eq!(last_said(&app), TREE_LOCKED);
    for event in [
        LayerEvent::AddBeside("cizim".into()),
        LayerEvent::Rename("cizim".into()),
        LayerEvent::Remove("cizim".into()),
    ] {
        let _ = app.update(Message::Layer(event));
        assert_eq!(last_said(&app), TREE_LOCKED);
    }
    assert_eq!(nodes(&app), before);
    assert!(app.renaming.is_none() && app.dialog.is_none());
    let visible = |app: &App| {
        let doc = app.document.as_ref().expect("open");
        doc.model.layers().get("cizim").is_some_and(|l| l.visible)
    };
    let was = visible(&app);
    let _ = app.update(Message::LayerVisible("cizim".into()));
    assert_ne!(visible(&app), was, "the eye still works");

    let mut app = signed_in();
    let mut project = info(ProjectStorage::File, true, ProjectState::Active);
    project.access.permissions = vec![ProjectPermission::Read, ProjectPermission::FeatureWrite];
    open(&mut app, project);
    assert!(
        app.available("layer.new"),
        "a file project saves its tree with the file"
    );
}

#[test]
fn the_status_bar_offers_sign_in_or_says_who_and_where() {
    let (mut app, _) = App::boot(None);
    assert!(app.cloud_available("cloud.signIn") && !app.cloud_available("cloud.signOut"));
    let mut app2 = signed_in();
    database(&mut app2);
    assert!(app2.cloud_available("cloud.signOut") && !app2.cloud_available("cloud.signIn"));
    // Signing out with nothing unsent: at once, the drawing stays as a local one.
    let task = app2.run("cloud.signOut");
    drive(&mut app2, task);
    assert!(app2.cloud.me.is_none());
    assert!(
        app2.document
            .as_ref()
            .is_some_and(|d| d.cloud_source().is_none())
    );
    let _ = app.run("cloud.conflicts");
    assert_eq!(last_said(&app), "Çözülecek bir çakışma yok.");
}
