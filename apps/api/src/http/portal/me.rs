//! The signed-in person on the portal: who they are, signing up as a professional (and staying signed in), their own
//! profile and password, the catalog, the municipalities to apply to, and their notifications.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kentos_application::portal::{accounts, catalog as names, notifications, organization};
use kentos_application::{AppError, identity};
use kentos_contracts::{
    NotificationSettings, NotificationSettingsEdit,
    MunicipalityCard, Notifications, NotificationsRead, PasswordChange, PortalCatalog, PortalMe,
    ProfessionalEdit, SignInMethod, Signup, SignupInfo,
};
use serde::Deserialize;

use crate::http::AppState;
use crate::http::auth::{Caller, check_client_header, session_cookie};
use crate::http::error::{Body, Failure};

/// `GET /v1/portal/me`.
pub async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<PortalMe>, Failure> {
    let run = async { accounts::me(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/portal/catalog`: open to everyone (signing up shows the professions).
pub async fn catalog() -> Json<PortalCatalog> {
    Json(names::catalog())
}

/// `GET /v1/portal/signup`.
pub async fn signup_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SignupInfo>, Failure> {
    let run = async {
        if !state.config.local_login {
            return Ok(SignupInfo { open: false });
        }
        accounts::signup_info(state.db()?).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/portal/signup`: the professional's account, signed in at once (their profile waits for a check).
pub async fn sign_up(
    State(state): State<AppState>,
    headers: HeaderMap,
    Body(input): Body<Signup>,
) -> Result<Response, Failure> {
    let fail = |e: AppError| Failure::with(e, &headers);
    if !state.config.local_login {
        return Err(fail(AppError::forbidden(
            "Bu sunucuda yerel hesapla giriş kapalı; kayıt da kapalı.",
        )));
    }
    check_client_header(&axum::http::Method::POST, &headers).map_err(fail)?;
    let db = state.db().map_err(fail)?;
    let user = accounts::sign_up(db, &input, None).await.map_err(fail)?;
    let token = identity::open_session(db, user, SignInMethod::Local)
        .await
        .map_err(fail)?;
    let actor = identity::actor_of(db, user, SignInMethod::Local)
        .await
        .map_err(fail)?
        .ok_or_else(|| fail(AppError::Unauthenticated("Hesap etkin değil.".into())))?;
    let me = accounts::me(db, &actor).await.map_err(fail)?;
    let cookie = session_cookie(
        &state,
        &token,
        i64::from(identity::SESSION_MAX_HOURS) * 3600,
    );
    Ok((
        StatusCode::CREATED,
        [(header::SET_COOKIE, cookie)],
        Json(me),
    )
        .into_response())
}

/// `PUT /v1/portal/me/professional`.
pub async fn edit_professional(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<ProfessionalEdit>,
) -> Result<Json<PortalMe>, Failure> {
    let run = async {
        let db = state.db()?;
        accounts::edit_own(db, &caller.0, &input).await?;
        accounts::me(db, &caller.0).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/portal/me/password`: every session closes; this one is opened again.
pub async fn password(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<PasswordChange>,
) -> Result<Response, Failure> {
    let fail = |e: AppError| Failure::with(e, &headers);
    let db = state.db().map_err(fail)?;
    accounts::change_password(db, &caller.0, &input)
        .await
        .map_err(fail)?;
    let token = identity::open_session(db, caller.0.user_id, caller.0.method)
        .await
        .map_err(fail)?;
    let cookie = session_cookie(
        &state,
        &token,
        i64::from(identity::SESSION_MAX_HOURS) * 3600,
    );
    Ok((StatusCode::NO_CONTENT, [(header::SET_COOKIE, cookie)]).into_response())
}

/// `GET /v1/me/notification-settings`: whether notifications also come by e-mail and SMS, and what came lately.
pub async fn notification_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<NotificationSettings>, Failure> {
    let run = async { kentos_application::notification_delivery::settings(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `PUT /v1/me/notification-settings`.
pub async fn set_notification_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<NotificationSettingsEdit>,
) -> Result<Json<NotificationSettings>, Failure> {
    let run = async { kentos_application::notification_delivery::set_settings(state.db()?, &caller.0, &input).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/portal/municipalities`.
pub async fn municipalities(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<Vec<MunicipalityCard>>, Failure> {
    let run = async { organization::municipalities(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Debug, Default, Deserialize)]
pub struct NotificationQuery {
    unread: Option<bool>,
    limit: Option<u32>,
}

/// `GET /v1/me/notifications`.
pub async fn notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Query(q): Query<NotificationQuery>,
) -> Result<Json<Notifications>, Failure> {
    let run = async {
        notifications::list(
            state.db()?,
            &caller.0,
            q.unread.unwrap_or(false),
            q.limit.unwrap_or(50),
        )
        .await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/me/notifications/read`: answers with the list as it is now.
pub async fn read(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<NotificationsRead>,
) -> Result<Json<Notifications>, Failure> {
    let run = async {
        let db = state.db()?;
        notifications::mark_read(db, &caller.0, &input).await?;
        notifications::list(db, &caller.0, false, 50).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}
