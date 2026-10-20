//! Files staged for server jobs (docs/adr/0561 §1): the big file of a `project.import` job reaches the server before the
//! job is asked for, in one streamed body, and the job names it.
//!
//! - **Asking** (`begin`): the file's name, size and SHA-256 as the user declares them; a row, an object key. The right
//!   is the job's: `jobs.run` and, since what is read is written to the project, `feature.write`.
//! - **Sending** (`start_receive`, `finish_receive`): the bytes stream to the object store; they are kept only when
//!   their size and SHA-256 are the declared ones, and a body cut short keeps nothing (the same file may be sent again).
//! - **Reading** (`staged`): a job's worker reads the row as the user who asked, under the project's row-level security:
//!   only the owner's own staged file of this project is there for it.
//! - **Retention:** the rows and the objects are removed with the finished jobs' (`job_upkeep::expire`), and the store's
//!   cleanup removes an object no row holds.

use kentos_contracts::{JobInput, JobInputBegin, ProjectPermission};
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::{BlobWriter, Blobs, WriteError};
use crate::error::{AppError, AppResult};
use crate::projects::rfc3339;

/// The longest file name a staged file has.
const NAME_MAX: usize = 200;

fn gone() -> AppError {
    AppError::not_found(
        "Hazırlanan dosya bulunamadı: süresi dolmuş ya da başkasına ait olabilir. Dosyayı yeniden yükleyin.",
    )
}

fn check_sha256(text: &str) -> AppResult<String> {
    if text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(text.to_string())
    } else {
        Err(AppError::invalid_at(
            "sha256",
            "sha256, dosyanın SHA-256 özeti olmalı: 64 küçük harfli onaltılık rakam.",
        ))
    }
}

/// A file name as a staged file's: one name, no folder, no control character.
pub fn check_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > NAME_MAX
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(AppError::invalid_at(
            "name",
            format!(
                "name 1–{NAME_MAX} karakterlik bir dosya adı olmalı (klasör yolu değil): parsel.gpkg."
            ),
        ));
    }
    Ok(name.to_owned())
}

/// A staged file's row, as a job's worker reads it.
#[derive(Clone, Debug)]
pub struct Staged {
    pub id: Uuid,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub blob_key: String,
    pub received: bool,
    pub created: OffsetDateTime,
}

impl Staged {
    fn view(&self) -> JobInput {
        JobInput {
            id: self.id.to_string(),
            name: self.name.clone(),
            size: self.size,
            sha256: self.sha256.clone(),
            created_at: rfc3339(self.created),
            received: self.received,
        }
    }
}

type Row = (
    Uuid,
    String,
    i64,
    String,
    String,
    OffsetDateTime,
    Option<OffsetDateTime>,
);

fn staged_of(row: Row) -> Staged {
    Staged {
        id: row.0,
        name: row.1,
        size: u64::try_from(row.2).unwrap_or(0),
        sha256: row.3,
        blob_key: row.4,
        created: row.5,
        received: row.6.is_some(),
    }
}

const SELECT: &str =
    "select id, name, size, sha256, blob_key, created_at, received_at from kentos.job_input
   where tenant_id = $1 and project_id = $2 and created_by = $3";

/// The caller's own staged file of the project, in a transaction already scoped to the project.
pub async fn staged_in(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    id: Uuid,
) -> AppResult<Staged> {
    let row: Option<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} and id = $4")))
        .bind(access.tenant)
        .bind(access.project)
        .bind(access.actor.user_id)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;
    row.map(staged_of).ok_or_else(gone)
}

/// The caller's own staged file of the project.
pub async fn staged(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    id: Uuid,
) -> AppResult<Staged> {
    let mut tx = db.scoped(access.scope()).await?;
    let found = staged_in(&mut tx, access, id).await?;
    tx.commit().await?;
    Ok(found)
}

/// Opens a staged file for a file of `size` bytes with this SHA-256 (at most `max` bytes).
pub async fn begin(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    asked: JobInputBegin,
    max: u64,
) -> AppResult<JobInput> {
    access.live()?;
    access.require(ProjectPermission::JobsRun)?;
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    let name = check_name(&asked.name)?;
    if asked.size == 0 || asked.size > max {
        return Err(AppError::invalid_at(
            "size",
            format!(
                "Dosya 1 bayt ile {} MiB arasında olmalı ({} bayt bildirildi). Sunucu yöneticisi sınırı KENTOS_JOB_INPUT_MAX_MB ile değiştirir.",
                max / (1024 * 1024),
                asked.size
            ),
        ));
    }
    let sha256 = check_sha256(&asked.sha256)?;
    let id = Uuid::now_v7();
    let key = Blobs::job_input_key(access.tenant, access.project, id, &sha256);
    let mut tx = db.scoped(access.scope()).await?;
    let (storage,): (String,) =
        sqlx::query_as("select storage from kentos.project where tenant_id = $1 and id = $2")
            .bind(access.tenant)
            .bind(access.project)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(crate::access::not_found)?;
    if storage == "file" {
        return Err(AppError::invalid(format!(
            "“{}” projesi dosya olarak saklanıyor; içe aktarma işi yalnız veritabanı projesinde çalışır. Dosyayı bilgisayarınızda açıp içe aktarın.",
            access.name
        )));
    }
    let created: OffsetDateTime = sqlx::query_scalar(
        "insert into kentos.job_input (tenant_id, project_id, id, created_by, name, size, sha256, blob_key)
         values ($1, $2, $3, $4, $5, $6, $7, $8) returning created_at",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(id)
    .bind(access.actor.user_id)
    .bind(&name)
    .bind(i64::try_from(asked.size).unwrap_or(i64::MAX))
    .bind(&sha256)
    .bind(&key)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(JobInput {
        id: id.to_string(),
        name,
        size: asked.size,
        sha256,
        created_at: rfc3339(created),
        received: false,
    })
}

/// One of the caller's own staged files as it stands: whether its bytes arrived (a client whose answer to the bytes was
/// lost asks this before sending them again).
pub async fn get(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    id: Uuid,
) -> AppResult<JobInput> {
    access.require(ProjectPermission::JobsRun)?;
    Ok(staged(db, access, id).await?.view())
}

/// A staged file whose bytes are arriving.
pub struct Receiving {
    staged: Staged,
    /// Where the bytes go; the caller feeds it the request's body.
    pub writer: BlobWriter,
}

/// Starts taking a staged file's bytes: the caller's own, not yet received.
pub async fn start_receive(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    id: Uuid,
) -> AppResult<Receiving> {
    access.live()?;
    access.require(ProjectPermission::JobsRun)?;
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    let found = staged(db, access, id).await?;
    if found.received {
        return Err(AppError::invalid(
            "Bu dosyanın baytları zaten alındı; içe aktarma işini isteyin ya da yeni bir dosya hazırlayın.",
        ));
    }
    let writer = blobs.create(&found.blob_key, found.size).await?;
    Ok(Receiving {
        staged: found,
        writer,
    })
}

/// A write of the body failed: the bytes so far are removed, and the error says why.
pub async fn failed_write(blobs: &Blobs, receiving: Receiving, error: WriteError) -> AppError {
    let _ = blobs.remove(&receiving.staged.blob_key).await;
    match error {
        WriteError::TooLarge { declared } => AppError::invalid_at(
            "size",
            format!(
                "Bildirilenden ({declared} bayt) fazla bayt geldi; hiçbir şey saklanmadı. Doğru boyutla yeni bir dosya hazırlayın."
            ),
        ),
        WriteError::Interrupted(why) => AppError::invalid(format!(
            "Dosyanın gönderimi yarıda kesildi ({why}); hiçbir şey saklanmadı. Aynı dosyayı yeniden gönderin."
        )),
        WriteError::Io(e) => AppError::Storage(e),
    }
}

/// The body ended: its size and hash must be the declared ones; otherwise nothing is kept.
pub async fn finish_receive(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    receiving: Receiving,
) -> AppResult<JobInput> {
    let Receiving { staged, writer } = receiving;
    let (got_size, got_sha) = writer.finish().await?;
    if got_size != staged.size || got_sha != staged.sha256 {
        let _ = blobs.remove(&staged.blob_key).await;
        return Err(AppError::invalid_at(
            if got_size != staged.size {
                "size"
            } else {
                "sha256"
            },
            format!(
                "Gelen dosya bildirilenle aynı değil ({got_size} bayt, SHA-256 {got_sha}; beklenen {} bayt, {}). Hiçbir şey saklanmadı; dosyanın özetini yeniden hesaplayıp yeni bir dosya hazırlayın.",
                staged.size, staged.sha256
            ),
        ));
    }
    let mut tx = db.scoped(access.scope()).await?;
    let marked = sqlx::query(
        "update kentos.job_input set received_at = now()
          where tenant_id = $1 and project_id = $2 and id = $3 and created_by = $4 and received_at is null",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(staged.id)
    .bind(access.actor.user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    if marked.rows_affected() == 0 {
        let _ = blobs.remove(&staged.blob_key).await;
        return Err(gone());
    }
    Ok(Staged {
        received: true,
        ..staged
    }
    .view())
}
