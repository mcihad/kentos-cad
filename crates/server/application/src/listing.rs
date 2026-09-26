//! The project catalog's lists (docs/adr/0015, 0028; TODOS.md CLOUD-04,
//! CLOUD-12): a tenant's projects, “Projelerim” across tenants, and the
//! views of one person's catalog (their own, a workspace's, shared with
//! them, recently opened, favourites, archived, the trash), searched,
//! sorted and paged on the server. A list holds only projects the caller has
//! a role in: row-level security leaves the others out before anything is
//! searched or counted, and each entry carries what the caller may do in it.
//! Also one project's entry (the answer of a catalog command) and details.
//!
//! A view over several tenants asks each tenant in its own scope (row-level
//! security sees one tenant at a time) and merges the answers: each tenant
//! gives at most one page after the cursor, in the same order, so the merged
//! page is exactly the next one. The order's key comes from the database
//! itself (a time, or the name folded as searches fold it and compared byte
//! by byte, `collate "C"`), so the merge agrees with the database whatever
//! its locale. Ties go by project id.

use std::cmp::Ordering;
use std::time::Duration;

use kentos_contracts::{
    AreaUnit, Bounds, CatalogSort, CatalogView, LayerNode, LayerNodeType, ProjectDetails,
    ProjectList, ProjectPage, ProjectState, ProjectStorage, ProjectSummary, ProjectType,
};
use kentos_postgres::{Db, Scope};
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::access::{ProjectAccess, not_found, tenant_kind, view_of};
use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::people::{FOLD_FROM, FOLD_TO, fold};
use crate::projects::rfc3339;
use crate::tenancy::{self, Access};

/// Rows of a page unless asked otherwise, and at most.
pub const PAGE_DEFAULT: u32 = 50;
pub const PAGE_LIMIT: u32 = 200;
/// Longest search text, in characters.
pub const SEARCH_MAX: usize = 100;

/// One project as a catalog list shows it, with the key it is ordered by.
#[derive(sqlx::FromRow)]
struct EntryRow {
    id: Uuid,
    name: String,
    srid: i32,
    data_revision: i64,
    updated_at: OffsetDateTime,
    role: Option<String>,
    tenant_id: Uuid,
    tenant_name: String,
    tenant_kind: String,
    viewer_download: bool,
    owner_name: Option<String>,
    project_type: String,
    description: String,
    tags: Vec<String>,
    catalog_version: i64,
    created_at: OffsetDateTime,
    creator_name: Option<String>,
    area_unit: Option<String>,
    archived_at: Option<OffsetDateTime>,
    trashed_at: Option<OffsetDateTime>,
    trashed_by_name: Option<String>,
    purge_after: Option<OffsetDateTime>,
    opened_at: Option<OffsetDateTime>,
    favorite: bool,
    sort_time: Option<OffsetDateTime>,
    sort_text: Option<String>,
}

/// Everything an entry shows. The owner's, creator's and trasher's names are
/// there when row-level security shows them (a member of the tenant, or a
/// personal space's own person); one who left the organisation comes without.
const ENTRY_COLUMNS: &str = "select p.id, p.name, p.srid, p.data_revision, p.updated_at,
        kentos.project_role(p.tenant_id, p.id, p.owner_user_id) as role,
        t.id as tenant_id, t.name as tenant_name, t.kind as tenant_kind, t.viewer_download,
        ou.display_name as owner_name, p.project_type, p.description, p.tags, p.catalog_version,
        p.created_at, cu.display_name as creator_name, p.settings ->> 'areaUnit' as area_unit,
        p.archived_at, p.deleted_at as trashed_at, du.display_name as trashed_by_name, p.purge_after,
        r.opened_at, f.user_id is not null as favorite";

/// The projects of the scope's tenant (`$1`) the caller has a role in, with
/// the caller's own recent and favourite marks.
const ENTRY_FROM: &str = " from kentos.project p join kentos.tenant t on t.id = p.tenant_id
   left join kentos.app_user ou on ou.id = p.owner_user_id
   left join kentos.app_user cu on cu.id = p.created_by
   left join kentos.app_user du on du.id = p.deleted_by
   left join kentos.project_recent r on r.tenant_id = p.tenant_id and r.project_id = p.id and r.user_id = kentos.current_user_id()
   left join kentos.project_favorite f on f.tenant_id = p.tenant_id and f.project_id = p.id and f.user_id = kentos.current_user_id()
  where p.tenant_id = $1";

const NO_SORT: &str = ", null::timestamptz as sort_time, null::text as sort_text";

/// Active: neither archived nor in the trash.
const ACTIVE: &str = " and p.deleted_at is null and p.archived_at is null";

impl EntryRow {
    fn state(&self) -> ProjectState {
        if self.trashed_at.is_some() {
            ProjectState::Trashed
        } else if self.archived_at.is_some() {
            ProjectState::Archived
        } else {
            ProjectState::Active
        }
    }

    fn summary(self) -> Option<ProjectSummary> {
        let access = view_of(self.role.as_deref()?, self.viewer_download)?;
        let state = self.state();
        let area_unit = self
            .area_unit
            .as_deref()
            .and_then(|u| serde_json::from_value(serde_json::Value::String(u.into())).ok())
            .unwrap_or(AreaUnit::M2);
        Some(ProjectSummary {
            id: self.id.to_string(),
            name: self.name,
            srid: self.srid as u32,
            data_revision: self.data_revision.to_string(),
            updated_at: rfc3339(self.updated_at),
            tenant_id: self.tenant_id.to_string(),
            tenant_name: self.tenant_name,
            tenant_kind: tenant_kind(&self.tenant_kind),
            owner_name: self.owner_name.unwrap_or_default(),
            access,
            project_type: ProjectType::from_name(&self.project_type).unwrap_or_default(),
            description: self.description,
            tags: self.tags,
            state,
            catalog_version: self.catalog_version.to_string(),
            created_at: rfc3339(self.created_at),
            creator_name: self.creator_name.unwrap_or_default(),
            area_unit,
            storage: ProjectStorage::Database,
            favorite: self.favorite,
            opened_at: self.opened_at.map(rfc3339),
            archived_at: self.archived_at.map(rfc3339),
            trashed_at: self.trashed_at.map(rfc3339),
            trashed_by_name: self.trashed_at.and(self.trashed_by_name),
            purge_after: self.purge_after.map(rfc3339),
        })
    }
}

/// Newest first.
fn summaries(mut rows: Vec<EntryRow>) -> Vec<ProjectSummary> {
    rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.id.cmp(&b.id)));
    rows.into_iter().filter_map(EntryRow::summary).collect()
}

/// A tenant's active projects the caller may see: the ones they own, the
/// ones shared with them and, for an organisation's owners and admins under
/// its policy, every one. Newest first.
pub async fn list(db: &Db, access: &Access) -> AppResult<ProjectList> {
    let mut tx = db.scoped(access.scope()).await?;
    let rows: Vec<EntryRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{ENTRY_COLUMNS}{NO_SORT}{ENTRY_FROM}{ACTIVE}"
    )))
    .bind(access.tenant)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ProjectList {
        projects: summaries(rows),
    })
}

/// Where the caller may have projects: their personal space (opened now if it
/// is not there yet), the tenants of their active memberships and of their grants.
async fn workspaces(db: &Db, actor: &Actor) -> AppResult<Vec<Uuid>> {
    let personal = tenancy::ensure_personal(db, actor).await?;
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
    tenants.sort_unstable();
    Ok(tenants)
}

fn tenant_scope(tenant: Uuid, actor: &Actor) -> Scope {
    Scope {
        tenant: Some(tenant),
        user: Some(actor.user_id),
        project: None,
    }
}

/// “Projelerim” (`GET /v1/me/projects`): the active projects the caller owns
/// (their personal space's and the organisation projects they created) and
/// the ones shared with them, in every tenant, newest first. An
/// organisation's projects they reach only through its policy are the
/// organisation's list, not this one.
pub async fn mine(db: &Db, actor: &Actor) -> AppResult<ProjectList> {
    let mut rows = Vec::new();
    for tenant in workspaces(db, actor).await? {
        let mut tx = db.scoped(tenant_scope(tenant, actor)).await?;
        let found: Vec<EntryRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{ENTRY_COLUMNS}{NO_SORT}{ENTRY_FROM}{ACTIVE} and (p.owner_user_id = $2 or t.owner_user_id = $2
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

/// One project's entry as the caller sees it now, inside a transaction scoped
/// to its tenant (and the project): the answer of a catalog command.
pub(crate) async fn entry(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
    project: Uuid,
) -> AppResult<ProjectSummary> {
    let row: Option<EntryRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{ENTRY_COLUMNS}{NO_SORT}{ENTRY_FROM} and p.id = $2"
    )))
    .bind(tenant)
    .bind(project)
    .fetch_optional(&mut **tx)
    .await?;
    row.and_then(EntryRow::summary).ok_or_else(not_found)
}

/// Leaf layers of a layer tree.
fn layers(nodes: &[LayerNode]) -> u32 {
    nodes
        .iter()
        .map(|n| u32::from(n.kind == LayerNodeType::Layer) + layers(&n.children))
        .sum()
}

/// A project's entry with what is worked out on asking: how many objects and
/// layers it has and the extent of its objects' stored geometry. Needs
/// `project.read` (every role); a project in the trash answers 410.
pub async fn details(db: &Db, access: &ProjectAccess) -> AppResult<ProjectDetails> {
    access.live()?;
    let mut tx = db.scoped(access.scope()).await?;
    let project = entry(&mut tx, access.tenant, access.project).await?;
    let tree: serde_json::Value =
        sqlx::query_scalar("select layers from kentos.project where tenant_id = $1 and id = $2")
            .bind(access.tenant)
            .bind(access.project)
            .fetch_one(&mut *tx)
            .await?;
    type Extent = (i64, Option<f64>, Option<f64>, Option<f64>, Option<f64>);
    let (objects, min_x, min_y, max_x, max_y): Extent = sqlx::query_as(
        "select count(*),
                public.st_xmin(public.st_extent(geom)::public.box3d), public.st_ymin(public.st_extent(geom)::public.box3d),
                public.st_xmax(public.st_extent(geom)::public.box3d), public.st_ymax(public.st_extent(geom)::public.box3d)
           from kentos.feature where tenant_id = $1 and project_id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let tree: Vec<LayerNode> = serde_json::from_value(tree)
        .map_err(|e| AppError::invalid(format!("Proje katmanları okunamadı: {e}")))?;
    let bounds = match (min_x, min_y, max_x, max_y) {
        (Some(min_x), Some(min_y), Some(max_x), Some(max_y)) => Some(Bounds {
            min_x,
            min_y,
            max_x,
            max_y,
        }),
        _ => None,
    };
    Ok(ProjectDetails {
        project,
        bounds,
        feature_count: objects.to_string(),
        layer_count: layers(&tree),
    })
}

// ── The catalog's views ──────────────────────────────────────────────────

/// A request for one page of a view (`GET /v1/me/catalog`).
#[derive(Clone, Debug)]
pub struct CatalogQuery {
    pub view: CatalogView,
    /// The workspace of the `organization` view; for the others, keeps the list to that workspace.
    pub tenant: Option<Uuid>,
    /// Words that must all be in the name, description or tags (Turkish letters folded).
    pub search: String,
    pub project_type: Option<ProjectType>,
    /// Absent: the view's own order.
    pub sort: Option<CatalogSort>,
    pub limit: Option<u32>,
    /// The `next` of the page before.
    pub after: Option<String>,
}

/// What the view holds, on top of the caller's role (row-level security).
fn view_filter(view: CatalogView) -> &'static str {
    match view {
        CatalogView::Mine => {
            " and (p.owner_user_id = kentos.current_user_id() or t.owner_user_id = kentos.current_user_id()) and p.deleted_at is null and p.archived_at is null"
        }
        CatalogView::Organization => ACTIVE,
        CatalogView::Shared => {
            " and p.deleted_at is null and p.archived_at is null and kentos.project_role(p.tenant_id, p.id, p.owner_user_id) not in ('owner', 'policy')"
        }
        CatalogView::Recent => " and r.opened_at is not null and p.deleted_at is null",
        CatalogView::Favorites => " and f.user_id is not null and p.deleted_at is null",
        CatalogView::Archived => " and p.archived_at is not null and p.deleted_at is null",
        // Those who may restore it or remove it for good: the roles with project.delete.
        CatalogView::Trash => {
            " and p.deleted_at is not null and kentos.project_role(p.tenant_id, p.id, p.owner_user_id) in ('owner', 'policy')"
        }
    }
}

/// The search (`$2`, patterns; `$3`, `$4` the fold tables) and the type (`$5`).
const SEARCH_FILTER: &str = " and lower(translate(p.name || ' ' || p.description || ' ' || array_to_string(p.tags, ' '), $3, $4) collate \"C\") like all ($2::text[])
    and ($5::text is null or p.project_type = $5)";

/// The name as it is sorted: folded like a search, compared byte by byte.
const FOLDED_NAME: &str = "lower(translate(p.name, $3, $4) collate \"C\")";

/// The view's own order when none is asked for.
fn default_sort(view: CatalogView) -> CatalogSort {
    match view {
        CatalogView::Recent => CatalogSort::Opened,
        CatalogView::Trash => CatalogSort::Trashed,
        _ => CatalogSort::Updated,
    }
}

/// The time a sort orders by (newest first), or none for the name.
fn sort_time(sort: CatalogSort) -> Option<&'static str> {
    match sort {
        CatalogSort::Updated => Some("p.updated_at"),
        CatalogSort::Created => Some("p.created_at"),
        CatalogSort::Opened => Some("r.opened_at"),
        CatalogSort::Trashed => Some("p.deleted_at"),
        CatalogSort::Name => None,
    }
}

/// The page's query: the view, the search, the cursor (`$6` the key, `$7`
/// the id) and the size (`$8`), in the sort's order.
fn page_sql(view: CatalogView, sort: CatalogSort) -> String {
    let filter = view_filter(view);
    match sort_time(sort) {
        Some(time) => format!(
            "{ENTRY_COLUMNS}, {time} as sort_time, null::text as sort_text{ENTRY_FROM}{filter}{SEARCH_FILTER}
               and ($6::timestamptz is null or {time} < $6 or ({time} = $6 and p.id < $7))
             order by {time} desc, p.id desc limit $8"
        ),
        None => format!(
            "{ENTRY_COLUMNS}, null::timestamptz as sort_time, {FOLDED_NAME} as sort_text{ENTRY_FROM}{filter}{SEARCH_FILTER}
               and ($6::text is null or {FOLDED_NAME} > $6 or ({FOLDED_NAME} = $6 and p.id > $7))
             order by {FOLDED_NAME} asc, p.id asc limit $8"
        ),
    }
}

fn count_sql(view: CatalogView) -> String {
    format!(
        "select count(*){}{}{SEARCH_FILTER}",
        ENTRY_FROM,
        view_filter(view)
    )
}

/// The search's words as LIKE patterns (`%word%`, `%`, `_` and `\` taken
/// literally), folded as the database folds the text; none for an empty search.
pub fn search_words(search: &str) -> AppResult<Vec<String>> {
    if search.chars().count() > SEARCH_MAX {
        return Err(AppError::invalid(format!(
            "Arama en çok {SEARCH_MAX} karakter olabilir."
        )));
    }
    Ok(search
        .split_whitespace()
        .map(|w| {
            format!(
                "%{}%",
                fold(w)
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
        })
        .collect())
}

/// Where a page ends: its last entry's key and id.
#[derive(Clone, Debug, PartialEq)]
enum Key {
    Time(OffsetDateTime),
    Text(String),
}

fn bad_cursor() -> AppError {
    AppError::invalid("Sayfa imleci geçersiz; listeyi baştan yükleyin.")
}

/// The cursor as the client sends it back: `[key, id]`, JSON.
fn read_cursor(after: &str, sort: CatalogSort) -> AppResult<(Key, Uuid)> {
    let (key, id): (String, String) = serde_json::from_str(after).map_err(|_| bad_cursor())?;
    let id = Uuid::parse_str(&id).map_err(|_| bad_cursor())?;
    let key = match sort_time(sort) {
        Some(_) => Key::Time(OffsetDateTime::parse(&key, &Rfc3339).map_err(|_| bad_cursor())?),
        None => Key::Text(key),
    };
    Ok((key, id))
}

fn write_cursor(key: &Key, id: Uuid) -> String {
    let key = match key {
        Key::Time(t) => rfc3339(*t),
        Key::Text(t) => t.clone(),
    };
    serde_json::to_string(&(key, id.to_string())).expect("cursors serialize")
}

fn key_of(row: &EntryRow, sort: CatalogSort) -> Key {
    match sort_time(sort) {
        Some(_) => Key::Time(row.sort_time.unwrap_or(OffsetDateTime::UNIX_EPOCH)),
        None => Key::Text(row.sort_text.clone().unwrap_or_default()),
    }
}

/// The page order: times newest first, names A to Z; ties by id the same way.
fn order(a: &EntryRow, b: &EntryRow, sort: CatalogSort) -> Ordering {
    match sort_time(sort) {
        Some(_) => b.sort_time.cmp(&a.sort_time).then(b.id.cmp(&a.id)),
        None => a.sort_text.cmp(&b.sort_text).then(a.id.cmp(&b.id)),
    }
}

/// One page of a view of the caller's catalog, and how many projects the
/// whole view holds with this search. `retention` is how long the trash keeps
/// a project moved there now.
pub async fn page(
    db: &Db,
    actor: &Actor,
    query: &CatalogQuery,
    retention: Duration,
) -> AppResult<ProjectPage> {
    let sort = query.sort.unwrap_or_else(|| default_sort(query.view));
    match (sort, query.view) {
        (CatalogSort::Opened, v) if v != CatalogView::Recent => {
            return Err(AppError::invalid(
                "Son açılmaya göre yalnız son kullanılanlar sıralanır.",
            ));
        }
        (CatalogSort::Trashed, v) if v != CatalogView::Trash => {
            return Err(AppError::invalid(
                "Çöpe taşınmaya göre yalnız çöp kutusu sıralanır.",
            ));
        }
        _ => {}
    }
    let limit = query.limit.unwrap_or(PAGE_DEFAULT).clamp(1, PAGE_LIMIT);
    let words = search_words(&query.search)?;
    let cursor = query
        .after
        .as_deref()
        .map(|a| read_cursor(a, sort))
        .transpose()?;
    let tenants = match query.view {
        CatalogView::Organization => {
            let tenant = query.tenant.ok_or_else(|| {
                AppError::invalid("Kurum projeleri için bir kurum seçin (tenant).")
            })?;
            // Its members only (404 for a tenant the caller is not in, 403 for one they may not use now).
            vec![tenancy::access(db, actor, tenant).await?.tenant]
        }
        _ => {
            let mut all = workspaces(db, actor).await?;
            if let Some(only) = query.tenant {
                all.retain(|t| *t == only);
            }
            all
        }
    };
    let (time_key, text_key, key_id) = match &cursor {
        Some((Key::Time(t), id)) => (Some(*t), None, Some(*id)),
        Some((Key::Text(t), id)) => (None, Some(t.clone()), Some(*id)),
        None => (None, None, None),
    };
    let page_sql = page_sql(query.view, sort);
    let count_sql = count_sql(query.view);
    let type_name = query.project_type.map(|t| t.name());
    let mut rows: Vec<EntryRow> = Vec::new();
    let mut total: i64 = 0;
    for tenant in tenants {
        let mut tx = db.scoped(tenant_scope(tenant, actor)).await?;
        let q = sqlx::query_as(sqlx::AssertSqlSafe(page_sql.clone()))
            .bind(tenant)
            .bind(&words)
            .bind(FOLD_FROM)
            .bind(FOLD_TO)
            .bind(type_name);
        let q = match sort_time(sort) {
            Some(_) => q.bind(time_key),
            None => q.bind(text_key.clone()),
        };
        let found: Vec<EntryRow> = q
            .bind(key_id)
            .bind(i64::from(limit) + 1)
            .fetch_all(&mut *tx)
            .await?;
        let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(count_sql.clone()))
            .bind(tenant)
            .bind(&words)
            .bind(FOLD_FROM)
            .bind(FOLD_TO)
            .bind(type_name)
            .fetch_one(&mut *tx)
            .await?;
        tx.commit().await?;
        total += n;
        rows.extend(found);
    }
    rows.sort_by(|a, b| order(a, b, sort));
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next = more
        .then(|| rows.last().map(|r| write_cursor(&key_of(r, sort), r.id)))
        .flatten();
    Ok(ProjectPage {
        projects: rows.into_iter().filter_map(EntryRow::summary).collect(),
        total: u32::try_from(total).unwrap_or(u32::MAX),
        next,
        trash_retention_days: u32::try_from(retention.as_secs() / 86_400).unwrap_or(u32::MAX),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn searches_fold_and_escape_every_word() {
        assert_eq!(search_words("  Ada  İFRAZ ").unwrap(), ["%ada%", "%ifraz%"]);
        assert_eq!(search_words("%40_").unwrap(), ["%\\%40\\_%"]);
        assert!(search_words("").unwrap().is_empty());
        assert!(search_words(&"x".repeat(SEARCH_MAX + 1)).is_err());
        // One letter is enough: the list is the caller's own already.
        assert_eq!(search_words("ş").unwrap(), ["%s%"]);
    }

    #[test]
    fn a_cursor_reads_back_what_it_was_written_from() {
        let id = Uuid::now_v7();
        let t = OffsetDateTime::parse("2026-09-26T10:11:12.123456Z", &Rfc3339).unwrap();
        let written = write_cursor(&Key::Time(t), id);
        assert_eq!(
            read_cursor(&written, CatalogSort::Updated).unwrap(),
            (Key::Time(t), id)
        );
        let named = write_cursor(&Key::Text("ada 101".into()), id);
        assert_eq!(
            read_cursor(&named, CatalogSort::Name).unwrap(),
            (Key::Text("ada 101".into()), id)
        );
        for bad in ["", "[]", "[\"x\"]", "[\"dün\", \"id\"]", "{\"a\":1}"] {
            assert!(read_cursor(bad, CatalogSort::Updated).is_err(), "{bad}");
        }
    }
}
