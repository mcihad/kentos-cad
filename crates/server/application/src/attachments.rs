//! A project's attachments (docs/adr/9000 §15; TODOS.md CLOUD-06): the reports, sheets, tables, photos and source
//! files kept beside its drawing, in the cloud for the desktop, a phone and an agent alike.
//!
//! - An attachment comes from a received upload (uploads.rs), so it is scanned before anyone of the project sees it,
//!   and kept with its scan's verdict and the media type its bytes were found to be.
//! - Who reads the project lists them (`project.read`); who may download its files downloads them (`project.download`);
//!   who changes its objects adds and removes them (`feature.write`), not in an archived or deleted project.
//! - Its bytes are written once under a key of its own; the project's removal for good takes them along.

use kentos_contracts::{AttachmentCreate, AttachmentView, ProjectPermission, ScanState};
use kentos_postgres::Db;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};
use crate::projects::rfc3339;

/// Shown as they are by a browser or a phone: documents and pictures that run nothing.
pub const VIEWABLE: &[&str] = &[
    "application/pdf",
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/bmp",
    "text/plain",
    "text/csv",
    "video/mp4",
    "video/quicktime",
];

type Row = (
    Uuid,
    String,
    String,
    i64,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<i64>,
    String,
    OffsetDateTime,
);

const SELECT: &str =
    "select id, name, media_type, size, sha256, blob_key, scan, scan_note, note, revision,
        created_by_name, created_at
   from kentos.project_attachment where tenant_id = $1 and project_id = $2";

fn view_of(row: &Row) -> AttachmentView {
    let (id, name, media_type, size, sha256, _, scan, scan_note, note, revision, by, at) = row;
    AttachmentView {
        id: id.to_string(),
        name: name.clone(),
        media_type: media_type.clone(),
        size: u64::try_from(*size).unwrap_or(0),
        sha256: sha256.clone(),
        scan: ScanState::of(scan),
        scan_note: scan_note.clone(),
        note: note.clone(),
        revision: *revision,
        created_by_name: by.clone(),
        created_at: rfc3339(*at),
        viewable: VIEWABLE.contains(&media_type.as_str()),
    }
}

pub fn key_of(tenant: Uuid, project: Uuid, id: Uuid, sha256: &str) -> String {
    format!("attachments/{tenant}/{project}/{id}-{sha256}")
}

fn missing(access: &ProjectAccess) -> AppError {
    AppError::not_found(format!("“{}” projesinde böyle bir ek yok.", access.name))
}

fn id_of(access: &ProjectAccess, id: &str) -> AppResult<Uuid> {
    Uuid::parse_str(id.trim()).map_err(|_| missing(access))
}

/// `GET …/attachments`: newest first.
pub async fn list(db: &Db, access: &ProjectAccess) -> AppResult<Vec<AttachmentView>> {
    access.live()?;
    access.require(ProjectPermission::Read)?;
    let mut tx = db.scoped(access.scope()).await?;
    let rows: Vec<Row> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} order by created_at desc, id desc"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows.iter().map(view_of).collect())
}

/// `POST …/attachments`: a received upload becomes the project's attachment.
pub async fn add(
    db: &Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    input: AttachmentCreate,
) -> AppResult<AttachmentView> {
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    let note = input
        .note
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty());
    if note.is_some_and(|n| n.chars().count() > 2000) {
        return Err(AppError::invalid_at(
            "note",
            "Not en çok 2000 karakter olabilir.",
        ));
    }
    let mut tx = db.scoped(access.scope()).await?;
    if let Some(r) = input.revision
        && crate::files::revision(&mut tx, access.tenant, access.project, r)
            .await?
            .is_none()
    {
        return Err(AppError::invalid_at(
            "revision",
            format!("“{}” projesinde {r}. revizyon yok.", access.name),
        ));
    }
    // The upload is the caller's own (the project's scope carries the caller): taken in the same transaction.
    let taken = crate::uploads::take(&mut tx, &access.actor, &input.upload_id).await?;
    let id = Uuid::now_v7();
    let key = key_of(access.tenant, access.project, id, &taken.sha256);
    let row: Row = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with added as (
           insert into kentos.project_attachment (tenant_id, project_id, id, name, media_type, size, sha256, blob_key,
                                                  scan, scan_note, note, revision, created_by, created_by_name)
           values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) returning *)
         {}",
        SELECT.replace("from kentos.project_attachment", "from added")
    )))
    .bind(access.tenant)
    .bind(access.project)
    .bind(id)
    .bind(&taken.name)
    .bind(&taken.scanned.media_type)
    .bind(i64::try_from(taken.size).unwrap_or(i64::MAX))
    .bind(&taken.sha256)
    .bind(&key)
    .bind(taken.scanned.state.code())
    .bind(taken.scanned.note.as_deref())
    .bind(note)
    .bind(input.revision)
    .bind(access.actor.user_id)
    .bind(&access.actor.display_name)
    .fetch_one(&mut *tx)
    .await?;
    // The object moves before the rows are committed: a failure leaves the upload as it was.
    blobs.promote(&taken.key, &key).await?;
    if let Err(e) = tx.commit().await {
        let _ = blobs.promote(&key, &taken.key).await;
        return Err(e.into());
    }
    Ok(view_of(&row))
}

/// One attachment and its bytes (`project.download`).
pub async fn open(
    db: &Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    id: &str,
) -> AppResult<(AttachmentView, tokio::fs::File)> {
    access.live()?;
    access.require(ProjectPermission::Download)?;
    let id = id_of(access, id)?;
    let mut tx = db.scoped(access.scope()).await?;
    let row: Row = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} and id = $3")))
        .bind(access.tenant)
        .bind(access.project)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| missing(access))?;
    tx.commit().await?;
    let file = blobs.open(&row.5).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            missing(access)
        } else {
            AppError::Storage(e)
        }
    })?;
    Ok((view_of(&row), file))
}

/// `DELETE …/attachments/{id}`.
pub async fn remove(db: &Db, blobs: &Blobs, access: &ProjectAccess, id: &str) -> AppResult<()> {
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    let id = id_of(access, id)?;
    let mut tx = db.scoped(access.scope()).await?;
    let key: Option<String> = sqlx::query_scalar(
        "delete from kentos.project_attachment where tenant_id = $1 and project_id = $2 and id = $3 returning blob_key",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let key = key.ok_or_else(|| missing(access))?;
    if let Err(e) = blobs.remove(&key).await {
        eprintln!("⚠ ekin nesnesi silinemedi ({key}): {e}");
    }
    Ok(())
}
