//! Row-level security as projects' (docs/adr/8059 §13): an organisation's releases and their moves are seen and
//! written only in its scope, an account's own only in its, and through the API a member of one organisation is not
//! told of another's catalog.

use super::*;

/// The catalogs whose releases the server's role sees in `scope`, but the public one's, and how many of their moves.
async fn seen_in(db: &TestDb, scope: Scope) -> (Vec<(String, String)>, i64) {
    let mut tx = db.app.pool.begin().await.unwrap();
    rescope(&mut tx, scope).await.unwrap();
    let rows: Vec<(String, String)> = sqlx::query_as(
        "select catalog_kind, catalog_ref from kentos.extension_release
          where catalog_kind <> 'public' order by 1, 2",
    )
    .fetch_all(&mut *tx)
    .await
    .unwrap();
    let moves: i64 = sqlx::query_scalar(
        "select count(*) from kentos.extension_release_move m
           join kentos.extension_release r on r.id = m.release_id where r.catalog_kind <> 'public'",
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    (rows, moves)
}

#[tokio::test]
async fn an_organisation_s_releases_are_not_seen_in_another_s_scope() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let bench = Bench::database(&db);
    let app = catalog(&bench).await;
    for slug in ["buro", "diger"] {
        let place = || Where::In(CatalogId::Tenant(slug.into()));
        bench.grant(place(), "ayse", Role::Publisher).await;
        let base = format!("/v1/extension-catalogs/tenant:{slug}/releases");
        let (status, view) = upload(&app, &base, "ayse", release("1.0.0")).await;
        assert_eq!(status, StatusCode::CREATED, "{view}");
    }
    upload(&app, &personal("can"), "can", release("1.0.0")).await;
    let (buro, diger) = (tenant_of(&db, "buro").await, tenant_of(&db, "diger").await);
    let scope = |tenant: Option<Uuid>, user: Option<&str>| Scope {
        tenant,
        user: user.map(|n| Uuid::parse_str(&id(n)).unwrap()),
        project: None,
    };
    let seen = |s: Scope| seen_in(&db, s);
    let tenant = |id: Uuid| ("tenant".to_owned(), id.to_string());
    assert_eq!(seen(scope(Some(buro), None)).await, (vec![tenant(buro)], 1));
    assert_eq!(
        seen(scope(Some(diger), None)).await,
        (vec![tenant(diger)], 1)
    );
    assert_eq!(
        seen(scope(None, Some("can"))).await,
        (vec![("personal".to_owned(), id("can"))], 1)
    );
    assert_eq!(seen(scope(None, Some("ayse"))).await, (vec![], 0));
    assert_eq!(seen(Scope::default()).await, (vec![], 0));
    // Nor written there: no row of another organisation changes, none is put in its catalog.
    let touch =
        format!("update kentos.extension_release set state = state where catalog_ref = '{diger}'");
    assert_eq!(as_server(&db, scope(Some(buro), None), &touch).await, Ok(0));
    let copy = |into: Uuid| {
        format!(
            "insert into kentos.extension_release (id, catalog_kind, catalog_ref, extension_id, version, publisher,
                author, state, package_digest, capabilities, record, moves)
             select '{}', catalog_kind, '{into}', extension_id, '2.0.0', publisher, author, state, package_digest,
                capabilities, jsonb_set(record, '{{version}}', '\"2.0.0\"'), moves
               from kentos.extension_release where catalog_ref = '{buro}'",
            Uuid::now_v7()
        )
    };
    let refused = as_server(&db, scope(Some(buro), None), &copy(diger))
        .await
        .unwrap_err();
    assert!(refused.contains("row-level security"), "{refused}");
    assert_eq!(
        as_server(&db, scope(Some(diger), None), &copy(diger)).await,
        Ok(0),
        "Büro's row is not there to copy in Diğer's scope"
    );
    // Through the API: a member of one organisation is in its catalog (where a draft is not shown to it) and is not
    // told of the other's.
    bench.admit(CatalogId::Tenant("buro".into()), "deniz").await;
    let list = |slug: &str| format!("/v1/extension-catalogs/tenant:{slug}/releases");
    let (status, body) = get(&app, Via::Web, &list("buro"), "deniz").await;
    assert_eq!((status, body["total"].as_u64()), (StatusCode::OK, Some(0)));
    let (status, hidden) = get(&app, Via::Web, &list("diger"), "deniz").await;
    let (_, unknown) = get(&app, Via::Web, &list("yok"), "deniz").await;
    assert_eq!(
        (status, code(&hidden)),
        (StatusCode::NOT_FOUND, "not_found".to_owned())
    );
    assert_eq!(hidden["message"], unknown["message"], "word for word");
    db.close().await;
}
