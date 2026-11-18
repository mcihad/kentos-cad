//! A municipality's figures for a period (docs/adr/9000 §22; plan BUL-20): as JSON for its page, as a workbook to
//! keep. The rules are `kentos_application::portal::statistics`.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use kentos_application::AppError;
use kentos_application::portal::statistics;
use kentos_application::tenancy;
use kentos_contracts::{PortalStatistics, WorkbookFormat};
use serde::Deserialize;

use super::path_id;
use crate::http::AppState;
use crate::http::auth::Caller;
use crate::http::error::Failure;

#[derive(Debug, Deserialize)]
pub struct PeriodQuery {
    from: Option<String>,
    to: Option<String>,
    format: Option<String>,
}

/// `GET /v1/tenants/{tenant}/portal/statistics?from=&to=`.
pub async fn page(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Query(q): Query<PeriodQuery>,
) -> Result<Json<PortalStatistics>, Failure> {
    let run = async {
        let db = state.db()?;
        let access = tenancy::access(db, &caller.0, path_id(&tenant, "Kurum")?).await?;
        let (from, to) = statistics::period(q.from.as_deref(), q.to.as_deref())?;
        statistics::statistics(db, &access, from, to).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/tenants/{tenant}/portal/statistics/export?format=xlsx|ods|csv&from=&to=`.
pub async fn export(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(tenant): Path<String>,
    Query(q): Query<PeriodQuery>,
) -> Result<Response, Failure> {
    let run = async {
        let db = state.db()?;
        let access = tenancy::access(db, &caller.0, path_id(&tenant, "Kurum")?).await?;
        let (from, to) = statistics::period(q.from.as_deref(), q.to.as_deref())?;
        let format = match q.format.as_deref().unwrap_or("xlsx") {
            "xlsx" => WorkbookFormat::Xlsx,
            "ods" => WorkbookFormat::Ods,
            "csv" => WorkbookFormat::Csv,
            _ => return Err(AppError::invalid_at("format", "format: xlsx, ods ya da csv.")),
        };
        statistics::export(db, &access, from, to, format).await
    };
    let (bytes, name, media) = run.await.map_err(|e| Failure::with(e, &headers))?;
    let mut res = bytes.into_response();
    let h = res.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(media));
    if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename=\"{name}\"")) {
        h.insert(header::CONTENT_DISPOSITION, v);
    }
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(res)
}
