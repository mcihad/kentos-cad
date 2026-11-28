//! The HTTP API (CLAUDE.md §14): routes, shared state and the middleware
//! every request passes (request id, trace, panic guard, timeout, body limit).

// Programs signing in with bearer tokens; an account's sessions and tokens (docs/adr/9000 §12).
pub mod access;
#[cfg(test)]
mod access_tests;
#[cfg(test)]
mod addresses_tests;
pub mod attachments;
pub mod auth;
#[cfg(test)]
mod catalog_tests;
pub mod checkpoints;
#[cfg(test)]
mod checkpoints_tests;
#[cfg(test)]
mod cloud_files_tests;
#[cfg(test)]
mod compose_tests;
#[cfg(test)]
mod delivery_tests;
pub mod download;
pub mod error;
pub mod job_inputs;
pub mod jobs;
#[cfg(test)]
mod jobs_audit_tests;
#[cfg(test)]
mod jobs_tests;
pub mod openapi;
pub mod uploads;
// The extension catalog's release flow (docs/adr/8059 §10).
pub mod extension_catalog;
#[cfg(test)]
mod extension_catalog_tests;
pub mod files;
#[cfg(test)]
mod files_tests;
#[cfg(test)]
mod filter_tests;
pub mod invitations;
#[cfg(test)]
mod invitations_tests;
pub mod limit;
#[cfg(test)]
mod native_tests;
#[cfg(test)]
mod networks_tests;
#[cfg(test)]
mod people_tests;
pub mod signup_guard;
// The cloud portal (docs/adr/9000).
pub mod portal;
pub mod projects;
#[cfg(test)]
mod temporal_tests;
// The map services' proxy (docs/adr/0208 §13).
pub mod proxy;
#[cfg(test)]
mod proxy_tests;
// The sheet template library (docs/sheet/design.md §13).
pub mod sheet_templates;
#[cfg(test)]
mod sheet_templates_tests;
#[cfg(test)]
mod tests;
pub mod ws;
#[cfg(test)]
mod ws_tests;

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
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

/// How long one upload's bytes may take to arrive (docs/adr/0031).
pub const UPLOAD_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    /// `None` when no database is configured: only `/v1/health` works then.
    pub database: Option<Db>,
    pub oidc: Option<Arc<Oidc>>,
    pub hub: Hub,
    /// Notifications as they happen (docs/adr/9000 §23).
    pub notices: crate::hub::Notices,
    pub logins: Arc<limit::LoginLimiter>,
    /// The sign-up's ticket, hidden field and limits per address (docs/adr/9000 §20).
    pub signups: Arc<signup_guard::SignupGuard>,
    /// The object store of file projects (docs/adr/0031).
    pub blobs: kentos_application::blobs::Blobs,
    /// The map services' proxy (docs/adr/0208 §13); `None`: off.
    pub proxy: Option<Arc<proxy::Proxy>>,
    /// The extension catalog (docs/adr/8059 §10); `None`: off, unless `KENTOS_EXTENSION_CATALOG=on` opens it on its
    /// persistent store (§13).
    pub extension_catalog: Option<Arc<crate::extension_catalog::Catalog>>,
}

impl AppState {
    pub fn db(&self) -> Result<&Db, AppError> {
        self.database
            .as_ref()
            .ok_or_else(|| AppError::Database(sqlx::Error::PoolClosed))
    }

    /// How this server keeps its catalog (the trash's retention, docs/adr/0028).
    pub fn catalog_policy(&self) -> kentos_application::commands::CatalogPolicy {
        kentos_application::commands::CatalogPolicy {
            trash_retention: self.config.trash_retention,
        }
    }
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        service: "kentos-api".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        commit: option_env!("KENTOS_COMMIT").map(str::to_string),
        contracts: CONTRACTS_VERSION,
        project_changes_versions: vec![
            kentos_contracts::PROJECT_CHANGES_VERSION,
            kentos_contracts::PROJECT_CHANGES_DIMENSION_SETS_VERSION,
        ],
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
    let api = Router::new()
        .route("/v1/health", get(health))
        // The API described for programs and AI agents (docs/api).
        .route("/v1/openapi.json", get(openapi::spec))
        .route("/v1/llms.txt", get(openapi::llms))
        .route("/v1/auth/config", get(auth::config))
        .route("/v1/auth/login", post(auth::login))
        .route("/v1/auth/logout", post(auth::logout))
        .route("/v1/auth/token", post(access::token))
        .route("/v1/me/sessions", get(access::sessions))
        .route("/v1/me/sessions/{id}", delete(access::end_session))
        .route(
            "/v1/me/tokens",
            get(access::tokens).post(access::create_token),
        )
        .route("/v1/me/tokens/{id}", delete(access::revoke_token))
        .route("/v1/auth/oidc/start", get(auth::oidc_start))
        .route("/v1/auth/oidc/callback", get(auth::oidc_callback))
        .route("/v1/me", get(auth::me))
        .route("/v1/me/projects", get(projects::mine))
        .route("/v1/me/catalog", get(projects::catalog))
        .route("/v1/invitations/accept", post(invitations::accept))
        .route("/v1/me/sheet-templates", get(sheet_templates::mine))
        .route(
            "/v1/me/sheet-templates/events",
            get(sheet_templates::events),
        )
        .route("/v1/sheet-templates/{id}", get(sheet_templates::detail))
        .route(
            "/v1/sheet-templates/{id}/access",
            get(sheet_templates::access),
        )
        .route(
            "/v1/sheet-templates/{id}/access/candidates",
            get(sheet_templates::candidates),
        )
        .route(
            "/v1/tenants/{tenant}/projects",
            get(projects::list).post(projects::create),
        )
        .route(
            "/v1/tenants/{tenant}/commands",
            post(projects::tenant_command),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}",
            get(projects::info).delete(projects::delete),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/details",
            get(projects::details),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/features",
            get(projects::features),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/blocks",
            get(projects::block_list),
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
        .route(
            "/v1/tenants/{tenant}/projects/{project}/uploads",
            post(files::begin),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/files",
            get(files::list),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/files/{revision}",
            get(files::download),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/snapshot",
            get(files::snapshot),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/checkpoints",
            get(checkpoints::list),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/invitations",
            get(invitations::list),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/checkpoints/{checkpoint}",
            get(checkpoints::download),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/job-inputs",
            post(job_inputs::begin),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/jobs",
            get(jobs::list).post(jobs::create),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/jobs/{job}",
            get(jobs::get),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/jobs/{job}/cancel",
            post(jobs::cancel),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/jobs/{job}/artifact",
            get(jobs::artifact),
        )
        .route("/v1/ws", get(ws::upgrade))
        .route("/v1/proxy", get(proxy::forward).post(proxy::forward))
        .route(
            "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}/moves",
            post(extension_catalog::act),
        )
        .route(
            "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}/install",
            get(extension_catalog::install),
        )
        .route(
            "/v1/extension-catalogs/{catalog}/revocations",
            get(extension_catalog::list_revocations),
        )
        // Files in the cloud (docs/adr/9000 §13-§15): attachments, pictures and the account's uploads.
        .route(
            "/v1/tenants/{tenant}/projects/{project}/attachments",
            get(attachments::list).post(attachments::add),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/attachments/{attachment}",
            get(attachments::download).delete(attachments::remove),
        )
        .route(
            "/v1/tenants/{tenant}/projects/{project}/preview.svg",
            get(attachments::project_preview),
        )
        .route("/v1/uploads", post(uploads::begin))
        // The cloud portal (docs/adr/9000).
        .merge(portal::routes())
        .layer(middleware);
    // An upload's bytes are larger and slower than any other request: their
    // route has its own limit (the largest file) and timeout.
    let upload_layers = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
            let rid = req.headers().get("x-request-id").and_then(|v| v.to_str().ok()).unwrap_or("-");
            tracing::info_span!("yükleme", method = %req.method(), path = %req.uri().path(), request_id = %rid)
        }))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CatchPanicLayer::new())
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, UPLOAD_TIMEOUT))
        .layer(RequestBodyLimitLayer::new(kentos_contracts::FILE_UPLOAD_MAX as usize));
    // Asking how an upload stands shares its path, so its route too (a short answer under the long limits).
    let uploads = Router::new()
        .route(
            "/v1/tenants/{tenant}/projects/{project}/uploads/{upload}",
            put(files::receive).get(files::upload),
        )
        .layer(upload_layers);
    // The file of an import job (docs/adr/0561 §1): the same, with the limit the server sets for it.
    let input_layers = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
            let rid = req.headers().get("x-request-id").and_then(|v| v.to_str().ok()).unwrap_or("-");
            tracing::info_span!("iş dosyası", method = %req.method(), path = %req.uri().path(), request_id = %rid)
        }))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CatchPanicLayer::new())
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, UPLOAD_TIMEOUT))
        .layer(RequestBodyLimitLayer::new(
            usize::try_from(state.config.job_limits.input_max).unwrap_or(usize::MAX),
        ));
    let inputs = Router::new()
        .route(
            "/v1/tenants/{tenant}/projects/{project}/job-inputs/{input}",
            put(job_inputs::receive).get(job_inputs::get),
        )
        .layer(input_layers);
    // An extension package's bytes (docs/adr/8059 §10): uploaded, renewed and downloaded under the largest package the
    // inspector opens; the list and a release's record share those paths, so their routes too.
    let package_layers = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
            let rid = req.headers().get("x-request-id").and_then(|v| v.to_str().ok()).unwrap_or("-");
            tracing::info_span!("eklenti paketi", method = %req.method(), path = %req.uri().path(), request_id = %rid)
        }))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CatchPanicLayer::new())
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, UPLOAD_TIMEOUT))
        .layer(RequestBodyLimitLayer::new(
            usize::try_from(kentos_extensions::Limits::default().archive).unwrap_or(usize::MAX),
        ));
    let packages = Router::new()
        .route(
            "/v1/extension-catalogs/{catalog}/releases",
            get(extension_catalog::list).post(extension_catalog::upload),
        )
        .route(
            "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}",
            get(extension_catalog::read).put(extension_catalog::replace),
        )
        .route(
            "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}/package",
            get(extension_catalog::package),
        )
        .route(
            "/v1/extension-catalogs/{catalog}/releases/{extension}/{version}/sbom",
            get(extension_catalog::sbom),
        )
        .layer(package_layers);
    // A dossier's file (docs/adr/9000 §7): the same, under the largest file a dossier takes.
    let dossier_layers = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
            let rid = req.headers().get("x-request-id").and_then(|v| v.to_str().ok()).unwrap_or("-");
            tracing::info_span!("dosya eki", method = %req.method(), path = %req.uri().path(), request_id = %rid)
        }))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CatchPanicLayer::new())
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, UPLOAD_TIMEOUT))
        .layer(RequestBodyLimitLayer::new(
            usize::try_from(kentos_application::portal::files::MAX_FILE_BYTES).unwrap_or(usize::MAX),
        ));
    let dossier_files = portal::upload_routes().layer(dossier_layers);
    // An upload's part (docs/adr/9000 §13): one part at most per request; asking and giving up share its path.
    let part_layers = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http().make_span_with(|req: &axum::http::Request<_>| {
            let rid = req.headers().get("x-request-id").and_then(|v| v.to_str().ok()).unwrap_or("-");
            tracing::info_span!("yükleme parçası", method = %req.method(), path = %req.uri().path(), request_id = %rid)
        }))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CatchPanicLayer::new())
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, UPLOAD_TIMEOUT))
        .layer(RequestBodyLimitLayer::new(
            usize::try_from(kentos_application::uploads::PART_MAX).unwrap_or(usize::MAX),
        ));
    let parts = Router::new()
        .route(
            "/v1/uploads/{id}",
            put(uploads::receive)
                .get(uploads::view)
                .delete(uploads::cancel),
        )
        .layer(part_layers);
    api.merge(uploads)
        .merge(inputs)
        .merge(packages)
        .merge(dossier_files)
        .merge(parts)
        .with_state(state)
}
