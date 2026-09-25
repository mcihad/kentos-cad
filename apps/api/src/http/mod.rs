//! The HTTP API (CLAUDE.md §14): routes, shared state and the middleware
//! every request passes (request id, trace, panic guard, timeout, body limit).

pub mod auth;
pub mod error;
pub mod limit;
#[cfg(test)]
mod people_tests;
pub mod projects;
#[cfg(test)]
mod tests;
pub mod ws;
#[cfg(test)]
mod ws_tests;

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use kentos_application::AppError;
use kentos_contracts::{CONTRACTS_VERSION, Health};
use kentos_postgres::Db;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::hub::Hub;
use crate::oidc::Oidc;

/// Largest request body: a batch of object changes (a whole imported sheet goes in several).
pub const BODY_LIMIT: usize = 16 * 1024 * 1024;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    /// `None` when no database is configured: only `/v1/health` works then.
    pub database: Option<Db>,
    pub oidc: Option<Arc<Oidc>>,
    pub hub: Hub,
    pub logins: Arc<limit::LoginLimiter>,
}

impl AppState {
    pub fn db(&self) -> Result<&Db, AppError> {
        self.database
            .as_ref()
            .ok_or_else(|| AppError::Database(sqlx::Error::PoolClosed))
    }
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        service: "kentos-api".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        commit: option_env!("KENTOS_COMMIT").map(str::to_string),
        contracts: CONTRACTS_VERSION,
    })
}

pub fn router(state: AppState) -> Router {
    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
            let rid = req.headers().get("x-request-id").and_then(|v| v.to_str().ok()).unwrap_or("-");
            tracing::info_span!("istek", method = %req.method(), path = %req.uri().path(), request_id = %rid)
        }))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CatchPanicLayer::new())
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30)))
        .layer(RequestBodyLimitLayer::new(BODY_LIMIT));
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/auth/config", get(auth::config))
        .route("/v1/auth/login", post(auth::login))
        .route("/v1/auth/logout", post(auth::logout))
        .route("/v1/auth/oidc/start", get(auth::oidc_start))
        .route("/v1/auth/oidc/callback", get(auth::oidc_callback))
        .route("/v1/me", get(auth::me))
        .route("/v1/me/projects", get(projects::mine))
        .route(
            "/v1/tenants/{tenant}/projects",
            get(projects::list).post(projects::create),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}",
            get(projects::info).delete(projects::delete),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/features",
            get(projects::features),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/commands",
            post(projects::command),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/events",
            get(projects::event_log),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/access",
            get(projects::access_list),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/access/candidates",
            get(projects::share_candidates),
        )
        .route("/v1/ws", get(ws::upgrade))
        .layer(middleware)
        .with_state(state)
}
