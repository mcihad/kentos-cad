//! A project's life beyond its content (CLAUDE.md §16, docs/adr/0028,
//! TODOS.md CLOUD-05): archiving, the trash and removing for good.
//!
//! - **Archive** (`project.archive`, `project.unarchive`; `project.edit`): an
//!   archived project opens and reads as before, but its content and catalog
//!   metadata do not change (409, `project_archived`) until it is unarchived.
//!   Its sharing may still change and it may be copied or moved to the trash.
//! - **Trash** (`project.trash`, the `DELETE` route; `project.delete`: the
//!   owner and, under the organisation's policy, its owners and admins,
//!   docs/adr/0015): the soft delete of migration 0002. The project leaves
//!   the lists and refuses opening and writing (410, `project_deleted`, see
//!   `projects::gone`), but its objects, command log, audit and events stay.
//!   Its event log stays readable: an editor who was away still learns why it
//!   stopped. `purge_after` fixes, at that moment, when the retention removes
//!   it (`CatalogPolicy`).
//! - **Restore** (`project.restore`; `project.delete`): back with everything;
//!   an archived project comes back archived. The operator's
//!   `kentosd project restore` does the same without an actor.
//! - **Purge** (`project.purge`; `project.delete`): only from the trash, and
//!   only with the project's name as the explicit confirmation. The objects,
//!   grants, log and events go (`kentos.purge_project`); the audit stays and
//!   records it. The retention does the same for projects whose time in the
//!   trash is over (`purge_expired`, `kentos.purge_trash`).
//!
//! Each change is one transaction under the project's lock, with its audit
//! record, its event (archive, trash and restore are heard by open
//! connections) and the idempotent answer.

use std::time::Duration;

use kentos_contracts::{
    CommandEnvelope, EmptyInput, PROJECT_ARCHIVE, PROJECT_ARCHIVE_VERSION, PROJECT_ARCHIVED,
    PROJECT_DELETED, PROJECT_PURGE, PROJECT_PURGE_VERSION, PROJECT_RESTORE,
    PROJECT_RESTORE_VERSION, PROJECT_RESTORED, PROJECT_TRASH, PROJECT_TRASH_VERSION,
    PROJECT_UNARCHIVE, PROJECT_UNARCHIVE_VERSION, PROJECT_UNARCHIVED, ProjectCatalogChange,
    ProjectPermission, ProjectPurge, ProjectPurged,
};
use serde_json::json;
use sqlx::{Postgres, Transaction};

use crate::access::ProjectAccess;
use crate::changes::{check_target, lock};
use crate::commands::{CatalogPolicy, input};
use crate::error::{AppError, AppResult};
use crate::projects::gone;
use crate::{idempotency, journal, listing};

/// A change of where a project is in its life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateChange {
    Archive,
    Unarchive,
    Trash,
    Restore,
}

impl StateChange {
    fn command(self) -> (&'static str, u32) {
        match self {
            Self::Archive => (PROJECT_ARCHIVE, PROJECT_ARCHIVE_VERSION),
            Self::Unarchive => (PROJECT_UNARCHIVE, PROJECT_UNARCHIVE_VERSION),
            Self::Trash => (PROJECT_TRASH, PROJECT_TRASH_VERSION),
            Self::Restore => (PROJECT_RESTORE, PROJECT_RESTORE_VERSION),
        }
    }
}

fn seconds(d: Duration) -> f64 {
    d.as_secs_f64()
}

/// Moves the locked project (`now`, asked again under the lock) to the trash,
/// unless it is there already; returns the event's cursor when it moved now.
async fn trash_in(
    tx: &mut Transaction<'static, Postgres>,
    now: &ProjectAccess,
    request_id: Option<&str>,
    retention: Duration,
) -> AppResult<Option<i64>> {
    now.require(ProjectPermission::Delete)?;
    if now.deleted {
        return Ok(None);
    }
    let (revision, purge_after): (i64, time::OffsetDateTime) = sqlx::query_as(
        "update kentos.project set deleted_at = now(), deleted_by = $3, purge_after = now() + make_interval(secs => $4),
                data_revision = data_revision + 1, updated_at = now()
          where tenant_id = $1 and id = $2 returning data_revision, purge_after",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(now.actor.user_id)
    .bind(seconds(retention))
    .fetch_one(&mut **tx)
    .await?;
    journal::audit(
        tx,
        now,
        PROJECT_TRASH,
        request_id,
        revision,
        json!({ "name": now.name, "purgeAfter": crate::projects::rfc3339(purge_after) }),
    )
    .await?;
    Ok(Some(
        journal::event(tx, now, PROJECT_DELETED, request_id, revision, false).await?,
    ))
}

/// Deletes a project for everyone (the `DELETE` route): moves it to the
/// trash for the default retention. Deleting it again changes nothing (a
/// retry after a lost answer). Returns the event cursor when it was deleted now.
pub async fn delete(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    request_id: Option<&str>,
) -> AppResult<Option<i64>> {
    delete_with(db, &CatalogPolicy::default(), access, request_id).await
}

/// [`delete`] under the server's own retention.
pub async fn delete_with(
    db: &kentos_postgres::Db,
    policy: &CatalogPolicy,
    access: &ProjectAccess,
    request_id: Option<&str>,
) -> AppResult<Option<i64>> {
    access.require(ProjectPermission::Delete)?;
    let mut tx = db.scoped(access.scope()).await?;
    // Locked like a commit: one in progress finishes first, and none starts after this.
    let now = lock(&mut tx, access).await?;
    let seq = trash_in(&mut tx, &now, request_id, policy.trash_retention).await?;
    tx.commit().await?;
    Ok(seq)
}

/// `project.archive`, `project.unarchive`, `project.trash` and `project.restore` v1.
pub async fn change_state(
    db: &kentos_postgres::Db,
    policy: &CatalogPolicy,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
    change: StateChange,
) -> AppResult<ProjectCatalogChange> {
    let (name, version) = change.command();
    let EmptyInput {} = input(&envelope, name, version)?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let text = idempotency::request_text(&envelope);
    let request = Some(envelope.request_id.as_str());
    let mut tx = db.scoped(access.scope()).await?;
    let now = lock(&mut tx, access).await?;
    if let Some(mut earlier) =
        idempotency::earlier::<ProjectCatalogChange>(&mut tx, &now, &envelope, &text).await?
    {
        tx.commit().await?;
        earlier.replayed = true;
        return Ok(earlier);
    }
    let seq = match change {
        StateChange::Trash => trash_in(&mut tx, &now, request, policy.trash_retention).await?,
        StateChange::Restore => {
            now.require(ProjectPermission::Delete)?;
            if !now.deleted {
                None
            } else {
                sqlx::query(
                    "update kentos.project set deleted_at = null, deleted_by = null, purge_after = null
                      where tenant_id = $1 and id = $2",
                )
                .bind(now.tenant)
                .bind(now.project)
                .execute(&mut *tx)
                .await?;
                let revision = journal::revision(&mut tx, &now).await?;
                journal::audit(
                    &mut tx,
                    &now,
                    PROJECT_RESTORE,
                    request,
                    revision,
                    json!({ "name": now.name }),
                )
                .await?;
                Some(
                    journal::event(&mut tx, &now, PROJECT_RESTORED, request, revision, false)
                        .await?,
                )
            }
        }
        StateChange::Archive | StateChange::Unarchive => {
            if now.deleted {
                return Err(gone(&now.name));
            }
            now.require(ProjectPermission::Edit)?;
            let archive = change == StateChange::Archive;
            if now.archived == archive {
                None
            } else {
                sqlx::query(
                    "update kentos.project set archived_at = case when $3 then now() end, archived_by = case when $3 then $4::uuid end
                      where tenant_id = $1 and id = $2",
                )
                .bind(now.tenant)
                .bind(now.project)
                .bind(archive)
                .bind(now.actor.user_id)
                .execute(&mut *tx)
                .await?;
                let revision = journal::revision(&mut tx, &now).await?;
                journal::audit(
                    &mut tx,
                    &now,
                    name,
                    request,
                    revision,
                    json!({ "name": now.name }),
                )
                .await?;
                let kind = if archive {
                    PROJECT_ARCHIVED
                } else {
                    PROJECT_UNARCHIVED
                };
                Some(journal::event(&mut tx, &now, kind, request, revision, false).await?)
            }
        }
    };
    let result = ProjectCatalogChange {
        project: listing::entry(&mut tx, now.tenant, now.project).await?,
        changed: seq.is_some(),
        event_seq: seq.map(|s| s.to_string()),
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &result).await?;
    tx.commit().await?;
    Ok(result)
}

/// `project.purge` v1: removes a project in the trash for good. The answer
/// is not kept for a retry: the command log goes with the project, and a
/// retry finds nothing (404).
pub async fn purge(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectPurged> {
    let ProjectPurge { confirm_name } = input(&envelope, PROJECT_PURGE, PROJECT_PURGE_VERSION)?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let now = lock(&mut tx, access).await?;
    // A key another command of this project used is refused, as everywhere.
    if idempotency::earlier::<serde_json::Value>(&mut tx, &now, &envelope, &text)
        .await?
        .is_some()
    {
        return Err(AppError::invalid(
            "Bu idempotency anahtarı başka bir istek için kullanılmış; her komuta yeni bir anahtar verin.",
        ));
    }
    now.require(ProjectPermission::Delete)?;
    if !now.deleted {
        return Err(AppError::invalid(format!(
            "“{}” çöp kutusunda değil. Kalıcı olarak silmek için önce çöp kutusuna taşıyın.",
            now.name
        )));
    }
    if confirm_name.trim() != now.name {
        return Err(AppError::invalid_at(
            "confirmName",
            format!(
                "Kalıcı silme onaylanmadı: confirmName projenin adıyla aynı olmalı (“{}”). Hiçbir şey silinmedi.",
                now.name
            ),
        ));
    }
    let (name, objects): (String, i64) =
        sqlx::query_as("select purged_name, purged_objects from kentos.purge_project($1, $2, $3)")
            .bind(now.tenant)
            .bind(now.project)
            .bind(&envelope.request_id)
            .fetch_one(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(ProjectPurged {
        project_id: now.project.to_string(),
        name,
        objects: objects.to_string(),
    })
}

/// The retention: removes for good up to `batch` projects whose time in the
/// trash is over, of every tenant (`kentos.purge_trash`, which runs as the
/// owner and never removes one moved there less than a day ago). Returns how
/// many went: a full batch means there may be more.
pub async fn purge_expired(db: &kentos_postgres::Db, batch: i32) -> AppResult<i64> {
    Ok(sqlx::query_scalar("select kentos.purge_trash($1)")
        .bind(batch)
        .fetch_one(&db.pool)
        .await?)
}
