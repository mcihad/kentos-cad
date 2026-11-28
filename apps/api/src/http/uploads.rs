//! `/v1/uploads`: an upload of the account's own, sent in parts and scanned before a dossier or a project takes it
//! (docs/adr/9000 §13; kentos_application::uploads).

use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::AppError;
use kentos_application::uploads;
use kentos_contracts::{UploadBegin, UploadView};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;
use super::auth::Caller;
use super::error::{Body as JsonBody, Failure};

fn upload_id(id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(id.trim())
        .map_err(|_| AppError::not_found("Yükleme bulunamadı: adresteki yükleme kimliği geçersiz."))
}

/// `POST /v1/uploads`: opens an upload for a file of this name, size and SHA-256 (201).
pub async fn begin(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    JsonBody(input): JsonBody<UploadBegin>,
) -> Result<(StatusCode, Json<UploadView>), Failure> {
    let run = async { uploads::begin(state.db()?, &caller.0, input).await };
    run.await
        .map(|u| (StatusCode::CREATED, Json(u)))
        .map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/uploads/{id}`: how far it came (`receivedBytes`) and, once received, its scan.
pub async fn view(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<Json<UploadView>, Failure> {
    let run = async { uploads::view(state.db()?, &state.blobs, &caller.0, upload_id(&id)?).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

#[derive(Deserialize)]
pub struct PartQuery {
    pub offset: Option<String>,
}

/// `PUT /v1/uploads/{id}?offset=N`: one part (`application/octet-stream`), going on from the bytes that arrived.
pub async fn receive(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Query(part): Query<PartQuery>,
    body: Body,
) -> Result<Json<UploadView>, Failure> {
    let run = async {
        let id = upload_id(&id)?;
        let offset: u64 =
            part.offset.as_deref().unwrap_or("0").parse().map_err(|_| {
                AppError::invalid_at("offset", "offset bir bayt sırası (sayı) olmalı.")
            })?;
        // A part is taken whole before anything is written: one cut short changes nothing.
        let max = usize::try_from(uploads::PART_MAX).unwrap_or(usize::MAX);
        let bytes = axum::body::to_bytes(body, max).await.map_err(|e| {
            AppError::invalid(format!(
                "Parça alınamadı ({e}); en çok {} MiB olabilir. Aynı parçayı yeniden gönderin.",
                uploads::PART_MAX / (1024 * 1024)
            ))
        })?;
        uploads::receive(
            state.db()?,
            &state.blobs,
            &state.config.scanner,
            &caller.0,
            id,
            offset,
            &bytes,
        )
        .await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `DELETE /v1/uploads/{id}`: gives an upload up (204).
pub async fn cancel(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<StatusCode, Failure> {
    let run =
        async { uploads::cancel(state.db()?, &state.blobs, &caller.0, upload_id(&id)?).await };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}
