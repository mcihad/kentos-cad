//! Cloud tests: this device's copy, a project opened and saved without a connection, ended projects,
//! removed copies and the link's words.

use super::*;

// ── Without a connection: the local copy ────────────────────────────────

/// The app signed in, with places for copies and drafts in `dir`.
pub(super) fn keeping(dir: &std::path::Path) -> App {
    let mut app = signed_in();
    app.cloud.replicas = Some(kentos_cloud::ReplicaStore::new(dir.join("kopya")));
    drafts(&mut app, dir.join("taslak"));
    app
}

/// The app after a restart without a session: the last account and the server
/// kept in its settings (the id is not a secret), the same places on disk.
pub(super) fn restarted(dir: &std::path::Path) -> App {
    let (mut app, _) = App::boot(None);
    let _ = app.settings.choose(&[
        (
            "cloud.server",
            serde_json::Value::from("http://127.0.0.1:9"),
        ),
        ("cloud.account", serde_json::Value::from(USER)),
    ]);
    app.cloud.replicas = Some(kentos_cloud::ReplicaStore::new(dir.join("kopya")));
    drafts(&mut app, dir.join("taslak"));
    app
}

#[test]
fn a_project_opens_from_this_device_s_copy_without_a_connection_and_its_work_waits() {
    let dir = scratch("bulut-cevrimdisi");
    let mut app = keeping(&dir);
    database(&mut app);
    assert!(app.cloud.held.is_some(), "the copy is kept and locked");
    let uid = edit(&mut app, 4.0);
    app.cloud_after(Instant::now());
    // The program ends with the edit unsent: its draft is written first.
    let task = app.update(Message::CloseRequested(iced::window::Id::unique()));
    drive(&mut app, task);
    drop(app);

    // Again, without a session: the catalog shows this device's copies.
    let mut app = restarted(&dir);
    let _ = app.run("cloud.open");
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    let c = app.cloud.catalog.as_ref().expect("the window");
    assert_eq!(c.list, List::Device);
    assert_eq!(c.device.len(), 1);
    assert_eq!(c.device[0].info.name, "Ada 101");
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    let task = app.update(crate::cloud::msg(Event::CatalogOpen));
    drive(&mut app, task);
    let doc = app.document.as_ref().expect("opened from the copy");
    assert_eq!(
        doc.cloud_source().map(|s| s.workspace.as_str()),
        Some("Harita Bürosu")
    );
    let back = doc
        .model
        .get(doc.model.slot_of(uid).expect("there"))
        .expect("an object");
    match (back, server_drawing().get(Slot(1)).expect("an object")) {
        (Entity::Point(back), Entity::Point(server)) => assert_eq!(back.p.x, server.p.x + 4.0),
        other => panic!("{other:?}"),
    }
    assert!(
        said(&app)
            .iter()
            .any(|t| t.contains("Çevrimdışı — değişiklikler bu cihazda saklanıyor"))
    );
    // Signed out, the work waits in the draft: said as the web says work waiting offline.
    assert_eq!(cell_text(&app), "Çevrimdışı: 1 bekliyor");
    assert_eq!(
        app.link_state(),
        crate::cloud::cells_plan::LinkState::AuthRequired
    );
    // Nothing goes without a session; the work waits in the draft.
    let _ = app.cloud_tick(Instant::now() + Duration::from_secs(10));
    assert!(!app.cloud.live.as_ref().expect("live").sending());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_second_window_cannot_open_a_project_this_one_holds() {
    let dir = scratch("bulut-kilit");
    let mut app = keeping(&dir);
    database(&mut app);
    let mut other = keeping(&dir);
    other.cloud.open_hint = Some(("Ada 101".into(), ProjectStorage::Database));
    let _ = other.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    assert!(other.cloud.opening.is_none());
    assert!(
        last_said(&other).contains("başka bir KentOS penceresinde açık"),
        "{}",
        last_said(&other)
    );
    drop(app);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_file_project_saved_without_a_connection_waits_in_the_copy_and_goes_later() {
    let dir = scratch("bulut-dosya-bekleyen");
    let mut app = keeping(&dir);
    file_project(&mut app);
    // The session ends: Kaydet keeps the save on this device.
    app.cloud.me = None;
    edit(&mut app, 1.0);
    let _ = app.update(Message::Modifiers(iced::keyboard::Modifiers::default()));
    let _ = save_file(&mut app);
    assert!(app.saving.is_none());
    assert!(
        !app.document.as_ref().expect("open").dirty(),
        "saved on this device"
    );
    assert_eq!(cell_text(&app), "Kaydedildi (bu cihazda) · gönderilecek");
    assert!(app.cloud.held.as_ref().is_some_and(|h| h.kept_save));

    // Signed in again: it goes, based on the revision it was made on.
    app.cloud.me = Some(me());
    app.cloud.link = crate::cloud::copy::Link::Offline;
    let _ = app.came_online();
    assert!(app.cloud.held.as_ref().is_some_and(|h| h.sending.is_some()));
    let session = session(&app);
    cloud(
        &mut app,
        Event::KeptSent {
            session,
            based_on: 3,
            result: Ok(FileCommitted {
                revision: "4".into(),
                size: 3,
                sha256: "cd".repeat(32),
                objects: Some("13".into()),
                replayed: false,
            }),
        },
    );
    assert!(app.cloud.held.as_ref().is_some_and(|h| !h.kept_save));
    assert_eq!(cell_text(&app), "Buluta kaydedildi · r4");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_kept_save_that_meets_a_newer_revision_keeps_both_until_the_choice() {
    let dir = scratch("bulut-dosya-iki");
    let mut app = keeping(&dir);
    file_project(&mut app);
    app.cloud.me = None;
    edit(&mut app, 1.0);
    let _ = save_file(&mut app);
    app.cloud.me = Some(me());
    app.cloud.link = crate::cloud::copy::Link::Offline;
    let _ = app.came_online();
    let session = session(&app);
    let clash =
        ApiFailure::new(409, "conflict", "Başka biri daha önce kaydetti.").with_conflicts(vec![
            FeatureConflict {
                id: "@file".into(),
                reason: ConflictReason::Changed,
                expected: Some("3".into()),
                actual: Some("6".into()),
                current: None,
            },
        ]);
    cloud(
        &mut app,
        Event::KeptSent {
            session,
            based_on: 3,
            result: Err(clash),
        },
    );
    assert_eq!(app.dialog, Some(Dialog::Revision));
    assert_eq!(
        app.cloud.question.as_ref().map(|q| q.id),
        Some(crate::cloud::revisions::QuestionId::Conflict)
    );
    assert!(
        app.cloud.held.as_ref().is_some_and(|h| h.kept_save),
        "never dropped without the choice"
    );
    assert_eq!(cell_text(&app), "Çakışma: r6 kaydedilmiş");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_ended_project_is_marked_in_the_copy_and_says_why_in_the_offline_list() {
    let dir = scratch("bulut-bitmis");
    let mut app = keeping(&dir);
    database(&mut app);
    let mut deleted = event("10", "web-başkası", Vec::new());
    deleted.kind = kentos_commands::PROJECT_DELETED.into();
    events(
        &mut app,
        Ok(EventPage {
            events: vec![deleted],
            next: "10".into(),
        }),
    );
    assert_eq!(state(&app), SaveState::Deleted);
    let store = app.cloud.replicas.clone().expect("a place for copies");
    let kept = store.list("http://127.0.0.1:9", USER);
    assert_eq!(kept.len(), 1);
    let ended = kept[0].ended.expect("marked");
    assert_eq!(words::ended(ended), "Çöp kutusunda");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn removing_a_copy_asks_about_its_unsent_work_first() {
    let dir = scratch("bulut-kaldir");
    let mut app = keeping(&dir);
    database(&mut app);
    edit(&mut app, 2.0);
    app.cloud_after(Instant::now());
    let task = app.run("cloud.signOut");
    drive(&mut app, task);
    assert!(app.cloud.held.is_none(), "the copy was let go");
    // As a sign-in leaves them: the server and the account (not a secret).
    let _ = app.settings.choose(&[
        (
            "cloud.server",
            serde_json::Value::from("http://127.0.0.1:9"),
        ),
        ("cloud.account", serde_json::Value::from(USER)),
    ]);
    let _ = app.run("cloud.open");
    let c = app.cloud.catalog.as_ref().expect("the window");
    assert_eq!((c.list.clone(), c.device.len()), (List::Device, 1));
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogRemove);
    assert_eq!(app.dialog, Some(Dialog::RemoveCopy));
    assert_eq!(app.cloud.removing.as_ref().map(|r| r.unsent), Some(1));
    // Vazgeç: back to the list, nothing removed.
    app.close_dialog();
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    assert_eq!(app.cloud.catalog.as_ref().map(|c| c.device.len()), Some(1));
    cloud(&mut app, Event::CatalogRemove);
    cloud(&mut app, Event::RemoveConfirmed);
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    assert_eq!(app.cloud.catalog.as_ref().map(|c| c.device.len()), Some(0));
    assert!(walk(&dir.join("taslak")).is_empty(), "its draft went too");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_link_says_live_offline_or_signed_out() {
    use crate::cloud::cells_plan::LinkState;
    let mut app = signed_in();
    database(&mut app);
    assert_eq!(app.link_state(), LinkState::Online);
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    assert_eq!(
        cell_text(&app),
        "Kaydedilecek: 1",
        "what waits, in the save cell"
    );
    app.cloud.link = crate::cloud::copy::Link::Offline;
    assert_eq!(app.link_state(), LinkState::Offline);
    app.cloud.me = None;
    assert_eq!(app.link_state(), LinkState::AuthRequired);
}

/// The connection's return with nothing waiting sends nothing, and the next
/// edit still waits its second (a bug the real-server run found).
#[test]
fn after_the_connection_returns_the_next_edit_still_waits_its_second() {
    let mut app = signed_in();
    database(&mut app);
    app.cloud.link = crate::cloud::copy::Link::Offline;
    let _ = app.came_online();
    assert!(
        said(&app)
            .iter()
            .any(|t| t.starts_with("Sunucuya yeniden ulaşıldı"))
    );
    let t0 = Instant::now();
    edit(&mut app, 1.0);
    app.cloud_after(t0);
    let _ = app.cloud_tick(t0 + Duration::from_millis(500));
    assert_eq!(state(&app), SaveState::Pending, "not before its second");
    let _ = app.cloud_tick(t0 + Duration::from_millis(1100));
    assert_eq!(state(&app), SaveState::Saving);
}

/// An answer goes to the copy's log first, then the draft says what is left
/// (docs/adr/0043): here nothing, so the draft is gone.
#[test]
fn an_answer_is_appended_to_the_copy_before_the_draft_is_written() {
    let dir = scratch("bulut-adim");
    let mut app = keeping(&dir);
    database(&mut app);
    assert_eq!(app.cloud.held.as_ref().map(|h| h.steps), Some(0));
    edit(&mut app, 1.0);
    app.cloud_after(Instant::now());
    let task = app.run("file.save");
    // The draft with the command on its way reaches the disk (the command itself is not sent here).
    let Some(mut stream) = iced_runtime::task::into_stream(task) else {
        panic!("a task");
    };
    use iced::futures::StreamExt as _;
    while let Some(action) = iced::futures::executor::block_on(stream.next()) {
        if let iced_runtime::Action::Output(Message::Cloud(e)) = action
            && matches!(*e, Event::DraftWritten { .. })
        {
            // Written: the command would go now (its request is dropped here).
            let _ = app.update(Message::Cloud(e));
            break;
        }
    }
    assert_eq!(
        walk(&dir.join("taslak")).len(),
        1,
        "the draft holds the command"
    );
    let answer = answer(&app, "2");
    let session = session(&app);
    let task = app.update(crate::cloud::msg(Event::Committed {
        session,
        result: Ok(answer),
    }));
    assert_eq!(
        app.cloud.held.as_ref().map(|h| h.steps),
        Some(1),
        "the step is in the copy before the draft is written"
    );
    drive(&mut app, task);
    assert!(
        walk(&dir.join("taslak")).is_empty(),
        "nothing is left to keep"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// The server does not answer an open: the project opens from this device's copy.
#[test]
fn an_open_the_server_does_not_answer_comes_from_the_copy() {
    let dir = scratch("bulut-yedek-acilis");
    let mut app = keeping(&dir);
    database(&mut app);
    let before = session(&app);
    app.cloud.open_hint = Some(("Ada 101".into(), ProjectStorage::Database));
    let _ = app.start_cloud_open(uuid(TENANT), uuid(PROJECT));
    let id = app.cloud.opening.as_ref().expect("opening").id;
    let task = app.update(crate::cloud::msg(Event::Opened {
        id,
        result: Err(ApiFailure::new(
            0,
            "network",
            "Sunucuya ulaşılamadı; bağlantınızı ve sunucu adresini denetleyin.",
        )),
    }));
    drive(&mut app, task);
    assert_ne!(session(&app), before, "opened again, from the copy");
    assert_eq!(app.cloud.link, crate::cloud::copy::Link::Offline);
    assert!(
        said(&app)
            .iter()
            .any(|t| t.contains("bu cihazdaki kopyasından açıldı"))
    );
    assert!(app.cloud.held.is_some(), "the copy stays locked");
    let _ = std::fs::remove_dir_all(dir);
}
