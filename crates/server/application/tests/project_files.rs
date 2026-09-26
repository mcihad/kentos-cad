//! Projects kept as files (docs/adr/0031): an upload is verified before it
//! is kept, a commit based on an older revision is a conflict, only writers
//! upload and only those allowed download, and the store's cleanup removes
//! what nobody will commit or what belongs to projects removed for good.
//! Real database (`KENTOS_TEST_DB=required` makes a missing one a failure).

mod common;

use common::{blobs, member, new_project, open};
use kentos_application::access::ProjectAccess;
use kentos_application::blobs::Blobs;
use kentos_application::commands::{CatalogPolicy, CommandOutcome, run};
use kentos_application::tenancy::Access;
use kentos_application::{AppError, admin, changes, files, projects};
use kentos_contracts::{
    CommandEnvelope, DocumentSnapshotV1, FileUploadBegin, PROJECT_FILE_COMMIT, ProjectCreate,
    ProjectStorage, TenantRole,
};
use kentos_postgres::testing::TestDb;
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

async fn file_project(db: &TestDb, who: &Access, name: &str) -> Uuid {
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

/// Opens an upload and sends `bytes` as its body.
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
    send(db, store, by, id, bytes).await.map(|_| id)
}

async fn send(
    db: &TestDb,
    store: &Blobs,
    by: &ProjectAccess,
    upload: Uuid,
    bytes: &[u8],
) -> Result<kentos_contracts::FileUpload, AppError> {
    let mut receiving = files::start_receive(&db.app, store, by, upload).await?;
    // In parts, as a request's body arrives.
    for part in bytes.chunks(97) {
        if let Err(e) = receiving.writer.write(part).await {
            return Err(files::failed_write(store, receiving, e).await);
        }
    }
    files::finish_receive(&db.app, store, by, receiving).await
}

fn commit_envelope(by: &ProjectAccess, upload: Uuid, expected: &str) -> CommandEnvelope {
    CommandEnvelope {
        command_name: PROJECT_FILE_COMMIT.into(),
        version: 1,
        tenant_id: by.tenant.to_string(),
        project_id: by.project.to_string(),
        request_id: format!("istek-{}", Uuid::new_v4()),
        idempotency_key: Uuid::new_v4().to_string(),
        expected_versions: [("@file".to_string(), expected.to_string())]
            .into_iter()
            .collect(),
        input: serde_json::json!({ "uploadId": upload.to_string() }),
    }
}

/// Moves an upload's object to a revision's key, as a commit cut short
/// between that and writing the database leaves it.
async fn cut_short(store: &Blobs, by: &ProjectAccess, upload: Uuid, revision: i64, bytes: &[u8]) {
    store
        .promote(
            &Blobs::upload_key(by.tenant, by.project, upload),
            &Blobs::revision_key(by.tenant, by.project, revision, &sha(bytes)),
        )
        .await
        .unwrap();
}

async fn commit(
    db: &TestDb,
    store: &Blobs,
    envelope: CommandEnvelope,
    by: &ProjectAccess,
) -> Result<kentos_contracts::FileCommitted, AppError> {
    match run(&db.app, store, &CatalogPolicy::default(), by, envelope).await? {
        CommandOutcome::FileCommitted(c) => Ok(c),
        other => panic!("not a file commit: {other:?}"),
    }
}

#[tokio::test]
async fn a_file_project_saves_verified_revisions_in_order() {
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

    assert_eq!(files::list(&db.app, &by).await.unwrap().current, None);
    let first = upload(&db, &store, &by, MINIMAL).await.unwrap();
    let envelope = commit_envelope(&by, first, "0");
    let one = commit(&db, &store, envelope.clone(), &by).await.unwrap();
    assert_eq!(
        (one.revision.as_str(), one.sha256.clone(), one.replayed),
        ("1", sha(MINIMAL), false)
    );
    // The same command again (its answer lost): the stored answer, nothing committed twice.
    let again = commit(&db, &store, envelope, &by).await.unwrap();
    assert_eq!((again.revision.as_str(), again.replayed), ("1", true));

    let second = upload(&db, &store, &by, DRAWING).await.unwrap();
    let two = commit(&db, &store, commit_envelope(&by, second, "1"), &by)
        .await
        .unwrap();
    assert_eq!(two.revision, "2");

    // A save based on revision 1 after revision 2 exists is a conflict, and nothing moves.
    let late = upload(&db, &store, &by, MINIMAL).await.unwrap();
    match commit(&db, &store, commit_envelope(&by, late, "1"), &by).await {
        Err(AppError::Conflict {
            conflicts,
            revision,
            ..
        }) => {
            assert_eq!(
                (
                    conflicts[0].id.as_str(),
                    conflicts[0].expected.as_deref(),
                    conflicts[0].actual.as_deref()
                ),
                ("@file", Some("1"), Some("2"))
            );
            assert!(revision.is_some(), "the conflict says the data revision");
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
    // The refused upload can still be committed on the newest revision.
    let three = commit(&db, &store, commit_envelope(&by, late, "2"), &by)
        .await
        .unwrap();
    assert_eq!(three.revision, "3");

    let listed = files::list(&db.app, &by).await.unwrap();
    assert_eq!(listed.current.as_deref(), Some("3"));
    assert_eq!(
        listed
            .revisions
            .iter()
            .map(|r| r.revision.as_str())
            .collect::<Vec<_>>(),
        ["3", "2", "1"]
    );
    assert_eq!(listed.revisions[2].created_by_name, "ayse");
    let (info, mut file) = files::download(&db.app, &store, &by, 2).await.unwrap();
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
        .await
        .unwrap();
    assert_eq!((info.sha256, bytes.as_slice()), (sha(DRAWING), DRAWING));
    assert!(matches!(
        files::download(&db.app, &store, &by, 9).await,
        Err(AppError::NotFound(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn an_upload_is_checked_before_anything_is_kept() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = file_project(&db, &ayse, "Dosya").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();
    let path = |r: Result<_, AppError>| match r {
        Err(e) => (e.code(), e.path().map(str::to_string)),
        Ok(_) => panic!("expected a refusal"),
    };

    let begin = |size: u32, sha256: &str| FileUploadBegin {
        size,
        sha256: sha256.into(),
    };
    assert_eq!(
        path(files::begin(&db.app, &by, begin(0, &sha(MINIMAL))).await),
        ("invalid", Some("size".into()))
    );
    assert_eq!(
        path(
            files::begin(
                &db.app,
                &by,
                begin(kentos_contracts::FILE_UPLOAD_MAX + 1, &sha(MINIMAL))
            )
            .await
        ),
        ("invalid", Some("size".into()))
    );
    assert_eq!(
        path(files::begin(&db.app, &by, begin(3, "ABC")).await),
        ("invalid", Some("sha256".into()))
    );

    // More bytes than declared: cut off, nothing kept.
    let short = files::begin(&db.app, &by, begin(10, &sha(MINIMAL)))
        .await
        .unwrap();
    let id = Uuid::parse_str(&short.id).unwrap();
    assert_eq!(
        path(send(&db, &store, &by, id, MINIMAL).await),
        ("invalid", Some("size".into()))
    );
    // Another hash than declared.
    let wrong = files::begin(&db.app, &by, begin(MINIMAL.len() as u32, &sha(b"x")))
        .await
        .unwrap();
    let id = Uuid::parse_str(&wrong.id).unwrap();
    assert_eq!(
        path(send(&db, &store, &by, id, MINIMAL).await),
        ("invalid", Some("sha256".into()))
    );
    // The right size and hash, but not a KCAD v2 file.
    let junk = vec![7u8; 64];
    match upload(&db, &store, &by, &junk).await {
        Err(AppError::Invalid { message, .. }) => assert!(message.contains("KCAD v2"), "{message}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    // Nothing of those was kept in the store.
    let kept = walk(store.root());
    assert!(kept.is_empty(), "{kept:?}");

    // A project kept object by object takes no files; a file project takes no object changes.
    let db_project = new_project(&db, &ayse, "Nesneler").await;
    let db_access = open(&db, &ayse, db_project).await;
    match files::begin(&db.app, &db_access, begin(3, &sha(b"abc"))).await {
        Err(AppError::Invalid { message, .. }) => {
            assert!(message.contains("nesne nesne"), "{message}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    let changes = common::envelope(&ayse, project, common::a_point(1.0), &[]);
    match changes::commit(&db.app, &by, changes).await {
        Err(AppError::Invalid { message, .. }) => {
            assert!(message.contains("dosya olarak"), "{message}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    db.close().await;
}

#[tokio::test]
async fn writers_upload_their_own_and_downloads_follow_the_policy() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let komsu = member(&db, "buro", "komsu", TenantRole::Editor).await;
    let dilek = member(&db, "buro", "dilek", TenantRole::Viewer).await;
    let project = file_project(&db, &ayse, "Paylaşılan dosya").await;
    let by = open(&db, &ayse, project).await;
    common::share(&db, &by, &komsu.actor, kentos_contracts::GrantRole::Editor).await;
    common::share(&db, &by, &dilek.actor, kentos_contracts::GrantRole::Viewer).await;
    let store = blobs();
    let mine = upload(&db, &store, &by, MINIMAL).await.unwrap();

    // Another editor cannot finish or commit someone else's upload: for them it does not exist.
    let k = open(&db, &komsu, project).await;
    assert!(matches!(
        files::start_receive(&db.app, &store, &k, mine).await,
        Err(AppError::NotFound(_))
    ));
    assert!(
        commit(&db, &store, commit_envelope(&k, mine, "0"), &k)
            .await
            .is_err()
    );
    commit(&db, &store, commit_envelope(&by, mine, "0"), &by)
        .await
        .unwrap();

    // A viewer reads the list and downloads while the policy lets viewers download; never uploads.
    let d = open(&db, &dilek, project).await;
    assert!(matches!(
        files::begin(
            &db.app,
            &d,
            FileUploadBegin {
                size: 3,
                sha256: sha(b"abc")
            }
        )
        .await,
        Err(AppError::Forbidden(_))
    ));
    assert_eq!(
        files::list(&db.app, &d).await.unwrap().current.as_deref(),
        Some("1")
    );
    assert!(files::download(&db.app, &store, &d, 1).await.is_ok());
    admin::set_tenant_policy(&db.owner, "buro", None, Some(false))
        .await
        .unwrap();
    let d = open(&db, &dilek, project).await;
    assert!(matches!(
        files::download(&db.app, &store, &d, 1).await,
        Err(AppError::Forbidden(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn a_commit_cut_short_is_finished_or_cleared_by_the_next() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = file_project(&db, &ayse, "Yarım kalan").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();

    // Committing the same upload again finds its object at the final key.
    let first = upload(&db, &store, &by, MINIMAL).await.unwrap();
    cut_short(&store, &by, first, 1, MINIMAL).await;
    let one = commit(&db, &store, commit_envelope(&by, first, "0"), &by)
        .await
        .unwrap();
    assert_eq!(one.revision, "1");
    let (_, mut file) = files::download(&db.app, &store, &by, 1).await.unwrap();
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
        .await
        .unwrap();
    assert_eq!(bytes, MINIMAL);

    // Another upload committed instead: the stray object goes, the cut-short upload is gone.
    let abandoned = upload(&db, &store, &by, DRAWING).await.unwrap();
    cut_short(&store, &by, abandoned, 2, DRAWING).await;
    let other = upload(&db, &store, &by, MINIMAL).await.unwrap();
    let two = commit(&db, &store, commit_envelope(&by, other, "1"), &by)
        .await
        .unwrap();
    assert_eq!(two.revision, "2");
    assert_eq!(walk(store.root()).len(), 2, "revisions 1 and 2 only");
    assert!(matches!(
        commit(&db, &store, commit_envelope(&by, abandoned, "2"), &by).await,
        Err(AppError::NotFound(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn the_cleanup_removes_expired_uploads_and_removed_projects() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = file_project(&db, &ayse, "Geçici").await;
    let by = open(&db, &ayse, project).await;
    let store = blobs();
    let committed = upload(&db, &store, &by, MINIMAL).await.unwrap();
    commit(&db, &store, commit_envelope(&by, committed, "0"), &by)
        .await
        .unwrap();
    let forgotten = upload(&db, &store, &by, DRAWING).await.unwrap();

    // Nothing is due yet.
    let done = files::cleanup(&db.app, &store).await.unwrap();
    assert_eq!((done.expired, done.purged), (0, 0));
    // A day later the upload nobody committed goes with its bytes.
    sqlx::query(
        "update kentos.project_upload set created_at = now() - interval '25 hours' where id = $1",
    )
    .bind(forgotten)
    .execute(&db.owner)
    .await
    .unwrap();
    let done = files::cleanup(&db.app, &store).await.unwrap();
    assert_eq!(done.expired, 1);
    assert!(
        store
            .read(&Blobs::upload_key(by.tenant, project, forgotten))
            .await
            .is_err()
    );
    // The committed revision stays until the project is removed for good.
    assert_eq!(walk(store.root()).len(), 1);
    // Removed for good as the trash's retention removes it (migration 0005's function).
    sqlx::query("select kentos.remove_project($1, $2)")
        .bind(by.tenant)
        .bind(project)
        .execute(&db.owner)
        .await
        .unwrap();
    let done = files::cleanup(&db.app, &store).await.unwrap();
    assert_eq!(done.purged, 1);
    assert!(walk(store.root()).is_empty());
    db.close().await;
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
