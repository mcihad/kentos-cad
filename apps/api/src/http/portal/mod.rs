//! The cloud portal's routes (docs/adr/9000): the signed-in person on the portal, signing up, the catalog, the
//! platform administrators' pages, an organisation's portal and members, dossiers with their steps, messages and files,
//! and notifications. The use cases are `kentos_application::portal`; these only read the request and word the answer.

mod dossiers;
mod me;
mod organization;
mod platform;

use axum::Router;
use axum::routing::{get, patch, post, put};
use kentos_application::AppError;
use kentos_application::portal::catalog;
use kentos_application::portal::dossiers::ListFilter;
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;

/// The portal's routes under the API's ordinary limits.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/portal/me", get(me::me))
        .route("/v1/portal/catalog", get(me::catalog))
        .route("/v1/portal/signup", get(me::signup_info).post(me::sign_up))
        .route("/v1/portal/me/professional", put(me::edit_professional))
        .route("/v1/portal/me/password", post(me::password))
        .route("/v1/portal/municipalities", get(me::municipalities))
        .route("/v1/me/notifications", get(me::notifications))
        .route("/v1/me/notifications/read", post(me::read))
        .route("/v1/me/dossiers", get(dossiers::mine).post(dossiers::apply))
        .route("/v1/me/dossiers/summary", get(dossiers::my_summary))
        .route("/v1/platform/summary", get(platform::summary))
        .route(
            "/v1/platform/tenants",
            get(platform::tenants).post(platform::create_tenant),
        )
        .route(
            "/v1/platform/tenants/{id}",
            get(platform::tenant).patch(platform::update_tenant),
        )
        .route(
            "/v1/platform/users",
            get(platform::users).post(platform::create_user),
        )
        .route("/v1/platform/users/{id}", patch(platform::edit_user))
        .route(
            "/v1/platform/professionals",
            get(platform::professionals).post(platform::create_professional),
        )
        .route(
            "/v1/platform/professionals/{id}/review",
            post(platform::review),
        )
        .route("/v1/platform/events", get(platform::events))
        .route(
            "/v1/platform/settings",
            get(platform::settings).put(platform::set_settings),
        )
        .route(
            "/v1/tenants/{tenant}/portal",
            get(organization::portal).put(organization::set_portal),
        )
        .route(
            "/v1/tenants/{tenant}/portal/summary",
            get(organization::summary),
        )
        .route(
            "/v1/tenants/{tenant}/dossiers",
            get(organization::dossiers).post(organization::transfer),
        )
        .route(
            "/v1/tenants/{tenant}/professionals",
            get(organization::directory),
        )
        .route(
            "/v1/tenants/{tenant}/members",
            get(organization::members).post(organization::add_member),
        )
        .route(
            "/v1/tenants/{tenant}/members/{user}",
            patch(organization::update_member),
        )
        .route(
            "/v1/dossiers/{id}",
            get(dossiers::detail)
                .patch(dossiers::edit)
                .delete(dossiers::remove),
        )
        .route("/v1/dossiers/{id}/actions", post(dossiers::act))
        .route("/v1/dossiers/{id}/messages", post(dossiers::message))
        .route(
            "/v1/dossiers/{id}/files/from-project",
            post(dossiers::from_project),
        )
        .route(
            "/v1/dossiers/{id}/files/{file}",
            get(dossiers::download).delete(dossiers::remove_file),
        )
        .route(
            "/v1/dossiers/{id}/files/{file}/import",
            post(dossiers::import),
        )
        .route("/v1/dossiers/{id}/files/from-upload", post(dossiers::from_upload))
        .route("/v1/dossiers/{id}/files/{file}/preview.svg", get(dossiers::preview))
}

/// A file's bytes coming to a dossier: under the upload limits (`super::router`).
pub fn upload_routes() -> Router<AppState> {
    Router::new().route("/v1/dossiers/{id}/files", post(dossiers::upload))
}

/// An id in a path; a malformed one is as missing as an unknown one.
fn path_id(text: &str, what: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(text).map_err(|_| AppError::not_found(format!("{what} bulunamadı.")))
}

/// A list's query: `direction`, `status` (comma separated), `q`, `mine`, `awaiting`, `limit`, `offset`.
#[derive(Debug, Default, Deserialize)]
pub struct ListQuery {
    direction: Option<String>,
    status: Option<String>,
    q: Option<String>,
    mine: Option<bool>,
    awaiting: Option<bool>,
    limit: Option<u32>,
    offset: Option<u32>,
}

impl ListQuery {
    fn filter(&self) -> Result<ListFilter, AppError> {
        let direction = match self.direction.as_deref().filter(|d| !d.is_empty()) {
            Some(d) => Some(catalog::direction_of(d).ok_or_else(|| {
                AppError::invalid_at("direction", "direction: application ya da transfer.")
            })?),
            None => None,
        };
        let statuses = self
            .status
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                catalog::status_of(s).ok_or_else(|| {
                    AppError::invalid_at("status", format!("“{s}” bir durum değil."))
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(ListFilter {
            direction,
            statuses,
            query: self.q.clone(),
            mine: self.mine.unwrap_or(false),
            awaiting: self.awaiting.unwrap_or(false),
            limit: self.limit.unwrap_or(50),
            offset: self.offset.unwrap_or(0),
        })
    }
}
