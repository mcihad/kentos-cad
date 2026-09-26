//! Projects and reading their objects (CLAUDE.md §15, §21.2). Opening a
//! project reads its metadata first (with the event cursor of that moment),
//! then the objects page by page; events after the cursor fill in whatever
//! changed while the pages were read.
//!
//! Who sees what (docs/adr/0015): a project's own use cases take a
//! [`ProjectAccess`]; the lists are listing.rs. The creator of a project is
//! its owner.
//!
//! A deleted project (in the trash, lifecycle.rs) is left out of the lists
//! and refuses opening (410, `project_deleted`) to those who had access; its
//! rows stay for recovery. An archived one opens read-only.
//!
//! Opening a project (its first page of objects) and creating one mark it in
//! the person's recently used projects (docs/adr/0028).

use kentos_contracts::{
    FeaturePage, FeatureRecord, LayerNode, LayerNodeType, ProjectCreate, ProjectInfo,
    ProjectStorage,
};
use kentos_postgres::Scope;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::{self, ProjectAccess, not_found};
use crate::cad::{Stored, from_stored};
use crate::error::{AppError, AppResult};
use crate::tenancy::{Access, Capability};

pub const PAGE_MAX: i64 = 2000;

/// A leaf layer of the tree and whether it is locked (itself or through a parent group).
pub fn find_layer<'a>(tree: &'a [LayerNode], id: &str) -> Option<(&'a LayerNode, bool)> {
    fn walk<'a>(nodes: &'a [LayerNode], id: &str, locked: bool) -> Option<(&'a LayerNode, bool)> {
        for n in nodes {
            let here = locked || n.locked;
            if n.id == id {
                return Some((n, here));
            }
            if let Some(found) = walk(&n.children, id, here) {
                return Some(found);
            }
        }
        None
    }
    walk(tree, id, false)
}

/// Unique ids, at least one layer, and the active layer is a layer (not a group).
pub fn check_tree(tree: &[LayerNode], active: &str) -> AppResult<()> {
    fn ids<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a str>) {
        for n in nodes {
            out.push(&n.id);
            ids(&n.children, out);
        }
    }
    let mut all = Vec::new();
    ids(tree, &mut all);
    let count = all.len();
    all.sort_unstable();
    all.dedup();
    if all.len() != count {
        return Err(AppError::invalid(
            "Katman ağacında aynı kimlik birden çok kez var.",
        ));
    }
    match find_layer(tree, active) {
        Some((n, _)) if n.kind == LayerNodeType::Layer => Ok(()),
        _ => Err(AppError::invalid(format!(
            "Etkin katman “{active}” ağaçta bir katman değil."
        ))),
    }
}

/// The stored name of a storage mode (`project.storage`, migration 0006).
pub fn storage_name(storage: ProjectStorage) -> &'static str {
    match storage {
        ProjectStorage::Database => "database",
        ProjectStorage::File => "file",
    }
}

/// A stored storage mode; anything else is the database's (the column's check allows no other).
pub fn storage_of(name: &str) -> ProjectStorage {
    if name == "file" {
        ProjectStorage::File
    } else {
        ProjectStorage::Database
    }
}

pub fn check_name(name: &str) -> AppResult<()> {
    if name.trim().is_empty() || name.len() > 200 {
        return Err(AppError::invalid_at(
            "name",
            "Proje adı boş olamaz ve en çok 200 karakter olabilir.",
        ));
    }
    Ok(())
}

/// Only SRIDs PostGIS knows are accepted (the same catalogue the transformations will use).
pub async fn check_srid(tx: &mut Transaction<'static, Postgres>, srid: u32) -> AppResult<()> {
    let known: bool =
        sqlx::query_scalar("select exists (select 1 from public.spatial_ref_sys where srid = $1)")
            .bind(srid as i32)
            .fetch_one(&mut **tx)
            .await?;
    if known {
        Ok(())
    } else {
        Err(AppError::invalid(format!(
            "EPSG:{srid} koordinat sistemi tanınmıyor."
        )))
    }
}

fn json<T: serde::Serialize>(v: &T) -> AppResult<Value> {
    serde_json::to_value(v).map_err(|e| AppError::invalid(e.to_string()))
}

pub(crate) fn rfc3339(t: time::OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// The project an earlier create with this key made (the actor's own creates only).
async fn created_with(
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    key: &str,
) -> AppResult<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "select project_id from kentos.command_log where tenant_id = $1 and idempotency_key = $2 and command_name = 'project.create' and actor = $3",
    )
    .bind(access.tenant)
    .bind(key)
    .bind(access.actor.user_id)
    .fetch_optional(&mut **tx)
    .await?)
}

/// Marks the project in scope as opened by the caller now (their recently used projects).
pub(crate) async fn opened(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
    project: Uuid,
    user: Uuid,
) -> AppResult<()> {
    sqlx::query(
        "insert into kentos.project_recent (tenant_id, project_id, user_id) values ($1, $2, $3)
         on conflict (tenant_id, project_id, user_id) do update set opened_at = now()",
    )
    .bind(tenant)
    .bind(project)
    .bind(user)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Opens a new, empty project owned by its creator, with its catalog
/// metadata (docs/adr/0028); `idempotency_key` makes a retried create return
/// the same project, also when two copies of the request arrive at once.
pub async fn create(
    db: &kentos_postgres::Db,
    access: &Access,
    input: ProjectCreate,
    idempotency_key: Option<&str>,
) -> AppResult<ProjectInfo> {
    access.require(Capability::ProjectCreate)?;
    check_name(&input.name)?;
    check_tree(&input.layers, &input.active_layer)?;
    let description =
        crate::catalog::check_description(input.description.as_deref().unwrap_or(""))?;
    let tags = crate::catalog::normalize_tags(input.tags.as_deref().unwrap_or(&[]))?;
    let project_type = input.project_type.unwrap_or_default();
    let id = Uuid::now_v7();
    // The new project is in scope from the start: its audit and command log rows belong to it.
    let mut tx = db
        .scoped(Scope {
            project: Some(id),
            ..access.scope()
        })
        .await?;
    if let Some(key) = idempotency_key
        && let Some(earlier) = created_with(&mut tx, access, key).await?
    {
        tx.commit().await?;
        return info(
            db,
            &access::project(db, &access.actor, access.tenant, earlier).await?,
        )
        .await;
    }
    check_srid(&mut tx, input.settings.srid).await?;
    sqlx::query(
        "insert into kentos.project (tenant_id, id, name, srid, settings, layers, active_layer, origin_x, origin_y, home_view, styles, created_by, owner_user_id,
                                     description, project_type, tags, storage)
         values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12, $13, $14, $15, $16)",
    )
    .bind(access.tenant)
    .bind(id)
    .bind(input.name.trim())
    .bind(input.settings.srid as i32)
    .bind(json(&input.settings)?)
    .bind(json(&input.layers)?)
    .bind(&input.active_layer)
    .bind(input.origin.x)
    .bind(input.origin.y)
    .bind(input.home_view.map(|b| json(&b)).transpose()?)
    .bind(json(&input.styles)?)
    .bind(access.actor.user_id)
    .bind(&description)
    .bind(project_type.name())
    .bind(&tags)
    .bind(storage_name(input.storage.unwrap_or(ProjectStorage::Database)))
    .execute(&mut *tx)
    .await?;
    sqlx::query("insert into kentos.audit_event (tenant_id, project_id, actor, action, detail) values ($1, $2, $3, 'project.create', $4)")
        .bind(access.tenant)
        .bind(id)
        .bind(access.actor.user_id)
        .bind(serde_json::json!({
            "name": input.name.trim(),
            "projectType": project_type.name(),
            "storage": storage_name(input.storage.unwrap_or(ProjectStorage::Database)),
        }))
        .execute(&mut *tx)
        .await?;
    opened(&mut tx, access.tenant, id, access.actor.user_id).await?;
    if let Some(key) = idempotency_key {
        let logged = sqlx::query(
            "insert into kentos.command_log (tenant_id, idempotency_key, project_id, command_name, request_hash, response, actor)
             values ($1, $2, $3, 'project.create', public.digest($4, 'sha256'), '{}'::jsonb, $5)",
        )
        .bind(access.tenant)
        .bind(key)
        .bind(id)
        .bind(&input.name)
        .bind(access.actor.user_id)
        .execute(&mut *tx)
        .await;
        match logged {
            Ok(_) => {}
            // The same create arrived twice at once and the other copy won: nothing of this one stays; answer with its project.
            Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23505") => {
                drop(tx);
                let mut tx = db.scoped(access.scope()).await?;
                let earlier = created_with(&mut tx, access, key).await?;
                tx.commit().await?;
                let Some(earlier) = earlier else {
                    return Err(AppError::invalid(
                        "Bu idempotency anahtarı başka bir istek için kullanılmış; her isteğe yeni bir anahtar verin.",
                    ));
                };
                return info(
                    db,
                    &access::project(db, &access.actor, access.tenant, earlier).await?,
                )
                .await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    tx.commit().await?;
    info(
        db,
        &access::project(db, &access.actor, access.tenant, id).await?,
    )
    .await
}

/// The refusal for a deleted project (in the trash), the same wherever it is asked for.
pub(crate) fn gone(name: &str) -> AppError {
    AppError::deleted(format!(
        "“{name}” projesi silindi (çöp kutusunda); açılamaz ve değiştirilemez. Proje sahibi ya da kurum yöneticisi onu çöp kutusundan geri yükleyebilir."
    ))
}

type ProjectRow = (
    String,
    Value,
    Value,
    String,
    f64,
    f64,
    Option<Value>,
    Value,
    i64,
    i64,
    i64,
    i64,
    bool,
    String,
);

/// A project's metadata, object count and the event cursor of this moment:
/// the newest event's, or the log's horizon once old events were removed
/// (never below it, or a client would be sent to reopen again and again).
/// Counted only after the access check, inside the project's scope.
pub async fn info(db: &kentos_postgres::Db, access: &ProjectAccess) -> AppResult<ProjectInfo> {
    access.live()?;
    let mut tx = db.scoped(access.scope()).await?;
    let row: Option<ProjectRow> = sqlx::query_as(
        "select p.name, p.settings, p.layers, p.active_layer, p.origin_x, p.origin_y, p.home_view, p.styles, p.meta_version, p.data_revision,
                (select count(*) from kentos.feature f where f.tenant_id = p.tenant_id and f.project_id = p.id),
                greatest(coalesce((select max(seq) from kentos.outbox_event o where o.tenant_id = p.tenant_id and o.project_id = p.id), 0),
                         coalesce((select pruned_through from kentos.outbox_horizon h where h.tenant_id = p.tenant_id and h.project_id = p.id), 0)),
                p.deleted_at is not null, p.storage
           from kentos.project p where p.tenant_id = $1 and p.id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    let (
        name,
        settings,
        layers,
        active_layer,
        ox,
        oy,
        home,
        styles,
        meta,
        rev,
        count,
        cursor,
        deleted,
        storage,
    ) = row.ok_or_else(not_found)?;
    if deleted {
        return Err(gone(&name));
    }
    let bad = |e: serde_json::Error| AppError::invalid(format!("Proje kaydı okunamadı: {e}"));
    Ok(ProjectInfo {
        id: access.project.to_string(),
        tenant_id: access.tenant.to_string(),
        tenant_name: access.tenant_name.clone(),
        tenant_kind: access.tenant_kind,
        access: access.view(),
        state: access.state(),
        name,
        settings: serde_json::from_value(settings).map_err(bad)?,
        origin: kentos_contracts::Vec2 { x: ox, y: oy },
        home_view: home.map(serde_json::from_value).transpose().map_err(bad)?,
        layers: serde_json::from_value(layers).map_err(bad)?,
        active_layer,
        styles: serde_json::from_value(styles).map_err(bad)?,
        meta_version: meta.to_string(),
        data_revision: rev.to_string(),
        feature_count: count.to_string(),
        event_cursor: cursor.to_string(),
        storage: storage_of(&storage),
    })
}

pub(crate) type FeatureRow = (
    Uuid,
    i64,
    String,
    String,
    String,
    Option<Vec<u8>>,
    Option<Value>,
    Value,
    Option<String>,
    Option<String>,
    Option<String>,
);

pub(crate) const FEATURE_COLUMNS: &str = "id, version, layer_id, kind, source_kind, public.st_asewkb(geom), cad_definition, properties, label, color, symbol";

pub(crate) fn record(
    (
        id,
        version,
        layer_id,
        kind,
        source_kind,
        geom,
        cad_definition,
        properties,
        label,
        color,
        symbol,
    ): FeatureRow,
) -> AppResult<FeatureRecord> {
    let source_kind = if source_kind == "geom" { "geom" } else { "cad" };
    let stored = Stored {
        layer_id,
        kind,
        source_kind,
        geom,
        cad_definition,
        properties,
        label,
        color,
        symbol,
    };
    let entity = from_stored(&stored)
        .map_err(|e| AppError::invalid(format!("Nesne {id} okunamadı: {e}")))?;
    Ok(FeatureRecord {
        id: id.to_string(),
        version: version.to_string(),
        entity,
    })
}

/// The project in scope, still visible to the caller and not deleted: 404
/// when they lost their role since the access check, 410 when it was deleted.
async fn live_project(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
) -> AppResult<()> {
    let row: Option<(String, bool)> = sqlx::query_as(
        "select name, deleted_at is not null from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_optional(&mut **tx)
    .await?;
    match row {
        None => Err(not_found()),
        Some((name, true)) => Err(gone(&name)),
        Some(_) => Ok(()),
    }
}

/// Objects in id order after `after`, at most `limit` (≤ 2 000). The first
/// page is opening the project: it goes to the caller's recently used ones.
pub async fn features(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    after: Option<Uuid>,
    limit: i64,
) -> AppResult<FeaturePage> {
    access.live()?;
    let limit = limit.clamp(1, PAGE_MAX);
    let mut tx = db.scoped(access.scope()).await?;
    live_project(&mut tx, access).await?;
    if after.is_none() {
        opened(&mut tx, access.tenant, access.project, access.actor.user_id).await?;
    }
    let rows: Vec<FeatureRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {FEATURE_COLUMNS} from kentos.feature where tenant_id = $1 and project_id = $2 and id > $3 order by id limit $4"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .bind(after.unwrap_or(Uuid::nil()))
    .bind(limit + 1)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let more = rows.len() as i64 > limit;
    let features: Vec<FeatureRecord> = rows
        .into_iter()
        .take(limit as usize)
        .map(record)
        .collect::<AppResult<_>>()?;
    let next = more
        .then(|| features.last().map(|f| f.id.clone()))
        .flatten();
    Ok(FeaturePage { features, next })
}

/// The current copies of some objects (after an event says they changed); missing ones are left out.
pub async fn features_by_id(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    ids: &[Uuid],
) -> AppResult<Vec<FeatureRecord>> {
    access.live()?;
    if ids.len() as i64 > PAGE_MAX {
        return Err(AppError::invalid(format!(
            "Bir istekte en çok {PAGE_MAX} nesne istenebilir."
        )));
    }
    let mut tx = db.scoped(access.scope()).await?;
    live_project(&mut tx, access).await?;
    let rows: Vec<FeatureRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {FEATURE_COLUMNS} from kentos.feature where tenant_id = $1 and project_id = $2 and id = any($3) order by id"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .bind(ids)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    rows.into_iter().map(record).collect()
}
