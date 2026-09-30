//! A database project's block definitions (docs/adr/0144 §5, migration
//! 0012): one row each, in the order they were made, with the version an
//! edit is checked against; the list a client opens with or asks for after
//! an event names changed blocks (`GET …/blocks`); a drawing's definitions
//! written into a new project; and the geometry of the inserts a changed
//! definition places, made again in the commit that changed it.

use std::collections::HashSet;

use kentos_contracts::{BlockDefinition, BlockId, BlockList, BlockRecord, Entity};
use kentos_formats::blocks::Placing;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::access::ProjectAccess;
use crate::cad::{insert_geometry, to_stored_block};
use crate::error::{AppError, AppResult};
use crate::projects::{FEATURE_COLUMNS, FeatureRow, record};

/// One stored definition and its version.
pub(crate) struct Stored {
    pub version: i64,
    pub block: BlockDefinition,
}

/// The project's definitions in the order they were made; `lock`: for the
/// commit, which changes them.
pub(crate) async fn read(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
    project: Uuid,
    lock: bool,
) -> AppResult<Vec<Stored>> {
    let sql = if lock {
        "select version, definition from kentos.block_definition where tenant_id = $1 and project_id = $2 order by seq for update"
    } else {
        "select version, definition from kentos.block_definition where tenant_id = $1 and project_id = $2 order by seq"
    };
    let rows: Vec<(i64, Value)> = sqlx::query_as(sql)
        .bind(tenant)
        .bind(project)
        .fetch_all(&mut **tx)
        .await?;
    rows.into_iter()
        .map(|(version, definition)| {
            serde_json::from_value(definition)
                .map(|block| Stored { version, block })
                .map_err(|e| AppError::invalid(format!("Projenin bir blok tanımı okunamadı: {e}")))
        })
        .collect()
}

/// The definitions with their versions, as the wire gives them.
pub(crate) fn records(list: Vec<Stored>) -> Vec<BlockRecord> {
    list.into_iter()
        .map(|s| BlockRecord {
            version: s.version.to_string(),
            block: s.block,
        })
        .collect()
}

/// `GET …/blocks`: the project's definitions with their versions.
pub async fn list(db: &kentos_postgres::Db, access: &ProjectAccess) -> AppResult<BlockList> {
    access.live()?;
    let mut tx = db.scoped(access.scope()).await?;
    let blocks = read(&mut tx, access.tenant, access.project, false).await?;
    tx.commit().await?;
    Ok(BlockList {
        blocks: records(blocks),
    })
}

/// A drawing's definitions into a new project (an import, a restored
/// checkpoint, a conversion), in the drawing's order, each at version 1 as
/// its objects are. Checked as the commit checks them (the drawing's reader
/// has checked the rules over the list).
pub(crate) async fn insert_all(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
    project: Uuid,
    actor: Uuid,
    blocks: &[BlockDefinition],
) -> AppResult<()> {
    for (i, b) in blocks.iter().enumerate() {
        let (block, value) = to_stored_block(b).map_err(|why| {
            AppError::invalid_at(
                format!("blocks[{i}]"),
                format!("Dosyanın “{}” bloğu içe aktarılamadı: {why}", b.name),
            )
        })?;
        sqlx::query(
            "insert into kentos.block_definition (tenant_id, project_id, id, name, definition, version, created_by, updated_by)
             values ($1, $2, $3, $4, $5, 1, $6, $6)",
        )
        .bind(tenant)
        .bind(project)
        .bind(Uuid::from_bytes(block.id.0))
        .bind(&block.name)
        .bind(value)
        .bind(actor)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// The definitions whose placed objects change when `changed` change: they
/// and every definition placing one of them, however deep.
pub(crate) fn affected(blocks: &[BlockDefinition], changed: &HashSet<BlockId>) -> HashSet<BlockId> {
    let mut out = changed.clone();
    loop {
        let before = out.len();
        for b in blocks {
            if !out.contains(&b.id)
                && b.entities
                    .iter()
                    .any(|e| matches!(e, Entity::Insert(i) if out.contains(&i.block)))
            {
                out.insert(b.id);
            }
        }
        if out.len() == before {
            return out;
        }
    }
}

/// The inserts placing one of `blocks` get their geometry anew from
/// `placing`; `skip`: the objects this commit writes itself. Their
/// versions stay: their source did not change.
pub(crate) async fn reproject(
    tx: &mut Transaction<'static, Postgres>,
    tenant: Uuid,
    project: Uuid,
    srid: u32,
    placing: &Placing,
    blocks: &HashSet<BlockId>,
    skip: &HashSet<Uuid>,
) -> AppResult<()> {
    if blocks.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = blocks.iter().map(ToString::to_string).collect();
    let rows: Vec<FeatureRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {FEATURE_COLUMNS} from kentos.feature
          where tenant_id = $1 and project_id = $2 and kind = 'insert' and cad_definition->>'block' = any($3)"
    )))
    .bind(tenant)
    .bind(project)
    .bind(&names)
    .fetch_all(&mut **tx)
    .await?;
    let mut ids = Vec::with_capacity(rows.len());
    let mut geoms = Vec::with_capacity(rows.len());
    for row in rows {
        let id = row.0;
        if skip.contains(&id) {
            continue;
        }
        let entity = record(row)?.entity;
        ids.push(id);
        geoms.push(insert_geometry(&entity, srid, placing));
    }
    if ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "update kentos.feature f set geom = public.st_geomfromewkb(u.geom)
           from unnest($3::uuid[], $4::bytea[]) as u(id, geom)
          where f.tenant_id = $1 and f.project_id = $2 and f.id = u.id",
    )
    .bind(tenant)
    .bind(project)
    .bind(&ids)
    .bind(&geoms)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(n: u8, inside: &[u8]) -> BlockDefinition {
        let id = |n: u8| format!("018f3a2b-0000-7000-8000-0000000000{n:02x}");
        serde_json::from_value(serde_json::json!({
            "id": id(n), "name": format!("B{n}"), "base": { "x": 0, "y": 0 },
            "entities": inside.iter().enumerate().map(|(k, b)| serde_json::json!({
                "kind": "insert", "id": k + 1, "layerId": "", "attrs": {}, "block": id(*b),
                "p": { "x": 0, "y": 0 }, "scale": 1, "rotation": 0
            })).collect::<Vec<_>>()
        }))
        .unwrap()
    }

    /// C holds B, B holds A, D holds nothing: a change of A reaches B and C, not D.
    #[test]
    fn a_change_reaches_every_definition_placing_it() {
        let blocks = [block(1, &[]), block(2, &[1]), block(3, &[2]), block(4, &[])];
        let changed: HashSet<BlockId> = [blocks[0].id].into();
        let reached = affected(&blocks, &changed);
        let want: HashSet<BlockId> = [blocks[0].id, blocks[1].id, blocks[2].id].into();
        assert_eq!(reached, want);
        assert_eq!(affected(&blocks, &HashSet::new()), HashSet::new());
    }
}
