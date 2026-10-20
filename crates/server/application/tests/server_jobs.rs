//! Durable server jobs (docs/adr/0542): a job is asked for once whatever the retries, a worker runs it to a staged
//! artifact that is the project's file, a worker that dies loses the lease and the next one finishes it (the dead one's
//! late finish refused by the fence), the attempts run out, a cancel stops a queued and a running job, a job past its
//! time fails, an export at another revision is refused, a grant taken away while the job runs refuses its commit, and
//! the store's cleanup removes artifacts no job holds. Real database (`KENTOS_TEST_DB=required` makes a missing one a
//! failure).

mod common;

use std::time::Duration;

use common::{a_point, blobs, envelope, member, new_project, open, revoke, share};
use kentos_application::jobs::{self, Worked};
use kentos_application::{AppError, changes, files, snapshot};
use kentos_contracts::{GrantRole, ServerJobCreate, ServerJobState, TenantRole};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const LEASE: Duration = Duration::from_secs(30);

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
        .bind(vec!["project.export".to_owned()])
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
    changes::commit(&db.app, &owner, envelope(&pm, project, a_point(486_500.0), &[]))
        .await
        .unwrap();
    let store = blobs();

    // Asked once whatever the retries.
    let first = jobs::create(&db.app, &owner, ask("istek-1", json!({}))).await.unwrap();
    let again = jobs::create(&db.app, &owner, ask("istek-1", json!({ "expectedRevision": "9" })))
        .await
        .unwrap();
    assert_eq!(first.id, again.id);
    assert_eq!(first.state, ServerJobState::Queued);
    assert_eq!(first.base_revision, "1");
    assert_eq!(jobs::list(&db.app, &owner).await.unwrap().jobs.len(), 1);
    // A kind the server has not, and input it cannot read.
    assert!(matches!(
        jobs::create(&db.app, &owner, ServerJobCreate { kind: "yok".into(), ..ask("x", json!({})) }).await,
        Err(AppError::Invalid { .. })
    ));
    assert!(matches!(
        jobs::create(&db.app, &owner, ask("y", json!({ "expectedRevision": "bir" }))).await,
        Err(AppError::Invalid { .. })
    ));

    // A worker runs it: the artifact is the project's file of that revision.
    let worker = Uuid::now_v7();
    let done = jobs::work_once(&db.app, &store, worker, LEASE).await.unwrap();
    assert_eq!(
        done,
        Some(Worked {
            job: Uuid::parse_str(&first.id).unwrap(),
            state: ServerJobState::Succeeded
        })
    );
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&first.id).unwrap()).await.unwrap();
    assert_eq!(job.state, ServerJobState::Succeeded);
    assert_eq!(job.progress, 1.0);
    assert_eq!(job.attempts, 1);
    let artifact = job.artifact.clone().expect("an artifact");
    let (_, mut file) = jobs::artifact(&db.app, &store, &owner, Uuid::parse_str(&job.id).unwrap())
        .await
        .unwrap();
    let mut bytes = Vec::new();
    tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes).await.unwrap();
    assert_eq!(sha(&bytes), artifact.sha256);
    assert_eq!(bytes.len().to_string(), artifact.size);
    let snap = snapshot::snapshot(&db.app, &owner).await.unwrap();
    assert_eq!(snap.sha256, artifact.sha256, "the project's own file");
    assert_eq!(job.result.as_ref().unwrap()["revision"], "1");
    assert_eq!(jobs::work_once(&db.app, &store, worker, LEASE).await.unwrap(), None);

    // A worker dies after claiming: its lease runs out, the next claim takes the job with a new fence and finishes it;
    // the dead one's beat and finish are refused.
    let second = jobs::create(&db.app, &owner, ask("istek-2", json!({}))).await.unwrap();
    let dead = Uuid::now_v7();
    let (id, old_fence) = claim(&db, dead, 1).await.expect("claimed");
    assert_eq!(id.to_string(), second.id);
    tokio::time::sleep(Duration::from_millis(1300)).await;
    let done = jobs::work_once(&db.app, &store, worker, LEASE).await.unwrap().unwrap();
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
    assert_eq!(jobs::get(&db.app, &owner, id).await.unwrap().state, ServerJobState::Succeeded);

    // Workers that keep dying use up the attempts.
    let third = jobs::create(&db.app, &owner, ask("istek-3", json!({}))).await.unwrap();
    for _ in 0..3 {
        claim(&db, Uuid::now_v7(), 1).await.expect("claimed again");
        tokio::time::sleep(Duration::from_millis(1100)).await;
    }
    assert_eq!(claim(&db, Uuid::now_v7(), 1).await, None);
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&third.id).unwrap()).await.unwrap();
    assert_eq!(job.state, ServerJobState::Failed);
    assert_eq!(job.error.unwrap().code, "lease_lost");

    // A queued job cancelled at once; a running one at its next beat.
    let queued = jobs::create(&db.app, &owner, ask("istek-4", json!({}))).await.unwrap();
    let qid = Uuid::parse_str(&queued.id).unwrap();
    assert_eq!(jobs::cancel(&db.app, &owner, qid).await.unwrap().state, ServerJobState::Cancelled);
    assert_eq!(jobs::work_once(&db.app, &store, worker, LEASE).await.unwrap(), None);
    let running = jobs::create(&db.app, &owner, ask("istek-5", json!({}))).await.unwrap();
    let (rid, fence) = claim(&db, Uuid::now_v7(), 30).await.unwrap();
    assert_eq!(rid.to_string(), running.id);
    let asked = jobs::cancel(&db.app, &owner, rid).await.unwrap();
    assert_eq!((asked.state, asked.cancel_requested), (ServerJobState::Running, true));
    assert_eq!(beat(&db, rid, fence).await, "cancel");
    assert_eq!(jobs::get(&db.app, &owner, rid).await.unwrap().state, ServerJobState::Cancelled);
    // The editor may not cancel the owner's job.
    let other = jobs::create(&db.app, &owner, ask("istek-6", json!({}))).await.unwrap();
    assert!(matches!(
        jobs::cancel(&db.app, &editor, Uuid::parse_str(&other.id).unwrap()).await,
        Err(AppError::Forbidden(_))
    ));
    jobs::cancel(&db.app, &owner, Uuid::parse_str(&other.id).unwrap()).await.unwrap();

    // A job past its time fails at the next claim.
    let slow = jobs::create(
        &db.app,
        &owner,
        ServerJobCreate { timeout_seconds: Some(1), ..ask("istek-7", json!({})) },
    )
    .await
    .unwrap();
    claim(&db, Uuid::now_v7(), 30).await.unwrap();
    tokio::time::sleep(Duration::from_millis(1300)).await;
    assert_eq!(claim(&db, Uuid::now_v7(), 30).await, None);
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&slow.id).unwrap()).await.unwrap();
    assert_eq!((job.state, job.error.unwrap().code.as_str()), (ServerJobState::Failed, "timeout"));

    // An export asked at another revision is refused, nothing staged.
    let stale = jobs::create(&db.app, &owner, ask("istek-8", json!({ "expectedRevision": "0" })))
        .await
        .unwrap();
    let done = jobs::work_once(&db.app, &store, worker, LEASE).await.unwrap().unwrap();
    assert_eq!(done.state, ServerJobState::Failed);
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&stale.id).unwrap()).await.unwrap();
    assert_eq!(job.error.unwrap().code, "revision_conflict");
    assert!(job.artifact.is_none());

    // The editor's grant taken away after asking: the job fails at its run and leaves nothing.
    let theirs = jobs::create(&db.app, &editor, ask("istek-9", json!({}))).await.unwrap();
    revoke(&db, &owner, &ed.actor).await;
    let done = jobs::work_once(&db.app, &store, worker, LEASE).await.unwrap().unwrap();
    assert_eq!(done.state, ServerJobState::Failed);
    let job = jobs::get(&db.app, &owner, Uuid::parse_str(&theirs.id).unwrap()).await.unwrap();
    assert_eq!(job.error.unwrap().code, "forbidden");
    assert!(job.artifact.is_none());

    // An artifact no job holds (a finish that never came) is removed by the store's cleanup; held ones stay.
    let key = kentos_application::blobs::Blobs::job_key(owner.tenant, project, Uuid::now_v7(), &"0".repeat(64));
    let mut w = store.create(&key, 3).await.unwrap();
    w.write(b"abc").await.unwrap();
    w.finish().await.unwrap();
    let cleaned = files::cleanup(&db.app, &store, Duration::ZERO).await.unwrap();
    assert_eq!(cleaned.jobs, 1);
    assert!(store.read(&key).await.is_err());
    assert!(
        jobs::artifact(&db.app, &store, &owner, Uuid::parse_str(&first.id).unwrap())
            .await
            .is_ok(),
        "a held artifact stays"
    );
}
