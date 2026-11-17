//! The administrator's grants are what the persistent catalog reads (docs/adr/8059 §11, §13), and the catalog as a
//! running server opens it takes no person's decision.

use kentos_contracts::TenantRole;

use super::*;
use crate::extension_catalog::grants as granted;
use crate::extension_catalog::store::Access;
use crate::extension_catalog::{Account, Move};
use crate::http::tests::{json_req, signed_in};

#[tokio::test]
async fn the_administrator_s_grants_are_what_the_catalog_reads() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let owner = &db.owner;
    let ayse =
        kentos_application::admin::create_local_user(owner, "ayse", "Ayşe", None, "dogru-parola-1")
            .await
            .unwrap();
    kentos_application::admin::create_tenant(owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let store = PgStore::new(
        db.app.clone(),
        Blobs::new(std::env::temp_dir().join("kentos-katalog-yok")),
    );
    let buro = CatalogId::Tenant("buro".into());
    let place = Where::parse("tenant:buro").unwrap();
    let access = |catalog: CatalogId| {
        let store = &store;
        async move { store.access(&catalog, &ayse.to_string()).await.unwrap() }
    };
    assert_eq!(access(buro.clone()).await, Access::default(), "outside");
    granted::grant(owner, &place, "ayse", Role::Publisher)
        .await
        .unwrap();
    granted::publisher(owner, "ornek-belediye", Some("Örnek Belediyesi"))
        .await
        .unwrap();
    assert!(
        granted::member(owner, "ornek-belediye", "ayse", true)
            .await
            .unwrap()
    );
    assert!(
        !granted::member(owner, "ornek-belediye", "ayse", true)
            .await
            .unwrap(),
        "once"
    );
    granted::trust(owner, &place, "ornek-belediye", true)
        .await
        .unwrap();
    let inside = access(buro.clone()).await;
    assert!(inside.member);
    assert_eq!(inside.roles, [Role::Publisher].into());
    assert_eq!(inside.publishers, ["ornek-belediye".to_owned()].into());
    assert!(store.trusted(&buro, "ornek-belediye").await.unwrap());
    assert!(
        !store
            .trusted(&CatalogId::Public, "ornek-belediye")
            .await
            .unwrap()
    );
    // A role in one catalog is not in another; the public catalog is everyone's, with no role.
    let public = access(CatalogId::Public).await;
    assert!(public.member && public.roles.is_empty());
    // Taken back: outside again; an organisation's member is in it without a role.
    assert!(
        granted::revoke(owner, &place, "ayse", Role::Publisher)
            .await
            .unwrap()
    );
    assert!(
        !granted::revoke(owner, &place, "ayse", Role::Publisher)
            .await
            .unwrap()
    );
    assert!(!access(buro.clone()).await.member);
    kentos_application::admin::set_membership(owner, "buro", "ayse", TenantRole::Editor, false)
        .await
        .unwrap();
    let member = access(buro.clone()).await;
    assert!(member.member && member.roles.is_empty(), "{member:?}");
    // The catalog's services are in every catalog, one's own too.
    granted::grant(owner, &Where::Every, "ayse", Role::Scanner)
        .await
        .unwrap();
    let own = access(CatalogId::Personal(Uuid::now_v7().to_string())).await;
    assert!(own.member && own.roles.contains(&Role::Scanner));
    // What is not given: one's own catalog, an account or an organisation that is not, a publisher that is not.
    for (result, words) in [
        (
            Where::parse(&format!("personal:{ayse}")).map(|_| ()),
            "kendi kataloğu",
        ),
        (
            granted::grant(owner, &place, "kimse", Role::Reviewer).await,
            "kimse",
        ),
        (
            granted::grant(
                owner,
                &Where::parse("tenant:yok").unwrap(),
                "ayse",
                Role::Reviewer,
            )
            .await,
            "yok",
        ),
        (
            granted::trust(owner, &place, "yok-yayimci", true).await,
            "yayımcısı yok",
        ),
        (
            granted::trust(owner, &Where::Every, "ornek-belediye", true).await,
            "public ya da tenant",
        ),
        (
            granted::publisher(owner, "Büyük Harf", Some("X")).await,
            "olmaz",
        ),
        (granted::role("approver").map(|_| ()), "publisher, reviewer"),
    ] {
        let why = result.unwrap_err();
        assert!(why.contains(words), "{why}");
    }
    let lines: Vec<String> = granted::listing(owner)
        .await
        .unwrap()
        .into_iter()
        .map(|l| format!("{}\t{}", l.what, l.text))
        .collect();
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("rol\tevery\tayse (Ayşe)\tscanner")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("yayımcı\tornek-belediye\tÖrnek Belediyesi\tayse")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l == "güvenilen\ttenant:buro\tornek-belediye"),
        "{lines:?}"
    );
    db.close().await;
}

/// The catalog as a running server opens it: the persistent store, the server's own sessions. A person signed in in
/// the web app uploads, sends and takes a release into review there, but approving answers that a decision needs a
/// fresh sign-in, and nothing moves (docs/adr/8059 §13).
#[tokio::test]
async fn on_a_running_server_a_person_in_the_window_takes_no_decision() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let ayse = kentos_application::admin::create_local_user(
        &db.owner,
        "ayse",
        "Ayşe",
        None,
        "dogru-parola-1",
    )
    .await
    .unwrap()
    .to_string();
    let bench = Bench::database(&db);
    bench.grant(Where::Every, "tarayici", Role::Scanner).await;
    let app = router(AppState {
        config: Arc::new(config()),
        database: Some(db.app.clone()),
        oidc: None,
        hub: crate::hub::Hub::default(),
        logins: Default::default(),
        signups: Default::default(),
        notices: Default::default(),
        blobs: Blobs::new(
            std::env::temp_dir().join(format!("kentos-api-blobs-{}", Uuid::now_v7())),
        ),
        proxy: None,
        extension_catalog: Some(Arc::new(Catalog::new(bench.store(), Accounts::Sessions))),
    });
    let cookie = signed_in(&app, "ayse").await;
    let mine = format!("/v1/extension-catalogs/personal:{ayse}/releases");
    let upload = Request::post(&mine)
        .header(header::COOKIE, &cookie)
        .header("x-kentos-client", "web")
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .body(Body::from(release("1.0.0")))
        .unwrap();
    let (status, view) = call(&app, upload).await;
    assert_eq!(status, StatusCode::CREATED, "{view}");
    // The catalog's scanner reports every gate clean (its own call, not through a session).
    let key = ReleaseKey {
        catalog: CatalogId::Personal(ayse.clone()),
        extension: "tr.ornek.altlik".into(),
        version: "1.0.0".into(),
    };
    let scanner = Account {
        id: id("tarayici"),
        via: Channel::Program,
        fresh_sign_in: false,
    };
    let service = Catalog::new(bench.store(), Accounts::Named);
    for gate in GateKind::ALL {
        let report = Move::Scan {
            gate,
            content: package_of(&view),
            outcome: ScanOutcome::Passed,
        };
        service.act(&key, &scanner, &report).await.unwrap();
    }
    let at = uri(&mine, "1.0.0");
    let mv = |body: Value| json_req("POST", &format!("{at}/moves"), &cookie, body);
    for action in ["submit", "startReview"] {
        let (status, view) = call(&app, mv(json!({ "action": action }))).await;
        assert_eq!(status, StatusCode::OK, "{action}: {view}");
    }
    let before = bench.stored(&key).await.unwrap();
    let (status, body) = call(&app, mv(json!({ "action": "approve" }))).await;
    assert_eq!(
        (status, code(&body), body["retryable"].as_bool()),
        (
            StatusCode::FORBIDDEN,
            "fresh_sign_in_required".to_owned(),
            Some(false)
        ),
        "{body}"
    );
    assert_eq!(bench.stored(&key).await.unwrap(), before, "nothing moved");
    db.close().await;
}
