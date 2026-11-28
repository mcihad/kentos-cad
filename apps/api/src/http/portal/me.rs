//! The signed-in person on the portal: who they are, signing up as a professional (and staying signed in), their own
//! profile and password, the catalog, the municipalities to apply to, and their notifications.

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kentos_application::portal::{accounts, catalog as names, notifications, organization};
use kentos_application::{AppError, identity};
use kentos_contracts::{
    MunicipalityCard, NotificationSettings, NotificationSettingsEdit, Notifications,
    NotificationsRead, PasswordChange, PortalCatalog, PortalMe, ProfessionalEdit, SignInMethod,
    Signup, SignupInfo,
};
use serde::Deserialize;

use crate::http::AppState;
use crate::http::auth::{Caller, check_client_header, session_cookie};
use crate::http::error::{Body, Failure};
use crate::http::signup_guard::ClientAddr;

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
            return Ok(SignupInfo {
                open: false,
                ticket: None,
            });
        }
        let mut info = accounts::signup_info(state.db()?).await?;
        // The form's ticket (docs/adr/9000 §20): it must come back with the sign-up, a person's while later.
        info.ticket = info.open.then(|| state.signups.ticket(unix_now()));
        Ok(info)
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/portal/signup`: the professional's account, signed in at once (their profile waits for a check).
pub async fn sign_up(
    State(state): State<AppState>,
    headers: HeaderMap,
    client: ClientAddr,
    Body(input): Body<Signup>,
) -> Result<Response, Failure> {
    let fail = |e: AppError| Failure::with(e, &headers);
    if !state.config.local_login {
        return Err(fail(AppError::forbidden(
            "Bu sunucuda yerel hesapla giriş kapalı; kayıt da kapalı.",
        )));
    }
    check_client_header(&axum::http::Method::POST, &headers).map_err(fail)?;
    // A machine's sign-up (docs/adr/9000 §20): too many tries from one address, the field no person fills, a form
    // sent at once or without its ticket, too many accounts from one address.
    let address = client.key();
    let limited = |wait: u64, what: &str| AppError::Limited {
        message: format!(
            "{what}; {} dakika sonra yeniden deneyin.",
            wait.div_ceil(60)
        ),
        retry_after: wait,
    };
    state
        .signups
        .tries
        .hit(&address)
        .map_err(|w| fail(limited(w, "Bu adresten çok fazla kayıt denemesi geldi")))?;
    if input
        .website
        .as_deref()
        .is_some_and(|w| !w.trim().is_empty())
    {
        return Err(fail(AppError::invalid("Kayıt alınamadı.")));
    }
    state
        .signups
        .redeem(input.ticket.as_deref(), unix_now())
        .map_err(|why| fail(AppError::invalid_at("ticket", why)))?;
    if let Some(w) = state.signups.accounts.blocked(&address) {
        return Err(fail(limited(
            w,
            "Bu adresten bir saatte açılabilecek hesap sayısı doldu",
        )));
    }
    let db = state.db().map_err(fail)?;
    let user = accounts::sign_up(db, &input, None).await.map_err(fail)?;
    let _ = state.signups.accounts.hit(&address);
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
    let run =
        async { kentos_application::notification_delivery::settings(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `PUT /v1/me/notification-settings`.
pub async fn set_notification_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<NotificationSettingsEdit>,
) -> Result<Json<NotificationSettings>, Failure> {
    let run = async {
        kentos_application::notification_delivery::set_settings(state.db()?, &caller.0, &input)
            .await
    };
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

/// Seconds since 1970, for the sign-up's ticket.
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// `GET /v1/me/notifications/stream`: the caller's unread count as it changes (Server-Sent Events, docs/adr/9000 §23).
/// The first event is the count now; then one each time a notification of the caller's is written or read, from any
/// server process. Each count is read with the caller's own rights; a lagging stream is sent the count again.
pub async fn stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<
    axum::response::sse::Sse<
        impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
    >,
    Failure,
> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use tokio::sync::broadcast::error::RecvError;
    let db = state.db().map_err(|e| Failure::with(e, &headers))?.clone();
    let actor = caller.0;
    let rx = state.notices.subscribe();
    let events = futures_util::stream::unfold(
        (db, actor, rx, true),
        |(db, actor, mut rx, first)| async move {
            if !first {
                loop {
                    match rx.recv().await {
                        Ok(user) if user == actor.user_id => break,
                        Ok(_) => continue,
                        Err(RecvError::Lagged(_)) => break,
                        Err(RecvError::Closed) => return None,
                    }
                }
            }
            let unread = notifications::unread(&db, &actor).await.ok()?;
            let event = Event::default()
                .event("unread")
                .data(serde_json::json!({ "unread": unread }).to_string());
            Some((Ok(event), (db, actor, rx, false)))
        },
    );
    Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(25))))
}
