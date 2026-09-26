//! The project catalog against a real PostgreSQL/PostGIS (docs/adr/0028,
//! TODOS.md CLOUD-02..04, CLOUD-12): catalog metadata checked, versioned and
//! audited; the views of one person's catalog searched, sorted, paged and
//! counted after the access check (never a project the person may not see);
//! recents and favourites that are a person's own; details with the exact
//! extent of the stored geometry.

mod common;

use common::{
    a_point, account, catalog_envelope, changed, envelope, member, new_project, open, personal,
    revoke, run, share,
};
use kentos_application::commands::{self, CatalogPolicy};
use kentos_application::identity::Actor;
use kentos_application::listing::{self, CatalogQuery};
use kentos_application::{AppError, admin, changes, events, projects};
use kentos_contracts::{
    CatalogSort, CatalogView, Entity, FeatureChange, GrantRole, PROJECT_FAVORITE,
    PROJECT_METADATA_CHANGED, PROJECT_METADATA_UPDATE, PROJECT_RENAME, PROJECT_TRASH,
    ProjectChanges, ProjectPage, ProjectPatch, ProjectType, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

fn query(view: CatalogView) -> CatalogQuery {
    CatalogQuery {
        view,
        tenant: None,
        search: String::new(),
        project_type: None,
        sort: None,
        limit: None,
        after: None,
    }
}

async fn page(db: &TestDb, who: &Actor, q: &CatalogQuery) -> Result<ProjectPage, AppError> {
    listing::page(&db.app, who, q, CatalogPolicy::default().trash_retention).await
}

async fn names(db: &TestDb, who: &Actor, q: CatalogQuery) -> Vec<String> {
    page(db, who, &q)
        .await
        .unwrap()
        .projects
        .into_iter()
        .map(|p| p.name)
        .collect()
}

#[tokio::test]
async fn catalog_metadata_is_checked_versioned_and_audited() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;

    // A new project is the general type, with no description or tags, at catalog version 1.
    let first = changed(
        run(&db, &owner, PROJECT_METADATA_UPDATE, json!({}))
            .await
            .unwrap(),
    );
    assert!(!first.changed);
    assert_eq!(
        (
            first.project.project_type,
            first.project.description.as_str(),
            first.project.catalog_version.as_str()
        ),
        (ProjectType::Cad, "", "1")
    );
    // An editor may not change it (project.edit); bad values are refused before anything is written.
    let edits = open(&db, &editor, project).await;
    assert!(matches!(
        run(&db, &edits, PROJECT_METADATA_UPDATE, json!({ "tags": ["a"] })).await,
        Err(AppError::Forbidden(m)) if m.contains("project.edit")
    ));
    for bad in [
        json!({ "projectType": "imar" }),
        json!({ "tags": ["x".repeat(33)] }),
        json!({ "description": "d".repeat(2001) }),
        json!({ "name": "  " }),
    ] {
        assert!(
            matches!(
                run(&db, &owner, PROJECT_METADATA_UPDATE, bad.clone()).await,
                Err(AppError::Invalid { .. })
            ),
            "{bad}"
        );
    }

    // The owner changes type, description and tags: one version up, tags normalized, no new name.
    let done = changed(
        run(
            &db,
            &owner,
            PROJECT_METADATA_UPDATE,
            json!({ "projectType": "landReadjustment", "description": "  18. madde  ", "tags": [" Kadıköy", "KADIKÖY", "DOP  %40"] }),
        )
        .await
        .unwrap(),
    );
    assert!(done.changed && done.event_seq.is_some());
    assert_eq!(done.project.project_type, ProjectType::LandReadjustment);
    assert_eq!(done.project.description, "18. madde");
    assert_eq!(done.project.tags, ["Kadıköy", "DOP %40"]);
    assert_eq!(done.project.catalog_version, "2");
    let info = projects::info(&db.app, &owner).await.unwrap();
    assert_eq!(
        info.meta_version, "1",
        "no new name: the drawing's metadata is as it was"
    );
    // Open editors hear a catalog event; `meta` only when the name changed.
    let log = events::after(&db.app, &edits, 0, 50).await.unwrap();
    let last = log.events.last().unwrap();
    assert_eq!(
        (last.kind.as_str(), last.meta),
        (PROJECT_METADATA_CHANGED, false)
    );

    // An edit prepared from an older version is refused, nothing written; from the current one it goes.
    let stale = commands::run(
        &db.app,
        &common::blobs(),
        &CatalogPolicy::default(),
        &owner,
        catalog_envelope(
            &owner,
            PROJECT_METADATA_UPDATE,
            json!({ "projectType": "gis" }),
            &[("@catalog", "1")],
        ),
    )
    .await;
    assert!(
        matches!(&stale, Err(AppError::Conflict { conflicts, .. }) if conflicts[0].id == "@catalog" && conflicts[0].actual.as_deref() == Some("2")),
        "{stale:?}"
    );
    let renamed = changed(
        commands::run(
            &db.app,
            &common::blobs(),
            &CatalogPolicy::default(),
            &owner,
            catalog_envelope(
                &owner,
                PROJECT_RENAME,
                json!({ "name": " Ada 101 (revize) " }),
                &[("@catalog", "2")],
            ),
        )
        .await
        .unwrap(),
    );
    assert_eq!(
        (
            renamed.project.name.as_str(),
            renamed.project.catalog_version.as_str()
        ),
        ("Ada 101 (revize)", "3")
    );
    // A new name is the drawing's too: its meta version rises and the event says `meta`.
    let info = projects::info(&db.app, &owner).await.unwrap();
    assert_eq!(
        (info.name.as_str(), info.meta_version.as_str()),
        ("Ada 101 (revize)", "2")
    );
    let last = events::after(&db.app, &edits, 0, 50)
        .await
        .unwrap()
        .events
        .pop()
        .unwrap();
    assert!(last.meta);
    // A rename through project.changes (the open drawing's autosave) raises the catalog version as well.
    changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                features: vec![],
                project: Some(ProjectPatch {
                    name: Some("Ada 101 (son)".into()),
                    ..Default::default()
                }),
            },
            &[("@project", "2")],
        ),
    )
    .await
    .unwrap();
    let now = changed(
        run(&db, &owner, PROJECT_METADATA_UPDATE, json!({}))
            .await
            .unwrap(),
    );
    assert_eq!(now.project.catalog_version, "4");

    // Every change is audited with what changed (not the description's text).
    let audit: Vec<(String, serde_json::Value)> = sqlx::query_as(
        "select action, detail from kentos.audit_event where project_id = $1 and action in ('project.metadata.update', 'project.rename', 'project.changes') order by id",
    )
    .bind(project)
    .fetch_all(&db.owner)
    .await
    .unwrap();
    assert_eq!(audit.len(), 3);
    assert_eq!(
        audit[0].1["projectType"],
        json!({ "from": "cad", "to": "landReadjustment" })
    );
    assert_eq!(audit[0].1["description"], json!(true));
    assert_eq!(
        audit[1].1["name"],
        json!({ "from": "Ada 101", "to": "Ada 101 (revize)" })
    );
    assert_eq!(
        audit[2].1["rename"],
        json!({ "from": "Ada 101 (revize)", "to": "Ada 101 (son)" })
    );
    db.close().await;
}

#[tokio::test]
async fn views_are_searched_sorted_paged_and_counted_after_the_access_check() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 8)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 3)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let other_pm = member(&db, "buro", "komsu", TenantRole::ProjectManager).await;
    let boss = member(&db, "buro", "mudur", TenantRole::Admin).await;
    let stranger = member(&db, "diger", "yabanci", TenantRole::Owner).await;
    let space = personal(&db, &pm.actor).await;

    // Hers: three in the organisation, two in her personal space; a neighbour's, shared with her;
    // the neighbour's own; another organisation's that matches every search.
    let mut made = Vec::new();
    for (who, name) in [
        (&pm, "İFRAZ Ada 12"),
        (&pm, "Çamlık yolu"),
        (&pm, "Zeytinlik"),
        (&space, "ifraz notlarım"),
        (&space, "Bahçe"),
    ] {
        made.push((name, new_project(&db, who, name).await));
    }
    let shared = new_project(&db, &other_pm, "Ifraz komşu").await;
    let unshared = new_project(&db, &other_pm, "İfraz gizli").await;
    let foreign = new_project(&db, &stranger, "İfraz yabancı").await;
    share(
        &db,
        &open(&db, &other_pm, shared).await,
        &pm.actor,
        GrantRole::Viewer,
    )
    .await;
    let zeytin = made[2].1;
    run(
        &db,
        &open(&db, &pm, zeytin).await,
        PROJECT_METADATA_UPDATE,
        json!({ "projectType": "gis", "tags": ["ifraz-sonrası"] }),
    )
    .await
    .unwrap();

    // Projelerim: her own, in both spaces; not the shared one or the neighbour's.
    let mine = names(&db, &pm.actor, query(CatalogView::Mine)).await;
    assert_eq!(mine.len(), 5);
    assert!(
        !mine
            .iter()
            .any(|n| n.contains("komşu") || n.contains("gizli"))
    );
    // Benimle paylaşılanlar: only the shared one.
    assert_eq!(
        names(&db, &pm.actor, query(CatalogView::Shared)).await,
        ["Ifraz komşu"]
    );
    // The organisation's view: what she may see there (hers and the shared one), not her personal space's.
    let org = names(
        &db,
        &pm.actor,
        CatalogQuery {
            tenant: Some(pm.tenant),
            ..query(CatalogView::Organization)
        },
    )
    .await;
    assert_eq!(org.len(), 4);
    // Its admin sees every one of the organisation's under the policy, never another tenant's.
    let all = names(
        &db,
        &boss.actor,
        CatalogQuery {
            tenant: Some(pm.tenant),
            ..query(CatalogView::Organization)
        },
    )
    .await;
    assert_eq!(all.len(), 5);
    assert!(!all.iter().any(|n| n.contains("yabancı")));
    // A tenant one is not in: 404, as for one that does not exist.
    for t in [stranger.tenant, Uuid::now_v7()] {
        let r = page(
            &db,
            &pm.actor,
            &CatalogQuery {
                tenant: Some(t),
                ..query(CatalogView::Organization)
            },
        )
        .await;
        assert!(matches!(r, Err(AppError::NotFound(_))), "{r:?}");
    }

    // Search: every word, in the name, description or tags, Turkish letters folded; counted after the access check.
    let found = page(
        &db,
        &pm.actor,
        &CatalogQuery {
            search: "ifraz".into(),
            ..query(CatalogView::Mine)
        },
    )
    .await
    .unwrap();
    let mut got: Vec<String> = found.projects.iter().map(|p| p.name.clone()).collect();
    got.sort();
    assert_eq!(got, ["Zeytinlik", "ifraz notlarım", "İFRAZ Ada 12"]);
    assert_eq!(found.total, 3);
    let admin_search = page(
        &db,
        &boss.actor,
        &CatalogQuery {
            tenant: Some(pm.tenant),
            search: "İfraz".into(),
            ..query(CatalogView::Organization)
        },
    )
    .await
    .unwrap();
    assert_eq!(
        admin_search.total, 4,
        "the neighbour's hidden one is there for the admin; the foreign one never"
    );
    let two_words = names(
        &db,
        &pm.actor,
        CatalogQuery {
            search: "ada IFRAZ".into(),
            ..query(CatalogView::Mine)
        },
    )
    .await;
    assert_eq!(two_words, ["İFRAZ Ada 12"]);
    let wildcards = page(
        &db,
        &pm.actor,
        &CatalogQuery {
            search: "%".into(),
            ..query(CatalogView::Mine)
        },
    )
    .await
    .unwrap();
    assert_eq!(wildcards.total, 0, "% is a letter, not a wildcard");
    // The type filter.
    assert_eq!(
        names(
            &db,
            &pm.actor,
            CatalogQuery {
                project_type: Some(ProjectType::Gis),
                ..query(CatalogView::Mine)
            }
        )
        .await,
        ["Zeytinlik"]
    );

    // Sorted by name across both spaces, Turkish letters folded; pages of two join into the whole list.
    let by_name = CatalogQuery {
        sort: Some(CatalogSort::Name),
        ..query(CatalogView::Mine)
    };
    let whole = names(&db, &pm.actor, by_name.clone()).await;
    assert_eq!(
        whole,
        [
            "Bahçe",
            "Çamlık yolu",
            "İFRAZ Ada 12",
            "ifraz notlarım",
            "Zeytinlik"
        ]
    );
    for sort in [
        CatalogSort::Name,
        CatalogSort::Updated,
        CatalogSort::Created,
    ] {
        let q = CatalogQuery {
            sort: Some(sort),
            limit: Some(2),
            ..query(CatalogView::Mine)
        };
        let full = names(
            &db,
            &pm.actor,
            CatalogQuery {
                limit: None,
                ..q.clone()
            },
        )
        .await;
        let mut paged = Vec::new();
        let mut after = None;
        loop {
            let p = page(
                &db,
                &pm.actor,
                &CatalogQuery {
                    after: after.clone(),
                    ..q.clone()
                },
            )
            .await
            .unwrap();
            assert_eq!(p.total, 5);
            assert!(p.projects.len() <= 2);
            paged.extend(p.projects.into_iter().map(|x| x.name));
            match p.next {
                Some(n) => after = Some(n),
                None => break,
            }
        }
        assert_eq!(paged, full, "{sort:?}");
    }
    // A cursor that is not one, and a sort a view does not have, are refused.
    assert!(matches!(
        page(
            &db,
            &pm.actor,
            &CatalogQuery {
                after: Some("dün".into()),
                ..query(CatalogView::Mine)
            }
        )
        .await,
        Err(AppError::Invalid { .. })
    ));
    assert!(matches!(
        page(
            &db,
            &pm.actor,
            &CatalogQuery {
                sort: Some(CatalogSort::Opened),
                ..query(CatalogView::Mine)
            }
        )
        .await,
        Err(AppError::Invalid { .. })
    ));
    let _ = (unshared, foreign);
    db.close().await;
}

#[tokio::test]
async fn recents_and_favourites_are_a_persons_own() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let a = new_project(&db, &pm, "Ada 1").await;
    let b = new_project(&db, &pm, "Ada 2").await;
    for p in [a, b] {
        share(
            &db,
            &open(&db, &pm, p).await,
            &editor.actor,
            GrantRole::Editor,
        )
        .await;
    }
    // Creating counts as using: both are her recent ones; the editor has opened none.
    assert_eq!(
        names(&db, &pm.actor, query(CatalogView::Recent))
            .await
            .len(),
        2
    );
    assert!(
        names(&db, &editor.actor, query(CatalogView::Recent))
            .await
            .is_empty()
    );
    // Opening (the first page of objects) makes it recent; the latest first.
    projects::features(&db.app, &open(&db, &editor, a).await, None, 10)
        .await
        .unwrap();
    projects::features(&db.app, &open(&db, &editor, b).await, None, 10)
        .await
        .unwrap();
    // A later page is not an opening.
    projects::features(&db.app, &open(&db, &editor, a).await, Some(Uuid::nil()), 10)
        .await
        .unwrap();
    let recent = page(&db, &editor.actor, &query(CatalogView::Recent))
        .await
        .unwrap();
    assert_eq!(
        recent
            .projects
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["Ada 2", "Ada 1"]
    );
    assert!(recent.projects[0].opened_at.is_some());

    // A favourite is one's own: the editor's does not show for the owner.
    let fav = changed(
        run(
            &db,
            &open(&db, &editor, a).await,
            PROJECT_FAVORITE,
            json!({ "favorite": true }),
        )
        .await
        .unwrap(),
    );
    assert!(fav.changed && fav.project.favorite && fav.event_seq.is_none());
    let again = changed(
        run(
            &db,
            &open(&db, &editor, a).await,
            PROJECT_FAVORITE,
            json!({ "favorite": true }),
        )
        .await
        .unwrap(),
    );
    assert!(!again.changed);
    assert_eq!(
        names(&db, &editor.actor, query(CatalogView::Favorites)).await,
        ["Ada 1"]
    );
    assert!(
        names(&db, &pm.actor, query(CatalogView::Favorites))
            .await
            .is_empty()
    );
    assert!(
        !listing::list(&db.app, &pm)
            .await
            .unwrap()
            .projects
            .iter()
            .any(|p| p.favorite)
    );
    // Not audited, not an event: nothing another person learns.
    let audited: i64 = sqlx::query_scalar(
        "select count(*) from kentos.audit_event where action = 'project.favorite'",
    )
    .fetch_one(&db.owner)
    .await
    .unwrap();
    assert_eq!(audited, 0);

    // Access taken away: the rows stay, the lists do not show the project any more.
    revoke(&db, &open(&db, &pm, a).await, &editor.actor).await;
    assert_eq!(
        names(&db, &editor.actor, query(CatalogView::Recent)).await,
        ["Ada 2"]
    );
    assert!(
        names(&db, &editor.actor, query(CatalogView::Favorites))
            .await
            .is_empty()
    );
    // A project in the trash leaves both too.
    run(&db, &open(&db, &pm, b).await, PROJECT_TRASH, json!({}))
        .await
        .unwrap();
    assert!(
        names(&db, &editor.actor, query(CatalogView::Recent))
            .await
            .is_empty()
    );
    let _ = account(&db, "bos").await;
    db.close().await;
}

#[tokio::test]
async fn details_give_the_counts_and_the_exact_extent() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 4)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm, "Ada 101").await;
    let owner = open(&db, &pm, project).await;
    // No objects: no extent.
    let empty = listing::details(&db.app, &owner).await.unwrap();
    assert_eq!(
        (
            empty.bounds,
            empty.feature_count.as_str(),
            empty.layer_count
        ),
        (None, "0", 1)
    );
    let line: Entity = serde_json::from_value(json!({ "kind": "line", "id": 1, "layerId": "cizim", "attrs": {},
        "a": { "x": 486512.345678901, "y": 4420210.12345678 }, "b": { "x": 486598.765432109, "y": 4420299.98765432 } }))
    .unwrap();
    changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                features: vec![FeatureChange::Create {
                    id: Uuid::now_v7().to_string(),
                    entity: line,
                }],
                project: None,
            },
            &[],
        ),
    )
    .await
    .unwrap();
    changes::commit(
        &db.app,
        &owner,
        envelope(&pm, project, a_point(486600.5), &[]),
    )
    .await
    .unwrap();
    let d = listing::details(&db.app, &owner).await.unwrap();
    let b = d.bounds.expect("an extent");
    // Exactly the stored coordinates (the point's y is 4420210.0): no rounding to a float box on the way.
    assert_eq!(
        (b.min_x, b.min_y, b.max_x, b.max_y),
        (486512.345678901, 4420210.0, 486600.5, 4420299.98765432)
    );
    assert_eq!(d.feature_count, "2");
    assert_eq!(d.project.name, "Ada 101");
    // A project in the trash has no details to give (410).
    kentos_application::lifecycle::delete(&db.app, &owner, None)
        .await
        .unwrap();
    let owner = open(&db, &pm, project).await;
    assert!(matches!(
        listing::details(&db.app, &owner).await,
        Err(AppError::Deleted(_))
    ));
    db.close().await;
}
