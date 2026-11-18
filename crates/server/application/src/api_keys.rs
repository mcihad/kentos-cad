//! Programs signing in to the cloud (docs/adr/9000 §12). A program (the desktop, a mobile app, an AI agent) signs in
//! with a local account and gets its session as a bearer token, `kentos_s_…`; an account makes personal access
//! tokens, `kentos_pat_…`, for agents and scripts. Either goes as `Authorization: Bearer …`; a bearer request needs no
//! `x-kentos-client` header, which guards cookies only (docs/adr/0007).
//!
//! - **A program's session** lives longer than a browser's: a week idle and thirty days at most (a browser's: twelve
//!   hours and seven days). A program keeps its token in the system's own key store, out of a web page's reach. The
//!   account sees every session, browser or program, and ends any of them.
//! - **A personal access token** reads only or reads and writes, and expires after 1–365 days. It never makes another
//!   token and never changes the password: those need a signed-in session. At most 20 are active per account.
//! - **Storage:** a token is shown once; only its SHA-256 is kept, made by PostgreSQL (`digest`), as for sessions.

use kentos_contracts::{
    ApiTokenCreate, ApiTokenCreated, ApiTokenView, SessionView, SignInMethod, TokenScope,
};
use kentos_postgres::{Db, Scope};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::identity::{Actor, actor_of};

/// A program's session token begins with this.
pub const SESSION_PREFIX: &str = "kentos_s_";
/// A personal access token begins with this.
pub const TOKEN_PREFIX: &str = "kentos_pat_";
/// A program's session ends after this many hours unused…
pub const BEARER_IDLE_HOURS: i32 = 24 * 7;
/// …and this many hours after it began, at the latest.
pub const BEARER_MAX_HOURS: i32 = 24 * 30;
/// Active personal access tokens an account may hold.
pub const TOKENS_PER_ACCOUNT: i64 = 20;

fn rfc3339(t: OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Whether `token` is `prefix` and 64 hexadecimal digits.
fn shaped(token: &str, prefix: &str) -> bool {
    token
        .strip_prefix(prefix)
        .is_some_and(|rest| rest.len() == 64 && rest.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// A program's name as it gives it: lower-case letters, digits and `-`, starting with a letter.
fn client_of(text: Option<&str>) -> AppResult<Option<String>> {
    let Some(c) = text.map(str::trim).filter(|c| !c.is_empty()) else {
        return Ok(None);
    };
    let ok = (2..=31).contains(&c.len())
        && c.bytes().next().is_some_and(|b| b.is_ascii_lowercase())
        && c.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if ok {
        Ok(Some(c.to_owned()))
    } else {
        Err(AppError::invalid_at(
            "client",
            "Programın adı küçük harf, rakam ve - ile yazılır (desktop, mobile, agent…).",
        ))
    }
}

fn label_of(text: Option<&str>) -> AppResult<Option<String>> {
    match text.map(str::trim).filter(|l| !l.is_empty()) {
        Some(l) if l.chars().count() > 80 => Err(AppError::invalid_at(
            "label",
            "Cihazın adı en çok 80 karakter olabilir.",
        )),
        l => Ok(l.map(str::to_owned)),
    }
}

/// Opens a program's session for `user`; its token and the latest moment it ends.
pub async fn open_bearer_session(
    db: &Db,
    user: Uuid,
    client: Option<&str>,
    label: Option<&str>,
) -> AppResult<(String, OffsetDateTime)> {
    let client = client_of(client)?;
    let label = label_of(label)?;
    let mut tx = db.pool.begin().await?;
    let token: String =
        sqlx::query_scalar("select $1 || encode(public.gen_random_bytes(32), 'hex')")
            .bind(SESSION_PREFIX)
            .fetch_one(&mut *tx)
            .await?;
    let ends: OffsetDateTime = sqlx::query_scalar(
        "insert into kentos.auth_session (token_hash, user_id, method, kind, client, label, expires_at)
         values (public.digest($1, 'sha256'), $2, 'local', 'bearer', $3, $4, now() + make_interval(hours => $5))
         returning created_at + make_interval(hours => $6)",
    )
    .bind(&token)
    .bind(user)
    .bind(&client)
    .bind(&label)
    .bind(BEARER_IDLE_HOURS)
    .bind(BEARER_MAX_HOURS)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((token, ends))
}

/// The account behind a program's session token, moving its idle limit; `None` if expired, revoked or not one.
pub async fn bearer_actor(db: &Db, token: &str) -> AppResult<Option<Actor>> {
    if !shaped(token, SESSION_PREFIX) {
        return Ok(None);
    }
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "update kentos.auth_session
            set last_seen_at = now(),
                expires_at = least(now() + make_interval(hours => $2), created_at + make_interval(hours => $3))
          where token_hash = public.digest($1, 'sha256') and kind = 'bearer' and revoked_at is null
            and expires_at > now()
          returning user_id, method",
    )
    .bind(token)
    .bind(BEARER_IDLE_HOURS)
    .bind(BEARER_MAX_HOURS)
    .fetch_optional(&db.pool)
    .await?;
    let Some((user, method)) = row else {
        return Ok(None);
    };
    let method = if method == "oidc" {
        SignInMethod::Oidc
    } else {
        SignInMethod::Local
    };
    actor_of(db, user, method).await
}

/// Ends a program's session (signing out). An unknown token is ignored.
pub async fn close_bearer_session(db: &Db, token: &str) -> AppResult<()> {
    if shaped(token, SESSION_PREFIX) {
        sqlx::query(
            "update kentos.auth_session set revoked_at = now()
              where token_hash = public.digest($1, 'sha256') and kind = 'bearer' and revoked_at is null",
        )
        .bind(token)
        .execute(&db.pool)
        .await?;
    }
    Ok(())
}

/// The account and scope behind a personal access token; `None` if expired, revoked, the account closed, or not one.
pub async fn token_actor(db: &Db, token: &str) -> AppResult<Option<(Actor, TokenScope)>> {
    if !shaped(token, TOKEN_PREFIX) {
        return Ok(None);
    }
    let row: Option<(Uuid, String)> =
        sqlx::query_as("select account, token_scope from kentos.api_token_user($1)")
            .bind(token)
            .fetch_optional(&db.pool)
            .await?;
    let Some((user, scope)) = row else {
        return Ok(None);
    };
    let scope = if scope == "read" {
        TokenScope::Read
    } else {
        TokenScope::Write
    };
    Ok(actor_of(db, user, SignInMethod::Token)
        .await?
        .map(|a| (a, scope)))
}

type SessionRow = (
    Uuid,
    String,
    Option<String>,
    Option<String>,
    String,
    OffsetDateTime,
    OffsetDateTime,
    OffsetDateTime,
    bool,
);

/// `GET /v1/me/sessions`: the account's live sessions, the latest used first; `current` is the request's own token
/// (a cookie's or a program's).
pub async fn sessions(
    db: &Db,
    actor: &Actor,
    current: Option<&str>,
) -> AppResult<Vec<SessionView>> {
    let rows: Vec<SessionRow> = sqlx::query_as(
        "select id, kind, client, label, method, created_at, last_seen_at, expires_at,
                coalesce(token_hash = public.digest($2, 'sha256'), false)
           from kentos.auth_session
          where user_id = $1 and revoked_at is null and expires_at > now()
          order by last_seen_at desc",
    )
    .bind(actor.user_id)
    .bind(current.unwrap_or_default())
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, kind, client, label, method, created, seen, expires, current)| SessionView {
                id: id.to_string(),
                kind,
                client,
                label,
                method,
                created_at: rfc3339(created),
                last_seen_at: rfc3339(seen),
                expires_at: rfc3339(expires),
                current,
            },
        )
        .collect())
}

/// `DELETE /v1/me/sessions/{id}`: ends one of the account's sessions.
pub async fn end_session(db: &Db, actor: &Actor, id: Uuid) -> AppResult<()> {
    let done = sqlx::query(
        "update kentos.auth_session set revoked_at = now()
          where id = $1 and user_id = $2 and revoked_at is null",
    )
    .bind(id)
    .bind(actor.user_id)
    .execute(&db.pool)
    .await?;
    if done.rows_affected() == 0 {
        return Err(AppError::not_found(
            "Oturum bulunamadı ya da zaten kapandı.",
        ));
    }
    Ok(())
}

fn user_scope(actor: &Actor) -> Scope {
    Scope {
        tenant: None,
        user: Some(actor.user_id),
        project: None,
    }
}

type TokenRow = (
    Uuid,
    String,
    String,
    String,
    OffsetDateTime,
    OffsetDateTime,
    Option<OffsetDateTime>,
);

fn token_view((id, name, prefix, scope, created, expires, used): TokenRow) -> ApiTokenView {
    ApiTokenView {
        id: id.to_string(),
        name,
        prefix,
        scope: if scope == "read" {
            TokenScope::Read
        } else {
            TokenScope::Write
        },
        created_at: rfc3339(created),
        expires_at: rfc3339(expires),
        last_used_at: used.map(rfc3339),
    }
}

const TOKEN_SELECT: &str =
    "select id, name, prefix, scope, created_at, expires_at, last_used_at from kentos.api_token";

/// `GET /v1/me/tokens`: the account's live personal access tokens, newest first.
pub async fn tokens(db: &Db, actor: &Actor) -> AppResult<Vec<ApiTokenView>> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let rows: Vec<TokenRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{TOKEN_SELECT} where user_id = $1 and revoked_at is null and expires_at > now() order by created_at desc"
    )))
    .bind(actor.user_id)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(rows.into_iter().map(token_view).collect())
}

/// A token or a program's session that is not a signed-in person at the keyboard may not make tokens.
fn signed_in_person(actor: &Actor, what: &str) -> AppResult<()> {
    if actor.method == SignInMethod::Token {
        return Err(AppError::forbidden(format!(
            "{what} erişim anahtarıyla yapılmaz; hesabınızla oturum açarak yapın."
        )));
    }
    Ok(())
}

/// `POST /v1/me/tokens`: a personal access token, shown this once.
pub async fn create_token(
    db: &Db,
    actor: &Actor,
    input: &ApiTokenCreate,
) -> AppResult<ApiTokenCreated> {
    signed_in_person(actor, "Yeni anahtar")?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(AppError::invalid_at(
            "name",
            "Anahtarın adı 1–80 karakter olmalı (neye verdiğinizi hatırlatan bir ad).",
        ));
    }
    if !(1..=365).contains(&input.expires_in_days) {
        return Err(AppError::invalid_at(
            "expiresInDays",
            "Anahtar 1 ile 365 gün arasında geçerli olur.",
        ));
    }
    let mut tx = db.scoped(user_scope(actor)).await?;
    let live: i64 = sqlx::query_scalar(
        "select count(*) from kentos.api_token where user_id = $1 and revoked_at is null and expires_at > now()",
    )
    .bind(actor.user_id)
    .fetch_one(&mut *tx)
    .await?;
    if live >= TOKENS_PER_ACCOUNT {
        return Err(AppError::invalid(format!(
            "Hesabın en çok {TOKENS_PER_ACCOUNT} etkin anahtarı olur; kullanmadığınız bir anahtarı silin."
        )));
    }
    let token: String =
        sqlx::query_scalar("select $1 || encode(public.gen_random_bytes(32), 'hex')")
            .bind(TOKEN_PREFIX)
            .fetch_one(&mut *tx)
            .await?;
    let prefix: String = token.chars().take(TOKEN_PREFIX.len() + 4).collect();
    let row: TokenRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "with made as (
           insert into kentos.api_token (id, user_id, name, token_hash, prefix, scope, expires_at)
           values ($1, $2, $3, public.digest($4, 'sha256'), $5, $6, now() + make_interval(days => $7))
           returning *)
         {}",
        TOKEN_SELECT.replace("from kentos.api_token", "from made")
    )))
    .bind(Uuid::now_v7())
    .bind(actor.user_id)
    .bind(name)
    .bind(&token)
    .bind(&prefix)
    .bind(match input.scope {
        TokenScope::Read => "read",
        TokenScope::Write => "write",
    })
    .bind(i32::try_from(input.expires_in_days).unwrap_or(365))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ApiTokenCreated {
        token,
        view: token_view(row),
    })
}

/// `DELETE /v1/me/tokens/{id}`: the token stops working at once.
pub async fn revoke_token(db: &Db, actor: &Actor, id: Uuid) -> AppResult<()> {
    let mut tx = db.scoped(user_scope(actor)).await?;
    let done = sqlx::query(
        "update kentos.api_token set revoked_at = now() where id = $1 and user_id = $2 and revoked_at is null",
    )
    .bind(id)
    .bind(actor.user_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    if done.rows_affected() == 0 {
        return Err(AppError::not_found(
            "Anahtar bulunamadı ya da zaten silindi.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_told_apart_by_their_prefix_and_shape() {
        let hex = "a".repeat(64);
        assert!(shaped(&format!("{SESSION_PREFIX}{hex}"), SESSION_PREFIX));
        assert!(!shaped(&format!("{TOKEN_PREFIX}{hex}"), SESSION_PREFIX));
        assert!(shaped(&format!("{TOKEN_PREFIX}{hex}"), TOKEN_PREFIX));
        assert!(!shaped(
            &format!("{TOKEN_PREFIX}{}", "g".repeat(64)),
            TOKEN_PREFIX
        ));
        assert!(!shaped("eyJhbGciOiJSUzI1NiJ9.e30.sig", TOKEN_PREFIX));
        assert_eq!(
            client_of(Some(" mobile ")).unwrap().as_deref(),
            Some("mobile")
        );
        assert!(client_of(Some("Mobil Uygulama")).is_err());
        assert!(client_of(None).unwrap().is_none());
    }
}
