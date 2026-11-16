//! The extension catalog's routes end to end (docs/adr/8059 §10, §13), every case on both stores ([`on_both_stores`]):
//! the memory store, and the persistent one on a test database of its own (skipped without one, as every database
//! test). The tests' named accounts and the real channel rule. A release goes from upload to installation; a program
//! never decides; nobody reviews their own release; new content needs its own approval; what a caller may not see
//! answers 404 and is not counted; only the approved content of a published release installs; a refusal keeps the
//! flow's code and its Turkish reason. What only the database keeps is in `persistent`.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use kentos_application::blobs::Blobs;
use kentos_contracts::ApiError;
use kentos_extensions::Limits;
use kentos_extensions::release::{Release, Role};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};
use uuid::Uuid;

use super::tests::{config, send};
use super::{AppState, router};
use crate::extension_catalog::postgres::PgStore;
use crate::extension_catalog::store::{ReleaseStore, Where};
use crate::extension_catalog::{Accounts, Catalog, CatalogId, ReleaseKey};

/// Runs each case on both stores: `in_memory`, and `on_the_database` on a test database of its own (skipped without
/// one; `KENTOS_TEST_DB=required` makes that a failure). A case is `async fn case(bench: Bench)`.
macro_rules! on_both_stores {
    ($($case:ident),+ $(,)?) => {$(
        mod $case {
            #[tokio::test]
            async fn in_memory() {
                super::$case(super::Bench::memory()).await;
            }

            #[tokio::test]
            async fn on_the_database() {
                let Some(db) = kentos_postgres::testing::TestDb::create().await else {
                    return;
                };
                super::$case(super::Bench::database(&db)).await;
                db.close().await;
            }
        }
    )+};
}

mod access;
mod audits;
mod bench;
mod experimental;
mod flow;
mod imitation;
mod persistent;
mod refusals;
mod revocations;
mod sbom;
mod secrets;

use bench::Bench;

const PUBLISHER: &str = "ornek-belediye";
const PUBLIC: &str = "/v1/extension-catalogs/public/releases";
const GATES: [&str; 7] = [
    "package",
    "dependencies",
    "licenses",
    "compatibility",
    "secrets",
    "tests",
    "publisher",
];

/// An account's id for a name: the name's bytes in a UUID (`ayse` is `00000000-0000-0000-0000-000061797365`), the same
/// in memory and in the database, where accounts are rows and their ids UUIDs.
fn id(name: &str) -> String {
    let bytes = name.as_bytes();
    assert!(bytes.len() <= 16, "{name}: at most 16 bytes");
    let mut raw = [0u8; 16];
    raw[16 - bytes.len()..].copy_from_slice(bytes);
    Uuid::from_bytes(raw).to_string()
}

/// One's own catalog's releases, by the owner's name.
fn personal(name: &str) -> String {
    format!("/v1/extension-catalogs/personal:{}/releases", id(name))
}

/// How a request comes: a person in the web app, a program with the account's session, an access token.
#[derive(Clone, Copy, Debug)]
enum Via {
    Web,
    Python,
    Desktop,
    Mcp,
    Token,
}

fn app_with(catalog: Option<Catalog>) -> Router {
    app_of(catalog.map(Arc::new))
}

fn app_of(catalog: Option<Arc<Catalog>>) -> Router {
    router(AppState {
        config: Arc::new(config()),
        database: None,
        oidc: None,
        hub: crate::hub::Hub::default(),
        logins: Default::default(),
        blobs: Blobs::new(
            std::env::temp_dir().join(format!("kentos-api-blobs-{}", Uuid::now_v7())),
        ),
        proxy: None,
        extension_catalog: catalog,
    })
}

/// The grants of the public catalog: Ayşe publishes for Örnek Belediyesi; Mehmet, her colleague there, also reviews;
/// Selin reviews; Yusuf administers; the scanner reports in every catalog; Can is merely signed in.
async fn grants(bench: &Bench) {
    let public = || Where::In(CatalogId::Public);
    bench.grant(public(), "ayse", Role::Publisher).await;
    bench.join("ayse", PUBLISHER).await;
    bench.grant(public(), "mehmet", Role::Publisher).await;
    bench.grant(public(), "mehmet", Role::Reviewer).await;
    bench.join("mehmet", PUBLISHER).await;
    bench.grant(public(), "selin", Role::Reviewer).await;
    bench.grant(public(), "yusuf", Role::CatalogAdmin).await;
    bench.grant(Where::Every, "tarayici", Role::Scanner).await;
}

/// The routes over the bench's store with the public catalog's grants.
async fn catalog(bench: &Bench) -> Router {
    grants(bench).await;
    app_with(Some(Catalog::new(bench.store(), Accounts::Named)))
}

/// The small example extension, its manifest edited, packed.
fn kek_with(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/extensions/v1/ornek-altlik");
    let mut files = BTreeMap::new();
    for name in ["LICENSE", "manifest.json"] {
        files.insert(name.to_owned(), std::fs::read(root.join(name)).unwrap());
    }
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    edit(&mut manifest);
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap()
        .bytes
}

/// … with its publisher, version and summary set.
fn kek(publisher: Option<&str>, version: &str, summary: &str) -> Vec<u8> {
    kek_with(|m| {
        if let Some(publisher) = publisher {
            m["publisherId"] = json!(publisher);
        }
        m["version"] = json!(version);
        m["summary"] = json!(summary);
    })
}

fn release(version: &str) -> Vec<u8> {
    kek(
        Some(PUBLISHER),
        version,
        "Belediyenin ortofoto ve imar planı servisleri.",
    )
}

fn uri(base: &str, version: &str) -> String {
    format!("{base}/tr.ornek.altlik/{version}")
}

fn key(version: &str) -> ReleaseKey {
    ReleaseKey {
        catalog: CatalogId::Public,
        extension: "tr.ornek.altlik".into(),
        version: version.into(),
    }
}

/// A request of the account named `account` (its id goes in the tests' header).
fn request(via: Via, method: &str, uri: &str, account: &str) -> axum::http::request::Builder {
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-kentos-test-account", id(account));
    match via {
        Via::Web => b.header("x-kentos-client", "web"),
        Via::Python => b.header("x-kentos-client", "python"),
        Via::Desktop => b.header("x-kentos-client", "desktop"),
        Via::Mcp => b.header("x-kentos-client", "mcp"),
        Via::Token => b.header(header::AUTHORIZATION, "Bearer erisim-anahtari"),
    }
}

async fn call(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let (status, _, body) = send(app, req).await;
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn get(app: &Router, via: Via, uri: &str, account: &str) -> (StatusCode, Value) {
    call(
        app,
        request(via, "GET", uri, account)
            .body(Body::empty())
            .unwrap(),
    )
    .await
}

async fn send_bytes(
    app: &Router,
    method: &str,
    uri: &str,
    account: &str,
    bytes: Vec<u8>,
) -> (StatusCode, Value) {
    let req = request(Via::Web, method, uri, account)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from(bytes))
        .unwrap();
    call(app, req).await
}

async fn upload(app: &Router, base: &str, account: &str, bytes: Vec<u8>) -> (StatusCode, Value) {
    send_bytes(app, "POST", base, account, bytes).await
}

async fn act(app: &Router, via: Via, account: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = request(via, "POST", &format!("{uri}/moves"), account)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    call(app, req).await
}

/// The scanner's clean reports, with its access token, for the content it scanned.
async fn scan_all(app: &Router, uri: &str, content: &str) {
    for gate in GATES {
        let report =
            json!({ "action": "scan", "gate": gate, "content": content, "outcome": "passed" });
        let (status, body) = act(app, Via::Token, "tarayici", uri, report).await;
        assert_eq!(status, StatusCode::OK, "{gate}: {body}");
    }
}

fn package_of(view: &Value) -> String {
    view["release"]["content"]["package"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn state_of(view: &Value) -> &str {
    view["release"]["state"]
        .as_str()
        .or(view["state"].as_str())
        .unwrap_or_default()
}

fn code(body: &Value) -> String {
    serde_json::from_value::<ApiError>(body.clone())
        .map(|e| e.error)
        .unwrap_or_else(|_| format!("bir hata gövdesi değil: {body}"))
}

/// Uploaded by Ayşe, every gate clean, sent by her program and taken into review by Selin; its content's digest.
async fn in_review(app: &Router, version: &str) -> String {
    let (status, view) = upload(app, PUBLIC, "ayse", release(version)).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    let content = package_of(&view);
    let at = uri(PUBLIC, version);
    scan_all(app, &at, &content).await;
    let (status, view) = act(app, Via::Python, "ayse", &at, json!({ "action": "submit" })).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let (status, view) = act(
        app,
        Via::Web,
        "selin",
        &at,
        json!({ "action": "startReview" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    content
}
