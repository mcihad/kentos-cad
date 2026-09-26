//! Who may do what in a project (docs/adr/0015, TODOS.md CLOUD-09..13). Every
//! use case that touches a project starts from a [`ProjectAccess`], and only
//! [`project`] (or [`evaluate`] inside a transaction) makes one:
//!
//! - The caller's role is the highest of ownership, a grant and the
//!   organisation's policy. `kentos.project_role` works it out in the
//!   database, the same function row-level security uses, so the two cannot
//!   disagree about who sees a project.
//! - [`permissions`] turns the role into the fixed permission names
//!   (`project.read`, `feature.write` …) under the tenant's policies.
//! - A project the caller has no role in answers 404, word for word as one
//!   that does not exist ([`not_found`]), after the same database work.
//! - Nothing outlives a request: every request, every write under the
//!   project's lock and every flush of a live connection asks again.

use kentos_contracts::{
    AccessSource, ProjectAccessView, ProjectPermission, ProjectRole, ProjectState, TenantKind,
};
use kentos_postgres::{Db, Scope};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::tenancy;

/// The permissions a role gives by itself; each role has the ones of the roles before it.
pub fn role_permissions(role: ProjectRole) -> &'static [ProjectPermission] {
    use ProjectPermission::*;
    match role {
        ProjectRole::Viewer => &[Read, Download, History],
        ProjectRole::Commenter => &[Read, Download, History, Comment],
        ProjectRole::Editor => &[Read, Download, History, Comment, FeatureWrite, JobsRun],
        ProjectRole::Manager => &[
            Read,
            Download,
            History,
            Comment,
            FeatureWrite,
            JobsRun,
            Edit,
            Share,
        ],
        ProjectRole::Owner => &[
            Read,
            Download,
            History,
            Comment,
            FeatureWrite,
            JobsRun,
            Edit,
            Share,
            Delete,
            Transfer,
        ],
    }
}

/// What a role allows, coming from `via`, under the tenant's `viewer_download`
/// policy: the policy's reach is a manager's rights and deleting; without
/// `viewer_download`, viewers and commenters do not download.
pub fn permissions(
    role: ProjectRole,
    via: AccessSource,
    viewer_download: bool,
) -> Vec<ProjectPermission> {
    let mut set = role_permissions(role).to_vec();
    if via == AccessSource::Policy {
        set.push(ProjectPermission::Delete);
    }
    if !viewer_download && role <= ProjectRole::Commenter {
        set.retain(|p| *p != ProjectPermission::Download);
    }
    set.sort_unstable();
    set.dedup();
    set
}

/// The role `kentos.project_role` names, and where it comes from.
fn role_of(text: &str) -> Option<(ProjectRole, AccessSource)> {
    match text {
        "owner" => Some((ProjectRole::Owner, AccessSource::Owner)),
        "policy" => Some((ProjectRole::Manager, AccessSource::Policy)),
        other => ProjectRole::from_name(other)
            .filter(|r| *r != ProjectRole::Owner)
            .map(|r| (r, AccessSource::Grant)),
    }
}

/// The answer for a project the caller may not see, the same as for one that does not exist.
pub fn not_found() -> AppError {
    AppError::not_found("Proje bulunamadı.")
}

/// The refusal of a change to an archived project, the same wherever it is asked for.
pub fn archived(name: &str) -> AppError {
    AppError::archived(format!(
        "“{name}” projesi arşivlenmiş; salt okunurdur. Değiştirmek için proje sahibi ya da yöneticisi onu arşivden çıkarmalı; dilerseniz kopyasını oluşturup kopyada çalışın."
    ))
}

pub fn tenant_kind(text: &str) -> TenantKind {
    if text == "personal" {
        TenantKind::Personal
    } else {
        TenantKind::Organization
    }
}

/// A verified actor's access to one project, for the length of one request.
#[derive(Clone, Debug)]
pub struct ProjectAccess {
    pub actor: Actor,
    pub tenant: Uuid,
    pub project: Uuid,
    /// The project's name (the caller may see it).
    pub name: String,
    /// Deleted projects (in the trash) are kept; opening and writing answer 410, the event log stays readable.
    pub deleted: bool,
    /// Archived projects open read-only: writing answers 409 (docs/adr/0028).
    pub archived: bool,
    pub tenant_name: String,
    pub tenant_kind: TenantKind,
    pub role: ProjectRole,
    pub via: AccessSource,
    permissions: Vec<ProjectPermission>,
}

impl ProjectAccess {
    /// This tenant, user and project: row-level security shows the project's rows and nothing else.
    pub fn scope(&self) -> Scope {
        Scope {
            tenant: Some(self.tenant),
            user: Some(self.actor.user_id),
            project: Some(self.project),
        }
    }

    pub fn allows(&self, permission: ProjectPermission) -> bool {
        self.permissions.contains(&permission)
    }

    pub fn permissions(&self) -> &[ProjectPermission] {
        &self.permissions
    }

    /// Refuses with a message naming the missing permission (the caller may see the project, so 403).
    pub fn require(&self, permission: ProjectPermission) -> AppResult<()> {
        if self.allows(permission) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "“{}” projesinde bu işlem için yetkiniz yok ({}); proje sahibinden ya da yöneticisinden isteyin.",
                self.name,
                permission.name()
            )))
        }
    }

    /// Refuses a deleted project (410).
    pub fn live(&self) -> AppResult<()> {
        if self.deleted {
            Err(crate::projects::gone(&self.name))
        } else {
            Ok(())
        }
    }

    /// Refuses a deleted project (410) and an archived one (409): what changes a project's content or catalog metadata.
    pub fn writable(&self) -> AppResult<()> {
        self.live()?;
        if self.archived {
            Err(archived(&self.name))
        } else {
            Ok(())
        }
    }

    /// Where the project is in its life, as the caller sees it.
    pub fn state(&self) -> ProjectState {
        if self.deleted {
            ProjectState::Trashed
        } else if self.archived {
            ProjectState::Archived
        } else {
            ProjectState::Active
        }
    }

    pub fn view(&self) -> ProjectAccessView {
        ProjectAccessView {
            role: self.role,
            via: self.via,
            permissions: self.permissions.clone(),
        }
    }
}

/// The caller's access to `project` in `tenant`: 404 when they have no role in
/// it (or it does not exist), 403 when they are a member of the tenant who
/// may not use it now (the same for every project id: it says nothing about
/// the project).
pub async fn project(
    db: &Db,
    actor: &Actor,
    tenant: Uuid,
    project: Uuid,
) -> AppResult<ProjectAccess> {
    let mut tx = db
        .scoped(Scope {
            tenant: Some(tenant),
            user: Some(actor.user_id),
            project: Some(project),
        })
        .await?;
    let access = evaluate(&mut tx, actor, tenant, project).await?;
    tx.commit().await?;
    Ok(access)
}

/// [`project`] inside a transaction already scoped to it: a write asks again
/// under the project's lock, so a grant taken away before it cannot let it in.
pub async fn evaluate(
    tx: &mut Transaction<'static, Postgres>,
    actor: &Actor,
    tenant: Uuid,
    project: Uuid,
) -> AppResult<ProjectAccess> {
    tenancy::check_member(tx, actor, tenant).await?;
    let row: Option<(String, String, bool, bool, String, String, bool)> = sqlx::query_as(
        "select role, name, deleted, archived, tenant_name, tenant_kind, viewer_download from kentos.project_access($1, $2)",
    )
    .bind(tenant)
    .bind(project)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((role, name, deleted, archived, tenant_name, kind, viewer_download)) = row else {
        return Err(not_found());
    };
    let (role, via) = role_of(&role).ok_or_else(not_found)?;
    Ok(ProjectAccess {
        actor: actor.clone(),
        tenant,
        project,
        name,
        deleted,
        archived,
        tenant_name,
        tenant_kind: tenant_kind(&kind),
        role,
        via,
        permissions: permissions(role, via, viewer_download),
    })
}

/// Asks again for the same caller and project inside `tx` (see [`evaluate`]).
pub async fn recheck(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
) -> AppResult<ProjectAccess> {
    evaluate(tx, &access.actor, access.tenant, access.project).await
}

/// The access view of a project's row read under row-level security (lists).
pub(crate) fn view_of(role: &str, viewer_download: bool) -> Option<ProjectAccessView> {
    let (role, via) = role_of(role)?;
    Some(ProjectAccessView {
        role,
        via,
        permissions: permissions(role, via, viewer_download),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ProjectPermission::*;

    #[test]
    fn each_role_has_the_permissions_of_the_ones_before_it() {
        let roles = [
            ProjectRole::Viewer,
            ProjectRole::Commenter,
            ProjectRole::Editor,
            ProjectRole::Manager,
            ProjectRole::Owner,
        ];
        for pair in roles.windows(2) {
            let (lower, higher) = (role_permissions(pair[0]), role_permissions(pair[1]));
            assert!(lower.iter().all(|p| higher.contains(p)), "{pair:?}");
            assert!(higher.len() > lower.len(), "{pair:?}");
        }
        // The table of docs/adr/0015.
        assert_eq!(
            role_permissions(ProjectRole::Viewer),
            &[Read, Download, History]
        );
        assert!(role_permissions(ProjectRole::Commenter).contains(&Comment));
        assert!(
            role_permissions(ProjectRole::Editor).contains(&FeatureWrite)
                && role_permissions(ProjectRole::Editor).contains(&JobsRun)
                && !role_permissions(ProjectRole::Editor).contains(&Edit)
        );
        assert!(
            role_permissions(ProjectRole::Manager).contains(&Share)
                && !role_permissions(ProjectRole::Manager).contains(&Delete)
        );
        assert_eq!(
            role_permissions(ProjectRole::Owner).len(),
            ProjectPermission::ALL.len()
        );
    }

    #[test]
    fn the_policy_is_a_manager_who_may_delete_and_viewers_may_lose_downloads() {
        let policy = permissions(ProjectRole::Manager, AccessSource::Policy, true);
        assert!(policy.contains(&Delete) && policy.contains(&Share) && !policy.contains(&Transfer));
        let granted = permissions(ProjectRole::Manager, AccessSource::Grant, true);
        assert!(!granted.contains(&Delete));
        assert!(!permissions(ProjectRole::Viewer, AccessSource::Grant, false).contains(&Download));
        assert!(
            !permissions(ProjectRole::Commenter, AccessSource::Grant, false).contains(&Download)
        );
        // Editors and up keep it: the policy is about viewing roles only.
        assert!(permissions(ProjectRole::Editor, AccessSource::Grant, false).contains(&Download));
    }

    #[test]
    fn the_database_role_names_map_to_roles_and_sources() {
        assert_eq!(
            role_of("owner"),
            Some((ProjectRole::Owner, AccessSource::Owner))
        );
        assert_eq!(
            role_of("policy"),
            Some((ProjectRole::Manager, AccessSource::Policy))
        );
        assert_eq!(
            role_of("editor"),
            Some((ProjectRole::Editor, AccessSource::Grant))
        );
        assert_eq!(
            role_of("viewer"),
            Some((ProjectRole::Viewer, AccessSource::Grant))
        );
        // A grant never makes an owner; unknown text is no role.
        assert_eq!(role_of("admin"), None);
        assert_eq!(role_of(""), None);
    }
}
