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
        assert_eq!(task.input.tool, "geometry.buffer");
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
            sink: Some(b"GPKG-bytes".to_vec()),
            sink_rows: 3,
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
