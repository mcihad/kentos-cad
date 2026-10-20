//! The upkeep of durable server jobs (docs/adr/0547): the retention of finished jobs and the queue's numbers for the
//! operator.
//!
//! - **Retention:** a finished job (succeeded, failed or cancelled) is kept `retention` after it ended, then its row
//!   goes and its artifact with it. The rows go first: an object the store fails to remove belongs to no job any more,
//!   and the object store's cleanup (`files::cleanup`) removes it.
//! - **Numbers:** how many jobs wait and run, how many ended in the last day, how long the oldest waiting one has
//!   waited, and which organisations have jobs under way. No job's input, result or name leaves here.

use std::time::Duration;

use sqlx::PgPool;
use uuid::Uuid;

use crate::blobs::Blobs;
use crate::error::AppResult;

/// What a retention sweep removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Expired {
    /// Finished jobs removed.
    pub jobs: usize,
    /// Their artifacts removed from the object store.
    pub objects: usize,
    /// Staged files of jobs removed (their rows and their objects, docs/adr/0561 §1).
    pub inputs: usize,
}

/// Removes finished jobs that ended more than `retention` ago and their artifacts, `batch` jobs at a time.
pub async fn expire(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    retention: Duration,
    batch: i32,
) -> AppResult<Expired> {
    let secs = retention.as_secs_f64();
    let batch = batch.max(1);
    let mut done = Expired::default();
    loop {
        let keys: Vec<Option<String>> = sqlx::query_scalar(
            "select artifact_key from kentos.expire_jobs(now() - make_interval(secs => $1), $2)",
        )
        .bind(secs)
        .bind(batch)
        .fetch_all(&db.pool)
        .await?;
        let round = keys.len();
        done.jobs += round;
        for key in keys.into_iter().flatten() {
            if blobs.remove(&key).await.is_ok() {
                done.objects += 1;
            }
        }
        if round < usize::try_from(batch).unwrap_or(1) {
            break;
        }
    }
    // The staged files are as old as the retention, whatever became of their jobs: the job that read one has been over
    // for as long, or never was.
    loop {
        let keys: Vec<String> = sqlx::query_scalar(
            "select blob_key from kentos.expire_job_inputs(now() - make_interval(secs => $1), $2)",
        )
        .bind(secs)
        .bind(batch)
        .fetch_all(&db.pool)
        .await?;
        let round = keys.len();
        for key in keys {
            if blobs.remove(&key).await.is_ok() {
                done.inputs += 1;
            }
        }
        if round < usize::try_from(batch).unwrap_or(1) {
            return Ok(done);
        }
    }
}

/// The queue as numbers, for the operator.
#[derive(Clone, Debug, PartialEq)]
pub struct JobMetrics {
    pub queued: i64,
    pub running: i64,
    pub failed_24h: i64,
    pub succeeded_24h: i64,
    pub cancelled_24h: i64,
    /// How long the oldest queued job has waited, in seconds; none when nothing waits.
    pub oldest_queued_seconds: Option<f64>,
    /// The organisations with queued or running jobs: (tenant id, queued, running), the busiest first.
    pub tenants: Vec<(Uuid, i64, i64)>,
}

/// Reads the queue's numbers (`kentosd jobs`).
pub async fn metrics(pool: &PgPool) -> AppResult<JobMetrics> {
    let (queued, running, failed_24h, succeeded_24h, cancelled_24h, oldest): (
        i64,
        i64,
        i64,
        i64,
        i64,
        Option<f64>,
    ) = sqlx::query_as(
        "select queued, running, failed_24h, succeeded_24h, cancelled_24h, oldest_queued_seconds from kentos.job_metrics()",
    )
    .fetch_one(pool)
    .await?;
    let tenants: Vec<(Uuid, i64, i64)> =
        sqlx::query_as("select tenant_id, queued, running from kentos.job_load()")
            .fetch_all(pool)
            .await?;
    Ok(JobMetrics {
        queued,
        running,
        failed_24h,
        succeeded_24h,
        cancelled_24h,
        oldest_queued_seconds: oldest,
        tenants,
    })
}
