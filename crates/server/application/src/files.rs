//! Projects kept as files (docs/adr/0031; TODOS.md SYNC-02..06, SYNC-16):
//! uploads, committed revisions, downloads and the store's cleanup.
//!
//! A revision is saved in steps, each checked on its own:
//!
//! 1. [`begin`] opens an upload for the size and SHA-256 the client
//!    computed (`feature.write`, the project not archived or in the trash).
//! 2. [`start_receive`] / [`finish_receive`] take the bytes to a temporary
//!    object, hashed as they arrive and cut off past the declared size; they
//!    are kept only if size and hash match and they read as a KCAD v2 file
//!    (the shared codec, off the async threads).
//! 3. [`commit`] (`project.file.commit`) locks the project, asks the access
//!    again, compares `expectedVersions["@file"]` with the newest revision
//!    and only then moves the object to its final key and adds the
//!    revision: a stale save is a conflict, never an overwrite. A commit
//!    cut short between the move and the database leaves the object at its
//!    final key: committing the same upload again finds it there, and the
//!    project's next commit removes it otherwise.
//!
//! A new database project takes an upload too: `project.import` brings the
//! file's drawing into it in one transaction (docs/adr/0036).
//!
//! Only the person who opened an upload sends its bytes and commits it; for
//! anyone else it does not exist. An upload not committed within a day is
//! removed with its bytes ([`cleanup`]).

use std::sync::OnceLock;

use kentos_contracts::{
    CommandEnvelope, ConflictReason, FILE_UPLOAD_MAX, FeatureConflict, FileCommit, FileCommitted,
    FileRevision, FileRevisions, FileUpload, FileUploadBegin, PROJECT_FILE_COMMIT,
    PROJECT_FILE_COMMIT_VERSION, PROJECT_FILE_COMMITTED, PROJECT_FILE_KEY, ProjectPermission,
    ProjectStorage, UPLOAD_LIFETIME_HOURS,
};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::{BlobWriter, Blobs, WriteError};
use crate::changes::{check_target, lock};
use crate::commands::input;
use crate::error::{AppError, AppResult};
use crate::projects::{rfc3339, storage_of};
use crate::{idempotency, journal};

/// Files verified (or, for a snapshot, written) at once: each is held whole
/// with its decoded drawing, so a few large ones cannot take all the
/// server's memory together.
pub(crate) fn verifying() -> &'static Semaphore {
    static VERIFYING: OnceLock<Semaphore> = OnceLock::new();
    VERIFYING.get_or_init(|| Semaphore::new(2))
}

/// The project's storage mode, newest file revision and data revision.
async fn file_state(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
) -> AppResult<(ProjectStorage, i64, i64)> {
    let (storage, file_revision, data_revision): (String, Option<i64>, i64) = sqlx::query_as(
        "select storage, file_revision, data_revision from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_one(&mut **tx)
    .await?;
    Ok((
        storage_of(&storage),
        file_revision.unwrap_or(0),
        data_revision,
    ))
}

fn needs_file(storage: ProjectStorage, name: &str) -> AppResult<()> {
    match storage {
        ProjectStorage::File => Ok(()),
        ProjectStorage::Database => Err(AppError::invalid(format!(
            "“{name}” projesi nesne nesne veritabanında saklanıyor; dosya yüklenmez. Değişiklikler project.changes ile kaydedilir."
        ))),
    }
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

fn expires(created: OffsetDateTime) -> String {
    rfc3339(created + time::Duration::hours(i64::from(UPLOAD_LIFETIME_HOURS)))
}

/// Opens an upload for a file of `size` bytes with this SHA-256.
pub async fn begin(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    begin: FileUploadBegin,
) -> AppResult<FileUpload> {
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    if begin.size == 0 || begin.size > FILE_UPLOAD_MAX {
        return Err(AppError::invalid_at(
            "size",
            format!(
                "Dosya 1 bayt ile {} MiB arasında olmalı ({} bayt bildirildi). Daha büyük projeler için parçalı yükleme henüz yok.",
                FILE_UPLOAD_MAX / (1024 * 1024),
                begin.size
            ),
        ));
    }
    let sha256 = check_sha256(&begin.sha256)?;
    // Either kind of project takes uploads: a file project commits them as
    // revisions, a new database project imports one (`project.import`, docs/adr/0036).
    let mut tx = db.scoped(access.scope()).await?;
    let id = Uuid::now_v7();
    let created: OffsetDateTime = sqlx::query_scalar(
        "insert into kentos.project_upload (tenant_id, project_id, id, created_by, size, sha256, blob_key)
         values ($1, $2, $3, $4, $5, $6, $7) returning created_at",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(id)
    .bind(access.actor.user_id)
    .bind(i64::from(begin.size))
    .bind(&sha256)
    .bind(Blobs::upload_key(access.tenant, access.project, id))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(FileUpload {
        id: id.to_string(),
        size: begin.size,
        sha256,
        created_at: rfc3339(created),
        expires_at: expires(created),
        received: false,
        objects: None,
    })
}

/// One of the caller's own uploads of this project.
pub(crate) struct Upload {
    pub size: i64,
    pub sha256: String,
    pub received: bool,
    pub blob_key: String,
    created: OffsetDateTime,
    /// Counted when the bytes were verified.
    pub objects: Option<i64>,
}

/// An upload's row: size, SHA-256, when it was received, its object's key, when it was opened, its object count.
type UploadRow = (
    i64,
    String,
    Option<OffsetDateTime>,
    String,
    OffsetDateTime,
    Option<i64>,
);

pub(crate) async fn own_upload(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    upload: Uuid,
) -> AppResult<Option<Upload>> {
    let row: Option<UploadRow> = sqlx::query_as(
        "select size, sha256, received_at, blob_key, created_at, objects from kentos.project_upload
              where tenant_id = $1 and project_id = $2 and id = $3 and created_by = $4",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(upload)
    .bind(access.actor.user_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.map(
        |(size, sha256, received_at, blob_key, created, objects)| Upload {
            size,
            sha256,
            received: received_at.is_some(),
            blob_key,
            created,
            objects,
        },
    ))
}

/// One of the caller's own uploads as it stands: whether its bytes arrived.
/// A client whose answer to the bytes was lost asks this before sending
/// them again, since a second send of received bytes is refused
/// (docs/adr/0040). Asks the access again like every step of an upload.
pub async fn upload(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    upload: Uuid,
) -> AppResult<FileUpload> {
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    let mut tx = db.scoped(access.scope()).await?;
    let found = own_upload(&mut tx, access, upload)
        .await?
        .ok_or_else(upload_gone)?;
    tx.commit().await?;
    Ok(FileUpload {
        id: upload.to_string(),
        size: u32::try_from(found.size).unwrap_or(u32::MAX),
        sha256: found.sha256,
        created_at: rfc3339(found.created),
        expires_at: expires(found.created),
        received: found.received,
        objects: found.objects.map(|n| n.to_string()),
    })
}

pub(crate) fn upload_gone() -> AppError {
    AppError::not_found(
        "Yükleme bulunamadı: süresi dolmuş, kaydedilmiş ya da başkasına ait olabilir. Yeni bir yükleme başlatın.",
    )
}

/// An upload whose bytes are arriving.
pub struct Receiving {
    upload: Uuid,
    size: u64,
    sha256: String,
    key: String,
    created: OffsetDateTime,
    /// Where the bytes go; the caller feeds it the request's body.
    pub writer: BlobWriter,
}

/// Starts taking an upload's bytes: the caller's own upload, not yet received.
pub async fn start_receive(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    upload: Uuid,
) -> AppResult<Receiving> {
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    let mut tx = db.scoped(access.scope()).await?;
    let found = own_upload(&mut tx, access, upload)
        .await?
        .ok_or_else(upload_gone)?;
    tx.commit().await?;
    if found.received {
        return Err(AppError::invalid(
            "Bu yüklemenin baytları zaten alındı; project.file.commit ile kaydedin ya da yeni bir yükleme başlatın.",
        ));
    }
    let size = u64::try_from(found.size).unwrap_or(0);
    let writer = blobs.create(&found.blob_key, size).await?;
    Ok(Receiving {
        upload,
        size,
        sha256: found.sha256,
        key: found.blob_key,
        created: found.created,
        writer,
    })
}

/// A write of the body failed: the bytes so far are removed, and the error says why.
pub async fn failed_write(blobs: &Blobs, receiving: Receiving, error: WriteError) -> AppError {
    let _ = blobs.remove(&receiving.key).await;
    match error {
        WriteError::TooLarge { declared } => AppError::invalid_at(
            "size",
            format!(
                "Bildirilenden ({declared} bayt) fazla bayt geldi; hiçbir şey saklanmadı. Doğru boyutla yeni bir yükleme başlatın."
            ),
        ),
        WriteError::Interrupted(why) => AppError::invalid(format!(
            "Dosyanın gönderimi yarıda kesildi ({why}); hiçbir şey saklanmadı. Aynı yüklemeyi yeniden gönderin."
        )),
        WriteError::Io(e) => AppError::Storage(e),
    }
}

/// The body ended: its size and hash must be the declared ones and it must
/// read as a KCAD v2 file; otherwise nothing is kept.
pub async fn finish_receive(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    receiving: Receiving,
) -> AppResult<FileUpload> {
    let Receiving {
        upload,
        size,
        sha256,
        key,
        created,
        writer,
    } = receiving;
    let (got_size, got_sha) = writer.finish().await?;
    if got_size != size || got_sha != sha256 {
        let _ = blobs.remove(&key).await;
        return Err(AppError::invalid_at(
            if got_size != size { "size" } else { "sha256" },
            format!(
                "Gelen dosya bildirilenle aynı değil ({got_size} bayt, SHA-256 {got_sha}; beklenen {size} bayt, {sha256}). Hiçbir şey saklanmadı; dosyanın özetini yeniden hesaplayıp yeni bir yükleme başlatın."
            ),
        ));
    }
    let objects = match verify(blobs, &key).await {
        Ok(n) => n,
        Err(why) => {
            let _ = blobs.remove(&key).await;
            return Err(why);
        }
    };
    let mut tx = db.scoped(access.scope()).await?;
    let marked = sqlx::query(
        "update kentos.project_upload set received_at = now(), objects = $5
          where tenant_id = $1 and project_id = $2 and id = $3 and created_by = $4 and received_at is null",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(upload)
    .bind(access.actor.user_id)
    .bind(objects)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    if marked.rows_affected() == 0 {
        // Expired or committed meanwhile: the bytes belong to nothing.
        let _ = blobs.remove(&key).await;
        return Err(upload_gone());
    }
    Ok(FileUpload {
        id: upload.to_string(),
        size: u32::try_from(size).unwrap_or(u32::MAX),
        sha256,
        created_at: rfc3339(created),
        expires_at: expires(created),
        received: true,
        objects: Some(objects.to_string()),
    })
}

/// Reads the object back and decodes it with the shared KCAD v2 codec, off
/// the async threads; returns how many objects it holds.
async fn verify(blobs: &Blobs, key: &str) -> AppResult<i64> {
    let _turn = verifying()
        .acquire()
        .await
        .map_err(|e| AppError::Storage(std::io::Error::other(e)))?;
    let bytes = blobs.read(key).await.map_err(missing_is_gone)?;
    let read = tokio::task::spawn_blocking(move || {
        kentos_kcad::read(&bytes)
            .map(|(_, doc)| i64::try_from(doc.entities.len()).unwrap_or(i64::MAX))
    })
    .await
    .map_err(|e| AppError::Storage(std::io::Error::other(e)))?;
    read.map_err(|e| {
        AppError::invalid(format!(
            "Yüklenen dosya geçerli bir KCAD v2 dosyası değil: {e}. Çizimi KentOS ile yeniden kaydedip yükleyin."
        ))
    })
}

/// `project.file.commit` v1: the verified upload becomes the next revision.
pub async fn commit(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<FileCommitted> {
    let FileCommit { upload_id } =
        input(&envelope, PROJECT_FILE_COMMIT, PROJECT_FILE_COMMIT_VERSION)?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let upload = Uuid::parse_str(&upload_id).map_err(|_| {
        AppError::invalid_at(
            "uploadId",
            format!("uploadId bir yükleme kimliği (UUID) olmalı: {upload_id}"),
        )
    })?;
    let expected_text = envelope
        .expected_versions
        .get(PROJECT_FILE_KEY)
        .ok_or_else(|| {
            AppError::invalid_at(
                "expectedVersions[@file]",
                "project.file.commit, dosyanın dayandığı revizyonu ister: expectedVersions[\"@file\"] (ilk kayıtta \"0\").",
            )
        })?;
    let expected: i64 = expected_text.parse().map_err(|_| {
        AppError::invalid_at(
            "expectedVersions[@file]",
            format!("expectedVersions[\"@file\"] bir tamsayı değil: {expected_text}"),
        )
    })?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    // Commits of one project happen one after another, with the access asked again under the lock.
    let now = lock(&mut tx, access).await?;
    if let Some(mut earlier) =
        idempotency::earlier::<FileCommitted>(&mut tx, &now, &envelope, &text).await?
    {
        tx.commit().await?;
        earlier.replayed = true;
        return Ok(earlier);
    }
    now.writable()?;
    now.require(ProjectPermission::FeatureWrite)?;
    let (storage, current, data_revision) = file_state(&mut tx, &now).await?;
    needs_file(storage, &now.name)?;
    if expected != current {
        return Err(AppError::Conflict {
            message: format!(
                "Dosya siz kaydederken başka biri tarafından kaydedildi (şimdiki revizyon {current}, sizinki {expected}'e dayanıyor); hiçbir şey yazılmadı. Son revizyonu açıp değişikliklerinizi yeniden uygulayın ya da dosyanızı ayrı bir kopya olarak saklayın."
            ),
            conflicts: vec![FeatureConflict {
                id: PROJECT_FILE_KEY.into(),
                reason: ConflictReason::Project,
                expected: Some(expected.to_string()),
                actual: Some(current.to_string()),
                current: None,
            }],
            revision: Some(data_revision),
        });
    }
    let found = own_upload(&mut tx, &now, upload)
        .await?
        .ok_or_else(upload_gone)?;
    if !found.received {
        return Err(AppError::invalid_at(
            "uploadId",
            "Yüklemenin baytları henüz gelmedi; önce PUT …/uploads/{yükleme} ile gönderin.",
        ));
    }
    let revision = current + 1;
    let final_key = Blobs::revision_key(now.tenant, now.project, revision, &found.sha256);
    // Above the newest revision are only objects of commits that did not finish.
    blobs
        .remove_revisions_after(now.tenant, now.project, current, &final_key)
        .await?;
    blobs
        .promote(&found.blob_key, &final_key)
        .await
        .map_err(missing_is_gone)?;
    match record(
        &mut tx, &now, &envelope, &text, upload, revision, &found, &final_key,
    )
    .await
    {
        Ok(result) => {
            // A commit that fails may still have been written (its answer lost), so the
            // object stays at its final key either way. If nothing was written, the upload
            // is committed again from there, or the project's next commit removes it.
            tx.commit().await?;
            Ok(result)
        }
        Err(e) => {
            // Nothing was written: the object goes back, so the upload can be committed again.
            let _ = blobs.promote(&final_key, &found.blob_key).await;
            Err(e)
        }
    }
}

/// An upload's object that is not in the store: the upload expired meanwhile.
fn missing_is_gone(e: std::io::Error) -> AppError {
    match e.kind() {
        std::io::ErrorKind::NotFound => upload_gone(),
        _ => AppError::Storage(e),
    }
}

/// The rows of a committed revision, its audit, its event and the stored answer.
#[allow(clippy::too_many_arguments)]
async fn record(
    tx: &mut Transaction<'static, Postgres>,
    now: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
    upload: Uuid,
    revision: i64,
    found: &Upload,
    final_key: &str,
) -> AppResult<FileCommitted> {
    sqlx::query(
        "insert into kentos.project_file_revision (tenant_id, project_id, revision, size, sha256, blob_key, created_by, request_id, objects)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(revision)
    .bind(found.size)
    .bind(&found.sha256)
    .bind(final_key)
    .bind(now.actor.user_id)
    .bind(&envelope.request_id)
    .bind(found.objects)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "delete from kentos.project_upload where tenant_id = $1 and project_id = $2 and id = $3",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(upload)
    .execute(&mut **tx)
    .await?;
    let data_revision: i64 = sqlx::query_scalar(
        "update kentos.project set file_revision = $3, data_revision = data_revision + 1, updated_at = now()
          where tenant_id = $1 and id = $2 returning data_revision",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(revision)
    .fetch_one(&mut **tx)
    .await?;
    let request = Some(envelope.request_id.as_str());
    journal::audit(
        tx,
        now,
        PROJECT_FILE_COMMIT,
        request,
        data_revision,
        json!({ "revision": revision, "size": found.size, "sha256": found.sha256, "objects": found.objects }),
    )
    .await?;
    journal::event(
        tx,
        now,
        PROJECT_FILE_COMMITTED,
        request,
        data_revision,
        false,
    )
    .await?;
    let result = FileCommitted {
        revision: revision.to_string(),
        size: u32::try_from(found.size).unwrap_or(u32::MAX),
        sha256: found.sha256.clone(),
        objects: found.objects.map(|n| n.to_string()),
        replayed: false,
    };
    idempotency::record(tx, now, envelope, text, &result).await?;
    Ok(result)
}

/// A revision's row: number, size, SHA-256, author and their name, time,
/// object count (null before migration 0007) and the object's key.
pub(crate) type RevisionRow = (
    i64,
    i64,
    String,
    Uuid,
    String,
    OffsetDateTime,
    Option<i64>,
    String,
);

fn revision_of((revision, size, sha256, by, name, at, objects, _key): RevisionRow) -> FileRevision {
    FileRevision {
        revision: revision.to_string(),
        size: u32::try_from(size).unwrap_or(u32::MAX),
        sha256,
        created_by: by.to_string(),
        created_by_name: name,
        created_at: rfc3339(at),
        objects: objects.map(|n| n.to_string()),
    }
}

const REVISION_SELECT: &str = "select r.revision, r.size, r.sha256, r.created_by, coalesce(u.display_name, ''), r.created_at, r.objects, r.blob_key
   from kentos.project_file_revision r left join kentos.app_user u on u.id = r.created_by
  where r.tenant_id = $1 and r.project_id = $2";

/// A file project's revisions, newest first (`project.read`).
pub async fn list(db: &kentos_postgres::Db, access: &ProjectAccess) -> AppResult<FileRevisions> {
    access.live()?;
    access.require(ProjectPermission::Read)?;
    let mut tx = db.scoped(access.scope()).await?;
    let (storage, current, _) = file_state(&mut tx, access).await?;
    needs_file(storage, &access.name)?;
    let rows: Vec<RevisionRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{REVISION_SELECT} order by r.revision desc"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(FileRevisions {
        current: (current > 0).then(|| current.to_string()),
        revisions: rows.into_iter().map(revision_of).collect(),
    })
}

/// The newest revision of the project in `tx`'s scope, if it has one (a copy starts from it).
pub(crate) async fn newest(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
    project: Uuid,
) -> AppResult<Option<RevisionRow>> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{REVISION_SELECT} and r.revision = (select p.file_revision from kentos.project p where p.tenant_id = $1 and p.id = $2)"
    )))
    .bind(tenant)
    .bind(project)
    .fetch_optional(&mut **tx)
    .await?)
}

/// One revision of the project in `tx`'s scope, if it exists (a restore starts from it).
pub(crate) async fn revision(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
    project: Uuid,
    revision: i64,
) -> AppResult<Option<RevisionRow>> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{REVISION_SELECT} and r.revision = $3"
    )))
    .bind(tenant)
    .bind(project)
    .bind(revision)
    .fetch_optional(&mut **tx)
    .await?)
}

/// One committed revision and its bytes (`project.download`).
pub async fn download(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    revision: i64,
) -> AppResult<(FileRevision, tokio::fs::File)> {
    access.live()?;
    access.require(ProjectPermission::Download)?;
    let mut tx = db.scoped(access.scope()).await?;
    let (storage, _, _) = file_state(&mut tx, access).await?;
    needs_file(storage, &access.name)?;
    let row: Option<RevisionRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{REVISION_SELECT} and r.revision = $3"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .bind(revision)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let row = row.ok_or_else(|| {
        AppError::not_found(format!(
            "“{}” projesinin {revision}. revizyonu yok.",
            access.name
        ))
    })?;
    let file = blobs.open(&row.7).await?;
    Ok((revision_of(row), file))
}

/// What one round of [`cleanup`] removed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cleanup {
    /// Uploads nobody committed within their lifetime.
    pub expired: usize,
    /// Upload files older than twice that, whatever the database says.
    pub swept: usize,
    /// Projects removed for good whose revisions were still in the store.
    pub purged: usize,
    /// Checkpoint objects of no checkpoint: its row was never committed, or
    /// was removed and the object was not (docs/adr/0034).
    pub checkpoints: usize,
}

/// How long a project's folder in the store stays unchanged before the
/// cleanup may take it for a folder of a project removed for good: a copy's
/// object is shared a moment before the copy's row is committed.
pub const SETTLED: std::time::Duration = std::time::Duration::from_secs(3600);

/// The store's cleanup, run by the server every hour: expired uploads (rows
/// and bytes), stray upload files, the objects of projects removed for good
/// and the checkpoint objects of no checkpoint, once their folder or file has
/// been still for `settled` ([`SETTLED`]).
pub async fn cleanup(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    settled: std::time::Duration,
) -> AppResult<Cleanup> {
    const BATCH: i32 = 200;
    let lifetime = std::time::Duration::from_secs(u64::from(UPLOAD_LIFETIME_HOURS) * 3600);
    let mut done = Cleanup::default();
    loop {
        let keys: Vec<String> = sqlx::query_scalar(
            "select blob_key from kentos.expire_uploads(now() - make_interval(hours => $1), $2)",
        )
        .bind(i32::try_from(UPLOAD_LIFETIME_HOURS).unwrap_or(24))
        .bind(BATCH)
        .fetch_all(&db.pool)
        .await?;
        for key in &keys {
            blobs.remove(key).await?;
        }
        done.expired += keys.len();
        if keys.len() < BATCH as usize {
            break;
        }
    }
    done.swept = blobs.sweep_uploads(lifetime * 2).await?;
    let stored = blobs.projects(settled).await?;
    if !stored.is_empty() {
        let ids: Vec<Uuid> = stored.iter().map(|(_, p)| *p).collect();
        let existing: Vec<Uuid> = sqlx::query_scalar("select id from kentos.existing_projects($1)")
            .bind(&ids)
            .fetch_all(&db.pool)
            .await?;
        for (tenant, project) in stored {
            if !existing.contains(&project) {
                blobs.remove_project(tenant, project).await?;
                done.purged += 1;
            }
        }
    }
    let objects = blobs.checkpoint_objects(settled).await?;
    if !objects.is_empty() {
        let ids: Vec<Uuid> = objects.iter().map(|(id, _)| *id).collect();
        let existing: Vec<Uuid> =
            sqlx::query_scalar("select id from kentos.existing_checkpoints($1)")
                .bind(&ids)
                .fetch_all(&db.pool)
                .await?;
        for (id, key) in objects {
            if !existing.contains(&id) {
                blobs.remove(&key).await?;
                done.checkpoints += 1;
            }
        }
    }
    Ok(done)
}
