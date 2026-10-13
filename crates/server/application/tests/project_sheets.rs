//! A database project's sheet book (docs/adr/0251, migration 0017): kept
//! in the project's row, given back with its info and in its `.kcad` image,
//! replaced or removed whole by `project.changes` (a metadata change), held
//! to the file format's rules, and taken by a copy and by a checkpoint's
//! restore.

mod common;

use common::{blobs, catalog_envelope, envelope, member, new_project, open, run};
use kentos_application::commands::{self, CatalogPolicy, CommandOutcome};
use kentos_application::{AppError, admin, changes, projects, snapshot};
use kentos_contracts::{
    PROJECT_CHECKPOINT_CREATE, PROJECT_CHECKPOINT_RESTORE, PROJECT_DUPLICATE, ProjectChanges,
    ProjectPatch, ProjectSheets, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/kcad/v2/sheets.json");

/// The sample's book: the sheet core's bureau sheet and its logo.
fn bureau() -> ProjectSheets {
    let doc: serde_json::Value = serde_json::from_str(SAMPLE).unwrap();
    serde_json::from_value(doc["sheets"].clone()).unwrap()
}

fn patch(sheets: Option<Option<ProjectSheets>>) -> ProjectChanges {
    ProjectChanges {
        features: Vec::new(),
        blocks: Vec::new(),
        project: Some(ProjectPatch {
            sheets,
            ..ProjectPatch::default()
        }),
    }
}

#[tokio::test]
async fn a_database_projects_sheet_book_is_kept_changed_and_copied() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Paftalı").await;
    let by = open(&db, &pm, project).await;
    assert_eq!(projects::info(&db.app, &by).await.unwrap().sheets, None);

    // Given: the project's info and its image hold it; the metadata's version moved.
    let book = bureau();
    changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            patch(Some(Some(book.clone()))),
            &[("@project", "1")],
        ),
    )
    .await
    .unwrap();
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(info.sheets.as_ref(), Some(&book));
    assert_eq!(info.meta_version, "2");
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(image.sheets.as_ref(), Some(&book));

    // A patch without it keeps it.
    let renamed = ProjectChanges {
        features: Vec::new(),
        blocks: Vec::new(),
        project: Some(ProjectPatch {
            name: Some("Paftalı proje".into()),
            ..ProjectPatch::default()
        }),
    };
    changes::commit(
        &db.app,
        &by,
        envelope(&pm, project, renamed, &[("@project", "2")]),
    )
    .await
    .unwrap();
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().sheets.as_ref(),
        Some(&book)
    );

    // One the file format would not keep is refused, at its place, and nothing changes.
    let mut wrong = book.clone();
    wrong.pictures[0].bytes.push(0);
    match changes::commit(
        &db.app,
        &by,
        envelope(&pm, project, patch(Some(Some(wrong))), &[("@project", "3")]),
    )
    .await
    {
        Err(AppError::Invalid { path, .. }) => {
            assert_eq!(path.as_deref(), Some("project.sheets.pictures[0].sha256"));
        }
        other => panic!("not refused: {other:?}"),
    }
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().meta_version,
        "3"
    );

    // A copy takes it; a checkpoint restored as a new project too.
    let copy = match run(&db, &by, PROJECT_DUPLICATE, json!({})).await {
        Ok(CommandOutcome::Duplicated(d)) => d,
        other => panic!("not a copy: {other:?}"),
    };
    let copied = open(&db, &pm, Uuid::parse_str(&copy.project.id).unwrap()).await;
    assert_eq!(
        projects::info(&db.app, &copied)
            .await
            .unwrap()
            .sheets
            .as_ref(),
        Some(&book)
    );
    let store = blobs();
    let point = match commands::run(
        &db.app,
        &store,
        &CatalogPolicy::default(),
        &by,
        catalog_envelope(
            &by,
            PROJECT_CHECKPOINT_CREATE,
            json!({ "name": "Teslim" }),
            &[],
        ),
    )
    .await
    .unwrap()
    {
        CommandOutcome::Checkpoint(c) => c.checkpoint.id,
        other => panic!("not a checkpoint: {other:?}"),
    };
    let restored = match commands::run(
        &db.app,
        &store,
        &CatalogPolicy::default(),
        &by,
        catalog_envelope(
            &by,
            PROJECT_CHECKPOINT_RESTORE,
            json!({ "checkpointId": point }),
            &[],
        ),
    )
    .await
    .unwrap()
    {
        CommandOutcome::Duplicated(d) => d,
        other => panic!("not a restore: {other:?}"),
    };
    let back = open(&db, &pm, Uuid::parse_str(&restored.project.id).unwrap()).await;
    assert_eq!(
        projects::info(&db.app, &back)
            .await
            .unwrap()
            .sheets
            .as_ref(),
        Some(&book)
    );

    // Removed: null, a metadata change of its own.
    changes::commit(
        &db.app,
        &by,
        envelope(&pm, project, patch(Some(None)), &[("@project", "3")]),
    )
    .await
    .unwrap();
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(info.sheets, None);
    assert_eq!(info.meta_version, "4");
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(image.sheets, None);
}
