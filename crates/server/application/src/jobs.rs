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
//! - **Limits (docs/adr/0547):** an organisation has at most `JobLimits::tenant_active` jobs queued and running (a
//!   request past it is refused with `quota_exceeded`), an artifact past `artifact_max` fails its job (`too_large`), an
//!   artifact that cannot be written fails it with `storage` and leaves no object, and `expire` removes finished jobs
//!   (and their artifacts) older than the retention.
//! - **Processing (docs/adr/0559):** `project.processing` runs an İşlemler tool through the host's [`Processor`] (the
//!   tools are not the server's) on the project as of a revision. The worker writes its progress and words at its
//!   beats; the tool has a CPU and memory budget. A result to commit is committed in the job's finish transaction,
//!   with the project required to be at the revision the tool read, so a lost lease or a conflict leaves nothing
//!   written; a result to a file is staged like an export's.
//! - **Seeing:** the project's users list and read its jobs; the asker or `project.edit` cancels one; `project.download`
//!   downloads a succeeded export.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use kentos_contracts::{
    CheckpointCreate, CommandEnvelope, PROJECT_CHANGES, PROJECT_CHANGES_VERSION,
    PROJECT_CHECKPOINT_CREATE, PROJECT_CHECKPOINT_CREATE_VERSION, PROJECT_CHECKPOINT_JOB,
    PROJECT_EXPORT_JOB, PROJECT_META_KEY, PROJECT_PROCESSING_JOB, ProcessingJobInput,
    ProcessingJobMode, ProcessingJobResult, ProjectExportInput, ProjectPermission,
    ProjectStorage, SERVER_JOB_ARTIFACT_MAX_DEFAULT, SERVER_JOB_CPU_DEFAULT_SECONDS,
    SERVER_JOB_KINDS, SERVER_JOB_LIST_MAX, SERVER_JOB_MEMORY_DEFAULT_MB,
    SERVER_JOB_TENANT_ACTIVE_DEFAULT, SERVER_JOB_TIMEOUT_DEFAULT, SERVER_JOB_TIMEOUT_MAX,
    ServerJob, ServerJobArtifact, ServerJobCreate, ServerJobError, ServerJobList, ServerJobState,
    SignInMethod,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::{Blobs, WriteError};
use crate::error::{AppError, AppResult};
use crate::processing_job::{
    Delta, JobProgress, ProcessHooks, ProcessingTask, Processor, budget_of, check_input,
};
use crate::projects::{rfc3339, storage_of};

/// What the server lets jobs take (docs/adr/0547): the server sets them (`KENTOS_JOB_LIMIT`,
/// `KENTOS_JOB_ARTIFACT_MAX_MB`), the defaults hold for tests and for a server that sets none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobLimits {
    /// Queued and running jobs of one organisation together.
    pub tenant_active: u32,
    /// The biggest artifact a job may stage, in bytes.
    pub artifact_max: u64,
    /// What one `project.processing` job may take (docs/adr/0559 §4): CPU seconds of the process the tool runs in,
    /// and its resident memory in megabytes. A request may ask for less, never for more.
    pub cpu_seconds: u32,
    pub memory_mb: u32,
}

impl Default for JobLimits {
    fn default() -> Self {
        Self {
            tenant_active: SERVER_JOB_TENANT_ACTIVE_DEFAULT,
            artifact_max: SERVER_JOB_ARTIFACT_MAX_DEFAULT,
            cpu_seconds: SERVER_JOB_CPU_DEFAULT_SECONDS,
            memory_mb: SERVER_JOB_MEMORY_DEFAULT_MB,
        }
    }
}

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
    progress_text: Option<String>,
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

const JOB_SELECT: &str = "select id, project_id, kind, input, input_sha256, base_revision, state, cancel_requested, attempts, max_attempts, timeout_seconds, progress, progress_text, created_by, created_at, started_at, finished_at, result, artifact_size, artifact_sha256, error_code, error_message
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
        progress_text,
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
            name: artifact_name(&kind, &input, project_name),
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
        progress_text,
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

/// The file name a job's artifact is downloaded under: the project's name and `.kcad` for an export; the project's
/// name, the tool's name and `.gpkg` for a processing job's file (docs/adr/0559 §2).
fn artifact_name(kind: &str, input: &Value, project: &str) -> String {
    if kind != PROJECT_PROCESSING_JOB {
        return format!("{project}.kcad");
    }
    let tool = input["label"]
        .as_str()
        .filter(|l| !l.trim().is_empty())
        .or_else(|| input["tool"].as_str())
        .unwrap_or("islem");
    let clean = |text: &str| -> String {
        text.chars()
            .map(|c| {
                if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                    '-'
                } else {
                    c
                }
            })
            .collect()
    };
    format!("{} - {}.gpkg", clean(project), clean(tool))
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
            let input = if input.is_null() {
                json!({})
            } else {
                input.clone()
            };
            let parsed: ProjectExportInput =
                serde_json::from_value(input.clone()).map_err(|e| {
                    AppError::invalid_at(
                        "input",
                        format!("project.export'un girdisi okunamadı: {e}"),
                    )
                })?;
            if let Some(r) = &parsed.expected_revision
                && r.parse::<i64>().ok().filter(|n| *n >= 0).is_none()
            {
                return Err(AppError::invalid_at(
                    "input.expectedRevision",
                    format!(
                        "expectedRevision bir veri revizyonu (sıfır ya da artı tam sayı) olmalı: {r}"
                    ),
                ));
            }
            Ok(input)
        }
        PROJECT_CHECKPOINT_JOB => {
            let parsed: CheckpointCreate = serde_json::from_value(input.clone()).map_err(|e| {
                AppError::invalid_at(
                    "input",
                    format!("project.checkpoint'in girdisi okunamadı: {e}"),
                )
            })?;
            crate::checkpoints::name_of(&parsed.name).map_err(|e| e.at("input.name"))?;
            crate::checkpoints::note_of(parsed.note.clone()).map_err(|e| e.at("input.note"))?;
            if let Some(r) = &parsed.file_revision
                && r.parse::<i64>().ok().filter(|n| *n > 0).is_none()
            {
                return Err(AppError::invalid_at(
                    "input.fileRevision",
                    format!("fileRevision bir revizyon numarası olmalı: {r}"),
                ));
            }
            Ok(input.clone())
        }
        PROJECT_PROCESSING_JOB => {
            let parsed: ProcessingJobInput =
                serde_json::from_value(input.clone()).map_err(|e| {
                    AppError::invalid_at(
                        "input",
                        format!("project.processing'in girdisi okunamadı: {e}"),
                    )
                })?;
            if let Some((path, message)) = check_input(&parsed) {
                return Err(AppError::invalid_at(path, message));
            }
            Ok(input.clone())
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
    limits: &JobLimits,
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
    if asked.kind == PROJECT_CHECKPOINT_JOB {
        access.writable()?;
        access.require(ProjectPermission::FeatureWrite)?;
    }
    if asked.kind == PROJECT_PROCESSING_JOB {
        let parsed: ProcessingJobInput = serde_json::from_value(input.clone()).unwrap_or_default();
        match parsed.mode {
            // The result is written to the project: as `project.changes` asks, the right to write objects (and to
            // edit the project's layers, asked again at the commit when the result carries them).
            ProcessingJobMode::Commit => {
                access.writable()?;
                access.require(ProjectPermission::FeatureWrite)?;
            }
            // The result is a file of the project's data: as an export's.
            ProcessingJobMode::Sink => access.require(ProjectPermission::Download)?,
        }
    }
    let digest = sha256_hex(serde_json::to_string(&input).unwrap_or_default().as_bytes());
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
    if asked.kind == PROJECT_PROCESSING_JOB && storage_of(&storage) == ProjectStorage::File {
        return Err(AppError::invalid(format!(
            "“{}” projesi dosya olarak saklanıyor; araçlar sunucuda yalnız veritabanı projesinde çalışır. Projeyi bilgisayarınızda açıp aracı orada çalıştırın.",
            access.name
        )));
    }
    // A retried request gets its job back whatever the organisation's load; a new one needs a place.
    let earlier: Option<Uuid> = sqlx::query_scalar(
        "select id from kentos.server_job where tenant_id = $1 and project_id = $2 and created_by = $3 and request_id = $4",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(access.actor.user_id)
    .bind(request)
    .fetch_optional(&mut *tx)
    .await?;
    if earlier.is_none() {
        let room: bool = sqlx::query_scalar("select kentos.job_slot($1, $2)")
            .bind(access.tenant)
            .bind(i32::try_from(limits.tenant_active).unwrap_or(i32::MAX))
            .fetch_one(&mut *tx)
            .await?;
        if !room {
            return Err(AppError::Quota(format!(
                "Kurumunuzun sırada ve çalışan iş sınırına ({}) ulaşıldı; bir iş bitince ya da durdurulunca yeniden isteyin.",
                limits.tenant_active
            )));
        }
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
pub async fn get(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    id: Uuid,
) -> AppResult<ServerJob> {
    let mut tx = db.scoped(access.scope()).await?;
    let row = row_of(&mut tx, access, id).await?;
    tx.commit().await?;
    Ok(view(row, &access.name))
}

/// Cancels a job: a queued one at once, a running one at its worker's next beat; one that is over stays as it is.
/// Its asker or a holder of `project.edit` may.
pub async fn cancel(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    id: Uuid,
) -> AppResult<ServerJob> {
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
    /// `artifact`: the staged object (key, size, SHA-256) when the kind has one.
    Done {
        result: Value,
        artifact: Option<(String, i64, String)>,
        /// What the result was cut by (docs/adr/0550 §5): the stamp of the caller's reach it was built
        /// for; the finish asks it again and refuses a result the caller no longer may take.
        reach: Option<String>,
        /// The result a `project.processing` job commits in its finish (docs/adr/0559 §2).
        commit: Option<Box<Staged>>,
    },
    Failed {
        code: &'static str,
        message: String,
    },
    /// Asked to stop, or the lease lost: no finish.
    Stopped,
}

/// What a processing job commits: the delta and the data revision the tool read, which the project must still be at.
struct Staged {
    delta: Delta,
    revision: i64,
}

/// The code of a failed run as a job's error code.
fn static_code(code: &str) -> &'static str {
    match code {
        "tool_failed" => "tool_failed",
        "tool_unavailable" => "tool_unavailable",
        "cpu_limit" => "cpu_limit",
        "memory_limit" => "memory_limit",
        "memory_estimate" => "memory_estimate",
        "unsupported_change" => "unsupported_change",
        "too_many_changes" => "too_many_changes",
        "invalid" => "invalid",
        "crashed" => "crashed",
        _ => "failed",
    }
}

fn failed_of(e: &AppError) -> (&'static str, String) {
    let code = match e {
        AppError::Forbidden(_) | AppError::NotFound(_) | AppError::Unauthenticated(_) => {
            "forbidden"
        }
        AppError::Deleted(_) => "gone",
        AppError::Archived(_) => "archived",
        AppError::Invalid { .. } => "invalid",
        AppError::Storage(_) => "storage",
        AppError::Conflict { .. } => "revision_conflict",
        _ => "failed",
    };
    (code, e.to_string())
}

/// Claims one job for `owner` and runs it to its end; none when the queue is empty. A worker with no processor does
/// not claim `project.processing` jobs.
pub async fn work_once(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    owner: Uuid,
    lease: Duration,
    limits: &JobLimits,
) -> AppResult<Option<Worked>> {
    work_once_with(db, blobs, owner, lease, limits, None).await
}

/// [`work_once`] for a worker whose host runs İşlemler's tools (`processor`, docs/adr/0559 §3).
pub async fn work_once_with(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    owner: Uuid,
    lease: Duration,
    limits: &JobLimits,
    processor: Option<Arc<dyn Processor>>,
) -> AppResult<Option<Worked>> {
    work_once_between_with(db, blobs, owner, lease, limits, processor, || async {}).await
}

/// [`work_once`] with something done between the run and its finish: what a test does to the project
/// while a job runs (a grant taken away, a field marked), the moment the finish asks everything again.
#[doc(hidden)]
pub async fn work_once_between<F, Fut>(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    owner: Uuid,
    lease: Duration,
    limits: &JobLimits,
    between: F,
) -> AppResult<Option<Worked>>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    work_once_between_with(db, blobs, owner, lease, limits, None, between).await
}

/// [`work_once_between`] for a worker with a processor.
#[doc(hidden)]
pub async fn work_once_between_with<F, Fut>(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    owner: Uuid,
    lease: Duration,
    limits: &JobLimits,
    processor: Option<Arc<dyn Processor>>,
    between: F,
) -> AppResult<Option<Worked>>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    /// `claim_job`'s row: tenant, project, job, kind, input, base revision, asker, fence, attempts.
    type Claimed = (Uuid, Uuid, Uuid, String, Value, i64, Uuid, i64, i32);
    let secs = i32::try_from(lease.as_secs().clamp(1, 3600)).unwrap_or(30);
    let kinds: Vec<String> = SERVER_JOB_KINDS
        .iter()
        .filter(|k| processor.is_some() || **k != PROJECT_PROCESSING_JOB)
        .map(|k| (*k).to_owned())
        .collect();
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
    // The beat: the lease renewed every third of it (at least every two seconds, so the progress the run says is
    // read soon); a cancel or a lost lease stops the run.
    let stop = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(JobProgress::default());
    let beat = {
        let db = db.clone();
        let stop = stop.clone();
        let progress = progress.clone();
        tokio::spawn(async move {
            let every = Duration::from_millis(
                (lease.as_millis() / 3).clamp(100, 2_000) as u64,
            );
            loop {
                tokio::time::sleep(every).await;
                let (fraction, said) = match progress.get() {
                    Some((f, t)) => (Some(f), Some(t)),
                    None => (None, None),
                };
                let answer: Result<String, _> =
                    sqlx::query_scalar("select kentos.beat_job($1, $2, $3, $4, $5)")
                        .bind(id)
                        .bind(fence)
                        .bind(fraction)
                        .bind(secs)
                        .bind(said)
                        .fetch_one(&db.pool)
                        .await;
                if !matches!(answer.as_deref(), Ok("run")) {
                    stop.store(true, Ordering::SeqCst);
                    return;
                }
            }
        })
    };
    let outcome = run(
        db,
        blobs,
        &claim,
        &stop,
        limits,
        processor.as_ref(),
        &progress,
    )
    .await;
    beat.abort();
    between().await;
    let state = finish(db, blobs, &claim, outcome).await?;
    Ok(Some(Worked { job: id, state }))
}

/// The run of one claim, the access asked as the user who asked for it.
async fn run(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    claim: &Claim,
    stop: &Arc<AtomicBool>,
    limits: &JobLimits,
    processor: Option<&Arc<dyn Processor>>,
    progress: &Arc<JobProgress>,
) -> Outcome {
    let access = match actor_access(db, claim).await {
        Ok(a) => a,
        Err(e) => {
            let (code, message) = failed_of(&e);
            return Outcome::Failed { code, message };
        }
    };
    match claim.kind.as_str() {
        PROJECT_EXPORT_JOB => export(db, blobs, claim, &access, stop, limits).await,
        PROJECT_CHECKPOINT_JOB => checkpoint(db, blobs, claim, &access, stop).await,
        PROJECT_PROCESSING_JOB => {
            processing(
                db, blobs, claim, &access, stop, limits, processor, progress,
            )
            .await
        }
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
    limits: &JobLimits,
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
    let snap = match crate::snapshot::take(db, access, crate::snapshot::Take::Cut).await {
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
    let (key, size) = match stage(blobs, claim, &snap.bytes, &snap.sha256, limits, "Projenin dosyası").await
    {
        Ok(staged) => staged,
        Err(failed) => return failed,
    };
    Outcome::Done {
        result: json!({
            "revision": snap.revision.to_string(),
            "objects": snap.objects,
            "eventCursor": snap.event_cursor.to_string(),
        }),
        artifact: Some((key, size, snap.sha256)),
        reach: snap.reach,
        commit: None,
    }
}

/// Writes `bytes` to the object store as the job's artifact (docs/adr/0547 §2): past the server's limit the job fails
/// with `too_large`, a file that cannot be written fails it with `storage` and leaves no object. `what` names the file
/// in the message.
async fn stage(
    blobs: &Blobs,
    claim: &Claim,
    bytes: &[u8],
    sha256: &str,
    limits: &JobLimits,
    what: &str,
) -> Result<(String, i64), Outcome> {
    let size = bytes.len();
    if size as u64 > limits.artifact_max {
        return Err(Outcome::Failed {
            code: "too_large",
            message: format!(
                "{what} {size} bayt; sunucunun bir işe verdiği sınır {} bayt. Sunucu yöneticisine başvurun.",
                limits.artifact_max
            ),
        });
    }
    let key = Blobs::job_key(claim.tenant, claim.project, claim.id, sha256);
    let written = async {
        let mut w = blobs.create(&key, size as u64).await?;
        w.write(bytes).await.map_err(|e| match e {
            WriteError::Io(e) => e,
            other => std::io::Error::other(format!("{other:?}")),
        })?;
        w.finish().await
    }
    .await;
    if let Err(e) = written {
        let _ = blobs.remove(&key).await;
        return Err(Outcome::Failed {
            code: "storage",
            message: format!("Sonuç nesne deposuna yazılamadı: {e}"),
        });
    }
    Ok((key, i64::try_from(size).unwrap_or(i64::MAX)))
}

/// `project.processing`: the project read at one revision, the tool run by the host's processor under its budget, the
/// result a delta to commit in the finish or a GeoPackage staged as the artifact (docs/adr/0559).
#[allow(clippy::too_many_arguments)]
async fn processing(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    claim: &Claim,
    access: &ProjectAccess,
    stop: &Arc<AtomicBool>,
    limits: &JobLimits,
    processor: Option<&Arc<dyn Processor>>,
    progress: &Arc<JobProgress>,
) -> Outcome {
    let failed = |code: &'static str, message: String| Outcome::Failed { code, message };
    let Some(processor) = processor else {
        return failed(
            "tool_unavailable",
            "Bu sunucu işçisi İşlemler araçlarını çalıştıramıyor.".into(),
        );
    };
    let input: ProcessingJobInput = match serde_json::from_value(claim.input.clone()) {
        Ok(i) => i,
        Err(e) => return failed("invalid", format!("İşin girdisi okunamadı: {e}")),
    };
    let rights = access
        .live()
        .and_then(|()| access.require(ProjectPermission::JobsRun))
        .and_then(|()| match input.mode {
            ProcessingJobMode::Commit => access
                .writable()
                .and_then(|()| access.require(ProjectPermission::FeatureWrite)),
            ProcessingJobMode::Sink => access.require(ProjectPermission::Download),
        });
    if let Err(e) = rights {
        let (code, message) = failed_of(&e);
        return Outcome::Failed { code, message };
    }
    progress.set(0.01, "Proje okunuyor");
    let snap = match crate::snapshot::take(db, access, crate::snapshot::Take::Cut).await {
        Ok(s) => s,
        Err(e) => {
            let (code, message) = failed_of(&e);
            return Outcome::Failed { code, message };
        }
    };
    if let Some(expected) = input.expected_revision.as_deref()
        && expected != snap.revision.to_string()
    {
        return failed(
            "revision_conflict",
            format!(
                "Proje {expected}. revizyonda değil ({}); araç çalıştırılmadı. İşi şimdiki revizyonla yeniden isteyin.",
                snap.revision
            ),
        );
    }
    // A copy cut by the forms' marks is not the project: what a tool computes on it must not be written back.
    if input.mode == ProcessingJobMode::Commit && snap.cut {
        return failed(
            "forbidden",
            "Formların gizlediği alanlar var ve onları görme yetkiniz yok; bu kopya üzerinde hesaplanan sonuç projeye yazılırsa o alanlar bozulur. Sonucu dosyaya (sink) yazdırın.".into(),
        );
    }
    let budget = budget_of(&input, limits);
    let need = kentos_contracts::processing_memory_estimate(snap.bytes.len() as u64);
    if need > budget.memory_bytes {
        return failed(
            "memory_estimate",
            format!(
                "Projenin dosyası {} bayt; aracın bunun için yaklaşık {} MB belleğe ihtiyacı olur, bu işin bellek bütçesi {} MB. İşe daha büyük bir memoryMb vermeyi deneyin (sunucunun sınırını aşamaz) ya da sunucu yöneticisine başvurun.",
                snap.bytes.len(),
                need / (1024 * 1024),
                budget.memory_bytes / (1024 * 1024)
            ),
        );
    }
    if stop.load(Ordering::SeqCst) {
        return Outcome::Stopped;
    }
    progress.set(0.04, "Araç hazırlanıyor");
    let task = ProcessingTask {
        job: claim.id,
        snapshot: snap.bytes,
        input: input.clone(),
        budget,
    };
    let hooks = ProcessHooks {
        progress: progress.clone(),
        stop: stop.clone(),
    };
    let processor = processor.clone();
    let ran = tokio::task::spawn_blocking(move || processor.process(&task, &hooks)).await;
    let processed = match ran {
        Ok(Ok(p)) => p,
        Ok(Err(f)) if f.code == "stopped" => return Outcome::Stopped,
        Ok(Err(f)) => return failed(static_code(&f.code), f.message),
        Err(e) => {
            return failed("crashed", format!("Aracın çalışması yarıda kaldı: {e}"));
        }
    };
    if stop.load(Ordering::SeqCst) {
        return Outcome::Stopped;
    }
    let mut result = ProcessingJobResult {
        tool: input.tool.clone(),
        mode: input.mode,
        base_revision: snap.revision.to_string(),
        revision: None,
        summary: processed.summary.clone(),
        added: 0,
        changed: 0,
        removed: 0,
        metadata: false,
        notes: processed.notes.clone(),
        usage: processed.usage.clone(),
    };
    match input.mode {
        ProcessingJobMode::Commit => {
            let Some(delta) = processed.delta else {
                return failed(
                    "failed",
                    "İşçi aracın sonucunu vermedi; hiçbir şey yazılmadı.".into(),
                );
            };
            result.added = delta.added;
            result.changed = delta.changed;
            result.removed = delta.removed;
            result.metadata = delta.project.is_some();
            progress.set(0.95, "Sonuç projeye yazılıyor");
            Outcome::Done {
                result: serde_json::to_value(&result).unwrap_or(Value::Null),
                artifact: None,
                reach: snap.reach,
                commit: Some(Box::new(Staged {
                    delta,
                    revision: snap.revision,
                })),
            }
        }
        ProcessingJobMode::Sink => {
            let Some(bytes) = processed.sink else {
                return failed(
                    "failed",
                    "İşçi sonuç dosyasını vermedi; hiçbir şey yazılmadı.".into(),
                );
            };
            result.added = processed.sink_rows;
            progress.set(0.95, "Sonuç dosyası saklanıyor");
            let sha256 = sha256_hex(&bytes);
            let (key, size) = match stage(blobs, claim, &bytes, &sha256, limits, "Sonuç dosyası").await {
                Ok(staged) => staged,
                Err(failed) => return failed,
            };
            Outcome::Done {
                result: serde_json::to_value(&result).unwrap_or(Value::Null),
                artifact: Some((key, size, sha256)),
                reach: snap.reach,
                commit: None,
            }
        }
    }
}

/// `project.checkpoint`: the checkpoint command run as the user who asked, its idempotency key the job's id, so a
/// run the lease took away from its worker and a second run make one checkpoint (the second gets the stored answer).
async fn checkpoint(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    claim: &Claim,
    access: &ProjectAccess,
    stop: &AtomicBool,
) -> Outcome {
    if let Err(e) = access
        .live()
        .and_then(|()| access.require(ProjectPermission::JobsRun))
    {
        let (code, message) = failed_of(&e);
        return Outcome::Failed { code, message };
    }
    if stop.load(Ordering::SeqCst) {
        return Outcome::Stopped;
    }
    let envelope = CommandEnvelope {
        command_name: PROJECT_CHECKPOINT_CREATE.into(),
        version: PROJECT_CHECKPOINT_CREATE_VERSION,
        tenant_id: claim.tenant.to_string(),
        project_id: claim.project.to_string(),
        request_id: format!("job-{}", claim.id),
        idempotency_key: format!("job-{}", claim.id),
        expected_versions: Default::default(),
        input: claim.input.clone(),
    };
    match crate::checkpoints::create(db, blobs, access, envelope).await {
        Ok(change) => Outcome::Done {
            result: serde_json::to_value(&change).unwrap_or(Value::Null),
            artifact: None,
            reach: None,
            commit: None,
        },
        Err(e) => {
            let (code, message) = failed_of(&e);
            Outcome::Failed { code, message }
        }
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
            artifact,
            reach,
            commit,
        } => {
            let mut result = result;
            let (key, size, sha256) = match artifact {
                Some((k, s, h)) => (Some(k), Some(s), Some(h)),
                None => (None, None, None),
            };
            let remove = |key: &Option<String>| {
                let key = key.clone();
                async move {
                    if let Some(key) = key {
                        let _ = blobs.remove(&key).await;
                    }
                }
            };
            // The access asked again at the commit: a grant taken away while the job ran does not let it in.
            let access = match actor_access(db, claim).await {
                Ok(a) => a,
                Err(e) => {
                    remove(&key).await;
                    let (code, message) = failed_of(&e);
                    return Box::pin(finish(db, blobs, claim, Outcome::Failed { code, message }))
                        .await;
                }
            };
            let mut tx = db.scoped(access.scope()).await?;
            let again = match crate::access::recheck(&mut tx, &access).await {
                Ok(a) => a
                    .live()
                    .and_then(|()| a.require(ProjectPermission::JobsRun))
                    .and_then(|()| {
                        if claim.kind == PROJECT_EXPORT_JOB || key.is_some() {
                            a.require(ProjectPermission::Download)
                        } else {
                            Ok(())
                        }
                    })
                    .map(|()| a),
                Err(e) => Err(e),
            };
            let a = match again {
                Ok(a) => a,
                Err(e) => {
                    drop(tx);
                    remove(&key).await;
                    let (code, message) = failed_of(&e);
                    return Box::pin(finish(db, blobs, claim, Outcome::Failed { code, message }))
                        .await;
                }
            };
            // The reach asked again too: a result cut for the caller as they were is not theirs once the
            // forms' marks or their right to design them changed while the job ran.
            if let Some(stamp) = &reach {
                let now = crate::projects::reading_of(&mut tx, &a).await?;
                if now.reach.stamp() != *stamp {
                    drop(tx);
                    remove(&key).await;
                    return Box::pin(finish(
                        db,
                        blobs,
                        claim,
                        Outcome::Failed {
                            code: "reach_changed",
                            message: "Formların gizlediği alanlar ya da form tasarlama yetkiniz iş sürerken değişti; verilen kopya artık sizin görebileceğiniz kopya değil, bu yüzden verilmedi.".into(),
                        },
                    ))
                    .await;
                }
            }
            // A processing job's result is committed here, in the transaction that finishes the job: the project must
            // still be at the revision the tool read, the caller's rights are asked again under the project's lock,
            // and a finish that does not pass (a lost lease) takes the commit back with it (docs/adr/0559 §2).
            if let Some(staged) = &commit {
                match commit_staged(&mut tx, &a, claim, staged).await {
                    Ok(revision) => result["revision"] = Value::String(revision),
                    Err(e) => {
                        drop(tx);
                        remove(&key).await;
                        let (code, message) = failed_of(&e);
                        return Box::pin(finish(
                            db,
                            blobs,
                            claim,
                            Outcome::Failed { code, message },
                        ))
                        .await;
                    }
                }
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
            // A finish that did not pass takes everything of this transaction back, a commit included.
            if !passed {
                drop(tx);
                remove(&key).await;
                return Ok(ServerJobState::Running);
            }
            tx.commit().await?;
            Ok(ServerJobState::Succeeded)
        }
    }
}

/// The commit of a processing job's delta inside the finish transaction: the versions of the objects it changes or
/// removes and of the project's metadata as the transaction sees them, expected, and the project required to be at the
/// revision the tool read, so the delta is the tool's on exactly the project it read.
async fn commit_staged(
    tx: &mut sqlx::Transaction<'static, sqlx::Postgres>,
    access: &ProjectAccess,
    claim: &Claim,
    staged: &Staged,
) -> AppResult<String> {
    let mut expected: std::collections::BTreeMap<String, String> = Default::default();
    let ids: Vec<Uuid> = staged
        .delta
        .guarded()
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect();
    if !ids.is_empty() {
        let rows: Vec<(Uuid, i64)> = sqlx::query_as(
            "select id, version from kentos.feature where tenant_id = $1 and project_id = $2 and id = any($3)",
        )
        .bind(access.tenant)
        .bind(access.project)
        .bind(&ids)
        .fetch_all(&mut **tx)
        .await?;
        for (id, version) in rows {
            expected.insert(id.to_string(), version.to_string());
        }
    }
    if staged.delta.project.is_some() {
        let meta: i64 = sqlx::query_scalar(
            "select meta_version from kentos.project where tenant_id = $1 and id = $2",
        )
        .bind(access.tenant)
        .bind(access.project)
        .fetch_one(&mut **tx)
        .await?;
        expected.insert(PROJECT_META_KEY.to_owned(), meta.to_string());
    }
    let envelope = CommandEnvelope {
        command_name: PROJECT_CHANGES.into(),
        version: PROJECT_CHANGES_VERSION,
        tenant_id: claim.tenant.to_string(),
        project_id: claim.project.to_string(),
        request_id: format!("job-{}", claim.id),
        idempotency_key: format!("job-{}", claim.id),
        expected_versions: expected,
        input: serde_json::to_value(staged.delta.changes()).unwrap_or(Value::Null),
    };
    let done =
        crate::changes::commit_in(tx, access, envelope, Some(staged.revision)).await?;
    Ok(done.data_revision)
}

/// Runs jobs until `stop` is set: one after another while there are any, then waiting `idle` between looks; `said`
/// hears what each claim ended in (the caller logs it).
#[allow(clippy::too_many_arguments)]
pub async fn run_worker(
    db: kentos_postgres::Db,
    blobs: Blobs,
    owner: Uuid,
    lease: Duration,
    idle: Duration,
    limits: JobLimits,
    processor: Option<Arc<dyn Processor>>,
    stop: Arc<AtomicBool>,
    said: impl Fn(AppResult<Worked>),
) {
    while !stop.load(Ordering::SeqCst) {
        match work_once_with(&db, &blobs, owner, lease, &limits, processor.clone()).await {
            Ok(Some(w)) => said(Ok(w)),
            Ok(None) => tokio::time::sleep(idle).await,
            Err(e) => {
                said(Err(e));
                tokio::time::sleep(idle).await;
            }
        }
    }
}
