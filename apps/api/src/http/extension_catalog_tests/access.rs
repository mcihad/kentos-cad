//! Who may do what and who may see what: a program never decides, a person decides only signed in afresh, nobody
//! reviews their own release, what a caller may not see answers 404 and is not counted, one's own catalog is one's
//! own, and the catalog is off unless the server has one. On both stores.

use axum::http::{HeaderMap, HeaderName, HeaderValue};
use kentos_extensions::release::{Channel, ReleaseState};

use super::*;
use crate::extension_catalog::memory::MemoryStore;
use crate::extension_catalog::{Account, CatalogError, Move};
use crate::http::extension_catalog::channel;

on_both_stores!(
    a_program_never_decides_whoever_s_token_it_carries,
    a_person_decides_only_signed_in_afresh,
    nobody_reviews_or_approves_their_own_release,
    what_a_caller_may_not_see_answers_404_and_is_not_counted,
    one_s_own_catalog_is_one_s_own,
);

async fn a_program_never_decides_whoever_s_token_it_carries(bench: Bench) {
    let app = catalog(&bench).await;
    in_review(&app, "1.0.0").await;
    let at = uri(PUBLIC, "1.0.0");
    let before = bench.stored(&key("1.0.0")).await.unwrap();
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
        bench.stored(&key("1.0.0")).await.unwrap(),
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

/// The server's sessions carry no fresh sign-in yet (docs/adr/8059 §13): a person's decision in the window is refused
/// after the flow would have allowed it, and nothing moves; the flow's own refusals come first; what is not a decision
/// goes on; the same person signed in afresh decides.
async fn a_person_decides_only_signed_in_afresh(bench: Bench) {
    let app = catalog(&bench).await;
    in_review(&app, "1.0.0").await;
    let catalog = Catalog::new(bench.store(), Accounts::Named);
    let at = key("1.0.0");
    let before = bench.stored(&at).await.unwrap();
    let account = |name: &str, via: Channel| Account {
        id: id(name),
        via,
        fresh_sign_in: false,
    };
    let approve = Move::Approve { note: None };
    let refused = |e: CatalogError| match e {
        CatalogError::Refused(r) => (r.code, r.message),
        other => panic!("{other:?}"),
    };
    let (code, message) = refused(
        catalog
            .act(&at, &account("selin", Channel::Window), &approve)
            .await
            .unwrap_err(),
    );
    assert_eq!(code, "fresh_sign_in_required");
    assert!(
        message.contains("“Onayla”") && message.contains("taze bir kimlik doğrulaması"),
        "{message}"
    );
    let send_back = Move::RequestChanges {
        changes: serde_json::from_value(json!([{ "code": "broken" }])).unwrap(),
    };
    let (code, _) = refused(
        catalog
            .act(&at, &account("selin", Channel::Window), &send_back)
            .await
            .unwrap_err(),
    );
    assert_eq!(code, "fresh_sign_in_required");
    // A program is never a person, the author no reviewer, and an unseen release is not there.
    for (name, via, expected) in [
        ("selin", Channel::Program, "decision_by_program"),
        ("ayse", Channel::Window, "permission_missing"),
    ] {
        let (code, _) = refused(
            catalog
                .act(&at, &account(name, via), &approve)
                .await
                .unwrap_err(),
        );
        assert_eq!(code, expected, "{name}");
    }
    assert!(matches!(
        catalog
            .act(&key("9.9.9"), &account("selin", Channel::Window), &approve)
            .await,
        Err(CatalogError::NotFound)
    ));
    assert_eq!(bench.stored(&at).await.unwrap(), before, "nothing moved");
    // The same reviewer, signed in afresh.
    let fresh = Account {
        fresh_sign_in: true,
        ..account("selin", Channel::Window)
    };
    let (approved, _) = catalog.act(&at, &fresh, &approve).await.unwrap();
    assert_eq!(approved.state(), ReleaseState::Approved);
    // Publishing is the publisher's, not a decision: it goes on without a fresh sign-in.
    let (published, _) = catalog
        .act(&at, &account("ayse", Channel::Program), &Move::Publish)
        .await
        .unwrap();
    assert_eq!(published.state(), ReleaseState::Published);
}

async fn nobody_reviews_or_approves_their_own_release(bench: Bench) {
    let app = catalog(&bench).await;
    // Ayşe, its author, also reviews in this catalog; Mehmet is a member of her publisher.
    bench
        .grant(Where::In(CatalogId::Public), "ayse", Role::Reviewer)
        .await;
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

async fn what_a_caller_may_not_see_answers_404_and_is_not_counted(bench: Bench) {
    let app = catalog(&bench).await;
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
    bench
        .grant(
            Where::In(CatalogId::Tenant("buro".into())),
            "bora",
            Role::Publisher,
        )
        .await;
    let buro = "/v1/extension-catalogs/tenant:buro/releases";
    assert_eq!(
        get(&app, Via::Web, buro, "can").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(get(&app, Via::Web, buro, "bora").await.0, StatusCode::OK);
    // A member let in without a role sees what is published there.
    bench.admit(CatalogId::Tenant("buro".into()), "deniz").await;
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

async fn one_s_own_catalog_is_one_s_own(bench: Bench) {
    let app = catalog(&bench).await;
    let mine = personal("can");
    // No catalog role needed and nobody else in it: its owner reviews there, the gates still pass first.
    let (status, view) = upload(&app, &mine, "can", release("1.0.0")).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    let at = uri(&mine, "1.0.0");
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
