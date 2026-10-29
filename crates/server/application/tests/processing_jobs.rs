//! An İşlemler tool as a durable server job (docs/adr/0559): the worker reads the project at a revision, the host's
//! processor (a fake here; the real one is `kentosd`'s child process, tested in apps/api) answers with a delta or a
//! file, and the result is committed in the finish transaction under the revision the tool read. Real database
//! (`KENTOS_TEST_DB=required` makes a missing one a failure).
//!
//! - the result is committed once, the project's revision moves by one, the job's result says which; the worker's
//!   progress and words are read by polling while the tool runs;
//! - a project that moved while the tool ran, a lease lost at the end, a grant taken away: nothing is written;
//! - a pinned revision that is not the project's is refused before the tool runs; a budget the file cannot fit
//!   (`memory_estimate`) too; a stop stops the tool; a tool's own failure codes become the job's;
//! - a result to a file is staged as the artifact and the project does not change; a worker with no processor leaves
//!   the job queued; a request is checked when it is made (no local file, rights by mode).

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use common::{a_point, blobs, envelope, member, new_project, open, revoke, share};
use kentos_application::jobs::{self, JobLimits, Worked};
use kentos_application::processing_job::{
    Delta, ProcessFailure, ProcessHooks, Processed, ProcessingTask, Processor,
};
use kentos_application::{AppError, changes, snapshot};
use kentos_contracts::{
    Entity, FeatureChange, GrantRole, PROJECT_PROCESSING_JOB, ProcessingJobInput,
    ProcessingJobMode, ProcessingJobResult, ProcessingUsage, ServerJob, ServerJobCreate,
    ServerJobState, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

const LEASE: Duration = Duration::from_secs(3);

type Run =
    dyn Fn(&ProcessingTask, &ProcessHooks) -> Result<Processed, ProcessFailure> + Send + Sync;

/// A processor that does what the test says and counts how often it was asked.
struct Fake {
    run: Box<Run>,
    calls: AtomicUsize,
}

impl Fake {
    fn new(
        run: impl Fn(&ProcessingTask, &ProcessHooks) -> Result<Processed, ProcessFailure>
        + Send
        + Sync
        + 'static,
    ) -> Arc<Self> {
        Arc::new(Fake {
            run: Box::new(run),
            calls: AtomicUsize::new(0),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Processor for Fake {
    fn process(
        &self,
        task: &ProcessingTask,
        hooks: &ProcessHooks,
    ) -> Result<Processed, ProcessFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        (self.run)(task, hooks)
    }
}

fn tool(mode: ProcessingJobMode) -> ProcessingJobInput {
    ProcessingJobInput {
        tool: "geometry.buffer".into(),
        label: Some("Tampon".into()),
        mode,
        ..ProcessingJobInput::default()
    }
}

fn ask(request: &str, input: ProcessingJobInput) -> ServerJobCreate {
    ServerJobCreate {
        kind: PROJECT_PROCESSING_JOB.into(),
        input: serde_json::to_value(input).unwrap(),
        request_id: request.into(),
        timeout_seconds: None,
        fresh: false,
    }
}

async fn create(
    db: &TestDb,
    access: &kentos_application::access::ProjectAccess,
    asked: ServerJobCreate,
) -> Result<ServerJob, AppError> {
    jobs::create(&db.app, access, asked, &JobLimits::default()).await
}

async fn work(
    db: &TestDb,
    store: &kentos_application::blobs::Blobs,
    processor: &Arc<Fake>,
) -> Option<Worked> {
    jobs::work_once_with(
        &db.app,
        store,
        Uuid::now_v7(),
        LEASE,
        &JobLimits::default(),
        Some(processor.clone() as Arc<dyn Processor>),
    )
    .await
    .unwrap()
}

fn id_of(job: &ServerJob) -> Uuid {
    Uuid::parse_str(&job.id).unwrap()
}

fn point(x: f64) -> Entity {
    serde_json::from_value(json!({
        "kind": "point", "id": 0, "layerId": "cizim", "attrs": {}, "p": { "x": x, "y": 4_420_210.0 }
    }))
    .unwrap()
}

/// A delta that creates one point.
fn creates(x: f64) -> Delta {
    Delta {
        features: a_point(x).features,
        added: 1,
        ..Delta::default()
    }
}

fn answer(delta: Delta) -> Processed {
    Processed {
        summary: "1 nesne eklendi".into(),
        delta: Some(delta),
        usage: ProcessingUsage {
            cpu_seconds: 0.5,
            peak_memory_mb: 80.0,
        },
        ..Processed::default()
    }
}

/// How many objects the project holds and the revision it is at.
async fn standing(db: &TestDb, access: &kentos_application::access::ProjectAccess) -> (usize, i64) {
    let snap = snapshot::snapshot(&db.app, access).await.unwrap();
    (snap.objects, snap.revision)
}

/// A project of one point (revision 1) with its owner, and the id of that point.
async fn project_of_one(
    db: &TestDb,
) -> (
    kentos_application::tenancy::Access,
    kentos_application::access::ProjectAccess,
    Uuid,
    String,
) {
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(db, &pm, "Araç işi").await;
    let owner = open(db, &pm, project).await;
    let done = changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486_500.0), &[]),
    )
    .await
    .unwrap();
    let id = done.versions.keys().next().unwrap().clone();
    (pm, owner, project, id)
}

#[tokio::test]
async fn a_tool_result_is_committed_with_the_job_and_its_progress_is_read_meanwhile() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, _project, existing) = project_of_one(&db).await;
    let store = blobs();
    assert_eq!(standing(&db, &owner).await, (1, 1));
    let moved = existing.clone();
    let processor = Fake::new(move |task, hooks| {
        assert_eq!(task.tool().unwrap().tool, "geometry.buffer");
        hooks.progress.set(0.5, "Yarı yolda");
        std::thread::sleep(Duration::from_millis(2600));
        let mut delta = creates(486_900.0);
        delta.features.push(FeatureChange::Update {
            id: moved.clone(),
            entity: point(486_550.0),
        });
        delta.changed = 1;
        Ok(answer(delta))
    });
    let job = create(&db, &owner, ask("a", tool(ProcessingJobMode::Commit)))
        .await
        .unwrap();
    assert_eq!(job.kind, PROJECT_PROCESSING_JOB);
    assert_eq!(job.progress_text, None);
    let worked = work(&db, &store, &processor);
    // The worker writes what the tool said at its beats; a poll sees it while the tool still runs.
    let poll = async {
        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
            if job.progress_text.as_deref() == Some("Yarı yolda") {
                assert_eq!(job.state, ServerJobState::Running);
                assert!((job.progress - 0.5).abs() < 1e-6, "{}", job.progress);
                return true;
            }
        }
        false
    };
    let (worked, saw) = tokio::join!(worked, poll);
    assert!(saw, "the progress words were read while the tool ran");
    assert_eq!(
        worked,
        Some(Worked {
            job: id_of(&job),
            state: ServerJobState::Succeeded
        })
    );
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    assert_eq!(job.state, ServerJobState::Succeeded);
    assert_eq!(job.progress, 1.0);
    assert_eq!(job.attempts, 1);
    assert!(job.artifact.is_none());
    let result: ProcessingJobResult = serde_json::from_value(job.result.clone().unwrap()).unwrap();
    assert_eq!(
        (result.base_revision.as_str(), result.revision.as_deref()),
        ("1", Some("2"))
    );
    assert_eq!((result.added, result.changed, result.removed), (1, 1, 0));
    assert_eq!(result.summary, "1 nesne eklendi");
    assert_eq!(result.usage.cpu_seconds, 0.5);
    // One commit: the revision moved by one, the created object is there, the changed one changed.
    assert_eq!(standing(&db, &owner).await, (2, 2));
    assert_eq!(processor.calls(), 1);
    db.close().await;
}

#[tokio::test]
async fn a_project_that_moved_while_the_tool_ran_takes_nothing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (pm, owner, project, _) = project_of_one(&db).await;
    let store = blobs();
    let handle = tokio::runtime::Handle::current();
    let (app, who, as_owner) = (db.app.clone(), pm.clone(), owner.clone());
    // Someone else's commit lands while the tool runs.
    let processor = Fake::new(move |_, _| {
        handle
            .block_on(changes::commit(
                &app,
                &as_owner,
                envelope(&who, project, a_point(487_000.0), &[]),
            ))
            .unwrap();
        Ok(answer(creates(486_900.0)))
    });
    let job = create(&db, &owner, ask("a", tool(ProcessingJobMode::Commit)))
        .await
        .unwrap();
    let worked = work(&db, &store, &processor).await.unwrap();
    assert_eq!(worked.state, ServerJobState::Failed);
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    let error = job.error.unwrap();
    assert_eq!(error.code, "revision_conflict");
    assert!(
        error.message.contains("revizyonda değil"),
        "{}",
        error.message
    );
    // Only the other commit is in the project; the tool's object is not.
    assert_eq!(standing(&db, &owner).await, (2, 2));
    db.close().await;
}

#[tokio::test]
async fn a_lost_lease_at_the_end_takes_the_commit_back_with_it() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, _project, _) = project_of_one(&db).await;
    let store = blobs();
    let processor = Fake::new(|_, _| Ok(answer(creates(486_900.0))));
    let job = create(&db, &owner, ask("a", tool(ProcessingJobMode::Commit)))
        .await
        .unwrap();
    let id = id_of(&job);
    // The lease is the table owner's to move: the app's role sees a project's rows only inside its scope.
    let pool = db.owner.clone();
    let worked = jobs::work_once_between_with(
        &db.app,
        &store,
        Uuid::now_v7(),
        LEASE,
        &JobLimits::default(),
        Some(processor.clone() as Arc<dyn Processor>),
        || async move {
            // The lease runs out between the tool's end and the finish.
            sqlx::query("update kentos.server_job set lease_until = now() - interval '1 second' where id = $1")
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        worked.state,
        ServerJobState::Running,
        "the finish did not pass"
    );
    assert_eq!(
        standing(&db, &owner).await,
        (1, 1),
        "the commit went with it"
    );
    assert_eq!(
        jobs::get(&db.app, &owner, id).await.unwrap().state,
        ServerJobState::Running
    );
    db.close().await;
}

#[tokio::test]
async fn a_grant_taken_away_while_the_tool_ran_refuses_the_commit() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, project, _) = project_of_one(&db).await;
    let ed = member(&db, "buro", "duzenleyici", TenantRole::Viewer).await;
    share(&db, &owner, &ed.actor, GrantRole::Editor).await;
    let editor = open(&db, &ed, project).await;
    let store = blobs();
    let processor = Fake::new(|_, _| Ok(answer(creates(486_900.0))));
    let job = create(&db, &editor, ask("a", tool(ProcessingJobMode::Commit)))
        .await
        .unwrap();
    let (by, who) = (owner.clone(), ed.actor.clone());
    let done = jobs::work_once_between_with(
        &db.app,
        &store,
        Uuid::now_v7(),
        LEASE,
        &JobLimits::default(),
        Some(processor.clone() as Arc<dyn Processor>),
        || async {
            // The editor's grant goes after the tool ran, before the commit.
            let _ = revoke_now(&db, &by, &who).await;
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(done.state, ServerJobState::Failed);
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    assert_eq!(job.error.unwrap().code, "forbidden");
    assert_eq!(standing(&db, &owner).await, (1, 1));
    db.close().await;
}

async fn revoke_now(
    db: &TestDb,
    by: &kentos_application::access::ProjectAccess,
    to: &kentos_application::identity::Actor,
) {
    revoke(db, by, to).await;
}

#[tokio::test]
async fn a_pinned_revision_or_a_budget_the_file_cannot_fit_is_refused_before_the_tool_runs() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, _project, _) = project_of_one(&db).await;
    let store = blobs();
    let processor = Fake::new(|_, _| Ok(answer(creates(486_900.0))));
    // Pinned to a revision the project is not at.
    let pinned = create(
        &db,
        &owner,
        ask(
            "a",
            ProcessingJobInput {
                expected_revision: Some("0".into()),
                ..tool(ProcessingJobMode::Commit)
            },
        ),
    )
    .await
    .unwrap();
    let worked = work(&db, &store, &processor).await.unwrap();
    assert_eq!(worked.state, ServerJobState::Failed);
    let job = jobs::get(&db.app, &owner, id_of(&pinned)).await.unwrap();
    assert_eq!(job.error.unwrap().code, "revision_conflict");
    // The estimate (a base of 192 MB and more) is past a budget of 64 MB.
    let small = create(
        &db,
        &owner,
        ask(
            "b",
            ProcessingJobInput {
                memory_mb: Some(64),
                ..tool(ProcessingJobMode::Commit)
            },
        ),
    )
    .await
    .unwrap();
    work(&db, &store, &processor).await.unwrap();
    let job = jobs::get(&db.app, &owner, id_of(&small)).await.unwrap();
    let error = job.error.unwrap();
    assert_eq!(error.code, "memory_estimate");
    assert!(error.message.contains("64 MB"), "{}", error.message);
    assert_eq!(processor.calls(), 0, "the tool never ran");
    assert_eq!(standing(&db, &owner).await, (1, 1));
    db.close().await;
}

#[tokio::test]
async fn a_tools_own_failure_codes_are_the_jobs_and_it_is_not_tried_again() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, _project, _) = project_of_one(&db).await;
    let store = blobs();
    for (k, code) in [
        "cpu_limit",
        "memory_limit",
        "tool_failed",
        "unsupported_change",
        "too_many_changes",
        "crashed",
        "tool_unavailable",
    ]
    .into_iter()
    .enumerate()
    {
        let processor =
            Fake::new(move |_, _| Err(ProcessFailure::new(code, format!("{code}: söz"))));
        let job = create(
            &db,
            &owner,
            ask(&format!("r{k}"), tool(ProcessingJobMode::Commit)),
        )
        .await
        .unwrap();
        let worked = work(&db, &store, &processor).await.unwrap();
        assert_eq!(worked.state, ServerJobState::Failed, "{code}");
        let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
        assert_eq!(job.attempts, 1, "{code}: no second attempt");
        let error = job.error.unwrap();
        assert_eq!(error.code, code);
        assert!(error.message.starts_with(code), "{}", error.message);
    }
    assert_eq!(standing(&db, &owner).await, (1, 1));
    db.close().await;
}

#[tokio::test]
async fn a_stop_stops_the_tool_and_the_job_is_cancelled() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, _project, _) = project_of_one(&db).await;
    let store = blobs();
    let started = Arc::new(AtomicBool::new(false));
    let flag = started.clone();
    let processor = Fake::new(move |_, hooks| {
        flag.store(true, Ordering::SeqCst);
        for _ in 0..200 {
            if hooks.stop.load(Ordering::SeqCst) {
                return Err(ProcessFailure::new("stopped", "durduruldu"));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(answer(creates(486_900.0)))
    });
    let job = create(&db, &owner, ask("a", tool(ProcessingJobMode::Commit)))
        .await
        .unwrap();
    let id = id_of(&job);
    let stopper = async {
        while !started.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        jobs::cancel(&db.app, &owner, id).await.unwrap();
    };
    let (worked, ()) = tokio::join!(work(&db, &store, &processor), stopper);
    assert_eq!(
        worked.unwrap().state,
        ServerJobState::Running,
        "no finish after a stop"
    );
    let job = jobs::get(&db.app, &owner, id).await.unwrap();
    assert_eq!(job.state, ServerJobState::Cancelled);
    assert_eq!(standing(&db, &owner).await, (1, 1));
    db.close().await;
}

#[tokio::test]
async fn a_result_to_a_file_is_staged_as_the_artifact_and_the_project_does_not_change() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, _project, _) = project_of_one(&db).await;
    let store = blobs();
    let processor = Fake::new(|_, _| {
        Ok(Processed {
            summary: "3 satır yazıldı".into(),
            file: Some(b"GPKG-bytes".to_vec()),
            rows: 3,
            ..Processed::default()
        })
    });
    let job = create(&db, &owner, ask("a", tool(ProcessingJobMode::Sink)))
        .await
        .unwrap();
    let worked = work(&db, &store, &processor).await.unwrap();
    assert_eq!(worked.state, ServerJobState::Succeeded);
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    let artifact = job.artifact.clone().expect("an artifact");
    assert_eq!(artifact.size, "10");
    assert_eq!(artifact.name, "Araç işi - Tampon.gpkg");
    let (_, mut file) = jobs::artifact(&db.app, &store, &owner, id_of(&job))
        .await
        .unwrap();
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
        .await
        .unwrap();
    assert_eq!(bytes, b"GPKG-bytes");
    let result: ProcessingJobResult = serde_json::from_value(job.result.unwrap()).unwrap();
    assert_eq!((result.added, result.revision), (3, None));
    assert_eq!(standing(&db, &owner).await, (1, 1));
    db.close().await;
}

#[tokio::test]
async fn a_worker_without_a_processor_leaves_the_job_and_a_request_is_checked_when_made() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, project, _) = project_of_one(&db).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    share(&db, &owner, &viewer.actor, GrantRole::Viewer).await;
    let viewer = open(&db, &viewer, project).await;
    let store = blobs();
    // No processor: the kind is not claimed at all.
    let job = create(&db, &owner, ask("a", tool(ProcessingJobMode::Commit)))
        .await
        .unwrap();
    assert!(
        jobs::work_once(
            &db.app,
            &store,
            Uuid::now_v7(),
            LEASE,
            &JobLimits::default()
        )
        .await
        .unwrap()
        .is_none()
    );
    assert_eq!(
        jobs::get(&db.app, &owner, id_of(&job)).await.unwrap().state,
        ServerJobState::Queued
    );
    // A local file is not a thing a server job reads; an empty tool, a bad uid and a bad box are named by their path.
    for (input, path) in [
        (
            json!({ "tool": "x", "parameters": { "dosya": { "path": "/etc/passwd" } } }),
            "input.parameters.dosya.path",
        ),
        (
            json!({ "tool": "x", "parameters": { "a": [{ "b": { "path": "x" } }] } }),
            "input.parameters.a[0].b.path",
        ),
        (json!({ "tool": "  " }), "input.tool"),
        (
            json!({ "tool": "x", "selection": ["yok"] }),
            "input.selection[0]",
        ),
        (
            json!({ "tool": "x", "view": [5.0, 0.0, 1.0, 1.0] }),
            "input.view",
        ),
        (
            json!({ "tool": "x", "expectedRevision": "bir" }),
            "input.expectedRevision",
        ),
        (json!({ "tool": "x", "cpuSeconds": 0 }), "input.cpuSeconds"),
        (json!({ "tool": "x", "memoryMb": 8 }), "input.memoryMb"),
    ] {
        let refused = jobs::create(
            &db.app,
            &owner,
            ServerJobCreate {
                kind: PROJECT_PROCESSING_JOB.into(),
                input,
                request_id: format!("x-{path}"),
                timeout_seconds: None,
                fresh: false,
            },
            &JobLimits::default(),
        )
        .await
        .unwrap_err();
        let AppError::Invalid { path: at, .. } = &refused else {
            panic!("{refused:?}");
        };
        assert_eq!(at.as_deref(), Some(path), "{refused}");
    }
    // A reader runs no job at all (`project.jobs.run` is the editor's), whatever the mode.
    for (request, mode) in [
        ("v1", ProcessingJobMode::Commit),
        ("v2", ProcessingJobMode::Sink),
    ] {
        assert!(
            matches!(
                create(&db, &viewer, ask(request, tool(mode))).await,
                Err(AppError::Forbidden(_))
            ),
            "{mode:?}"
        );
    }
    db.close().await;
}

#[tokio::test]
async fn a_result_that_changes_the_projects_metadata_needs_the_right_to_edit_it() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, project, _) = project_of_one(&db).await;
    let ed = member(&db, "buro", "duzenleyici", TenantRole::Viewer).await;
    share(&db, &owner, &ed.actor, GrantRole::Editor).await;
    let editor = open(&db, &ed, project).await;
    let store = blobs();
    // The tool made a point and set the project's active layer: the objects are the editor's to write, the
    // metadata is not.
    let processor = Fake::new(|_, _| {
        let mut delta = creates(486_900.0);
        delta.project = Some(kentos_contracts::ProjectPatch {
            active_layer: Some("cizim".into()),
            ..Default::default()
        });
        Ok(answer(delta))
    });
    let theirs = create(&db, &editor, ask("e", tool(ProcessingJobMode::Commit)))
        .await
        .unwrap();
    assert_eq!(
        work(&db, &store, &processor).await.unwrap().state,
        ServerJobState::Failed
    );
    let job = jobs::get(&db.app, &owner, id_of(&theirs)).await.unwrap();
    assert_eq!(job.error.unwrap().code, "forbidden");
    assert_eq!(standing(&db, &owner).await, (1, 1));
    // The owner's job commits both in one revision.
    let mine = create(&db, &owner, ask("o", tool(ProcessingJobMode::Commit)))
        .await
        .unwrap();
    assert_eq!(
        work(&db, &store, &processor).await.unwrap().state,
        ServerJobState::Succeeded
    );
    let job = jobs::get(&db.app, &owner, id_of(&mine)).await.unwrap();
    let result: ProcessingJobResult = serde_json::from_value(job.result.unwrap()).unwrap();
    assert_eq!(
        (result.metadata, result.revision.as_deref()),
        (true, Some("2"))
    );
    assert_eq!(standing(&db, &owner).await, (2, 2));
    db.close().await;
}

// ── A file import and an atlas (docs/adr/0561) ──────────────────────────────

use kentos_application::job_inputs;
use kentos_application::processing_job::RunRequest;
use kentos_contracts::{
    BlockChange, BlockDefinition, ImportFormat, JobInput, JobInputBegin, PROJECT_ATLAS_JOB,
    PROJECT_IMPORT_JOB, ProjectAtlasInput, ProjectAtlasResult, ProjectImportInput,
    ProjectImportResult,
};
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Stages `bytes` as the caller's file for the project: asked for, sent, received.
async fn stage_file(
    db: &TestDb,
    store: &kentos_application::blobs::Blobs,
    access: &kentos_application::access::ProjectAccess,
    name: &str,
    bytes: &[u8],
) -> JobInput {
    let begun = job_inputs::begin(
        &db.app,
        access,
        JobInputBegin {
            name: name.into(),
            size: bytes.len() as u64,
            sha256: sha256_hex(bytes),
        },
        1 << 30,
    )
    .await
    .unwrap();
    let mut receiving =
        job_inputs::start_receive(&db.app, store, access, Uuid::parse_str(&begun.id).unwrap())
            .await
            .unwrap();
    receiving.writer.write(bytes).await.unwrap();
    job_inputs::finish_receive(&db.app, store, access, receiving)
        .await
        .unwrap()
}

fn ask_import(request: &str, upload: &str) -> ServerJobCreate {
    ServerJobCreate {
        kind: PROJECT_IMPORT_JOB.into(),
        input: serde_json::to_value(ProjectImportInput {
            upload: upload.into(),
            ..ProjectImportInput::default()
        })
        .unwrap(),
        request_id: request.into(),
        timeout_seconds: None,
        fresh: false,
    }
}

fn ask_atlas(request: &str, input: ProjectAtlasInput) -> ServerJobCreate {
    ServerJobCreate {
        kind: PROJECT_ATLAS_JOB.into(),
        input: serde_json::to_value(input).unwrap(),
        request_id: request.into(),
        timeout_seconds: None,
        fresh: false,
    }
}

fn atlas_of(sheet: &str) -> ProjectAtlasInput {
    ProjectAtlasInput {
        sheet: sheet.into(),
        ..ProjectAtlasInput::default()
    }
}

#[tokio::test]
async fn a_staged_file_is_taken_in_by_the_worker_and_its_objects_are_committed_with_the_job() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, _project, _) = project_of_one(&db).await;
    let store = blobs();
    let bytes = b"not a real gpkg: the fake reads only that it is the file".to_vec();
    let file = stage_file(&db, &store, &owner, "parsel.gpkg", &bytes).await;
    assert!(file.received);
    let want = bytes.clone();
    let processor = Fake::new(move |task, hooks| {
        let RunRequest::Import(run) = &task.request else {
            panic!("an import: {:?}", task.request);
        };
        assert_eq!(
            (run.format, run.name.as_str()),
            (ImportFormat::Gpkg, "parsel.gpkg")
        );
        assert_eq!(run.options["reproject"], true);
        // The processor is handed the file where the store keeps it, not its bytes.
        let staged = task.file.as_ref().expect("the staged file");
        assert_eq!(std::fs::read(&staged.path).unwrap(), want);
        assert_eq!(
            (staged.name.as_str(), staged.size),
            ("parsel.gpkg", want.len() as u64)
        );
        hooks.progress.set(0.6, "Dosya okunuyor");
        let mut done = answer(creates(486_900.0));
        done.summary = "“parsel.gpkg” içe aktarıldı: 1 nesne eklendi.".into();
        done.notes = vec![kentos_contracts::ProcessingJobNote {
            level: "warn".into(),
            text: "import_losses: 2 satırın geometrisi okunamadı".into(),
        }];
        done.details = json!({ "objects": 1, "tables": [{ "table": "parsel", "objects": 1 }] });
        Ok(done)
    });
    let mut asked = ask_import("i1", &file.id);
    asked.input["options"] = json!({ "reproject": true });
    let job = create(&db, &owner, asked).await.unwrap();
    assert_eq!(job.kind, PROJECT_IMPORT_JOB);
    let worked = work(&db, &store, &processor).await.unwrap();
    assert_eq!(worked.state, ServerJobState::Succeeded);
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    assert!(job.artifact.is_none(), "an import leaves no file");
    let result: ProjectImportResult = serde_json::from_value(job.result.unwrap()).unwrap();
    assert_eq!(result.command, "file.import.gpkg");
    assert_eq!(
        (result.file.as_str(), result.bytes, result.sha256.as_str()),
        (
            "parsel.gpkg",
            bytes.len() as u64,
            sha256_hex(&bytes).as_str()
        )
    );
    assert_eq!(
        (result.base_revision.as_str(), result.revision.as_deref()),
        ("1", Some("2"))
    );
    assert_eq!((result.added, result.changed, result.removed), (1, 0, 0));
    assert_eq!(result.notes.len(), 1, "the losses are in the result");
    assert!(result.notes[0].text.starts_with("import_losses"));
    assert_eq!(result.details["tables"][0]["table"], "parsel");
    assert_eq!(standing(&db, &owner).await, (2, 2));
    db.close().await;
}

#[tokio::test]
async fn an_import_bigger_than_a_request_may_carry_is_one_commit_of_the_job() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (pm, owner, project, _) = project_of_one(&db).await;
    let store = blobs();
    let file = stage_file(&db, &store, &owner, "noktalar.csv", b"X,Y\n1,2\n").await;
    // 6 001 points: past what `project.changes` takes from a request (5 000), within what a job's own commit takes.
    let processor = Fake::new(|_, _| {
        let mut delta = Delta::default();
        for i in 0..6001 {
            delta
                .features
                .extend(a_point(486_000.0 + f64::from(i)).features);
        }
        delta.added = 6001;
        Ok(answer(delta))
    });
    let job = create(&db, &owner, ask_import("i1", &file.id))
        .await
        .unwrap();
    assert_eq!(
        work(&db, &store, &processor).await.unwrap().state,
        ServerJobState::Succeeded
    );
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    let result: ProjectImportResult = serde_json::from_value(job.result.unwrap()).unwrap();
    assert_eq!(
        (result.added, result.revision.as_deref()),
        (6001, Some("2"))
    );
    assert_eq!(standing(&db, &owner).await, (6002, 2));
    // The same set as a request is refused.
    let refused = changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            {
                let mut all = a_point(1.0);
                for i in 0..5000 {
                    all.features.extend(a_point(2.0 + f64::from(i)).features);
                }
                all
            },
            &[],
        ),
    )
    .await
    .unwrap_err();
    assert!(refused.to_string().contains("en çok 5000"), "{refused}");
    db.close().await;
}

fn a_block(id: u128, name: &str) -> BlockDefinition {
    serde_json::from_value(json!({
        "id": Uuid::from_u128(id).to_string(), "name": name, "base": { "x": 0.0, "y": 0.0 },
        "entities": [{ "kind": "point", "id": 1, "layerId": "0", "attrs": {}, "p": { "x": 0.0, "y": 0.0 } }]
    }))
    .unwrap()
}

/// How many block definitions the project holds, by the file the server hands over.
async fn block_names(
    db: &TestDb,
    access: &kentos_application::access::ProjectAccess,
) -> Vec<String> {
    let snap = snapshot::snapshot(&db.app, access).await.unwrap();
    let mut names: Vec<String> = kentos_kcad::decode(&snap.bytes)
        .unwrap()
        .blocks
        .into_iter()
        .map(|b| b.name)
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn an_import_brings_block_definitions_and_a_later_job_replaces_and_removes_them() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, _project, _) = project_of_one(&db).await;
    let store = blobs();
    let file = stage_file(&db, &store, &owner, "bloklar.dxf", b"0\nEOF\n").await;
    let blocks = |changes: Vec<BlockChange>| {
        Fake::new(move |_, _| {
            Ok(answer(Delta {
                blocks: changes.clone(),
                ..Delta::default()
            }))
        })
    };
    // Made.
    let made = blocks(vec![
        BlockChange::Create {
            block: a_block(0x2001, "Kapı"),
        },
        BlockChange::Create {
            block: a_block(0x2002, "Pencere"),
        },
    ]);
    create(&db, &owner, ask_import("b1", &file.id))
        .await
        .unwrap();
    assert_eq!(
        work(&db, &store, &made).await.unwrap().state,
        ServerJobState::Succeeded
    );
    assert_eq!(block_names(&db, &owner).await, ["Kapı", "Pencere"]);
    // One replaced, one removed: the versions the commit expects are the ones it reads under the lock.
    let changed = blocks(vec![
        BlockChange::Update {
            block: a_block(0x2001, "Kapı 2"),
        },
        BlockChange::Delete {
            id: kentos_contracts::BlockId(Uuid::from_u128(0x2002).into_bytes()),
        },
    ]);
    let job = create(&db, &owner, ask_import("b2", &file.id))
        .await
        .unwrap();
    assert_eq!(
        work(&db, &store, &changed).await.unwrap().state,
        ServerJobState::Succeeded
    );
    assert_eq!(block_names(&db, &owner).await, ["Kapı 2"]);
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    let result: ProjectImportResult = serde_json::from_value(job.result.unwrap()).unwrap();
    assert_eq!(result.blocks, 2);
    db.close().await;
}

#[tokio::test]
async fn an_import_names_a_file_the_caller_staged_whose_bytes_came_and_a_format_it_fits() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, project, _) = project_of_one(&db).await;
    let store = blobs();
    let file = stage_file(&db, &store, &owner, "parsel.gpkg", b"abc").await;
    let other = member(&db, "buro", "baskasi", TenantRole::ProjectManager).await;
    let theirs = new_project(&db, &other, "Başkasının").await;
    let others = open(&db, &other, theirs).await;
    let foreign = stage_file(&db, &store, &others, "baskasi.gpkg", b"xyz").await;
    // Begun and never sent.
    let waiting = job_inputs::begin(
        &db.app,
        &owner,
        JobInputBegin {
            name: "bekleyen.gpkg".into(),
            size: 3,
            sha256: sha256_hex(b"abc"),
        },
        1 << 30,
    )
    .await
    .unwrap();
    let unnamed = stage_file(&db, &store, &owner, "adsiz", b"abc").await;
    let invalid = |e: AppError, path: &str| {
        let AppError::Invalid { path: at, .. } = &e else {
            panic!("{e:?}");
        };
        assert_eq!(at.as_deref(), Some(path), "{e}");
    };
    // Not a file of this project's, someone else's file, a file that has not arrived, a name that says no format.
    invalid(
        create(&db, &owner, ask_import("a", &Uuid::new_v4().to_string()))
            .await
            .unwrap_err(),
        "input.upload",
    );
    invalid(
        create(&db, &owner, ask_import("b", &foreign.id))
            .await
            .unwrap_err(),
        "input.upload",
    );
    invalid(
        create(&db, &owner, ask_import("c", &waiting.id))
            .await
            .unwrap_err(),
        "input.upload",
    );
    invalid(
        create(&db, &owner, ask_import("d", &unnamed.id))
            .await
            .unwrap_err(),
        "input.format",
    );
    // A format said overrides the name's.
    let mut said = ask_import("e", &unnamed.id);
    said.input["format"] = json!("csv");
    create(&db, &owner, said).await.unwrap();
    // The path of an option is refused with its path; a project's other users cannot read the file.
    let mut path = ask_import("f", &file.id);
    path.input["options"] = json!({ "path": "/etc/passwd" });
    invalid(
        create(&db, &owner, path).await.unwrap_err(),
        "input.options.path",
    );
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    share(&db, &owner, &viewer.actor, GrantRole::Viewer).await;
    let viewer = open(&db, &viewer, project).await;
    assert!(matches!(
        create(&db, &viewer, ask_import("g", &file.id)).await,
        Err(AppError::Forbidden(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn a_staged_file_is_kept_only_when_its_size_and_hash_are_the_declared_ones() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, project, _) = project_of_one(&db).await;
    let store = blobs();
    let begin = |name: &str, size: u64, sha: String| JobInputBegin {
        name: name.into(),
        size,
        sha256: sha,
    };
    // A declared size past the server's limit, an empty file, a hash that is not a hash, a name with a folder.
    for (asked, path) in [
        (begin("a.gpkg", 3, sha256_hex(b"abc")), "ok"),
        (begin("a.gpkg", 2048, sha256_hex(b"abc")), "size"),
        (begin("a.gpkg", 0, sha256_hex(b"abc")), "size"),
        (begin("a.gpkg", 3, "xyz".into()), "sha256"),
        (begin("klasor/a.gpkg", 3, sha256_hex(b"abc")), "name"),
        (begin("  ", 3, sha256_hex(b"abc")), "name"),
    ] {
        let got = job_inputs::begin(&db.app, &owner, asked, 1024).await;
        match (got, path) {
            (Ok(_), "ok") => {}
            (Err(AppError::Invalid { path: at, .. }), want) => {
                assert_eq!(at.as_deref(), Some(want));
            }
            (other, _) => panic!("{other:?}"),
        }
    }
    // Bytes that are not the declared ones: nothing is kept and the same file may come again.
    let begun = job_inputs::begin(
        &db.app,
        &owner,
        begin("a.gpkg", 3, sha256_hex(b"abc")),
        1024,
    )
    .await
    .unwrap();
    let id = Uuid::parse_str(&begun.id).unwrap();
    let mut receiving = job_inputs::start_receive(&db.app, &store, &owner, id)
        .await
        .unwrap();
    receiving.writer.write(b"abd").await.unwrap();
    let refused = job_inputs::finish_receive(&db.app, &store, &owner, receiving)
        .await
        .unwrap_err();
    assert!(
        refused.to_string().contains("bildirilenle aynı değil"),
        "{refused}"
    );
    assert!(!job_inputs::get(&db.app, &owner, id).await.unwrap().received);
    let mut receiving = job_inputs::start_receive(&db.app, &store, &owner, id)
        .await
        .unwrap();
    receiving.writer.write(b"abc").await.unwrap();
    assert!(
        job_inputs::finish_receive(&db.app, &store, &owner, receiving)
            .await
            .unwrap()
            .received
    );
    // Received bytes are not taken twice; a viewer opens nothing.
    assert!(
        job_inputs::start_receive(&db.app, &store, &owner, id)
            .await
            .is_err()
    );
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    share(&db, &owner, &viewer.actor, GrantRole::Viewer).await;
    let viewer = open(&db, &viewer, project).await;
    assert!(matches!(
        job_inputs::begin(
            &db.app,
            &viewer,
            begin("a.gpkg", 3, sha256_hex(b"abc")),
            1024
        )
        .await,
        Err(AppError::Forbidden(_))
    ));
    db.close().await;
}

#[tokio::test]
async fn a_file_gone_from_the_store_and_a_commands_refusal_end_the_job_with_their_codes() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, project, _) = project_of_one(&db).await;
    let store = blobs();
    let file = stage_file(&db, &store, &owner, "parsel.gpkg", b"abc").await;
    // The object is gone by the time a worker comes.
    let key = kentos_application::blobs::Blobs::job_input_key(
        owner.tenant,
        project,
        Uuid::parse_str(&file.id).unwrap(),
        &file.sha256,
    );
    let processor = Fake::new(|_, _| Ok(answer(creates(486_900.0))));
    let job = create(&db, &owner, ask_import("g", &file.id))
        .await
        .unwrap();
    store.remove(&key).await.unwrap();
    work(&db, &store, &processor).await.unwrap();
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    assert_eq!(job.error.unwrap().code, "gone");
    assert_eq!(processor.calls(), 0, "nothing ran");
    // A command's own refusal code is the job's, once; a code of no shape is `failed`.
    let file = stage_file(&db, &store, &owner, "baska.gpkg", b"abc").await;
    for (k, (code, want)) in [
        ("crs_mismatch", "crs_mismatch"),
        ("nothing_to_import", "nothing_to_import"),
        ("Kötü Kod!", "failed"),
        ("", "failed"),
    ]
    .into_iter()
    .enumerate()
    {
        let processor = Fake::new(move |_, _| Err(ProcessFailure::new(code, "söz")));
        let job = create(&db, &owner, ask_import(&format!("r{k}"), &file.id))
            .await
            .unwrap();
        work(&db, &store, &processor).await.unwrap();
        let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
        assert_eq!(job.attempts, 1, "{code}");
        assert_eq!(job.error.unwrap().code, want, "{code}");
    }
    assert_eq!(standing(&db, &owner).await, (1, 1));
    db.close().await;
}

#[tokio::test]
async fn an_import_over_a_project_that_moved_takes_nothing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (pm, owner, project, _) = project_of_one(&db).await;
    let store = blobs();
    let file = stage_file(&db, &store, &owner, "parsel.gpkg", b"abc").await;
    let handle = tokio::runtime::Handle::current();
    let (app, who, as_owner) = (db.app.clone(), pm.clone(), owner.clone());
    let processor = Fake::new(move |_, _| {
        handle
            .block_on(changes::commit(
                &app,
                &as_owner,
                envelope(&who, project, a_point(487_000.0), &[]),
            ))
            .unwrap();
        Ok(answer(creates(486_900.0)))
    });
    let job = create(&db, &owner, ask_import("a", &file.id))
        .await
        .unwrap();
    assert_eq!(
        work(&db, &store, &processor).await.unwrap().state,
        ServerJobState::Failed
    );
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    assert_eq!(job.error.unwrap().code, "revision_conflict");
    assert_eq!(standing(&db, &owner).await, (2, 2));
    db.close().await;
}

#[tokio::test]
async fn an_atlas_is_staged_as_a_pdf_and_says_its_pages_and_the_state_it_was_made_from() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, project, _) = project_of_one(&db).await;
    let store = blobs();
    let processor = Fake::new(|task, hooks| {
        let RunRequest::Atlas(input) = &task.request else {
            panic!("an atlas: {:?}", task.request);
        };
        assert_eq!(input.sheet, "atlas-paftasi");
        assert!(task.file.is_none());
        hooks.progress.set(0.5, "Atlasın sayfaları çiziliyor");
        Ok(Processed {
            summary: "2 sayfalık atlas yazıldı (11 bayt).".into(),
            file: Some(b"%PDF-1.7 ok".to_vec()),
            details: json!({
                "pages": [{ "name": "A1/1" }, { "name": "A1/2" }],
                "pin": { "digest": "ab12" },
                "bindings": { "srid": 5253, "geo": true },
                "fields": ["Ada", "Parsel"],
                "withheld": ["Malik"],
            }),
            notes: vec![kentos_contracts::ProcessingJobNote {
                level: "info".into(),
                text: "Bir SVG resim kutu olarak çizildi.".into(),
            }],
            usage: ProcessingUsage {
                cpu_seconds: 1.5,
                peak_memory_mb: 120.0,
            },
            ..Processed::default()
        })
    });
    let job = create(&db, &owner, ask_atlas("a1", atlas_of("atlas-paftasi")))
        .await
        .unwrap();
    assert_eq!(job.kind, PROJECT_ATLAS_JOB);
    assert_eq!(
        work(&db, &store, &processor).await.unwrap().state,
        ServerJobState::Succeeded
    );
    let atlas_id = id_of(&job);
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    let artifact = job.artifact.clone().expect("a PDF");
    assert_eq!(
        (artifact.size.as_str(), artifact.name.as_str()),
        ("11", "Araç işi - atlas-paftasi.pdf")
    );
    let result: ProjectAtlasResult = serde_json::from_value(job.result.unwrap()).unwrap();
    assert_eq!(result.pages, 2);
    assert_eq!(
        (result.first.as_deref(), result.last.as_deref()),
        (Some("A1/1"), Some("A1/2"))
    );
    assert_eq!(result.pin_digest, "ab12");
    assert_eq!(result.fields, ["Ada", "Parsel"]);
    assert_eq!(result.withheld, ["Malik"]);
    assert_eq!(result.base_revision, "1");
    assert_eq!(result.bindings["srid"], 5253);
    assert_eq!(result.notes.len(), 1);
    assert_eq!(result.usage.cpu_seconds, 1.5);
    let (_, mut file) = jobs::artifact(&db.app, &store, &owner, atlas_id)
        .await
        .unwrap();
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
        .await
        .unwrap();
    assert_eq!(bytes, b"%PDF-1.7 ok");
    // The same atlas asked again of the same state: the first one's PDF, the processor not called (docs/adr/0739 §2).
    let taken = create(&db, &owner, ask_atlas("a1-yine", atlas_of("atlas-paftasi")))
        .await
        .unwrap();
    work(&db, &store, &processor).await.unwrap();
    let taken = jobs::get(&db.app, &owner, id_of(&taken)).await.unwrap();
    assert_eq!(taken.state, ServerJobState::Succeeded);
    assert_eq!(taken.cached_from, Some(atlas_id.to_string()));
    assert_eq!(
        taken.artifact.map(|a| a.sha256),
        Some(artifact.sha256.clone())
    );
    assert_eq!(processor.calls(), 1);
    // The project did not change, and the pin the command refused is the job's code.
    assert_eq!(standing(&db, &owner).await, (1, 1));
    let refusing = Fake::new(|_, _| {
        Err(ProcessFailure::new(
            "pin_mismatch",
            "Çizim sabitlenen durumda değil.",
        ))
    });
    // (Asked fresh: the same atlas of the same state would be the first one's PDF, docs/adr/0739 §2.)
    let again = create(
        &db,
        &owner,
        ServerJobCreate {
            fresh: true,
            ..ask_atlas("a2", atlas_of("atlas-paftasi"))
        },
    )
    .await
    .unwrap();
    work(&db, &store, &refusing).await.unwrap();
    let again = jobs::get(&db.app, &owner, id_of(&again)).await.unwrap();
    assert_eq!(again.error.unwrap().code, "pin_mismatch");
    assert_eq!(refusing.calls(), 1, "a refusal is not tried again");
    // A reader runs no job; a pinned revision that is not the project's is refused before anything is drawn.
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    share(&db, &owner, &viewer.actor, GrantRole::Viewer).await;
    let viewer = open(&db, &viewer, project).await;
    assert!(matches!(
        create(&db, &viewer, ask_atlas("v", atlas_of("atlas-paftasi"))).await,
        Err(AppError::Forbidden(_))
    ));
    let pinned = create(
        &db,
        &owner,
        ask_atlas(
            "p",
            ProjectAtlasInput {
                expected_revision: Some("0".into()),
                ..atlas_of("atlas-paftasi")
            },
        ),
    )
    .await
    .unwrap();
    let before = refusing.calls();
    work(&db, &store, &refusing).await.unwrap();
    assert_eq!(refusing.calls(), before, "the revision was refused first");
    let pinned = jobs::get(&db.app, &owner, id_of(&pinned)).await.unwrap();
    assert_eq!(pinned.error.unwrap().code, "revision_conflict");
    db.close().await;
}

#[tokio::test]
async fn the_retention_removes_staged_files_and_the_cleanup_the_objects_no_row_holds() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, project, _) = project_of_one(&db).await;
    let store = blobs();
    let file = stage_file(&db, &store, &owner, "parsel.gpkg", b"abc").await;
    let key = kentos_application::blobs::Blobs::job_input_key(
        owner.tenant,
        project,
        Uuid::parse_str(&file.id).unwrap(),
        &file.sha256,
    );
    assert_eq!(store.len(&key).await.unwrap(), 3);
    // Nothing is that old yet; with no retention everything is.
    let kept =
        kentos_application::job_upkeep::expire(&db.app, &store, Duration::from_secs(3600), 10)
            .await
            .unwrap();
    assert_eq!(kept.inputs, 0);
    let gone = kentos_application::job_upkeep::expire(&db.app, &store, Duration::ZERO, 10)
        .await
        .unwrap();
    assert_eq!(gone.inputs, 1);
    assert_eq!(
        store.len(&key).await.unwrap(),
        0,
        "the object went with its row"
    );
    assert!(
        job_inputs::get(&db.app, &owner, Uuid::parse_str(&file.id).unwrap())
            .await
            .is_err()
    );
    // An object of no row is the store's cleanup's.
    let stray = kentos_application::blobs::Blobs::job_input_key(
        owner.tenant,
        project,
        Uuid::new_v4(),
        &sha256_hex(b"x"),
    );
    let mut w = store.create(&stray, 1).await.unwrap();
    w.write(b"x").await.unwrap();
    w.finish().await.unwrap();
    let done = kentos_application::files::cleanup(&db.app, &store, Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(done.job_inputs, 1);
    assert_eq!(store.len(&stray).await.unwrap(), 0);
    db.close().await;
}

/// The time a big import's commit takes (docs/adr/0561 Performans): `KENTOS_PERF_CHANGES` points (50 000) created in
/// the finish of one job, the revision they make and the snapshot the next job reads.
///
/// ```text
/// KENTOS_TEST_DB=required cargo test --release -p kentos-application --test processing_jobs import_timing -- --ignored --nocapture
/// ```
#[tokio::test]
#[ignore = "timing, run by hand"]
async fn import_timing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let (_pm, owner, _project, _) = project_of_one(&db).await;
    let store = blobs();
    let count: u32 = std::env::var("KENTOS_PERF_CHANGES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50_000);
    let file = stage_file(&db, &store, &owner, "noktalar.csv", b"X,Y\n1,2\n").await;
    let processor = Fake::new(move |_, _| {
        let mut delta = Delta::default();
        for i in 0..count {
            delta
                .features
                .extend(a_point(486_000.0 + f64::from(i)).features);
        }
        delta.added = u64::from(count);
        Ok(answer(delta))
    });
    let job = create(&db, &owner, ask_import("t", &file.id))
        .await
        .unwrap();
    let started = std::time::Instant::now();
    let worked = work(&db, &store, &processor).await.unwrap();
    let took = started.elapsed();
    assert_eq!(worked.state, ServerJobState::Succeeded);
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    println!(
        "{count} nesnelik içe aktarma: {:.2} sn (iş, işleme ve commit dahil; nesne başına {:.0} µs), proje {} nesne",
        took.as_secs_f64(),
        took.as_secs_f64() * 1e6 / f64::from(count),
        standing(&db, &owner).await.0
    );
    assert!(job.error.is_none());
    db.close().await;
}
