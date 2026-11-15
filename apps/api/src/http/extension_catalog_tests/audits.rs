//! The catalog's side of `project.audit` jobs (docs/adr/8057 §42): the release a cloud job runs is a published one (the
//! newest, or the version asked when it may run) and its bytes are the approved content; the worker's audit source
//! looks in the organisation's catalog, then in the public one, and gives the interpreter the server prepared; an
//! organisation's catalog that knows the extension decides alone.

use std::sync::Arc;

use axum::Router;
use axum::http::StatusCode;
use kentos_application::audit_catalog::{AuditPackage, AuditPythons, AuditSource};
use kentos_extensions::Limits;
use kentos_extensions::release::Role;
use serde_json::json;

use super::{
    AppState, PUBLIC, PUBLISHER, Via, act, config, grants, in_review, kek, package_of, release,
    router, scan_all, state_of, upload, uri,
};
use crate::extension_catalog::audits::CatalogAudits;
use crate::extension_catalog::memory::{MemoryStore, Where};
use crate::extension_catalog::{Accounts, Catalog, CatalogError, CatalogId};

const EXTENSION: &str = "tr.ornek.altlik";
const BURO: &str = "/v1/extension-catalogs/tenant:buro/releases";

/// The routes and the catalog behind them, shared: the routes move the releases, the job reads the catalog. In the
/// organisation Büro's catalog Ayşe publishes, Selin reviews and Yusuf administers too.
fn shared() -> (Router, Arc<Catalog>) {
    let store = Arc::new(MemoryStore::default());
    grants(&store);
    let buro = || Where::In(CatalogId::Tenant("buro".into()));
    store.grant(buro(), "ayse", Role::Publisher);
    store.grant(buro(), "selin", Role::Reviewer);
    store.grant(buro(), "yusuf", Role::CatalogAdmin);
    let catalog = Arc::new(Catalog::new(store, Accounts::Named));
    let app = router(AppState {
        config: Arc::new(config()),
        database: None,
        oidc: None,
        hub: crate::hub::Hub::default(),
        logins: Default::default(),
        blobs: kentos_application::blobs::Blobs::new(
            std::env::temp_dir().join(format!("kentos-api-blobs-{}", uuid::Uuid::now_v7())),
        ),
        proxy: None,
        extension_catalog: Some(catalog.clone()),
    });
    (app, catalog)
}

/// Uploaded, reviewed, approved by Selin and published by Ayşe; its content's digest.
async fn published(app: &Router, version: &str) -> String {
    let content = in_review(app, version).await;
    let at = uri(PUBLIC, version);
    act(app, Via::Web, "selin", &at, json!({ "action": "approve" })).await;
    let (status, view) = act(app, Via::Web, "ayse", &at, json!({ "action": "publish" })).await;
    assert_eq!((status, state_of(&view)), (StatusCode::OK, "published"));
    content
}

/// The newest published version, or the one asked when it may run; its bytes the approved content. A suspended
/// version does not run and the newest published before it does; a draft runs nowhere; an extension the catalog has
/// not is not found.
#[tokio::test]
async fn a_cloud_job_runs_the_newest_published_release_or_the_one_asked() {
    let (app, catalog) = shared();
    let first = published(&app, "1.0.0").await;
    let second = published(&app, "1.1.0").await;
    let public = CatalogId::Public;
    let (newest, bytes) = catalog.runnable(&public, EXTENSION, None).await.unwrap();
    assert_eq!(
        (newest.version(), newest.content().package.as_str()),
        ("1.1.0", second.as_str())
    );
    let opened = kentos_extensions::inspect(&bytes, &Limits::default());
    assert_eq!(opened.digest.as_deref(), Some(second.as_str()));
    let (asked, _) = catalog
        .runnable(&public, EXTENSION, Some("1.0.0"))
        .await
        .unwrap();
    assert_eq!(asked.content().package, first);
    let suspend = json!({ "action": "suspend", "reason": "Şüpheli ağ trafiği bildirildi." });
    let (status, _) = act(&app, Via::Web, "yusuf", &uri(PUBLIC, "1.1.0"), suspend).await;
    assert_eq!(status, StatusCode::OK);
    let (now, _) = catalog.runnable(&public, EXTENSION, None).await.unwrap();
    assert_eq!(now.version(), "1.0.0", "the newest published before it");
    assert!(matches!(
        catalog.runnable(&public, EXTENSION, Some("1.1.0")).await,
        Err(CatalogError::Refused(r)) if r.code == "not_runnable"
    ));
    let (status, _) = upload(&app, PUBLIC, "ayse", release("1.2.0")).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(matches!(
        catalog.runnable(&public, EXTENSION, Some("1.2.0")).await,
        Err(CatalogError::Refused(_))
    ));
    assert!(matches!(
        catalog.runnable(&public, "tr.ornek.yok", None).await,
        Err(CatalogError::NotFound)
    ));
}

/// The worker's source: the organisation's catalog has no such extension, so the public one's release is taken, with
/// where it came from; the interpreter is the one the server prepared for that version; what cannot run is said.
#[tokio::test]
async fn the_audit_source_takes_the_organisation_s_catalog_then_the_public_one() {
    let (app, catalog) = shared();
    let content = published(&app, "1.0.0").await;
    let pythons = AuditPythons::parse(
        r#"{"tr.ornek.altlik@1.0.0": "/srv/kentos/python/altlik-1.0.0/bin/python"}"#,
    )
    .unwrap();
    let source = CatalogAudits::new(catalog, pythons);
    let package = source.release("buro", EXTENSION, None).await.unwrap();
    assert_eq!(
        (
            package.catalog.as_str(),
            package.version.as_str(),
            package.digest.as_str()
        ),
        ("public", "1.0.0", content.as_str())
    );
    assert_eq!(
        kentos_extensions::inspect(&package.bytes, &Limits::default()).digest,
        Some(content)
    );
    assert_eq!(
        source.python(EXTENSION, "1.0.0").as_deref(),
        Some("/srv/kentos/python/altlik-1.0.0/bin/python")
    );
    assert_eq!(source.python(EXTENSION, "1.1.0"), None);
    let missing = source
        .release("buro", "tr.ornek.yok", None)
        .await
        .unwrap_err();
    assert!(missing.contains("yayımlanmış bir sürümü"), "{missing}");
    let asked = source
        .release("buro", EXTENSION, Some("9.9.9"))
        .await
        .unwrap_err();
    assert!(asked.contains("9.9.9"), "{asked}");
}

/// Where a package came from and its content.
fn origin(package: AuditPackage) -> (String, String) {
    (package.catalog, package.digest)
}

/// The organisation's catalog decides for an extension it knows: its release is taken over the public one's of the
/// same id, and once it is suspended there the job is refused rather than given the public one (which may be another
/// publisher's). A draft there claims nothing: until it is published the public release runs; another organisation's
/// jobs take the public one.
#[tokio::test]
async fn the_organisation_s_catalog_decides_for_an_extension_it_knows() {
    let (app, catalog) = shared();
    let public = published(&app, "1.0.0").await;
    let source = CatalogAudits::new(catalog, AuditPythons::default());
    let ours = kek(Some(PUBLISHER), "1.0.0", "Büronun kendi altlık servisleri.");
    let (status, view) = upload(&app, BURO, "ayse", ours).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    let content = package_of(&view);
    assert_ne!(content, public);
    assert_eq!(
        origin(source.release("buro", EXTENSION, None).await.unwrap()),
        ("public".to_owned(), public.clone()),
        "a draft claims nothing"
    );
    let at = uri(BURO, "1.0.0");
    scan_all(&app, &at, &content).await;
    for (via, account, action) in [
        (Via::Python, "ayse", "submit"),
        (Via::Web, "selin", "startReview"),
        (Via::Web, "selin", "approve"),
        (Via::Web, "ayse", "publish"),
    ] {
        let (status, view) = act(&app, via, account, &at, json!({ "action": action })).await;
        assert_eq!(status, StatusCode::OK, "{action}: {view}");
    }
    assert_eq!(
        origin(source.release("buro", EXTENSION, None).await.unwrap()),
        ("tenant:buro".to_owned(), content)
    );
    assert_eq!(
        origin(source.release("diger", EXTENSION, None).await.unwrap()),
        ("public".to_owned(), public)
    );
    let suspend = json!({ "action": "suspend", "reason": "Şüpheli ağ trafiği bildirildi." });
    let (status, view) = act(&app, Via::Web, "yusuf", &at, suspend).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let refused = source.release("buro", EXTENSION, None).await.unwrap_err();
    assert!(
        refused.contains("genel katalogdaki aynı kimliğe bakılmadı"),
        "{refused}"
    );
}
