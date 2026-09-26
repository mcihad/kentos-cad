//! The acceptance set of TODOS.md CLOUD-29, run the same way on a project
//! kept in the database and one kept as a file: A saves their 18 uygulaması
//! project; B views and downloads it, C edits it, D (a member it was not
//! shared with) and E (another organisation) reach nothing; a checkpoint
//! comes back as a new project; the binary export is the drawing; taking
//! B's share away closes it to B. What the desktop and the web do with it
//! is theirs to accept. Real database (`KENTOS_TEST_DB=required` makes a
//! missing one a failure).

mod common;

use common::{a_point, blobs, catalog_envelope, envelope, member, open, try_open};
use kentos_application::access::ProjectAccess;
use kentos_application::blobs::Blobs;
use kentos_application::commands::{CatalogPolicy, CommandOutcome, run};
use kentos_application::tenancy::Access;
use kentos_application::{AppError, admin, changes, files, listing, projects, snapshot};
use kentos_contracts::{
    CommandEnvelope, DocumentSnapshotV1, FileUploadBegin, GrantRole, PROJECT_CHECKPOINT_CREATE,
    PROJECT_CHECKPOINT_RESTORE, PROJECT_FILE_COMMIT, ProjectCreate, ProjectStorage, ProjectType,
    TenantRole,
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
) -> Result<CommandOutcome, AppError> {
    run(&db.app, store, &CatalogPolicy::default(), by, envelope).await
}

/// A file project's save: upload, verify, commit on `base`.
async fn save_file(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    bytes: &[u8],
    base: &str,
) -> Result<(), AppError> {
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
    command(db, store, by, commit).await.map(|_| ())
}

/// A saves (first) or C edits (then) the project, in its storage mode.
async fn save(
    db: &TestDb,
    store: &Blobs,
    who: &Access,
    by: &ProjectAccess,
    storage: ProjectStorage,
    first: bool,
) -> Result<(), AppError> {
    match (storage, first) {
        (ProjectStorage::Database, true) => changes::commit(
            &db.app,
            by,
            envelope(who, by.project, a_point(486500.0), &[]),
        )
        .await
        .map(|_| ()),
        (ProjectStorage::Database, false) => changes::commit(
            &db.app,
            by,
            envelope(who, by.project, a_point(486510.0), &[]),
        )
        .await
        .map(|_| ()),
        (ProjectStorage::File, true) => save_file(db, store, by, MINIMAL, "0").await,
        (ProjectStorage::File, false) => save_file(db, store, by, DRAWING, "1").await,
    }
}

/// The project's binary export as `by` downloads it: a database project's
/// snapshot, a file project's newest revision.
async fn export(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    storage: ProjectStorage,
) -> Result<Vec<u8>, AppError> {
    match storage {
        ProjectStorage::Database => Ok(snapshot::snapshot(&db.app, by).await?.bytes),
        ProjectStorage::File => {
            let current = files::list(&db.app, by).await?.current.unwrap();
            let (_, mut file) =
                files::download(&db.app, store, by, current.parse().unwrap()).await?;
            let mut bytes = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
                .await
                .unwrap();
            Ok(bytes)
        }
    }
}

async fn scenario(storage: ProjectStorage) {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let buro = admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 5)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer Büro", 2)
        .await
        .unwrap();
    let a = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let b = member(&db, "buro", "bora", TenantRole::Viewer).await;
    let c = member(&db, "buro", "cem", TenantRole::Editor).await;
    let d = member(&db, "buro", "derya", TenantRole::Editor).await;
    let e = member(&db, "diger", "emre", TenantRole::Owner).await;
    let store = blobs();

    // A makes and saves their 18 uygulaması project, and names the moment.
    let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    let input = ProjectCreate {
        name: "Ada 101 — 18 uygulaması".into(),
        settings: s.settings,
        origin: s.origin,
        home_view: s.home_view,
        layers: serde_json::from_value(json!([{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }])).unwrap(),
        active_layer: "cizim".into(),
        styles: s.styles,
        description: Some("Kabul seti".into()),
        project_type: Some(ProjectType::LandReadjustment),
        tags: None,
        storage: Some(storage),
    };
    let project =
        Uuid::parse_str(&projects::create(&db.app, &a, input, None).await.unwrap().id).unwrap();
    let by_a = open(&db, &a, project).await;
    save(&db, &store, &a, &by_a, storage, true).await.unwrap();
    let point = match command(
        &db,
        &store,
        &by_a,
        catalog_envelope(
            &by_a,
            PROJECT_CHECKPOINT_CREATE,
            json!({ "name": "A'nın kaydı" }),
            &[],
        ),
    )
    .await
    .unwrap()
    {
        CommandOutcome::Checkpoint(c) => c.checkpoint,
        other => panic!("not a checkpoint: {other:?}"),
    };
    let saved = export(&db, &store, &by_a, storage).await.unwrap();
    common::share(&db, &by_a, &b.actor, GrantRole::Viewer).await;
    common::share(&db, &by_a, &c.actor, GrantRole::Editor).await;

    // B views and downloads; B cannot change it.
    let by_b = open(&db, &b, project).await;
    assert_eq!(
        projects::info(&db.app, &by_b).await.unwrap().name,
        "Ada 101 — 18 uygulaması"
    );
    assert!(
        listing::mine(&db.app, &b.actor)
            .await
            .unwrap()
            .projects
            .iter()
            .any(|p| p.id == project.to_string())
    );
    assert_eq!(export(&db, &store, &by_b, storage).await.unwrap(), saved);
    assert!(matches!(
        save(&db, &store, &b, &by_b, storage, false).await,
        Err(AppError::Forbidden(_))
    ));

    // C edits.
    let by_c = open(&db, &c, project).await;
    save(&db, &store, &c, &by_c, storage, false).await.unwrap();
    let edited = export(&db, &store, &by_a, storage).await.unwrap();
    assert_ne!(edited, saved);

    // D, a member it was not shared with, and E, of another organisation, reach nothing.
    for who in [&d, &e] {
        assert!(matches!(
            try_open(&db, &who.actor, buro, project).await,
            Err(AppError::NotFound(_))
        ));
        let mine = listing::mine(&db.app, &who.actor).await.unwrap();
        assert!(mine.projects.iter().all(|p| p.id != project.to_string()));
    }

    // The saved moment comes back as a new project; the project itself keeps C's edit.
    let restored = match command(
        &db,
        &store,
        &by_a,
        catalog_envelope(
            &by_a,
            PROJECT_CHECKPOINT_RESTORE,
            json!({ "checkpointId": point.id }),
            &[],
        ),
    )
    .await
    .unwrap()
    {
        CommandOutcome::Duplicated(d) => d,
        other => panic!("not a restore: {other:?}"),
    };
    let back = open(&db, &a, Uuid::parse_str(&restored.project.id).unwrap()).await;
    let old = export(&db, &store, &back, storage).await.unwrap();
    match storage {
        // A snapshot of the new project is written anew; its drawing is the saved one.
        ProjectStorage::Database => {
            let (old, saved) = (
                kentos_kcad::decode(&old).unwrap(),
                kentos_kcad::decode(&saved).unwrap(),
            );
            assert_eq!(
                (old.entities.len(), old.uids.clone()),
                (saved.entities.len(), saved.uids.clone())
            );
        }
        ProjectStorage::File => assert_eq!(old, saved),
    }
    assert_eq!(export(&db, &store, &by_a, storage).await.unwrap(), edited);

    // B's share taken away: closed to B.
    common::revoke(&db, &by_a, &b.actor).await;
    assert!(matches!(
        try_open(&db, &b.actor, buro, project).await,
        Err(AppError::NotFound(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn a_database_project_passes_the_acceptance_set() {
    scenario(ProjectStorage::Database).await;
}

#[tokio::test]
async fn a_file_project_passes_the_acceptance_set() {
    scenario(ProjectStorage::File).await;
}
