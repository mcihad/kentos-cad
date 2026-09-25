//! Sharing a project (docs/adr/0015, TODOS.md CLOUD-13, CLOUD-16):
//! `project.share` gives a person a role in a project or changes it,
//! `project.access.revoke` takes it away; both need `project.share`. Each
//! change is one transaction under the project's lock, with its audit
//! record, a `project.access` event and the idempotent answer. Open
//! connections of the project hear the event, check their access again, and
//! one that lost it is closed; the next request of the person is refused.
//! Copies downloaded before are not taken back.
//!
//! - The owner's access is not a grant: it changes by a transfer (CLOUD-08).
//! - In an organisation only its members can be given a role (guests,
//!   CLOUD-17, come later); a personal space's projects can be shared with
//!   any account.
//! - Nobody changes their own access this way.

use kentos_contracts::{
    CommandEnvelope, EventRecord, GrantRole, PROJECT_ACCESS_CHANGED, PROJECT_ACCESS_REVOKE,
    PROJECT_ACCESS_REVOKE_VERSION, PROJECT_SHARE, PROJECT_SHARE_VERSION, ProjectAccessChange,
    ProjectAccessList, ProjectAccessRevoke, ProjectGrant, ProjectPermission, ProjectShare,
    TenantKind,
};
use serde::de::DeserializeOwned;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::changes::{check_target, lock};
use crate::error::{AppError, AppResult};
use crate::idempotency;
use crate::projects::{gone, rfc3339};

type GrantRow = (
    Uuid,
    Option<String>,
    String,
    Option<OffsetDateTime>,
    OffsetDateTime,
);

const GRANT_SELECT: &str = "select g.user_id, u.display_name, g.role, g.expires_at, g.updated_at
   from kentos.project_grant g left join kentos.app_user u on u.id = g.user_id
  where g.tenant_id = $1 and g.project_id = $2";

fn grant((user, name, role, expires, updated): GrantRow) -> Option<ProjectGrant> {
    Some(ProjectGrant {
        user_id: user.to_string(),
        display_name: name.unwrap_or_default(),
        role: GrantRole::from_name(&role)?,
        expires_at: expires.map(rfc3339),
        updated_at: rfc3339(updated),
    })
}

/// Who has access to a project besides the organisation's policy: its owner
/// and every grant, expired ones included (for the share dialog). Needs
/// `project.share`.
pub async fn list(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
) -> AppResult<ProjectAccessList> {
    access.live()?;
    access.require(ProjectPermission::Share)?;
    let mut tx = db.scoped(access.scope()).await?;
    let owner: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "select p.owner_user_id, u.display_name from kentos.project p left join kentos.app_user u on u.id = p.owner_user_id
          where p.tenant_id = $1 and p.id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_optional(&mut *tx)
    .await?;
    let rows: Vec<GrantRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{GRANT_SELECT} order by u.display_name, g.user_id"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let (owner_id, owner_name) = owner.ok_or_else(crate::access::not_found)?;
    Ok(ProjectAccessList {
        owner_id: owner_id.to_string(),
        owner_name: owner_name.unwrap_or_default(),
        grants: rows.into_iter().filter_map(grant).collect(),
    })
}

/// The command's name, version and input (a role outside viewer, commenter,
/// editor and manager does not parse: ownership is not given by sharing).
fn input<T: DeserializeOwned>(
    envelope: &CommandEnvelope,
    name: &str,
    version: u32,
) -> AppResult<T> {
    if envelope.command_name != name {
        return Err(AppError::invalid(format!(
            "Bilinmeyen komut: {}",
            envelope.command_name
        )));
    }
    if envelope.version != version {
        return Err(AppError::invalid(format!(
            "{name} komutunun {} sürümü desteklenmiyor (desteklenen: {version}).",
            envelope.version
        )));
    }
    serde_json::from_value(envelope.input.clone())
        .map_err(|e| AppError::invalid(format!("Komut girdisi okunamadı: {e}")))
}

fn person(text: &str) -> AppResult<Uuid> {
    Uuid::parse_str(text)
        .map_err(|_| AppError::invalid(format!("userId bir hesap kimliği (UUID) olmalı: {text}")))
}

/// What the checks under the lock decided.
enum Prepared {
    /// Go on, with the caller's access as it is now.
    Go(ProjectAccess),
    /// A retry: its stored answer.
    Replay(ProjectAccessChange),
}

/// The checks both commands make under the lock, before changing anything:
/// the stored answer of a retry, a deleted project, the caller's right, and
/// whose grant this may be.
async fn prepare(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
    user: Uuid,
) -> AppResult<Prepared> {
    let now = lock(tx, access).await?;
    if let Some(mut earlier) =
        idempotency::earlier::<ProjectAccessChange>(tx, &now, envelope, text).await?
    {
        earlier.replayed = true;
        return Ok(Prepared::Replay(earlier));
    }
    if now.deleted {
        return Err(gone(&now.name));
    }
    now.require(ProjectPermission::Share)?;
    if user == now.actor.user_id {
        return Err(AppError::invalid(
            "Kendi erişiminizi paylaşımla değiştiremezsiniz; proje sahibine ya da başka bir yöneticiye başvurun.",
        ));
    }
    let owner: Uuid = sqlx::query_scalar(
        "select owner_user_id from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(now.tenant)
    .bind(now.project)
    .fetch_one(&mut **tx)
    .await?;
    if user == owner {
        return Err(AppError::invalid(
            "Proje sahibinin erişimi paylaşımla değişmez; sahiplik ayrı bir komutla devredilir.",
        ));
    }
    Ok(Prepared::Go(now))
}

/// Writes the audit record and the `project.access` event of a change; returns the event's cursor.
async fn announce(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: &CommandEnvelope,
    detail: serde_json::Value,
) -> AppResult<i64> {
    let revision: i64 = sqlx::query_scalar(
        "select data_revision from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_one(&mut **tx)
    .await?;
    let actor = access.actor.user_id;
    sqlx::query(
        "insert into kentos.audit_event (tenant_id, project_id, actor, action, request_id, data_revision, detail)
         values ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(actor)
    .bind(&envelope.command_name)
    .bind(&envelope.request_id)
    .bind(revision)
    .bind(detail)
    .execute(&mut **tx)
    .await?;
    // No objects and no names: open connections only learn that access changed and ask again.
    let event = EventRecord {
        seq: String::new(),
        data_revision: revision.to_string(),
        kind: PROJECT_ACCESS_CHANGED.into(),
        actor: Some(actor.to_string()),
        request_id: Some(envelope.request_id.clone()),
        features: Vec::new(),
        meta: false,
    };
    Ok(sqlx::query_scalar(
        "insert into kentos.outbox_event (tenant_id, project_id, data_revision, kind, payload) values ($1, $2, $3, $4, $5) returning seq",
    )
    .bind(access.tenant)
    .bind(access.project)
    .bind(revision)
    .bind(PROJECT_ACCESS_CHANGED)
    .bind(serde_json::to_value(&event).expect("event serializes"))
    .fetch_one(&mut **tx)
    .await?)
}

/// `project.share` v1.
pub async fn share(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectAccessChange> {
    let ProjectShare {
        user_id,
        role,
        expires_at,
    } = input(&envelope, PROJECT_SHARE, PROJECT_SHARE_VERSION)?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let user = person(&user_id)?;
    let expires = expires_at
        .as_deref()
        .map(|t| {
            OffsetDateTime::parse(t, &Rfc3339)
                // Stored to the microsecond, like every timestamptz.
                .map(|t| t - time::Duration::nanoseconds(i64::from(t.nanosecond() % 1000)))
                .map_err(|_| {
                    AppError::invalid(format!("expiresAt bir RFC 3339 zamanı olmalı: {t}"))
                })
        })
        .transpose()?;
    if expires.is_some_and(|t| t <= OffsetDateTime::now_utc()) {
        return Err(AppError::invalid(
            "Paylaşımın bitiş zamanı gelecekte olmalı.",
        ));
    }
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let now = match prepare(&mut tx, access, &envelope, &text, user).await? {
        Prepared::Go(now) => now,
        Prepared::Replay(earlier) => {
            tx.commit().await?;
            return Ok(earlier);
        }
    };
    if now.tenant_kind == TenantKind::Organization {
        let member: bool = sqlx::query_scalar(
            "select exists (select 1 from kentos.membership where tenant_id = $1 and user_id = $2)",
        )
        .bind(now.tenant)
        .bind(user)
        .fetch_one(&mut *tx)
        .await?;
        if !member {
            return Err(AppError::invalid(format!(
                "Bu kişi “{}” kurumunun üyesi değil. Kurum projeleri yalnız kurum üyeleriyle paylaşılır; kurum dışından paylaşım henüz yok.",
                now.tenant_name
            )));
        }
    }
    let before: Option<(String, Option<OffsetDateTime>)> = sqlx::query_as(
        "select role, expires_at from kentos.project_grant where tenant_id = $1 and project_id = $2 and user_id = $3",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(user)
    .fetch_optional(&mut *tx)
    .await?;
    let same = before
        .as_ref()
        .is_some_and(|(r, e)| r == role.name() && *e == expires);
    let mut event_seq = None;
    if !same {
        let written = sqlx::query(
            "insert into kentos.project_grant (tenant_id, project_id, user_id, role, expires_at, granted_by)
             values ($1, $2, $3, $4, $5, $6)
             on conflict (tenant_id, project_id, user_id) do update set
               role = excluded.role, expires_at = excluded.expires_at, granted_by = excluded.granted_by, updated_at = now()",
        )
        .bind(now.tenant)
        .bind(now.project)
        .bind(user)
        .bind(role.name())
        .bind(expires)
        .bind(now.actor.user_id)
        .execute(&mut *tx)
        .await;
        match written {
            Ok(_) => {}
            // No such account (the foreign key): nothing was written.
            Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23503") => {
                return Err(AppError::not_found("Bu kimlikte bir hesap bulunamadı."));
            }
            Err(e) => return Err(e.into()),
        }
        let seq = announce(
            &mut tx,
            &now,
            &envelope,
            serde_json::json!({
                "userId": user,
                "role": role.name(),
                "previousRole": before.as_ref().map(|(r, _)| r.clone()),
                "expiresAt": expires.map(rfc3339),
            }),
        )
        .await?;
        event_seq = Some(seq.to_string());
    }
    let row: Option<GrantRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{GRANT_SELECT} and g.user_id = $3"
    )))
    .bind(now.tenant)
    .bind(now.project)
    .bind(user)
    .fetch_optional(&mut *tx)
    .await?;
    let result = ProjectAccessChange {
        user_id: user.to_string(),
        grant: row.and_then(grant),
        changed: !same,
        event_seq,
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &result).await?;
    tx.commit().await?;
    Ok(result)
}

/// `project.access.revoke` v1.
pub async fn revoke(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<ProjectAccessChange> {
    let ProjectAccessRevoke { user_id } = input(
        &envelope,
        PROJECT_ACCESS_REVOKE,
        PROJECT_ACCESS_REVOKE_VERSION,
    )?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let user = person(&user_id)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let now = match prepare(&mut tx, access, &envelope, &text, user).await? {
        Prepared::Go(now) => now,
        Prepared::Replay(earlier) => {
            tx.commit().await?;
            return Ok(earlier);
        }
    };
    let removed: Option<String> = sqlx::query_scalar(
        "delete from kentos.project_grant where tenant_id = $1 and project_id = $2 and user_id = $3 returning role",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(user)
    .fetch_optional(&mut *tx)
    .await?;
    let event_seq = match &removed {
        Some(role) => Some(
            announce(
                &mut tx,
                &now,
                &envelope,
                serde_json::json!({ "userId": user, "previousRole": role }),
            )
            .await?
            .to_string(),
        ),
        None => None,
    };
    let result = ProjectAccessChange {
        user_id: user.to_string(),
        grant: None,
        changed: removed.is_some(),
        event_seq,
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &result).await?;
    tx.commit().await?;
    Ok(result)
}
