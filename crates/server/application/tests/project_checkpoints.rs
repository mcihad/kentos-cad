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
use kentos_application::{AppError, admin, changes, checkpoints, files, projects, snapshot};
use kentos_contracts::{
    CheckpointChange, CheckpointKind, CommandEnvelope, DocumentSnapshotV1, EntityId, FeatureChange,
    FileUploadBegin, GrantRole, PROJECT_ARCHIVE, PROJECT_CHECKPOINT_CREATE,
    PROJECT_CHECKPOINT_DELETE, PROJECT_CHECKPOINT_RESTORE, PROJECT_FILE_COMMIT, ProjectChanges,
    ProjectCreate, ProjectDuplicated, ProjectPatch, ProjectStorage, TenantRole,
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

async fn restore(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    envelope: CommandEnvelope,
) -> Result<ProjectDuplicated, AppError> {
    match run(&db.app, store, &CatalogPolicy::default(), by, envelope).await? {
        CommandOutcome::Duplicated(d) => Ok(d),
        other => panic!("not a restore: {other:?}"),
    }
}

/// Every object by its persistent id, without its slot (a file keeps no slots).
fn by_uid(
    doc: &kentos_contracts::DocumentSnapshotV2,
) -> std::collections::BTreeMap<EntityId, serde_json::Value> {
    doc.uids
        .iter()
        .zip(&doc.entities)
        .map(|(uid, e)| {
            let mut v = serde_json::to_value(e).unwrap();
            v.as_object_mut().unwrap().remove("id");
            (*uid, v)
        })
        .collect()
}

fn unlock(layers: &mut [kentos_contracts::LayerNode]) {
    for l in layers {
        l.locked = false;
        unlock(&mut l.children);
    }
}

#[tokio::test]
async fn a_database_checkpoint_is_restored_as_a_new_project() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let store = blobs();
    // The drawing of the snapshot tests (docs/adr/0033), less its object of extreme values.
    let source = kentos_kcad::decode(DRAWING).unwrap();
    let mut unlocked = source.layers.clone();
    unlock(&mut unlocked);
    let input = ProjectCreate {
        name: "Ada 5".into(),
        settings: source.settings.clone(),
        origin: source.origin,
        home_view: source.home_view,
        layers: unlocked,
        active_layer: source.active_layer.clone(),
        styles: source.styles.clone(),
        description: Some("İfraz dosyası".into()),
        project_type: None,
        tags: Some(vec!["ifraz".into()]),
        storage: None,
    };
    let project = Uuid::parse_str(
        &projects::create(&db.app, &ayse, input, None)
            .await
            .unwrap()
            .id,
    )
    .unwrap();
    let by = open(&db, &ayse, project).await;
    let features = source
        .uids
        .iter()
        .zip(&source.entities)
        .enumerate()
        .filter(|(i, _)| *i != 13)
        .map(|(_, (uid, e))| FeatureChange::Create {
            id: uid.to_text(),
            entity: e.clone(),
        })
        .collect();
    let batch = ProjectChanges {
        features,
        project: None,
    };
    changes::commit(&db.app, &by, envelope(&ayse, project, batch, &[]))
        .await
        .unwrap();
    let meta = projects::info(&db.app, &by).await.unwrap().meta_version;
    let lock = ProjectChanges {
        features: vec![],
        project: Some(ProjectPatch {
            layers: Some(source.layers.clone()),
            ..Default::default()
        }),
    };
    changes::commit(
        &db.app,
        &by,
        envelope(&ayse, project, lock, &[("@project", &meta)]),
    )
    .await
    .unwrap();
    let point = make(&db, &store, &by, json!({ "name": "Teslim" }))
        .await
        .unwrap()
        .checkpoint;
    let (_, file) =
        checkpoints::download(&db.app, &store, &by, Uuid::parse_str(&point.id).unwrap())
            .await
            .unwrap();
    let kept = kentos_kcad::decode(&read_all(file).await).unwrap();

    // The drawing goes on after the checkpoint.
    let later: kentos_contracts::Entity = serde_json::from_value(json!({ "kind": "point", "id": 1, "layerId": "parsel", "attrs": {}, "p": { "x": 486520.0, "y": 4420220.0 } })).unwrap();
    let more = ProjectChanges {
        features: vec![FeatureChange::Create {
            id: Uuid::now_v7().to_string(),
            entity: later,
        }],
        project: None,
    };
    changes::commit(&db.app, &by, envelope(&ayse, project, more, &[]))
        .await
        .unwrap();

    let asked = catalog_envelope(
        &by,
        PROJECT_CHECKPOINT_RESTORE,
        json!({ "checkpointId": point.id }),
        &[],
    );
    let restored = restore(&db, &store, &by, asked.clone()).await.unwrap();
    assert_eq!(
        (restored.project.name.as_str(), restored.objects.as_str()),
        ("Ada 5 (Teslim)", "14")
    );
    assert_eq!(restored.source_id, project.to_string());
    let copy = Uuid::parse_str(&restored.project.id).unwrap();
    let c = open(&db, &ayse, copy).await;
    let info = projects::info(&db.app, &c).await.unwrap();
    assert_eq!(
        (info.feature_count.as_str(), info.data_revision.as_str()),
        ("14", "1")
    );
    // What the checkpoint kept, object by object, with the same ids, settings, layers and styles.
    let now = kentos_kcad::decode(&snapshot::snapshot(&db.app, &c).await.unwrap().bytes).unwrap();
    assert_eq!(by_uid(&now), by_uid(&kept));
    assert_eq!(
        (now.settings.clone(), now.layers.clone(), now.styles.clone()),
        (
            kept.settings.clone(),
            kept.layers.clone(),
            kept.styles.clone()
        )
    );
    assert_eq!(
        (now.origin, now.home_view, now.active_layer.clone()),
        (kept.origin, kept.home_view, kept.active_layer.clone())
    );
    // Its catalog metadata came along; its history did not.
    assert_eq!(info.feature_count, "14");
    assert!(
        checkpoints::list(&db.app, &c)
            .await
            .unwrap()
            .checkpoints
            .is_empty()
    );
    // The same command again (its answer lost): the same new project, nothing restored twice.
    let again = restore(&db, &store, &by, asked).await.unwrap();
    assert_eq!(
        (again.project.id.as_str(), again.replayed),
        (restored.project.id.as_str(), true)
    );
    // The source did not change.
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().feature_count,
        "15"
    );
    db.close().await;
}

#[tokio::test]
async fn a_file_point_is_restored_as_a_new_file_project() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let dilek = member(&db, "buro", "dilek", TenantRole::Viewer).await;
    let project = file_project(&db, &ayse, "Ada 12 dosyası").await;
    let by = open(&db, &ayse, project).await;
    common::share(&db, &by, &dilek.actor, GrantRole::Viewer).await;
    let store = blobs();
    save(&db, &store, &by, MINIMAL, "0").await;
    save(&db, &store, &by, DRAWING, "1").await;
    let first = make(
        &db,
        &store,
        &by,
        json!({ "name": "İlk", "fileRevision": "1" }),
    )
    .await
    .unwrap()
    .checkpoint;

    // A revision by its number.
    let asked = catalog_envelope(
        &by,
        PROJECT_CHECKPOINT_RESTORE,
        json!({ "fileRevision": "2" }),
        &[],
    );
    let two = restore(&db, &store, &by, asked).await.unwrap();
    assert_eq!(two.project.name, "Ada 12 dosyası (r2)");
    let t = open(&db, &ayse, Uuid::parse_str(&two.project.id).unwrap()).await;
    let listed = files::list(&db.app, &t).await.unwrap();
    assert_eq!(
        (listed.current.as_deref(), listed.revisions.len()),
        (Some("1"), 1)
    );
    assert_eq!(listed.revisions[0].sha256, sha(DRAWING));
    // A checkpoint, under another name.
    let asked = catalog_envelope(
        &by,
        PROJECT_CHECKPOINT_RESTORE,
        json!({ "checkpointId": first.id, "name": "Ada 12 ilk hâli" }),
        &[],
    );
    let one = restore(&db, &store, &by, asked).await.unwrap();
    assert_eq!(one.project.name, "Ada 12 ilk hâli");
    let o = open(&db, &ayse, Uuid::parse_str(&one.project.id).unwrap()).await;
    let (_, file) = files::download(&db.app, &store, &o, 1).await.unwrap();
    assert_eq!(read_all(file).await, MINIMAL);

    // Exactly one point; a database project has no file revisions.
    for input in [
        json!({}),
        json!({ "checkpointId": first.id, "fileRevision": "1" }),
    ] {
        let asked = catalog_envelope(&by, PROJECT_CHECKPOINT_RESTORE, input, &[]);
        assert!(matches!(
            restore(&db, &store, &by, asked).await,
            Err(AppError::Invalid { .. })
        ));
    }
    let plain = new_project(&db, &ayse, "Nesneler").await;
    let p = open(&db, &ayse, plain).await;
    let asked = catalog_envelope(
        &p,
        PROJECT_CHECKPOINT_RESTORE,
        json!({ "fileRevision": "1" }),
        &[],
    );
    match restore(&db, &store, &p, asked).await {
        Err(e) => assert_eq!(e.path(), Some("fileRevision")),
        Ok(r) => panic!("restored a revision of a database project: {r:?}"),
    }
    // A viewer may not open projects in the organisation: nothing is restored for them.
    let d = open(&db, &dilek, project).await;
    let asked = catalog_envelope(
        &d,
        PROJECT_CHECKPOINT_RESTORE,
        json!({ "fileRevision": "1" }),
        &[],
    );
    assert!(matches!(
        restore(&db, &store, &d, asked).await,
        Err(AppError::Forbidden(_))
    ));
    db.close().await;
}
