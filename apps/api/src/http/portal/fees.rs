//! Fees and charges (docs/adr/9003; plan BUL-22): an organisation's tariffs (its admins write them, its members read
//! them) and a dossier's assessments (both sides read them; the municipality issues them, records their payment and
//! cancels them). The rules are `kentos_application::portal::fees`.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::AppError;
use kentos_application::portal::fees;
use kentos_application::tenancy::{self, Access};
use kentos_contracts::{
    FeeAssess, FeeAssessment, FeeCancel, FeeDefinition, FeePaid, FeePreview, FeeSchedule, FeeScheduleCopy,
    FeeScheduleEdit, FeeTry,
};

use super::path_id;
use crate::http::AppState;
use crate::http::auth::Caller;
use crate::http::error::{Body, Failure};

async fn access(state: &AppState, caller: &Caller, tenant: &str) -> Result<Access, AppError> {
    tenancy::access(state.db()?, &caller.0, path_id(tenant, "Kurum")?).await
}

fn schedule_id(id: &str) -> Result<uuid::Uuid, AppError> {
    path_id(id, "Ücret tarifesi")
}

/// `GET /v1/tenants/{tenant}/fees/schedules`: the organisation's tariffs, newest year first.
pub async fn schedules(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
) -> Result<Json<Vec<FeeSchedule>>, Failure> {
    let run = async { fees::schedules(state.db()?, &access(&state, &caller, &tenant).await?).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/tenants/{tenant}/fees/schedules`: a new draft tariff (201).
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Body(input): Body<FeeScheduleEdit>,
) -> Result<(StatusCode, Json<FeeSchedule>), Failure> {
    let run = async { fees::create_schedule(state.db()?, &access(&state, &caller, &tenant).await?, &input).await };
    run.await.map(|s| (StatusCode::CREATED, Json(s))).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/tenants/{tenant}/fees/schedules/{id}`.
pub async fn schedule(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, id)): Path<(String, String)>,
) -> Result<Json<FeeSchedule>, Failure> {
    let run = async {
        fees::schedule(state.db()?, &access(&state, &caller, &tenant).await?, schedule_id(&id)?).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `PUT /v1/tenants/{tenant}/fees/schedules/{id}`: a draft's new text.
pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, id)): Path<(String, String)>,
    Body(input): Body<FeeScheduleEdit>,
) -> Result<Json<FeeSchedule>, Failure> {
    let run = async {
        let access = access(&state, &caller, &tenant).await?;
        fees::update_schedule(state.db()?, &access, schedule_id(&id)?, &input).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `DELETE /v1/tenants/{tenant}/fees/schedules/{id}`: a draft (204).
pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, id)): Path<(String, String)>,
) -> Result<StatusCode, Failure> {
    let run = async {
        let access = access(&state, &caller, &tenant).await?;
        fees::delete_schedule(state.db()?, &access, schedule_id(&id)?).await
    };
    run.await.map(|()| StatusCode::NO_CONTENT).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/tenants/{tenant}/fees/schedules/{id}/activate`: the draft in force; the one it replaces retired.
pub async fn activate(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, id)): Path<(String, String)>,
) -> Result<Json<FeeSchedule>, Failure> {
    let run = async {
        let access = access(&state, &caller, &tenant).await?;
        fees::activate(state.db()?, &access, schedule_id(&id)?).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/tenants/{tenant}/fees/schedules/{id}/copy`: next year's draft, revalued (201).
pub async fn copy(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((tenant, id)): Path<(String, String)>,
    Body(input): Body<FeeScheduleCopy>,
) -> Result<(StatusCode, Json<FeeSchedule>), Failure> {
    let run = async {
        let access = access(&state, &caller, &tenant).await?;
        fees::copy_schedule(state.db()?, &access, schedule_id(&id)?, &input).await
    };
    run.await.map(|s| (StatusCode::CREATED, Json(s))).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/tenants/{tenant}/fees/template`: the structure of 2464 sayılı Kanun's fees, every rate 0, to start from.
pub async fn template(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
) -> Result<Json<FeeDefinition>, Failure> {
    let run = async {
        access(&state, &caller, &tenant).await?;
        Ok::<_, AppError>(fees::template())
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/tenants/{tenant}/fees/try`: a tariff tried on a kind's inputs before it is saved.
pub async fn try_out(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Body(input): Body<FeeTry>,
) -> Result<Json<FeePreview>, Failure> {
    let run = async {
        access(&state, &caller, &tenant).await?;
        fees::try_out(&input)
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/dossiers/{id}/fees`: the dossier's assessments, both sides.
pub async fn dossier_fees(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<Json<Vec<FeeAssessment>>, Failure> {
    let run = async { fees::dossier_fees(state.db()?, &caller.0, path_id(&id, "Dosya")?).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/dossiers/{id}/fees/preview`: what the tariff in force makes of these inputs; nothing is saved.
pub async fn preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<FeeAssess>,
) -> Result<Json<FeePreview>, Failure> {
    let run = async { fees::preview(state.db()?, &caller.0, path_id(&id, "Dosya")?, &input).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/dossiers/{id}/fees`: the assessment issued (201); the professional's side is told.
pub async fn issue(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<FeeAssess>,
) -> Result<(StatusCode, Json<FeeAssessment>), Failure> {
    let run = async { fees::issue(state.db()?, &caller.0, path_id(&id, "Dosya")?, &input).await };
    run.await.map(|f| (StatusCode::CREATED, Json(f))).map_err(|e| Failure::with(e, &headers))
}

fn fee_id(id: &str) -> Result<uuid::Uuid, AppError> {
    path_id(id, "Tahakkuk")
}

/// `POST /v1/dossiers/{id}/fees/{fee}/paid`: the payment's receipt.
pub async fn paid(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((id, fee)): Path<(String, String)>,
    Body(input): Body<FeePaid>,
) -> Result<Json<FeeAssessment>, Failure> {
    let run = async { fees::paid(state.db()?, &caller.0, path_id(&id, "Dosya")?, fee_id(&fee)?, &input).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/dossiers/{id}/fees/{fee}/cancel`: cancelled with a reason.
pub async fn cancel(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path((id, fee)): Path<(String, String)>,
    Body(input): Body<FeeCancel>,
) -> Result<Json<FeeAssessment>, Failure> {
    let run = async { fees::cancel(state.db()?, &caller.0, path_id(&id, "Dosya")?, fee_id(&fee)?, &input).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}
