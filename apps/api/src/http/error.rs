//! Error responses: every failure is JSON (`ApiError`) with a stable code, a
//! Turkish message and the request id. Database details go to the log only.

use axum::Json;
use axum::extract::FromRequest;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use kentos_application::AppError;
use kentos_contracts::ApiError;

/// An `AppError` on its way to the client, with the request it belongs to.
pub struct Failure {
    pub error: AppError,
    pub request_id: Option<String>,
}

impl Failure {
    pub fn with(error: AppError, headers: &HeaderMap) -> Self {
        Self {
            error,
            request_id: request_id(headers),
        }
    }
}

impl From<AppError> for Failure {
    fn from(error: AppError) -> Self {
        Self {
            error,
            request_id: None,
        }
    }
}

pub fn request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

pub fn status_of(error: &AppError) -> StatusCode {
    match error {
        AppError::Unauthenticated(_) => StatusCode::UNAUTHORIZED,
        AppError::Forbidden(_) => StatusCode::FORBIDDEN,
        AppError::NotFound(_) => StatusCode::NOT_FOUND,
        AppError::Invalid { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        AppError::Deleted(_) | AppError::ResyncRequired(_) => StatusCode::GONE,
        AppError::Conflict { .. } | AppError::Archived(_) => StatusCode::CONFLICT,
        AppError::Limited { .. } => StatusCode::TOO_MANY_REQUESTS,
        AppError::Database(_) if error.code() == "unavailable" => StatusCode::SERVICE_UNAVAILABLE,
        AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let status = status_of(&self.error);
        if let AppError::Database(e) = &self.error {
            tracing::error!(request_id = self.request_id.as_deref().unwrap_or("-"), error = %e, "veritabanı hatası");
        }
        let (conflicts, revision) = match &self.error {
            AppError::Conflict {
                conflicts,
                revision,
                ..
            } => (Some(conflicts.clone()), revision.map(|r| r.to_string())),
            _ => (None, None),
        };
        let (retryable, retry_after) = self.error.retry();
        let body = ApiError {
            error: self.error.code().into(),
            message: self.error.to_string(),
            request_id: self.request_id,
            conflicts,
            path: self.error.path().map(str::to_string),
            revision,
            retryable,
            retry_after,
        };
        let mut response = (status, Json(body)).into_response();
        if let Some(secs) = retry_after {
            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, secs.into());
        }
        response
    }
}

/// `Json<T>` whose rejection (bad JSON, wrong shape, wrong content type) is an `ApiError` too.
pub struct Body<T>(pub T);

impl<S, T> FromRequest<S> for Body<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = Failure;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let rid = request_id(req.headers());
        match Json::<T>::from_request(req, state).await {
            Ok(Json(v)) => Ok(Body(v)),
            Err(e) => Err(Failure {
                error: AppError::invalid(format!("İstek gövdesi okunamadı: {}", e.body_text())),
                request_id: rid,
            }),
        }
    }
}
