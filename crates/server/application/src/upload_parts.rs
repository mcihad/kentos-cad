//! An upload sent in parts (docs/adr/0045; TODOS.md SYNC-10): a file goes to
//! the server a part at a time, so a connection cut short costs only the
//! part on its way — the next part goes on from what arrived.
//!
//! - `PUT …/uploads/{upload}?offset=N` carries one part, at most
//!   [`PART_MAX`] bytes, taken whole before anything is written.
//! - Under the upload's row lock its offset must be the bytes that arrived so
//!   far; then it is added at the end of the upload's object and flushed. Two
//!   parts never interleave, and a part cut short never reaches the object.
//! - The last part completes the file: its size and SHA-256 must be the
//!   declared ones (hashed from the disk) and it must read as a KCAD v2 file,
//!   as a file sent whole must (files.rs); otherwise nothing is kept and a new
//!   upload is needed.
//! - How far an upload came is its object's length (`GET …/uploads/{upload}`):
//!   nothing new in the database. The day-old cleanup removes a part-sent
//!   upload nobody finished, as it removes any other.

use kentos_contracts::{FileUpload, ProjectPermission};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};
use crate::files::{expires, upload_gone, verified};
use crate::projects::rfc3339;

/// The largest part of one request.
pub const PART_MAX: usize = 32 * 1024 * 1024;

type Row = (i64, String, bool, String, OffsetDateTime);

/// Takes one part of the caller's own upload at `offset` (see the module comment).
pub async fn receive_part(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    upload: Uuid,
    offset: u64,
    bytes: &[u8],
) -> AppResult<FileUpload> {
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    if bytes.is_empty() || bytes.len() > PART_MAX {
        return Err(AppError::invalid_at(
            "size",
            format!(
                "Bir parça 1 bayt ile {} MiB arasında olmalı ({} bayt geldi).",
                PART_MAX / (1024 * 1024),
                bytes.len()
            ),
        ));
    }
    let mut tx = db.scoped(access.scope()).await?;
    // The upload's row, locked: the parts of one upload go one at a time.
    let row: Option<Row> = sqlx::query_as(
        "select size, sha256, received_at is not null, blob_key, created_at from kentos.project_upload
          where tenant_id = $1 and project_id = $2 and id = $3 and created_by = $4 for update",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(upload)
    .bind(access.actor.user_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (size, sha256, received, key, created) = row.ok_or_else(upload_gone)?;
    if received {
        return Err(AppError::invalid(
            "Bu yüklemenin baytları zaten alındı; project.file.commit ile kaydedin ya da yeni bir yükleme başlatın.",
        ));
    }
    let size = u64::try_from(size).unwrap_or(0);
    let arrived = blobs.len(&key).await?;
    if offset != arrived {
        return Err(AppError::invalid_at(
            "offset",
            format!(
                "Yüklemenin {arrived} baytı geldi; sonraki parça {arrived}. bayttan başlamalı ({offset} değil)."
            ),
        ));
    }
    if arrived + bytes.len() as u64 > size {
        return Err(AppError::invalid_at(
            "size",
            format!(
                "Bu parçayla bildirilenden ({size} bayt) fazla bayt gelirdi; parça alınmadı. Doğru boyutla yeni bir yükleme başlatın."
            ),
        ));
    }
    let now = blobs.append(&key, bytes).await?;
    tx.commit().await?;
    let answer = |received: bool, arrived: u64, objects: Option<i64>| FileUpload {
        id: upload.to_string(),
        size: u32::try_from(size).unwrap_or(u32::MAX),
        sha256: sha256.clone(),
        created_at: rfc3339(created),
        expires_at: expires(created),
        received,
        received_bytes: Some(arrived.to_string()),
        objects: objects.map(|n| n.to_string()),
    };
    if now < size {
        return Ok(answer(false, now, None));
    }
    // The last part: the whole file is checked, as a file sent whole is.
    let got = blobs.sha256(&key).await?;
    if got != sha256 {
        let _ = blobs.remove(&key).await;
        return Err(AppError::invalid_at(
            "sha256",
            format!(
                "Gelen dosya bildirilenle aynı değil (SHA-256 {got}, beklenen {sha256}). Hiçbir şey saklanmadı; dosyanın özetini yeniden hesaplayıp yeni bir yükleme başlatın."
            ),
        ));
    }
    let objects = verified(db, blobs, access, upload, &key).await?;
    Ok(answer(true, size, Some(objects)))
}
