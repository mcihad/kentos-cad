//! Who is calling (docs/adr/0007-authentication.md). A browser session is a
//! random token in an HttpOnly cookie; the database keeps only its SHA-256.
//! Local passwords are checked by `kentos.check_local_login` (bcrypt in
//! pgcrypto), so the server's role never reads a password hash. OpenID
//! identities are found or created by `kentos.resolve_identity`.

use kentos_contracts::SignInMethod;
use kentos_postgres::{Db, Scope};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

/// Local accounts' issuer; their subject is the account id.
pub const LOCAL_ISSUER: &str = "kentos:local";
/// A session lives this long without use, and at most `SESSION_MAX_HOURS` in all.
pub const SESSION_IDLE_HOURS: i32 = 12;
pub const SESSION_MAX_HOURS: i32 = 24 * 7;

/// A verified caller. Everything a use case authorizes starts from this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    pub user_id: Uuid,
    pub display_name: String,
    pub email: Option<String>,
    pub method: SignInMethod,
}

/// Checks a local login; `None` for a wrong login or password (the two are not told apart).
pub async fn check_local_login(db: &Db, login: &str, password: &str) -> AppResult<Option<Uuid>> {
    if login.is_empty() || password.is_empty() || login.len() > 200 || password.len() > 200 {
        return Ok(None);
    }
    Ok(
        sqlx::query_scalar("select kentos.check_local_login($1, $2)")
            .bind(login)
            .bind(password)
            .fetch_one(&db.pool)
            .await?,
    )
}

/// Finds or creates the account of an OpenID identity; `None` when that account is disabled.
pub async fn resolve_identity(
    db: &Db,
    issuer: &str,
    subject: &str,
    name: &str,
    email: Option<&str>,
) -> AppResult<Option<Uuid>> {
    if issuer == LOCAL_ISSUER || issuer.is_empty() || subject.is_empty() {
        return Err(AppError::invalid(
            "Kimlik sağlayıcısı geçersiz bir kimlik gönderdi.",
        ));
    }
    let name = if name.trim().is_empty() {
        subject
    } else {
        name.trim()
    };
    Ok(
        sqlx::query_scalar("select kentos.resolve_identity($1, $2, $3, $4, $5)")
            .bind(Uuid::now_v7())
            .bind(issuer)
            .bind(subject)
            .bind(name)
            .bind(email)
            .fetch_one(&db.pool)
            .await?,
    )
}

/// Opens a session and returns its token (shown to the browser once, stored only hashed).
pub async fn open_session(db: &Db, user: Uuid, method: SignInMethod) -> AppResult<String> {
    let method = match method {
        SignInMethod::Local => "local",
        SignInMethod::Oidc => "oidc",
        SignInMethod::Bearer => {
            return Err(AppError::invalid("Erişim anahtarıyla oturum açılmaz."));
        }
    };
    let mut tx = db.pool.begin().await?;
    let token: String = sqlx::query_scalar("select encode(public.gen_random_bytes(32), 'hex')")
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query(
        "insert into kentos.auth_session (token_hash, user_id, method, expires_at)
         values (public.digest($1, 'sha256'), $2, $3, now() + make_interval(hours => $4))",
    )
    .bind(&token)
    .bind(user)
    .bind(method)
    .bind(SESSION_IDLE_HOURS)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(token)
}

fn looks_like_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The account behind a session token, extending the session's idle limit; `None` if expired or revoked.
pub async fn session_actor(db: &Db, token: &str) -> AppResult<Option<Actor>> {
    if !looks_like_token(token) {
        return Ok(None);
    }
    let row: Option<(Uuid, String)> = sqlx::query_as(
        "update kentos.auth_session
            set last_seen_at = now(),
                expires_at = least(now() + make_interval(hours => $2), created_at + make_interval(hours => $3))
          where token_hash = public.digest($1, 'sha256') and revoked_at is null and expires_at > now()
          returning user_id, method",
    )
    .bind(token)
    .bind(SESSION_IDLE_HOURS)
    .bind(SESSION_MAX_HOURS)
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

/// The active account `user` as an actor (`None` when missing or disabled).
pub async fn actor_of(db: &Db, user: Uuid, method: SignInMethod) -> AppResult<Option<Actor>> {
    let mut tx = db
        .scoped(Scope {
            user: Some(user),
            ..Scope::default()
        })
        .await?;
    let row: Option<(String, Option<String>, String)> =
        sqlx::query_as("select display_name, email, status from kentos.app_user where id = $1")
            .bind(user)
            .fetch_optional(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(row
        .filter(|(_, _, status)| status == "active")
        .map(|(display_name, email, _)| Actor {
            user_id: user,
            display_name,
            email,
            method,
        }))
}

/// Ends a session (sign out). Unknown tokens are ignored.
pub async fn close_session(db: &Db, token: &str) -> AppResult<()> {
    if looks_like_token(token) {
        sqlx::query("update kentos.auth_session set revoked_at = now() where token_hash = public.digest($1, 'sha256') and revoked_at is null")
            .bind(token)
            .execute(&db.pool)
            .await?;
    }
    Ok(())
}
