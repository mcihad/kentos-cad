//! Tenants, membership and what a member may do (CLAUDE.md §16). Access to a
//! tenant needs an active membership, an allocated seat and an active
//! tenant; every tenant-bound use case starts from [`access`], which checks
//! all three under the tenant's row-level security scope.

use kentos_contracts::{MembershipView, TenantRole};
use kentos_postgres::{Db, Scope};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::identity::Actor;

/// Fine-grained rights, checked by each use case (never inferred from the UI).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    ProjectRead,
    ProjectCreate,
    /// Name, settings, layer tree and styles of a project.
    ProjectEdit,
    FeatureWrite,
    /// Deleting a project for everyone (soft: the operator can restore it).
    ProjectDelete,
    MemberManage,
}

impl Capability {
    pub const ALL: [Capability; 6] = [
        Self::ProjectRead,
        Self::ProjectCreate,
        Self::ProjectEdit,
        Self::FeatureWrite,
        Self::ProjectDelete,
        Self::MemberManage,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::ProjectRead => "project.read",
            Self::ProjectCreate => "project.create",
            Self::ProjectEdit => "project.edit",
            Self::FeatureWrite => "feature.write",
            Self::ProjectDelete => "project.delete",
            Self::MemberManage => "member.manage",
        }
    }
}

/// The rights of a role. Roles are ordered, each has the rights of the ones below it.
pub fn allows(role: TenantRole, cap: Capability) -> bool {
    use Capability::*;
    let needed = match cap {
        ProjectRead => TenantRole::Viewer,
        FeatureWrite => TenantRole::Editor,
        ProjectCreate | ProjectEdit => TenantRole::ProjectManager,
        // Deleting hides the project from everyone in the tenant: above the one who manages projects.
        ProjectDelete | MemberManage => TenantRole::Admin,
    };
    role >= needed
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
}

impl Access {
    pub fn scope(&self) -> Scope {
        Scope {
            tenant: Some(self.tenant),
            user: Some(self.actor.user_id),
        }
    }

    /// Refuses with a message naming the missing right.
    pub fn require(&self, cap: Capability) -> AppResult<()> {
        if allows(self.role, cap) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "Bu işlem için yetkiniz yok ({}); kurum yöneticinizden isteyin.",
                cap.name()
            )))
        }
    }
}

type MembershipRow = (Uuid, String, String, String, String, String, bool);

const MEMBERSHIP_SELECT: &str = "select t.id, t.slug, t.name, m.role, m.status, t.status,
        exists (select 1 from kentos.seat_allocation s where s.tenant_id = m.tenant_id and s.user_id = m.user_id)
   from kentos.membership m join kentos.tenant t on t.id = m.tenant_id";

fn view(
    (id, slug, name, role, status, tenant_status, seat): MembershipRow,
) -> Option<MembershipView> {
    let role = role_from_db(&role)?;
    let active = status == "active" && tenant_status == "active";
    let capabilities = if active && seat {
        Capability::ALL
            .iter()
            .filter(|c| allows(role, **c))
            .map(|c| c.name().to_string())
            .collect()
    } else {
        Vec::new()
    };
    Some(MembershipView {
        tenant_id: id.to_string(),
        tenant_slug: slug,
        tenant_name: name,
        role,
        seat,
        active,
        capabilities,
    })
}

/// Every tenant the actor belongs to (for `/v1/me`).
pub async fn memberships(db: &Db, actor: &Actor) -> AppResult<Vec<MembershipView>> {
    let mut tx = db
        .scoped(Scope {
            tenant: None,
            user: Some(actor.user_id),
        })
        .await?;
    let rows: Vec<MembershipRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{MEMBERSHIP_SELECT} where m.user_id = $1 order by t.name"
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
    if !view.active {
        return Err(AppError::forbidden(format!(
            "“{}” kurumundaki üyeliğiniz etkin değil; kurum yöneticinize başvurun.",
            view.tenant_name
        )));
    }
    if !view.seat {
        return Err(AppError::forbidden(format!(
            "“{}” kurumunda size koltuk ayrılmamış; kurum yöneticinize başvurun.",
            view.tenant_name
        )));
    }
    Ok(Access {
        actor: actor.clone(),
        tenant,
        role: view.role,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_role_has_the_rights_of_the_ones_below() {
        use Capability::*;
        assert!(
            allows(TenantRole::Viewer, ProjectRead) && !allows(TenantRole::Viewer, FeatureWrite)
        );
        assert!(
            allows(TenantRole::Editor, FeatureWrite) && !allows(TenantRole::Editor, ProjectEdit)
        );
        assert!(
            allows(TenantRole::ProjectManager, ProjectCreate)
                && !allows(TenantRole::ProjectManager, MemberManage)
        );
        assert!(
            allows(TenantRole::Admin, ProjectDelete)
                && !allows(TenantRole::ProjectManager, ProjectDelete)
        );
        assert!(
            Capability::ALL
                .iter()
                .all(|c| allows(TenantRole::Owner, *c))
        );
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
