//! What the API says when it says no: the refusal's body with the flow's code and Turkish reason, its HTTP status, and
//! a move that lost a race to another, made again once on the newer record. On both stores.

use std::sync::atomic::{AtomicBool, Ordering};

use axum::body::Bytes;
use kentos_extensions::release::Action;

use super::*;
use crate::extension_catalog::store::{Access, StoreError, StoreFuture};
use crate::http::extension_catalog::status_of;

on_both_stores!(
    a_refusal_keeps_the_flow_s_code_and_turkish_reason,
    a_move_that_lost_a_race_is_made_again_on_the_newer_record_once,
);

async fn a_refusal_keeps_the_flow_s_code_and_turkish_reason(bench: Bench) {
    let app = catalog(&bench).await;
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
    bench
        .grant(Where::In(CatalogId::Public), "kerem", Role::Publisher)
        .await;
    bench.join("kerem", "baska-belediye").await;
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
        .header("x-kentos-test-account", id("ayse"))
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
                "fresh_sign_in_required",
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

/// A store whose first write of a moved release loses to another request's move.
struct Racy {
    inner: Arc<dyn ReleaseStore>,
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

async fn a_move_that_lost_a_race_is_made_again_on_the_newer_record_once(bench: Bench) {
    grants(&bench).await;
    let racy = Arc::new(Racy {
        inner: bench.store(),
        raced: AtomicBool::new(false),
    });
    let app = app_with(Some(Catalog::new(racy.clone(), Accounts::Named)));
    let (_, view) = upload(&app, PUBLIC, "ayse", release("1.0.0")).await;
    let at = uri(PUBLIC, "1.0.0");
    let report = json!({ "action": "scan", "gate": "secrets", "content": package_of(&view), "outcome": "passed" });
    let (status, view) = act(&app, Via::Token, "tarayici", &at, report).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert!(racy.raced.load(Ordering::SeqCst));
    let stored = bench.stored(&key("1.0.0")).await.unwrap();
    let actions: Vec<Action> = stored.audit().iter().map(|r| r.action).collect();
    assert_eq!(actions, [Action::Upload, Action::Scan]);
}
