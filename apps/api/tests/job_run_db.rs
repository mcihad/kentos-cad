//! A processing, import or atlas job through the whole chain (docs/adr/0559, 0561): a database project, the job asked
//! for as a user (an import's file staged first), a worker that runs the work in a real child process (`kentos-jobrun`),
//! and the result committed in the finish or staged as a file. Real database (`KENTOS_TEST_DB=required` makes a
//! missing one a failure).

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

/// A project of a drawing's objects, layers and sheets, owned by a project manager of the organisation "buro"; the
/// access, the objects' persistent ids and how many there are.
async fn project_of(
    db: &TestDb,
    name: &str,
    doc: &kentos_contracts::DocumentSnapshotV2,
) -> (
    kentos_application::access::ProjectAccess,
    Vec<String>,
    usize,
) {
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

    // A project of the drawing: its settings, layers and styles, each object under its persistent id.
    let created = projects::create(
        &db.app,
        &ayse,
        ProjectCreate {
            dimension_sets: Vec::new(),
            constraints: Default::default(),
            name: name.into(),
            settings: doc.settings.clone(),
            origin: doc.origin,
            home_view: doc.home_view,
            layers: unlocked(doc.layers.clone()),
            active_layer: doc.active_layer.clone(),
            styles: doc.styles.clone(),
            sheets: doc.sheets.clone(),
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
    (access, ids, objects)
}

#[tokio::test]
async fn a_tool_run_by_a_worker_in_a_child_commits_its_result_or_stages_its_file() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let doc = parcels();
    let (access, ids, objects) = project_of(&db, "Parseller", &doc).await;
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
    // What it took is said whatever its length (docs/adr/0561 §5).
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

// ── Imports and the atlas through the chain (docs/adr/0561) ─────────────────

const ATLAS: &[u8] = include_bytes!("../../../fixtures/server-jobs/v1/atlas.kcad");
const GPKG: &[u8] = include_bytes!("../../../fixtures/geopackage/v1/kentos-genis.gpkg");
const GPKG_WGS84: &[u8] =
    include_bytes!("../../../fixtures/geopackage/v1/kentos-synced-wgs84.gpkg");
const DXF_BLOCKS: &[u8] = include_bytes!("../../../fixtures/formats/v1/blocks.dxf");

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `bytes` staged as the caller's file for the project: asked for, sent, received.
async fn stage_file(
    db: &TestDb,
    store: &kentos_application::blobs::Blobs,
    access: &kentos_application::access::ProjectAccess,
    name: &str,
    bytes: &[u8],
) -> String {
    let begun = kentos_application::job_inputs::begin(
        &db.app,
        access,
        kentos_contracts::JobInputBegin {
            name: name.into(),
            size: bytes.len() as u64,
            sha256: sha256_hex(bytes),
        },
        1 << 30,
    )
    .await
    .unwrap();
    let mut receiving = kentos_application::job_inputs::start_receive(
        &db.app,
        store,
        access,
        Uuid::parse_str(&begun.id).unwrap(),
    )
    .await
    .unwrap();
    receiving.writer.write(bytes).await.unwrap();
    kentos_application::job_inputs::finish_receive(&db.app, store, access, receiving)
        .await
        .unwrap();
    begun.id
}

fn ask_import(request: &str, upload: &str, options: serde_json::Value) -> ServerJobCreate {
    ServerJobCreate {
        kind: kentos_contracts::PROJECT_IMPORT_JOB.into(),
        input: serde_json::json!({ "upload": upload, "options": options }),
        request_id: request.into(),
        timeout_seconds: None,
    }
}

#[tokio::test]
async fn a_file_staged_for_the_project_is_imported_by_the_child_and_committed_by_the_job() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let doc = parcels();
    let (access, _ids, objects) = project_of(&db, "Parseller", &doc).await;
    let store = kentos_application::blobs::Blobs::new(
        std::env::temp_dir().join(format!("kentos-job-run-blobs-{}", Uuid::now_v7())),
    );
    // A GeoPackage in the project's system: its three objects come in, one commit, and the command's words with them.
    let gpkg = stage_file(&db, &store, &access, "kentos-genis.gpkg", GPKG).await;
    let job = run_one(
        &db,
        &store,
        &access,
        ask_import("a", &gpkg, serde_json::json!({})),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Succeeded, "{:?}", job.error);
    let result: kentos_contracts::ProjectImportResult =
        serde_json::from_value(job.result.unwrap()).unwrap();
    assert_eq!(result.command, "file.import.gpkg");
    assert_eq!(
        (result.file.as_str(), result.bytes, result.sha256.as_str()),
        (
            "kentos-genis.gpkg",
            GPKG.len() as u64,
            sha256_hex(GPKG).as_str()
        )
    );
    assert_eq!((result.added, result.changed, result.removed), (3, 0, 0));
    assert_eq!(
        (result.base_revision.as_str(), result.revision.as_deref()),
        ("1", Some("2"))
    );
    assert!(
        result.summary.contains("3 nesne eklendi"),
        "{}",
        result.summary
    );
    assert_eq!(result.details["objects"], 3);
    let after = snapshot::snapshot(&db.app, &access).await.unwrap();
    assert_eq!((after.objects, after.revision), (objects + 3, 2));

    // A file in another coordinate system: the command's refusal is the job's code, and nothing was written.
    let wgs84 = stage_file(&db, &store, &access, "wgs84.gpkg", GPKG_WGS84).await;
    let job = run_one(
        &db,
        &store,
        &access,
        ask_import("b", &wgs84, serde_json::json!({})),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Failed);
    assert_eq!(job.error.unwrap().code, "crs_mismatch");
    assert_eq!(
        snapshot::snapshot(&db.app, &access).await.unwrap().revision,
        2
    );
    // Asked to move its coordinates into the project's, it comes in.
    let job = run_one(
        &db,
        &store,
        &access,
        ask_import("c", &wgs84, serde_json::json!({ "reproject": true })),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Succeeded, "{:?}", job.error);
    assert_eq!(
        snapshot::snapshot(&db.app, &access).await.unwrap().revision,
        3
    );

    // A DXF with blocks: its block definitions are committed with its objects.
    let dxf = stage_file(&db, &store, &access, "bloklar.dxf", DXF_BLOCKS).await;
    let job = run_one(
        &db,
        &store,
        &access,
        ask_import("d", &dxf, serde_json::json!({})),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Succeeded, "{:?}", job.error);
    let result: kentos_contracts::ProjectImportResult =
        serde_json::from_value(job.result.unwrap()).unwrap();
    assert!(result.blocks > 0, "{result:?}");
    let snap = snapshot::snapshot(&db.app, &access).await.unwrap();
    assert_eq!(snap.revision, 4);
    assert_eq!(
        kentos_kcad::decode(&snap.bytes).unwrap().blocks.len() as u64,
        result.blocks
    );
    db.close().await;
}

#[tokio::test]
async fn the_atlas_of_a_sheet_is_drawn_by_the_child_and_downloaded_as_the_jobs_pdf() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let doc = kentos_kcad::decode(ATLAS).expect("the atlas project reads");
    let (access, _ids, objects) = project_of(&db, "Parsel atlası", &doc).await;
    let store = kentos_application::blobs::Blobs::new(
        std::env::temp_dir().join(format!("kentos-job-run-blobs-{}", Uuid::now_v7())),
    );
    let ask = |request: &str, input: serde_json::Value| ServerJobCreate {
        kind: kentos_contracts::PROJECT_ATLAS_JOB.into(),
        input,
        request_id: request.into(),
        timeout_seconds: None,
    };
    // The pin a plan of the same drawing made (here by the Python SDK, from the file): the project the server keeps is
    // the same state, so its pages are made.
    let good: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/server-jobs/v1/atlas-pin.json"
    ))
    .unwrap();
    let job = run_one(
        &db,
        &store,
        &access,
        ask(
            "a",
            serde_json::json!({ "sheet": "atlas-paftasi", "pin": good }),
        ),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Succeeded, "{:?}", job.error);
    let result: kentos_contracts::ProjectAtlasResult =
        serde_json::from_value(job.result.clone().unwrap()).unwrap();
    assert_eq!(result.pages, 6);
    assert_eq!(result.first.as_deref(), Some("A1/1"));
    assert_eq!(result.sheet, "atlas-paftasi");
    assert_eq!(result.pin_digest.len(), 64);
    assert!(
        result.withheld.iter().any(|f| f == "Malik"),
        "{:?}",
        result.withheld
    );
    let artifact = job.artifact.clone().expect("a PDF");
    assert_eq!(artifact.name, "Parsel atlası - atlas-paftasi.pdf");
    let (_, mut file) = jobs::artifact(&db.app, &store, &access, Uuid::parse_str(&job.id).unwrap())
        .await
        .unwrap();
    let mut pdf = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut pdf)
        .await
        .unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(pdf.len().to_string(), artifact.size);
    // Nothing of the project changed.
    let after = snapshot::snapshot(&db.app, &access).await.unwrap();
    assert_eq!((after.objects, after.revision), (objects, 1));
    // A pin that is not the project's state: the command refuses it by its code and writes nothing.
    let mut pin: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/server-jobs/v1/atlas-pin.json"
    ))
    .unwrap();
    pin["digest"] = serde_json::json!("0".repeat(64));
    pin["layers"] = serde_json::json!("0".repeat(64));
    let job = run_one(
        &db,
        &store,
        &access,
        ask(
            "b",
            serde_json::json!({ "sheet": "atlas-paftasi", "pin": pin }),
        ),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Failed);
    assert_eq!(job.error.unwrap().code, "pin_mismatch");
    // A sheet that is not there.
    let job = run_one(
        &db,
        &store,
        &access,
        ask("c", serde_json::json!({ "sheet": "yok" })),
    )
    .await;
    assert_eq!(job.error.unwrap().code, "unknown_sheet");
    db.close().await;
}

const SHP_ZIP: &[u8] = include_bytes!("../../../fixtures/gis-import/v1/katmanlar.zip");
const GEOJSON: &[u8] = include_bytes!("../../../fixtures/gis-import/v1/katmanli.geojson");
const KML_WGS84: &[u8] = include_bytes!("../../../fixtures/gis-import/v1/nokta.kml");
const QGIS_GPKG: &[u8] = include_bytes!("../../../fixtures/geopackage/v1/qgis.gpkg");

/// A Shapefile archive, a GeoJSON file and a KML are taken in by their commands and committed by the job
/// (docs/adr/0563 §3); a GeoPackage with a circle, an area of two corners whose edges are arcs (docs/adr/0069), is
/// committed too, and its PostGIS projection is a valid polygon (§4: the server's rule is the desktop's).
#[tokio::test]
async fn shapefile_geojson_and_kml_files_and_a_circle_are_committed_by_the_job() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let doc = parcels();
    let (access, _ids, objects) = project_of(&db, "Parseller", &doc).await;
    let store = kentos_application::blobs::Blobs::new(
        std::env::temp_dir().join(format!("kentos-job-run-blobs-{}", Uuid::now_v7())),
    );
    let mut revision = 1;
    let mut count = objects as u64;
    for (request, name, bytes, options, added, command) in [
        (
            "a",
            "katmanlar.zip",
            SHP_ZIP,
            serde_json::json!({}),
            5,
            "file.import.shp",
        ),
        (
            "b",
            "katmanli.geojson",
            GEOJSON,
            serde_json::json!({}),
            3,
            "file.import.geojson",
        ),
        (
            "c",
            "nokta.kml",
            KML_WGS84,
            serde_json::json!({ "reproject": true }),
            1,
            "file.import.kml",
        ),
    ] {
        let upload = stage_file(&db, &store, &access, name, bytes).await;
        let job = run_one(&db, &store, &access, ask_import(request, &upload, options)).await;
        assert_eq!(
            job.state,
            ServerJobState::Succeeded,
            "{name}: {:?}",
            job.error
        );
        let result: kentos_contracts::ProjectImportResult =
            serde_json::from_value(job.result.unwrap()).unwrap();
        assert_eq!(result.command, command);
        assert_eq!(result.added, added, "{name}: {result:?}");
        revision += 1;
        count += added;
        let after = snapshot::snapshot(&db.app, &access).await.unwrap();
        assert_eq!(
            (after.objects as u64, after.revision),
            (count, revision),
            "{name}"
        );
    }
    // WGS 84 without the move: the command's refusal is the job's code, nothing is written.
    let upload = stage_file(&db, &store, &access, "nokta.kml", KML_WGS84).await;
    let job = run_one(
        &db,
        &store,
        &access,
        ask_import("d", &upload, serde_json::json!({})),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Failed);
    assert_eq!(job.error.unwrap().code, "crs_mismatch");
    assert_eq!(
        snapshot::snapshot(&db.app, &access).await.unwrap().revision,
        revision
    );
    // The QGIS GeoPackage's circle (two corners, two arcs): committed, and PostGIS holds a valid polygon of it.
    let upload = stage_file(&db, &store, &access, "qgis.gpkg", QGIS_GPKG).await;
    let job = run_one(
        &db,
        &store,
        &access,
        ask_import("e", &upload, serde_json::json!({ "tables": ["egri"] })),
    )
    .await;
    assert_eq!(job.state, ServerJobState::Succeeded, "{:?}", job.error);
    let (valid, points): (bool, i32) = sqlx::query_as(
        "select bool_and(st_isvalid(geom)), max(st_npoints(geom)) from kentos.feature \
         where project_id = $1 and kind = 'polygon' and source_kind = 'cad'",
    )
    .bind(access.project)
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert!(valid, "the circle's projection is a valid polygon");
    assert!(points > 8, "its arcs have their inner points: {points}");
    db.close().await;
}
