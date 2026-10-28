//! A project's kitchen runs (docs/adr/0727 §5) commit with their objects and blocks, come back in the project's
//! information and its `.kcad` snapshot, are copied with the project, refused when they break the contract's rules and
//! taken away by an empty list. KENTOS_TEST_DB=required makes a missing real PostgreSQL test environment fail.

mod common;

use common::{envelope, member, open};
use kentos_application::{admin, changes, projects, snapshot};
use kentos_contracts::{
    BlockChange, DocumentSnapshotV2, FeatureChange, ProjectChanges, ProjectCreate, ProjectPatch, TenantRole,
};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

fn model() -> DocumentSnapshotV2 {
    serde_json::from_str(include_str!("../../../../fixtures/kcad/v2/casework.json")).unwrap()
}

fn create(source: &DocumentSnapshotV2) -> ProjectCreate {
    ProjectCreate {
        name: "Mutfak".into(),
        settings: source.settings.clone(),
        origin: source.origin,
        home_view: source.home_view,
        layers: source.layers.clone(),
        active_layer: source.active_layer.clone(),
        styles: source.styles.clone(),
        sheets: None,
        arrays: Vec::new(),
        dimension_sets: Vec::new(),
        casework: Vec::new(),
        constraints: Default::default(),
        description: None,
        project_type: None,
        tags: None,
        storage: None,
    }
}

#[tokio::test]
async fn kitchen_runs_commit_snapshot_copy_and_go() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 3).await.unwrap();
    let who = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let source = model();
    assert_eq!(source.casework.len(), 1, "the fixture's L run");
    let created = projects::create(&db.app, &who, create(&source), None).await.unwrap();
    let project = Uuid::parse_str(&created.id).unwrap();
    let by = open(&db, &who, project).await;

    // A run against the contract's rules is refused at its place, nothing written.
    let mut twice = source.casework.clone();
    twice.push(twice[0].clone());
    let refused = changes::commit(
        &db.app,
        &by,
        envelope(
            &who,
            project,
            ProjectChanges {
                features: Vec::new(),
                blocks: Vec::new(),
                project: Some(ProjectPatch { casework: Some(twice), ..Default::default() }),
            },
            &[("@project", &created.meta_version)],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(refused.path(), Some("project.casework[1]"));
    assert_eq!(projects::info(&db.app, &by).await.unwrap().data_revision, "0");

    // The run's objects, its blocks and its record in one revision.
    let changes = ProjectChanges {
        features: source
            .uids
            .iter()
            .zip(&source.entities)
            .map(|(id, entity)| FeatureChange::Create { id: id.to_text(), entity: entity.clone() })
            .collect(),
        blocks: source.blocks.iter().map(|b| BlockChange::Create { block: b.clone() }).collect(),
        project: Some(ProjectPatch { casework: Some(source.casework.clone()), ..Default::default() }),
    };
    let committed = changes::commit(
        &db.app,
        &by,
        envelope(&who, project, changes, &[("@project", &created.meta_version)]),
    )
    .await
    .unwrap();
    assert_eq!(committed.data_revision, "1");
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(info.casework, source.casework);
    assert_eq!(info.feature_count, source.entities.len().to_string());
    let back = kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert_eq!(back.casework, source.casework, "the snapshot carries the run");
    assert_eq!(back.uids.len(), source.uids.len());

    // SQL duplication keeps the run with its objects.
    let copy = Uuid::now_v7();
    let mut tx = db.app.scoped(by.scope()).await.unwrap();
    let count: i64 = sqlx::query_scalar("select kentos.duplicate_project($1, $2, $1, $3, $4)")
        .bind(who.tenant)
        .bind(project)
        .bind(copy)
        .bind("Mutfak kopyası")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(count, source.entities.len() as i64);
    tx.commit().await.unwrap();
    let copy_by = open(&db, &who, copy).await;
    assert_eq!(projects::info(&db.app, &copy_by).await.unwrap().casework, source.casework);

    // An empty list takes the runs away; the objects stay.
    let cleared = changes::commit(
        &db.app,
        &by,
        envelope(
            &who,
            project,
            ProjectChanges {
                features: Vec::new(),
                blocks: Vec::new(),
                project: Some(ProjectPatch { casework: Some(Vec::new()), ..Default::default() }),
            },
            &[("@project", &info.meta_version)],
        ),
    )
    .await
    .unwrap();
    assert_eq!(cleared.data_revision, "2", "every commit is a revision");
    let after = projects::info(&db.app, &by).await.unwrap();
    assert_ne!(after.meta_version, info.meta_version, "the record is the project's meta");
    assert!(after.casework.is_empty());
    assert_eq!(after.feature_count, source.entities.len().to_string());
    let back = kentos_kcad::decode(&snapshot::snapshot(&db.app, &by).await.unwrap().bytes).unwrap();
    assert!(back.casework.is_empty());
}
