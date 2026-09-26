//! `project.import` (docs/adr/0036): a verified `.kcad` upload comes into a
//! new, empty database project whole, in one transaction, or not at all.
//! Real database (`KENTOS_TEST_DB=required` makes a missing one a failure).

mod common;

use std::collections::BTreeMap;

use common::{blobs, catalog_envelope, member, new_project, open};
use kentos_application::access::ProjectAccess;
use kentos_application::blobs::Blobs;
use kentos_application::commands::{CatalogPolicy, CommandOutcome, run};
use kentos_application::{AppError, admin, files, projects, snapshot};
use kentos_contracts::{
    DocumentSnapshotV1, DocumentSnapshotV2, EntityId, FileUploadBegin, GrantRole, PROJECT_IMPORT,
    ProjectCreate, ProjectImported, ProjectStorage, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const DRAWING: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/drawing.kcad");
const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");
/// The object of `drawing.kcad` whose values the server does not keep (beyond ±10⁹).
const EXTREME: usize = 13;

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `drawing.kcad` without its object of extreme values: every kind the server keeps.
fn keepable() -> DocumentSnapshotV2 {
    let mut doc = kentos_kcad::decode(DRAWING).unwrap();
    doc.entities.remove(EXTREME);
    doc.uids.remove(EXTREME);
    doc
}

/// Every object by its persistent id, without its slot (a file keeps no slots).
fn by_uid(doc: &DocumentSnapshotV2) -> BTreeMap<EntityId, Value> {
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

async fn upload(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    bytes: &[u8],
) -> Result<Uuid, AppError> {
    let begun = files::begin(
        &db.app,
        by,
        FileUploadBegin {
            size: bytes.len() as u32,
            sha256: sha(bytes),
        },
    )
    .await?;
    let id = Uuid::parse_str(&begun.id).unwrap();
    let mut receiving = files::start_receive(&db.app, store, by, id).await?;
    receiving.writer.write(bytes).await.unwrap();
    files::finish_receive(&db.app, store, by, receiving).await?;
    Ok(id)
}

async fn import(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    upload: Uuid,
) -> Result<ProjectImported, AppError> {
    let envelope = catalog_envelope(
        by,
        PROJECT_IMPORT,
        json!({ "uploadId": upload.to_string() }),
        &[],
    );
    match run(&db.app, store, &CatalogPolicy::default(), by, envelope).await? {
        CommandOutcome::Imported(i) => Ok(i),
        other => panic!("not an import: {other:?}"),
    }
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
async fn a_drawing_comes_into_a_new_project_whole() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = new_project(&db, &ayse, "Ada 5 (içe aktarım)").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();
    let file = keepable();
    let bytes = kentos_kcad::encode(&file).unwrap();

    let sent = upload(&db, &store, &by, &bytes).await.unwrap();
    let envelope = catalog_envelope(
        &by,
        PROJECT_IMPORT,
        json!({ "uploadId": sent.to_string() }),
        &[],
    );
    let done = match run(
        &db.app,
        &store,
        &CatalogPolicy::default(),
        &by,
        envelope.clone(),
    )
    .await
    .unwrap()
    {
        CommandOutcome::Imported(i) => i,
        other => panic!("not an import: {other:?}"),
    };
    assert_eq!(
        (
            done.objects.as_str(),
            done.data_revision.as_str(),
            done.replayed
        ),
        ("14", "1", false)
    );
    // The project is the file: its objects under their ids, its settings, layers (locked and hidden ones too) and styles.
    let now = kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(by_uid(&now), by_uid(&file));
    assert_eq!(
        (now.settings.clone(), now.layers.clone(), now.styles.clone()),
        (
            file.settings.clone(),
            file.layers.clone(),
            file.styles.clone()
        )
    );
    assert_eq!(
        (now.origin, now.home_view, now.active_layer.clone()),
        (file.origin, file.home_view, file.active_layer.clone())
    );
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().meta_version,
        done.meta_version
    );
    // The upload is gone with its bytes; the same command again is answered from the log.
    assert!(walk(store.root()).is_empty());
    match run(&db.app, &store, &CatalogPolicy::default(), &by, envelope)
        .await
        .unwrap()
    {
        CommandOutcome::Imported(again) => assert!(again.replayed && again.objects == "14"),
        other => panic!("not an import: {other:?}"),
    }
    // A project with something in it takes no import.
    let second = upload(&db, &store, &by, &bytes).await.unwrap();
    match import(&db, &store, &by, second).await {
        Err(AppError::Invalid { message, .. }) => assert!(message.contains("boş"), "{message}"),
        other => panic!("imported into a project that has a drawing: {other:?}"),
    }
    db.close().await;
}

#[tokio::test]
async fn one_object_the_server_does_not_keep_refuses_the_whole_file() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = new_project(&db, &ayse, "Uç değerler").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();
    let sent = upload(&db, &store, &by, DRAWING).await.unwrap();
    match import(&db, &store, &by, sent).await {
        Err(e) => assert_eq!((e.code(), e.path()), ("invalid", Some("entities[13]"))),
        Ok(done) => panic!("imported values beyond the server's range: {done:?}"),
    }
    // Nothing was written: the project is as new.
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(
        (info.feature_count.as_str(), info.data_revision.as_str()),
        ("0", "0")
    );
    db.close().await;
}

#[tokio::test]
async fn an_import_needs_the_right_project_rights_and_upload() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let cem = member(&db, "buro", "cem", TenantRole::Editor).await;
    let project = new_project(&db, &ayse, "Haklar").await;
    let by = open(&db, &ayse, project).await;
    common::share(&db, &by, &cem.actor, GrantRole::Editor).await;
    let store = blobs();
    let bytes = kentos_kcad::encode(&keepable()).unwrap();

    // An editor changes objects, not the project's settings and layers: an import is both.
    let c = open(&db, &cem, project).await;
    let theirs = upload(&db, &store, &c, &bytes).await.unwrap();
    assert!(matches!(
        import(&db, &store, &c, theirs).await,
        Err(AppError::Forbidden(_))
    ));
    // Someone else's upload does not exist for the caller.
    assert!(matches!(
        import(&db, &store, &by, theirs).await,
        Err(AppError::NotFound(_))
    ));
    // A file project keeps files as revisions, not imports.
    let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    let input = ProjectCreate {
        name: "Dosya".into(),
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
    let file = Uuid::parse_str(
        &projects::create(&db.app, &ayse, input, None)
            .await
            .unwrap()
            .id,
    )
    .unwrap();
    let f = open(&db, &ayse, file).await;
    let sent = upload(&db, &store, &f, &bytes).await.unwrap();
    match import(&db, &store, &f, sent).await {
        Err(AppError::Invalid { message, .. }) => {
            assert!(message.contains("dosya olarak"), "{message}")
        }
        other => panic!("imported into a file project: {other:?}"),
    }
    db.close().await;
}
