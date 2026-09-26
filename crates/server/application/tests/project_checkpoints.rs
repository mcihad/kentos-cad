//! Checkpoints (docs/adr/0034): a database project's checkpoint keeps the
//! moment it was made, a file project's names a revision, retries are
//! answered once, only the maker or a manager removes one, the history and
//! download permissions apply, and the store's cleanup removes objects of no
//! checkpoint. Real database (`KENTOS_TEST_DB=required` makes a missing one a
//! failure).

mod common;

use std::time::Duration;

use common::{a_point, blobs, catalog_envelope, envelope, member, new_project, open};
use kentos_application::access::ProjectAccess;
use kentos_application::blobs::Blobs;
use kentos_application::commands::{CatalogPolicy, CommandOutcome, run};
use kentos_application::{AppError, admin, changes, checkpoints, files, projects};
use kentos_contracts::{
    CheckpointChange, CheckpointKind, CommandEnvelope, DocumentSnapshotV1, FileUploadBegin,
    GrantRole, PROJECT_ARCHIVE, PROJECT_CHECKPOINT_CREATE, PROJECT_CHECKPOINT_DELETE,
    PROJECT_FILE_COMMIT, ProjectCreate, ProjectStorage, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MINIMAL: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/minimal.kcad");
const DRAWING: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/drawing.kcad");
const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

async fn command(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    envelope: CommandEnvelope,
) -> Result<CheckpointChange, AppError> {
    match run(&db.app, store, &CatalogPolicy::default(), by, envelope).await? {
        CommandOutcome::Checkpoint(c) => Ok(c),
        other => panic!("not a checkpoint: {other:?}"),
    }
}

async fn make(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    input: serde_json::Value,
) -> Result<CheckpointChange, AppError> {
    command(
        db,
        store,
        by,
        catalog_envelope(by, PROJECT_CHECKPOINT_CREATE, input, &[]),
    )
    .await
}

async fn remove(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    id: &str,
) -> Result<CheckpointChange, AppError> {
    let input = json!({ "checkpointId": id });
    command(
        db,
        store,
        by,
        catalog_envelope(by, PROJECT_CHECKPOINT_DELETE, input, &[]),
    )
    .await
}

async fn read_all(file: tokio::fs::File) -> Vec<u8> {
    let mut file = file;
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
        .await
        .unwrap();
    bytes
}

/// Every file under a folder.
fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            found.extend(walk(&p));
        } else {
            found.push(p);
        }
    }
    found
}

#[tokio::test]
async fn a_database_checkpoint_keeps_the_moment_it_was_made() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let komsu = member(&db, "buro", "komsu", TenantRole::Editor).await;
    let dilek = member(&db, "buro", "dilek", TenantRole::Viewer).await;
    let project = new_project(&db, &ayse, "Ada 101").await;
    let by = open(&db, &ayse, project).await;
    common::share(&db, &by, &komsu.actor, GrantRole::Editor).await;
    common::share(&db, &by, &dilek.actor, GrantRole::Viewer).await;
    let store = blobs();
    changes::commit(
        &db.app,
        &by,
        envelope(&ayse, project, a_point(486500.0), &[]),
    )
    .await
    .unwrap();

    let first = catalog_envelope(
        &by,
        PROJECT_CHECKPOINT_CREATE,
        json!({ "name": "  Belediyeye teslim ", "note": "Ada 101\nifraz" }),
        &[],
    );
    let made = command(&db, &store, &by, first.clone()).await.unwrap();
    let info = projects::info(&db.app, &by).await.unwrap();
    let c = made.checkpoint.clone();
    assert_eq!(
        (c.name.as_str(), c.note.as_deref()),
        ("Belediyeye teslim", Some("Ada 101\nifraz"))
    );
    assert_eq!(
        (c.kind, c.revision.as_str(), c.objects.as_deref()),
        (
            CheckpointKind::Snapshot,
            info.data_revision.as_str(),
            Some("1")
        )
    );
    assert_eq!(c.created_by_name, "ayse");
    // The same command again (its answer lost): the stored answer; its second snapshot is not kept.
    let again = command(&db, &store, &by, first).await.unwrap();
    assert_eq!(
        (again.checkpoint.id.as_str(), again.replayed),
        (c.id.as_str(), true)
    );
    assert_eq!(walk(store.root()).len(), 1);

    // The drawing goes on; the checkpoint keeps the moment it was made.
    changes::commit(
        &db.app,
        &by,
        envelope(&ayse, project, a_point(486510.0), &[]),
    )
    .await
    .unwrap();
    let id = Uuid::parse_str(&c.id).unwrap();
    let (got, file) = checkpoints::download(&db.app, &store, &by, id)
        .await
        .unwrap();
    let bytes = read_all(file).await;
    assert_eq!(
        (got.sha256.clone(), sha(&bytes)),
        (c.sha256.clone(), c.sha256.clone())
    );
    assert_eq!(kentos_kcad::decode(&bytes).unwrap().entities.len(), 1);

    // A viewer sees the history and downloads while viewers may download; never makes one.
    let d = open(&db, &dilek, project).await;
    assert_eq!(
        checkpoints::list(&db.app, &d)
            .await
            .unwrap()
            .checkpoints
            .len(),
        1
    );
    assert!(checkpoints::download(&db.app, &store, &d, id).await.is_ok());
    assert!(matches!(
        make(&db, &store, &d, json!({ "name": "Benim" })).await,
        Err(AppError::Forbidden(_))
    ));
    admin::set_tenant_policy(&db.owner, "buro", None, Some(false))
        .await
        .unwrap();
    let d = open(&db, &dilek, project).await;
    assert!(matches!(
        checkpoints::download(&db.app, &store, &d, id).await,
        Err(AppError::Forbidden(_))
    ));

    // Another editor did not make it and does not manage the project: it stays.
    let k = open(&db, &komsu, project).await;
    assert!(matches!(
        remove(&db, &store, &k, &c.id).await,
        Err(AppError::Forbidden(_))
    ));
    // An editor's own checkpoint is theirs to remove.
    let theirs = make(&db, &store, &k, json!({ "name": "Komşunun" }))
        .await
        .unwrap();
    assert!(
        remove(&db, &store, &k, &theirs.checkpoint.id)
            .await
            .unwrap()
            .removed
    );
    // Its maker removes it, and its object goes with it.
    let gone = remove(&db, &store, &by, &c.id).await.unwrap();
    assert!(gone.removed);
    assert!(
        checkpoints::list(&db.app, &by)
            .await
            .unwrap()
            .checkpoints
            .is_empty()
    );
    assert!(walk(store.root()).is_empty());
    assert!(matches!(
        remove(&db, &store, &by, &c.id).await,
        Err(AppError::NotFound(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn names_and_notes_are_checked_and_archived_projects_take_none() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = new_project(&db, &ayse, "Kurallar").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();
    let path = |r: Result<CheckpointChange, AppError>| match r {
        Err(e) => (e.code(), e.path().map(str::to_string)),
        Ok(c) => panic!("expected a refusal, got {c:?}"),
    };
    assert_eq!(
        path(make(&db, &store, &by, json!({ "name": "  " })).await),
        ("invalid", Some("name".into()))
    );
    assert_eq!(
        path(make(&db, &store, &by, json!({ "name": "x".repeat(121) })).await),
        ("invalid", Some("name".into()))
    );
    assert_eq!(
        path(make(&db, &store, &by, json!({ "name": "iki\nsatır" })).await),
        ("invalid", Some("name".into()))
    );
    assert_eq!(
        path(
            make(
                &db,
                &store,
                &by,
                json!({ "name": "Not", "note": "x".repeat(2001) })
            )
            .await
        ),
        ("invalid", Some("note".into()))
    );
    assert_eq!(
        path(
            make(
                &db,
                &store,
                &by,
                json!({ "name": "Revizyon", "fileRevision": "1" })
            )
            .await
        ),
        ("invalid", Some("fileRevision".into()))
    );
    // Nothing of those was kept.
    assert!(walk(store.root()).is_empty());
    // An archived project changes nothing, its history neither.
    let archive = catalog_envelope(&by, PROJECT_ARCHIVE, json!({}), &[]);
    run(&db.app, &store, &CatalogPolicy::default(), &by, archive)
        .await
        .unwrap();
    let by = open(&db, &ayse, project).await;
    assert!(matches!(
        make(&db, &store, &by, json!({ "name": "Arşivde" })).await,
        Err(AppError::Archived(_))
    ));
    db.close().await;
}

async fn file_project(db: &TestDb, who: &kentos_application::tenancy::Access, name: &str) -> Uuid {
    let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    let input = ProjectCreate {
        name: name.into(),
        settings: s.settings,
        origin: s.origin,
        home_view: s.home_view,
        layers: s.layers,
        active_layer: s.active_layer,
        styles: s.styles,
        description: None,
        project_type: None,
        tags: None,
        storage: Some(ProjectStorage::File),
    };
    Uuid::parse_str(
        &projects::create(&db.app, who, input, None)
            .await
            .unwrap()
            .id,
    )
    .unwrap()
}

/// Uploads `bytes` and commits them on `base`.
async fn save(db: &TestDb, store: &Blobs, by: &ProjectAccess, bytes: &[u8], base: &str) {
    let begun = files::begin(
        &db.app,
        by,
        FileUploadBegin {
            size: bytes.len() as u32,
            sha256: sha(bytes),
        },
    )
    .await
    .unwrap();
    let id = Uuid::parse_str(&begun.id).unwrap();
    let mut receiving = files::start_receive(&db.app, store, by, id).await.unwrap();
    receiving.writer.write(bytes).await.unwrap();
    files::finish_receive(&db.app, store, by, receiving)
        .await
        .unwrap();
    let commit = CommandEnvelope {
        command_name: PROJECT_FILE_COMMIT.into(),
        version: 1,
        tenant_id: by.tenant.to_string(),
        project_id: by.project.to_string(),
        request_id: format!("istek-{}", Uuid::new_v4()),
        idempotency_key: Uuid::new_v4().to_string(),
        expected_versions: [("@file".to_string(), base.to_string())]
            .into_iter()
            .collect(),
        input: json!({ "uploadId": id.to_string() }),
    };
    run(&db.app, store, &CatalogPolicy::default(), by, commit)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_file_checkpoint_names_a_revision() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = file_project(&db, &ayse, "Ada 12 dosyası").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();
    // Nothing saved yet: nothing to name.
    match make(&db, &store, &by, json!({ "name": "Erken" })).await {
        Err(e) => assert_eq!(e.path(), Some("fileRevision")),
        Ok(c) => panic!("named nothing: {c:?}"),
    }
    save(&db, &store, &by, MINIMAL, "0").await;
    save(&db, &store, &by, DRAWING, "1").await;
    let files_before = walk(store.root()).len();

    // The newest when none is given; an earlier one when asked; nothing is copied.
    let newest = make(&db, &store, &by, json!({ "name": "Son hâl" }))
        .await
        .unwrap()
        .checkpoint;
    assert_eq!(
        (newest.kind, newest.revision.as_str(), newest.sha256.clone()),
        (CheckpointKind::Revision, "2", sha(DRAWING))
    );
    assert_eq!(
        newest.objects,
        Some(
            kentos_kcad::decode(DRAWING)
                .unwrap()
                .entities
                .len()
                .to_string()
        )
    );
    let first = make(
        &db,
        &store,
        &by,
        json!({ "name": "İlk", "fileRevision": "1" }),
    )
    .await
    .unwrap()
    .checkpoint;
    assert_eq!(first.revision, "1");
    assert_eq!(walk(store.root()).len(), files_before);
    match make(
        &db,
        &store,
        &by,
        json!({ "name": "Yok", "fileRevision": "9" }),
    )
    .await
    {
        Err(e) => assert_eq!(e.path(), Some("fileRevision")),
        Ok(c) => panic!("named a revision that does not exist: {c:?}"),
    }
    let (_, file) =
        checkpoints::download(&db.app, &store, &by, Uuid::parse_str(&first.id).unwrap())
            .await
            .unwrap();
    assert_eq!(read_all(file).await, MINIMAL);
    // Removing it keeps the revision it named.
    remove(&db, &store, &by, &first.id).await.unwrap();
    let (_, file) = files::download(&db.app, &store, &by, 1).await.unwrap();
    assert_eq!(read_all(file).await, MINIMAL);
    assert_eq!(
        checkpoints::list(&db.app, &by)
            .await
            .unwrap()
            .checkpoints
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["Son hâl"]
    );
    db.close().await;
}

#[tokio::test]
async fn the_cleanup_removes_checkpoint_objects_of_no_checkpoint() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = new_project(&db, &ayse, "Temizlik").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();
    let kept = make(&db, &store, &by, json!({ "name": "Kalan" }))
        .await
        .unwrap()
        .checkpoint;
    // As a checkpoint whose row was never committed leaves its object.
    let stray = Blobs::checkpoint_key(by.tenant, project, Uuid::now_v7(), &sha(MINIMAL));
    let mut w = store.create(&stray, MINIMAL.len() as u64).await.unwrap();
    w.write(MINIMAL).await.unwrap();
    w.finish().await.unwrap();

    // Not while it may still be committed.
    let done = files::cleanup(&db.app, &store, files::SETTLED)
        .await
        .unwrap();
    assert_eq!(done.checkpoints, 0);
    let done = files::cleanup(&db.app, &store, Duration::ZERO)
        .await
        .unwrap();
    assert_eq!((done.checkpoints, done.purged), (1, 0));
    assert!(store.read(&stray).await.is_err());
    let id = Uuid::parse_str(&kept.id).unwrap();
    assert!(
        checkpoints::download(&db.app, &store, &by, id)
            .await
            .is_ok()
    );

    // A project removed for good takes its checkpoints' objects.
    sqlx::query("select kentos.remove_project($1, $2)")
        .bind(by.tenant)
        .bind(project)
        .execute(&db.owner)
        .await
        .unwrap();
    let done = files::cleanup(&db.app, &store, Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(done.purged, 1);
    assert!(walk(store.root()).is_empty());
    db.close().await;
}
