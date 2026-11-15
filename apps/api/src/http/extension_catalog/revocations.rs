//! `GET /v1/extension-catalogs/{catalog}/revocations`: the catalog's revocation list (plan/eklenti.md ADIM 22A;
//! docs/adr/8059 §12) in the host's own format (`kentos.extension-revocations` v1), the same for every member.
//!
//! It is not signed yet, so the desktop must not apply it until it is (§9). Cache: `private, no-cache` with an `ETag`.
//! A withdrawal must reach the next sync, so no copy may be used without asking; an unchanged list then costs a 304.
//! `private`, because the answer belongs to a signed-in member and never to a shared cache.

use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use kentos_extensions::integrity::sha256_hex;

use super::{CatalogCaller, CatalogFailure, Why, catalog_id};
use crate::extension_catalog::CatalogError;

const CACHE: &str = "private, no-cache";

pub async fn list_revocations(
    headers: HeaderMap,
    Path(catalog): Path<String>,
    caller: CatalogCaller,
) -> Result<Response, CatalogFailure> {
    let run = async {
        let list = caller
            .catalog
            .revocations(&catalog_id(&catalog)?, &caller.account)
            .await?;
        let body = serde_json::to_vec(&list).map_err(|e| {
            Why::Catalog(CatalogError::Store(format!(
                "geri çekme listesi yazılamadı: {e}"
            )))
        })?;
        let tag = format!("\"{}\"", sha256_hex(&body));
        let unchanged = headers
            .get(header::IF_NONE_MATCH)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(',')
                    .map(|t| t.trim().trim_start_matches("W/"))
                    .any(|t| t == tag || t == "*")
            });
        let fields = [
            (header::ETAG, tag),
            (header::CACHE_CONTROL, CACHE.to_owned()),
        ];
        if unchanged {
            return Ok((StatusCode::NOT_MODIFIED, fields).into_response());
        }
        Ok((fields, [(header::CONTENT_TYPE, "application/json")], body).into_response())
    };
    run.await.map_err(|e| CatalogFailure::new(e, &headers))
}
