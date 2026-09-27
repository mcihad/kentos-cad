//! The catalog's Geçmiş tab message by message, without a server
//! (docs/adr/0087): the history asked when the tab shows a project and
//! followed by its events, the checkpoint form, the restore form that opens
//! the new project, the removal's question, and the downloads.

use std::time::{Duration, Instant};

use kentos_cloud::ApiFailure;
use kentos_contracts::{
    CatalogView, Checkpoint, CheckpointChange, CheckpointKind, EventPage, EventRecord,
    FileRevision, FileRevisions, ProjectDuplicated, ProjectPermission, ProjectStorage,
};

use crate::app::App;
use crate::cloud::Event;
use crate::cloud::catalog::{List, Tab};
use crate::cloud::catalog_actions::Fetch;
use crate::cloud::catalog_history::{Form, HistoryActed, HistoryState, Point};
use crate::cloud::catalog_tests::{SECOND, catalog, listed, owned};
use crate::cloud::history::HistoryData;
use crate::cloud::tests::{PROJECT, TENANT, cloud, last_said, page, signed_in};

fn checkpoint(id: &str, name: &str, by: &str) -> Checkpoint {
    Checkpoint {
        id: id.into(),
        name: name.into(),
        note: Some("İmar müdürlüğüne gönderilen sürüm.".into()),
        kind: CheckpointKind::Snapshot,
        revision: "1".into(),
        size: "1638".into(),
        sha256: "a".repeat(64),
        objects: Some("6".into()),
        created_by: by.into(),
        created_by_name: "Ayşe Yılmaz".into(),
        created_at: "2026-09-27T09:54:00Z".into(),
    }
}

fn revision(n: &str) -> FileRevision {
    FileRevision {
        revision: n.into(),
        size: 1700,
        sha256: "b".repeat(64),
        created_by: "u2".into(),
        created_by_name: "Mehmet Demir".into(),
        created_at: "2026-09-27T09:56:00Z".into(),
        objects: Some("7".into()),
    }
}

/// The catalog on “Projelerim” with `p` selected and its Geçmiş tab shown.
fn history_of(storage: ProjectStorage) -> App {
    let mut app = signed_in();
    let mut p = owned(PROJECT, "Ada 1246 ölçü projesi");
    p.storage = storage;
    listed(&mut app, CatalogView::Mine, vec![p, owned(SECOND, "Başka")]);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    cloud(&mut app, Event::CatalogTab(Tab::History));
    app
}

fn asked(app: &App) -> u64 {
    catalog(app)
        .history
        .request()
        .expect("the history is asked")
}

fn answer(app: &mut App, data: HistoryData) {
    let id = asked(app);
    cloud(
        app,
        Event::HistoryLoaded {
            id,
            result: Ok(data),
        },
    );
}

fn database_history(checkpoints: Vec<Checkpoint>) -> HistoryData {
    HistoryData {
        storage: ProjectStorage::Database,
        revisions: None,
        checkpoints: Some(checkpoints),
    }
}

fn acting(app: &App) -> u64 {
    catalog(app).history.acting().expect("a request on its way")
}

#[test]
fn the_tab_asks_the_history_follows_its_events_and_lets_go_of_it() {
    let mut app = history_of(ProjectStorage::Database);
    assert_eq!(catalog(&app).history.state, HistoryState::Loading);
    let first = asked(&app);
    // Its events are followed: the cursor first.
    let watch = catalog(&app)
        .history
        .watching()
        .expect("its events followed");
    // Another project: the late answer is dropped.
    cloud(&mut app, Event::CatalogPick(SECOND.into()));
    cloud(
        &mut app,
        Event::HistoryLoaded {
            id: first,
            result: Ok(database_history(vec![checkpoint("c1", "Eski", "u1")])),
        },
    );
    assert_eq!(catalog(&app).history.state, HistoryState::Loading);
    cloud(&mut app, Event::CatalogPick(PROJECT.into()));
    answer(
        &mut app,
        database_history(vec![checkpoint("c1", "Belediyeye teslim", "u1")]),
    );
    assert!(matches!(
        catalog(&app).history.state,
        HistoryState::Loaded(_)
    ));
    // A checkpoint event: asked again a moment later, the list staying meanwhile.
    let watch_now = catalog(&app).history.watching().expect("followed");
    assert_ne!(watch, watch_now, "a new wait for the new project");
    let event = |kind: &str| EventRecord {
        seq: "12".into(),
        data_revision: "1".into(),
        kind: kind.into(),
        actor: None,
        request_id: None,
        features: Vec::new(),
        meta: false,
    };
    cloud(
        &mut app,
        Event::HistoryEvents {
            id: watch_now,
            result: Ok(EventPage {
                events: vec![event("project.changes")],
                next: "12".into(),
            }),
        },
    );
    assert!(
        catalog(&app).history.again_at.is_none(),
        "an edit is not history"
    );
    let watch_next = catalog(&app).history.watching().expect("waits again");
    cloud(
        &mut app,
        Event::HistoryEvents {
            id: watch_next,
            result: Ok(EventPage {
                events: vec![event("project.checkpoint")],
                next: "13".into(),
            }),
        },
    );
    assert!(catalog(&app).history.again_at.is_some());
    let _ = app.cloud_tick(Instant::now() + Duration::from_millis(300));
    assert!(catalog(&app).history.request().is_some(), "asked again");
    assert!(
        matches!(catalog(&app).history.state, HistoryState::Loaded(_)),
        "quietly: the list stays until the answer"
    );
    // Bilgiler: the tab lets go of it.
    cloud(&mut app, Event::CatalogTab(Tab::Info));
    assert_eq!(catalog(&app).history.state, HistoryState::None);
    assert!(catalog(&app).history.watching().is_none());
}

#[test]
fn naming_a_checkpoint_asks_a_name_and_says_what_it_made() {
    let mut app = history_of(ProjectStorage::File);
    answer(
        &mut app,
        HistoryData {
            storage: ProjectStorage::File,
            revisions: Some(FileRevisions {
                current: Some("5".into()),
                revisions: vec![revision("5"), revision("4")],
            }),
            checkpoints: Some(Vec::new()),
        },
    );
    cloud(&mut app, Event::HistoryCreate);
    let Some(Form::Checkpoint(f)) = &catalog(&app).history.form else {
        panic!("the form");
    };
    assert_eq!(
        (f.revisions.len(), f.revision),
        (2, 0),
        "the newest named by default"
    );
    // No name, no request.
    cloud(&mut app, Event::HistoryName("   ".into()));
    cloud(&mut app, Event::HistorySubmit);
    assert!(catalog(&app).history.acting().is_none());
    cloud(&mut app, Event::HistoryName("  Teslim öncesi ".into()));
    cloud(&mut app, Event::HistoryRevision(1));
    cloud(&mut app, Event::HistorySubmit);
    let id = acting(&app);
    let Some(Form::Checkpoint(f)) = &catalog(&app).history.form else {
        panic!("the form waits");
    };
    assert!(f.busy);
    assert_eq!(
        f.status.as_ref().map(|s| s.text.as_str()),
        Some("Oluşturuluyor…")
    );
    // Refused: said in the form, which can be sent again.
    cloud(
        &mut app,
        Event::HistoryActed {
            id,
            result: Err(ApiFailure::new(
                403,
                "forbidden",
                "Bu işlem için yetkiniz yok.",
            )),
        },
    );
    let Some(Form::Checkpoint(f)) = &catalog(&app).history.form else {
        panic!("the form stays");
    };
    assert!(!f.busy && f.status.as_ref().is_some_and(|s| s.error));
    cloud(&mut app, Event::HistorySubmit);
    let id = acting(&app);
    let mut made = checkpoint("c9", "Teslim öncesi", "u1");
    made.kind = CheckpointKind::Revision;
    made.revision = "4".into();
    cloud(
        &mut app,
        Event::HistoryActed {
            id,
            result: Ok(HistoryActed::Made(Box::new(CheckpointChange {
                checkpoint: made,
                removed: false,
                replayed: false,
            }))),
        },
    );
    assert!(catalog(&app).history.form.is_none());
    assert_eq!(
        last_said(&app),
        "“Teslim öncesi” kontrol noktası oluşturuldu: revizyon 4."
    );
    assert!(catalog(&app).history.request().is_some(), "the list again");
}

#[test]
fn a_restored_point_is_a_new_project_opened_from_projelerim() {
    let mut app = history_of(ProjectStorage::Database);
    let cp = checkpoint(
        "0199aaaa-0000-7000-8000-0000000000c1",
        "Belediyeye teslim",
        "u1",
    );
    answer(&mut app, database_history(vec![cp.clone()]));
    cloud(
        &mut app,
        Event::HistoryRestore(Point::Checkpoint(cp.clone())),
    );
    let Some(Form::Restore(f)) = &catalog(&app).history.form else {
        panic!("the form");
    };
    // The project's workspace first; the personal space can take it too.
    assert_eq!(f.places.first().map(|(id, _)| id.as_str()), Some(TENANT));
    assert_eq!(f.places.len(), 2);
    cloud(&mut app, Event::HistorySubmit);
    let id = acting(&app);
    const MADE: &str = "0199aaaa-0000-7000-8000-00000000000d";
    let mut made = owned(MADE, "Ada 1246 ölçü projesi (Belediyeye teslim)");
    made.tenant_id = TENANT.into();
    cloud(
        &mut app,
        Event::HistoryActed {
            id,
            result: Ok(HistoryActed::Restored(Box::new(ProjectDuplicated {
                project: made.clone(),
                source_id: PROJECT.into(),
                objects: "1284".into(),
                replayed: false,
            }))),
        },
    );
    assert_eq!(
        last_said(&app),
        "“Ada 1246 ölçü projesi (Belediyeye teslim)” oluşturuldu: “Belediyeye teslim” kontrol noktası (veri revizyonu 1) geri yüklendi (1.284 nesne)."
    );
    // “Projelerim”, with Bilgiler; once the list shows it, it opens.
    let c = catalog(&app);
    assert_eq!(
        (c.list.clone(), c.tab),
        (List::View(CatalogView::Mine), Tab::Info)
    );
    let page_id = c.request().expect("the list asked");
    cloud(
        &mut app,
        Event::CatalogPage {
            id: page_id,
            more: false,
            result: Ok(page(
                vec![made, owned(PROJECT, "Ada 1246 ölçü projesi")],
                2,
                None,
            )),
        },
    );
    assert_eq!(catalog(&app).picked.as_deref(), Some(MADE));
    assert!(app.cloud.opening.is_some(), "the new project opens");
}

#[test]
fn a_checkpoint_goes_after_its_question_and_a_refusal_names_it() {
    let mut app = history_of(ProjectStorage::Database);
    let cp = checkpoint(
        "0199aaaa-0000-7000-8000-0000000000c1",
        "Belediyeye teslim",
        "u1",
    );
    answer(&mut app, database_history(vec![cp.clone()]));
    cloud(&mut app, Event::HistoryRemove(cp.clone()));
    assert!(catalog(&app).history.removing.is_some());
    // Esc closes the question, not the window.
    app.close_dialog();
    assert!(catalog(&app).history.removing.is_none());
    assert!(app.cloud.catalog.is_some());
    cloud(&mut app, Event::HistoryRemove(cp.clone()));
    cloud(&mut app, Event::HistoryRemoveAnswer(true));
    let id = acting(&app);
    cloud(
        &mut app,
        Event::HistoryActed {
            id,
            result: Err(ApiFailure::new(
                403,
                "forbidden",
                "Kontrol noktasını yalnız onu oluşturan silebilir.",
            )),
        },
    );
    assert_eq!(
        last_said(&app),
        "“Belediyeye teslim” kontrol noktası silinemedi: Kontrol noktasını yalnız onu oluşturan silebilir."
    );
    cloud(&mut app, Event::HistoryRemove(cp.clone()));
    cloud(&mut app, Event::HistoryRemoveAnswer(true));
    let id = acting(&app);
    cloud(
        &mut app,
        Event::HistoryActed {
            id,
            result: Ok(HistoryActed::Removed(Box::new(CheckpointChange {
                checkpoint: cp,
                removed: true,
                replayed: false,
            }))),
        },
    );
    assert_eq!(
        last_said(&app),
        "“Belediyeye teslim” kontrol noktası silindi."
    );
}

#[test]
fn history_downloads_are_named_and_checked_against_the_list() {
    let mut app = history_of(ProjectStorage::File);
    answer(
        &mut app,
        HistoryData {
            storage: ProjectStorage::File,
            revisions: Some(FileRevisions {
                current: Some("5".into()),
                revisions: vec![revision("5")],
            }),
            checkpoints: Some(Vec::new()),
        },
    );
    cloud(&mut app, Event::HistoryDownloadRevision(revision("5")));
    let a = catalog(&app).acting.as_ref().expect("a download");
    let d = a.download.as_ref().expect("what it fetches");
    assert_eq!(
        (&d.fetch, d.name.as_str(), d.listed.as_deref()),
        (
            &Fetch::Revision("5".into()),
            "Ada 1246 ölçü projesi (revizyon 5)",
            Some("b".repeat(64).as_str())
        )
    );
    // The save window closed: nothing happens.
    let id = a.id;
    cloud(&mut app, Event::CatalogDownloadTo { id, path: None });
    assert!(catalog(&app).acting.is_none());
    // Without project.download a revision is not offered.
    app.cloud
        .catalog
        .as_mut()
        .and_then(|c| c.projects.iter_mut().find(|p| p.id == PROJECT))
        .expect("listed")
        .access
        .permissions = vec![ProjectPermission::Read, ProjectPermission::History];
    cloud(&mut app, Event::HistoryDownloadRevision(revision("5")));
    assert!(catalog(&app).acting.is_none());
}

#[test]
fn the_history_command_opens_the_open_projects_history_in_the_catalog() {
    use crate::app::Dialog;
    use crate::cloud::tests::database;

    let mut app = signed_in();
    database(&mut app);
    assert!(
        !app.cloud_available("cloud.history"),
        "without project.history it is off"
    );
    app.document
        .as_mut()
        .and_then(|d| d.cloud_source_mut())
        .expect("a cloud project")
        .info
        .access
        .permissions
        .push(ProjectPermission::History);
    assert!(app.cloud_available("cloud.history"));
    let _ = app.run("cloud.history");
    assert_eq!(app.dialog, Some(Dialog::Catalog));
    let c = catalog(&app);
    assert_eq!(
        (c.list.clone(), c.tab),
        (List::View(CatalogView::Recent), Tab::History)
    );
    let id = c.request().expect("the list asked");
    cloud(
        &mut app,
        Event::CatalogPage {
            id,
            more: false,
            result: Ok(page(vec![owned(PROJECT, "Ada 101")], 1, None)),
        },
    );
    assert_eq!(catalog(&app).picked.as_deref(), Some(PROJECT));
    assert!(
        catalog(&app).history.request().is_some(),
        "its history asked"
    );
    assert!(app.cloud.opening.is_none(), "shown, not opened again");
}
