//! `project.changes` v1: the one lasting edit command of Faz B (CLAUDE.md
//! §15 "Edit commit protokolü", §18). One envelope is one atomic commit:
//! object creates, updates and deletes and optionally the project's
//! metadata. In a single transaction it
//!
//! 1. locks the project row (commits of one project are serialized),
//! 2. returns the stored answer if this idempotency key was already
//!    committed (checked under the lock, so a concurrent retry waits and
//!    then replays instead of conflicting),
//! 3. asks the caller's access again under the lock (a grant taken away or
//!    lowered before this commit counts, docs/adr/0015): objects need
//!    `feature.write`, the metadata `project.edit`,
//! 4. compares every expected version; any difference is a 409 with the
//!    server's current copies, and nothing is written,
//! 5. writes the changes, bumps versions and the project's data revision,
//!    and records audit, an outbox event and the idempotent answer.
//!
//! Objects on a locked layer are refused (CLAUDE.md §7), and so is any
//! change to a deleted project (410) or an archived one (409,
//! docs/adr/0028); a command it had already committed is still answered
//! from the log. A new name is also a change of the catalog's metadata: it
//! raises the project's catalog version and the audit records it.
//!
//! Block definitions (docs/adr/0144 §5) are made, replaced and removed in the
//! same commit, each guarded by `expectedVersions["block:<id>"]`: the
//! definitions it leaves pass the block rules, every insert it leaves places
//! one of them (a definition in use is not removed), and the inserts a
//! changed definition places get their geometry anew.
//!
//! An object's id is its persistent id, chosen by the client that made it
//! (docs/adr/0014, 0026). A deleted object's row is removed, and the same id
//! may be created again (an undone deletion, or "keep mine" over someone
//! else's deletion). So that a version is never given to the same id twice,
//! a created object's version is the commit's data revision: every version
//! an id had is at most the data revision of the commit that wrote it, so
//! the id comes back above all of them, and an edit based on a version from
//! before the deletion is a conflict, never a silent overwrite. A change
//! adds one to the version.

use std::collections::{BTreeMap, HashMap, HashSet};

use kentos_contracts::{
    BlockChange, BlockDefinition, BlockId, CommandEnvelope, CommitResult, ConflictReason,
    EventBlock, EventFeature, EventRecord, FeatureChange, FeatureConflict, FeatureOp, LayerNode,
    LayerNodeType, PROJECT_CHANGES, PROJECT_CHANGES_DIMENSION_SETS_VERSION,
    PROJECT_CHANGES_VERSION, PROJECT_META_KEY, ProjectChanges, ProjectPermission, ProjectSettings,
    block_key,
};
use kentos_formats::blocks::Placing;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::{self, ProjectAccess, not_found};
use crate::blocks;
use crate::cad::{PROJECTION_VERSION, Stored, to_stored, to_stored_block};
use crate::error::{AppError, AppResult};
use crate::idempotency;
use crate::projects::{
    FEATURE_COLUMNS, FeatureRow, check_address_locators, check_name, check_networks,
    check_services, check_sheets, check_srid, check_tree, find_layer, gone, record,
};

/// Most object changes one command may carry; larger sets go in several commands.
pub const MAX_CHANGES: usize = kentos_contracts::processing_job::PROJECT_CHANGES_MAX;

fn parse_uuid(text: &str, what: &str) -> AppResult<Uuid> {
    Uuid::parse_str(text).map_err(|_| AppError::invalid(format!("{what} bir UUID değil: {text}")))
}

fn expected(envelope: &CommandEnvelope, key: &str) -> AppResult<Option<i64>> {
    envelope
        .expected_versions
        .get(key)
        .map(|v| {
            v.parse::<i64>().map_err(|_| {
                AppError::invalid_at(
                    format!("expectedVersions[{key}]"),
                    format!("expectedVersions[{key}] bir tamsayı değil: {v}"),
                )
            })
        })
        .transpose()
}

/// The layer an object may be written to: a layer (not a group) that is not locked.
fn writable_layer(tree: &[LayerNode], id: &str) -> AppResult<()> {
    match find_layer(tree, id) {
        Some((n, _)) if n.kind != LayerNodeType::Layer => Err(AppError::invalid(format!(
            "“{}” bir grup; nesne bir katmana yazılır.",
            n.name
        ))),
        Some((n, true)) => Err(AppError::forbidden(format!(
            "“{}” katmanı kilitli. Kilidi Katmanlar panelinden açın.",
            n.name
        ))),
        Some((n, _)) if n.service.is_some() => Err(AppError::invalid(format!(
            "“{}” bir servis katmanı; çizimi servisten gelir, ona nesne yazılmaz.",
            n.name
        ))),
        Some(_) => Ok(()),
        None => Err(AppError::invalid(format!("“{id}” katmanı projede yok."))),
    }
}

/// The layers (not groups) of a tree, in tree order.
fn leaf_ids<'a>(nodes: &'a [LayerNode], out: &mut Vec<&'a str>) {
    for n in nodes {
        match n.kind {
            LayerNodeType::Layer => out.push(&n.id),
            LayerNodeType::Group => leaf_ids(&n.children, out),
        }
    }
}

struct Planned {
    id: Uuid,
    op: FeatureOp,
    stored: Option<Stored>,
}

/// `input` with every `edit` judged as an update, and the bases those edits were made from, by id.
fn split_bases(
    mut input: ProjectChanges,
) -> (ProjectChanges, HashMap<Uuid, kentos_contracts::Entity>) {
    let mut bases = HashMap::new();
    for change in &mut input.features {
        if let FeatureChange::Edit { id, entity, base } = change {
            if let Ok(uid) = Uuid::parse_str(id) {
                bases.insert(uid, (**base).clone());
            }
            *change = FeatureChange::Update {
                id: std::mem::take(id),
                entity: entity.clone(),
            };
        }
    }
    (input, bases)
}

async fn current_copies(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    ids: &[Uuid],
) -> AppResult<HashMap<Uuid, kentos_contracts::FeatureRecord>> {
    let rows: Vec<FeatureRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {FEATURE_COLUMNS} from kentos.feature where tenant_id = $1 and project_id = $2 and id = any($3)"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .bind(ids)
    .fetch_all(&mut **tx)
    .await?;
    rows.into_iter()
        .map(|r| record(r).map(|f| (Uuid::parse_str(&f.id).expect("stored ids are UUIDs"), f)))
        .collect()
}

/// The earlier copies of the records `plan` changes or removes, as far as a Veri girişi's reach needs them
/// (docs/adr/0738 §3), by id: each on a layer whose forms narrow its records, with the attributes its scopes
/// read under the forms as the commit leaves them. A layer whose scopes read more than attributes, or none
/// narrows, keeps none: a Veri girişi then hears of the record's change as before.
async fn earlier_copies(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    settings: &ProjectSettings,
    tree: &[LayerNode],
    plan: &[Planned],
    existing: &HashMap<Uuid, (i64, String)>,
) -> AppResult<BTreeMap<String, crate::events::Earlier>> {
    if settings.forms.is_empty() {
        return Ok(BTreeMap::new());
    }
    let reach = crate::form_guard::reach_with(
        settings,
        tree,
        &[],
        kentos_forms::reach::Caller::through_forms(),
    );
    let mut wanted = Vec::new();
    for p in plan.iter().filter(|p| p.op != FeatureOp::Create) {
        let Some((_, layer)) = existing.get(&p.id) else {
            continue;
        };
        if let kentos_forms::scope::ScopeReads::Fields(fields) = reach.scope_reads(layer) {
            wanted.push((p.id, layer.clone(), fields));
        }
    }
    let ids: Vec<Uuid> = wanted
        .iter()
        .filter(|(_, _, fields)| !fields.is_empty())
        .map(|(id, _, _)| *id)
        .collect();
    let properties: HashMap<Uuid, Value> = if ids.is_empty() {
        HashMap::new()
    } else {
        sqlx::query_as::<_, (Uuid, Value)>(
            "select id, properties from kentos.feature where tenant_id = $1 and project_id = $2 and id = any($3)",
        )
        .bind(access.tenant)
        .bind(access.project)
        .bind(&ids)
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .collect()
    };
    Ok(wanted
        .into_iter()
        .map(|(id, layer, fields)| {
            let stored = properties.get(&id).and_then(Value::as_object);
            let attrs = fields
                .iter()
                .filter_map(|f| {
                    let v = stored?.get(f)?.as_str()?;
                    Some((f.clone(), v.to_owned()))
                })
                .collect();
            let copy = crate::events::Earlier {
                layer,
                fields: fields.into_iter().collect(),
                attrs,
            };
            (id.to_string(), copy)
        })
        .collect())
}

/// The shapes a form's spatial lists read (docs/adr/0759 §2): of each layer in `around`, its objects
/// within the written records' box grown by the list's distance and the projection's tolerance (a
/// GiST lookup: `&&` against a box in the project's own system), or all of them for a nearest list
/// without a tolerance. A record without a shape narrows nothing: nothing is read for it.
async fn spatial_shapes(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    around: &[(String, Option<f64>)],
    written: &[(usize, &str, &kentos_contracts::Entity)],
    srid: u32,
) -> AppResult<HashMap<String, std::sync::Arc<kentos_geometry_core::entity::Shape>>> {
    use kentos_geometry_core::ops::spatial_query::geometry_of;
    let mut boxed: Option<[f64; 4]> = None;
    for (_, _, e) in written {
        let Some(b) = kentos_forms::object_of(e, "", 0.0)
            .shape
            .as_deref()
            .and_then(geometry_of)
            .map(|g| g.bounds)
        else {
            continue;
        };
        boxed = Some(match boxed {
            None => [b.min_x, b.min_y, b.max_x, b.max_y],
            Some([x0, y0, x1, y1]) => [
                x0.min(b.min_x),
                y0.min(b.min_y),
                x1.max(b.max_x),
                y1.max(b.max_y),
            ],
        });
    }
    let mut out = HashMap::new();
    for (layer, reach) in around {
        let rows: Vec<FeatureRow> = match (reach, boxed) {
            (_, None) => continue,
            (None, Some(_)) => sqlx::query_as(sqlx::AssertSqlSafe(format!(
                "select {FEATURE_COLUMNS} from kentos.feature where tenant_id=$1 and project_id=$2 and layer_id=$3"
            )))
            .bind(access.tenant)
            .bind(access.project)
            .bind(layer)
            .fetch_all(&mut **tx)
            .await?,
            (Some(d), Some([x0, y0, x1, y1])) => {
                let pad = d + crate::cad::PROJECTION_TOLERANCE;
                sqlx::query_as(sqlx::AssertSqlSafe(format!(
                    "select {FEATURE_COLUMNS} from kentos.feature where tenant_id=$1 and project_id=$2 and layer_id=$3 \
                     and geom && public.st_makeenvelope($4,$5,$6,$7,$8)"
                )))
                .bind(access.tenant)
                .bind(access.project)
                .bind(layer)
                .bind(x0 - pad)
                .bind(y0 - pad)
                .bind(x1 + pad)
                .bind(y1 + pad)
                .bind(srid as i32)
                .fetch_all(&mut **tx)
                .await?
            }
        };
        for row in rows {
            let r = record(row)?;
            if let Some(shape) = kentos_forms::object_of(&r.entity, "", 0.0).shape {
                out.insert(r.id, shape);
            }
        }
    }
    Ok(out)
}

/// The forms' guard over the commit's object changes (docs/adr/0411 §2, §4): a Veri girişi's
/// (`entry`) change is its record's attributes through its layer's published form; a write to a
/// layer whose form enforces its rules keeps them. The forms judge with the project's variables (the
/// moment's left out), its reading unit and the stored objects of the layers they read.
#[allow(clippy::too_many_arguments)]
async fn guard_forms(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    name: &str,
    settings: &ProjectSettings,
    tree: &[LayerNode],
    changes: &[FeatureChange],
    entry: bool,
    srid: u32,
) -> AppResult<()> {
    let written: Vec<(usize, &str, &kentos_contracts::Entity)> = changes
        .iter()
        .enumerate()
        .filter_map(|(i, c)| match c {
            FeatureChange::Create { id, entity } | FeatureChange::Update { id, entity } => {
                Some((i, id.as_str(), entity))
            }
            FeatureChange::Delete { .. } | FeatureChange::Edit { .. } => None,
        })
        .collect();
    let forms: Vec<&kentos_contracts::ProjectForm> = settings
        .forms
        .iter()
        .filter(|f| !f.releases.is_empty() && (entry || f.enforce))
        .filter(|f| written.iter().any(|(_, _, e)| e.base().layer_id == f.layer))
        .collect();
    if forms.is_empty() {
        if entry && !written.is_empty() {
            return Err(AppError::forbidden(
                "Bu katmanın yayımlanmış formu yok; Veri girişi kayıtları yalnız yayımlanmış formla düzenler.",
            ));
        }
        return Ok(());
    }
    let mut read: Vec<String> = Vec::new();
    for f in &forms {
        if let Some(r) = f.releases.last() {
            for l in crate::form_guard::world_layers(&r.body) {
                if !read.contains(&l) {
                    read.push(l);
                }
            }
        }
    }
    let variables = crate::form_guard::variables_for(name, settings, &access.actor);
    // The spatial lists' layers and how far around the written records their shapes are needed
    // (docs/adr/0759 §2).
    let unit = kentos_expression::Unit::per_metre(settings.unit().per_metre());
    let mut around: Vec<(String, Option<f64>)> = Vec::new();
    for f in &forms {
        if let Some(r) = f.releases.last() {
            for (layer, a) in crate::form_guard::spatial_reach(&r.body, unit) {
                match around.iter_mut().find(|(l, _)| *l == layer) {
                    Some((_, was)) => *was = was.zip(a).map(|(x, y)| x.max(y)),
                    None => around.push((layer, a)),
                }
            }
        }
    }
    let mut shapes: HashMap<String, std::sync::Arc<kentos_geometry_core::entity::Shape>> =
        HashMap::new();
    // A Veri girişi writes only the records its forms reach (docs/adr/0539, 0550).
    let reach = entry.then(|| {
        crate::form_guard::reach_with(
            settings,
            tree,
            &variables,
            kentos_forms::reach::Caller::THROUGH_FORMS,
        )
    });
    // The world the forms read: for a Veri girişi only the records it reaches and the fields it reads, so
    // a value the form refuses or accepts never says whether a record out of its reach exists.
    let mut rows: Vec<(String, String, BTreeMap<String, String>)> = Vec::new();
    if !read.is_empty()
        && let Some(reach) = &reach
    {
        let stored: Vec<crate::projects::FeatureRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "select {} from kentos.feature where tenant_id = $1 and project_id = $2 and layer_id = any($3)",
            crate::projects::FEATURE_COLUMNS
        )))
        .bind(access.tenant)
        .bind(access.project)
        .bind(&read)
        .fetch_all(&mut **tx)
        .await?;
        for row in stored {
            let r = crate::projects::record(row)?;
            if !crate::form_guard::in_reach(reach, tree, &r.entity, &r.id) {
                continue;
            }
            let entity = crate::form_guard::projected(r.entity, reach);
            if around.iter().any(|(l, _)| *l == entity.base().layer_id)
                && let Some(shape) = kentos_forms::object_of(&entity, "", 0.0).shape
            {
                shapes.insert(r.id.clone(), shape);
            }
            rows.push((
                entity.base().layer_id.clone(),
                r.id,
                entity.base().attrs.clone(),
            ));
        }
    } else if !read.is_empty() {
        let stored: Vec<(String, Uuid, Value)> = sqlx::query_as(
            "select layer_id, id, properties from kentos.feature where tenant_id = $1 and project_id = $2 and layer_id = any($3)",
        )
        .bind(access.tenant)
        .bind(access.project)
        .bind(&read)
        .fetch_all(&mut **tx)
        .await?;
        for (layer, id, props) in stored {
            let attrs = props
                .as_object()
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| {
                            let text = v.as_str().map_or_else(|| v.to_string(), str::to_owned);
                            (k.clone(), text)
                        })
                        .collect()
                })
                .unwrap_or_default();
            rows.push((layer, id.to_string(), attrs));
        }
    }
    if reach.is_none() && !around.is_empty() {
        shapes = spatial_shapes(tx, access, &around, &written, srid).await?;
    }
    let spatial: Vec<String> = around.into_iter().map(|(l, _)| l).collect();
    let judge = crate::form_guard::Judge {
        settings,
        tree,
        variables,
        unit,
        world: crate::form_guard::world_of(tree, &rows, &spatial, &shapes),
    };
    let ids: Vec<Uuid> = written
        .iter()
        .filter_map(|(_, id, _)| Uuid::parse_str(id).ok())
        .collect();
    let before = current_copies(tx, access, &ids).await?;
    for (i, id, after) in written {
        let old = Uuid::parse_str(id).ok().and_then(|u| before.get(&u));
        if entry {
            let Some(old) = old else {
                return Err(AppError::forbidden(
                    "Veri girişi nesne oluşturmaz; yalnız var olan kayıtları formla düzenler.",
                ));
            };
            if let Some(reach) = &reach {
                // A record out of reach is not there for it; one it would take out of reach is refused.
                if !crate::form_guard::in_reach(reach, tree, &old.entity, id) {
                    return Err(AppError::not_found("Kayıt bulunamadı."));
                }
                if !crate::form_guard::in_reach(reach, tree, after, id) {
                    return Err(AppError::forbidden(
                        "Bu değişiklik kaydı formun kapsamının dışına çıkarır; kapsamdaki değerlerle yazın ya da proje yöneticisine başvurun.",
                    ));
                }
            }
            judge
                .entry_write(id, &old.entity, after)
                .map_err(AppError::forbidden)?;
        }
        if let Some(why) = judge.enforced(id, old.map(|r| &r.entity), after) {
            return Err(AppError::invalid_at(format!("features[{i}].entity"), why));
        }
    }
    Ok(())
}

/// The envelope names the project it is sent to (`access`), in this tenant.
pub(crate) fn check_target(envelope: &CommandEnvelope, access: &ProjectAccess) -> AppResult<()> {
    if parse_uuid(&envelope.tenant_id, "tenantId")? != access.tenant {
        return Err(AppError::invalid(
            "Komutun kurumu adresteki kurumla aynı değil.",
        ));
    }
    if parse_uuid(&envelope.project_id, "projectId")? != access.project {
        return Err(AppError::invalid(
            "Komutun projesi adresteki projeyle aynı değil.",
        ));
    }
    Ok(())
}

/// Locks the project row for a command (commits, deletion and sharing of one
/// project happen one after another) and returns the caller's access asked
/// again under the lock: whatever changed before the lock was taken counts.
pub(crate) async fn lock(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
) -> AppResult<ProjectAccess> {
    let found: Option<i32> = sqlx::query_scalar(
        "select 1 from kentos.project where tenant_id = $1 and id = $2 for update",
    )
    .bind(access.tenant)
    .bind(access.project)
    .fetch_optional(&mut **tx)
    .await?;
    if found.is_none() {
        return Err(not_found());
    }
    access::recheck(tx, access).await
}

/// One planned change of a definition: which, what happens to it, and for a
/// create or an update its stored name and JSON.
type PlannedBlock = (BlockId, FeatureOp, Option<(String, Value)>);

/// The block definitions a command changes (docs/adr/0144 §5), planned.
struct BlockPlan {
    /// The project's definitions as the command leaves them, in their order.
    after: Vec<BlockDefinition>,
    changes: Vec<PlannedBlock>,
    /// Definitions whose placed objects change: the replaced ones, and every one placing them.
    changed: HashSet<BlockId>,
    /// Definitions removed, with their names and versions before the command.
    removed: Vec<(BlockId, String, i64)>,
}

/// The command's block changes checked against the project's definitions
/// (versions: a difference is a conflict, gathered in `conflicts`), each
/// definition as the server keeps it, and the definitions it leaves checked
/// against the block rules, in the documents' words.
async fn plan_blocks(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: &CommandEnvelope,
    changes: &[BlockChange],
    conflicts: &mut Vec<FeatureConflict>,
) -> AppResult<BlockPlan> {
    let before = blocks::read(tx, access.tenant, access.project, true).await?;
    let known: HashMap<BlockId, (i64, String)> = before
        .iter()
        .map(|s| (s.block.id, (s.version, s.block.name.clone())))
        .collect();
    let mut after: Vec<BlockDefinition> = before.into_iter().map(|s| s.block).collect();
    let mut planned = Vec::with_capacity(changes.len());
    let mut seen = HashSet::new();
    let mut updated = HashSet::new();
    let mut removed = Vec::new();
    for (i, change) in changes.iter().enumerate() {
        let (id, op, block) = match change {
            BlockChange::Create { block } => (block.id, FeatureOp::Create, Some(block)),
            BlockChange::Update { block } => (block.id, FeatureOp::Update, Some(block)),
            BlockChange::Delete { id } => (*id, FeatureOp::Delete, None),
        };
        if !seen.insert(id) {
            return Err(AppError::invalid_at(
                format!("blocks[{i}]"),
                format!("{id} bloğu komutta birden çok kez geçiyor."),
            ));
        }
        let key = block_key(id);
        let actual = known.get(&id).map(|(v, _)| *v);
        let reason = match op {
            FeatureOp::Create => actual.map(|_| ConflictReason::Exists),
            FeatureOp::Update | FeatureOp::Delete => {
                let want = expected(envelope, &key)?.ok_or_else(|| {
                    AppError::invalid_at(
                        format!("expectedVersions[{key}]"),
                        format!("expectedVersions[{key}] eksik."),
                    )
                })?;
                match actual {
                    None => Some(ConflictReason::Deleted),
                    Some(v) if v != want => Some(ConflictReason::Changed),
                    Some(_) => None,
                }
            }
        };
        if let Some(reason) = reason {
            conflicts.push(FeatureConflict {
                id: key.clone(),
                reason,
                expected: envelope.expected_versions.get(&key).cloned(),
                actual: actual.map(|v| v.to_string()),
                current: None,
            });
        }
        let stored = match block {
            Some(b) => Some(to_stored_block(b).map_err(|why| {
                AppError::invalid_at(
                    format!("blocks[{i}]"),
                    format!("Blok değişikliği {} (“{}”): {why}.", i + 1, b.name),
                )
            })?),
            None => None,
        };
        // As the command leaves them, even past a conflict (which stops it later): its objects are checked against these.
        match (op, &stored) {
            (FeatureOp::Create, Some((b, _))) => after.push(b.clone()),
            (FeatureOp::Update, Some((b, _))) => {
                match after.iter_mut().find(|x| x.id == id) {
                    Some(slot) => *slot = b.clone(),
                    None => after.push(b.clone()),
                }
                updated.insert(id);
            }
            (FeatureOp::Delete, _) => {
                after.retain(|x| x.id != id);
                if let Some((version, name)) = known.get(&id) {
                    removed.push((id, name.clone(), *version));
                }
            }
            _ => {}
        }
        planned.push((id, op, stored.map(|(b, v)| (b.name, v))));
    }
    if !changes.is_empty() {
        // A removed definition still placed by one the command leaves: said by name.
        for (id, name, _) in &removed {
            if let Some(holder) = after.iter().find(|b| {
                b.entities
                    .iter()
                    .any(|e| matches!(e, kentos_contracts::Entity::Insert(i) if i.block == *id))
            }) {
                return Err(AppError::invalid_at(
                    "blocks",
                    format!(
                        "“{name}” bloğu “{}” bloğunun içinde kullanılıyor; silinemez.",
                        holder.name
                    ),
                ));
            }
        }
        kentos_contracts::blocks::check(&after, &[]).map_err(|fault| {
            AppError::invalid_at(
                "blocks",
                fault.message(|i| after.get(i).map_or("", |b| b.name.as_str())),
            )
        })?;
    }
    let changed = blocks::affected(&after, &updated);
    Ok(BlockPlan {
        after,
        changes: planned,
        changed,
        removed,
    })
}

pub async fn commit(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<CommitResult> {
    let mut tx = db.scoped(access.scope()).await?;
    let result = commit_in(&mut tx, access, envelope, None, MAX_CHANGES).await?;
    tx.commit().await?;
    Ok(result)
}

/// Validate the persisted cache against the bounded final source graph. This is the
/// shared pure wall/opening/row geometry, not a server edit, solver or identity allocator.
/// Appearance, text and manual row placement remain client-owned input.
#[allow(clippy::too_many_arguments)]
async fn check_dimension_rows(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    sets: &[kentos_contracts::DimensionSet],
    previous: &[kentos_contracts::DimensionSet],
    scale: f64,
    full_recipe: bool,
    features: &[FeatureChange],
    tree: &[LayerNode],
) -> AppResult<()> {
    use kentos_contracts::{
        DimensionSetFace, DimensionSetOpenings, DimensionSetRow, DimensionStyle, Entity, EntityId,
        OpeningAnchor, SubPlace, SubRef,
    };
    use kentos_geometry_core::geom::{opening, wall};
    use kentos_geometry_core::ops::dimension_set::{self as core, Frame};
    use kentos_geometry_core::vec2::Vec2;
    if sets.is_empty() {
        return Ok(());
    }
    let ids: HashSet<Uuid> = sets
        .iter()
        .flat_map(|set| {
            set.sources().into_iter().map(|id| Uuid::from_bytes(id.0)).chain(
                set.members
                    .iter()
                    .map(|member| Uuid::from_bytes(member.uid.0)),
            )
        })
        .collect();
    let records = current_copies(tx, access, &ids.into_iter().collect::<Vec<_>>()).await?;
    let hosts: Vec<String> = sets.iter().map(|set| set.wall.to_text()).collect();
    // The final opening state is needed too: a newly inserted opening may not
    // appear in the old recipe at all. UUID order is deterministic; coincident
    // jamb references are matched by their resolved point, not that order.
    let rows: Vec<FeatureRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {FEATURE_COLUMNS} from kentos.feature where tenant_id=$1 and project_id=$2 and kind='opening' and lower(cad_definition->>'host')=any($3) order by id"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .bind(hosts)
    .fetch_all(&mut **tx)
    .await?;
    let mut by_host: HashMap<EntityId, Vec<(EntityId, kentos_contracts::OpeningFields)>> =
        HashMap::new();
    for row in rows {
        let record = record(row)?;
        let Entity::Opening(entity) = record.entity else {
            continue;
        };
        if let (Some(host), Some(uid)) = (
            EntityId::parse(&entity.opening.host),
            EntityId::parse(&record.id),
        ) {
            by_host.entry(host).or_default().push((uid, entity.opening));
        }
    }
    let p = |point: kentos_contracts::Vec2| Vec2::new(point.x, point.y);
    let mid = |a: Vec2, b: Vec2| Vec2::new(a.x + (b.x - a.x) * 0.5, a.y + (b.y - a.y) * 0.5);
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 + 16. * f64::EPSILON * a.abs().max(b.abs());
    let same_point = |a: kentos_contracts::Vec2, b: Vec2| close(a.x, b.x) && close(a.y, b.y);
    let row = |row| match row {
        DimensionSetRow::Openings => core::Row::Openings,
        DimensionSetRow::Overall => core::Row::Overall,
        DimensionSetRow::Walls => core::Row::Walls,
        DimensionSetRow::Grids => core::Row::Grids,
    };
    let bits = |value: f64| if value == 0. { 0 } else { value.to_bits() };
    let geometric_key = |row: DimensionSetRow, a: Vec2, b: Vec2| {
        (row.name(), [bits(a.x), bits(a.y), bits(b.x), bits(b.y)])
    };
    let source_key = |row: DimensionSetRow, a: &SubRef, b: &SubRef| {
        serde_json::to_string(&(row, a, b)).expect("validated source references")
    };
    let previous: BTreeMap<_, _> = previous.iter().map(|set| (set.id, set)).collect();
    let written: HashMap<_, _> = features
        .iter()
        .enumerate()
        .filter_map(|(i, change)| {
            let id = match change {
                FeatureChange::Create { id, .. }
                | FeatureChange::Update { id, .. }
                | FeatureChange::Edit { id, .. }
                | FeatureChange::Delete { id } => id,
            };
            Some((Uuid::parse_str(id).ok()?, i))
        })
        .collect();
    for (i, set) in sets.iter().enumerate() {
        if set.span.is_some() {
            check_span_rows(
                tx,
                access,
                i,
                set,
                previous.get(&set.id).copied(),
                scale,
                full_recipe,
                &written,
                &records,
                tree,
            )
            .await?;
            continue;
        }
        let Some(record) = records.get(&Uuid::from_bytes(set.wall.0)) else {
            continue;
        };
        let Entity::Wall(entity) = &record.entity else {
            continue;
        };
        let w = &entity.wall;
        let n = w.pts.len();
        // The measured straight edge (docs/adr/0700); a source without one retains its last geometry.
        let edge = match set.edge {
            Some(edge) => edge as usize,
            None if !w.closed && n == 2 => 0,
            None => continue,
        };
        let pts: Vec<Vec2> = w.pts.iter().copied().map(p).collect();
        let bulges = w.bulges.clone().unwrap_or_else(|| vec![0.; n]);
        let spec = kentos_formats::walls::spec(w);
        let Ok(lines) = wall::lines(&pts, w.bulges.as_deref(), &spec) else {
            continue;
        };
        let part = match set.face {
            DimensionSetFace::Left => 0,
            DimensionSetFace::Right => lines.len().saturating_sub(1),
        };
        // A bevelled face has no vertex per path vertex: its places are not the path's.
        let Some(face) = lines.get(part).filter(|line| line.pts.len() == n) else {
            continue;
        };
        let Some((line, ends)) = core::edge_face(&face.pts, &bulges, w.closed, edge) else {
            continue;
        };
        if Frame::new(line).is_err() {
            continue;
        }
        let path = opening::wall_path(&pts, w.bulges.as_deref(), w.closed);
        let mut openings = Vec::new();
        let mut jambs_by_uid = HashMap::new();
        let mut frozen = false;
        let mut stale_source = false;
        if set.rows.contains(&DimensionSetRow::Openings) {
            let sources = by_host.get(&set.wall).map_or(&[][..], Vec::as_slice);
            if sources
                .iter()
                .any(|(_, source)| source.broken.is_some() || source.place.len() != 4)
            {
                continue;
            }
            if sources.len() > kentos_contracts::DIMENSION_SET_MEMBERS_MAX {
                return Err(AppError::invalid_at(
                    format!("project.dimensionSets[{i}].members"),
                    "Kaynağın açıklıkları otomatik ölçü segment sınırını aşıyor.",
                ));
            }
            for (uid, source) in sources {
                if source.broken.is_some() || source.place.len() != 4 {
                    frozen = true;
                    break;
                }
                let anchor = match source.anchor {
                    OpeningAnchor::Start => opening::Anchor::Start,
                    OpeningAnchor::Center => opening::Anchor::Center,
                    OpeningAnchor::End => opening::Anchor::End,
                };
                let (lo, hi) = opening::span(source.at, anchor, source.width);
                let Some(jambs) = path
                    .as_ref()
                    .and_then(|path| opening::jambs(path, &spec, lo, hi))
                else {
                    frozen = true;
                    break;
                };
                // A moved host requires its opening's own cache in the same
                // transaction; otherwise even correctly referenced dimensions
                // could follow stale jambs which only look healthy.
                if !source
                    .place
                    .iter()
                    .copied()
                    .zip(jambs)
                    .all(|(a, b)| same_point(a, b))
                {
                    stale_source = true;
                }
                // Another edge's opening belongs to that edge's chain.
                if !core::on_edge(line, &jambs, spec.width) {
                    continue;
                }
                jambs_by_uid.insert(*uid, jambs);
                openings.push(core::Opening {
                    source: openings.len() + 1,
                    jambs,
                });
            }
        }
        if frozen {
            continue;
        }
        let Ok(spans) = core::plan(
            line,
            part,
            ends,
            match set.face {
                DimensionSetFace::Left => core::Face::Left,
                DimensionSetFace::Right => core::Face::Right,
            },
            &openings,
            match set.openings {
                DimensionSetOpenings::Ends => core::Openings::Ends,
                DimensionSetOpenings::Centers => core::Openings::Centers,
            },
            &set.rows.iter().copied().map(row).collect::<Vec<_>>(),
            p(set.at),
            set.gap_mm * scale / 1000.,
            &set.row_places
                .iter()
                .map(|place| (row(place.row), p(place.at)))
                .collect::<Vec<_>>(),
        ) else {
            continue;
        };
        if stale_source {
            return Err(AppError::invalid_at(
                format!("project.dimensionSets[{i}].wall"),
                "Kaynak açıklığın pervazları güncel duvar geometrisiyle uyuşmuyor. Kaynak duvarı, açıklıklarını ve ölçü üyelerini aynı işlemde yeniden üretin.",
            ));
        }
        let resolve = |reference: &SubRef| -> Option<Vec2> {
            if reference.uid == set.wall {
                return match reference.place {
                    SubPlace::Vertex {
                        part: q,
                        ring: 0,
                        index,
                    } if q as usize == part && ends.contains(&(index as usize)) => {
                        Some(line[usize::from(index as usize == ends[1])])
                    }
                    _ => None,
                };
            }
            let jambs = jambs_by_uid.get(&reference.uid)?;
            match (&set.openings, reference.place) {
                (DimensionSetOpenings::Centers, SubPlace::Center) => {
                    Some(mid(mid(jambs[0], jambs[1]), mid(jambs[2], jambs[3])))
                }
                (
                    DimensionSetOpenings::Ends,
                    SubPlace::Vertex {
                        part: 0,
                        ring: 0,
                        index,
                    },
                ) if (set.face == DimensionSetFace::Left && [0, 2].contains(&index))
                    || (set.face == DimensionSetFace::Right && [1, 3].contains(&index)) =>
                {
                    Some(jambs[index as usize])
                }
                _ => None,
            }
        };
        let hidden: HashSet<_> = set
            .suppressed
            .iter()
            .filter_map(|segment| {
                Some(geometric_key(
                    segment.row,
                    resolve(&segment.a)?,
                    resolve(&segment.b)?,
                ))
            })
            .collect();
        let mut active = HashMap::new();
        let old: HashMap<_, _> = previous
            .get(&set.id)
            .into_iter()
            .flat_map(|old| &old.members)
            .map(|member| (source_key(member.row, &member.a, &member.b), member.uid))
            .collect();
        for (k, member) in set.members.iter().enumerate() {
            let Some((a, b)) = resolve(&member.a).zip(resolve(&member.b)) else {
                return Err(AppError::invalid_at(
                    format!("project.dimensionSets[{i}].members[{k}]"),
                    "Ölçü üyesi seçilen yüzün güncel duvar/açıklık kaynağını ölçmeli.",
                ));
            };
            let key = source_key(member.row, &member.a, &member.b);
            if old.get(&key).is_some_and(|uid| *uid != member.uid) {
                return Err(AppError::invalid_at(
                    format!("project.dimensionSets[{i}].members[{k}].uid"),
                    "Değişmeyen aktif kaynak çiftinin ölçü kimliği korunmalı.",
                ));
            }
            if active
                .insert(geometric_key(member.row, a, b), (k, member))
                .is_some()
            {
                return Err(AppError::invalid_at(
                    format!("project.dimensionSets[{i}].members[{k}]"),
                    "Aynı sıradaki kaynak aralığı birden fazla aktif üyeyle yinelenemez.",
                ));
            }
        }
        // A span measuring the same two places as one drawn before it, in the rows' order, is not drawn again: an edge
        // without openings has its openings row its whole length, the overall dimension once (the desktop's rule,
        // docs/adr/0695's 27 October addendum). What the user removed is not drawn, so the other row's stays.
        let mut drawn: Vec<[(usize, kentos_geometry_core::ops::subref::Place); 2]> = Vec::new();
        for span in spans {
            let row = match span.row {
                core::Row::Openings => DimensionSetRow::Openings,
                core::Row::Overall => DimensionSetRow::Overall,
                core::Row::Walls => DimensionSetRow::Walls,
                core::Row::Grids => DimensionSetRow::Grids,
            };
            let key = geometric_key(row, span.a.at, span.b.at);
            if hidden.contains(&key) && !active.contains_key(&key) {
                continue;
            }
            let places = [(span.a.source, span.a.place), (span.b.source, span.b.place)];
            if drawn.contains(&places) {
                continue;
            }
            drawn.push(places);
            let Some((k, member)) = active.remove(&key) else {
                return Err(AppError::invalid_at(
                    format!("project.dimensionSets[{i}].members"),
                    "Güncel kaynağın ölçü üyeliği eksik. Yeni ve kaldırılan açıklık aralıklarını tam tarifle aynı işlemde yeniden üretin.",
                ));
            };
            let Some(record) = records.get(&Uuid::from_bytes(member.uid.0)) else {
                continue;
            };
            let Entity::Dimension(d) = &record.entity else {
                continue;
            };
            let field = if !same_point(d.a, span.a.at) {
                Some("a")
            } else if !same_point(d.b, span.b.at) {
                Some("b")
            } else if !close(d.offset, span.offset) {
                Some("offset")
            } else if d.style != Some(DimensionStyle::Linear)
                || !d.angle.is_some_and(|a| close(a, span.angle))
                || d.c.is_some()
                || d.za.is_some()
                || d.zb.is_some()
            {
                Some("style")
            } else if !kentos_formats::dimension_spans::scales_agree(
                set,
                d,
                span.offset,
                Frame::new(line).map_or(0.0, |f| f.offset(span.at)),
                scale,
            ) {
                // The sheet scales' representations (docs/adr/0730 §5).
                Some("annotative")
            } else {
                None
            };
            let Some(field) = field else { continue };
            if !full_recipe {
                return Err(AppError::invalid_at(
                    "project.dimensionSets",
                    "Kaynak veya çizim ölçeği ölçü geometrisini değiştiriyor. Tam ölçü tarifini ve değişen üyelerini project.changes v2 ile aynı işlemde gönderin.",
                ));
            }
            let written = written.get(&Uuid::from_bytes(member.uid.0)).copied();
            let Some(written) = written else {
                return Err(AppError::invalid_at(
                    format!("project.dimensionSets[{i}].members[{k}].uid"),
                    "Güncel kaynağın veya çizim ölçeğinin değiştirdiği ölçü üyesi aynı işlemde yazılmalı.",
                ));
            };
            return Err(AppError::invalid_at(
                format!("features[{written}].entity.{field}"),
                "Ölçü geometrisi güncel kaynak ve sıra tarifiyle uyuşmuyor. Üyeleri tam tariften yeniden üretin.",
            ));
        }
        if !active.is_empty() {
            return Err(AppError::invalid_at(
                format!("project.dimensionSets[{i}].members"),
                "Kaynakta artık bulunmayan ölçü aralıkları aynı işlemde kaldırılmalı.",
            ));
        }
    }
    Ok(())
}

/// Whether a layer is shown: it and every group above it open and not frozen; an unknown one is (the desktop's rule).
fn layer_shown(tree: &[LayerNode], id: &str) -> bool {
    fn walk(nodes: &[LayerNode], id: &str, on: bool) -> Option<bool> {
        for n in nodes {
            let here = on && n.visible && n.frozen != Some(true);
            if n.id == id {
                return Some(here);
            }
            if let Some(found) = walk(&n.children, id, here) {
                return Some(found);
            }
        }
        None
    }
    walk(tree, id, true).unwrap_or(true)
}

/// A façade's or a space's members against the shared rule (docs/adr/0729 §8): its sources, every grid, the walls and
/// openings in its box and its sources' openings, as this transaction leaves them, planned as the desktop plans them
/// (`kentos_formats::dimension_spans`). Each planned interval has its member, an unchanged pair keeps its identity, a
/// member measures what the plan measures; a source that cannot be planned keeps its last geometry (a broken recipe).
#[allow(clippy::too_many_arguments)]
async fn check_span_rows(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    i: usize,
    set: &kentos_contracts::DimensionSet,
    previous: Option<&kentos_contracts::DimensionSet>,
    scale: f64,
    full_recipe: bool,
    written: &HashMap<Uuid, usize>,
    records: &HashMap<Uuid, kentos_contracts::FeatureRecord>,
    tree: &[LayerNode],
) -> AppResult<()> {
    use kentos_contracts::{DimensionSetSegment, DimensionStyle, Entity, EntityId, SubRef};
    use kentos_formats::dimension_spans as spans;
    let by_record = |id: EntityId| records.get(&Uuid::from_bytes(id.0)).map(|r| &r.entity);
    let Some(reach) = spans::reach(set, &by_record) else {
        return Ok(());
    };
    let sources: Vec<String> = set.sources().iter().map(|s| s.to_text()).collect();
    let rows: Vec<FeatureRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {FEATURE_COLUMNS} from kentos.feature where tenant_id=$1 and project_id=$2 and (kind='grid' \
         or (kind in ('wall','opening') and (geom is null or geom && public.st_makeenvelope($3,$4,$5,$6,srid))) \
         or (kind='opening' and lower(cad_definition->>'host')=any($7))) order by id"
    )))
    .bind(access.tenant)
    .bind(access.project)
    .bind(reach[0])
    .bind(reach[1])
    .bind(reach[2])
    .bind(reach[3])
    .bind(&sources)
    .fetch_all(&mut **tx)
    .await?;
    let found: Vec<(EntityId, kentos_contracts::FeatureRecord)> = rows
        .into_iter()
        .map(|row| {
            let r = record(row)?;
            let id = EntityId::parse(&r.id).ok_or_else(|| AppError::invalid("Nesne kimliği UUID değil."))?;
            Ok((id, r))
        })
        .collect::<AppResult<_>>()?;
    let near: Vec<spans::Near<'_>> = found
        .iter()
        .map(|(id, r)| spans::Near {
            id: *id,
            entity: &r.entity,
            shown: layer_shown(tree, &r.entity.base().layer_id),
        })
        .collect();
    let find = |id: EntityId| {
        by_record(id).or_else(|| found.iter().find(|(f, _)| *f == id).map(|(_, r)| &r.entity))
    };
    let Ok(planned) = spans::plan(set, &find, &near, scale) else {
        return Ok(());
    };
    let reference = |point: &kentos_geometry_core::ops::dimension_set::Point| SubRef {
        uid: planned.sources[point.source],
        place: spans::contract_place(point.place),
        path: Vec::new(),
        broken: None,
    };
    let key = |segment: &DimensionSetSegment| serde_json::to_string(segment).expect("finite source places");
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 + 16. * f64::EPSILON * a.abs().max(b.abs());
    let at = |p: kentos_contracts::Vec2, q: kentos_geometry_core::vec2::Vec2| close(p.x, q.x) && close(p.y, q.y);
    let old: HashMap<String, EntityId> = previous
        .into_iter()
        .flat_map(|old| &old.members)
        .map(|m| (key(&m.segment()), m.uid))
        .collect();
    let mut members: HashMap<String, (usize, &kentos_contracts::DimensionSetMember)> = HashMap::new();
    for (k, member) in set.members.iter().enumerate() {
        let segment = member.segment();
        if old.get(&key(&segment)).is_some_and(|uid| *uid != member.uid) {
            return Err(AppError::invalid_at(
                format!("project.dimensionSets[{i}].members[{k}].uid"),
                "Değişmeyen aktif kaynak çiftinin ölçü kimliği korunmalı.",
            ));
        }
        if members.insert(key(&segment), (k, member)).is_some() {
            return Err(AppError::invalid_at(
                format!("project.dimensionSets[{i}].members[{k}]"),
                "Aynı sıradaki kaynak aralığı birden fazla aktif üyeyle yinelenemez.",
            ));
        }
    }
    for span in &planned.spans {
        let segment = DimensionSetSegment {
            row: spans::contract_row(span.row),
            a: reference(&span.a),
            b: reference(&span.b),
        };
        let Some((k, member)) = members.remove(&key(&segment)) else {
            return Err(AppError::invalid_at(
                format!("project.dimensionSets[{i}].members"),
                "Güncel kaynakların ölçü üyeliği eksik. Cephenin ya da mahalin ölçülerini tam tariften aynı işlemde yeniden üretin.",
            ));
        };
        let Some(Entity::Dimension(d)) = records.get(&Uuid::from_bytes(member.uid.0)).map(|r| &r.entity) else {
            continue;
        };
        let field = if !at(d.a, span.a.at) {
            Some("a")
        } else if !at(d.b, span.b.at) {
            Some("b")
        } else if !close(d.offset, span.offset) {
            Some("offset")
        } else if d.style != Some(DimensionStyle::Linear)
            || !d.angle.is_some_and(|a| close(a, span.angle))
            || d.c.is_some()
            || d.za.is_some()
            || d.zb.is_some()
        {
            Some("style")
        } else if !spans::scales_agree(set, d, span.offset, spans::row_delta(&planned, span.at), scale) {
            // The sheet scales' representations (docs/adr/0730 §5).
            Some("annotative")
        } else {
            None
        };
        let Some(field) = field else { continue };
        if !full_recipe {
            return Err(AppError::invalid_at(
                "project.dimensionSets",
                "Kaynak veya çizim ölçeği ölçü geometrisini değiştiriyor. Tam ölçü tarifini ve değişen üyelerini project.changes v2 ile aynı işlemde gönderin.",
            ));
        }
        let Some(w) = written.get(&Uuid::from_bytes(member.uid.0)).copied() else {
            return Err(AppError::invalid_at(
                format!("project.dimensionSets[{i}].members[{k}].uid"),
                "Güncel kaynağın veya çizim ölçeğinin değiştirdiği ölçü üyesi aynı işlemde yazılmalı.",
            ));
        };
        return Err(AppError::invalid_at(
            format!("features[{w}].entity.{field}"),
            "Ölçü geometrisi güncel kaynak ve sıra tarifiyle uyuşmuyor. Üyeleri tam tariften yeniden üretin.",
        ));
    }
    if !members.is_empty() {
        return Err(AppError::invalid_at(
            format!("project.dimensionSets[{i}].members"),
            "Kaynakta artık bulunmayan ölçü aralıkları aynı işlemde kaldırılmalı.",
        ));
    }
    Ok(())
}

/// [`commit`] inside a transaction the caller owns and commits (docs/adr/0559 §2: a server job's commit and its
/// finish are one transaction, so a lost lease takes the commit back). `at_revision`: the project must be at this data
/// revision under the lock, else nothing is written and the answer is a conflict with the project's revision.
/// `max_changes`: the most object and block changes it takes ([`MAX_CHANGES`] for a request; a job's own commit takes
/// more, docs/adr/0561 §1).
pub(crate) async fn commit_in(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
    at_revision: Option<i64>,
    max_changes: usize,
) -> AppResult<CommitResult> {
    if envelope.command_name != PROJECT_CHANGES {
        return Err(AppError::invalid(format!(
            "Bilinmeyen komut: {}",
            envelope.command_name
        )));
    }
    if ![
        PROJECT_CHANGES_VERSION,
        PROJECT_CHANGES_DIMENSION_SETS_VERSION,
    ]
    .contains(&envelope.version)
    {
        return Err(AppError::invalid(format!(
            "{PROJECT_CHANGES} komutunun {} sürümü desteklenmiyor (desteklenen: {PROJECT_CHANGES_VERSION}, {PROJECT_CHANGES_DIMENSION_SETS_VERSION}).",
            envelope.version
        )));
    }
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let key = envelope.idempotency_key.as_str();
    let input: ProjectChanges = serde_json::from_value(envelope.input.clone())
        .map_err(|e| AppError::invalid(format!("Komut girdisi okunamadı: {e}")))?;
    if envelope.version == PROJECT_CHANGES_VERSION
        && envelope
            .input
            .get("project")
            .and_then(|p| p.get("dimensionSets"))
            .is_some()
    {
        return Err(AppError::invalid_at(
            "project.dimensionSets",
            "Mimari ölçü dizileri project.changes sürüm 2 gerektirir; sürüm 1 bu alanı taşıyamaz.",
        ));
    }
    // An `edit` is an update made from the client's last known copy (docs/adr/0648 §1): judged as an update, its base
    // kept for the merge a moved version asks for.
    let (input, bases) = split_bases(input);
    if input.features.len() + input.blocks.len() > max_changes {
        return Err(AppError::invalid(format!(
            "Bir komutta en çok {max_changes} nesne değişikliği olabilir; değişiklikleri parçalara bölün."
        )));
    }

    // 1. Lock the project: commits of one project happen one after another.
    let now = lock(tx, access).await?;
    // 2. A committed request keeps its answer even if later commits add source ownership,
    // archive the project or otherwise change what a new write would be allowed to do.
    let text = idempotency::request_text(&envelope);
    if let Some(mut result) =
        idempotency::earlier::<CommitResult>(tx, &now, &envelope, &text).await?
    {
        result.replayed = true;
        return Ok(result);
    }
    let project = now.project;
    let (srid, layers, meta_version, data_revision, old_name, storage, stored_dimension_sets): (i32, Value, i64, i64, String, String, Option<Value>) = sqlx::query_as(
        "select srid, layers, meta_version, data_revision, name, storage, dimension_sets from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(now.tenant)
    .bind(project)
    .fetch_one(&mut **tx)
    .await?;
    let stored_dimension_sets: Vec<kentos_contracts::DimensionSet> = stored_dimension_sets
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| AppError::invalid(format!("Mimari ölçü dizileri okunamadı: {e}")))?
        .unwrap_or_default();
    if input
        .project
        .as_ref()
        .is_none_or(|patch| patch.dimension_sets.is_none())
        && !stored_dimension_sets.is_empty()
    {
        let mut touches =
            kentos_contracts::dimension_sets_touched(&stored_dimension_sets, &input.features);
        if !touches {
            // Deleting/rehosting an old opening can affect a broken recipe even
            // when that opening has no active member in its cached source list.
            let ids: Vec<Uuid> = input
                .features
                .iter()
                .filter_map(|change| {
                    let id = match change {
                        FeatureChange::Create { id, .. }
                        | FeatureChange::Update { id, .. }
                        | FeatureChange::Edit { id, .. }
                        | FeatureChange::Delete { id } => id,
                    };
                    Uuid::parse_str(id).ok()
                })
                .collect();
            let hosts: Vec<String> = stored_dimension_sets
                .iter()
                .map(|set| set.wall.to_text())
                .collect();
            touches = sqlx::query_scalar("select exists(select 1 from kentos.feature where tenant_id=$1 and project_id=$2 and id=any($3) and kind='opening' and cad_definition->>'host'=any($4))")
                .bind(access.tenant).bind(project).bind(&ids).bind(&hosts).fetch_one(&mut **tx).await?;
        }
        if touches {
            return Err(AppError::invalid_at(
                "project.dimensionSets",
                "Bu değişiklik otomatik ölçü dizisinin kaynağına veya üyesine dokunuyor. Tarifi ve bağlı nesneleri project.changes v2 ile aynı işlemde gönderin.",
            ));
        }
    }
    // A file project's content is its revisions (docs/adr/0031): objects are not written one by one.
    if storage == "file" {
        return Err(AppError::invalid(format!(
            "“{old_name}” projesi dosya olarak saklanıyor; nesneler tek tek yazılmaz. Çizimi kaydedip yeni bir dosya revizyonu olarak yükleyin (project.file.commit)."
        )));
    }
    // The version a created object gets: this commit's data revision (the module's comment says why).
    let created_version = data_revision + 1;

    if now.deleted {
        return Err(gone(&now.name));
    }
    if now.archived {
        return Err(access::archived(&now.name));
    }
    if let Some(at) = at_revision
        && data_revision != at
    {
        return Err(AppError::Conflict {
            message: format!(
                "Proje {at}. revizyonda değil ({data_revision}); değişiklikler yazılmadı."
            ),
            conflicts: Vec::new(),
            revision: Some(data_revision),
        });
    }
    // 3. The rights, as they are now. Block definitions are the drawing's content, as its objects.
    // A Veri girişi (`forms.use` without `feature.write`) changes existing records' attributes
    // through a published form alone (docs/adr/0411 §2): judged with the forms below.
    let entry = !now.allows(ProjectPermission::FeatureWrite)
        && now.allows(ProjectPermission::FormsUse)
        && input.blocks.is_empty()
        && input.project.is_none();
    if entry {
        if let Some(i) = input
            .features
            .iter()
            .position(|c| !matches!(c, FeatureChange::Update { .. }))
        {
            return Err(AppError::forbidden(format!(
                "Veri girişi nesne oluşturmaz ve silmez; yalnız var olan kayıtların özniteliklerini formla düzenler (değişiklik {}).",
                i + 1
            )));
        }
    } else if !input.features.is_empty() || !input.blocks.is_empty() {
        now.require(ProjectPermission::FeatureWrite)?;
    }
    if input.project.is_some() {
        now.require(ProjectPermission::Edit)?;
    }
    let access = &now;
    // A Veri girişi writes what it reads (docs/adr/0411 §6): the attributes its forms do not show
    // are kept as stored.
    let reading = crate::projects::reading_for(tx, access).await?;
    let input = match &reading {
        Some(reading) if entry => {
            let ids: Vec<Uuid> = input
                .features
                .iter()
                .filter_map(|c| match c {
                    FeatureChange::Update { id, .. } => Uuid::parse_str(id).ok(),
                    _ => None,
                })
                .collect();
            let stored = current_copies(tx, access, &ids).await?;
            let mut input = input;
            for change in &mut input.features {
                if let FeatureChange::Update { id, entity } = change
                    && let Some(before) = Uuid::parse_str(id).ok().and_then(|u| stored.get(&u))
                {
                    *entity =
                        crate::form_guard::merged(entity.clone(), &before.entity, &reading.reach);
                }
            }
            input
        }
        _ => input,
    };
    let mut input = input;

    // 4. Plan and check: layer tree after the patch, stored forms, versions.
    let current_tree: Vec<LayerNode> = serde_json::from_value(layers)
        .map_err(|e| AppError::invalid(format!("Proje katmanları okunamadı: {e}")))?;
    let patch = input.project.clone().unwrap_or_default();
    let tree = patch.layers.clone().unwrap_or_else(|| current_tree.clone());
    let new_srid = patch
        .settings
        .as_ref()
        .map(|s| s.srid)
        .unwrap_or(srid as u32);
    if let Some(name) = &patch.name {
        check_name(name)?;
    }
    if patch.layers.is_some() || patch.active_layer.is_some() {
        let active = match &patch.active_layer {
            Some(a) => a.clone(),
            None => {
                sqlx::query_scalar(
                    "select active_layer from kentos.project where tenant_id = $1 and id = $2",
                )
                .bind(access.tenant)
                .bind(project)
                .fetch_one(&mut **tx)
                .await?
            }
        };
        check_tree(&tree, &active)?;
    }
    // A layer's service names one of the project's connections (docs/adr/0208 §2): checked with the settings after
    // the patch, the stored ones when it brings none.
    if patch.layers.is_some() || patch.settings.is_some() {
        let connections = match &patch.settings {
            Some(s) => s.connections.clone(),
            None => {
                let stored: Value = sqlx::query_scalar(
                    "select settings from kentos.project where tenant_id = $1 and id = $2",
                )
                .bind(access.tenant)
                .bind(project)
                .fetch_one(&mut **tx)
                .await?;
                stored
                    .get("connections")
                    .cloned()
                    .map(serde_json::from_value::<Vec<kentos_contracts::ServiceConnection>>)
                    .transpose()
                    .map_err(|e| {
                        AppError::invalid(format!("Projenin bağlantıları okunamadı: {e}"))
                    })?
                    .unwrap_or_default()
            }
        };
        check_services(&tree, &connections)?;
    }
    // The settings as they are, and as the patch leaves them: the forms' permissions and rules
    // (docs/adr/0411 §3).
    let stored_settings: ProjectSettings = {
        let stored: Value = sqlx::query_scalar(
            "select settings from kentos.project where tenant_id = $1 and id = $2",
        )
        .bind(access.tenant)
        .bind(project)
        .fetch_one(&mut **tx)
        .await?;
        serde_json::from_value(stored)
            .map_err(|e| AppError::invalid(format!("Projenin ayarları okunamadı: {e}")))?
    };
    if let Some(settings) = &patch.settings {
        for need in crate::form_guard::settings_needs(&stored_settings, settings) {
            access.require(need)?;
        }
        if let Some((path, why)) = crate::form_guard::settings_problem(settings) {
            return Err(AppError::invalid_at(path, why));
        }
        if let Some(why) = crate::form_guard::releases_problem(&stored_settings, settings) {
            return Err(AppError::invalid_at("project.settings.forms", why));
        }
    }
    // A project's networks keep their rules (docs/adr/0209 §2); the layers they name need not exist.
    if let Some(settings) = &patch.settings {
        check_networks(&settings.networks, "project.settings.networks")?;
        // So does its address data (docs/adr/0229 §2).
        check_address_locators(
            &settings.address_locators,
            "project.settings.addressLocators",
        )?;
        // And the profiles it carries inside its file (docs/adr/0735).
        crate::projects::check_profile_files(
            &settings.profile_files,
            "project.settings.profileFiles",
        )?;
        // A CAD work is a CAD project's (docs/adr/0801).
        crate::projects::check_cad_work(settings, "project.settings.cadWork")?;
    }
    // The sheet book keeps the file format's rules (docs/adr/0251).
    if let Some(Some(sheets)) = &patch.sheets {
        check_sheets(Some(sheets), "project.sheets")?;
    }
    // So do the associative arrays, but for their blocks and objects (docs/adr/0300 §5).
    if let Some(arrays) = &patch.arrays {
        crate::projects::check_arrays(arrays, "project.arrays")?;
    }
    if let Some(sets) = &patch.dimension_sets {
        crate::projects::check_dimension_sets(sets, "project.dimensionSets")?;
    }
    // So do the kitchen runs, but for the objects and walls they name (docs/adr/0727 §5).
    if let Some(runs) = &patch.casework {
        crate::projects::check_casework(runs, "project.casework")?;
    }
    // So do the constraints, but for the objects they name (docs/adr/0336 §3).
    if let Some(set) = &patch.constraints {
        crate::projects::check_constraints(set, "project.constraints")?;
    }
    if new_srid != srid as u32 {
        check_srid(tx, new_srid).await?;
    }
    let mut conflicts = Vec::new();
    if input.project.is_some() {
        match expected(&envelope, PROJECT_META_KEY)? {
            Some(v) if v == meta_version => {}
            Some(v) => conflicts.push(FeatureConflict {
                id: PROJECT_META_KEY.into(),
                reason: ConflictReason::Project,
                expected: Some(v.to_string()),
                actual: Some(meta_version.to_string()),
                current: None,
            }),
            None => {
                return Err(AppError::invalid_at(
                    "expectedVersions[@project]",
                    "Proje bilgisi değişikliği expectedVersions[\"@project\"] ister.",
                ));
            }
        }
    }

    // Block definitions (docs/adr/0144 §5): the project's, and as this command leaves them.
    let block_plan = plan_blocks(tx, access, &envelope, &input.blocks, &mut conflicts).await?;
    let placing = Placing::new(&block_plan.after);

    let mut plan = Vec::with_capacity(input.features.len());
    let mut seen = HashSet::new();
    for (i, change) in input.features.iter().enumerate() {
        let (id, op, entity) = match change {
            FeatureChange::Create { id, entity } => (id, FeatureOp::Create, Some(entity)),
            FeatureChange::Update { id, entity } => (id, FeatureOp::Update, Some(entity)),
            FeatureChange::Delete { id } => (id, FeatureOp::Delete, None),
            FeatureChange::Edit { .. } => unreachable!("edits are judged as updates (split_bases)"),
        };
        let id = parse_uuid(id, "nesne kimliği").map_err(|e| e.at(format!("features[{i}].id")))?;
        if !seen.insert(id) {
            return Err(AppError::invalid_at(
                format!("features[{i}].id"),
                format!("{id} nesnesi komutta birden çok kez geçiyor."),
            ));
        }
        let stored = match entity {
            Some(e) => {
                if let kentos_contracts::Entity::Insert(x) = e
                    && !block_plan.after.iter().any(|b| b.id == x.block)
                {
                    return Err(AppError::invalid_at(
                        format!("features[{i}].entity"),
                        format!(
                            "Değişiklik {} ({id}): yerleştirilen blok projede tanımlı değil.",
                            i + 1
                        ),
                    ));
                }
                let s = to_stored(e, new_srid, &placing).map_err(|why| {
                    AppError::invalid_at(
                        format!("features[{i}].entity"),
                        format!("Değişiklik {} ({id}): {why}.", i + 1),
                    )
                })?;
                writable_layer(&tree, &s.layer_id)?;
                Some(s)
            }
            None => None,
        };
        plan.push(Planned { id, op, stored });
    }
    let ids: Vec<Uuid> = plan.iter().map(|p| p.id).collect();
    let existing: HashMap<Uuid, (i64, String)> = sqlx::query_as::<_, (Uuid, i64, String)>(
        "select id, version, layer_id from kentos.feature where tenant_id = $1 and project_id = $2 and id = any($3) for update",
    )
    .bind(access.tenant)
    .bind(project)
    .bind(&ids)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|(id, v, l)| (id, (v, l)))
    .collect();
    // A record out of a Veri girişi's reach is not there for it (docs/adr/0539, 0550 §2): a change to one
    // is a 404 before any version is compared, so neither its version nor its copy is ever told.
    let hidden: HashSet<Uuid> = match &reading {
        Some(reading) if entry => current_copies(tx, access, &ids)
            .await?
            .into_iter()
            .filter(|(_, r)| !reading.sees(r))
            .map(|(id, _)| id)
            .collect(),
        _ => HashSet::new(),
    };
    let mut conflicting_ids = Vec::new();
    let mut to_merge: Vec<Uuid> = Vec::new();
    for p in &plan {
        let key = p.id.to_string();
        if hidden.contains(&p.id) {
            return Err(AppError::not_found("Kayıt bulunamadı."));
        }
        let actual = existing.get(&p.id).map(|(v, _)| *v);
        let reason = match p.op {
            FeatureOp::Create => actual.map(|_| ConflictReason::Exists),
            FeatureOp::Update | FeatureOp::Delete => {
                let want = expected(&envelope, &key)?.ok_or_else(|| {
                    AppError::invalid_at(
                        format!("expectedVersions[{key}]"),
                        format!("expectedVersions[{key}] eksik."),
                    )
                })?;
                match actual {
                    None => Some(ConflictReason::Deleted),
                    // Someone saved a newer version of an object edited from a known copy: merged below, if it can be;
                    // a Veri girişi's too, over the fields it reads (docs/adr/0737 §1).
                    Some(v)
                        if v != want && p.op == FeatureOp::Update && bases.contains_key(&p.id) =>
                    {
                        to_merge.push(p.id);
                        None
                    }
                    Some(v) if v != want => Some(ConflictReason::Changed),
                    Some(_) => None,
                }
            }
        };
        if let Some(reason) = reason {
            conflicting_ids.push(p.id);
            conflicts.push(FeatureConflict {
                id: key.clone(),
                reason,
                expected: envelope.expected_versions.get(&key).cloned(),
                actual: actual.map(|v| v.to_string()),
                current: None,
            });
        } else if p.op != FeatureOp::Create {
            // Changing or deleting takes the object off its current layer, which must not be locked either.
            // A layer this command's tree removes is judged as it was: its objects go with it.
            let layer = &existing[&p.id].1;
            let judged = if find_layer(&tree, layer).is_some() {
                &tree
            } else {
                &current_tree
            };
            writable_layer(judged, layer)?;
        }
    }
    // The merges (docs/adr/0648 §1): the stored copy and the client's, field by field against its base; a clean merge is
    // what this command writes, fields both changed differently stay a conflict. A Veri girişi's base and copy are as it
    // reads them: the fields its forms hide come from the stored copy for both (docs/adr/0737 §1); the forms' guard below
    // judges the merge as it judges its write.
    let mut merged_ids: HashSet<Uuid> = HashSet::new();
    if !to_merge.is_empty() {
        let stored_copies = current_copies(tx, access, &to_merge).await?;
        for id in &to_merge {
            let key = id.to_string();
            let Some(i) = plan.iter().position(|p| p.id == *id) else {
                continue;
            };
            let merge = match (stored_copies.get(id), bases.get(id), &input.features[i]) {
                (Some(theirs), Some(base), FeatureChange::Update { entity, .. }) => {
                    match reading.as_ref().filter(|_| entry) {
                        Some(reading) => {
                            let layer = theirs.entity.base().layer_id.clone();
                            kentos_contracts::merge_reading(base, entity, &theirs.entity, &|k| {
                                reading.reach.level(&layer, k).readable()
                            })
                            .ok()
                        }
                        None => kentos_contracts::merge_entities(base, entity, &theirs.entity).ok(),
                    }
                }
                _ => None,
            };
            let written = merge.and_then(|m| {
                to_stored(&m.entity, new_srid, &placing)
                    .ok()
                    .filter(|s| writable_layer(&tree, &s.layer_id).is_ok())
                    .map(|s| (m.entity, s))
            });
            match written {
                Some((entity, stored)) => {
                    let layer = &existing[id].1;
                    let judged = if find_layer(&tree, layer).is_some() {
                        &tree
                    } else {
                        &current_tree
                    };
                    writable_layer(judged, layer)?;
                    plan[i].stored = Some(stored);
                    if let FeatureChange::Update { entity: e, .. } = &mut input.features[i] {
                        *e = entity;
                    }
                    merged_ids.insert(*id);
                }
                None => {
                    conflicting_ids.push(*id);
                    conflicts.push(FeatureConflict {
                        id: key.clone(),
                        reason: ConflictReason::Changed,
                        expected: envelope.expected_versions.get(&key).cloned(),
                        actual: existing.get(id).map(|(v, _)| v.to_string()),
                        current: None,
                    });
                }
            }
        }
    }
    if !conflicts.is_empty() {
        let mut copies = current_copies(tx, access, &conflicting_ids).await?;
        for c in &mut conflicts {
            if let Ok(id) = Uuid::parse_str(&c.id) {
                // A Veri girişi sees the server's copy as it reads it (docs/adr/0411 §6).
                c.current = copies.remove(&id).map(|mut r| {
                    if let Some(reading) = &reading {
                        r.entity = crate::form_guard::projected(r.entity, &reading.reach);
                    }
                    r
                });
            }
        }
        let n = conflicts.len();
        return Err(AppError::Conflict {
            message: format!(
                "{n} değişiklik, başka biri aynı nesneleri değiştirdiği için kaydedilmedi. Sunucudaki hâli ile sizinkini karşılaştırın."
            ),
            conflicts,
            revision: Some(data_revision),
        });
    }

    // A definition this command removes is placed by no object it leaves (someone else's, or one not sent).
    if !block_plan.removed.is_empty() {
        let names: Vec<String> = block_plan
            .removed
            .iter()
            .map(|(id, _, _)| id.to_string())
            .collect();
        let placed: Vec<(Uuid, Option<String>)> = sqlx::query_as(
            "select id, cad_definition->>'block' from kentos.feature
              where tenant_id = $1 and project_id = $2 and kind = 'insert' and cad_definition->>'block' = any($3)",
        )
        .bind(access.tenant)
        .bind(project)
        .bind(&names)
        .fetch_all(&mut **tx)
        .await?;
        let written: HashSet<Uuid> = plan.iter().map(|p| p.id).collect();
        let left: Vec<String> = placed
            .into_iter()
            .filter(|(id, _)| !written.contains(id))
            .filter_map(|(_, block)| block)
            .collect();
        if let Some((id, name, version)) = block_plan
            .removed
            .iter()
            .find(|(id, _, _)| left.contains(&id.to_string()))
        {
            let n = left.iter().filter(|b| **b == id.to_string()).count();
            let key = block_key(*id);
            return Err(AppError::Conflict {
                message: format!(
                    "“{name}” bloğu kullanılıyor (çizimde {n} yerleştirmesi; başka biri eklemiş olabilir); silinmedi. Sunucudaki hâli ile sizinkini karşılaştırın."
                ),
                conflicts: vec![FeatureConflict {
                    expected: envelope.expected_versions.get(&key).cloned(),
                    id: key,
                    reason: ConflictReason::Changed,
                    actual: Some(version.to_string()),
                    current: None,
                }],
                revision: Some(data_revision),
            });
        }
    }

    // The forms' guard (docs/adr/0411 §2, §4): a Veri girişi's change through its layer's published
    // form; anyone's write to a layer whose form enforces its rules, by them.
    let settings_after = patch.settings.as_ref().unwrap_or(&stored_settings);
    guard_forms(
        tx,
        access,
        &old_name,
        settings_after,
        &tree,
        &input.features,
        entry,
        new_srid,
    )
    .await?;
    // What a Veri girişi will need of each changed record's earlier copy to tell whether it had reached it
    // (docs/adr/0738 §3): kept with the event, never sent.
    let earlier = earlier_copies(tx, access, settings_after, &tree, &plan, &existing).await?;

    // 5. Write.
    let actor = access.actor.user_id;
    let mut versions = BTreeMap::new();
    let mut deleted = Vec::new();
    let mut events = Vec::with_capacity(plan.len());
    for p in &plan {
        match (&p.op, &p.stored) {
            (FeatureOp::Delete, _) => {
                sqlx::query("delete from kentos.feature where tenant_id = $1 and project_id = $2 and id = $3")
                    .bind(access.tenant)
                    .bind(project)
                    .bind(p.id)
                    .execute(&mut **tx)
                    .await?;
                deleted.push(p.id.to_string());
                events.push(EventFeature {
                    id: p.id.to_string(),
                    op: FeatureOp::Delete,
                    version: None,
                });
            }
            (op, Some(s)) => {
                let version: i64 = sqlx::query_scalar(
                    "insert into kentos.feature (tenant_id, project_id, id, layer_id, kind, source_kind, srid, geom, cad_definition, properties,
                                                 label, color, symbol, line_weight, label_pins, attachments, look, projection_version, created_by, updated_by, version)
                     values ($1, $2, $3, $4, $5, $6, $7, public.st_geomfromewkb($8), $9, $10, $11, $12, $13, $17, $18, $19, $20, $14, $15, $15, $16)
                     on conflict (tenant_id, project_id, id) do update set
                       layer_id = excluded.layer_id, kind = excluded.kind, source_kind = excluded.source_kind, srid = excluded.srid,
                       geom = excluded.geom, cad_definition = excluded.cad_definition, properties = excluded.properties,
                       label = excluded.label, color = excluded.color, symbol = excluded.symbol, line_weight = excluded.line_weight,
                       label_pins = excluded.label_pins, attachments = excluded.attachments, look = excluded.look,
                       projection_version = excluded.projection_version, updated_by = excluded.updated_by,
                       updated_at = now(), version = kentos.feature.version + 1
                     returning version",
                )
                .bind(access.tenant)
                .bind(project)
                .bind(p.id)
                .bind(&s.layer_id)
                .bind(&s.kind)
                .bind(s.source_kind)
                .bind(new_srid as i32)
                .bind(&s.geom)
                .bind(&s.cad_definition)
                .bind(&s.properties)
                .bind(&s.label)
                .bind(&s.color)
                .bind(&s.symbol)
                .bind(PROJECTION_VERSION)
                .bind(actor)
                .bind(created_version)
                .bind(s.line_weight)
                .bind(&s.label_pins)
                .bind(&s.attachments)
                .bind(&s.look)
                .fetch_one(&mut **tx)
                .await?;
                versions.insert(p.id.to_string(), version.to_string());
                events.push(EventFeature {
                    id: p.id.to_string(),
                    op: *op,
                    version: Some(version.to_string()),
                });
            }
            (_, None) => unreachable!("creates and updates carry an object"),
        }
    }
    // The block definitions, each under its key.
    let mut block_events = Vec::with_capacity(block_plan.changes.len());
    for (id, op, stored) in &block_plan.changes {
        let key = block_key(*id);
        let row = Uuid::from_bytes(id.0);
        match (op, stored) {
            (FeatureOp::Delete, _) => {
                sqlx::query("delete from kentos.block_definition where tenant_id = $1 and project_id = $2 and id = $3")
                    .bind(access.tenant)
                    .bind(project)
                    .bind(row)
                    .execute(&mut **tx)
                    .await?;
                deleted.push(key);
                block_events.push(EventBlock {
                    id: id.to_string(),
                    op: FeatureOp::Delete,
                    version: None,
                });
            }
            (op, Some((name, value))) => {
                let version: i64 = sqlx::query_scalar(
                    "insert into kentos.block_definition (tenant_id, project_id, id, name, definition, version, created_by, updated_by)
                     values ($1, $2, $3, $4, $5, $6, $7, $7)
                     on conflict (tenant_id, project_id, id) do update set
                       name = excluded.name, definition = excluded.definition, updated_by = excluded.updated_by,
                       updated_at = now(), version = kentos.block_definition.version + 1
                     returning version",
                )
                .bind(access.tenant)
                .bind(project)
                .bind(row)
                .bind(name)
                .bind(value)
                .bind(created_version)
                .bind(actor)
                .fetch_one(&mut **tx)
                .await?;
                versions.insert(key, version.to_string());
                block_events.push(EventBlock {
                    id: id.to_string(),
                    op: *op,
                    version: Some(version.to_string()),
                });
            }
            (_, None) => unreachable!("creates and updates carry a definition"),
        }
    }
    // A layer the new tree drops must be empty once this command's objects
    // are written: an object still on it (someone else's, or one not sent)
    // keeps the layer, and nothing of the command is written.
    if patch.layers.is_some() {
        let mut layers = Vec::new();
        leaf_ids(&current_tree, &mut layers);
        let dropped: Vec<&str> = layers
            .into_iter()
            .filter(|id| find_layer(&tree, id).is_none())
            .collect();
        if !dropped.is_empty() {
            let held: Option<String> = sqlx::query_scalar(
                "select layer_id from kentos.feature where tenant_id = $1 and project_id = $2 and layer_id = any($3) limit 1",
            )
            .bind(access.tenant)
            .bind(project)
            .bind(&dropped)
            .fetch_optional(&mut **tx)
            .await?;
            if let Some(layer) = held {
                let name = find_layer(&current_tree, &layer)
                    .map_or(layer.as_str(), |(n, _)| n.name.as_str());
                return Err(AppError::Conflict {
                    message: format!(
                        "“{name}” katmanında hâlâ nesne var (başka biri eklemiş ya da taşımış olabilir); katman silinmedi. Sunucudaki hâli ile sizinkini karşılaştırın."
                    ),
                    conflicts: vec![FeatureConflict {
                        id: PROJECT_META_KEY.into(),
                        reason: ConflictReason::Project,
                        expected: envelope.expected_versions.get(PROJECT_META_KEY).cloned(),
                        actual: Some(meta_version.to_string()),
                        current: None,
                    }],
                    revision: Some(data_revision),
                });
            }
        }
    }
    let meta_changed = input.project.is_some();
    // The name is the catalog's too: a new one raises its version (docs/adr/0028).
    let new_name = patch
        .name
        .as_ref()
        .map(|n| n.trim().to_string())
        .filter(|n| *n != old_name);
    if new_srid != srid as u32 {
        // Assigning a CRS relabels the coordinates, it does not transform them (CLAUDE.md §5).
        sqlx::query("update kentos.feature set srid = $3, geom = public.st_setsrid(geom, $3) where tenant_id = $1 and project_id = $2")
            .bind(access.tenant)
            .bind(project)
            .bind(new_srid as i32)
            .execute(&mut **tx)
            .await?;
    }
    // The inserts a changed definition places, which this command does not write itself: their geometry anew.
    let written: HashSet<Uuid> = plan.iter().map(|p| p.id).collect();
    blocks::reproject(
        tx,
        access.tenant,
        project,
        new_srid,
        &placing,
        &block_plan.changed,
        &written,
    )
    .await?;
    let retained_sets: Vec<kentos_contracts::DimensionSet> = match &patch.dimension_sets {
        Some(sets) => sets.clone(),
        None => stored_dimension_sets.clone(),
    };
    if patch.dimension_sets.is_some()
        || patch
            .settings
            .as_ref()
            .is_some_and(|settings| settings.plot_scale != stored_settings.plot_scale)
    {
        check_dimension_rows(
            tx,
            access,
            &retained_sets,
            &stored_dimension_sets,
            settings_after.plot_scale,
            patch.dimension_sets.is_some(),
            &input.features,
            &tree,
        )
        .await?;
    }
    crate::projects::check_dimension_layers(&retained_sets, &tree, "project.dimensionSets")?;
    crate::projects::check_dimension_members(
        tx,
        access.tenant,
        project,
        &retained_sets,
        "project.dimensionSets",
    )
    .await?;
    let (new_revision, new_meta): (i64, i64) = sqlx::query_as(
        "update kentos.project set
            data_revision = data_revision + 1,
            meta_version = meta_version + $3::int,
            name = coalesce($4, name), settings = coalesce($5, settings), srid = $6,
            layers = coalesce($7, layers), active_layer = coalesce($8, active_layer), styles = coalesce($9, styles),
            sheets = case when $11 then $12 else sheets end,
            arrays = case when $13 then $14 else arrays end,
            constraints = case when $15 then $16 else constraints end,
            dimension_sets = case when $17 then $18 else dimension_sets end,
            casework = case when $19 then $20 else casework end,
            catalog_version = catalog_version + $10::int, updated_at = now()
          where tenant_id = $1 and id = $2
          returning data_revision, meta_version",
    )
    .bind(access.tenant)
    .bind(project)
    .bind(i32::from(meta_changed))
    .bind(patch.name.as_ref().map(|n| n.trim().to_string()))
    .bind(patch.settings.as_ref().map(|s| serde_json::to_value(s).expect("settings serialize")))
    .bind(new_srid as i32)
    .bind(patch.layers.as_ref().map(|l| serde_json::to_value(l).expect("layers serialize")))
    .bind(&patch.active_layer)
    .bind(patch.styles.as_ref().map(|s| serde_json::to_value(s).expect("styles serialize")))
    .bind(i32::from(new_name.is_some()))
    .bind(patch.sheets.is_some())
    .bind(
        patch
            .sheets
            .as_ref()
            .and_then(Option::as_ref)
            .map(|s| serde_json::to_value(s).expect("sheets serialize")),
    )
    .bind(patch.arrays.is_some())
    .bind(
        patch
            .arrays
            .as_deref()
            .map(crate::projects::arrays_value)
            .transpose()?
            .flatten(),
    )
    .bind(patch.constraints.is_some())
    .bind(
        patch
            .constraints
            .as_ref()
            .map(crate::projects::constraints_value)
            .transpose()?
            .flatten(),
    )
    .bind(patch.dimension_sets.is_some())
    .bind(patch.dimension_sets.as_deref().map(crate::projects::dimension_sets_value).transpose()?.flatten())
    .bind(patch.casework.is_some())
    .bind(patch.casework.as_deref().map(crate::projects::casework_value).transpose()?.flatten())
    .fetch_one(&mut **tx)
    .await?;
    // The project row has been locked since the revision was read: created objects carry this commit's.
    debug_assert_eq!(new_revision, created_version);
    sqlx::query(
        "insert into kentos.audit_event (tenant_id, project_id, actor, action, request_id, data_revision, detail)
         values ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(access.tenant)
    .bind(project)
    .bind(actor)
    .bind(PROJECT_CHANGES)
    .bind(&envelope.request_id)
    .bind(new_revision)
    .bind({
        let mut detail = serde_json::json!({
            "created": plan.iter().filter(|p| p.op == FeatureOp::Create).count(),
            "updated": plan.iter().filter(|p| p.op == FeatureOp::Update).count(),
            "deleted": deleted.len(),
            "meta": meta_changed,
            "idempotencyKey": key,
        });
        if let Some(name) = &new_name {
            detail["rename"] = serde_json::json!({ "from": old_name, "to": name });
        }
        if !block_plan.changes.is_empty() {
            let count = |op: FeatureOp| block_plan.changes.iter().filter(|c| c.1 == op).count();
            detail["blocks"] = serde_json::json!({
                "created": count(FeatureOp::Create),
                "updated": count(FeatureOp::Update),
                "deleted": count(FeatureOp::Delete),
            });
        }
        detail
    })
    .execute(&mut **tx)
    .await?;
    let mut event = EventRecord {
        blocks: block_events,
        seq: String::new(),
        data_revision: new_revision.to_string(),
        kind: PROJECT_CHANGES.into(),
        actor: Some(actor.to_string()),
        request_id: Some(envelope.request_id.clone()),
        features: events,
        meta: meta_changed,
    };
    let seq: i64 = sqlx::query_scalar(
        "insert into kentos.outbox_event (tenant_id, project_id, data_revision, kind, payload) values ($1, $2, $3, $4, $5) returning seq",
    )
    .bind(access.tenant)
    .bind(project)
    .bind(new_revision)
    .bind(PROJECT_CHANGES)
    .bind({
        let mut payload = serde_json::to_value(&event).expect("event serializes");
        if !earlier.is_empty() {
            payload["earlier"] = serde_json::to_value(&earlier).expect("copies serialize");
        }
        payload
    })
    .fetch_one(&mut **tx)
    .await?;
    event.seq = seq.to_string();
    // The merged objects as written, for the client to take.
    let merged: Vec<kentos_contracts::FeatureRecord> = input
        .features
        .iter()
        .filter_map(|c| match c {
            FeatureChange::Update { id, entity }
                if Uuid::parse_str(id).is_ok_and(|u| merged_ids.contains(&u)) =>
            {
                Some(kentos_contracts::FeatureRecord {
                    id: id.clone(),
                    version: versions.get(id)?.clone(),
                    // A Veri girişi gets the merge as it reads it (docs/adr/0411 §6, 0737 §1).
                    entity: match &reading {
                        Some(reading) => {
                            crate::form_guard::projected(entity.clone(), &reading.reach)
                        }
                        None => entity.clone(),
                    },
                })
            }
            _ => None,
        })
        .collect();
    let result = CommitResult {
        data_revision: new_revision.to_string(),
        meta_version: new_meta.to_string(),
        versions,
        deleted,
        event_seq: seq.to_string(),
        replayed: false,
        merged,
    };
    idempotency::record(tx, access, &envelope, &text, &result).await?;
    Ok(result)
}
