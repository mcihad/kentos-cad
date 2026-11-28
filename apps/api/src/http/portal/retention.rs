//! How long personal data is kept and its periodic destruction (docs/adr/9000 §21; plan BUL-18): the platform's
//! administrators read and set the periods, see the runs and run it by hand. The server runs it daily (main.rs).

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use kentos_application::portal::retention;
use kentos_contracts::{RetentionPage, RetentionRunView, RetentionSettings};

use crate::http::AppState;
use crate::http::auth::Caller;
use crate::http::error::{Body, Failure};

/// `GET /v1/platform/retention`.
pub async fn page(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<RetentionPage>, Failure> {
    let run = async { retention::page(state.db()?, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `PUT /v1/platform/retention`.
pub async fn set(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<RetentionSettings>,
) -> Result<Json<RetentionPage>, Failure> {
    let run = async { retention::set(state.db()?, &caller.0, &input).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/platform/retention/run`: now, by hand.
pub async fn run_now(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
) -> Result<Json<RetentionRunView>, Failure> {
    let run = async { retention::run_now(state.db()?, &state.blobs, &caller.0).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}
