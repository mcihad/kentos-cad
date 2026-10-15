//! A database project's associative arrays (docs/adr/0300 §5, migration
//! 0019): kept in the project's row as metadata, replaced or removed whole by
//! `project.changes`, held to the contract's shape (the blocks and objects
//! being the document's), given back in its `.kcad` image as its blocks and
//! objects hold them, and taken by a copy.

mod common;

use common::{envelope, member, new_project, open, run};
use kentos_application::commands::CommandOutcome;
use kentos_application::{AppError, admin, changes, projects, snapshot};
use kentos_contracts::{
    AssocArray, BlockChange, BlockDefinition, Entity, FeatureChange, PROJECT_DUPLICATE,
    ProjectChanges, ProjectPatch, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/kcad/v2/arrays.json");

/// The sample's block, its inserts under their ids on the test project's layer, and its arrays.
fn sample() -> (BlockDefinition, Vec<(String, Entity)>, Vec<AssocArray>) {
    let doc: serde_json::Value = serde_json::from_str(SAMPLE).unwrap();
    let block = serde_json::from_value(doc["blocks"][0].clone()).unwrap();
    let objects = doc["entities"]
        .as_array()
        .unwrap()
        .iter()
        .zip(doc["uids"].as_array().unwrap())
        .map(|(e, uid)| {
            let mut e: Entity = serde_json::from_value(e.clone()).unwrap();
            e.base_mut().layer_id = "cizim".into();
            (uid.as_str().unwrap().to_owned(), e)
        })
        .collect();
    let arrays = serde_json::from_value(doc["arrays"].clone()).unwrap();
    (block, objects, arrays)
}

fn patch(arrays: Vec<AssocArray>) -> ProjectChanges {
    ProjectChanges {
        features: Vec::new(),
        blocks: Vec::new(),
        project: Some(ProjectPatch {
            arrays: Some(arrays),
            ..ProjectPatch::default()
        }),
    }
}

#[tokio::test]
async fn a_database_projects_arrays_are_kept_followed_and_copied() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Dizili").await;
    let by = open(&db, &pm, project).await;
    assert!(
        projects::info(&db.app, &by)
            .await
            .unwrap()
            .arrays
            .is_empty()
    );

    // The block, its inserts and the arrays in one commit: the info and the image hold them.
    let (block, objects, arrays) = sample();
    let made = changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            ProjectChanges {
                features: objects
                    .iter()
                    .map(|(id, entity)| FeatureChange::Create {
                        id: id.clone(),
                        entity: entity.clone(),
                    })
                    .collect(),
                blocks: vec![BlockChange::Create { block }],
                project: Some(ProjectPatch {
                    arrays: Some(arrays.clone()),
                    ..ProjectPatch::default()
                }),
            },
            &[("@project", "1")],
        ),
    )
    .await
    .unwrap();
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(info.arrays, arrays);
    assert_eq!(info.meta_version, "2");
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(image.arrays, arrays);

    // An item's insert removed without its array (another client): the image's item is removed, the file
    // stays valid.
    let first = &objects[0].0;
    changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            ProjectChanges {
                features: vec![FeatureChange::Delete { id: first.clone() }],
                blocks: Vec::new(),
                project: None,
            },
            &[(first.as_str(), made.versions[first].as_str())],
        ),
    )
    .await
    .unwrap();
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(image.arrays.len(), arrays.len());
    assert_eq!(image.arrays[0].items[0].uid, None);
    assert_eq!(image.arrays[0].items[1], arrays[0].items[1]);

    // A shape the contract would not keep is refused at its place, and nothing changes.
    let mut wrong = arrays.clone();
    wrong[0].items[2].uid = wrong[0].items[1].uid;
    match changes::commit(
        &db.app,
        &by,
        envelope(&pm, project, patch(wrong), &[("@project", "2")]),
    )
    .await
    {
        Err(AppError::Invalid { path, .. }) => {
            assert_eq!(path.as_deref(), Some("project.arrays[0].items[2]"));
        }
        other => panic!("not refused: {other:?}"),
    }
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().meta_version,
        "2"
    );

    // A copy takes them.
    let copy = match run(&db, &by, PROJECT_DUPLICATE, json!({})).await {
        Ok(CommandOutcome::Duplicated(d)) => d,
        other => panic!("not a copy: {other:?}"),
    };
    let copied = open(&db, &pm, Uuid::parse_str(&copy.project.id).unwrap()).await;
    assert_eq!(
        projects::info(&db.app, &copied).await.unwrap().arrays,
        arrays
    );

    // Removed: none left, a metadata change of its own.
    changes::commit(
        &db.app,
        &by,
        envelope(&pm, project, patch(Vec::new()), &[("@project", "2")]),
    )
    .await
    .unwrap();
    let info = projects::info(&db.app, &by).await.unwrap();
    assert!(info.arrays.is_empty());
    assert_eq!(info.meta_version, "3");
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert!(image.arrays.is_empty());
}
