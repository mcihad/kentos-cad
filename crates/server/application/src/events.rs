//! A project's committed changes after a cursor, in commit order (the
//! outbox, CLAUDE.md §17 "Tile güncellik kapısı", §21.1). Clients replay what
//! they missed from here; the WebSocket pushes the same records live.
//!
//! The log is kept for a while, not forever (migration 0003): `prune`
//! removes old events, and each project's horizon records the newest one it
//! removed. A cursor below the horizon cannot be continued from (some events
//! after it are gone), and one beyond the newest event never existed (a
//! restored database): both answer `ResyncRequired`, and the client opens
//! the project again. A deleted project's log stays readable.

use std::time::Duration;

use kentos_contracts::{EventPage, EventRecord};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::tenancy::{Access, Capability};

pub const PAGE_MAX: i64 = 500;

pub fn record(seq: i64, payload: Value) -> AppResult<EventRecord> {
    let mut e: EventRecord = serde_json::from_value(payload)
        .map_err(|e| AppError::invalid(format!("Olay {seq} okunamadı: {e}")))?;
    e.seq = seq.to_string();
    Ok(e)
}

/// Where a project's log stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// The cursor of a client that has everything: the newest event's, or the horizon once every event is gone.
    pub newest: i64,
    /// Events with `seq <= pruned_through` may be gone (0: nothing was removed).
    pub pruned_through: i64,
}

impl Bounds {
    /// Whether the events after `cursor` can still be replayed from the log.
    pub fn can_continue(&self, cursor: i64) -> bool {
        (self.pruned_through..=self.newest).contains(&cursor)
    }
}

fn resync() -> AppError {
    AppError::ResyncRequired(
        "Bu noktadan sonraki değişiklikler sunucuda artık saklanmıyor; projeyi yeniden açın."
            .into(),
    )
}

async fn exists(
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    project: Uuid,
) -> AppResult<()> {
    let found: bool = sqlx::query_scalar(
        "select exists (select 1 from kentos.project where tenant_id = $1 and id = $2)",
    )
    .bind(access.tenant)
    .bind(project)
    .fetch_one(&mut **tx)
    .await?;
    if found {
        Ok(())
    } else {
        Err(AppError::not_found("Proje bulunamadı."))
    }
}

async fn horizon(
    tx: &mut Transaction<'static, Postgres>,
    access: &Access,
    project: Uuid,
) -> AppResult<i64> {
    let through: Option<i64> = sqlx::query_scalar(
        "select pruned_through from kentos.outbox_horizon where tenant_id = $1 and project_id = $2",
    )
    .bind(access.tenant)
    .bind(project)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(through.unwrap_or(0))
}

/// Events with `seq > after`, at most `limit`; `next` is the cursor to continue
/// from. A cursor older than what is kept is refused (`ResyncRequired`).
pub async fn after(
    db: &kentos_postgres::Db,
    access: &Access,
    project: Uuid,
    after: i64,
    limit: i64,
) -> AppResult<EventPage> {
    access.require(Capability::ProjectRead)?;
    let mut tx = db.scoped(access.scope()).await?;
    exists(&mut tx, access, project).await?;
    let rows: Vec<(i64, Value)> = sqlx::query_as(
        "select seq, payload from kentos.outbox_event where tenant_id = $1 and project_id = $2 and seq > $3 order by seq limit $4",
    )
    .bind(access.tenant)
    .bind(project)
    .bind(after)
    .bind(limit.clamp(1, PAGE_MAX))
    .fetch_all(&mut *tx)
    .await?;
    // Read after the events: a prune that removed some of them had committed its
    // horizon before, so it shows here. (One committing in between only makes
    // this a resync that was not needed.)
    let pruned = horizon(&mut tx, access, project).await?;
    tx.commit().await?;
    if after < pruned {
        return Err(resync());
    }
    let next = rows.last().map(|(s, _)| *s).unwrap_or(after);
    let events = rows
        .into_iter()
        .map(|(s, p)| record(s, p))
        .collect::<AppResult<_>>()?;
    Ok(EventPage {
        events,
        next: next.to_string(),
    })
}

/// The newest cursor and the horizon of a project's log (a subscription checks its cursor against both).
pub async fn bounds(db: &kentos_postgres::Db, access: &Access, project: Uuid) -> AppResult<Bounds> {
    access.require(Capability::ProjectRead)?;
    let mut tx = db.scoped(access.scope()).await?;
    exists(&mut tx, access, project).await?;
    let newest: i64 = sqlx::query_scalar(
        "select coalesce(max(seq), 0) from kentos.outbox_event where tenant_id = $1 and project_id = $2",
    )
    .bind(access.tenant)
    .bind(project)
    .fetch_one(&mut *tx)
    .await?;
    let pruned_through = horizon(&mut tx, access, project).await?;
    tx.commit().await?;
    Ok(Bounds {
        newest: newest.max(pruned_through),
        pruned_through,
    })
}

/// The cursor of a client that has everything (0 when the project has no events yet).
pub async fn latest(db: &kentos_postgres::Db, access: &Access, project: Uuid) -> AppResult<i64> {
    Ok(bounds(db, access, project).await?.newest)
}

/// Removes up to `batch` events older than `keep`, of every tenant (through
/// `kentos.prune_outbox`, which runs as the owner and refuses a window under
/// an hour), and records each project's horizon. Returns how many went: a
/// full batch means there may be more.
pub async fn prune(db: &kentos_postgres::Db, keep: Duration, batch: i32) -> AppResult<i64> {
    Ok(
        sqlx::query_scalar("select kentos.prune_outbox(make_interval(secs => $1), $2)")
            .bind(keep.as_secs_f64())
            .bind(batch)
            .fetch_one(&db.pool)
            .await?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_continues_only_inside_the_kept_log() {
        let b = Bounds {
            newest: 40,
            pruned_through: 25,
        };
        assert!(b.can_continue(25) && b.can_continue(31) && b.can_continue(40));
        // Some events after 24 are gone; nothing after 41 ever existed.
        assert!(!b.can_continue(24) && !b.can_continue(0) && !b.can_continue(41));
        // Nothing removed yet: every cursor up to the newest is fine.
        let fresh = Bounds {
            newest: 7,
            pruned_through: 0,
        };
        assert!(fresh.can_continue(0) && !fresh.can_continue(8));
    }
}
