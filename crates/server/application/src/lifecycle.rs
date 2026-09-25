//! A project's life beyond its content: deleting it (CLAUDE.md §16).
//! Deleting is soft (migration 0002): the project leaves the lists and
//! refuses opening and writing (410, `project_deleted`, see
//! `projects::gone`), but its objects, command log, audit and events stay,
//! so the operator can restore it (`admin::restore_project`). Its event log
//! stays readable: an editor who was away still learns why it stopped.
//!
//! Who deletes (docs/adr/0015): `project.delete`, the project's owner and,
//! while the organisation's policy is on, its owners and admins.

use kentos_contracts::{EventRecord, PROJECT_DELETED, ProjectPermission};

use crate::access::ProjectAccess;
use crate::changes::lock;
use crate::error::AppResult;

/// Deletes a project for everyone: marks it, and writes the audit record and
/// a `project.deleted` event (its open editors stop sending) in the same
/// transaction. Deleting it again changes nothing (a retry after a lost
/// answer). Returns the event cursor when it was deleted now.
pub async fn delete(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    request_id: Option<&str>,
) -> AppResult<Option<i64>> {
    access.require(ProjectPermission::Delete)?;
    let mut tx = db.scoped(access.scope()).await?;
    // Locked like a commit: one in progress finishes first, and none starts after this.
    let now = lock(&mut tx, access).await?;
    now.require(ProjectPermission::Delete)?;
    if now.deleted {
        tx.commit().await?;
        return Ok(None);
    }
    let actor = now.actor.user_id;
    let revision: i64 = sqlx::query_scalar(
        "update kentos.project set deleted_at = now(), deleted_by = $3, data_revision = data_revision + 1, updated_at = now()
          where tenant_id = $1 and id = $2 returning data_revision",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(actor)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "insert into kentos.audit_event (tenant_id, project_id, actor, action, request_id, data_revision, detail)
         values ($1, $2, $3, 'project.delete', $4, $5, $6)",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(actor)
    .bind(request_id)
    .bind(revision)
    .bind(serde_json::json!({ "name": now.name }))
    .execute(&mut *tx)
    .await?;
    let event = EventRecord {
        seq: String::new(),
        data_revision: revision.to_string(),
        kind: PROJECT_DELETED.into(),
        actor: Some(actor.to_string()),
        request_id: request_id.map(str::to_string),
        features: Vec::new(),
        meta: false,
    };
    let seq: i64 = sqlx::query_scalar(
        "insert into kentos.outbox_event (tenant_id, project_id, data_revision, kind, payload) values ($1, $2, $3, $4, $5) returning seq",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(revision)
    .bind(PROJECT_DELETED)
    .bind(serde_json::to_value(&event).expect("event serializes"))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(seq))
}
