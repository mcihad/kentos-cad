//! How long the portal keeps personal data, and its periodic destruction (docs/adr/9000 §21; plan BUL-18). The work is
//! `kentos.retention_sweep` (migration 0051): it archives the closed dossiers past their period, destroys what has
//! served its purpose and records the run; the keys of the destroyed files come back here to leave the store too.
//!
//! The server runs it every day (`sweep`); a platform administrator may run it by hand (`run_now`) and sets the
//! periods (`set`). Daily is well within the regulation's at most six months between two periodic destructions.

use kentos_contracts::{RetentionPage, RetentionRunView, RetentionSettings};
use kentos_postgres::Db;
use sqlx::types::Json;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;

use super::accounts::user_scope;
use super::files::remove_all;
use super::{answer, rfc3339};
use crate::blobs::Blobs;
use crate::error::{AppError, AppResult};
use crate::identity::Actor;

async fn admin_tx(db: &Db, actor: &Actor) -> AppResult<Transaction<'static, Postgres>> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let ok: bool = sqlx::query_scalar("select kentos.is_platform_admin()")
        .fetch_one(&mut *tx)
        .await?;
    if ok {
        Ok(tx)
    } else {
        Err(AppError::forbidden(
            "Bu sayfa yalnız sistem yöneticilerine açık.",
        ))
    }
}

fn days(n: i32) -> u32 {
    u32::try_from(n).unwrap_or(0)
}

type RunRow = (
    i64,
    OffsetDateTime,
    String,
    Option<String>,
    i32,
    i32,
    i32,
    i32,
    i32,
    i32,
    Json<RetentionSettings>,
);

fn run_view(r: RunRow) -> RetentionRunView {
    let (
        id,
        at,
        trigger,
        actor_name,
        archived,
        drafts,
        files,
        notes,
        deliveries,
        requests,
        Json(settings),
    ) = r;
    RetentionRunView {
        id: id.to_string(),
        at: rfc3339(at),
        trigger,
        actor_name,
        archived: days(archived),
        drafts_deleted: days(drafts),
        files_deleted: days(files),
        notifications_deleted: days(notes),
        deliveries_deleted: days(deliveries),
        signing_requests_deleted: days(requests),
        settings,
    }
}

const RUN_SELECT: &str = "select id, started_at, trigger, actor_name, archived, drafts_deleted, files_deleted,
        notifications_deleted, deliveries_deleted, signing_requests_deleted, settings from kentos.retention_run";

async fn page_in(tx: &mut Transaction<'static, Postgres>) -> AppResult<RetentionPage> {
    let (a, d, n, t): (i32, i32, i32, i32) = sqlx::query_as(
        "select archive_after_days, draft_keep_days, notification_keep_days, technical_keep_days
           from kentos.platform_setting",
    )
    .fetch_one(&mut **tx)
    .await?;
    let runs: Vec<RunRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{RUN_SELECT} order by id desc limit 20"
    )))
    .fetch_all(&mut **tx)
    .await?;
    Ok(RetentionPage {
        settings: RetentionSettings {
            archive_after_days: days(a),
            draft_keep_days: days(d),
            notification_keep_days: days(n),
            technical_keep_days: days(t),
        },
        runs: runs.into_iter().map(run_view).collect(),
    })
}

/// `GET /v1/platform/retention`.
pub async fn page(db: &Db, actor: &Actor) -> AppResult<RetentionPage> {
    let mut tx = admin_tx(db, actor).await?;
    let page = page_in(&mut tx).await?;
    tx.commit().await?;
    Ok(page)
}

/// `PUT /v1/platform/retention`: the periods; in the platform's record.
pub async fn set(db: &Db, actor: &Actor, input: &RetentionSettings) -> AppResult<RetentionPage> {
    let check = |field: &str, v: u32, lo: u32| {
        if (lo..=3650).contains(&v) {
            Ok(i32::try_from(v).unwrap_or(i32::MAX))
        } else {
            Err(AppError::invalid_at(
                field,
                format!("{lo} ile 3650 gün arasında olmalı."),
            ))
        }
    };
    let a = check("archiveAfterDays", input.archive_after_days, 30)?;
    let d = check("draftKeepDays", input.draft_keep_days, 30)?;
    let n = check("notificationKeepDays", input.notification_keep_days, 30)?;
    let t = check("technicalKeepDays", input.technical_keep_days, 7)?;
    let mut tx = admin_tx(db, actor).await?;
    let code: Option<String> =
        sqlx::query_scalar("select kentos.platform_set_retention($1, $2, $3, $4)")
            .bind(a)
            .bind(d)
            .bind(n)
            .bind(t)
            .fetch_one(&mut *tx)
            .await?;
    answer(code)?;
    let page = page_in(&mut tx).await?;
    tx.commit().await?;
    Ok(page)
}

/// The server's daily run: the number of files that left the store.
pub async fn sweep(db: &Db, blobs: &Blobs) -> AppResult<usize> {
    let keys: Vec<String> =
        sqlx::query_scalar("select blob_key from kentos.retention_sweep('schedule')")
            .fetch_all(&db.pool)
            .await?;
    remove_all(blobs, &keys).await;
    Ok(keys.len())
}

/// `POST /v1/platform/retention/run`: a platform administrator's run, now; its record.
pub async fn run_now(db: &Db, blobs: &Blobs, actor: &Actor) -> AppResult<RetentionRunView> {
    let mut tx = admin_tx(db, actor).await?;
    let keys: Vec<String> =
        sqlx::query_scalar("select blob_key from kentos.retention_sweep('manual')")
            .fetch_all(&mut *tx)
            .await?;
    let run: RunRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{RUN_SELECT} order by id desc limit 1"
    )))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    // The rows are gone once committed; then their bytes.
    remove_all(blobs, &keys).await;
    Ok(run_view(run))
}
