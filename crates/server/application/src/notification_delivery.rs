//! Notifications that leave the portal (docs/adr/9000 §17): each person's choice of e-mail and SMS, what was sent to
//! them, and the outbox the server's sender reads. A trigger on `kentos.notification` fills the outbox (migration 0046);
//! the sender takes due deliveries under a lease and says what became of each.

use kentos_contracts::{DeliveryView, NotificationSettings, NotificationSettingsEdit};
use kentos_postgres::Db;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::identity::Actor;
use crate::portal::accounts::user_scope;
use crate::projects::rfc3339;

/// A phone number as it is kept: digits with an optional leading `+`, spaces and punctuation dropped.
pub fn phone_of(text: &str) -> AppResult<Option<String>> {
    let t = text.trim();
    if t.is_empty() {
        return Ok(None);
    }
    let plus = t.starts_with('+');
    let digits: String = t.chars().filter(char::is_ascii_digit).collect();
    let ok = t
        .chars()
        .all(|c| c.is_ascii_digit() || " +-().".contains(c))
        && (10..=15).contains(&digits.len());
    if !ok {
        return Err(AppError::invalid_at(
            "phone",
            "Telefon numarası 10–15 rakam olmalı: +905321234567 ya da 05321234567.",
        ));
    }
    Ok(Some(if plus { format!("+{digits}") } else { digits }))
}

/// `GET /v1/me/notification-settings`.
pub async fn settings(db: &Db, actor: &Actor) -> AppResult<NotificationSettings> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let (email_offered, sms_offered): (bool, bool) = sqlx::query_as(
        "select email_notifications, sms_notifications from kentos.platform_setting",
    )
    .fetch_one(&mut *tx)
    .await?;
    let (email_address, email_verified): (Option<String>, bool) =
        sqlx::query_as("select email, email_verified from kentos.app_user where id = $1")
            .bind(actor.user_id)
            .fetch_one(&mut *tx)
            .await?;
    let pref: Option<(bool, bool, Option<String>)> = sqlx::query_as(
        "select email, sms, phone from kentos.notification_preference where user_id = $1",
    )
    .bind(actor.user_id)
    .fetch_optional(&mut *tx)
    .await?;
    type Row = (
        String,
        String,
        String,
        String,
        i32,
        OffsetDateTime,
        Option<OffsetDateTime>,
        Option<String>,
    );
    let recent: Vec<Row> = sqlx::query_as(
        "select d.channel, d.address, d.state, n.title, d.attempts, d.created_at, d.sent_at, d.last_error
           from kentos.notification_delivery d join kentos.notification n on n.id = d.notification_id
          where d.user_id = $1 order by d.created_at desc limit 20",
    )
    .bind(actor.user_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    let (email, sms, phone) = pref.unwrap_or((true, false, None));
    Ok(NotificationSettings {
        email,
        sms,
        phone,
        email_address,
        email_verified,
        email_offered,
        sms_offered,
        recent: recent
            .into_iter()
            .map(
                |(channel, address, state, title, attempts, created, sent, error)| DeliveryView {
                    channel,
                    address,
                    state,
                    title,
                    attempts: u32::try_from(attempts).unwrap_or(0),
                    created_at: rfc3339(created),
                    sent_at: sent.map(rfc3339),
                    last_error: error,
                },
            )
            .collect(),
    })
}

/// `PUT /v1/me/notification-settings`.
pub async fn set_settings(
    db: &Db,
    actor: &Actor,
    input: &NotificationSettingsEdit,
) -> AppResult<NotificationSettings> {
    if actor.method == kentos_contracts::SignInMethod::Token {
        return Err(AppError::forbidden(
            "Bildirim tercihleri erişim anahtarıyla değiştirilmez; hesabınızla oturum açın.",
        ));
    }
    let phone = match input.phone.as_deref() {
        Some(p) => phone_of(p)?,
        None => None,
    };
    if input.sms && phone.is_none() {
        return Err(AppError::invalid_at(
            "phone",
            "SMS için telefon numaranızı yazın.",
        ));
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    sqlx::query(
        "insert into kentos.notification_preference (user_id, email, sms, phone) values ($1, $2, $3, $4)
         on conflict (user_id) do update set email = excluded.email, sms = excluded.sms, phone = excluded.phone,
                                             updated_at = now()",
    )
    .bind(actor.user_id)
    .bind(input.email)
    .bind(input.sms)
    .bind(&phone)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    settings(db, actor).await
}

/// A delivery the sender took: where it goes, what it says and where the notification opens.
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Due {
    pub id: Uuid,
    pub channel: String,
    pub address: String,
    pub attempts: i32,
    pub recipient: String,
    pub title: String,
    pub body: String,
    pub dossier_id: Option<Uuid>,
    pub tenant_slug: Option<String>,
    pub created_at: OffsetDateTime,
}

/// At most `batch` due deliveries of `channels`, each leased for `lease_secs`.
pub async fn take(db: &Db, channels: &[&str], batch: i32, lease_secs: i32) -> AppResult<Vec<Due>> {
    let channels: Vec<String> = channels.iter().map(|c| (*c).to_owned()).collect();
    Ok(
        sqlx::query_as("select * from kentos.take_deliveries($1, $2, $3)")
            .bind(&channels)
            .bind(batch)
            .bind(lease_secs)
            .fetch_all(&db.pool)
            .await?,
    )
}

/// The pause before the next try after `attempts` tries; none: given up.
pub fn retry_after(attempts: i32) -> Option<i32> {
    match attempts {
        ..=1 => Some(60),
        2 => Some(300),
        3 => Some(900),
        4 => Some(3600),
        5 => Some(14_400),
        _ => None,
    }
}

/// What became of a delivery.
pub async fn finish(db: &Db, due: &Due, result: Result<(), String>) -> AppResult<()> {
    let (sent, error, retry) = match result {
        Ok(()) => (true, None, None),
        Err(why) => (false, Some(why), retry_after(due.attempts)),
    };
    sqlx::query("select kentos.finish_delivery($1, $2, $3, $4)")
        .bind(due.id)
        .bind(sent)
        .bind(error)
        .bind(retry)
        .execute(&db.pool)
        .await?;
    Ok(())
}

/// Where a notification opens in the portal (`#/…`), as its recipient reaches it.
pub fn link(due: &Due) -> String {
    match (due.dossier_id, &due.tenant_slug) {
        (Some(d), Some(slug)) => format!("#/kurum/{slug}/dosya/{d}"),
        (Some(d), None) => format!("#/mesleki/dosya/{d}"),
        _ => "#/bildirimler".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_phone_is_kept_as_digits_and_tries_are_spaced_out_then_given_up() {
        assert_eq!(
            phone_of(" +90 (532) 123-45-67 ").unwrap().as_deref(),
            Some("+905321234567")
        );
        assert_eq!(
            phone_of("0532 123 45 67").unwrap().as_deref(),
            Some("05321234567")
        );
        assert_eq!(phone_of("").unwrap(), None);
        assert!(phone_of("12345").is_err() && phone_of("0532-ABC-4567").is_err());
        let pauses: Vec<_> = (1..=6).map(retry_after).collect();
        assert_eq!(
            pauses,
            vec![
                Some(60),
                Some(300),
                Some(900),
                Some(3600),
                Some(14_400),
                None
            ]
        );
    }
}
