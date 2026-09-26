//! `project.checkpoint.restore` v1 (docs/adr/0034, second step; TODOS.md
//! SYNC-11): a new project from a point of a project's history, made as a
//! copy is (docs/adr/0028): its own id, owned by the caller, without
//! history, grants or favourites. The source does not change; its audit
//! records the restore and keeps the answer for a retry.
//!
//! - **A file project's point** (a checkpoint, or a revision by its number)
//!   becomes a file project whose revision 1 is that revision: its object is
//!   shared in the store (docs/adr/0031), nothing is decoded.
//! - **A database project's checkpoint** becomes a database project imported
//!   from its file: the file's settings, layers, styles, origin and view,
//!   and every object with its persistent id at version 1 (data revision 1),
//!   inserted a thousand at a time. The file is checked against its hash,
//!   decoded and its objects converted before the source's lock is taken.
//! - **Who:** the source's `project.history` and `project.download`, and
//!   `project.create` in the workspace the new project goes to.

use kentos_contracts::{
    CheckpointRestore, CommandEnvelope, DocumentSnapshotV2, PROJECT_CHECKPOINT_RESTORE,
    PROJECT_CHECKPOINT_RESTORE_VERSION, ProjectDuplicated, ProjectPermission, ProjectStorage,
};
use kentos_postgres::{Scope, rescope};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::blobs::Blobs;
use crate::cad::Stored;
use crate::changes::{check_target, lock};
use crate::checkpoints::missing;
use crate::commands::input;
use crate::duplicate::{self, Copy, Origin};
use crate::error::{AppError, AppResult};
use crate::files::{self, RevisionRow};
use crate::projects::{check_name, check_srid, check_tree, gone, opened, storage_of};
use crate::tenancy::{self, Capability};
use crate::{idempotency, importing, journal, listing};

/// The point asked for.
enum Asked {
    Checkpoint(Uuid),
    Revision(i64),
}

/// The point found.
enum Point {
    /// A file project's revision, named by a checkpoint or asked by its number.
    File {
        row: RevisionRow,
        checkpoint: Option<Uuid>,
        label: String,
    },
    /// A database project's checkpoint.
    Snapshot {
        key: String,
        sha256: String,
        checkpoint: Uuid,
        revision: i64,
        label: String,
    },
}

/// The name of a restored project unless it is given one: the source's with
/// the point's after it, kept within the 200 bytes of a name.
pub fn restore_name(name: &str, label: &str) -> String {
    let mut label_end = label.len().min(60);
    while !label.is_char_boundary(label_end) {
        label_end -= 1;
    }
    let suffix = format!(" ({})", &label[..label_end]);
    let room = 200 - suffix.len();
    let mut end = name.len().min(room);
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{suffix}", name[..end].trim_end())
}

async fn resolve(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    asked: Asked,
) -> AppResult<Point> {
    match asked {
        Asked::Checkpoint(id) => {
            let row: Option<(String, i64, String, Option<String>)> = sqlx::query_as(
                "select name, revision, sha256, blob_key from kentos.project_checkpoint
                  where tenant_id = $1 and project_id = $2 and id = $3",
            )
            .bind(access.tenant)
            .bind(access.project)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
            let (label, revision, sha256, key) = row.ok_or_else(missing)?;
            Ok(match key {
                Some(key) => Point::Snapshot {
                    key,
                    sha256,
                    checkpoint: id,
                    revision,
                    label,
                },
                None => Point::File {
                    row: files::revision(tx, access.tenant, access.project, revision)
                        .await?
                        .ok_or_else(missing)?,
                    checkpoint: Some(id),
                    label,
                },
            })
        }
        Asked::Revision(n) => {
            let storage: String = sqlx::query_scalar(
                "select storage from kentos.project where tenant_id = $1 and id = $2",
            )
            .bind(access.tenant)
            .bind(access.project)
            .fetch_one(&mut **tx)
            .await?;
            if storage_of(&storage) != ProjectStorage::File {
                return Err(AppError::invalid_at(
                    "fileRevision",
                    format!(
                        "“{}” projesi nesne nesne veritabanında saklanıyor; dosya revizyonu yoktur. Bir kontrol noktası verin.",
                        access.name
                    ),
                ));
            }
            let row = files::revision(tx, access.tenant, access.project, n)
                .await?
                .ok_or_else(|| {
                    AppError::invalid_at(
                        "fileRevision",
                        format!("“{}” projesinin {n}. revizyonu yok.", access.name),
                    )
                })?;
            Ok(Point::File {
                row,
                checkpoint: None,
                label: format!("r{n}"),
            })
        }
    }
}

/// `project.checkpoint.restore` v1.
pub async fn restore(
    db: &kentos_postgres::Db,
    blobs: &Blobs,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectDuplicated> {
    let CheckpointRestore {
        checkpoint_id,
        file_revision,
        name,
        tenant_id,
    } = input(
        &envelope,
        PROJECT_CHECKPOINT_RESTORE,
        PROJECT_CHECKPOINT_RESTORE_VERSION,
    )?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    if let Some(n) = &name {
        check_name(n)?;
    }
    let asked = match (checkpoint_id.as_deref(), file_revision.as_deref()) {
        (Some(c), None) => Asked::Checkpoint(Uuid::parse_str(c).map_err(|_| missing())?),
        (None, Some(r)) => Asked::Revision(r.parse().ok().filter(|n| *n > 0).ok_or_else(|| {
            AppError::invalid_at(
                "fileRevision",
                format!("fileRevision bir revizyon numarası olmalı: {r}"),
            )
        })?),
        _ => {
            return Err(AppError::invalid(
                "Geri yüklenecek noktayı verin: checkpointId ya da fileRevision (yalnız biri).",
            ));
        }
    };
    // Where the new project goes: a workspace the caller may open projects in (404 for one they are not in).
    let target = match tenant_id.as_deref() {
        None => access.tenant,
        Some(t) => Uuid::parse_str(t)
            .map_err(|_| AppError::not_found("Kurum bulunamadı ya da üyesi değilsiniz."))?,
    };
    tenancy::access(db, &access.actor, target)
        .await?
        .require(Capability::ProjectCreate)?;
    access.live()?;
    access.require(ProjectPermission::History)?;
    access.require(ProjectPermission::Download)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let point = resolve(&mut tx, access, asked).await?;
    tx.commit().await?;
    let id = Uuid::now_v7();
    match point {
        Point::File {
            row,
            checkpoint,
            label,
        } => {
            let name = name
                .map(|n| n.trim().to_string())
                .unwrap_or_else(|| restore_name(&access.name, &label));
            let key = Blobs::revision_key(target, id, 1, &row.2);
            blobs.share(&row.7, &key).await?;
            let mut tx = db.scoped(access.scope()).await?;
            let origin = Origin::Restore {
                checkpoint,
                revision: row.0,
            };
            let written = async {
                let now = match recheck(&mut tx, access, &envelope, &text).await? {
                    Rechecked::Earlier(earlier) => return Ok(Done::Earlier(*earlier)),
                    Rechecked::Fresh(now) => now,
                };
                let copy = Copy {
                    target,
                    id,
                    name: &name,
                    file: Some((&row, &key)),
                    origin,
                };
                duplicate::write(&mut tx, &now, &envelope, &text, copy)
                    .await
                    .map(Done::New)
            }
            .await;
            finish(tx, blobs, Some(&key), written).await
        }
        Point::Snapshot {
            key,
            sha256,
            checkpoint,
            revision,
            label,
        } => {
            let name = name
                .map(|n| n.trim().to_string())
                .unwrap_or_else(|| restore_name(&access.name, &label));
            let doc =
                importing::read(blobs, &key, Some(&sha256), "kontrol noktasının dosyası").await?;
            let rows = importing::objects(&doc)?;
            check_tree(&doc.layers, &doc.active_layer)?;
            let mut tx = db.scoped(access.scope()).await?;
            let written = async {
                let now = match recheck(&mut tx, access, &envelope, &text).await? {
                    Rechecked::Earlier(earlier) => return Ok(Done::Earlier(*earlier)),
                    Rechecked::Fresh(now) => now,
                };
                let import = Import {
                    target,
                    id,
                    name: &name,
                    doc: &doc,
                    rows: &rows,
                    checkpoint,
                    revision,
                };
                import_into(&mut tx, &now, &envelope, &text, import)
                    .await
                    .map(Done::New)
            }
            .await;
            finish(tx, blobs, None, written).await
        }
    }
}

/// What the rows' writing ended in, the commit still to come.
enum Done {
    Earlier(ProjectDuplicated),
    New(ProjectDuplicated),
}

/// The source under its lock: a retry's stored answer, or the access asked again.
enum Rechecked {
    Earlier(Box<ProjectDuplicated>),
    Fresh(ProjectAccess),
}

async fn recheck(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
) -> AppResult<Rechecked> {
    let now = lock(tx, access).await?;
    if let Some(earlier) =
        idempotency::earlier::<ProjectDuplicated>(tx, &now, envelope, text).await?
    {
        return Ok(Rechecked::Earlier(Box::new(earlier)));
    }
    if now.deleted {
        return Err(gone(&now.name));
    }
    now.require(ProjectPermission::History)?;
    now.require(ProjectPermission::Download)?;
    Ok(Rechecked::Fresh(now))
}

/// Commits what was written; a retry answered from the log or an error
/// removes the object shared for this one (an unknown commit leaves it to
/// the store's cleanup, which removes it with the project that does not exist).
async fn finish(
    tx: Transaction<'static, Postgres>,
    blobs: &Blobs,
    shared: Option<&str>,
    written: AppResult<Done>,
) -> AppResult<ProjectDuplicated> {
    let drop_shared = || async {
        if let Some(key) = shared {
            let _ = blobs.remove(key).await;
        }
    };
    match written {
        Ok(Done::New(result)) => {
            tx.commit().await?;
            Ok(result)
        }
        Ok(Done::Earlier(mut earlier)) => {
            tx.commit().await?;
            drop_shared().await;
            earlier.replayed = true;
            Ok(earlier)
        }
        Err(e) => {
            drop(tx);
            drop_shared().await;
            Err(e)
        }
    }
}

/// A database project's checkpoint to import as a new project.
struct Import<'a> {
    target: Uuid,
    id: Uuid,
    name: &'a str,
    doc: &'a DocumentSnapshotV2,
    rows: &'a [(Uuid, Stored)],
    checkpoint: Uuid,
    revision: i64,
}

/// The new project's rows, both projects' audit records and the stored answer, in the open transaction.
async fn import_into(
    tx: &mut Transaction<'static, Postgres>,
    now: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
    import: Import<'_>,
) -> AppResult<ProjectDuplicated> {
    let Import {
        target,
        id,
        name,
        doc,
        rows,
        checkpoint,
        revision,
    } = import;
    let request = Some(envelope.request_id.as_str());
    let actor = now.actor.user_id;
    let from = json!({ "checkpoint": checkpoint, "revision": revision });
    // The source's catalog metadata goes with it, as a copy's does.
    let (description, project_type, tags): (String, String, Vec<String>) = sqlx::query_as(
        "select description, project_type, tags from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(now.tenant)
    .bind(now.project)
    .fetch_one(&mut **tx)
    .await?;
    let source_revision = journal::revision(tx, now).await?;
    journal::audit(
        tx,
        now,
        PROJECT_CHECKPOINT_RESTORE,
        request,
        source_revision,
        json!({ "copy": id, "tenant": target, "name": name, "objects": rows.len(), "from": from }),
    )
    .await?;
    // The new project's own rows, in its scope.
    rescope(
        tx,
        Scope {
            tenant: Some(target),
            user: Some(actor),
            project: Some(id),
        },
    )
    .await?;
    check_srid(tx, doc.settings.srid).await?;
    let value = |v: serde_json::Result<serde_json::Value>| {
        v.map_err(|e| AppError::invalid(format!("Kontrol noktasının kaydı yazılamadı: {e}")))
    };
    sqlx::query(
        "insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y, home_view, styles, created_by, owner_user_id,
                                     description, project_type, tags, storage)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12, $13, $14, $15, 'database')",
    )
    .bind(target)
    .bind(id)
    .bind(name)
    .bind(doc.settings.srid as i32)
    .bind(value(serde_json::to_value(&doc.settings))?)
    .bind(value(serde_json::to_value(&doc.layers))?)
    .bind(&doc.active_layer)
    .bind(doc.origin.x)
    .bind(doc.origin.y)
    .bind(doc.home_view.map(serde_json::to_value).transpose().map_err(|e| {
        AppError::invalid(format!("Kontrol noktasının kaydı yazılamadı: {e}"))
    })?)
    .bind(value(serde_json::to_value(&doc.styles))?)
    .bind(actor)
    .bind(&description)
    .bind(&project_type)
    .bind(&tags)
    .execute(&mut **tx)
    .await?;
    for batch in rows.chunks(importing::BATCH) {
        importing::insert_objects(tx, target, id, doc.settings.srid, actor, batch).await?;
    }
    if !rows.is_empty() {
        // A created object's version is its commit's data revision (docs/adr/0026): this is the first.
        sqlx::query("update kentos.project set data_revision = 1 where tenant_id = $1 and id = $2")
            .bind(target)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    let subject = journal::Subject {
        tenant: target,
        project: id,
        actor,
    };
    journal::audit(
        tx,
        subject,
        "project.create",
        request,
        i64::from(!rows.is_empty()),
        json!({ "name": name, "restoredFrom": { "tenant": now.tenant, "project": now.project, "checkpoint": checkpoint, "revision": revision }, "objects": rows.len() }),
    )
    .await?;
    opened(tx, target, id, actor).await?;
    let project = listing::entry(tx, target, id).await?;
    // Back in the source's scope, where its command log is.
    rescope(tx, now.scope()).await?;
    let result = ProjectDuplicated {
        project,
        source_id: now.project.to_string(),
        objects: rows.len().to_string(),
        replayed: false,
    };
    idempotency::record(tx, now, envelope, text, &result).await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_restored_project_is_named_after_its_source_and_point_within_the_limit() {
        assert_eq!(restore_name("Ada 101", "Teslim"), "Ada 101 (Teslim)");
        assert_eq!(restore_name("Ada 101", "r3"), "Ada 101 (r3)");
        let long = "ş".repeat(100); // 200 bytes
        let named = restore_name(&long, &"ğ".repeat(80));
        assert!(named.len() <= 200 && named.ends_with(')'), "{named}");
    }
}
