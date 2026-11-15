//! The extension catalog over HTTP (plan/eklenti.md ADIM 22A; docs/adr/8059 §10).
//!
//! | Route | |
//! |---|---|
//! | `GET  /v1/extension-catalogs/{catalog}/releases` | the releases the caller may see; the search and the counts are of those |
//! | `POST /v1/extension-catalogs/{catalog}/releases` | a `.kek`'s bytes uploaded: a draft, not a publication |
//! | `GET  …/releases/{extension}/{version}` | one release: its whole record for its insiders, what anyone may know for the rest |
//! | `PUT  …/releases/{extension}/{version}` | new content for a draft or a release sent back |
//! | `POST …/releases/{extension}/{version}/moves` | a move of the flow: `{"action": "approve", "note": …}` |
//! | `GET  …/releases/{extension}/{version}/install` | what may be installed: a published release's approved content |
//! | `GET  …/releases/{extension}/{version}/package` | its approved bytes, checked against the approval before they leave |
//! | `GET  /v1/extension-catalogs/{catalog}/revocations` | the catalog's revocation list, the same for every member (§12) |
//!
//! `{catalog}` is `public`, `tenant:{slug}` or `personal:{account}`. A refused move answers in the API's error body
//! with the flow's own code and Turkish reason; a catalog the caller is not in, a release it may not see and one that
//! does not exist answer 404 word for word (PY-CLOUD-17). Who decides is [`channel`]'s.

use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{FromRequestParts, Path, Query};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderName, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kentos_application::AppError;
use kentos_contracts::ApiError;
use kentos_extensions::release::{Channel, Refusal, ReleaseState};
use serde::Deserialize;

use super::AppState;
use super::auth::{CLIENT_HEADER, Caller};
use super::error::{Body, Failure, request_id};
use crate::extension_catalog::{
    Account, Accounts, Catalog, CatalogError, CatalogId, Filter, Move, ReleaseKey,
};

mod revocations;
mod views;

pub use revocations::list_revocations;
use views::view;
pub use views::{InstallTicket, ReleaseList, ReleaseView};

/// Releases on a page of the list when the request does not say, and at most.
const LIST_DEFAULT: usize = 50;
const LIST_MAX: usize = 200;

const NOT_FOUND: &str = "Böyle bir katalog ya da sürüm yok, ya da onu görme yetkiniz yok.";

/// How a call came (PY-CLOUD-18). Only the web app's own request on a browser session, with no access token and
/// `x-kentos-client: web`, is the catalog's window, where a person decides. An access token, the desktop, the Python
/// SDK, the MCP server and a request that does not say are programs: they upload, send for review, report scans and
/// publish what a person approved, but never approve, send back, lift a block or take a release out of suspension.
/// The header is the app's word, not a proof; docs/adr/8059 §10 says what closes that before the catalog opens.
pub fn channel(headers: &HeaderMap) -> Channel {
    let web = headers.get(CLIENT_HEADER).and_then(|v| v.to_str().ok()) == Some("web");
    if web && !headers.contains_key(header::AUTHORIZATION) {
        Channel::Window
    } else {
        Channel::Program
    }
}

/// The caller of a catalog route: the catalog, the account (found as on every protected route) and how the call came.
pub struct CatalogCaller {
    pub catalog: Arc<Catalog>,
    pub account: Account,
}

impl FromRequestParts<AppState> for CatalogCaller {
    type Rejection = Failure;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Failure> {
        let catalog = state.extension_catalog.clone().ok_or_else(|| {
            Failure::with(
                AppError::not_found("Bu sunucuda eklenti kataloğu açık değil."),
                &parts.headers,
            )
        })?;
        let via = channel(&parts.headers);
        let id = match catalog.accounts() {
            Accounts::Sessions => Caller::from_request_parts(parts, state)
                .await?
                .0
                .user_id
                .to_string(),
            #[cfg(test)]
            Accounts::Named => named(parts)?,
        };
        Ok(CatalogCaller {
            catalog,
            account: Account { id, via },
        })
    }
}

/// The tests' account, named by `x-kentos-test-account`; a request without an access token keeps the browser
/// session's cross-site rule.
#[cfg(test)]
fn named(parts: &Parts) -> Result<String, Failure> {
    let fail = |e: AppError| Failure::with(e, &parts.headers);
    if !parts.headers.contains_key(header::AUTHORIZATION) {
        super::auth::check_client_header(&parts.method, &parts.headers).map_err(fail)?;
    }
    parts
        .headers
        .get("x-kentos-test-account")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| fail(AppError::Unauthenticated("Oturum açın.".into())))
}

/// Why a catalog route did not answer: the server's own error or the catalog's.
pub enum Why {
    App(AppError),
    Catalog(CatalogError),
}

impl From<AppError> for Why {
    fn from(e: AppError) -> Why {
        Why::App(e)
    }
}

impl From<CatalogError> for Why {
    fn from(e: CatalogError) -> Why {
        Why::Catalog(e)
    }
}

/// A catalog route's failure on its way to the client, with the request it belongs to.
pub struct CatalogFailure {
    why: Why,
    request_id: Option<String>,
}

impl CatalogFailure {
    fn new(why: Why, headers: &HeaderMap) -> CatalogFailure {
        CatalogFailure {
            why,
            request_id: request_id(headers),
        }
    }
}

impl IntoResponse for CatalogFailure {
    fn into_response(self) -> Response {
        let request_id = self.request_id;
        let error = match self.why {
            Why::App(error) => error,
            Why::Catalog(CatalogError::NotFound) => AppError::not_found(NOT_FOUND),
            Why::Catalog(CatalogError::Store(detail)) => {
                AppError::Storage(std::io::Error::other(detail))
            }
            Why::Catalog(CatalogError::Refused(refusal)) => return refused(refusal, request_id),
        };
        Failure { error, request_id }.into_response()
    }
}

/// A refusal in the API's error body: the flow's code as `error`, its Turkish reason and remedy as `message`.
fn refused(refusal: Refusal, request_id: Option<String>) -> Response {
    let status = status_of(refusal.code);
    if status.is_server_error() {
        tracing::error!(
            request_id = request_id.as_deref().unwrap_or("-"),
            code = refusal.code,
            message = %refusal.message,
            "eklenti kataloğunun kaydı bozuk"
        );
    }
    let body = ApiError {
        error: refusal.code.to_owned(),
        message: refusal.message,
        request_id,
        conflicts: None,
        path: None,
        revision: None,
        retryable: refusal.code == "release_changed",
        retry_after: None,
    };
    (status, Json(body)).into_response()
}

/// A refusal's HTTP status: who may not (403), what the request got wrong (422), what the release's state or another's
/// move keeps from happening (409, the rest), what the server's own records got wrong (500).
pub fn status_of(code: &str) -> StatusCode {
    match code {
        "permission_missing"
        | "not_owner"
        | "self_review"
        | "self_scan"
        | "decision_by_program" => StatusCode::FORBIDDEN,
        "reason_missing" | "changes_missing" | "package_invalid" | "identity_mismatch"
        | "publisher_mismatch" | "publisher_missing" | "scan_unexplained"
        | "waiver_not_allowed" => StatusCode::UNPROCESSABLE_ENTITY,
        "package_mismatch" | "baseline_invalid" | "record_invalid" | "record_inconsistent" => {
            StatusCode::INTERNAL_SERVER_ERROR
        }
        _ => StatusCode::CONFLICT,
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    q: Option<String>,
    state: Option<String>,
    extension: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
}

impl ListQuery {
    fn filter(self) -> Result<Filter, AppError> {
        let state = match self.state.as_deref() {
            None | Some("") => None,
            Some(text) => Some(
                ReleaseState::ALL
                    .into_iter()
                    .find(|s| s.id() == text)
                    .ok_or_else(|| {
                        let ids: Vec<&str> = ReleaseState::ALL.iter().map(|s| s.id()).collect();
                        AppError::invalid_at(
                            "state",
                            format!(
                                "“{text}” bir sürüm durumu değil; şunlardan biri olur: {}.",
                                ids.join(", ")
                            ),
                        )
                    })?,
            ),
        };
        Ok(Filter {
            text: self.q,
            state,
            extension: self.extension,
            offset: self.offset.unwrap_or(0),
            limit: self.limit.unwrap_or(LIST_DEFAULT).clamp(1, LIST_MAX),
        })
    }
}

#[derive(Deserialize)]
pub struct ReleasePath {
    catalog: String,
    extension: String,
    version: String,
}

fn catalog_id(text: &str) -> Result<CatalogId, CatalogError> {
    CatalogId::parse(text).ok_or(CatalogError::NotFound)
}

fn key(path: ReleasePath) -> Result<ReleaseKey, CatalogError> {
    Ok(ReleaseKey {
        catalog: catalog_id(&path.catalog)?,
        extension: path.extension,
        version: path.version,
    })
}

/// `GET /v1/extension-catalogs/{catalog}/releases?q=&state=&extension=&offset=&limit=`
pub async fn list(
    headers: HeaderMap,
    Path(catalog): Path<String>,
    Query(query): Query<ListQuery>,
    caller: CatalogCaller,
) -> Result<Json<ReleaseList>, CatalogFailure> {
    let run = async {
        let filter = query.filter()?;
        let listing = caller
            .catalog
            .list(&catalog_id(&catalog)?, &caller.account, &filter)
            .await?;
        Ok::<_, Why>(ReleaseList::of(listing))
    };
    run.await
        .map(Json)
        .map_err(|e| CatalogFailure::new(e, &headers))
}

/// `POST /v1/extension-catalogs/{catalog}/releases`: a `.kek`'s bytes, uploaded as a draft (`201`, `Location`).
pub async fn upload(
    headers: HeaderMap,
    Path(catalog): Path<String>,
    caller: CatalogCaller,
    body: Bytes,
) -> Result<Response, CatalogFailure> {
    let run = async {
        let id = catalog_id(&catalog)?;
        let (release, actor) = caller.catalog.upload(&id, &caller.account, body).await?;
        let location = format!(
            "/v1/extension-catalogs/{id}/releases/{}/{}",
            release.extension_id(),
            release.version()
        );
        let view = view(release, &actor);
        Ok::<_, Why>(
            (
                StatusCode::CREATED,
                [(header::LOCATION, location)],
                Json(view),
            )
                .into_response(),
        )
    };
    run.await.map_err(|e| CatalogFailure::new(e, &headers))
}

/// `GET /v1/extension-catalogs/{catalog}/releases/{extension}/{version}`
pub async fn read(
    headers: HeaderMap,
    Path(path): Path<ReleasePath>,
    caller: CatalogCaller,
) -> Result<Json<ReleaseView>, CatalogFailure> {
    let run = async {
        let (release, actor) = caller.catalog.release(&key(path)?, &caller.account).await?;
        Ok::<_, Why>(view(release, &actor))
    };
    run.await
        .map(Json)
        .map_err(|e| CatalogFailure::new(e, &headers))
}

/// `PUT /v1/extension-catalogs/{catalog}/releases/{extension}/{version}`: a `.kek`'s bytes as the release's new content.
pub async fn replace(
    headers: HeaderMap,
    Path(path): Path<ReleasePath>,
    caller: CatalogCaller,
    body: Bytes,
) -> Result<Json<ReleaseView>, CatalogFailure> {
    let run = async {
        let (release, actor) = caller
            .catalog
            .replace(&key(path)?, &caller.account, body)
            .await?;
        Ok::<_, Why>(view(release, &actor))
    };
    run.await
        .map(Json)
        .map_err(|e| CatalogFailure::new(e, &headers))
}

/// `POST /v1/extension-catalogs/{catalog}/releases/{extension}/{version}/moves`: one move of the flow.
pub async fn act(
    headers: HeaderMap,
    Path(path): Path<ReleasePath>,
    caller: CatalogCaller,
    Body(request): Body<Move>,
) -> Result<Json<ReleaseView>, CatalogFailure> {
    let run = async {
        let (release, actor) = caller
            .catalog
            .act(&key(path)?, &caller.account, &request)
            .await?;
        Ok::<_, Why>(view(release, &actor))
    };
    run.await
        .map(Json)
        .map_err(|e| CatalogFailure::new(e, &headers))
}

/// `GET /v1/extension-catalogs/{catalog}/releases/{extension}/{version}/install`: the approved content, or why it is
/// not to be installed.
pub async fn install(
    headers: HeaderMap,
    Path(path): Path<ReleasePath>,
    caller: CatalogCaller,
) -> Result<Json<InstallTicket>, CatalogFailure> {
    let run = async {
        let release = caller
            .catalog
            .installable(&key(path)?, &caller.account)
            .await?;
        let Some(approval) = release.approval() else {
            return Err(Why::Catalog(CatalogError::Store(format!(
                "{} {} kurulabilir ama onayı yok",
                release.extension_id(),
                release.version()
            ))));
        };
        Ok(InstallTicket {
            extension_id: release.extension_id().to_owned(),
            version: release.version().to_owned(),
            publisher: release.publisher().to_owned(),
            content: approval.content.clone(),
            approved_at: approval.at,
            depth: approval.depth,
        })
    };
    run.await
        .map(Json)
        .map_err(|e| CatalogFailure::new(e, &headers))
}

/// `GET /v1/extension-catalogs/{catalog}/releases/{extension}/{version}/package`: the approved package's bytes.
pub async fn package(
    headers: HeaderMap,
    Path(path): Path<ReleasePath>,
    caller: CatalogCaller,
) -> Result<Response, CatalogFailure> {
    let run = async {
        let (release, bytes) = caller.catalog.package(&key(path)?, &caller.account).await?;
        let file = format!("{}-{}.kek", release.extension_id(), release.version());
        let fields = [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{file}\""),
            ),
            (
                HeaderName::from_static("x-kentos-content-digest"),
                release.content().package.clone(),
            ),
        ];
        Ok::<_, Why>((fields, bytes).into_response())
    };
    run.await.map_err(|e| CatalogFailure::new(e, &headers))
}
