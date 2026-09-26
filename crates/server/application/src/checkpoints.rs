//! Checkpoints: named points of a project's history (docs/adr/0034;
//! TODOS.md SYNC-11, CLOUD-07).
//!
//! - **A database project's checkpoint** is its snapshot of one moment
//!   (docs/adr/0033), kept in the object store. The snapshot is taken
//!   without the project's lock, so writers never wait for it; its object is
//!   written, and only then is the row committed under the lock, with the
//!   access asked again (TODOS.md SYNC-16). A failure before the commit
//!   removes the object; an object whose row was never committed is removed
//!   by the store's cleanup (`files::cleanup`).
//! - **A file project's checkpoint** names one of its revisions (the newest
//!   when none is given); the row repeats the revision's size, hash and
//!   object count, and nothing is copied.
//! - **Who:** `feature.write` makes one (not in an archived or deleted
//!   project); its maker or a holder of `project.edit` removes it;
//!   `project.history` lists them, and `project.download` besides gives one's
//!   file.
//! - Each change is audited and heard by the project's open connections
//!   (`project.checkpoint`); a retried command gets the stored answer.

use kentos_contracts::{
    CHECKPOINT_NAME_MAX, CHECKPOINT_NOTE_MAX, Checkpoint, CheckpointChange, CheckpointCreate,
    CheckpointDelete, CheckpointKind, CommandEnvelope, PROJECT_CHECKPOINT_CREATE,
    PROJECT_CHECKPOINT_CREATE_VERSION, PROJECT_CHECKPOINT_DELETE,
    PROJECT_CHECKPOINT_DELETE_VERSION, PROJECT_CHECKPOINT_EVENT, ProjectCheckpoints,
    ProjectPermission, ProjectStorage,
};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::{Blobs, WriteError};
use crate::changes::{check_target, lock};
use crate::commands::input;
use crate::error::{AppError, AppResult};
use crate::projects::{rfc3339, storage_of};
use crate::{idempotency, journal, snapshot};

/// A checkpoint's row: id, name, note, kind, revision, size, SHA-256, object
/// count, a snapshot's object key, its maker and their name, and when.
type CheckpointRow = (
    Uuid,
    String,
    Option<String>,
    String,
    i64,
    i64,
    String,
    Option<i64>,
    Option<String>,
    Uuid,
    String,
    OffsetDateTime,
);

const CHECKPOINT_SELECT: &str = "select c.id, c.name, c.note, c.kind, c.revision, c.size, c.sha256, c.objects, c.blob_key, c.created_by, coalesce(u.display_name, ''), c.created_at
   from kentos.project_checkpoint c left join kentos.app_user u on u.id = c.created_by
  where c.tenant_id = $1 and c.project_id = $2";

fn checkpoint_of(row: &CheckpointRow) -> Checkpoint {
    let (id, name, note, kind, revision, size, sha256, objects, _, by, by_name, at) = row;
    Checkpoint {
        id: id.to_string(),
        name: name.clone(),
        note: note.clone(),
        kind: if kind == "snapshot" {
            CheckpointKind::Snapshot
        } else {
            CheckpointKind::Revision
        },
        revision: revision.to_string(),
        size: size.to_string(),
        sha256: sha256.clone(),
        objects: objects.map(|n| n.to_string()),
        created_by: by.to_string(),
        created_by_name: by_name.clone(),
        created_at: rfc3339(*at),
    }
}

/// A checkpoint's name as it is kept: trimmed, 1 to [`CHECKPOINT_NAME_MAX`]
/// characters, on one line.
fn name_of(name: &str) -> AppResult<String> {
    let name = name.trim();
    let count = name.chars().count();
    if count == 0 || count > CHECKPOINT_NAME_MAX {
        return Err(AppError::invalid_at(
            "name",
            format!(
                "Kontrol noktasının adı 1 ile {CHECKPOINT_NAME_MAX} karakter arasında olmalı ({count} karakter)."
            ),
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(AppError::invalid_at(
            "name",
            "Kontrol noktasının adı tek satır olmalı; satır sonu ya da denetim karakteri içeremez.",
        ));
    }
    Ok(name.to_string())
}

/// A note as it is kept: trimmed, at most [`CHECKPOINT_NOTE_MAX`]
/// characters, lines and tabs allowed; an empty one is none.
fn note_of(note: Option<String>) -> AppResult<Option<String>> {
    let Some(note) = note else {
        return Ok(None);
    };
    let note = note.trim();
    let count = note.chars().count();
    if count > CHECKPOINT_NOTE_MAX {
        return Err(AppError::invalid_at(
            "note",
            format!("Not en çok {CHECKPOINT_NOTE_MAX} karakter olabilir ({count} karakter)."),
        ));
    }
    if note
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(AppError::invalid_at(
            "note",
            "Not denetim karakteri içeremez (satır sonu ve sekme olabilir).",
        ));
    }
    Ok((!note.is_empty()).then(|| note.to_string()))
}

fn missing() -> AppError {
    AppError::not_found("Kontrol noktası bulunamadı; silinmiş olabilir. Listeyi yenileyin.")
}

/// What a new checkpoint keeps.
enum Source {
    /// A database project's snapshot, its object already written.
    Snapshot {
        id: Uuid,
        revision: i64,
        size: i64,
        sha256: String,
        objects: i64,
        key: String,
    },
    /// A file project's revision: the one asked for, or the newest.
    Revision(Option<i64>),
}

/// What the row writing ended in, the commit still to come.
enum Written {
    /// An identical command was answered before: its answer.
    Earlier(CheckpointChange),
    New(CheckpointChange),
}

/// `project.checkpoint.create` v1.
pub async fn create(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<CheckpointChange> {
    let CheckpointCreate {
        name,
        note,
        file_revision,
    } = input(
        &envelope,
        PROJECT_CHECKPOINT_CREATE,
        PROJECT_CHECKPOINT_CREATE_VERSION,
    )?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let name = name_of(&name)?;
    let note = note_of(note)?;
    access.writable()?;
    access.require(ProjectPermission::FeatureWrite)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let storage: String =
        sqlx::query_scalar("select storage from kentos.project where tenant_id = $1 and id = $2")
            .bind(access.tenant)
            .bind(access.project)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(crate::access::not_found)?;
    tx.commit().await?;
    let asked = file_revision
        .as_deref()
        .map(|t| {
            t.parse::<i64>().ok().filter(|n| *n > 0).ok_or_else(|| {
                AppError::invalid_at(
                    "fileRevision",
                    format!("fileRevision bir revizyon numarası olmalı: {t}"),
                )
            })
        })
        .transpose()?;
    let (source, key) = match storage_of(&storage) {
        ProjectStorage::File => (Source::Revision(asked), None),
        ProjectStorage::Database => {
            if asked.is_some() {
                return Err(AppError::invalid_at(
                    "fileRevision",
                    "Veritabanı projesinin kontrol noktası şimdiki hâlidir; revizyon verilmez.",
                ));
            }
            // Taken before the lock: writers never wait for a snapshot.
            let snap = snapshot::take(db, access).await?;
            let id = Uuid::now_v7();
            let key = Blobs::checkpoint_key(access.tenant, access.project, id, &snap.sha256);
            let size = snap.bytes.len();
            let mut writer = blobs.create(&key, size as u64).await?;
            if let Err(e) = writer.write(&snap.bytes).await {
                let _ = blobs.remove(&key).await;
                return Err(match e {
                    WriteError::Io(e) => AppError::Storage(e),
                    other => AppError::Storage(std::io::Error::other(format!("{other:?}"))),
                });
            }
            writer.finish().await?;
            let source = Source::Snapshot {
                id,
                revision: snap.revision,
                size: i64::try_from(size).unwrap_or(i64::MAX),
                sha256: snap.sha256,
                objects: i64::try_from(snap.objects).unwrap_or(i64::MAX),
                key: key.clone(),
            };
            (source, Some(key))
        }
    };
    let mut tx = db.scoped(access.scope()).await?;
    let written = write(&mut tx, access, &envelope, &text, &name, note, source).await;
    match written {
        Ok(Written::New(result)) => {
            // A commit that fails may still have been written: the object stays either
            // way; if nothing was written, the store's cleanup removes it.
            tx.commit().await?;
            Ok(result)
        }
        Ok(Written::Earlier(mut earlier)) => {
            tx.commit().await?;
            // The earlier command kept its own object; this one's is nobody's.
            if let Some(key) = &key {
                let _ = blobs.remove(key).await;
            }
            earlier.replayed = true;
            Ok(earlier)
        }
        Err(e) => {
            if let Some(key) = &key {
                let _ = blobs.remove(key).await;
            }
            Err(e)
        }
    }
}

/// Under the project's lock: the stored answer of a retry, or the new row,
/// its audit, its event and the stored answer (not committed).
async fn write(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
    name: &str,
    note: Option<String>,
    source: Source,
) -> AppResult<Written> {
    let now = lock(tx, access).await?;
    if let Some(earlier) =
        idempotency::earlier::<CheckpointChange>(tx, &now, envelope, text).await?
    {
        return Ok(Written::Earlier(earlier));
    }
    now.writable()?;
    now.require(ProjectPermission::FeatureWrite)?;
    let (id, kind, revision, size, sha256, objects, key) = match source {
        Source::Snapshot {
            id,
            revision,
            size,
            sha256,
            objects,
            key,
        } => (
            id,
            "snapshot",
            revision,
            size,
            sha256,
            Some(objects),
            Some(key),
        ),
        Source::Revision(asked) => {
            let newest: Option<i64> = sqlx::query_scalar(
                "select file_revision from kentos.project where tenant_id = $1 and id = $2",
            )
            .bind(now.tenant)
            .bind(now.project)
            .fetch_one(&mut **tx)
            .await?;
            let revision = match (asked, newest) {
                (Some(n), _) => n,
                (None, Some(n)) => n,
                (None, None) => {
                    return Err(AppError::invalid_at(
                        "fileRevision",
                        format!(
                            "“{}” projesinin henüz kaydedilmiş bir revizyonu yok; önce projeyi kaydedin.",
                            now.name
                        ),
                    ));
                }
            };
            let found: Option<(i64, String, Option<i64>)> = sqlx::query_as(
                "select size, sha256, objects from kentos.project_file_revision
                  where tenant_id = $1 and project_id = $2 and revision = $3",
            )
            .bind(now.tenant)
            .bind(now.project)
            .bind(revision)
            .fetch_optional(&mut **tx)
            .await?;
            let (size, sha256, objects) = found.ok_or_else(|| {
                AppError::invalid_at(
                    "fileRevision",
                    format!("“{}” projesinin {revision}. revizyonu yok.", now.name),
                )
            })?;
            (
                Uuid::now_v7(),
                "revision",
                revision,
                size,
                sha256,
                objects,
                None,
            )
        }
    };
    let created: OffsetDateTime = sqlx::query_scalar(
        "insert into kentos.project_checkpoint (tenant_id, project_id, id, name, note, kind, revision, size, sha256, objects, blob_key, created_by, request_id)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) returning created_at",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(id)
    .bind(name)
    .bind(&note)
    .bind(kind)
    .bind(revision)
    .bind(size)
    .bind(&sha256)
    .bind(objects)
    .bind(&key)
    .bind(now.actor.user_id)
    .bind(&envelope.request_id)
    .fetch_one(&mut **tx)
    .await?;
    let request = Some(envelope.request_id.as_str());
    let data_revision = journal::revision(tx, &now).await?;
    journal::audit(
        tx,
        &now,
        PROJECT_CHECKPOINT_CREATE,
        request,
        data_revision,
        json!({ "checkpoint": id, "name": name, "kind": kind, "revision": revision, "size": size, "sha256": sha256 }),
    )
    .await?;
    journal::event(
        tx,
        &now,
        PROJECT_CHECKPOINT_EVENT,
        request,
        data_revision,
        false,
    )
    .await?;
    let row: CheckpointRow = (
        id,
        name.to_string(),
        note,
        kind.to_string(),
        revision,
        size,
        sha256,
        objects,
        key,
        now.actor.user_id,
        now.actor.display_name.clone(),
        created,
    );
    let result = CheckpointChange {
        checkpoint: checkpoint_of(&row),
        removed: false,
        replayed: false,
    };
    idempotency::record(tx, &now, envelope, text, &result).await?;
    Ok(Written::New(result))
}

/// `project.checkpoint.delete` v1: its maker, or a holder of `project.edit`.
pub async fn delete(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<CheckpointChange> {
    let CheckpointDelete { checkpoint_id } = input(
        &envelope,
        PROJECT_CHECKPOINT_DELETE,
        PROJECT_CHECKPOINT_DELETE_VERSION,
    )?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let id = Uuid::parse_str(&checkpoint_id).map_err(|_| missing())?;
    access.writable()?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let now = lock(&mut tx, access).await?;
    if let Some(mut earlier) =
        idempotency::earlier::<CheckpointChange>(&mut tx, &now, &envelope, &text).await?
    {
        tx.commit().await?;
        earlier.replayed = true;
        return Ok(earlier);
    }
    now.writable()?;
    now.require(ProjectPermission::FeatureWrite)?;
    let row: CheckpointRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{CHECKPOINT_SELECT} and c.id = $3"
    )))
    .bind(now.tenant)
    .bind(now.project)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(missing)?;
    if row.9 != now.actor.user_id && !now.allows(ProjectPermission::Edit) {
        return Err(AppError::forbidden(format!(
            "“{}” kontrol noktasını yalnız onu oluşturan kişi ya da projeyi yöneten (project.edit) siler.",
            row.1
        )));
    }
    sqlx::query(
        "delete from kentos.project_checkpoint where tenant_id = $1 and project_id = $2 and id = $3",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    let request = Some(envelope.request_id.as_str());
    let data_revision = journal::revision(&mut tx, &now).await?;
    journal::audit(
        &mut tx,
        &now,
        PROJECT_CHECKPOINT_DELETE,
        request,
        data_revision,
        json!({ "checkpoint": id, "name": row.1, "kind": row.3, "revision": row.4 }),
    )
    .await?;
    journal::event(
        &mut tx,
        &now,
        PROJECT_CHECKPOINT_EVENT,
        request,
        data_revision,
        false,
    )
    .await?;
    let result = CheckpointChange {
        checkpoint: checkpoint_of(&row),
        removed: true,
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &result).await?;
    tx.commit().await?;
    // The row is gone: its snapshot's object follows (the cleanup removes it if this fails).
    if let Some(key) = &row.8 {
        let _ = blobs.remove(key).await;
    }
    Ok(result)
}

/// A project's checkpoints, newest first (`project.history`).
pub async fn list(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
) -> AppResult<ProjectCheckpoints> {
    access.live()?;
    access.require(ProjectPermission::History)?;
    let mut tx = db.scoped(access.scope()).await?;
    let rows: Vec<CheckpointRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{CHECKPOINT_SELECT} order by c.created_at desc, c.id desc"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ProjectCheckpoints {
        checkpoints: rows.iter().map(checkpoint_of).collect(),
    })
}

/// One checkpoint and its file (`project.history` and `project.download`):
/// a snapshot's own object, or the revision a file project's checkpoint names.
pub async fn download(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    id: Uuid,
) -> AppResult<(Checkpoint, tokio::fs::File)> {
    access.live()?;
    access.require(ProjectPermission::History)?;
    access.require(ProjectPermission::Download)?;
    let mut tx = db.scoped(access.scope()).await?;
    let row: CheckpointRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{CHECKPOINT_SELECT} and c.id = $3"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(missing)?;
    let key = match &row.8 {
        Some(key) => key.clone(),
        None => sqlx::query_scalar(
            "select blob_key from kentos.project_file_revision where tenant_id = $1 and project_id = $2 and revision = $3",
        )
        .bind(access.tenant)
        .bind(access.project)
        .bind(row.4)
        .fetch_one(&mut *tx)
        .await?,
    };
    tx.commit().await?;
    let file = blobs.open(&key).await?;
    Ok((checkpoint_of(&row), file))
}
