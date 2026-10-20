//! Durable server jobs (docs/adr/0542): a job is asked for once whatever the retries, a worker runs it to a staged
//! artifact that is the project's file, a worker that dies loses the lease and the next one finishes it (the dead one's
//! late finish refused by the fence), the attempts run out, a cancel stops a queued and a running job, a job past its
//! time fails, an export at another revision is refused, a grant taken away while the job runs refuses its commit, and
//! the store's cleanup removes artifacts no job holds. Real database (`KENTOS_TEST_DB=required` makes a missing one a
//! failure). A second file (docs/adr/0547) holds the limits: an organisation's cap on queued and running jobs even
//! under concurrent asking, the artifact size cap, a store that cannot be written, a worker that died mid-write, two
//! workers for one job, the retention sweep and the queue's numbers; and the checkpoint job.

mod common;

use std::time::Duration;

use common::{a_point, blobs, envelope, member, new_project, open, revoke, share};
use kentos_application::jobs::{JobLimits, Worked};
use kentos_application::{AppError, changes, checkpoints, files, job_upkeep, snapshot};
use kentos_contracts::{GrantRole, ServerJob, ServerJobCreate, ServerJobState, TenantRole};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const LEASE: Duration = Duration::from_secs(30);

/// The jobs' calls with the default limits (the first test does not care about them).
mod jobs {
    use std::time::Duration;

    use kentos_application::AppError;
    use kentos_application::access::ProjectAccess;
    use kentos_application::blobs::Blobs;
    pub use kentos_application::jobs::{JobLimits, Worked, artifact, cancel, get, list};
    use kentos_contracts::{ServerJob, ServerJobCreate};
    use uuid::Uuid;

    pub async fn create(
        db: &kentos_postgres::Db,
        access: &ProjectAccess,
        asked: ServerJobCreate,
    ) -> Result<ServerJob, AppError> {
        kentos_application::jobs::create(db, access, asked, &JobLimits::default()).await
    }

    pub async fn work_once(
        db: &kentos_postgres::Db,
        blobs: &Blobs,
        owner: Uuid,
        lease: Duration,
    ) -> Result<Option<Worked>, AppError> {
        kentos_application::jobs::work_once(db, blobs, owner, lease, &JobLimits::default()).await
    }
}

fn ask(request: &str, input: serde_json::Value) -> ServerJobCreate {
    ServerJobCreate {
        kind: "project.export".into(),
        input,
        request_id: request.into(),
        timeout_seconds: None,
    }
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A raw claim, as a worker that then dies would make: (job, fence).
async fn claim(db: &TestDb, owner: Uuid, lease_secs: i32) -> Option<(Uuid, i64)> {
    sqlx::query_as("select id, fence from kentos.claim_job($1, $2, $3)")
        .bind(owner)
        .bind(lease_secs)
        .bind(vec![
            "project.export".to_owned(),
            "project.checkpoint".to_owned(),
        ])
        .fetch_optional(&db.app.pool)
        .await
        .unwrap()
}

async fn beat(db: &TestDb, job: Uuid, fence: i64) -> String {
    sqlx::query_scalar("select kentos.beat_job($1, $2, 0.5, 30)")
        .bind(job)
        .bind(fence)
        .fetch_one(&db.app.pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn jobs_run_once_and_stand_failures() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let ed = member(&db, "buro", "duzenleyici", TenantRole::Viewer).await;
    let project = new_project(&db, &pm, "Sunucu işleri").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &ed.actor, GrantRole::Editor).await;
    let editor = open(&db, &ed, project).await;
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486_500.0), &[]),
    )
    .await
    .unwrap();
    let store = blobs();

    // Asked once whatever the retries.
    let first = jobs::create(&db.app, &owner, ask("istek-1", json!({})))
        .await
        .unwrap();
    let again = jobs::create(
        &db.app,
        &owner,
        ask("istek-1", json!({ "expectedRevision": "9" })),
    )
    .await
    .unwrap();
    assert_eq!(first.id, again.id);
    assert_eq!(first.state, ServerJobState::Queued);
    assert_eq!(first.base_revision, "1");
    assert_eq!(jobs::list(&db.app, &owner).await.unwrap().jobs.len(), 1);
    // A kind the server has not, and input it cannot read.
    assert!(matches!(
        jobs::create(
            &db.app,
            &owner,
            ServerJobCreate {
                kind: "yok".into(),
                ..ask("x", json!({}))
            }
        )
        .await,
        Err(AppError::Invalid { .. })
    ));
    assert!(matches!(
        jobs::create(
            &db.app,
            &owner,
            ask("y", json!({ "expectedRevision": "bir" }))
        )
        .await,
        Err(AppError::Invalid { .. })
    ));

    // A worker runs it: the artifact is the project's file of that revision.
    let worker = Uuid::now_v7();
    let done = jobs::work_once(&db.app, &store, worker, LEASE)
        .await
        .unwrap();
    assert_eq!(
        done,
        Some(Worked {
            job: Uuid::parse_str(&first.id).unwrap(),
            state: ServerJobState::Succeeded
        })
    );
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&first.id).unwrap())
        .await
        .unwrap();
    assert_eq!(job.state, ServerJobState::Succeeded);
    assert_eq!(job.progress, 1.0);
    assert_eq!(job.attempts, 1);
    let artifact = job.artifact.clone().expect("an artifact");
    let (_, mut file) = jobs::artifact(&db.app, &store, &owner, Uuid::parse_str(&job.id).unwrap())
        .await
        .unwrap();
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
        .await
        .unwrap();
    assert_eq!(sha(&bytes), artifact.sha256);
    assert_eq!(bytes.len().to_string(), artifact.size);
    let snap = snapshot::snapshot(&db.app, &owner).await.unwrap();
    assert_eq!(snap.sha256, artifact.sha256, "the project's own file");
    assert_eq!(job.result.as_ref().unwrap()["revision"], "1");
    assert_eq!(
        jobs::work_once(&db.app, &store, worker, LEASE)
            .await
            .unwrap(),
        None
    );

    // A worker dies after claiming: its lease runs out, the next claim takes the job with a new fence and finishes it;
    // the dead one's beat and finish are refused.
    let second = jobs::create(&db.app, &owner, ask("istek-2", json!({})))
        .await
        .unwrap();
    let dead = Uuid::now_v7();
    let (id, old_fence) = claim(&db, dead, 1).await.expect("claimed");
    assert_eq!(id.to_string(), second.id);
    tokio::time::sleep(Duration::from_millis(1300)).await;
    let done = jobs::work_once(&db.app, &store, worker, LEASE)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded);
    let job = jobs::get(&db.app, &owner, id).await.unwrap();
    assert_eq!(job.attempts, 2);
    assert_eq!(beat(&db, id, old_fence).await, "lost");
    let late: bool = sqlx::query_scalar(
        "select kentos.finish_job($1, $2, false, null, null, null, null, 'failed', 'geç')",
    )
    .bind(id)
    .bind(old_fence)
    .fetch_one(&db.app.pool)
    .await
    .unwrap();
    assert!(!late, "a stale fence finishes nothing");
    assert_eq!(
        jobs::get(&db.app, &owner, id).await.unwrap().state,
        ServerJobState::Succeeded
    );

    // Workers that keep dying use up the attempts.
    let third = jobs::create(&db.app, &owner, ask("istek-3", json!({})))
        .await
        .unwrap();
    for _ in 0..3 {
        claim(&db, Uuid::now_v7(), 1).await.expect("claimed again");
        tokio::time::sleep(Duration::from_millis(1100)).await;
    }
    assert_eq!(claim(&db, Uuid::now_v7(), 1).await, None);
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&third.id).unwrap())
        .await
        .unwrap();
    assert_eq!(job.state, ServerJobState::Failed);
    assert_eq!(job.error.unwrap().code, "lease_lost");

    // A queued job cancelled at once; a running one at its next beat.
    let queued = jobs::create(&db.app, &owner, ask("istek-4", json!({})))
        .await
        .unwrap();
    let qid = Uuid::parse_str(&queued.id).unwrap();
    assert_eq!(
        jobs::cancel(&db.app, &owner, qid).await.unwrap().state,
        ServerJobState::Cancelled
    );
    assert_eq!(
        jobs::work_once(&db.app, &store, worker, LEASE)
            .await
            .unwrap(),
        None
    );
    let running = jobs::create(&db.app, &owner, ask("istek-5", json!({})))
        .await
        .unwrap();
    let (rid, fence) = claim(&db, Uuid::now_v7(), 30).await.unwrap();
    assert_eq!(rid.to_string(), running.id);
    let asked = jobs::cancel(&db.app, &owner, rid).await.unwrap();
    assert_eq!(
        (asked.state, asked.cancel_requested),
        (ServerJobState::Running, true)
    );
    assert_eq!(beat(&db, rid, fence).await, "cancel");
    assert_eq!(
        jobs::get(&db.app, &owner, rid).await.unwrap().state,
        ServerJobState::Cancelled
    );
    // The editor may not cancel the owner's job.
    let other = jobs::create(&db.app, &owner, ask("istek-6", json!({})))
        .await
        .unwrap();
    assert!(matches!(
        jobs::cancel(&db.app, &editor, Uuid::parse_str(&other.id).unwrap()).await,
        Err(AppError::Forbidden(_))
    ));
    jobs::cancel(&db.app, &owner, Uuid::parse_str(&other.id).unwrap())
        .await
        .unwrap();

    // A job past its time fails at the next claim.
    let slow = jobs::create(
        &db.app,
        &owner,
        ServerJobCreate {
            timeout_seconds: Some(1),
            ..ask("istek-7", json!({}))
        },
    )
    .await
    .unwrap();
    claim(&db, Uuid::now_v7(), 30).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1300)).await;
    assert_eq!(claim(&db, Uuid::now_v7(), 30).await, None);
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&slow.id).unwrap())
        .await
        .unwrap();
    assert_eq!(
        (job.state, job.error.unwrap().code.as_str()),
        (ServerJobState::Failed, "timeout")
    );

    // An export asked at another revision is refused, nothing staged.
    let stale = jobs::create(
        &db.app,
        &owner,
        ask("istek-8", json!({ "expectedRevision": "0" })),
    )
    .await
    .unwrap();
    let done = jobs::work_once(&db.app, &store, worker, LEASE)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.state, ServerJobState::Failed);
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&stale.id).unwrap())
        .await
        .unwrap();
    assert_eq!(job.error.unwrap().code, "revision_conflict");
    assert!(job.artifact.is_none());

    // The editor's grant taken away after asking: the job fails at its run and leaves nothing.
    let theirs = jobs::create(&db.app, &editor, ask("istek-9", json!({})))
        .await
        .unwrap();
    revoke(&db, &owner, &ed.actor).await;
    let done = jobs::work_once(&db.app, &store, worker, LEASE)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.state, ServerJobState::Failed);
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&theirs.id).unwrap())
        .await
        .unwrap();
    assert_eq!(job.error.unwrap().code, "forbidden");
    assert!(job.artifact.is_none());

    // An artifact no job holds (a finish that never came) is removed by the store's cleanup; held ones stay.
    let key = kentos_application::blobs::Blobs::job_key(
        owner.tenant,
        project,
        Uuid::now_v7(),
        &"0".repeat(64),
    );
    let mut w = store.create(&key, 3).await.unwrap();
    w.write(b"abc").await.unwrap();
    w.finish().await.unwrap();
    let cleaned = files::cleanup(&db.app, &store, Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(cleaned.jobs, 1);
    assert!(store.read(&key).await.is_err());
    assert!(
        jobs::artifact(&db.app, &store, &owner, Uuid::parse_str(&first.id).unwrap())
            .await
            .is_ok(),
        "a held artifact stays"
    );
}

// ── Limits, retention, measures, the checkpoint job (docs/adr/0547) ─────────────────────────────────────────────

fn limited(tenant_active: u32) -> JobLimits {
    JobLimits {
        tenant_active,
        ..JobLimits::default()
    }
}

async fn make(
    db: &TestDb,
    access: &kentos_application::access::ProjectAccess,
    request: &str,
    limits: &JobLimits,
) -> Result<ServerJob, AppError> {
    kentos_application::jobs::create(&db.app, access, ask(request, json!({})), limits).await
}

fn id_of(job: &ServerJob) -> Uuid {
    Uuid::parse_str(&job.id).unwrap()
}

/// Everything a job staged in the store, whatever its age.
async fn staged(store: &kentos_application::blobs::Blobs) -> Vec<String> {
    store
        .job_objects(Duration::ZERO)
        .await
        .unwrap()
        .into_iter()
        .map(|(_, key)| key)
        .collect()
}

#[tokio::test]
async fn an_organisation_has_a_cap_on_its_jobs_even_asked_at_once() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    kentos_application::admin::create_tenant(&db.owner, "diger", "Diğer", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let other = member(&db, "diger", "baska", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Sınır").await;
    let theirs = new_project(&db, &other, "Başkasının").await;
    let owner = open(&db, &pm, project).await;
    let outsider = open(&db, &other, theirs).await;
    let cap = limited(2);

    let one = make(&db, &owner, "a", &cap).await.unwrap();
    make(&db, &owner, "b", &cap).await.unwrap();
    // The third is refused with its own code, a Turkish message naming the cap, and an advice to try again.
    let refused = make(&db, &owner, "c", &cap).await.unwrap_err();
    assert!(matches!(refused, AppError::Quota(_)));
    assert_eq!(refused.code(), "quota_exceeded");
    assert!(refused.to_string().contains("(2)"), "{refused}");
    assert!(refused.retry().0);
    // A retried request gets its job back whatever the load.
    assert_eq!(make(&db, &owner, "a", &cap).await.unwrap().id, one.id);
    // Another organisation has its own count.
    make(&db, &outsider, "a", &cap).await.unwrap();
    // A place is free again when a job is cancelled; a running job holds its place.
    kentos_application::jobs::cancel(&db.app, &owner, id_of(&one))
        .await
        .unwrap();
    make(&db, &owner, "c", &cap).await.unwrap();
    assert!(matches!(
        make(&db, &owner, "d", &cap).await,
        Err(AppError::Quota(_))
    ));
    let (running, _) = claim(&db, Uuid::now_v7(), 30).await.expect("claimed");
    assert!(
        matches!(make(&db, &owner, "e", &cap).await, Err(AppError::Quota(_))),
        "a running job holds its place ({running})"
    );

    // Many asked at once for the last places: exactly as many as there are places get one.
    let burst = limited(5);
    let all = futures_join(
        (0..8)
            .map(|i| {
                let access = &outsider;
                let db = &db;
                let burst = &burst;
                async move { make(db, access, &format!("burst-{i}"), burst).await }
            })
            .collect(),
    )
    .await;
    let made = all.iter().filter(|r| r.is_ok()).count();
    assert_eq!(
        made, 4,
        "five places, one already taken by the other request"
    );
    assert!(
        all.iter()
            .filter(|r| r.is_err())
            .all(|r| matches!(r, Err(AppError::Quota(_))))
    );
    db.close().await;
}

/// Runs futures together on one task (the test's pool serves them at once) and gives their answers in order.
async fn futures_join<T>(futures: Vec<impl std::future::Future<Output = T>>) -> Vec<T> {
    let mut pending: Vec<_> = futures.into_iter().map(|f| Some(Box::pin(f))).collect();
    let mut done: Vec<Option<T>> = (0..pending.len()).map(|_| None).collect();
    std::future::poll_fn(|cx| {
        let mut all = true;
        for (i, slot) in pending.iter_mut().enumerate() {
            if let Some(f) = slot {
                match f.as_mut().poll(cx) {
                    std::task::Poll::Ready(v) => {
                        done[i] = Some(v);
                        *slot = None;
                    }
                    std::task::Poll::Pending => all = false,
                }
            }
        }
        if all {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    done.into_iter().flatten().collect()
}

#[tokio::test]
async fn an_artifact_that_is_too_big_or_cannot_be_written_fails_its_job_and_leaves_nothing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Dosya").await;
    let owner = open(&db, &pm, project).await;
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486_500.0), &[]),
    )
    .await
    .unwrap();
    let worker = Uuid::now_v7();

    // Past the server's size limit: `too_large`, nothing staged.
    let store = blobs();
    let small = JobLimits {
        artifact_max: 10,
        ..JobLimits::default()
    };
    let job = make(&db, &owner, "buyuk", &small).await.unwrap();
    let done = kentos_application::jobs::work_once(&db.app, &store, worker, LEASE, &small)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.state, ServerJobState::Failed);
    let job = kentos_application::jobs::get(&db.app, &owner, id_of(&job))
        .await
        .unwrap();
    let error = job.error.unwrap();
    assert_eq!(error.code, "too_large");
    assert!(error.message.contains("10 bayt"), "{}", error.message);
    assert!(job.artifact.is_none());
    assert!(staged(&store).await.is_empty());

    // A store that cannot be written (the folder for jobs is a file; a full disk fails the same way): `storage`, no
    // object, and the job is over rather than tried again for ever.
    let broken_root = std::env::temp_dir().join(format!("kentos-broken-store-{}", Uuid::now_v7()));
    std::fs::create_dir_all(&broken_root).unwrap();
    std::fs::write(broken_root.join("jobs"), b"not a folder").unwrap();
    let broken = kentos_application::blobs::Blobs::new(&broken_root);
    let job = make(&db, &owner, "disk", &JobLimits::default())
        .await
        .unwrap();
    let done =
        kentos_application::jobs::work_once(&db.app, &broken, worker, LEASE, &JobLimits::default())
            .await
            .unwrap()
            .unwrap();
    assert_eq!(done.state, ServerJobState::Failed);
    let job = kentos_application::jobs::get(&db.app, &owner, id_of(&job))
        .await
        .unwrap();
    assert_eq!(job.error.unwrap().code, "storage");
    assert!(job.artifact.is_none());
    assert_eq!(job.attempts, 1);
    assert!(
        std::fs::metadata(broken_root.join("jobs"))
            .unwrap()
            .is_file(),
        "nothing was written over what is there"
    );
    db.close().await;
}

#[tokio::test]
async fn a_worker_that_died_mid_write_leaves_a_partial_object_the_next_run_replaces() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Yarım").await;
    let owner = open(&db, &pm, project).await;
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486_500.0), &[]),
    )
    .await
    .unwrap();
    let store = blobs();
    let snap = snapshot::snapshot(&db.app, &owner).await.unwrap();

    // The first worker claims, writes part of the file under the key the job will use, and dies.
    let job = make(&db, &owner, "yarim", &JobLimits::default())
        .await
        .unwrap();
    let id = id_of(&job);
    let key = kentos_application::blobs::Blobs::job_key(owner.tenant, project, id, &snap.sha256);
    let (claimed, _) = claim(&db, Uuid::now_v7(), 1).await.expect("claimed");
    assert_eq!(claimed, id);
    let mut w = store.create(&key, 10).await.unwrap();
    w.write(&snap.bytes[..10]).await.unwrap();
    w.finish().await.unwrap();
    assert_eq!(store.len(&key).await.unwrap(), 10);
    tokio::time::sleep(Duration::from_millis(1300)).await;
    // Nobody holds it yet: it is a stray the cleanup would remove (the partial file is the dead worker's).
    assert_eq!(staged(&store).await, vec![key.clone()]);

    // The next worker takes the job and writes the whole file over the partial one.
    let done = jobs::work_once(&db.app, &store, Uuid::now_v7(), LEASE)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded);
    let job = jobs::get(&db.app, &owner, id).await.unwrap();
    assert_eq!(job.attempts, 2);
    assert_eq!(job.artifact.unwrap().sha256, snap.sha256);
    let whole = store.read(&key).await.unwrap();
    assert_eq!(whole, snap.bytes, "the partial bytes were replaced");
    // The held object stays when the cleanup runs.
    let cleaned = files::cleanup(&db.app, &store, Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(cleaned.jobs, 0);
    assert!(store.read(&key).await.is_ok());

    // A job cancelled after its worker staged part of a file: the cleanup removes the stray.
    let second = make(&db, &owner, "yarim-2", &JobLimits::default())
        .await
        .unwrap();
    let id2 = id_of(&second);
    claim(&db, Uuid::now_v7(), 30).await.expect("claimed");
    let key2 = kentos_application::blobs::Blobs::job_key(owner.tenant, project, id2, &snap.sha256);
    let mut w = store.create(&key2, 4).await.unwrap();
    w.write(&snap.bytes[..4]).await.unwrap();
    w.finish().await.unwrap();
    jobs::cancel(&db.app, &owner, id2).await.unwrap();
    let cleaned = files::cleanup(&db.app, &store, Duration::ZERO)
        .await
        .unwrap();
    assert_eq!(cleaned.jobs, 1);
    assert!(store.read(&key2).await.is_err());
    db.close().await;
}

#[tokio::test]
async fn two_workers_for_one_job_run_it_once() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Çift").await;
    let owner = open(&db, &pm, project).await;
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486_500.0), &[]),
    )
    .await
    .unwrap();
    let store = blobs();
    let job = make(&db, &owner, "cift", &JobLimits::default())
        .await
        .unwrap();
    // Eight workers look at the one queued job at the same time.
    let ran = futures_join(
        (0..8)
            .map(|_| {
                let (db, store) = (&db, &store);
                async move {
                    jobs::work_once(&db.app, store, Uuid::now_v7(), LEASE)
                        .await
                        .unwrap()
                }
            })
            .collect(),
    )
    .await;
    assert_eq!(
        ran.iter().filter(|r| r.is_some()).count(),
        1,
        "one worker claimed it"
    );
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    assert_eq!((job.state, job.attempts), (ServerJobState::Succeeded, 1));
    assert_eq!(staged(&store).await.len(), 1, "one artifact");
    db.close().await;
}

#[tokio::test]
async fn finished_jobs_and_their_files_go_after_the_retention_and_the_queue_is_counted() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Saklama").await;
    let owner = open(&db, &pm, project).await;
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486_500.0), &[]),
    )
    .await
    .unwrap();
    let store = blobs();
    let worker = Uuid::now_v7();
    let limits = JobLimits::default();

    // Old and new: a succeeded export, a failed one, a cancelled one, a fresh succeeded one, a queued one.
    let old_ok = make(&db, &owner, "eski-biten", &limits).await.unwrap();
    jobs::work_once(&db.app, &store, worker, LEASE)
        .await
        .unwrap()
        .unwrap();
    let old_failed = kentos_application::jobs::create(
        &db.app,
        &owner,
        ask("eski-dusen", json!({ "expectedRevision": "0" })),
        &limits,
    )
    .await
    .unwrap();
    jobs::work_once(&db.app, &store, worker, LEASE)
        .await
        .unwrap()
        .unwrap();
    let old_cancelled = make(&db, &owner, "eski-durdurulan", &limits).await.unwrap();
    jobs::cancel(&db.app, &owner, id_of(&old_cancelled))
        .await
        .unwrap();
    let fresh = make(&db, &owner, "yeni-biten", &limits).await.unwrap();
    jobs::work_once(&db.app, &store, worker, LEASE)
        .await
        .unwrap()
        .unwrap();
    let waiting = make(&db, &owner, "bekleyen", &limits).await.unwrap();
    let key = store_key_of(&db, &owner, &old_ok).await;
    assert!(store.read(&key).await.is_ok());

    // The queue's numbers, before the sweep.
    let m = job_upkeep::metrics(&db.owner).await.unwrap();
    assert_eq!((m.queued, m.running), (1, 0));
    assert_eq!((m.succeeded_24h, m.failed_24h, m.cancelled_24h), (2, 1, 1));
    assert!(m.oldest_queued_seconds.unwrap() >= 0.0);
    assert_eq!(m.tenants, vec![(owner.tenant, 1, 0)]);

    // Three of the finished jobs ended a month ago.
    for job in [&old_ok, &old_failed, &old_cancelled] {
        sqlx::query(
            "update kentos.server_job set finished_at = now() - interval '30 days' where id = $1",
        )
        .bind(id_of(job))
        .execute(&db.owner)
        .await
        .unwrap();
    }
    let gone = job_upkeep::expire(&db.app, &store, Duration::from_secs(14 * 24 * 3600), 2)
        .await
        .unwrap();
    assert_eq!(
        (gone.jobs, gone.objects),
        (3, 1),
        "batches of two, until none is left"
    );
    assert!(store.read(&key).await.is_err(), "the old artifact is gone");
    for job in [&old_ok, &old_failed, &old_cancelled] {
        assert!(matches!(
            jobs::get(&db.app, &owner, id_of(job)).await,
            Err(AppError::NotFound(_))
        ));
    }
    // What is recent or still waiting stays, with its artifact.
    assert_eq!(
        jobs::get(&db.app, &owner, id_of(&fresh))
            .await
            .unwrap()
            .state,
        ServerJobState::Succeeded
    );
    assert!(
        jobs::artifact(&db.app, &store, &owner, id_of(&fresh))
            .await
            .is_ok()
    );
    assert_eq!(
        jobs::get(&db.app, &owner, id_of(&waiting))
            .await
            .unwrap()
            .state,
        ServerJobState::Queued
    );
    assert_eq!(
        job_upkeep::expire(&db.app, &store, Duration::from_secs(14 * 24 * 3600), 2)
            .await
            .unwrap(),
        Default::default()
    );

    // An artifact the store fails to remove is nobody's after its job is gone: the cleanup finds it.
    let stray = kentos_application::blobs::Blobs::job_key(
        owner.tenant,
        project,
        id_of(&old_ok),
        &"1".repeat(64),
    );
    let mut w = store.create(&stray, 1).await.unwrap();
    w.write(b"x").await.unwrap();
    w.finish().await.unwrap();
    assert_eq!(
        files::cleanup(&db.app, &store, Duration::ZERO)
            .await
            .unwrap()
            .jobs,
        1
    );
    db.close().await;
}

/// The artifact key a job holds.
async fn store_key_of(
    db: &TestDb,
    access: &kentos_application::access::ProjectAccess,
    job: &ServerJob,
) -> String {
    let artifact = jobs::get(&db.app, access, id_of(job))
        .await
        .unwrap()
        .artifact
        .unwrap();
    kentos_application::blobs::Blobs::job_key(
        access.tenant,
        access.project,
        id_of(job),
        &artifact.sha256,
    )
}

#[tokio::test]
async fn a_checkpoint_can_be_made_as_a_job() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    kentos_application::admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    let project = new_project(&db, &pm, "Kontrol").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &viewer.actor, GrantRole::Viewer).await;
    let looker = open(&db, &viewer, project).await;
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486_500.0), &[]),
    )
    .await
    .unwrap();
    let store = blobs();
    let limits = JobLimits::default();
    let ask_checkpoint = |request: &str, input: serde_json::Value| ServerJobCreate {
        kind: "project.checkpoint".into(),
        input,
        request_id: request.into(),
        timeout_seconds: None,
    };

    // Input is checked when asked: a name, a note within its limit, a revision number.
    for bad in [
        json!({}),
        json!({ "name": "  " }),
        json!({ "name": "a\nb" }),
        json!({ "name": "x", "fileRevision": "0" }),
    ] {
        assert!(matches!(
            kentos_application::jobs::create(&db.app, &owner, ask_checkpoint("kotu", bad), &limits)
                .await,
            Err(AppError::Invalid { .. })
        ));
    }
    // A viewer neither runs jobs nor writes.
    assert!(matches!(
        kentos_application::jobs::create(
            &db.app,
            &looker,
            ask_checkpoint("izle", json!({ "name": "x" })),
            &limits
        )
        .await,
        Err(AppError::Forbidden(_))
    ));

    // The first worker dies after claiming; the second makes the checkpoint, once, as the user who asked.
    let job = kentos_application::jobs::create(
        &db.app,
        &owner,
        ask_checkpoint(
            "teslim",
            json!({ "name": "Belediyeye teslim", "note": "ilk" }),
        ),
        &limits,
    )
    .await
    .unwrap();
    claim(&db, Uuid::now_v7(), 1).await.expect("claimed");
    tokio::time::sleep(Duration::from_millis(1300)).await;
    let done = jobs::work_once(&db.app, &store, Uuid::now_v7(), LEASE)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded);
    let job = jobs::get(&db.app, &owner, id_of(&job)).await.unwrap();
    assert!(job.artifact.is_none(), "a checkpoint keeps its own file");
    assert_eq!(
        job.result.as_ref().unwrap()["checkpoint"]["name"],
        "Belediyeye teslim"
    );
    let list = checkpoints::list(&db.app, &owner)
        .await
        .unwrap()
        .checkpoints;
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].note.as_deref(), Some("ilk"));
    assert_eq!(list[0].id, job.result.as_ref().unwrap()["checkpoint"]["id"]);
    // Its artifact route has nothing to give.
    assert!(matches!(
        jobs::artifact(&db.app, &store, &owner, id_of(&job)).await,
        Err(AppError::NotFound(_))
    ));
    db.close().await;
}
