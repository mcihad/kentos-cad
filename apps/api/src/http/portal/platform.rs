//! The platform administrators' routes (docs/adr/9000 §3). Every use case asks the database whether the caller is
//! one; nothing here decides it.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::AppError;
use kentos_application::portal::{accounts, catalog, platform};
use kentos_contracts::{
    PlatformEventView, PlatformSettings, PlatformSettingsEdit, PlatformSummary, PlatformTenant,
    PlatformTenantCreate, PlatformTenantDetail, PlatformTenantEdit, PlatformUser,
    PlatformUserCreate, PlatformUserEdit, ProfessionalReview, ProfessionalView, Signup,
};
use serde::{Deserialize, Serialize};

use super::path_id;
use crate::http::AppState;
use crate::http::auth::Caller;
use crate::http::error::{Body, Failure};

/// What a create answers: the new record's id.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Created {
    id: String,
}

fn created(id: uuid::Uuid) -> (StatusCode, Json<Created>) {
    (StatusCode::CREATED, Json(Created { id: id.to_string() }))
}

pub async fn summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<PlatformSummary>, Failure> {
    let run = async { platform::summary(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn tenants(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<Vec<PlatformTenant>>, Failure> {
    let run = async { platform::tenants(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<Json<PlatformTenantDetail>, Failure> {
    let run = async { platform::tenant(state.db()?, &caller.0, path_id(&id, "Kurum")?).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn create_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<PlatformTenantCreate>,
) -> Result<(StatusCode, Json<Created>), Failure> {
    let run = async { platform::create_tenant(state.db()?, &caller.0, &input).await };
    run.await
        .map(created)
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn update_tenant(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<PlatformTenantEdit>,
) -> Result<Json<PlatformTenantDetail>, Failure> {
    let run = async {
        let db = state.db()?;
        let id = path_id(&id, "Kurum")?;
        platform::update_tenant(db, &caller.0, id, &input).await?;
        platform::tenant(db, &caller.0, id).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Debug, Default, Deserialize)]
pub struct Search {
    q: Option<String>,
    status: Option<String>,
    limit: Option<u32>,
}

pub async fn users(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Query(search): Query<Search>,
) -> Result<Json<Vec<PlatformUser>>, Failure> {
    let run = async { platform::users(state.db()?, &caller.0, search.q.as_deref()).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<PlatformUserCreate>,
) -> Result<(StatusCode, Json<Created>), Failure> {
    let run = async { platform::create_user(state.db()?, &caller.0, &input).await };
    run.await
        .map(created)
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn edit_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<PlatformUserEdit>,
) -> Result<StatusCode, Failure> {
    let run =
        async { platform::edit_user(state.db()?, &caller.0, path_id(&id, "Hesap")?, &input).await };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn professionals(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Query(search): Query<Search>,
) -> Result<Json<Vec<ProfessionalView>>, Failure> {
    let run = async {
        let status = match search.status.as_deref().filter(|s| !s.is_empty()) {
            Some(s) => Some(catalog::professional_status_of(s).ok_or_else(|| {
                AppError::invalid_at("status", format!("“{s}” bir kayıt durumu değil."))
            })?),
            None => None,
        };
        platform::professionals(state.db()?, &caller.0, status).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/platform/professionals`: an administrator opens a professional's account (it still waits for the check).
pub async fn create_professional(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<Signup>,
) -> Result<(StatusCode, Json<Created>), Failure> {
    let run = async { accounts::sign_up(state.db()?, &input, Some(&caller.0)).await };
    run.await
        .map(created)
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn review(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<ProfessionalReview>,
) -> Result<StatusCode, Failure> {
    let run = async {
        platform::review(
            state.db()?,
            &caller.0,
            path_id(&id, "Meslek mensubu")?,
            &input,
        )
        .await
    };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn events(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Query(search): Query<Search>,
) -> Result<Json<Vec<PlatformEventView>>, Failure> {
    let run = async { platform::events(state.db()?, &caller.0, search.limit.unwrap_or(100)).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// The platform's settings, with how this server sends notifications (its own settings, not the database's).
fn with_ways(state: &AppState, mut s: PlatformSettings) -> PlatformSettings {
    if let Some(d) = &state.config.delivery {
        (s.email_transport, s.sms_transport) = d.ways();
    }
    s
}

pub async fn settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<PlatformSettings>, Failure> {
    let run = async { platform::settings(state.db()?, &caller.0).await };
    run.await
        .map(|s| Json(with_ways(&state, s)))
        .map_err(|e| Failure::with(e, &headers))
}

/// `PUT /v1/platform/settings`: the fields the body names change.
pub async fn set_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<PlatformSettingsEdit>,
) -> Result<Json<PlatformSettings>, Failure> {
    let run = async {
        let db = state.db()?;
        platform::set_settings(db, &caller.0, &input).await?;
        platform::settings(db, &caller.0).await
    };
    run.await
        .map(|s| Json(with_ways(&state, s)))
        .map_err(|e| Failure::with(e, &headers))
}
