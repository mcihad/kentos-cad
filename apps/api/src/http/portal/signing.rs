//! E-signing a dossier's files (docs/adr/9004; plan BUL-21): the server's key for the signing service, a request for
//! the files' signatures, the delivery checked against the server's own request, and the signatures kept. The rules
//! are `kentos_application::portal::signing`.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use kentos_application::portal::signing::{self, SigningDelivery};
use kentos_contracts::{
    DossierSignatureView, SigningRequestCreate, SigningRequestCreated, SigningTrust, SigningVerdict,
};

use super::path_id;
use crate::http::AppState;
use crate::http::auth::Caller;
use crate::http::error::{Body, Failure};

/// `GET /v1/signing/trust`: the server's public key and names, for the signing service's setup and the desktop.
pub async fn trust(
    State(state): State<AppState>,
    headers: HeaderMap,
    _caller: Caller,
) -> Result<Json<SigningTrust>, Failure> {
    state
        .config
        .signing
        .as_deref()
        .map(|s| Json(s.trust()))
        .ok_or_else(|| Failure::with(signing::off(), &headers))
}

/// `POST /v1/dossiers/{id}/signing`: a signed request for these files (201).
pub async fn request(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
    Body(input): Body<SigningRequestCreate>,
) -> Result<(StatusCode, Json<SigningRequestCreated>), Failure> {
    let run = async {
        let id = path_id(&id, "Dosya")?;
        signing::request(
            state.db()?,
            &state.blobs,
            state.config.signing.as_deref(),
            &caller.0,
            id,
            &input,
        )
        .await
    };
    run.await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/signing/{transaction}/result`: the signing service's result as it gave it.
pub async fn deliver(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(transaction): Path<String>,
    Body(result): Body<SigningDelivery>,
) -> Result<Json<SigningVerdict>, Failure> {
    let run = async {
        let transaction = path_id(&transaction, "İmza isteği")?;
        signing::deliver(state.db()?, &state.blobs, &caller.0, transaction, &result).await
    };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}

/// `POST /v1/signing/{transaction}/cancel` (204).
pub async fn cancel(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(transaction): Path<String>,
) -> Result<StatusCode, Failure> {
    let run = async {
        signing::cancel(
            state.db()?,
            &caller.0,
            path_id(&transaction, "İmza isteği")?,
        )
        .await
    };
    run.await
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| Failure::with(e, &headers))
}

/// `GET /v1/dossiers/{id}/signatures`.
pub async fn signatures(
    State(state): State<AppState>,
    headers: HeaderMap,
    caller: Caller,
    Path(id): Path<String>,
) -> Result<Json<Vec<DossierSignatureView>>, Failure> {
    let run = async { signing::signatures(state.db()?, &caller.0, path_id(&id, "Dosya")?).await };
    run.await.map(Json).map_err(|e| Failure::with(e, &headers))
}
