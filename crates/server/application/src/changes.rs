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
    CommandEnvelope, CommitResult, ConflictReason, EventFeature, EventRecord, FeatureChange,
    FeatureConflict, FeatureOp, LayerNode, LayerNodeType, PROJECT_CHANGES, PROJECT_CHANGES_VERSION,
    PROJECT_META_KEY, ProjectChanges, ProjectPermission,
};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::{self, ProjectAccess, not_found};
use crate::cad::{PROJECTION_VERSION, Stored, to_stored};
use crate::error::{AppError, AppResult};
use crate::idempotency;
use crate::projects::{
    FEATURE_COLUMNS, FeatureRow, check_name, check_srid, check_tree, find_layer, gone, record,
};

/// Most object changes one command may carry; larger sets go in several commands.
pub const MAX_CHANGES: usize = 5000;

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
        Some(_) => Ok(()),
        None => Err(AppError::invalid(format!("“{id}” katmanı projede yok."))),
    }
}

struct Planned {
    id: Uuid,
    op: FeatureOp,
    stored: Option<Stored>,
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

pub async fn commit(
    db: &kentos_postgres::Db,
    access: &ProjectAccess,
    envelope: CommandEnvelope,
) -> AppResult<CommitResult> {
    if envelope.command_name != PROJECT_CHANGES {
        return Err(AppError::invalid(format!(
            "Bilinmeyen komut: {}",
            envelope.command_name
        )));
    }
    if envelope.version != PROJECT_CHANGES_VERSION {
        return Err(AppError::invalid(format!(
            "{PROJECT_CHANGES} komutunun {} sürümü desteklenmiyor (desteklenen: {PROJECT_CHANGES_VERSION}).",
            envelope.version
        )));
    }
    check_target(&envelope, access)?;
    idempotency::check_key(&envelope)?;
    let key = envelope.idempotency_key.as_str();
    let input: ProjectChanges = serde_json::from_value(envelope.input.clone())
        .map_err(|e| AppError::invalid(format!("Komut girdisi okunamadı: {e}")))?;
    if input.features.len() > MAX_CHANGES {
        return Err(AppError::invalid(format!(
            "Bir komutta en çok {MAX_CHANGES} nesne değişikliği olabilir; değişiklikleri parçalara bölün."
        )));
    }

    let mut tx = db.scoped(access.scope()).await?;
    // 1. Lock the project: commits of one project happen one after another.
    let now = lock(&mut tx, access).await?;
    let project = now.project;
    let (srid, layers, meta_version, data_revision, old_name, storage): (i32, Value, i64, i64, String, String) = sqlx::query_as(
        "select srid, layers, meta_version, data_revision, name, storage from kentos.project where tenant_id = $1 and id = $2",
    )
    .bind(now.tenant)
    .bind(project)
    .fetch_one(&mut *tx)
    .await?;
    // A file project's content is its revisions (docs/adr/0031): objects are not written one by one.
    if storage == "file" {
        return Err(AppError::invalid(format!(
            "“{old_name}” projesi dosya olarak saklanıyor; nesneler tek tek yazılmaz. Çizimi kaydedip yeni bir dosya revizyonu olarak yükleyin (project.file.commit)."
        )));
    }
    // The version a created object gets: this commit's data revision (the module's comment says why).
    let created_version = data_revision + 1;

    // 2. The same key again: the stored answer, or a refusal if the request differs.
    let text = idempotency::request_text(&envelope);
    if let Some(mut result) =
        idempotency::earlier::<CommitResult>(&mut tx, &now, &envelope, &text).await?
    {
        tx.commit().await?;
        result.replayed = true;
        return Ok(result);
    }
    if now.deleted {
        return Err(gone(&now.name));
    }
    if now.archived {
        return Err(access::archived(&now.name));
    }
    // 3. The rights, as they are now.
    if !input.features.is_empty() {
        now.require(ProjectPermission::FeatureWrite)?;
    }
    if input.project.is_some() {
        now.require(ProjectPermission::Edit)?;
    }
    let access = &now;

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
                .fetch_one(&mut *tx)
                .await?
            }
        };
        check_tree(&tree, &active)?;
    }
    if new_srid != srid as u32 {
        check_srid(&mut tx, new_srid).await?;
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

    let mut plan = Vec::with_capacity(input.features.len());
    let mut seen = HashSet::new();
    for (i, change) in input.features.iter().enumerate() {
        let (id, op, entity) = match change {
            FeatureChange::Create { id, entity } => (id, FeatureOp::Create, Some(entity)),
            FeatureChange::Update { id, entity } => (id, FeatureOp::Update, Some(entity)),
            FeatureChange::Delete { id } => (id, FeatureOp::Delete, None),
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
                let s = to_stored(e, new_srid).map_err(|why| {
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
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|(id, v, l)| (id, (v, l)))
    .collect();
    let mut conflicting_ids = Vec::new();
    for p in &plan {
        let key = p.id.to_string();
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
            writable_layer(&tree, &existing[&p.id].1)?;
        }
    }
    if !conflicts.is_empty() {
        let mut copies = current_copies(&mut tx, access, &conflicting_ids).await?;
        for c in &mut conflicts {
            if let Ok(id) = Uuid::parse_str(&c.id) {
                c.current = copies.remove(&id);
            }
        }
        drop(tx);
        let n = conflicts.len();
        return Err(AppError::Conflict {
            message: format!(
                "{n} değişiklik, başka biri aynı nesneleri değiştirdiği için kaydedilmedi. Sunucudaki hâli ile sizinkini karşılaştırın."
            ),
            conflicts,
            revision: Some(data_revision),
        });
    }

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
                    .execute(&mut *tx)
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
                                                 label, color, symbol, projection_version, created_by, updated_by, version)
                     values ($1, $2, $3, $4, $5, $6, $7, public.st_geomfromewkb($8), $9, $10, $11, $12, $13, $14, $15, $15, $16)
                     on conflict (tenant_id, project_id, id) do update set
                       layer_id = excluded.layer_id, kind = excluded.kind, source_kind = excluded.source_kind, srid = excluded.srid,
                       geom = excluded.geom, cad_definition = excluded.cad_definition, properties = excluded.properties,
                       label = excluded.label, color = excluded.color, symbol = excluded.symbol,
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
                .fetch_one(&mut *tx)
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
            .execute(&mut *tx)
            .await?;
    }
    let (new_revision, new_meta): (i64, i64) = sqlx::query_as(
        "update kentos.project set
            data_revision = data_revision + 1,
            meta_version = meta_version + $3::int,
            name = coalesce($4, name), settings = coalesce($5, settings), srid = $6,
            layers = coalesce($7, layers), active_layer = coalesce($8, active_layer), styles = coalesce($9, styles),
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
    .fetch_one(&mut *tx)
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
        detail
    })
    .execute(&mut *tx)
    .await?;
    let mut event = EventRecord {
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
    .bind(serde_json::to_value(&event).expect("event serializes"))
    .fetch_one(&mut *tx)
    .await?;
    event.seq = seq.to_string();
    let result = CommitResult {
        data_revision: new_revision.to_string(),
        meta_version: new_meta.to_string(),
        versions,
        deleted,
        event_seq: seq.to_string(),
        replayed: false,
    };
    idempotency::record(&mut tx, access, &envelope, &text, &result).await?;
    tx.commit().await?;
    Ok(result)
}
