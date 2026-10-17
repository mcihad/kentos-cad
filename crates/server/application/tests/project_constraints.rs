//! A database project's constraints and parameters (docs/adr/0336 §3, migration 0022): kept in the project's row as
//! metadata, replaced or removed whole by `project.changes`, held to the contract's rules (the objects being the
//! document's), given back in its `.kcad` image as its objects hold them, and taken by a copy.

mod common;

use common::{envelope, member, new_project, open, run};
use kentos_application::commands::CommandOutcome;
use kentos_application::{AppError, admin, changes, projects, snapshot};
use kentos_contracts::{
    ConstraintSet, Entity, FeatureChange, PROJECT_DUPLICATE, ProjectChanges, ProjectPatch,
    TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/kcad/v2/constraints.json");

/// The sample's objects under their ids on the test project's layer, and its constraints.
fn sample() -> (Vec<(String, Entity)>, ConstraintSet) {
    let doc: serde_json::Value = serde_json::from_str(SAMPLE).unwrap();
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
    let set = serde_json::from_value(doc["constraints"].clone()).unwrap();
    (objects, set)
}

fn patch(set: ConstraintSet) -> ProjectChanges {
    ProjectChanges {
        features: Vec::new(),
        blocks: Vec::new(),
        project: Some(ProjectPatch {
            constraints: Some(set),
            ..ProjectPatch::default()
        }),
    }
}

#[tokio::test]
async fn a_database_projects_constraints_are_kept_followed_and_copied() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Kısıtlı").await;
    let by = open(&db, &pm, project).await;
    assert!(
        projects::info(&db.app, &by)
            .await
            .unwrap()
            .constraints
            .is_empty()
    );

    // The objects and the constraints in one commit: the info and the image hold them.
    let (objects, set) = sample();
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
                blocks: Vec::new(),
                project: Some(ProjectPatch {
                    constraints: Some(set.clone()),
                    ..ProjectPatch::default()
                }),
            },
            &[("@project", "1")],
        ),
    )
    .await
    .unwrap();
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(info.constraints, set);
    assert_eq!(info.meta_version, "2");
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(image.constraints, set);

    // The diagonal removed without its constraints (another client): the image's set drops the ones naming it and
    // the parameters only they had; the file stays valid.
    let diagonal = &objects[5].0;
    changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            ProjectChanges {
                features: vec![FeatureChange::Delete {
                    id: diagonal.clone(),
                }],
                blocks: Vec::new(),
                project: None,
            },
            &[(diagonal.as_str(), made.versions[diagonal].as_str())],
        ),
    )
    .await
    .unwrap();
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(
        image.constraints.constraints.len(),
        set.constraints.len() - 3
    );
    assert!(
        image.constraints.parameter("a1").is_none() && image.constraints.parameter("d3").is_none()
    );
    assert!(image.constraints.parameter("genislik").is_some());

    // A set the contract would not keep is refused at its place, and nothing changes.
    let mut wrong = set.clone();
    wrong.parameters[0].name = "2x".into();
    match changes::commit(
        &db.app,
        &by,
        envelope(&pm, project, patch(wrong), &[("@project", "2")]),
    )
    .await
    {
        Err(AppError::Invalid { path, .. }) => {
            assert_eq!(path.as_deref(), Some("project.constraints.parameters[0]"));
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
        projects::info(&db.app, &copied).await.unwrap().constraints,
        set
    );

    // Removed: none left, a metadata change of its own.
    changes::commit(
        &db.app,
        &by,
        envelope(
            &pm,
            project,
            patch(ConstraintSet::default()),
            &[("@project", "2")],
        ),
    )
    .await
    .unwrap();
    let info = projects::info(&db.app, &by).await.unwrap();
    assert!(info.constraints.is_empty());
    assert_eq!(info.meta_version, "3");
    let image =
        kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert!(image.constraints.is_empty());
}
