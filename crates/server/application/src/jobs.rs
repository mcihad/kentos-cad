//! Durable server jobs (docs/adr/0542; plan/gis.md CBS-39).
//!
//! - **Asking:** a user asks for a job in a project (`create`): its kind, its input (kept as given, with its SHA-256)
//!   and the project's data revision then. The same user's same request id gets the same job: a retried request does
//!   not ask twice. A job asks for `jobs.run`; `project.export` also for `project.download`.
//! - **Running:** a worker (`work_once`, `run_worker`) claims the oldest queued job across tenants with a lease and a
//!   fence (`kentos.claim_job`), beats while it works (`kentos.beat_job`: the lease renewed, the progress written; a
//!   cancel or a lost lease stops it), and finishes it as the user who asked, under the project's row-level security,
//!   with the access asked again in the same transaction (`kentos.finish_job`, which only the claim holding the current
//!   fence passes). A worker that dies loses the lease; the next claim takes the job again until its attempts run out.
//! - **Staged:** the artifact is written to the object store before the finish; a finish that does not pass (a lost
//!   lease, a cancel, a refused access) removes it, and the store's cleanup removes any that no job holds.
//! - **Seeing:** the project's users list and read its jobs; the asker or `project.edit` cancels one; `project.download`
//!   downloads a succeeded export.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use kentos_contracts::{
    PROJECT_EXPORT_JOB, ProjectExportInput, ProjectPermission, ProjectStorage, SERVER_JOB_KINDS,
    SERVER_JOB_LIST_MAX, SERVER_JOB_TIMEOUT_DEFAULT, SERVER_JOB_TIMEOUT_MAX, ServerJob,
    ServerJobArtifact, ServerJobCreate, ServerJobError, ServerJobList, ServerJobState,
    SignInMethod,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::{Blobs, WriteError};
use crate::error::{AppError, AppResult};
use crate::projects::{rfc3339, storage_of};

/// A job's row as its project's users read it.
#[derive(sqlx::FromRow)]
struct JobRow {
    id: Uuid,
    project_id: Uuid,
    kind: String,
    input: Value,
    input_sha256: String,
    base_revision: i64,
    state: String,
    cancel_requested: bool,
    attempts: i32,
    max_attempts: i32,
    timeout_seconds: i32,
    progress: f32,
    created_by: Uuid,
    created_at: OffsetDateTime,
    started_at: Option<OffsetDateTime>,
    finished_at: Option<OffsetDateTime>,
    result: Option<Value>,
    artifact_size: Option<i64>,
    artifact_sha256: Option<String>,
    error_code: Option<String>,
    error_message: Option<String>,
}

const JOB_SELECT: &str = "select id, project_id, kind, input, input_sha256, base_revision, state, cancel_requested, attempts, max_attempts, timeout_seconds, progress, created_by, created_at, started_at, finished_at, result, artifact_size, artifact_sha256, error_code, error_message
   from kentos.server_job where tenant_id = $1 and project_id = $2";

fn view(row: JobRow, project_name: &str) -> ServerJob {
    let JobRow {
        id,
        project_id: project,
        kind,
        input,
        input_sha256,
        base_revision,
        state,
        cancel_requested,
        attempts,
        max_attempts,
        timeout_seconds,
        progress,
        created_by,
        created_at,
        started_at,
        finished_at,
        result,
        artifact_size,
        artifact_sha256,
        error_code,
        error_message,
    } = row;
    let state = ServerJobState::of(&state).unwrap_or(ServerJobState::Failed);
    let artifact = match (artifact_size, artifact_sha256) {
        (Some(size), Some(sha256)) => Some(ServerJobArtifact {
            size: size.to_string(),
            sha256,
            name: format!("{project_name}.kcad"),
        }),
        _ => None,
    };
    ServerJob {
        id: id.to_string(),
        project: project.to_string(),
        kind,
        input,
        input_sha256,
        base_revision: base_revision.to_string(),
        state,
        progress,
        attempts: u32::try_from(attempts).unwrap_or(0),
        max_attempts: u32::try_from(max_attempts).unwrap_or(0),
        timeout_seconds: u32::try_from(timeout_seconds).unwrap_or(0),
        cancel_requested,
        created_by: created_by.to_string(),
        created_at: rfc3339(created_at),
        started_at: started_at.map(rfc3339),
        finished_at: finished_at.map(rfc3339),
        result,
        artifact,
        error: error_code.map(|code| ServerJobError {
            code,
            message: error_message.unwrap_or_default(),
        }),
    }
}

fn job_not_found() -> AppError {
    AppError::not_found("İş bulunamadı; projede böyle bir iş yok ya da görme yetkiniz yok.")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The input checked for its kind.
fn checked_input(kind: &str, input: &Value) -> AppResult<Value> {
    match kind {
        PROJECT_EXPORT_JOB => {
            let input = if input.is_null() { json!({}) } else { input.clone() };
            let parsed: ProjectExportInput = serde_json::from_value(input.clone()).map_err(|e| {
                AppError::invalid_at("input", format!("project.export'un girdisi okunamadı: {e}"))
            })?;
            if let Some(r) = &parsed.expected_revision
                && r.parse::<i64>().ok().filter(|n| *n >= 0).is_none()
            {
                return Err(AppError::invalid_at(
                    "input.expectedRevision",
                    format!("expectedRevision bir veri revizyonu (sıfır ya da artı tam sayı) olmalı: {r}"),
                ));
            }
            Ok(input)
        }
        other => Err(AppError::invalid_at(
            "kind",
            format!(
                "“{other}” bir sunucu işi değil; işler: {}.",
                SERVER_JOB_KINDS.join(", ")
            ),
        )),
    }
}

/// Asks for a job (`POST …/projects/{project}/jobs`); a request id the same user sent before for the project gives
/// that job back.
pub async fn create(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    asked: ServerJobCreate,
) -> AppResult<ServerJob> {
    access.live()?;
    let request = asked.request_id.trim();
    if request.is_empty() || request.chars().count() > 200 {
        return Err(AppError::invalid_at(
            "requestId",
            "requestId 1–200 karakter olmalı.",
        ));
    }
    let timeout = asked.timeout_seconds.unwrap_or(SERVER_JOB_TIMEOUT_DEFAULT);
    if !(1..=SERVER_JOB_TIMEOUT_MAX).contains(&timeout) {
        return Err(AppError::invalid_at(
            "timeoutSeconds",
            format!("timeoutSeconds 1 ile {SERVER_JOB_TIMEOUT_MAX} saniye arasında olmalı."),
        ));
    }
    let input = checked_input(&asked.kind, &asked.input)?;
    access.require(ProjectPermission::JobsRun)?;
    if asked.kind == PROJECT_EXPORT_JOB {
        access.require(ProjectPermission::Download)?;
    }
    let digest = sha256_hex(
        serde_json::to_string(&input)
            .unwrap_or_default()
            .as_bytes(),
    );
    let mut tx = db.scoped(access.scope()).await?;
    let (storage, revision): (String, i64) = sqlx::query_as(
        "select storage, data_revision from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(crate::access::not_found)?;
    if asked.kind == PROJECT_EXPORT_JOB && storage_of(&storage) == ProjectStorage::File {
        return Err(AppError::invalid(format!(
            "“{}” projesi dosya olarak saklanıyor; .kcad dosyası zaten onun revizyonlarıdır.",
            access.name
        )));
    }
    let id = Uuid::now_v7();
    sqlx::query(
        "insert into kentos.server_job (tenant_id, project_id, id, kind, input, input_sha256, base_revision, created_by, request_id, timeout_seconds)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         on conflict (tenant_id, project_id, created_by, request_id) do nothing",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(id)
    .bind(&asked.kind)
    .bind(&input)
    .bind(&digest)
    .bind(revision)
    .bind(access.actor.user_id)
    .bind(request)
    .bind(i32::try_from(timeout).unwrap_or(600))
    .execute(&mut *tx)
    .await?;
    let row: JobRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{JOB_SELECT} and created_by = $3 and request_id = $4"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .bind(access.actor.user_id)
    .bind(request)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(view(row, &access.name))
}

/// The project's jobs, newest first.
pub async fn list(db: &kentos_postgres::Db, access: &ProjectAccess) -> AppResult<ServerJobList> {
    let mut tx = db.scoped(access.scope()).await?;
    let rows: Vec<JobRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{JOB_SELECT} order by created_at desc, id desc limit {SERVER_JOB_LIST_MAX}"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ServerJobList {
        jobs: rows.into_iter().map(|r| view(r, &access.name)).collect(),
    })
}

async fn row_of(
    tx: &mut sqlx::Transaction<'static, sqlx::Postgres>,
    access: &ProjectAccess,
    id: Uuid,
) -> AppResult<JobRow> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{JOB_SELECT} and id = $3")))
        .bind(access.tenant)
        .bind(access.project)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(job_not_found)
}

/// One job.
pub async fn get(db: &kentos_postgres::Db, access: &ProjectAccess, id: Uuid) -> AppResult<ServerJob> {
    let mut tx = db.scoped(access.scope()).await?;
    let row = row_of(&mut tx, access, id).await?;
    tx.commit().await?;
    Ok(view(row, &access.name))
}

/// Cancels a job: a queued one at once, a running one at its worker's next beat; one that is over stays as it is.
/// Its asker or a holder of `project.edit` may.
pub async fn cancel(db: &kentos_postgres::Db, access: &ProjectAccess, id: Uuid) -> AppResult<ServerJob> {
    let mut tx = db.scoped(access.scope()).await?;
    let row = row_of(&mut tx, access, id).await?;
    if row.created_by != access.actor.user_id && !access.allows(ProjectPermission::Edit) {
        return Err(AppError::forbidden(
            "Bu işi yalnız isteyen ya da projeyi düzenleme yetkisi olan durdurabilir.",
        ));
    }
    sqlx::query(
        "update kentos.server_job
            set state = case when state = 'queued' then 'cancelled' else state end,
                finished_at = case when state = 'queued' then now() else finished_at end,
                cancel_requested = case when state in ('queued', 'running') then true else cancel_requested end
          where tenant_id = $1 and project_id = $2 and id = $3",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    let row = row_of(&mut tx, access, id).await?;
    tx.commit().await?;
    Ok(view(row, &access.name))
}

/// A succeeded export's file (`project.download`): the job and its open object.
pub async fn artifact(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    id: Uuid,
) -> AppResult<(ServerJob, tokio::fs::File)> {
    access.require(ProjectPermission::Download)?;
    let mut tx = db.scoped(access.scope()).await?;
    let row = row_of(&mut tx, access, id).await?;
    let key: Option<String> = sqlx::query_scalar(
        "select artifact_key from kentos.server_job where tenant_id = $1 and project_id = $2 and id = $3",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let job = view(row, &access.name);
    let Some(key) = key else {
        return Err(AppError::not_found(
            "İşin indirilecek bir sonucu yok; iş bitmemiş ya da başarısız olmuş.",
        ));
    };
    let file = blobs.open(&key).await?;
    Ok((job, file))
}

// ── The worker ──────────────────────────────────────────────────────────────

/// What a worker did with one claim.
#[derive(Clone, Debug, PartialEq)]
pub struct Worked {
    pub job: Uuid,
    /// Where the job ended; `Running` when the worker's finish did not pass (its lease or the job was lost).
    pub state: ServerJobState,
}

/// A claimed job.
struct Claim {
    tenant: Uuid,
    project: Uuid,
    id: Uuid,
    kind: String,
    input: Value,
    created_by: Uuid,
    fence: i64,
}

/// What a job's run ended in, before its finish.
enum Outcome {
    Done { result: Value, key: String, size: i64, sha256: String },
    Failed { code: &'static str, message: String },
    /// Asked to stop, or the lease lost: no finish.
    Stopped,
}

fn failed_of(e: &AppError) -> (&'static str, String) {
    let code = match e {
        AppError::Forbidden(_) | AppError::NotFound(_) | AppError::Unauthenticated(_) => "forbidden",
        AppError::Deleted(_) => "gone",
        AppError::Invalid { .. } => "invalid",
        AppError::Storage(_) => "storage",
        AppError::Conflict { .. } => "revision_conflict",
        _ => "failed",
    };
    (code, e.to_string())
}

/// Claims one job of `kinds` for `owner` and runs it to its end; none when the queue is empty.
pub async fn work_once(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    owner: Uuid,
    lease: Duration,
) -> AppResult<Option<Worked>> {
    /// `claim_job`'s row: tenant, project, job, kind, input, base revision, asker, fence, attempts.
    type Claimed = (Uuid, Uuid, Uuid, String, Value, i64, Uuid, i64, i32);
    let secs = i32::try_from(lease.as_secs().clamp(1, 3600)).unwrap_or(30);
    let kinds: Vec<String> = SERVER_JOB_KINDS.iter().map(|k| (*k).to_owned()).collect();
    let row: Option<Claimed> = sqlx::query_as(
        "select tenant_id, project_id, id, kind, input, base_revision, created_by, fence, attempts from kentos.claim_job($1, $2, $3)",
    )
    .bind(owner)
    .bind(secs)
    .bind(&kinds)
    .fetch_optional(&db.pool)
    .await?;
    let Some((tenant, project, id, kind, input, _base, created_by, fence, _attempts)) = row else {
        return Ok(None);
    };
    let claim = Claim {
        tenant,
        project,
        id,
        kind,
        input,
        created_by,
        fence,
    };
    // The beat: the lease renewed every third of it; a cancel or a lost lease stops the run.
    let stop = Arc::new(AtomicBool::new(false));
    let beat = {
        let db = db.clone();
        let stop = stop.clone();
        tokio::spawn(async move {
            let every = Duration::from_millis((lease.as_millis() / 3).clamp(200, 60_000) as u64);
            loop {
                tokio::time::sleep(every).await;
                let answer: Result<String, _> =
                    sqlx::query_scalar("select kentos.beat_job($1, $2, null, $3)")
                        .bind(id)
                        .bind(fence)
                        .bind(secs)
                        .fetch_one(&db.pool)
                        .await;
                if !matches!(answer.as_deref(), Ok("run")) {
                    stop.store(true, Ordering::SeqCst);
                    return;
                }
            }
        })
    };
    let outcome = run(db, blobs, &claim, &stop).await;
    beat.abort();
    let state = finish(db, blobs, &claim, outcome).await?;
    Ok(Some(Worked { job: id, state }))
}

/// The run of one claim, the access asked as the user who asked for it.
async fn run(db: &kentos_postgres::Db, blobs: &Blobs, claim: &Claim, stop: &AtomicBool) -> Outcome {
    let access = match actor_access(db, claim).await {
        Ok(a) => a,
        Err(e) => {
            let (code, message) = failed_of(&e);
            return Outcome::Failed { code, message };
        }
    };
    match claim.kind.as_str() {
        PROJECT_EXPORT_JOB => export(db, blobs, claim, &access, stop).await,
        other => Outcome::Failed {
            code: "invalid",
            message: format!("“{other}” bu sunucunun bilmediği bir iş."),
        },
    }
}

async fn actor_access(db: &kentos_postgres::Db, claim: &Claim) -> AppResult<ProjectAccess> {
    let actor = crate::identity::actor_of(db, claim.created_by, SignInMethod::Local)
        .await?
        .ok_or_else(|| AppError::forbidden("İşi isteyen kullanıcı artık yok."))?;
    crate::access::project(db, &actor, claim.tenant, claim.project).await
}

/// `project.export`: the project as one `.kcad`, written to the object store as the job's artifact.
async fn export(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    claim: &Claim,
    access: &ProjectAccess,
    stop: &AtomicBool,
) -> Outcome {
    if let Err(e) = access
        .live()
        .and_then(|()| access.require(ProjectPermission::JobsRun))
        .and_then(|()| access.require(ProjectPermission::Download))
    {
        let (code, message) = failed_of(&e);
        return Outcome::Failed { code, message };
    }
    let input: ProjectExportInput = serde_json::from_value(claim.input.clone()).unwrap_or_default();
    let snap = match crate::snapshot::take(db, access).await {
        Ok(s) => s,
        Err(e) => {
            let (code, message) = failed_of(&e);
            return Outcome::Failed { code, message };
        }
    };
    if let Some(expected) = input.expected_revision.as_deref()
        && expected != snap.revision.to_string()
    {
        return Outcome::Failed {
            code: "revision_conflict",
            message: format!(
                "Proje {expected}. revizyonda değil ({}); dışa aktarma yapılmadı. İşi şimdiki revizyonla yeniden isteyin.",
                snap.revision
            ),
        };
    }
    if stop.load(Ordering::SeqCst) {
        return Outcome::Stopped;
    }
    let key = Blobs::job_key(claim.tenant, claim.project, claim.id, &snap.sha256);
    let size = snap.bytes.len();
    let written = async {
        let mut w = blobs.create(&key, size as u64).await?;
        w.write(&snap.bytes).await.map_err(|e| match e {
            WriteError::Io(e) => e,
            other => std::io::Error::other(format!("{other:?}")),
        })?;
        w.finish().await
    }
    .await;
    if let Err(e) = written {
        let _ = blobs.remove(&key).await;
        return Outcome::Failed {
            code: "storage",
            message: format!("Sonuç nesne deposuna yazılamadı: {e}"),
        };
    }
    Outcome::Done {
        result: json!({
            "revision": snap.revision.to_string(),
            "objects": snap.objects,
            "eventCursor": snap.event_cursor.to_string(),
        }),
        key,
        size: i64::try_from(size).unwrap_or(i64::MAX),
        sha256: snap.sha256,
    }
}

/// Finishes the claim: a done run as the user who asked, with the access asked again in the same transaction; a
/// failed one with its code. A finish that does not pass leaves the job as it is and removes the artifact.
async fn finish(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    claim: &Claim,
    outcome: Outcome,
) -> AppResult<ServerJobState> {
    match outcome {
        Outcome::Stopped => Ok(ServerJobState::Running),
        Outcome::Failed { code, message } => {
            let passed: bool = sqlx::query_scalar(
                "select kentos.finish_job($1, $2, false, null, null, null, null, $3, $4)",
            )
            .bind(claim.id)
            .bind(claim.fence)
            .bind(code)
            .bind(&message)
            .fetch_one(&db.pool)
            .await?;
            Ok(if passed {
                ServerJobState::Failed
            } else {
                ServerJobState::Running
            })
        }
        Outcome::Done {
            result,
            key,
            size,
            sha256,
        } => {
            // The access asked again at the commit: a grant taken away while the job ran does not let it in.
            let access = match actor_access(db, claim).await {
                Ok(a) => a,
                Err(e) => {
                    let _ = blobs.remove(&key).await;
                    let (code, message) = failed_of(&e);
                    return Box::pin(finish(db, blobs, claim, Outcome::Failed { code, message })).await;
                }
            };
            let mut tx = db.scoped(access.scope()).await?;
            let again = crate::access::recheck(&mut tx, &access)
                .await
                .and_then(|a| {
                    a.live()?;
                    a.require(ProjectPermission::JobsRun)?;
                    a.require(ProjectPermission::Download)
                });
            if let Err(e) = again {
                drop(tx);
                let _ = blobs.remove(&key).await;
                let (code, message) = failed_of(&e);
                return Box::pin(finish(db, blobs, claim, Outcome::Failed { code, message })).await;
            }
            let passed: bool = sqlx::query_scalar(
                "select kentos.finish_job($1, $2, true, $3, $4, $5, $6, null, null)",
            )
            .bind(claim.id)
            .bind(claim.fence)
            .bind(&result)
            .bind(&key)
            .bind(size)
            .bind(&sha256)
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            if passed {
                Ok(ServerJobState::Succeeded)
            } else {
                let _ = blobs.remove(&key).await;
                Ok(ServerJobState::Running)
            }
        }
    }
}

/// Runs jobs until `stop` is set: one after another while there are any, then waiting `idle` between looks; `said`
/// hears what each claim ended in (the caller logs it).
pub async fn run_worker(
    db: kentos_postgres::Db,
    blobs: Blobs,
    owner: Uuid,
    lease: Duration,
    idle: Duration,
    stop: Arc<AtomicBool>,
    said: impl Fn(AppResult<Worked>),
) {
    while !stop.load(Ordering::SeqCst) {
        match work_once(&db, &blobs, owner, lease).await {
            Ok(Some(w)) => said(Ok(w)),
            Ok(None) => tokio::time::sleep(idle).await,
            Err(e) => {
                said(Err(e));
                tokio::time::sleep(idle).await;
            }
        }
    }
}
