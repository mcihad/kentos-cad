//! Lists of projects (docs/adr/0015, TODOS.md CLOUD-04, CLOUD-12): a
//! tenant's, and “Projelerim” across tenants. A list holds only projects the
//! caller has a role in: row-level security leaves the others out before
//! anything is counted, and each entry carries what the caller may do in it.
//! Deleted projects are never listed.

use kentos_contracts::{ProjectList, ProjectSummary};
use kentos_postgres::Scope;
use uuid::Uuid;

use crate::access::{tenant_kind, view_of};
use crate::error::AppResult;
use crate::identity::Actor;
use crate::projects::rfc3339;
use crate::tenancy::{self, Access};

type SummaryRow = (
    Uuid,
    String,
    i32,
    i64,
    time::OffsetDateTime,
    Option<String>,
    Uuid,
    String,
    String,
    bool,
    Option<String>,
);

/// The projects of the scope's tenant the caller has a role in and that are not deleted;
/// `mine` keeps only the ones they own or that were shared with them. The
/// owner's name is there when row-level security shows the owner (a member
/// of the tenant, or a personal space's own person).
const SUMMARY_SELECT: &str = "select p.id, p.name, p.srid, p.data_revision, p.updated_at,
        kentos.project_role(p.tenant_id, p.id, p.owner_user_id), t.id, t.name, t.kind, t.viewer_download,
        u.display_name
   from kentos.project p join kentos.tenant t on t.id = p.tenant_id
   left join kentos.app_user u on u.id = p.owner_user_id
  where p.tenant_id = $1 and p.deleted_at is null";

/// Newest first.
fn summaries(mut rows: Vec<SummaryRow>) -> Vec<ProjectSummary> {
    rows.sort_by(|a, b| b.4.cmp(&a.4).then(a.0.cmp(&b.0)));
    rows.into_iter()
        .filter_map(
            |(id, name, srid, rev, at, role, tenant, tenant_name, kind, download, owner)| {
                Some(ProjectSummary {
                    id: id.to_string(),
                    name,
                    srid: srid as u32,
                    data_revision: rev.to_string(),
                    updated_at: rfc3339(at),
                    tenant_id: tenant.to_string(),
                    tenant_name,
                    tenant_kind: tenant_kind(&kind),
                    owner_name: owner.unwrap_or_default(),
                    access: view_of(role.as_deref()?, download)?,
                })
            },
        )
        .collect()
}

/// A tenant's projects the caller may see: the ones they own, the ones shared
/// with them and, for an organisation's owners and admins under its policy,
/// every one.
pub async fn list(db: &kentos_postgres::Db, access: &Access) -> AppResult<ProjectList> {
    let mut tx = db.scoped(access.scope()).await?;
    let rows: Vec<SummaryRow> = sqlx::query_as(SUMMARY_SELECT)
        .bind(access.tenant)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ProjectList {
        projects: summaries(rows),
    })
}

/// “Projelerim”: the projects the caller owns (their personal space's and
/// the organisation projects they created) and the ones shared with them, in
/// every tenant, newest first. An organisation's projects they reach only
/// through its policy are the organisation's list, not this one. Opens the
/// personal space if it is not there yet.
pub async fn mine(db: &kentos_postgres::Db, actor: &Actor) -> AppResult<ProjectList> {
    let personal = tenancy::ensure_personal(db, actor).await?;
    // Where to look: the personal space, the tenants of active memberships and of grants.
    let mut tx = db
        .scoped(Scope {
            user: Some(actor.user_id),
            ..Scope::default()
        })
        .await?;
    let mut tenants: Vec<Uuid> = sqlx::query_scalar(
        "select tenant_id from kentos.membership where user_id = $1 and status = 'active'
         union select tenant_id from kentos.project_grant where user_id = $1",
    )
    .bind(actor.user_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    if !tenants.contains(&personal) {
        tenants.push(personal);
    }
    let mut rows = Vec::new();
    for tenant in tenants {
        let mut tx = db
            .scoped(Scope {
                tenant: Some(tenant),
                user: Some(actor.user_id),
                project: None,
            })
            .await?;
        let found: Vec<SummaryRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{SUMMARY_SELECT} and (p.owner_user_id = $2 or t.owner_user_id = $2
                  or exists (select 1 from kentos.project_grant g
                              where g.tenant_id = p.tenant_id and g.project_id = p.id and g.user_id = $2
                                and (g.expires_at is null or g.expires_at > now())))"
        )))
        .bind(tenant)
        .bind(actor.user_id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        rows.extend(found);
    }
    Ok(ProjectList {
        projects: summaries(rows),
    })
}
