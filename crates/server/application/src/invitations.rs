//! Invitations to a project by link, and guests (docs/adr/0035; TODOS.md
//! CLOUD-16, CLOUD-17). The owner's decision of 26 September: the inviter
//! sends the link themselves (no mail server); only the account with the
//! invitation's verified e-mail accepts it; someone outside the project's
//! organisation becomes a guest, who reaches only the projects shared with
//! them while the organisation takes guests.
//!
//! - `project.invite` v1 (`project.share`): an invitation for one e-mail,
//!   at most editor, waiting 14 days unless told otherwise (90 at most). The
//!   token is 32 random bytes; only its SHA-256 is kept, and only the first
//!   answer carries it (a retry's stored answer has none). A new invitation
//!   for the same e-mail replaces a waiting one.
//! - `project.invitation.revoke` v1 (`project.share`): a waiting one is withdrawn.
//! - [`list`] (`project.share`): waiting invitations and those of the last 30 days.
//! - [`accept`]: the signed-in account, which has no role in the project
//!   yet; `kentos.accept_invitation` does what only the owner role may (it
//!   finds the invitation by its token and gives the grant). Then the
//!   project's audit and its open connections hear of it.

use kentos_contracts::{
    CommandEnvelope, GrantRole, INVITATION_DAYS, INVITATION_MAX_DAYS, InvitationAccept,
    InvitationAccepted, InvitationChange, InvitationRevoke, InvitationState,
    PROJECT_ACCESS_CHANGED, PROJECT_INVITATION_REVOKE, PROJECT_INVITATION_REVOKE_VERSION,
    PROJECT_INVITE, PROJECT_INVITE_VERSION, ProjectInvitation, ProjectInvitations, ProjectInvite,
    ProjectPermission, ProjectRole, TenantKind,
};
use kentos_postgres::{Db, Scope, rescope};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::changes::{check_target, lock};
use crate::commands::input;
use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::projects::{gone, rfc3339};
use crate::{idempotency, journal};

/// An invitation's row: id, e-mail, role, state, end, inviter and their
/// name, when it was made, who accepted it (by name) and when.
type InvitationRow = (
    Uuid,
    String,
    String,
    String,
    OffsetDateTime,
    Uuid,
    Option<String>,
    OffsetDateTime,
    Option<String>,
    Option<OffsetDateTime>,
);

const INVITATION_SELECT: &str = "select i.id, i.email, i.role, i.state, i.expires_at, i.created_by, cu.display_name, i.created_at, au.display_name, i.accepted_at
   from kentos.project_invitation i
   left join kentos.app_user cu on cu.id = i.created_by
   left join kentos.app_user au on au.id = i.accepted_by
  where i.tenant_id = $1 and i.project_id = $2";

fn invitation_of(row: InvitationRow) -> ProjectInvitation {
    let (id, email, role, state, expires, by, by_name, created, accepted_by, accepted_at) = row;
    let state = match state.as_str() {
        "accepted" => InvitationState::Accepted,
        "revoked" => InvitationState::Revoked,
        _ if expires <= OffsetDateTime::now_utc() => InvitationState::Expired,
        _ => InvitationState::Pending,
    };
    ProjectInvitation {
        id: id.to_string(),
        email,
        role: GrantRole::from_name(&role).unwrap_or(GrantRole::Viewer),
        state,
        created_by: by.to_string(),
        created_by_name: by_name.unwrap_or_default(),
        created_at: rfc3339(created),
        expires_at: rfc3339(expires),
        accepted_by_name: accepted_by,
        accepted_at: accepted_at.map(rfc3339),
    }
}

/// An e-mail as an invitation keeps it: trimmed, lower case, one `@` with
/// something on both sides and a dot after it, no spaces, at most 254 bytes.
pub fn check_email(email: &str) -> AppResult<String> {
    let email = email.trim().to_lowercase();
    let ok = email.len() <= 254
        && !email.chars().any(|c| c.is_whitespace() || c.is_control())
        && match email.split_once('@') {
            Some((local, domain)) => {
                !local.is_empty()
                    && !domain.contains('@')
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.')
            }
            None => false,
        };
    if ok {
        Ok(email)
    } else {
        Err(AppError::invalid_at(
            "email",
            "Davet için geçerli bir e-posta adresi yazın (ör. ad@kurum.gov.tr).",
        ))
    }
}

fn missing() -> AppError {
    AppError::not_found("Davet bulunamadı ya da artık beklemiyor; listeyi yenileyin.")
}

/// Under the project's lock: a retry's stored answer, or the caller's access now.
async fn prepare(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
) -> AppResult<Result<ProjectAccess, InvitationChange>> {
    let now = lock(tx, access).await?;
    if let Some(mut earlier) =
        idempotency::earlier::<InvitationChange>(tx, &now, envelope, text).await?
    {
        earlier.replayed = true;
        return Ok(Err(earlier));
    }
    if now.deleted {
        return Err(gone(&now.name));
    }
    now.require(ProjectPermission::Share)?;
    Ok(Ok(now))
}

/// `project.invite` v1.
pub async fn invite(
    db: &Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<InvitationChange> {
    let ProjectInvite {
        email,
        role,
        expires_at,
    } = input(&envelope, PROJECT_INVITE, PROJECT_INVITE_VERSION)?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let email = check_email(&email)?;
    if role == GrantRole::Manager {
        return Err(AppError::invalid_at(
            "role",
            "Davetle en çok düzenleyici rolü verilir; yöneticilik kurum üyelerine paylaşımla verilir.",
        ));
    }
    let now_utc = OffsetDateTime::now_utc();
    let expires = match expires_at.as_deref() {
        None => now_utc + time::Duration::days(i64::from(INVITATION_DAYS)),
        Some(t) => OffsetDateTime::parse(t, &Rfc3339)
            // Stored to the microsecond, like every timestamptz.
            .map(|t| t - time::Duration::nanoseconds(i64::from(t.nanosecond() % 1000)))
            .map_err(|_| {
                AppError::invalid_at(
                    "expiresAt",
                    format!("expiresAt bir RFC 3339 zamanı olmalı: {t}"),
                )
            })?,
    };
    if expires <= now_utc
        || expires > now_utc + time::Duration::days(i64::from(INVITATION_MAX_DAYS))
    {
        return Err(AppError::invalid_at(
            "expiresAt",
            format!("Davetin bitişi gelecekte ve en çok {INVITATION_MAX_DAYS} gün sonra olmalı."),
        ));
    }
    access.live()?;
    access.require(ProjectPermission::Share)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let now = match prepare(&mut tx, access, &envelope, &text).await? {
        Ok(now) => now,
        Err(earlier) => {
            tx.commit().await?;
            return Ok(earlier);
        }
    };
    // One waiting invitation per e-mail: a new one replaces it (its link stops working).
    sqlx::query(
        "update kentos.project_invitation set state = 'revoked', revoked_by = $4, revoked_at = now()
          where tenant_id = $1 and project_id = $2 and email = $3 and state = 'pending'",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(&email)
    .bind(now.actor.user_id)
    .execute(&mut *tx)
    .await?;
    let token: String = sqlx::query_scalar("select encode(public.gen_random_bytes(32), 'hex')")
        .fetch_one(&mut *tx)
        .await?;
    let id = Uuid::now_v7();
    sqlx::query(
        "insert into kentos.project_invitation (tenant_id, project_id, id, email, role, token_hash, created_by, expires_at)
         values ($1, $2, $3, $4, $5, public.digest($6, 'sha256'), $7, $8)",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(id)
    .bind(&email)
    .bind(role.name())
    .bind(&token)
    .bind(now.actor.user_id)
    .bind(expires)
    .execute(&mut *tx)
    .await?;
    let row: InvitationRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{INVITATION_SELECT} and i.id = $3"
    )))
    .bind(now.tenant)
    .bind(now.project)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let request = Some(envelope.request_id.as_str());
    let revision = journal::revision(&mut tx, &now).await?;
    journal::audit(
        &mut tx,
        &now,
        PROJECT_INVITE,
        request,
        revision,
        json!({ "invitation": id, "email": email, "role": role.name(), "expiresAt": rfc3339(expires) }),
    )
    .await?;
    // The stored answer has no token: a retry does not show the link again.
    let stored = InvitationChange {
        invitation: invitation_of(row),
        token: None,
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &stored).await?;
    tx.commit().await?;
    Ok(InvitationChange {
        token: Some(token),
        ..stored
    })
}

/// `project.invitation.revoke` v1: a waiting invitation is withdrawn.
pub async fn revoke(
    db: &Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<InvitationChange> {
    let InvitationRevoke { invitation_id } = input(
        &envelope,
        PROJECT_INVITATION_REVOKE,
        PROJECT_INVITATION_REVOKE_VERSION,
    )?;
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let id = Uuid::parse_str(&invitation_id).map_err(|_| missing())?;
    access.live()?;
    access.require(ProjectPermission::Share)?;
    let text = idempotency::request_text(&envelope);
    let mut tx = db.scoped(access.scope()).await?;
    let now = match prepare(&mut tx, access, &envelope, &text).await? {
        Ok(now) => now,
        Err(earlier) => {
            tx.commit().await?;
            return Ok(earlier);
        }
    };
    let revoked = sqlx::query(
        "update kentos.project_invitation set state = 'revoked', revoked_by = $4, revoked_at = now()
          where tenant_id = $1 and project_id = $2 and id = $3 and state = 'pending'",
    )
    .bind(now.tenant)
    .bind(now.project)
    .bind(id)
    .bind(now.actor.user_id)
    .execute(&mut *tx)
    .await?;
    if revoked.rows_affected() == 0 {
        return Err(missing());
    }
    let row: InvitationRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{INVITATION_SELECT} and i.id = $3"
    )))
    .bind(now.tenant)
    .bind(now.project)
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    let request = Some(envelope.request_id.as_str());
    let revision = journal::revision(&mut tx, &now).await?;
    journal::audit(
        &mut tx,
        &now,
        PROJECT_INVITATION_REVOKE,
        request,
        revision,
        json!({ "invitation": id, "email": row.1 }),
    )
    .await?;
    let result = InvitationChange {
        invitation: invitation_of(row),
        token: None,
        replayed: false,
    };
    idempotency::record(&mut tx, &now, &envelope, &text, &result).await?;
    tx.commit().await?;
    Ok(result)
}

/// A project's invitations for those who may share it: waiting ones and
/// those of the last 30 days, newest first.
pub async fn list(db: &Db, access: &ProjectAccess) -> AppResult<ProjectInvitations> {
    access.live()?;
    access.require(ProjectPermission::Share)?;
    let mut tx = db.scoped(access.scope()).await?;
    let rows: Vec<InvitationRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{INVITATION_SELECT} and (i.state = 'pending' or coalesce(i.accepted_at, i.revoked_at, i.created_at) > now() - interval '30 days')
          order by i.created_at desc, i.id desc"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ProjectInvitations {
        invitations: rows.into_iter().map(invitation_of).collect(),
    })
}

/// What `kentos.accept_invitation` answers: the outcome, and for `ok` or
/// `owner` the project, the invitation, the role and whether it is a guest's.
type Accepted = (
    String,
    Option<Uuid>,
    Option<Uuid>,
    Option<Uuid>,
    Option<String>,
    Option<bool>,
);

/// `POST /v1/invitations/accept`: the signed-in account takes the invitation of this token.
pub async fn accept(
    db: &Db,
    actor: &Actor,
    input: InvitationAccept,
) -> AppResult<InvitationAccepted> {
    let not_found = || {
        AppError::not_found(
            "Davet bulunamadı: süresi dolmuş, kullanılmış ya da geri alınmış olabilir. Davet edenden yeni bir bağlantı isteyin.",
        )
    };
    let token = input.token.trim();
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(not_found());
    }
    let mut tx = db
        .scoped(Scope {
            user: Some(actor.user_id),
            ..Scope::default()
        })
        .await?;
    let (outcome, tenant, project, invitation, role, guest): Accepted =
        sqlx::query_as("select * from kentos.accept_invitation($1)")
            .bind(token.to_ascii_lowercase())
            .fetch_one(&mut *tx)
            .await?;
    let (Some(tenant), Some(project), Some(invitation), Some(role)) =
        (tenant, project, invitation, role)
    else {
        return Err(match outcome.as_str() {
            "wrong_email" => AppError::forbidden(
                "Bu davet başka bir e-posta adresi için. Davetin gönderildiği adresin hesabıyla giriş yapın.",
            ),
            "unverified" => AppError::forbidden(
                "Hesabınızın e-posta adresi doğrulanmamış. Adresinizi kimlik sağlayıcınızda doğrulayıp yeniden giriş yapın; yerel hesapta kurum yöneticinize başvurun.",
            ),
            "gone" => AppError::deleted(
                "Davet edilen proje silinmiş ya da kurumu etkin değil; davet edenle görüşün.",
            ),
            "inactive" => AppError::forbidden(
                "Bu kurumdaki üyeliğiniz ya da koltuğunuz etkin değil; kurum yöneticinize başvurun.",
            ),
            "guests_off" => AppError::forbidden(
                "Bu kurum dışarıdan misafir kabul etmiyor; kurum yöneticisiyle görüşün.",
            ),
            _ => not_found(),
        });
    };
    let guest = guest.unwrap_or(false);
    // The account has a role in the project now: its own rows are written in its scope.
    rescope(
        &mut tx,
        Scope {
            tenant: Some(tenant),
            user: Some(actor.user_id),
            project: Some(project),
        },
    )
    .await?;
    let subject = journal::Subject {
        tenant,
        project,
        actor: actor.user_id,
    };
    let revision = journal::revision(&mut tx, subject).await?;
    journal::audit(
        &mut tx,
        subject,
        "project.invitation.accept",
        None,
        revision,
        json!({ "invitation": invitation, "role": role, "guest": guest }),
    )
    .await?;
    if outcome == "ok" {
        // Who may use the project changed: open connections ask again.
        journal::event(
            &mut tx,
            subject,
            PROJECT_ACCESS_CHANGED,
            None,
            revision,
            false,
        )
        .await?;
    }
    let (project_name, tenant_name, kind): (String, String, String) = sqlx::query_as(
        "select p.name, t.name, t.kind from kentos.project p join kentos.tenant t on t.id = p.tenant_id
          where p.tenant_id = $1 and p.id = $2",
    )
    .bind(tenant)
    .bind(project)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(InvitationAccepted {
        tenant_id: tenant.to_string(),
        tenant_name,
        tenant_kind: if kind == "personal" {
            TenantKind::Personal
        } else {
            TenantKind::Organization
        },
        project_id: project.to_string(),
        project_name,
        role: ProjectRole::from_name(&role).unwrap_or(ProjectRole::Viewer),
        guest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e_mails_are_kept_in_lower_case_and_checked() {
        assert_eq!(
            check_email("  Ayse@Kurum.Gov.TR ").unwrap(),
            "ayse@kurum.gov.tr"
        );
        for bad in [
            "",
            "ayse",
            "@kurum.gov.tr",
            "ayse@",
            "ayse@kurum",
            "a b@kurum.tr",
            "a@b@c.tr",
            "ayse@.tr",
            "ayse@kurum.",
        ] {
            assert!(check_email(bad).is_err(), "{bad}");
        }
    }
}
