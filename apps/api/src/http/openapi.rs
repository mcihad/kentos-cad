//! The API described for programs and AI agents (docs/adr/9000 §16; docs/api/README.md).
//!
//! - `GET /v1/openapi.json`: the OpenAPI 3.1 description of every route, its request and answer shapes (JSON Schema
//!   2020-12 made from the shared contracts), how to sign in and what an error looks like.
//! - `GET /v1/llms.txt`: a short text an AI agent reads first: how to sign in, the flows, where the rest is.
//!
//! Both are the files under `docs/api/`, built into the server: what the repository shows is what the server answers.
//! The description is generated from the route catalog (openapi/catalog.rs) by a test that also checks the catalog
//! names every route of the router and nothing else (openapi/tests.rs); `KENTOS_WRITE_OPENAPI=1` writes it anew.

use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};

#[cfg(test)]
mod catalog;
#[cfg(test)]
mod tests;

const SPEC: &str = include_str!("../../../../docs/api/openapi.json");
const LLMS: &str = include_str!("../../../../docs/api/llms.txt");

fn text(body: &'static str, kind: &'static str) -> Response {
    let mut r = body.into_response();
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
    h.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    r
}

/// `GET /v1/openapi.json`: open to everyone.
pub async fn spec() -> Response {
    text(SPEC, "application/json")
}

/// `GET /v1/llms.txt`: open to everyone.
pub async fn llms() -> Response {
    text(LLMS, "text/plain; charset=utf-8")
}
