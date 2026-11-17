//! An organisation's portal routes (docs/adr/9000 §3–5): its settings and figures, its dossiers and the transfers it
//! opens, its directory of professionals and its members. Each starts from the caller's usable membership.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::AppError;
use kentos_application::portal::{catalog, dossiers, organization};
use kentos_application::tenancy::{self, Access};
use kentos_contracts::{
    DossierDashboard, DossierList, MemberCreate, MemberEdit, MemberView, OrganizationPortal,
    OrganizationPortalEdit, ProfessionalCard, TransferCreate,
};
use serde::{Deserialize, Serialize};

use super::{ListQuery, path_id};
use crate::http::AppState;
use crate::http::auth::Caller;
use crate::http::error::{Body, Failure};

async fn access(state: &AppState, caller: &Caller, tenant: &str) -> Result<Access, AppError> {
    tenancy::access(state.db()?, &caller.0, path_id(tenant, "Kurum")?).await
}

pub async fn portal(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
) -> Result<Json<OrganizationPortal>, Failure> {
    let run =
        async { organization::portal(state.db()?, &access(&state, &caller, &tenant).await?).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn set_portal(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Body(input): Body<OrganizationPortalEdit>,
) -> Result<Json<OrganizationPortal>, Failure> {
    let run = async {
        let a = access(&state, &caller, &tenant).await?;
        organization::set_portal(state.db()?, &a, &input).await?;
        organization::portal(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
) -> Result<Json<DossierDashboard>, Failure> {
    let run = async {
        dossiers::dashboard_for_tenant(state.db()?, &access(&state, &caller, &tenant).await?).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn dossiers(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Query(q): Query<ListQuery>,
) -> Result<Json<DossierList>, Failure> {
    let run = async {
        let a = access(&state, &caller, &tenant).await?;
        dossiers::list_for_tenant(state.db()?, &a, &q.filter()?).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    id: String,
}

/// `POST /v1/tenants/{tenant}/dossiers`: a transfer to a professional, as a draft.
pub async fn transfer(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Body(input): Body<TransferCreate>,
) -> Result<(StatusCode, Json<Opened>), Failure> {
    let run = async {
        let a = access(&state, &caller, &tenant).await?;
        dossiers::create_transfer(state.db()?, &a, &input).await
    };
    run.await
        .map(|id| (StatusCode::CREATED, Json(Opened { id: id.to_string() })))
        .map_err(|e| Failure::with(e, &headers))
}

#[derive(Debug, Default, Deserialize)]
pub struct DirectoryQuery {
    q: Option<String>,
    profession: Option<String>,
}

pub async fn directory(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Query(q): Query<DirectoryQuery>,
) -> Result<Json<Vec<ProfessionalCard>>, Failure> {
    let run = async {
        let a = access(&state, &caller, &tenant).await?;
        let profession = match q.profession.as_deref().filter(|p| !p.is_empty()) {
            Some(p) => Some(catalog::profession_of(p).ok_or_else(|| {
                AppError::invalid_at("profession", format!("“{p}” bir meslek değil."))
            })?),
            None => None,
        };
        organization::directory(state.db()?, &a, q.q.as_deref(), profession).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn members(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<MemberView>>, Failure> {
    let run = async {
        organization::members(state.db()?, &access(&state, &caller, &tenant).await?).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

pub async fn add_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Body(input): Body<MemberCreate>,
) -> Result<(StatusCode, Json<Vec<MemberView>>), Failure> {
    let run = async {
        let a = access(&state, &caller, &tenant).await?;
        organization::add_member(state.db()?, &a, &input).await?;
        organization::members(state.db()?, &a).await
    };
    run.await
        .map(|m| (StatusCode::CREATED, Json(m)))
        .map_err(|e| Failure::with(e, &headers))
}

pub async fn update_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, user)): Path<(String, String)>,
    Body(input): Body<MemberEdit>,
) -> Result<Json<Vec<MemberView>>, Failure> {
    let run = async {
        let a = access(&state, &caller, &tenant).await?;
        organization::update_member(state.db()?, &a, path_id(&user, "Üye")?, &input).await?;
        organization::members(state.db()?, &a).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}
