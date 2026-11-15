//! The catalog's revocation list over HTTP (docs/adr/8059 §12): every member gets the same list, and it tells nothing
//! an insider alone may know; everyone else gets the same 404 as on the other routes; an unchanged list costs a 304.

use kentos_extensions::revocations::Revocations;

use super::*;

const REVOCATIONS: &str = "/v1/extension-catalogs/public/revocations";

/// 1.0.0 published and withdrawn, 1.1.0 published and suspended, 1.2.0 still a draft.
async fn taken_out(app: &Router) {
    for version in ["1.0.0", "1.1.0"] {
        in_review(app, version).await;
        let at = uri(PUBLIC, version);
        act(app, Via::Web, "selin", &at, json!({ "action": "approve" })).await;
        let (status, view) = act(app, Via::Mcp, "ayse", &at, json!({ "action": "publish" })).await;
        assert_eq!((status, state_of(&view)), (StatusCode::OK, "published"));
    }
    let withdraw =
        json!({ "action": "withdraw", "reason": "Kullanıcı verisini dışarı gönderiyor." });
    let (status, _) = act(app, Via::Web, "yusuf", &uri(PUBLIC, "1.0.0"), withdraw).await;
    assert_eq!(status, StatusCode::OK);
    let suspend = json!({ "action": "suspend", "reason": "Şüpheli ağ trafiği bildirildi." });
    let (status, _) = act(app, Via::Web, "yusuf", &uri(PUBLIC, "1.1.0"), suspend).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = upload(app, PUBLIC, "ayse", release("1.2.0")).await;
    assert_eq!(status, StatusCode::CREATED);
}

fn fetch(uri: &str, account: &str, tag: Option<&str>) -> Request<Body> {
    let mut b = request(Via::Desktop, "GET", uri, account);
    if let Some(tag) = tag {
        b = b.header(header::IF_NONE_MATCH, tag);
    }
    b.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn every_member_gets_the_same_list_and_it_tells_no_insider_anything() {
    let (app, _store) = catalog();
    taken_out(&app).await;
    let (status, outsider) = get(&app, Via::Desktop, REVOCATIONS, "can").await;
    assert_eq!(status, StatusCode::OK, "{outsider}");
    let suspended = "Şüpheli ağ trafiği bildirildi (askıya alındı; inceleme bitince askıdan alınabilir ya da \
                     geri çekilir)";
    assert_eq!(
        outsider,
        json!({
            "format": "kentos.extension-revocations",
            "schemaVersion": 1,
            "entries": [
                { "id": "tr.ornek.altlik", "versions": "=1.0.0", "reason": "Kullanıcı verisini dışarı gönderiyor" },
                { "id": "tr.ornek.altlik", "versions": "=1.1.0", "reason": suspended },
            ],
        })
    );
    // The same for its author, a reviewer, the administrator and the scanner, by window, program or token.
    for (via, account) in [
        (Via::Web, "ayse"),
        (Via::Python, "ayse"),
        (Via::Web, "selin"),
        (Via::Web, "yusuf"),
        (Via::Token, "tarayici"),
    ] {
        let (status, list) = get(&app, via, REVOCATIONS, account).await;
        assert_eq!((status, &list), (StatusCode::OK, &outsider), "{account}");
    }
    let text = outsider.to_string();
    for insider in ["ayse", "selin", "yusuf", "tarayici", "sha256:", "1.2.0"] {
        assert!(!text.contains(insider), "{insider} in {text}");
    }
    // The host's own reader takes it as it comes.
    let list = Revocations::read(&text).unwrap();
    assert_eq!(
        list.reason("tr.ornek.altlik", "1.0.0"),
        Some("Kullanıcı verisini dışarı gönderiyor")
    );
    assert_eq!(list.reason("tr.ornek.altlik", "1.2.0"), None);
}

#[tokio::test]
async fn outside_a_catalog_the_list_answers_the_same_404() {
    let (app, store) = catalog();
    store.grant(
        Where::In(CatalogId::Tenant("buro".into())),
        "bora",
        Role::Publisher,
    );
    let buro = "/v1/extension-catalogs/tenant:buro/revocations";
    let (status, hidden) = get(&app, Via::Web, buro, "can").await;
    assert_eq!(
        (status, code(&hidden)),
        (StatusCode::NOT_FOUND, "not_found".to_owned())
    );
    let (status, list) = get(&app, Via::Web, buro, "bora").await;
    assert_eq!(
        (status, list["entries"].as_array().map(Vec::len)),
        (StatusCode::OK, Some(0))
    );
    // One's own catalog's list is its owner's alone: not even the catalog's scanner gets it.
    let mine = "/v1/extension-catalogs/personal:can/revocations";
    assert_eq!(get(&app, Via::Web, mine, "can").await.0, StatusCode::OK);
    for other in ["ayse", "yusuf", "tarayici"] {
        let (status, body) = get(&app, Via::Token, mine, other).await;
        assert_eq!(
            (status, code(&body)),
            (StatusCode::NOT_FOUND, "not_found".to_owned()),
            "{other}"
        );
        assert_eq!(body["message"], hidden["message"], "word for word");
    }
    let (status, _) = get(
        &app,
        Via::Web,
        "/v1/extension-catalogs/herkes/revocations",
        "can",
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    // And a release's 404 says the same.
    let (_, missing) = get(&app, Via::Web, &uri(PUBLIC, "9.9.9"), "can").await;
    assert_eq!(missing["message"], hidden["message"]);
}

#[tokio::test]
async fn an_unchanged_list_costs_a_304_and_a_new_withdrawal_makes_a_new_list() {
    let (app, _store) = catalog();
    let (status, headers, body) = send(&app, fetch(REVOCATIONS, "can", None)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "private, no-cache");
    assert_eq!(headers[header::CONTENT_TYPE], "application/json");
    let first = headers[header::ETAG].to_str().unwrap().to_owned();
    let empty: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(empty["entries"], json!([]));
    // Asked again with its tag: nothing to send.
    let (status, headers, body) = send(&app, fetch(REVOCATIONS, "can", Some(&first))).await;
    assert_eq!((status, body.len()), (StatusCode::NOT_MODIFIED, 0));
    assert_eq!(headers[header::ETAG], first.as_str());
    assert_eq!(headers[header::CACHE_CONTROL], "private, no-cache");
    // A withdrawal makes another list, which the old tag no longer stands for.
    in_review(&app, "1.0.0").await;
    let at = uri(PUBLIC, "1.0.0");
    act(&app, Via::Web, "selin", &at, json!({ "action": "approve" })).await;
    act(&app, Via::Mcp, "ayse", &at, json!({ "action": "publish" })).await;
    let withdraw = json!({ "action": "withdraw", "reason": "Kötü amaçlı kod bulundu." });
    assert_eq!(
        act(&app, Via::Web, "yusuf", &at, withdraw).await.0,
        StatusCode::OK
    );
    let (status, headers, body) = send(&app, fetch(REVOCATIONS, "can", Some(&first))).await;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(headers[header::ETAG].to_str().unwrap(), first);
    let list: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(list["entries"][0]["versions"], "=1.0.0");
}
