//! The command log behind idempotency keys (CLAUDE.md §13.1): a retried
//! command gets the answer stored the first time, never a second commit.
//! Every product command on a project uses it inside its own transaction,
//! under the project's lock. The log is read only in the project's scope
//! (migration 0004); a key another project's command already took is refused
//! like any key reused for a different request.

use kentos_contracts::CommandEnvelope;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::{Postgres, Transaction};

use crate::access::ProjectAccess;
use crate::error::{AppError, AppResult};

/// The key and the request id are within their limits.
pub(crate) fn check_key(envelope: &CommandEnvelope) -> AppResult<()> {
    if !(8..=200).contains(&envelope.idempotency_key.len()) || envelope.request_id.len() > 200 {
        return Err(AppError::invalid(
            "idempotencyKey 8–200 karakter, requestId en çok 200 karakter olmalı.",
        ));
    }
    Ok(())
}

/// The input's canonical text, whose hash tells a retry from a different request with the same key.
pub(crate) fn request_text(envelope: &CommandEnvelope) -> String {
    serde_json::json!({
        "command": envelope.command_name,
        "version": envelope.version,
        "project": envelope.project_id,
        "expected": envelope.expected_versions,
        "input": envelope.input,
    })
    .to_string()
}

fn reused() -> AppError {
    AppError::invalid(
        "Bu idempotency anahtarı başka bir istek için kullanılmış; her komuta yeni bir anahtar verin.",
    )
}

/// The answer stored for this key when the same request was committed before.
pub(crate) async fn earlier<T: DeserializeOwned>(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
) -> AppResult<Option<T>> {
    let row: Option<(bool, Value)> = sqlx::query_as(
        "select request_hash = public.digest($3, 'sha256'), response from kentos.command_log where tenant_id = $1 and idempotency_key = $2",
    )
    .bind(access.tenant)
    .bind(&envelope.idempotency_key)
    .bind(text)
    .fetch_optional(&mut **tx)
    .await?;
    match row {
        None => Ok(None),
        Some((false, _)) => Err(reused()),
        Some((true, response)) => serde_json::from_value(response)
            .map(Some)
            .map_err(|e| AppError::invalid(format!("Saklı yanıt okunamadı: {e}"))),
    }
}

/// Stores the answer with the key, in the command's transaction.
pub(crate) async fn record<T: Serialize>(
    tx: &mut Transaction<'static, Postgres>,
    access: &ProjectAccess,
    envelope: &CommandEnvelope,
    text: &str,
    result: &T,
) -> AppResult<()> {
    let response = serde_json::to_value(result)
        .map_err(|e| AppError::invalid(format!("Yanıt saklanamadı: {e}")))?;
    let done = sqlx::query(
        "insert into kentos.command_log (tenant_id, idempotency_key, project_id, command_name, request_hash, response, actor)
         values ($1, $2, $3, $4, public.digest($5, 'sha256'), $6, $7)",
    )
    .bind(access.tenant)
    .bind(&envelope.idempotency_key)
    .bind(access.project)
    .bind(&envelope.command_name)
    .bind(text)
    .bind(response)
    .bind(access.actor.user_id)
    .execute(&mut **tx)
    .await;
    match done {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(d)) if d.code().as_deref() == Some("23505") => Err(reused()),
        Err(e) => Err(e.into()),
    }
}
