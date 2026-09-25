//! Tenants, membership and what a member may do in the tenant itself
//! (CLAUDE.md §16, docs/adr/0015). A tenant is an organisation or a person's
//! personal space ([`ensure_personal`] opens it). Tenant-level use cases
//! (listing a tenant's projects, creating one) start from [`access`]: an
//! active membership, an allocated seat and an active tenant. A tenant role
//! opens no project by itself: what a person may do in a project is that
//! project's (`access.rs`).

use kentos_contracts::{MembershipView, TenantKind, TenantRole};
use kentos_postgres::{Db, Scope};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::tenant_kind;
use crate::error::{AppError, AppResult};
use crate::identity::Actor;

/// Rights in the tenant itself (never inferred from the UI). Rights in a
/// project are `kentos_contracts::ProjectPermission`s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    /// Opening a new project (its creator becomes its owner).
    ProjectCreate,
    MemberManage,
}

impl Capability {
    pub const ALL: [Capability; 2] = [Self::ProjectCreate, Self::MemberManage];

    pub fn name(self) -> &'static str {
        match self {
            Self::ProjectCreate => "project.create",
            Self::MemberManage => "member.manage",
        }
    }
}

/// The rights of a role in a tenant of this kind. Roles are ordered; a
/// personal space's only member (its owner) creates projects there, and no
/// one manages its members (others come through sharing).
pub fn allows(role: TenantRole, kind: TenantKind, cap: Capability) -> bool {
    match cap {
        Capability::ProjectCreate => role >= TenantRole::ProjectManager,
        Capability::MemberManage => kind == TenantKind::Organization && role >= TenantRole::Admin,
    }
}

pub fn role_from_db(text: &str) -> Option<TenantRole> {
    Some(match text {
        "owner" => TenantRole::Owner,
        "admin" => TenantRole::Admin,
        "project_manager" => TenantRole::ProjectManager,
        "editor" => TenantRole::Editor,
        "viewer" => TenantRole::Viewer,
        _ => return None,
    })
}

pub fn role_to_db(role: TenantRole) -> &'static str {
    match role {
        TenantRole::Owner => "owner",
        TenantRole::Admin => "admin",
        TenantRole::ProjectManager => "project_manager",
        TenantRole::Editor => "editor",
        TenantRole::Viewer => "viewer",
    }
}

/// A verified actor inside one tenant.
#[derive(Clone, Debug)]
pub struct Access {
    pub actor: Actor,
    pub tenant: Uuid,
    pub role: TenantRole,
    pub kind: TenantKind,
}

impl Access {
    /// The tenant and user, no project: row-level security shows the tenant's projects the user has a role in.
    pub fn scope(&self) -> Scope {
        Scope {
            tenant: Some(self.tenant),
            user: Some(self.actor.user_id),
            project: None,
        }
    }

    /// Refuses with a message naming the missing right.
    pub fn require(&self, cap: Capability) -> AppResult<()> {
        if allows(self.role, self.kind, cap) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "Bu işlem için yetkiniz yok ({}); kurum yöneticinizden isteyin.",
                cap.name()
            )))
        }
    }
}

type MembershipRow = (Uuid, String, String, String, String, String, bool, String);

const MEMBERSHIP_SELECT: &str = "select t.id, t.slug, t.name, m.role, m.status, t.status,
        exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id),
        t.kind
   from kentos.membership m join kentos.tenant t on t.id = m.tenant_id";

fn view(
    (id, slug, name, role, status, tenant_status, seat, kind): MembershipRow,
) -> Option<MembershipView> {
    let role = role_from_db(&role)?;
    let kind = tenant_kind(&kind);
    let active = status == "active" && tenant_status == "active";
    let capabilities = if active && seat {
        Capability::ALL
            .iter()
            .filter(|c| allows(role, kind, **c))
            .map(|c| c.name().to_string())
            .collect()
    } else {
        Vec::new()
    };
    Some(MembershipView {
        tenant_id: id.to_string(),
        tenant_slug: slug,
        tenant_name: name,
        tenant_kind: kind,
        role,
        seat,
        active,
        capabilities,
    })
}

/// A membership that may not be used now, with the reason and what to do.
fn unusable(view: &MembershipView) -> Option<AppError> {
    if !view.active {
        return Some(AppError::forbidden(format!(
            "“{}” kurumundaki üyeliğiniz etkin değil; kurum yöneticinize başvurun.",
            view.tenant_name
        )));
    }
    if !view.seat {
        return Some(AppError::forbidden(format!(
            "“{}” kurumunda size koltuk ayrılmamış; kurum yöneticinize başvurun.",
            view.tenant_name
        )));
    }
    None
}

/// Every tenant the actor belongs to (for `/v1/me`): organisations by name, then the personal space.
pub async fn memberships(db: &Db, actor: &Actor) -> AppResult<Vec<MembershipView>> {
    let mut tx = db
        .scoped(Scope {
            tenant: None,
            user: Some(actor.user_id),
            project: None,
        })
        .await?;
    let rows: Vec<MembershipRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{MEMBERSHIP_SELECT} where m.user_id = $1 order by t.kind = 'personal', t.name"
    )))
    .bind(actor.user_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows.into_iter().filter_map(view).collect())
}

/// The actor's access to `tenant`. A tenant the actor does not belong to is
/// "not found", not "forbidden": its existence is not revealed.
pub async fn access(db: &Db, actor: &Actor, tenant: Uuid) -> AppResult<Access> {
    let mut tx = db
        .scoped(Scope {
            tenant: Some(tenant),
            user: Some(actor.user_id),
            project: None,
        })
        .await?;
    let row: Option<MembershipRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{MEMBERSHIP_SELECT} where m.tenant_id = $1 and m.user_id = $2"
    )))
    .bind(tenant)
    .bind(actor.user_id)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let Some(view) = row.and_then(view) else {
        return Err(AppError::not_found(
            "Kurum bulunamadı ya da üyesi değilsiniz.",
        ));
    };
    if let Some(refusal) = unusable(&view) {
        return Err(refusal);
    }
    Ok(Access {
        actor: actor.clone(),
        tenant,
        role: view.role,
        kind: view.tenant_kind,
    })
}

/// Inside a transaction scoped to `tenant`: refuses a member of it who may
/// not use it now (membership off, no seat, tenant suspended). A non-member
/// passes: they may still hold a grant in a personal space, which the
/// project's own check decides.
pub(crate) async fn check_member(
    tx: &mut Transaction<'static, Postgres>,
    actor: &Actor,
    tenant: Uuid,
) -> AppResult<()> {
    let row: Option<MembershipRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{MEMBERSHIP_SELECT} where m.tenant_id = $1 and m.user_id = $2"
    )))
    .bind(tenant)
    .bind(actor.user_id)
    .fetch_optional(&mut **tx)
    .await?;
    match row.and_then(view).as_ref().and_then(unusable) {
        Some(refusal) => Err(refusal),
        None => Ok(()),
    }
}

/// The actor's personal space, opened now if this is the first time
/// (`kentos.ensure_personal_tenant`: safe to call again, and at once from
/// several requests). Its name is the person's name; the interface says “Kişisel”.
pub async fn ensure_personal(db: &Db, actor: &Actor) -> AppResult<Uuid> {
    let mut tx = db
        .scoped(Scope {
            tenant: None,
            user: Some(actor.user_id),
            project: None,
        })
        .await?;
    let name = actor.display_name.trim();
    let id: Uuid = sqlx::query_scalar("select kentos.ensure_personal_tenant($1, $2)")
        .bind(Uuid::now_v7())
        .bind(if name.is_empty() { "Kişisel" } else { name })
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_rights_follow_the_role_and_the_kind() {
        use Capability::*;
        let org = TenantKind::Organization;
        assert!(!allows(TenantRole::Editor, org, ProjectCreate));
        assert!(allows(TenantRole::ProjectManager, org, ProjectCreate));
        assert!(!allows(TenantRole::ProjectManager, org, MemberManage));
        assert!(allows(TenantRole::Admin, org, MemberManage));
        assert!(
            Capability::ALL
                .iter()
                .all(|c| allows(TenantRole::Owner, org, *c))
        );
        // A personal space: its owner creates projects; nobody manages members there.
        assert!(allows(
            TenantRole::Owner,
            TenantKind::Personal,
            ProjectCreate
        ));
        assert!(!allows(
            TenantRole::Owner,
            TenantKind::Personal,
            MemberManage
        ));
        for r in [
            TenantRole::Owner,
            TenantRole::Admin,
            TenantRole::ProjectManager,
            TenantRole::Editor,
            TenantRole::Viewer,
        ] {
            assert_eq!(role_from_db(role_to_db(r)), Some(r));
        }
        assert_eq!(role_from_db("root"), None);
    }
}
