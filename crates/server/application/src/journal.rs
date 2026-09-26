//! What a catalog or lifecycle command leaves besides its change
//! (docs/adr/0028): the audit record, with the actor, tenant, project,
//! revision and request (TODOS.md CLOUD-25), and the outbox event the
//! project's open connections hear. Both are written in the command's own
//! transaction, in the project's scope, so neither can exist without the
//! change. Neither holds a name or a grant beyond what the audit needs:
//! events carry no people and no content.

use kentos_contracts::EventRecord;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::error::AppResult;

/// Whose project, and who acts: what every record names.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Subject {
    pub tenant: Uuid,
    pub project: Uuid,
    pub actor: Uuid,
}

impl From<&ProjectAccess> for Subject {
    fn from(a: &ProjectAccess) -> Self {
        Self {
            tenant: a.tenant,
            project: a.project,
            actor: a.actor.user_id,
        }
    }
}

/// The project's data revision now (a change that does not touch its data keeps it).
pub(crate) async fn revision(
    tx: &mut Transaction<'static, Postgres>,
    at: impl Into<Subject>,
) -> AppResult<i64> {
    let at = at.into();
    Ok(sqlx::query_scalar(
        "select data_revision from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(at.tenant)
    .bind(at.project)
    .fetch_one(&mut **tx)
    .await?)
}

/// One audit record of the caller's action on the project.
pub(crate) async fn audit(
    tx: &mut Transaction<'static, Postgres>,
    at: impl Into<Subject>,
    action: &str,
    request_id: Option<&str>,
    revision: i64,
    detail: Value,
) -> AppResult<()> {
    let at = at.into();
    sqlx::query(
        "insert into kentos.audit_event (tenant_id, project_id, actor, action, request_id, data_revision, detail)
         values ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(at.tenant)
    .bind(at.project)
    .bind(at.actor)
    .bind(action)
    .bind(request_id)
    .bind(revision)
    .bind(detail)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// An event without objects (`kind`; `meta` when the drawing's own metadata,
/// its name, changed); returns its cursor.
pub(crate) async fn event(
    tx: &mut Transaction<'static, Postgres>,
    at: impl Into<Subject>,
    kind: &str,
    request_id: Option<&str>,
    revision: i64,
    meta: bool,
) -> AppResult<i64> {
    let at = at.into();
    let record = EventRecord {
        seq: String::new(),
        data_revision: revision.to_string(),
        kind: kind.into(),
        actor: Some(at.actor.to_string()),
        request_id: request_id.map(str::to_string),
        features: Vec::new(),
        meta,
    };
    Ok(sqlx::query_scalar(
        "insert into kentos.outbox_event (tenant_id, project_id, data_revision, kind, payload) values ($1, $2, $3, $4, $5) returning seq",
    )
    .bind(at.tenant)
    .bind(at.project)
    .bind(revision)
    .bind(kind)
    .bind(serde_json::to_value(&record).expect("events serialize"))
    .fetch_one(&mut **tx)
    .await?)
}
