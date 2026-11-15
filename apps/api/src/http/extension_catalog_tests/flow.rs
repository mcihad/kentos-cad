//! The flow through the API: a release from upload to installation, only a published release's approved content
//! installs, new content needs its own approval, and the review reads the difference from the version approved before.

use kentos_extensions::release::{Action, Channel, ReleaseFacts};

use super::*;

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
