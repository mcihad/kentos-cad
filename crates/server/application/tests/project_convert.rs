//! `project.convert` (docs/adr/0039): a new project in the other storage
//! mode from a project's present state; the source does not change. Real
//! database (`KENTOS_TEST_DB=required` makes a missing one a failure).

mod common;

use std::collections::BTreeMap;

use common::{a_point, blobs, catalog_envelope, envelope, member, new_project, open};
use kentos_application::access::ProjectAccess;
use kentos_application::blobs::Blobs;
use kentos_application::commands::{CatalogPolicy, CommandOutcome, run};
use kentos_application::{AppError, admin, changes, files, listing, projects, snapshot};
use kentos_contracts::{
    CommandEnvelope, DocumentSnapshotV1, DocumentSnapshotV2, EntityId, FileUploadBegin, GrantRole,
    PROJECT_CONVERT, PROJECT_FILE_COMMIT, ProjectCreate, ProjectDuplicated, ProjectStorage,
    TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const DRAWING: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/drawing.kcad");
const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `drawing.kcad` without its object of extreme values (beyond the server's ±10⁹).
fn keepable() -> Vec<u8> {
    let mut doc = kentos_kcad::decode(DRAWING).unwrap();
    doc.entities.remove(13);
    doc.uids.remove(13);
    kentos_kcad::encode(&doc).unwrap()
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

/// Saves `bytes` as a file project's first revision.
async fn save_first(db: &TestDb, store: &Blobs, by: &ProjectAccess, bytes: &[u8]) {
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
        expected_versions: [("@file".to_string(), "0".to_string())]
            .into_iter()
            .collect(),
        input: json!({ "uploadId": id.to_string() }),
    };
    run(&db.app, store, &CatalogPolicy::default(), by, commit)
        .await
        .unwrap();
}

async fn convert(
    db: &TestDb,
    store: &Blobs,
    envelope: CommandEnvelope,
    by: &ProjectAccess,
) -> Result<ProjectDuplicated, AppError> {
    match run(&db.app, store, &CatalogPolicy::default(), by, envelope).await? {
        CommandOutcome::Duplicated(d) => Ok(d),
        other => panic!("not a conversion: {other:?}"),
    }
}

#[tokio::test]
async fn a_file_project_goes_into_postgis_as_a_new_project() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = file_project(&db, &ayse, "Ada 5 dosyası").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();
    let bytes = keepable();
    save_first(&db, &store, &by, &bytes).await;

    let asked = catalog_envelope(&by, PROJECT_CONVERT, json!({ "to": "database" }), &[]);
    let made = convert(&db, &store, asked.clone(), &by).await.unwrap();
    assert_eq!(
        (made.project.name.as_str(), made.objects.as_str()),
        ("Ada 5 dosyası (PostGIS)", "14")
    );
    assert_eq!(made.project.storage, ProjectStorage::Database);
    // The new project is the file, object by object; the source is still the file project it was.
    let c = open(&db, &ayse, Uuid::parse_str(&made.project.id).unwrap()).await;
    let now = kentos_kcad::decode(&snapshot::snapshot(&db.app, &c).await.unwrap().bytes).unwrap();
    let file = kentos_kcad::decode(&bytes).unwrap();
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
        files::list(&db.app, &by).await.unwrap().current.as_deref(),
        Some("1")
    );
    // The same command again: the same new project.
    let again = convert(&db, &store, asked, &by).await.unwrap();
    assert_eq!((again.project.id, again.replayed), (made.project.id, true));
    db.close().await;
}

#[tokio::test]
async fn a_file_the_database_does_not_keep_is_refused_by_its_place() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = file_project(&db, &ayse, "Uç değerler").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();
    save_first(&db, &store, &by, DRAWING).await;
    let before = listing::mine(&db.app, &ayse.actor)
        .await
        .unwrap()
        .projects
        .len();
    match convert(
        &db,
        &store,
        catalog_envelope(&by, PROJECT_CONVERT, json!({ "to": "database" }), &[]),
        &by,
    )
    .await
    {
        Err(e) => assert_eq!((e.code(), e.path()), ("invalid", Some("entities[13]"))),
        Ok(made) => panic!("converted values beyond the database's range: {made:?}"),
    }
    assert_eq!(
        listing::mine(&db.app, &ayse.actor)
            .await
            .unwrap()
            .projects
            .len(),
        before
    );
    db.close().await;
}

#[tokio::test]
async fn a_database_project_becomes_a_file_project_of_one_moment() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let dilek = member(&db, "buro", "dilek", TenantRole::Viewer).await;
    let project = new_project(&db, &ayse, "Ada 7").await;
    let by = open(&db, &ayse, project).await;
    common::share(&db, &by, &dilek.actor, GrantRole::Viewer).await;
    let store = blobs();
    for x in [486500.0, 486510.0] {
        changes::commit(&db.app, &by, envelope(&ayse, project, a_point(x), &[]))
            .await
            .unwrap();
    }
    let source =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();

    let made = convert(
        &db,
        &store,
        catalog_envelope(
            &by,
            PROJECT_CONVERT,
            json!({ "to": "file", "name": "Ada 7 teslim dosyası" }),
            &[],
        ),
        &by,
    )
    .await
    .unwrap();
    assert_eq!(
        (
            made.project.name.as_str(),
            made.objects.as_str(),
            made.project.storage
        ),
        ("Ada 7 teslim dosyası", "2", ProjectStorage::File)
    );
    let f = open(&db, &ayse, Uuid::parse_str(&made.project.id).unwrap()).await;
    let listed = files::list(&db.app, &f).await.unwrap();
    assert_eq!(
        (
            listed.current.as_deref(),
            listed.revisions[0].objects.as_deref()
        ),
        (Some("1"), Some("2"))
    );
    let (_, mut file) = files::download(&db.app, &store, &f, 1).await.unwrap();
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
        .await
        .unwrap();
    let kept = kentos_kcad::decode(&bytes).unwrap();
    assert_eq!(by_uid(&kept), by_uid(&source));
    // The source is still a database project with its two objects.
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().feature_count,
        "2"
    );

    // The same mode is not a conversion; a viewer who may not download converts nothing.
    match convert(
        &db,
        &store,
        catalog_envelope(&by, PROJECT_CONVERT, json!({ "to": "database" }), &[]),
        &by,
    )
    .await
    {
        Err(e) => assert_eq!(e.path(), Some("to")),
        Ok(made) => panic!("converted into its own mode: {made:?}"),
    }
    admin::set_tenant_policy(&db.owner, "buro", None, Some(false))
        .await
        .unwrap();
    let d = open(&db, &dilek, project).await;
    assert!(matches!(
        convert(
            &db,
            &store,
            catalog_envelope(&d, PROJECT_CONVERT, json!({ "to": "file" }), &[]),
            &d
        )
        .await,
        Err(AppError::Forbidden(_))
    ));
    // A file project with nothing saved has nothing to convert.
    let empty = file_project(&db, &ayse, "Boş dosya").await;
    let e = open(&db, &ayse, empty).await;
    assert!(matches!(
        convert(
            &db,
            &store,
            catalog_envelope(&e, PROJECT_CONVERT, json!({ "to": "database" }), &[]),
            &e
        )
        .await,
        Err(AppError::Invalid { .. })
    ));
    db.close().await;
}
