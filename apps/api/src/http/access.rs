//! Programs signing in and an account's sessions and personal access tokens (docs/adr/9000 §12): `POST /v1/auth/token`
//! gives a program its session as a bearer token; `/v1/me/sessions` lists and ends the account's sessions, browser or
//! program; `/v1/me/tokens` makes, lists and revokes its personal access tokens.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::{AppError, api_keys, identity};
use kentos_contracts::{
    ApiTokenCreate, ApiTokenCreated, ApiTokenView, SessionView, SignInMethod, TokenIssued,
    TokenRequest,
};
use uuid::Uuid;

use super::AppState;
use super::auth::{Caller, SESSION_COOKIE, bearer, cookie, me_of};
use super::error::{Body, Failure};

/// `POST /v1/auth/token`: a program signs in with a local account and gets its session as a bearer token.
pub async fn token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Body(req): Body<TokenRequest>,
) -> Result<(StatusCode, Json<TokenIssued>), Failure> {
    let fail = |e: AppError| Failure::with(e, &headers);
    if !state.config.local_login {
        return Err(fail(AppError::forbidden(
            "Bu sunucuda yerel hesapla giriş kapalı; kurum hesabınızla girin.",
        )));
    }
    let db = state.db().map_err(fail)?;
    if let Some(retry_after) = state.logins.blocked(&req.login) {
        return Err(fail(AppError::Limited {
            message: format!(
                "Bu hesapla çok sayıda yanlış giriş denendi; {} dakika sonra yeniden deneyin.",
                retry_after.div_ceil(60)
            ),
            retry_after,
        }));
    }
    let Some(user) = identity::check_local_login(db, req.login.trim(), &req.password)
        .await
        .map_err(fail)?
    else {
        state.logins.failed(&req.login);
        return Err(fail(AppError::Unauthenticated(
            "Giriş adı ya da parola yanlış.".into(),
        )));
    };
    state.logins.succeeded(&req.login);
    let (token, ends) =
        api_keys::open_bearer_session(db, user, req.client.as_deref(), req.label.as_deref())
            .await
            .map_err(fail)?;
    let actor = identity::actor_of(db, user, SignInMethod::Local)
        .await
        .map_err(fail)?
        .ok_or_else(|| fail(AppError::Unauthenticated("Hesap etkin değil.".into())))?;
    let me = me_of(&state, actor).await.map_err(fail)?;
    Ok((
        StatusCode::CREATED,
        Json(TokenIssued {
            token,
            expires_at: ends
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            me,
        }),
    ))
}

/// The token this request came with: a program's bearer session or a browser's cookie.
fn current_token(headers: &HeaderMap) -> Option<String> {
    bearer(headers)
        .filter(|t| t.starts_with(api_keys::SESSION_PREFIX))
        .map(str::to_owned)
        .or_else(|| cookie(headers, SESSION_COOKIE))
}

/// `GET /v1/me/sessions`.
pub async fn sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<Vec<SessionView>>, Failure> {
    let run = async {
        api_keys::sessions(state.db()?, &caller.0, current_token(&headers).as_deref()).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `DELETE /v1/me/sessions/{id}`.
pub async fn end_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    let run = async {
        let id = Uuid::parse_str(&id).map_err(|_| AppError::not_found("Oturum bulunamadı."))?;
        api_keys::end_session(state.db()?, &caller.0, id).await
    };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/me/tokens`.
pub async fn tokens(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<Vec<ApiTokenView>>, Failure> {
    let run = async { api_keys::tokens(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/me/tokens`.
pub async fn create_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<ApiTokenCreate>,
) -> Result<(StatusCode, Json<ApiTokenCreated>), Failure> {
    let run = async { api_keys::create_token(state.db()?, &caller.0, &input).await };
    run.await
        .map(|t| (StatusCode::CREATED, Json(t)))
        .map_err(|e| Failure::with(e, &headers))
}

/// `DELETE /v1/me/tokens/{id}`.
pub async fn revoke_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    let run = async {
        let id = Uuid::parse_str(&id).map_err(|_| AppError::not_found("Anahtar bulunamadı."))?;
        api_keys::revoke_token(state.db()?, &caller.0, id).await
    };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}
