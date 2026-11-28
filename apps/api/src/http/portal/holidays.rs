//! The holidays the legal deadlines are counted over (docs/adr/9000 §19; plan BUL-16): anyone signed in reads a year,
//! the platform's administrators add and remove the religious holidays. The rules are
//! `kentos_application::portal::holidays`.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::portal::holidays;
use kentos_contracts::{HolidayCreate, HolidayView};
use serde::Deserialize;

use crate::http::AppState;
use crate::http::auth::Caller;
use crate::http::error::{Body, Failure};

#[derive(Debug, Deserialize)]
pub struct YearQuery {
    year: Option<i32>,
}

/// `GET /v1/holidays?year=`: the year's holidays (this year's when none is given).
pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Query(q): Query<YearQuery>,
) -> Result<Json<Vec<HolidayView>>, Failure> {
    let year = q
        .year
        .unwrap_or_else(|| time::OffsetDateTime::now_utc().year());
    let run = async { holidays::list(state.db()?, &caller.0, year).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/platform/holidays` (201).
pub async fn add(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Body(input): Body<HolidayCreate>,
) -> Result<(StatusCode, Json<HolidayView>), Failure> {
    let run = async { holidays::add(state.db()?, &caller.0, &input).await };
    run.await
        .map(|h| (StatusCode::CREATED, Json(h)))
        .map_err(|e| Failure::with(e, &headers))
}

/// `DELETE /v1/platform/holidays/{day}` (204).
pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(day): Path<String>,
) -> Result<StatusCode, Failure> {
    let run = async { holidays::remove(state.db()?, &caller.0, &day).await };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}
