//! A processing job through the whole chain (docs/adr/0559): a database project, the job asked for as a user, a worker
//! that runs the tool in a real child process (`kentosd job-run`), and the result committed in the finish or staged as
//! a GeoPackage. Real database (`KENTOS_TEST_DB=required` makes a missing one a failure).

#![allow(dead_code)]

#[path = "../src/processing_job.rs"]
mod processing_job;
mod support;

use std::sync::Arc;
use std::time::Duration;

use kentos_application::jobs::{self, JobLimits};
use kentos_application::processing_job::Processor;
use kentos_application::{admin, changes, identity, projects, snapshot, tenancy};
use kentos_contracts::{
    CommandEnvelope, Entity, FeatureChange, PROJECT_CHANGES, PROJECT_PROCESSING_JOB,
    ProcessingJobInput, ProcessingJobMode, ProcessingJobResult, ProjectChanges, ProjectCreate,
    ServerJob, ServerJobCreate, ServerJobState, SignInMethod, TenantRole,
};
use kentos_postgres::testing::TestDb;
use processing_job::ChildProcessor;
use uuid::Uuid;

/// The parcels as one KCAD v2 file, as the server's own snapshot is.
fn parcels() -> kentos_contracts::DocumentSnapshotV2 {
    kentos_kcad::decode(support::PARCELS).expect("the parcels read")
}

/// The layers with every lock opened: the fixture holds a locked layer for its own cases, and a job's commit would meet it.
fn unlocked(mut layers: Vec<kentos_contracts::LayerNode>) -> Vec<kentos_contracts::LayerNode> {
    for l in &mut layers {
        l.locked = false;
        l.children = unlocked(std::mem::take(&mut l.children));
    }
    layers
}

fn processor() -> Arc<dyn Processor> {
    Arc::new(ChildProcessor::with(
        support::child_exe(),
        std::env::temp_dir().join(format!("kentos-job-run-db-{}", Uuid::now_v7())),
    ))
}

fn ask(
    request: &str,
    tool: &str,
    selection: Vec<String>,
    mode: ProcessingJobMode,
) -> ServerJobCreate {
    ServerJobCreate {
        kind: PROJECT_PROCESSING_JOB.into(),
        input: serde_json::to_value(ProcessingJobInput {
            tool: tool.into(),
            selection,
            mode,
            ..ProcessingJobInput::default()
        })
        .unwrap(),
        request_id: request.into(),
        timeout_seconds: None,
    }
}

async fn run_one(
    db: &TestDb,
    store: &kentos_application::blobs::Blobs,
    access: &kentos_application::access::ProjectAccess,
    asked: ServerJobCreate,
) -> ServerJob {
    let job = jobs::create(&db.app, access, asked, &JobLimits::default())
        .await
        .unwrap();
    let worked = jobs::work_once_with(
        &db.app,
        store,
        Uuid::now_v7(),
        Duration::from_secs(30),
        &JobLimits::default(),
        Some(processor()),
    )
    .await
    .unwrap()
    .expect("the worker took a job");
    assert_eq!(worked.job.to_string(), job.id);
    jobs::get(&db.app, access, worked.job).await.unwrap()
}

#[tokio::test]
async fn a_tool_run_by_a_worker_in_a_child_commits_its_result_or_stages_its_file() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let user = admin::create_local_user(&db.owner, "ayse", "ayse", None, "dogru-parola-123")
        .await
        .unwrap();
    admin::set_membership(&db.owner, "buro", "ayse", TenantRole::ProjectManager, true)
        .await
        .unwrap();
    let actor = identity::actor_of(&db.app, user, SignInMethod::Local)
        .await
        .unwrap()
        .unwrap();
    let tenant: Uuid = sqlx::query_scalar("select id from kentos.tenant where slug = 'buro'")
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let ayse = tenancy::access(&db.app, &actor, tenant).await.unwrap();

    // A project of the parcels: their settings, layers and styles, each parcel under its persistent id.
    let doc = parcels();
    let created = projects::create(
        &db.app,
        &ayse,
        ProjectCreate {
            constraints: Default::default(),
            name: "Parseller".into(),
            settings: doc.settings.clone(),
            origin: doc.origin,
            home_view: doc.home_view,
            layers: unlocked(doc.layers.clone()),
            active_layer: doc.active_layer.clone(),
            styles: doc.styles.clone(),
            sheets: None,
            arrays: Vec::new(),
            description: None,
            project_type: None,
            tags: None,
            storage: None,
        },
        None,
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&created.id).unwrap();
    let access = kentos_application::access::project(&db.app, &actor, tenant, project)
        .await
        .unwrap();
    let ids: Vec<String> = doc
        .uids
        .iter()
        .map(|u| Uuid::from_bytes(u.0).to_string())
        .collect();
    let features: Vec<FeatureChange> = ids
        .iter()
        .zip(&doc.entities)
        .map(|(id, entity)| FeatureChange::Create {
            id: id.clone(),
            entity: entity.clone(),
        })
        .collect();
    let objects = features.len();
    changes::commit(
        &db.app,
        &access,
        CommandEnvelope {
            command_name: PROJECT_CHANGES.into(),
            version: 1,
            tenant_id: tenant.to_string(),
            project_id: project.to_string(),
            request_id: "kur-parseller".into(),
            idempotency_key: "kur-parseller".into(),
            expected_versions: Default::default(),
            input: serde_json::to_value(ProjectChanges {
                features,
                project: None,
                blocks: Vec::new(),
            })
            .unwrap(),
        },
    )
    .await
    .unwrap();
    let store = kentos_application::blobs::Blobs::new(
        std::env::temp_dir().join(format!("kentos-job-run-blobs-{}", Uuid::now_v7())),
    );
    let before = snapshot::snapshot(&db.app, &access).await.unwrap();
    assert_eq!((before.objects, before.revision), (objects, 1));
    let parcels_chosen: Vec<String> = ids.iter().take(3).cloned().collect();

    // Öznitelik hesapla: three objects' attributes changed, one commit.
    let job = run_one(
        &db,
        &store,
        &access,
        ask(
            "a",
            "attributes.calculate",
            parcels_chosen.clone(),
            ProcessingJobMode::Commit,
        ),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Succeeded, "{:?}", job.error);
    let result: ProcessingJobResult = serde_json::from_value(job.result.unwrap()).unwrap();
    assert!(result.summary.contains("3 nesnede"), "{}", result.summary);
    assert_eq!((result.added, result.changed, result.removed), (0, 3, 0));
    assert_eq!(result.revision.as_deref(), Some("2"));
    assert!(result.usage.peak_memory_mb > 1.0, "{:?}", result.usage);
    let after = snapshot::snapshot(&db.app, &access).await.unwrap();
    assert_eq!((after.objects, after.revision), (objects, 2));
    let read = kentos_kcad::decode(&after.bytes).unwrap();
    let first = read
        .uids
        .iter()
        .position(|u| Uuid::from_bytes(u.0).to_string() == ids[0])
        .unwrap();
    let Entity::Polygon(parcel) = &read.entities[first] else {
        panic!("a parcel is an area: {:?}", read.entities[first]);
    };
    assert_eq!(
        parcel.base.attrs.get("Hesap alanı").map(String::as_str),
        Some("600.00")
    );

    // The same tool to a file: the project does not change (it is at revision 2); the artifact is a GeoPackage.
    let job = run_one(
        &db,
        &store,
        &access,
        ask(
            "c",
            "points.numberVertices",
            parcels_chosen.iter().take(2).cloned().collect(),
            ProcessingJobMode::Sink,
        ),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Succeeded, "{:?}", job.error);
    let artifact = job.artifact.clone().expect("a file");
    assert!(artifact.name.ends_with(".gpkg"), "{}", artifact.name);
    let (_, mut file) = jobs::artifact(&db.app, &store, &access, Uuid::parse_str(&job.id).unwrap())
        .await
        .unwrap();
    let mut head = vec![0u8; 16];
    tokio::io::AsyncReadExt::read_exact(&mut file, &mut head)
        .await
        .unwrap();
    assert_eq!(&head, b"SQLite format 3\0");
    let unchanged = snapshot::snapshot(&db.app, &access).await.unwrap();
    assert_eq!((unchanged.objects, unchanged.revision), (objects, 2));

    // The same tool to the project: six points on a layer the tool made, with the layer, one commit.
    let job = run_one(
        &db,
        &store,
        &access,
        ask(
            "b",
            "points.numberVertices",
            parcels_chosen.iter().take(2).cloned().collect(),
            ProcessingJobMode::Commit,
        ),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Succeeded, "{:?}", job.error);
    let result: ProcessingJobResult = serde_json::from_value(job.result.unwrap()).unwrap();
    assert_eq!((result.added, result.changed, result.removed), (6, 0, 0));
    assert!(result.metadata, "the new layer went with the points");
    assert_eq!(result.revision.as_deref(), Some("3"));
    let after = snapshot::snapshot(&db.app, &access).await.unwrap();
    assert_eq!((after.objects, after.revision), (objects + 6, 3));
    let read = kentos_kcad::decode(&after.bytes).unwrap();
    assert!(read.layers.iter().any(|l| l.id == "islem-kose-noktalari"));

    // The project moved on: a job pinned to the revision it read before is refused, and nothing is written.
    let pinned = ServerJobCreate {
        kind: PROJECT_PROCESSING_JOB.into(),
        input: serde_json::to_value(ProcessingJobInput {
            tool: "attributes.calculate".into(),
            selection: parcels_chosen,
            expected_revision: Some("2".into()),
            ..ProcessingJobInput::default()
        })
        .unwrap(),
        request_id: "d".into(),
        timeout_seconds: None,
    };
    let job = run_one(&db, &store, &access, pinned).await;
    assert_eq!(job.state, ServerJobState::Failed);
    assert_eq!(job.error.unwrap().code, "revision_conflict");
    db.close().await;
}
