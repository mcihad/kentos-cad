//! Signing in and out, and the caller of every protected route
//! (docs/adr/0007-authentication.md).
//!
//! - Browser: the session token is an HttpOnly, SameSite=Strict cookie. A
//!   request that changes something must also carry `x-kentos-client: web`;
//!   another site's page cannot add that header without a CORS preflight,
//!   which this server never grants, so cross-site request forgery fails.
//! - Desktop (docs/adr/0040): the same session, signed in with a local
//!   account; the program holds the token in memory and sends it as the
//!   cookie with `x-kentos-client: desktop`. Any custom header stops a
//!   cross-site form, so the rule is the same; the name only says who asked.
//! - API clients: `Authorization: Bearer <OpenID access token>`.

use axum::Json;
use axum::extract::{FromRequestParts, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Redirect, Response};
use kentos_application::identity::{self, Actor};
use kentos_application::{AppError, tenancy};
use kentos_contracts::{AuthConfig, LoginRequest, Me, OidcLoginInfo, SignInMethod, UserView};

use super::AppState;
use super::error::{Body, Failure, request_id};

pub const SESSION_COOKIE: &str = "kentos_session";
pub const CLIENT_HEADER: &str = "x-kentos-client";
/// The programs that send [`CLIENT_HEADER`]: the web app and the desktop app.
pub const CLIENTS: [&str; 2] = ["web", "desktop"];

pub fn cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|pair| {
            let (k, v) = pair.trim().split_once('=')?;
            (k == name).then(|| v.to_string())
        })
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

/// Unsafe methods with the session cookie must say they come from one of the apps (see the module comment).
pub fn check_client_header(
    parts_method: &axum::http::Method,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    if parts_method.is_safe()
        || headers
            .get(CLIENT_HEADER)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|c| CLIENTS.contains(&c))
    {
        Ok(())
    } else {
        Err(AppError::forbidden(
            "İstek uygulamanın kendisinden gelmiyor (x-kentos-client başlığı eksik).",
        ))
    }
}

pub fn session_cookie(state: &AppState, token: &str, max_age_secs: i64) -> HeaderValue {
    let secure = if state.config.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age_secs}{secure}"))
        .expect("cookie text is ASCII")
}

/// The verified caller of a protected route.
pub struct Caller(pub Actor);

impl FromRequestParts<AppState> for Caller {
    type Rejection = Failure;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let fail = |e: AppError| Failure::with(e, &parts.headers);
        let db = state.db().map_err(fail)?;
        if let Some(token) = bearer(&parts.headers) {
            let Some(oidc) = &state.oidc else {
                return Err(fail(AppError::Unauthenticated(
                    "Bu sunucu erişim anahtarı kabul etmiyor (OpenID yapılandırılmamış).".into(),
                )));
            };
            let actor = oidc.bearer_actor(db, token).await.map_err(fail)?;
            return Ok(Caller(actor));
        }
        let Some(token) = cookie(&parts.headers, SESSION_COOKIE) else {
            return Err(fail(AppError::Unauthenticated("Oturum açın.".into())));
        };
        check_client_header(&parts.method, &parts.headers).map_err(fail)?;
        match identity::session_actor(db, &token).await.map_err(fail)? {
            Some(actor) => Ok(Caller(actor)),
            None => Err(fail(AppError::Unauthenticated(
                "Oturumunuz sona erdi; yeniden giriş yapın.".into(),
            ))),
        }
    }
}

pub async fn config(State(state): State<AppState>) -> Json<AuthConfig> {
    Json(AuthConfig {
        local: state.config.local_login,
        oidc: state.config.oidc.as_ref().map(|o| OidcLoginInfo {
            label: o.label.clone(),
            start_url: "/v1/auth/oidc/start".into(),
        }),
    })
}

/// The account and its tenants. The personal space is opened here the first
/// time (signing in answers with this, and so does `/v1/me` after an OpenID
/// sign-in), so it is always among the memberships (docs/adr/0015).
pub async fn me_of(state: &AppState, actor: Actor) -> Result<Me, AppError> {
    tenancy::ensure_personal(state.db()?, &actor).await?;
    let memberships = tenancy::memberships(state.db()?, &actor).await?;
    Ok(Me {
        user: UserView {
            id: actor.user_id.to_string(),
            display_name: actor.display_name,
            email: actor.email,
            method: actor.method,
        },
        memberships,
    })
}

pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
    Caller(actor): Caller,
) -> Result<Json<Me>, Failure> {
    me_of(&state, actor)
        .await
        .map(Json)
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Body(req): Body<LoginRequest>,
) -> Result<Response, Failure> {
    let fail = |e: AppError| Failure::with(e, &headers);
    if !state.config.local_login {
        return Err(fail(AppError::forbidden(
            "Bu sunucuda yerel hesapla giriş kapalı; kurum hesabınızla girin.",
        )));
    }
    check_client_header(&axum::http::Method::POST, &headers).map_err(fail)?;
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
        tracing::info!(
            request_id = request_id(&headers).as_deref().unwrap_or("-"),
            "yerel giriş reddedildi"
        );
        state.logins.failed(&req.login);
        return Err(fail(AppError::Unauthenticated(
            "Giriş adı ya da parola yanlış.".into(),
        )));
    };
    state.logins.succeeded(&req.login);
    let token = identity::open_session(db, user, SignInMethod::Local)
        .await
        .map_err(fail)?;
    let actor = identity::actor_of(db, user, SignInMethod::Local)
        .await
        .map_err(fail)?
        .ok_or_else(|| fail(AppError::Unauthenticated("Hesap etkin değil.".into())))?;
    let me = me_of(&state, actor).await.map_err(fail)?;
    let cookie = session_cookie(
        &state,
        &token,
        i64::from(identity::SESSION_MAX_HOURS) * 3600,
    );
    Ok(([(header::SET_COOKIE, cookie)], Json(me)).into_response())
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, Failure> {
    let fail = |e: AppError| Failure::with(e, &headers);
    check_client_header(&axum::http::Method::POST, &headers).map_err(fail)?;
    if let Some(token) = cookie(&headers, SESSION_COOKIE) {
        identity::close_session(state.db().map_err(fail)?, &token)
            .await
            .map_err(fail)?;
    }
    Ok((
        [(header::SET_COOKIE, session_cookie(&state, "", 0))],
        axum::http::StatusCode::NO_CONTENT,
    )
        .into_response())
}

#[derive(serde::Deserialize)]
pub struct StartQuery {
    #[serde(rename = "returnTo")]
    return_to: Option<String>,
}

/// `GET /v1/auth/oidc/start`: redirects the browser to the organisation's sign-in page.
pub async fn oidc_start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<StartQuery>,
) -> Result<Response, Failure> {
    let fail = |e: AppError| Failure::with(e, &headers);
    let oidc = state.oidc.as_ref().ok_or_else(|| {
        fail(AppError::not_found(
            "Bu sunucuda OpenID girişi yapılandırılmamış.",
        ))
    })?;
    let url = oidc
        .start(
            state.db().map_err(fail)?,
            &crate::oidc::safe_return_to(q.return_to.as_deref()),
        )
        .await
        .map_err(fail)?;
    Ok(Redirect::to(&url).into_response())
}

#[derive(serde::Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// Back to the app with a reason the page can explain (`?oidc=error&reason=…`).
fn back_with_error(state: &AppState, reason: &str) -> Response {
    let mut url =
        reqwest::Url::parse(&format!("{}/", state.config.public_url)).expect("public URL is a URL");
    url.query_pairs_mut()
        .append_pair("oidc", "error")
        .append_pair("reason", reason);
    Redirect::to(url.as_str()).into_response()
}

/// `GET /v1/auth/oidc/callback`: the provider sends the browser back here with a code.
pub async fn oidc_callback(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let Some(oidc) = state.oidc.clone() else {
        return back_with_error(&state, "not_configured");
    };
    let (Some(code), Some(login_state), None) = (q.code, q.state, q.error.as_deref()) else {
        return back_with_error(&state, q.error.as_deref().unwrap_or("invalid"));
    };
    let Ok(db) = state.db() else {
        return back_with_error(&state, "unavailable");
    };
    match oidc.finish(db, &code, &login_state).await {
        Ok((token, return_to)) => {
            let cookie = session_cookie(
                &state,
                &token,
                i64::from(identity::SESSION_MAX_HOURS) * 3600,
            );
            (
                [(header::SET_COOKIE, cookie)],
                Redirect::to(&format!("{}{return_to}", state.config.public_url)),
            )
                .into_response()
        }
        Err(e) => {
            tracing::info!(error = %e, "OpenID girişi tamamlanamadı");
            back_with_error(&state, e.code())
        }
    }
}
