//! Lists of dossiers and the overviews' figures (docs/adr/9000 §5): an organisation's and a professional's, filtered by
//! direction, status, a search, what is assigned to the caller and what waits for the caller's side; a page of them
//! with how many there are, newest change first.

use kentos_contracts::{
    DossierDashboard, DossierDirection, DossierList, DossierSide, DossierStatus, StatusCount,
};
use kentos_postgres::Db;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::accounts::user_scope;
use super::catalog;
use super::rows::{SUMMARY_SELECT, SummaryRow};
use super::today;
use crate::error::AppResult;
use crate::identity::Actor;
use crate::tenancy::Access;

/// What a list asks for.
#[derive(Clone, Debug, Default)]
pub struct ListFilter {
    pub direction: Option<DossierDirection>,
    pub statuses: Vec<DossierStatus>,
    pub query: Option<String>,
    /// Assigned to the caller (an organisation's list).
    pub mine: bool,
    /// Waiting for the caller's side.
    pub awaiting: bool,
    pub limit: u32,
    pub offset: u32,
}

/// The statuses each side is waited for in, as SQL over `d`.
const AWAITING_MUNICIPALITY: &str =
    "((d.direction = 'application' and d.status in ('submitted', 'in_review'))
     or (d.direction = 'transfer' and d.status in ('draft', 'delivered')))";
const AWAITING_PROFESSIONAL: &str =
    "((d.direction = 'application' and d.status in ('draft', 'changes_requested'))
     or (d.direction = 'transfer' and d.status in ('sent', 'accepted', 'revision_requested')))";

#[derive(sqlx::FromRow)]
struct Listed {
    #[sqlx(flatten)]
    row: SummaryRow,
    total: i64,
}

async fn list(
    tx: &mut Transaction<'static, Postgres>,
    side: DossierSide,
    owner: Uuid,
    me: Uuid,
    filter: &ListFilter,
) -> AppResult<DossierList> {
    let (base, awaiting) = match side {
        DossierSide::Municipality => ("d.tenant_id = $1", AWAITING_MUNICIPALITY),
        DossierSide::Professional => ("d.professional_id = $1", AWAITING_PROFESSIONAL),
    };
    let sql = format!(
        "select s.*, count(*) over () as total from ({SUMMARY_SELECT}
          where {base}
            and ($2::text is null or d.direction = $2)
            and (cardinality($3::text[]) = 0 or d.status = any($3))
            and ($4::text = '' or d.title ilike '%' || $4 || '%'
                 or (d.year::text || '/' || lpad(d.number::text, 4, '0')) like '%' || $4 || '%'
                 or coalesce(u.display_name, '') ilike '%' || $4 || '%'
                 or coalesce(d.site ->> 'parcel', '') = $4)
            and (not $5 or d.assignee_id = $6)
            and (not $7 or {awaiting})) s
         order by s.updated_at desc
         limit $8 offset $9"
    );
    let statuses: Vec<&str> = filter
        .statuses
        .iter()
        .map(|s| catalog::status_code(*s))
        .collect();
    let rows: Vec<Listed> = sqlx::query_as(sqlx::AssertSqlSafe(sql))
        .bind(owner)
        .bind(filter.direction.map(catalog::direction_code))
        .bind(&statuses)
        .bind(filter.query.as_deref().map(str::trim).unwrap_or_default())
        .bind(filter.mine)
        .bind(me)
        .bind(filter.awaiting)
        .bind(i64::from(filter.limit.clamp(1, 200)))
        .bind(i64::from(filter.offset))
        .fetch_all(&mut **tx)
        .await?;
    let today = today();
    let total = rows.first().map_or(0, |r| r.total);
    Ok(DossierList {
        items: rows.iter().map(|r| r.row.view(Some(side), today)).collect(),
        total: u32::try_from(total).unwrap_or(u32::MAX),
    })
}

/// `GET /v1/tenants/{tenant}/dossiers`: the organisation's, newest change first.
pub async fn list_for_tenant(
    db: &Db,
    access: &Access,
    filter: &ListFilter,
) -> AppResult<DossierList> {
    let mut tx = db.scoped(access.scope()).await?;
    let found = list(
        &mut tx,
        DossierSide::Municipality,
        access.tenant,
        access.actor.user_id,
        filter,
    )
    .await?;
    tx.commit().await?;
    Ok(found)
}

/// `GET /v1/me/dossiers`: the professional's own.
pub async fn list_for_professional(
    db: &Db,
    actor: &Actor,
    filter: &ListFilter,
) -> AppResult<DossierList> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let found = list(
        &mut tx,
        DossierSide::Professional,
        actor.user_id,
        actor.user_id,
        filter,
    )
    .await?;
    tx.commit().await?;
    Ok(found)
}

async fn dashboard(
    tx: &mut Transaction<'static, Postgres>,
    side: DossierSide,
    owner: Uuid,
    me: Uuid,
) -> AppResult<DossierDashboard> {
    let base = match side {
        DossierSide::Municipality => "d.tenant_id = $1",
        DossierSide::Professional => "d.professional_id = $1",
    };
    let counts: Vec<(String, String, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select d.direction, d.status, count(*) from kentos.dossier d where {base} group by 1, 2"
    )))
    .bind(owner)
    .fetch_all(&mut **tx)
    .await?;
    let mine: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "select count(*) from kentos.dossier d where {base} and d.assignee_id = $2 and d.closed_at is null"
    )))
    .bind(owner)
    .bind(me)
    .fetch_one(&mut **tx)
    .await?;
    let open = list(
        tx,
        side,
        owner,
        me,
        &ListFilter {
            limit: 200,
            ..ListFilter::default()
        },
    )
    .await?;
    let awaiting = open.items.iter().filter(|s| s.awaiting_me).count();
    let due_soon = open
        .items
        .iter()
        .filter(|s| s.clock.as_ref().is_some_and(|c| c.days_left <= 3))
        .count();
    Ok(DossierDashboard {
        counts: counts
            .into_iter()
            .filter_map(|(direction, status, n)| {
                Some(StatusCount {
                    direction: catalog::direction_of(&direction)?,
                    status: catalog::status_of(&status)?,
                    count: u32::try_from(n).unwrap_or(u32::MAX),
                })
            })
            .collect(),
        awaiting: u32::try_from(awaiting).unwrap_or(u32::MAX),
        due_soon: u32::try_from(due_soon).unwrap_or(u32::MAX),
        mine: u32::try_from(mine).unwrap_or(u32::MAX),
        recent: open.items.into_iter().take(8).collect(),
    })
}

/// `GET /v1/tenants/{tenant}/portal/summary`.
pub async fn dashboard_for_tenant(db: &Db, access: &Access) -> AppResult<DossierDashboard> {
    let mut tx = db.scoped(access.scope()).await?;
    let found = dashboard(
        &mut tx,
        DossierSide::Municipality,
        access.tenant,
        access.actor.user_id,
    )
    .await?;
    tx.commit().await?;
    Ok(found)
}

/// `GET /v1/me/dossiers/summary`.
pub async fn dashboard_for_professional(db: &Db, actor: &Actor) -> AppResult<DossierDashboard> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let found = dashboard(
        &mut tx,
        DossierSide::Professional,
        actor.user_id,
        actor.user_id,
    )
    .await?;
    tx.commit().await?;
    Ok(found)
}
