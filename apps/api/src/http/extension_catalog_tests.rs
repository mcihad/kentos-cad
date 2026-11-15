//! The extension catalog's routes end to end without a database (docs/adr/8059 §10): the in-memory store, the tests'
//! named accounts and the real channel rule. A release goes from upload to installation; a program never decides;
//! nobody reviews their own release; new content needs its own approval; what a caller may not see answers 404 and is
//! not counted; only the approved content of a published release installs; a refusal keeps the flow's code and its
//! Turkish reason.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Router;
use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode, header};
use kentos_contracts::ApiError;
use kentos_extensions::Limits;
use kentos_extensions::release::{Action, Channel, Release, ReleaseFacts, Role};
use serde_json::{Value, json};

use super::extension_catalog::{channel, status_of};
use super::tests::{config, send};
use super::{AppState, router};
use crate::extension_catalog::memory::{MemoryStore, Where};
use crate::extension_catalog::store::{Access, ReleaseStore, StoreError, StoreFuture};
use crate::extension_catalog::{Accounts, Catalog, CatalogId, ReleaseKey};

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
    router(AppState {
        config: Arc::new(config()),
        database: None,
        oidc: None,
        hub: crate::hub::Hub::default(),
        logins: Default::default(),
        blobs: kentos_application::blobs::Blobs::new(
            std::env::temp_dir().join(format!("kentos-api-blobs-{}", uuid::Uuid::now_v7())),
        ),
        proxy: None,
        extension_catalog: catalog.map(Arc::new),
    })
}

/// The grants of the public catalog: Ayşe publishes for Örnek Belediyesi; Mehmet, her colleague there, also reviews;
/// Selin reviews; Yusuf administers; the scanner reports in every catalog; Can is merely signed in.
fn grants(store: &MemoryStore) {
    let public = || Where::In(CatalogId::Public);
    store.grant(public(), "ayse", Role::Publisher);
    store.join("ayse", PUBLISHER);
    store.grant(public(), "mehmet", Role::Publisher);
    store.grant(public(), "mehmet", Role::Reviewer);
    store.join("mehmet", PUBLISHER);
    store.grant(public(), "selin", Role::Reviewer);
    store.grant(public(), "yusuf", Role::CatalogAdmin);
    store.grant(Where::Every, "tarayici", Role::Scanner);
}

fn catalog() -> (Router, Arc<MemoryStore>) {
    let store = Arc::new(MemoryStore::default());
    grants(&store);
    let app = app_with(Some(Catalog::new(store.clone(), Accounts::Named)));
    (app, store)
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

fn request(via: Via, method: &str, uri: &str, account: &str) -> axum::http::request::Builder {
    let b = Request::builder()
        .method(method)
        .uri(uri)
        .header("x-kentos-test-account", account);
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

#[tokio::test]
async fn a_release_goes_from_upload_to_installation_through_the_api() {
    let (app, store) = catalog();
    let req = request(Via::Web, "POST", PUBLIC, "ayse")
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from(release("1.0.0")))
        .unwrap();
    let (status, headers, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        headers[header::LOCATION],
        "/v1/extension-catalogs/public/releases/tr.ornek.altlik/1.0.0"
    );
    let view: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        (view["view"].as_str(), state_of(&view)),
        (Some("insider"), "draft")
    );
    let content = package_of(&view);
    let at = uri(PUBLIC, "1.0.0");
    scan_all(&app, &at, &content).await;
    // A program sends it for review and publishes what a person approved.
    let submit = json!({ "action": "submit", "note": "İlk sürüm." });
    let (status, view) = act(&app, Via::Python, "ayse", &at, submit).await;
    assert_eq!((status, state_of(&view)), (StatusCode::OK, "submitted"));
    let (status, view) = act(
        &app,
        Via::Web,
        "selin",
        &at,
        json!({ "action": "startReview" }),
    )
    .await;
    assert_eq!((status, state_of(&view)), (StatusCode::OK, "inReview"));
    assert_eq!(view["release"]["focus"]["depth"], "deep");
    let approve = json!({ "action": "approve", "note": "Yetenekler gerekçeleriyle tutuyor." });
    let (status, view) = act(&app, Via::Web, "selin", &at, approve).await;
    assert_eq!((status, state_of(&view)), (StatusCode::OK, "approved"));
    let (status, view) = act(&app, Via::Mcp, "ayse", &at, json!({ "action": "publish" })).await;
    assert_eq!((status, state_of(&view)), (StatusCode::OK, "published"));
    // Every move is on the stored record, with who and how.
    let stored = store.stored(&key("1.0.0")).unwrap();
    let moves: Vec<(Action, &str, Channel)> = stored
        .audit()
        .iter()
        .filter(|r| r.action != Action::Scan)
        .map(|r| (r.action, r.by.as_str(), r.via))
        .collect();
    assert_eq!(
        moves,
        [
            (Action::Upload, "ayse", Channel::Window),
            (Action::Submit, "ayse", Channel::Program),
            (Action::StartReview, "selin", Channel::Window),
            (Action::Approve, "selin", Channel::Window),
            (Action::Publish, "ayse", Channel::Program),
        ]
    );
    // Anyone signed in sees what was published, without the record's accounts, notes and findings.
    let (status, public) = get(&app, Via::Web, &at, "can").await;
    assert_eq!(
        (status, public["view"].as_str(), state_of(&public)),
        (StatusCode::OK, Some("public"), "published")
    );
    let text = public.to_string();
    assert!(
        public.get("release").is_none() && !text.contains("selin") && !text.contains("İlk sürüm"),
        "{text}"
    );
    let history: Vec<&str> = public["history"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["action"].as_str())
        .collect();
    assert_eq!(
        history,
        ["upload", "submit", "startReview", "approve", "publish"]
    );
    // … finds it in the list, and installs exactly the approved content.
    let (status, list) = get(&app, Via::Web, PUBLIC, "can").await;
    assert_eq!(
        (
            status,
            list["total"].as_u64(),
            &list["releases"][0]["listed"]
        ),
        (StatusCode::OK, Some(1), &json!(true))
    );
    let (status, ticket) = get(&app, Via::Desktop, &format!("{at}/install"), "can").await;
    assert_eq!(
        (status, ticket["content"]["package"].as_str()),
        (StatusCode::OK, Some(content.as_str()))
    );
    assert_eq!(ticket["depth"], "deep");
    let req = request(Via::Desktop, "GET", &format!("{at}/package"), "can")
        .body(Body::empty())
        .unwrap();
    let (status, headers, bytes) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["x-kentos-content-digest"], content.as_str());
    let opened = kentos_extensions::open(&bytes, &Limits::default());
    assert_eq!(
        ReleaseFacts::of(&opened).unwrap().content().package,
        content
    );
}

#[tokio::test]
async fn a_program_never_decides_whoever_s_token_it_carries() {
    let (app, store) = catalog();
    in_review(&app, "1.0.0").await;
    let at = uri(PUBLIC, "1.0.0");
    let before = store.stored(&key("1.0.0")).unwrap();
    let decisions = [
        json!({ "action": "approve" }),
        json!({ "action": "requestChanges", "changes": [{ "code": "broken" }] }),
    ];
    for via in [Via::Token, Via::Python, Via::Desktop, Via::Mcp] {
        for decision in &decisions {
            let (status, body) = act(&app, via, "selin", &at, decision.clone()).await;
            assert_eq!(
                (status, code(&body)),
                (StatusCode::FORBIDDEN, "decision_by_program".to_owned()),
                "{via:?} {decision}"
            );
        }
    }
    assert_eq!(
        store.stored(&key("1.0.0")).unwrap(),
        before,
        "nothing moved"
    );
    // Publishing what nobody approved is the flow's own refusal, whatever asks.
    for via in [Via::Token, Via::Web] {
        let (status, body) = act(&app, via, "ayse", &at, json!({ "action": "publish" })).await;
        assert_eq!(
            (status, code(&body)),
            (StatusCode::CONFLICT, "not_approved".to_owned())
        );
    }
    // The rule itself: only the web app's own request, without an access token, is the catalog's window.
    let headers = |pairs: &[(&'static str, &'static str)]| -> HeaderMap {
        pairs
            .iter()
            .map(|(k, v)| (HeaderName::from_static(k), HeaderValue::from_static(v)))
            .collect()
    };
    assert_eq!(
        channel(&headers(&[("x-kentos-client", "web")])),
        Channel::Window
    );
    let programs: [&[(&'static str, &'static str)]; 6] = [
        &[],
        &[("x-kentos-client", "python")],
        &[("x-kentos-client", "mcp")],
        &[("x-kentos-client", "desktop")],
        &[("authorization", "Bearer x")],
        &[("x-kentos-client", "web"), ("authorization", "Bearer x")],
    ];
    for pairs in programs {
        assert_eq!(channel(&headers(pairs)), Channel::Program, "{pairs:?}");
    }
}

#[tokio::test]
async fn nobody_reviews_or_approves_their_own_release() {
    let (app, store) = catalog();
    // Ayşe, its author, also reviews in this catalog; Mehmet is a member of her publisher.
    store.grant(Where::In(CatalogId::Public), "ayse", Role::Reviewer);
    let (_, view) = upload(&app, PUBLIC, "ayse", release("1.0.0")).await;
    let at = uri(PUBLIC, "1.0.0");
    scan_all(&app, &at, &package_of(&view)).await;
    act(&app, Via::Web, "ayse", &at, json!({ "action": "submit" })).await;
    for own in ["ayse", "mehmet"] {
        let (status, body) =
            act(&app, Via::Web, own, &at, json!({ "action": "startReview" })).await;
        assert_eq!(
            (status, code(&body)),
            (StatusCode::FORBIDDEN, "self_review".to_owned()),
            "{own}"
        );
    }
    let (status, _) = act(
        &app,
        Via::Web,
        "selin",
        &at,
        json!({ "action": "startReview" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for own in ["ayse", "mehmet"] {
        let (status, body) = act(&app, Via::Web, own, &at, json!({ "action": "approve" })).await;
        assert_eq!(
            (status, code(&body)),
            (StatusCode::FORBIDDEN, "self_review".to_owned()),
            "{own}"
        );
    }
    let (status, view) = act(&app, Via::Web, "selin", &at, json!({ "action": "approve" })).await;
    assert_eq!((status, state_of(&view)), (StatusCode::OK, "approved"));
}

#[tokio::test]
async fn new_content_drops_the_approval_and_needs_its_own() {
    let (app, store) = catalog();
    let first = in_review(&app, "1.0.0").await;
    let at = uri(PUBLIC, "1.0.0");
    act(&app, Via::Web, "selin", &at, json!({ "action": "approve" })).await;
    // Approved content is fixed; the refused bytes are not even kept.
    let other = kek(Some(PUBLISHER), "1.0.0", "Başka bir özet.");
    let (status, body) = send_bytes(&app, "PUT", &at, "ayse", other.clone()).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "content_frozen".to_owned())
    );
    assert_eq!(store.packages(), 1);
    // Sent back before publication: the approval goes, and new content starts every gate again.
    let send_back = json!({ "action": "requestChanges",
        "changes": [{ "code": "listing_misleading", "note": "Özet servisleri anlatmıyor." }] });
    let (status, view) = act(&app, Via::Web, "selin", &at, send_back).await;
    assert_eq!(
        (status, state_of(&view)),
        (StatusCode::OK, "changesRequested")
    );
    assert!(view["release"].get("approval").is_none(), "{view}");
    assert_eq!(
        view["release"]["changeRequest"]["reasons"][0]["code"],
        "listing_misleading"
    );
    let (status, view) = send_bytes(&app, "PUT", &at, "ayse", other).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let second = package_of(&view);
    assert_ne!(second, first);
    assert!(
        view["release"]["gates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|g| g["status"] == "verifying" && g.get("at").is_none()),
        "{view}"
    );
    let (status, body) = act(&app, Via::Mcp, "ayse", &at, json!({ "action": "publish" })).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "not_approved".to_owned())
    );
    // A late report about the first content does not verify the second.
    let late =
        json!({ "action": "scan", "gate": "secrets", "content": first, "outcome": "passed" });
    let (status, body) = act(&app, Via::Token, "tarayici", &at, late).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "scan_stale".to_owned())
    );
    // The second content goes through again, and it is what installs.
    scan_all(&app, &at, &second).await;
    act(
        &app,
        Via::Python,
        "ayse",
        &at,
        json!({ "action": "submit" }),
    )
    .await;
    act(
        &app,
        Via::Web,
        "selin",
        &at,
        json!({ "action": "startReview" }),
    )
    .await;
    act(&app, Via::Web, "selin", &at, json!({ "action": "approve" })).await;
    let (status, view) = act(
        &app,
        Via::Token,
        "ayse",
        &at,
        json!({ "action": "publish" }),
    )
    .await;
    assert_eq!((status, state_of(&view)), (StatusCode::OK, "published"));
    let (_, ticket) = get(&app, Via::Desktop, &format!("{at}/install"), "can").await;
    assert_eq!(ticket["content"]["package"].as_str(), Some(second.as_str()));
    assert_eq!(store.packages(), 2);
}

#[tokio::test]
async fn what_a_caller_may_not_see_answers_404_and_is_not_counted() {
    let (app, store) = catalog();
    upload(&app, PUBLIC, "ayse", release("1.0.0")).await;
    let at = uri(PUBLIC, "1.0.0");
    // Can, outside the publisher and the catalog's staff: no draft, no count, no search hit, no move.
    let (status, hidden) = get(&app, Via::Web, &at, "can").await;
    assert_eq!(
        (status, code(&hidden)),
        (StatusCode::NOT_FOUND, "not_found".to_owned())
    );
    for query in [
        "",
        "?q=altlik",
        "?state=draft",
        "?extension=tr.ornek.altlik",
    ] {
        let (status, list) = get(&app, Via::Web, &format!("{PUBLIC}{query}"), "can").await;
        assert_eq!(
            (status, list["total"].as_u64()),
            (StatusCode::OK, Some(0)),
            "{query}"
        );
    }
    let (status, moved) = act(&app, Via::Web, "can", &at, json!({ "action": "approve" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, unknown) = get(&app, Via::Web, &uri(PUBLIC, "9.9.9"), "can").await;
    assert_eq!(moved["message"], unknown["message"], "word for word");
    assert_eq!(hidden["message"], unknown["message"]);
    // Its publisher's members and the catalog's reviewers see it.
    for account in ["ayse", "mehmet", "selin"] {
        let (_, list) = get(&app, Via::Web, &format!("{PUBLIC}?q=ALTLIK"), account).await;
        assert_eq!(list["total"].as_u64(), Some(1), "{account}");
    }
    // An organisation's catalog is not there for someone outside it, nor is a catalog that does not exist.
    store.grant(
        Where::In(CatalogId::Tenant("buro".into())),
        "bora",
        Role::Publisher,
    );
    let buro = "/v1/extension-catalogs/tenant:buro/releases";
    assert_eq!(
        get(&app, Via::Web, buro, "can").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(get(&app, Via::Web, buro, "bora").await.0, StatusCode::OK);
    // A member let in without a role sees what is published there.
    store.admit(CatalogId::Tenant("buro".into()), "deniz");
    assert_eq!(get(&app, Via::Web, buro, "deniz").await.0, StatusCode::OK);
    for missing in ["herkes", "tenant:B", "personal:"] {
        let (status, _) = get(
            &app,
            Via::Web,
            &format!("/v1/extension-catalogs/{missing}/releases"),
            "can",
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{missing}");
    }
    // A state the flow does not know is the request's mistake.
    let (status, body) = get(&app, Via::Web, &format!("{PUBLIC}?state=hazir"), "can").await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::UNPROCESSABLE_ENTITY, "invalid".to_owned())
    );
}

#[tokio::test]
async fn only_a_published_release_s_approved_content_installs() {
    let (app, _store) = catalog();
    let content = in_review(&app, "1.0.0").await;
    let at = uri(PUBLIC, "1.0.0");
    let (install, package) = (format!("{at}/install"), format!("{at}/package"));
    for path in [&install, &package] {
        let (status, body) = get(&app, Via::Desktop, path, "ayse").await;
        assert_eq!(
            (status, code(&body)),
            (StatusCode::CONFLICT, "not_installable".to_owned()),
            "{path}"
        );
        // Someone who may not see it is not told it exists.
        assert_eq!(
            get(&app, Via::Desktop, path, "can").await.0,
            StatusCode::NOT_FOUND
        );
    }
    act(&app, Via::Web, "selin", &at, json!({ "action": "approve" })).await;
    act(&app, Via::Web, "ayse", &at, json!({ "action": "publish" })).await;
    let (status, ticket) = get(&app, Via::Desktop, &install, "can").await;
    assert_eq!(
        (
            status,
            ticket["content"]["package"].as_str(),
            ticket["publisher"].as_str()
        ),
        (StatusCode::OK, Some(content.as_str()), Some(PUBLISHER))
    );
    // Suspended: no install, the reason given, and honestly that copies already downloaded stay where they are.
    let suspend = json!({ "action": "suspend", "reason": "Şüpheli ağ trafiği bildirildi." });
    let (status, _) = act(&app, Via::Web, "yusuf", &at, suspend).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = get(&app, Via::Desktop, &install, "can").await;
    let message = body["message"].as_str().unwrap_or_default();
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "not_installable".to_owned())
    );
    assert!(
        message.contains("Şüpheli ağ trafiği bildirildi") && message.contains("geri alınamaz"),
        "{message}"
    );
    assert_eq!(
        get(&app, Via::Desktop, &package, "can").await.0,
        StatusCode::CONFLICT
    );
}

/// A version whose first map service moved to another path on the same server: a change of behaviour, not of text.
fn moved_service(version: &str, path: &str) -> Vec<u8> {
    kek_with(|m| {
        m["publisherId"] = json!(PUBLISHER);
        m["version"] = json!(version);
        m["contributions"]["dataSources"][0]["url"] =
            json!(format!("https://harita.ornek.bel.tr/{path}"));
    })
}

/// Uploaded, scanned, sent and taken into review by Selin; the focus the server computed, then approved and published.
async fn reviewed_and_published(app: &Router, version: &str, bytes: Vec<u8>) -> Value {
    let (status, view) = upload(app, PUBLIC, "ayse", bytes).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    let at = uri(PUBLIC, version);
    scan_all(app, &at, &package_of(&view)).await;
    act(app, Via::Python, "ayse", &at, json!({ "action": "submit" })).await;
    let (status, view) = act(
        app,
        Via::Web,
        "selin",
        &at,
        json!({ "action": "startReview" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{view}");
    act(app, Via::Web, "selin", &at, json!({ "action": "approve" })).await;
    let (status, published) = act(app, Via::Mcp, "ayse", &at, json!({ "action": "publish" })).await;
    assert_eq!(
        (status, state_of(&published)),
        (StatusCode::OK, "published")
    );
    view["release"]["focus"].clone()
}

#[tokio::test]
async fn the_review_reads_the_difference_and_a_trusted_publisher_s_code_takes_the_fast_path() {
    let (app, store) = catalog();
    let first = reviewed_and_published(&app, "1.0.0", release("1.0.0")).await;
    assert_eq!(
        (&first["depth"], first.get("previous")),
        (&json!("deep"), None)
    );
    // A moved service is behaviour: read as a difference against the version approved before.
    let second = reviewed_and_published(&app, "1.1.0", moved_service("1.1.0", "wmts-2026")).await;
    assert_eq!(
        (
            &second["depth"],
            &second["previous"],
            second.get("trustedFastPath")
        ),
        (&json!("standard"), &json!("1.0.0"), None)
    );
    assert_eq!(
        second["differences"][0],
        json!({ "kind": "code", "subject": "manifest.json", "detail": "değişen alanlar: contributions" })
    );
    // Once the catalog trusts the publisher, its code with nothing new asked for takes the fast path.
    store.trust(CatalogId::Public, PUBLISHER);
    let third = reviewed_and_published(&app, "1.2.0", moved_service("1.2.0", "wmts-2027")).await;
    assert_eq!(
        (
            &third["depth"],
            &third["previous"],
            &third["trustedFastPath"]
        ),
        (&json!("fast"), &json!("1.1.0"), &json!(true))
    );
}

#[tokio::test]
async fn a_refusal_keeps_the_flow_s_code_and_turkish_reason() {
    let (app, store) = catalog();
    // The API's error body: the flow's code, its reason and remedy in Turkish, the request's id.
    upload(&app, PUBLIC, "ayse", release("1.0.0")).await;
    let at = uri(PUBLIC, "1.0.0");
    let (status, body) = act(&app, Via::Web, "ayse", &at, json!({ "action": "approve" })).await;
    let error: ApiError = serde_json::from_value(body).unwrap();
    assert_eq!(
        (status, error.error.as_str(), error.retryable),
        (StatusCode::FORBIDDEN, "permission_missing", false)
    );
    assert!(
        error.message.contains("“Onaylama” yetkisi gerekir"),
        "{}",
        error.message
    );
    assert!(error.request_id.is_some());
    // Bytes that are not a package: what is wrong, in the inspector's words.
    let (status, body) = upload(&app, PUBLIC, "ayse", b"bir paket degil".to_vec()).await;
    let message = body["message"].as_str().unwrap_or_default();
    assert_eq!(
        (status, code(&body)),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "package_invalid".to_owned()
        )
    );
    assert!(
        message.starts_with("Paket denetimden geçmedi: "),
        "{message}"
    );
    // No publisher in the manifest; the same version again; another publisher's extension.
    let (status, body) = upload(&app, PUBLIC, "ayse", kek(None, "1.1.0", "Özet.")).await;
    assert_eq!(
        (status, code(&body)),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            "publisher_missing".to_owned()
        )
    );
    let again = kek(Some(PUBLISHER), "1.0.0", "Başka bir özet.");
    let (status, body) = upload(&app, PUBLIC, "ayse", again).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "version_taken".to_owned())
    );
    store.grant(Where::In(CatalogId::Public), "kerem", Role::Publisher);
    store.join("kerem", "baska-belediye");
    let squat = kek(Some("baska-belediye"), "2.0.0", "Özet.");
    let (status, body) = upload(&app, PUBLIC, "kerem", squat).await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::CONFLICT, "extension_owned".to_owned())
    );
    // A move the flow does not have, and a field it does not know, are the request's mistakes.
    for wrong in [
        json!({ "action": "teleport" }),
        json!({ "action": "submit", "hemen": true }),
    ] {
        let (status, body) = act(&app, Via::Web, "ayse", &at, wrong.clone()).await;
        assert_eq!(
            (status, code(&body)),
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid".to_owned()),
            "{wrong}"
        );
    }
    // A change without the app's header is refused before anything, as on every route.
    let req = Request::post(format!("{at}/moves"))
        .header("x-kentos-test-account", "ayse")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "action": "submit" }).to_string()))
        .unwrap();
    assert_eq!(call(&app, req).await.0, StatusCode::FORBIDDEN);
}

#[test]
fn every_refusal_has_its_status() {
    let table: [(&[&str], StatusCode); 4] = [
        (
            &[
                "permission_missing",
                "not_owner",
                "self_review",
                "self_scan",
                "decision_by_program",
            ],
            StatusCode::FORBIDDEN,
        ),
        (
            &[
                "reason_missing",
                "changes_missing",
                "package_invalid",
                "identity_mismatch",
                "publisher_mismatch",
                "publisher_missing",
                "scan_unexplained",
                "waiver_not_allowed",
            ],
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            &[
                "wrong_state",
                "not_approved",
                "content_frozen",
                "gates_closed",
                "gates_open",
                "approval_stale",
                "gate_blocked",
                "unchanged_resubmit",
                "version_taken",
                "extension_owned",
                "scan_stale",
                "waiver_not_blocked",
                "content_unchanged",
                "not_installable",
                "not_runnable",
                "content_mismatch",
                "release_changed",
            ],
            StatusCode::CONFLICT,
        ),
        (
            &[
                "package_mismatch",
                "baseline_invalid",
                "record_invalid",
                "record_inconsistent",
            ],
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    ];
    for (codes, status) in table {
        for code in codes {
            assert_eq!(status_of(code), status, "{code}");
        }
    }
}

#[tokio::test]
async fn one_s_own_catalog_is_one_s_own() {
    let (app, _store) = catalog();
    let mine = "/v1/extension-catalogs/personal:can/releases";
    // No catalog role needed and nobody else in it: its owner reviews there, the gates still pass first.
    let (status, view) = upload(&app, mine, "can", release("1.0.0")).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    let at = uri(mine, "1.0.0");
    scan_all(&app, &at, &package_of(&view)).await;
    act(&app, Via::Web, "can", &at, json!({ "action": "submit" })).await;
    act(
        &app,
        Via::Web,
        "can",
        &at,
        json!({ "action": "startReview" }),
    )
    .await;
    // Still, a program decides nothing there either.
    let (status, body) = act(
        &app,
        Via::Python,
        "can",
        &at,
        json!({ "action": "approve" }),
    )
    .await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::FORBIDDEN, "decision_by_program".to_owned())
    );
    let (status, view) = act(&app, Via::Web, "can", &at, json!({ "action": "approve" })).await;
    assert_eq!((status, state_of(&view)), (StatusCode::OK, "approved"));
    for other in ["ayse", "selin", "yusuf"] {
        assert_eq!(
            get(&app, Via::Web, &at, other).await.0,
            StatusCode::NOT_FOUND,
            "{other}"
        );
    }
}

/// A store whose first write of a moved release loses to another request's move.
struct Racy {
    inner: MemoryStore,
    raced: AtomicBool,
}

impl ReleaseStore for Racy {
    fn releases<'a>(&'a self, catalog: &'a CatalogId) -> StoreFuture<'a, Vec<Release>> {
        self.inner.releases(catalog)
    }

    fn release<'a>(&'a self, key: &'a ReleaseKey) -> StoreFuture<'a, Option<Release>> {
        self.inner.release(key)
    }

    fn write<'a>(
        &'a self,
        key: &'a ReleaseKey,
        moves: Option<usize>,
        release: &'a Release,
    ) -> StoreFuture<'a, ()> {
        if moves.is_some() && !self.raced.swap(true, Ordering::SeqCst) {
            return Box::pin(async { Err(StoreError::Changed) });
        }
        self.inner.write(key, moves, release)
    }

    fn put_package<'a>(&'a self, digest: &'a str, bytes: Bytes) -> StoreFuture<'a, ()> {
        self.inner.put_package(digest, bytes)
    }

    fn package<'a>(&'a self, digest: &'a str) -> StoreFuture<'a, Option<Bytes>> {
        self.inner.package(digest)
    }

    fn access<'a>(&'a self, catalog: &'a CatalogId, account: &'a str) -> StoreFuture<'a, Access> {
        self.inner.access(catalog, account)
    }

    fn trusted<'a>(&'a self, catalog: &'a CatalogId, publisher: &'a str) -> StoreFuture<'a, bool> {
        self.inner.trusted(catalog, publisher)
    }
}

#[tokio::test]
async fn a_move_that_lost_a_race_is_made_again_on_the_newer_record_once() {
    let racy = Arc::new(Racy {
        inner: MemoryStore::default(),
        raced: AtomicBool::new(false),
    });
    grants(&racy.inner);
    let app = app_with(Some(Catalog::new(racy.clone(), Accounts::Named)));
    let (_, view) = upload(&app, PUBLIC, "ayse", release("1.0.0")).await;
    let at = uri(PUBLIC, "1.0.0");
    let report = json!({ "action": "scan", "gate": "secrets", "content": package_of(&view), "outcome": "passed" });
    let (status, view) = act(&app, Via::Token, "tarayici", &at, report).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert!(racy.raced.load(Ordering::SeqCst));
    let stored = racy.inner.stored(&key("1.0.0")).unwrap();
    let actions: Vec<Action> = stored.audit().iter().map(|r| r.action).collect();
    assert_eq!(actions, [Action::Upload, Action::Scan]);
}

#[tokio::test]
async fn the_catalog_is_off_unless_the_server_has_one() {
    let (status, body) = get(&app_with(None), Via::Web, PUBLIC, "ayse").await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::NOT_FOUND, "not_found".to_owned())
    );
    // With the server's own sessions and no database, it answers as every protected route does then.
    let sessions = Catalog::new(Arc::new(MemoryStore::default()), Accounts::Sessions);
    let (status, body) = get(&app_with(Some(sessions)), Via::Web, PUBLIC, "ayse").await;
    assert_eq!(
        (status, code(&body)),
        (StatusCode::SERVICE_UNAVAILABLE, "unavailable".to_owned())
    );
}
