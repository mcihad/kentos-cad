//! Each person's notifications (docs/adr/9000 §6): a dossier's step or message for the other side, a professional's
//! check, a new sign-up for the administrators. They are written by the security-definer functions of migration 0043,
//! which decide who is told; the server only words them. Read on the portal; nothing is mailed yet.

use kentos_contracts::{DossierSide, NotificationView, Notifications, NotificationsRead};
use kentos_postgres::Db;
use sqlx::{Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use super::accounts::user_scope;
use super::rfc3339;
use super::rows::side_code;
use crate::error::AppResult;
use crate::identity::Actor;

type Row = (
    Uuid,
    OffsetDateTime,
    String,
    String,
    String,
    bool,
    Option<Uuid>,
    Option<String>,
);

/// The caller's unread count (the live channel's message, docs/adr/9000 §23).
pub async fn unread(db: &Db, actor: &Actor) -> AppResult<u32> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let n: i64 = sqlx::query_scalar("select count(*) from kentos.notification where user_id = $1 and read_at is null")
        .bind(actor.user_id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

/// `GET /v1/me/notifications?unread=`: the newest first.
pub async fn list(
    db: &Db,
    actor: &Actor,
    unread_only: bool,
    limit: u32,
) -> AppResult<Notifications> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let rows: Vec<Row> = sqlx::query_as(
        "select n.id, n.at, n.kind, n.title, n.body, n.read_at is not null, n.dossier_id, t.slug
           from kentos.notification n left join kentos.tenant t on t.id = n.tenant_id
          where n.user_id = $1 and (not $2 or n.read_at is null)
          order by n.at desc
          limit $3",
    )
    .bind(actor.user_id)
    .bind(unread_only)
    .bind(i64::from(limit.clamp(1, 200)))
    .fetch_all(&mut *tx)
    .await?;
    let unread: i64 = sqlx::query_scalar(
        "select count(*) from kentos.notification where user_id = $1 and read_at is null",
    )
    .bind(actor.user_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Notifications {
        items: rows
            .into_iter()
            .map(
                |(id, at, kind, title, body, read, dossier, slug)| NotificationView {
                    id: id.to_string(),
                    at: rfc3339(at),
                    kind,
                    title,
                    body,
                    read,
                    dossier_id: dossier.map(|d| d.to_string()),
                    tenant_slug: slug,
                },
            )
            .collect(),
        unread: u32::try_from(unread).unwrap_or(u32::MAX),
    })
}

/// `POST /v1/me/notifications/read`: returns how many were marked.
pub async fn mark_read(db: &Db, actor: &Actor, input: &NotificationsRead) -> AppResult<u32> {
    let ids: Vec<Uuid> = input
        .ids
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect();
    let mut tx = db.scoped(user_scope(actor)).await?;
    let done = sqlx::query(
        "update kentos.notification set read_at = now()
          where user_id = $1 and read_at is null and ($2 or id = any($3))",
    )
    .bind(actor.user_id)
    .bind(input.all)
    .bind(&ids)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(u32::try_from(done.rows_affected()).unwrap_or(u32::MAX))
}

/// Tells `audience` of the dossier in the open transaction (`kentos.notify_dossier`); returns how many were told.
pub(crate) async fn tell(
    tx: &mut Transaction<'static, Postgres>,
    dossier: Uuid,
    audience: DossierSide,
    kind: &str,
    title: &str,
    body: &str,
) -> AppResult<i32> {
    Ok(
        sqlx::query_scalar("select kentos.notify_dossier($1, $2, $3, $4, $5)")
            .bind(dossier)
            .bind(side_code(audience))
            .bind(kind)
            .bind(title)
            .bind(body)
            .fetch_one(&mut **tx)
            .await?,
    )
}
