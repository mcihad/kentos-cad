//! A release's moves as their table keeps them (docs/adr/8059 §11, §13): one row per move, in the record's order,
//! in the record's own words; added in one statement, read back and compared with the record row for row.

use kentos_extensions::release::{AuditRecord, Release};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::failed;
use crate::extension_catalog::store::StoreError;

/// A move as its table keeps it, and as a record's move is written there.
#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
pub(super) struct MoveRow {
    pub(super) release_id: Uuid,
    pub(super) seq: i32,
    pub(super) at: i64,
    pub(super) account: String,
    pub(super) via: String,
    pub(super) action: String,
    pub(super) from_state: Option<String>,
    pub(super) to_state: String,
    pub(super) content: String,
    pub(super) gate: Option<String>,
    pub(super) reason: Option<String>,
}

const MOVE_COLUMNS: &str =
    "release_id, seq, at, account, via, action, from_state, to_state, content, gate, reason";

impl MoveRow {
    /// A record's move as its row: the record's own words (`via`, `action`, the states and the gate as its JSON says
    /// them), so the row and the record are compared as written.
    fn of(release_id: Uuid, seq: usize, record: &AuditRecord) -> Result<MoveRow, StoreError> {
        let json = serde_json::to_value(record).map_err(failed)?;
        let word = |key: &str| json.get(key).and_then(Value::as_str).map(str::to_owned);
        Ok(MoveRow {
            release_id,
            seq: i32::try_from(seq).map_err(failed)?,
            at: i64::try_from(record.at.0).map_err(failed)?,
            account: record.by.clone(),
            via: word("via").unwrap_or_default(),
            action: word("action").unwrap_or_default(),
            from_state: word("from"),
            to_state: word("to").unwrap_or_default(),
            content: record.content.clone(),
            gate: word("gate"),
            reason: record.reason.clone(),
        })
    }
}

/// A release's rows: its moves, in order.
pub(super) fn rows_of(release_id: Uuid, release: &Release) -> Result<Vec<MoveRow>, StoreError> {
    release
        .audit()
        .iter()
        .enumerate()
        .map(|(seq, record)| MoveRow::of(release_id, seq, record))
        .collect()
}

/// A stored record, read through the flow's checks; its moves must be the table's.
pub(super) fn read(release_id: Uuid, text: &str, kept: &[MoveRow]) -> Result<Release, StoreError> {
    let release = Release::read(text).map_err(failed)?;
    if rows_of(release_id, &release)? != kept {
        return Err(StoreError::Failed(format!(
            "{} {}: hareket tablosu kaydın hareketleriyle aynı değil",
            release.extension_id(),
            release.version()
        )));
    }
    Ok(release)
}

/// Adds moves in one statement.
pub(super) async fn add_moves(
    tx: &mut Transaction<'static, Postgres>,
    release_id: Uuid,
    rows: &[MoveRow],
) -> Result<(), StoreError> {
    if rows.is_empty() {
        return Ok(());
    }
    let column = |f: fn(&MoveRow) -> Option<String>| rows.iter().map(f).collect::<Vec<_>>();
    sqlx::query(
        "insert into kentos.extension_release_move
           (release_id, seq, at, account, via, action, from_state, to_state, content, gate, reason)
         select $1, * from unnest($2::integer[], $3::bigint[], $4::text[], $5::text[], $6::text[], $7::text[],
                                  $8::text[], $9::text[], $10::text[], $11::text[])",
    )
    .bind(release_id)
    .bind(rows.iter().map(|r| r.seq).collect::<Vec<_>>())
    .bind(rows.iter().map(|r| r.at).collect::<Vec<_>>())
    .bind(column(|r| Some(r.account.clone())))
    .bind(column(|r| Some(r.via.clone())))
    .bind(column(|r| Some(r.action.clone())))
    .bind(column(|r| r.from_state.clone()))
    .bind(column(|r| Some(r.to_state.clone())))
    .bind(column(|r| Some(r.content.clone())))
    .bind(column(|r| r.gate.clone()))
    .bind(column(|r| r.reason.clone()))
    .execute(&mut **tx)
    .await
    .map_err(failed)?;
    Ok(())
}

/// A release's kept moves, in order.
pub(super) async fn moves_of(
    tx: &mut Transaction<'static, Postgres>,
    release_id: Uuid,
) -> Result<Vec<MoveRow>, StoreError> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "select {MOVE_COLUMNS} from kentos.extension_release_move where release_id = $1 order by seq"
    )))
    .bind(release_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(failed)
}
