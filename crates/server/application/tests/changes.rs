//! The edit protocol against a real PostgreSQL/PostGIS: lossless storage of
//! every object kind, versions and conflicts, idempotency, locked layers,
//! rights, tenant isolation and the event log.

use std::collections::BTreeMap;

use kentos_application::access::{self, ProjectAccess};
use kentos_application::identity::{self};
use kentos_application::tenancy::{self, Access};
use kentos_application::{AppError, admin, changes, events, listing, projects, sharing};
use kentos_contracts::{
    CommandEnvelope, ConflictReason, DocumentSnapshotV1, Entity, FeatureChange, FeatureOp,
    GrantRole, PROJECT_CHANGES, PROJECT_SHARE, ProjectChanges, ProjectCreate, ProjectPatch,
    SignInMethod, TenantRole,
};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

const PASSWORD: &str = "dogru-parola-123";
const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

fn sample() -> DocumentSnapshotV1 {
    DocumentSnapshotV1::from_json(SAMPLE).unwrap()
}

async fn member(db: &TestDb, tenant: &str, login: &str, role: TenantRole) -> Access {
    let user = admin::create_local_user(&db.owner, login, login, None, PASSWORD)
        .await
        .unwrap();
    admin::set_membership(&db.owner, tenant, login, role, true)
        .await
        .unwrap();
    let actor = identity::actor_of(&db.app, user, SignInMethod::Local)
        .await
        .unwrap()
        .unwrap();
    let tenant_id: Uuid = sqlx::query_scalar("select id from kentos.tenant where slug = $1")
        .bind(tenant)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    tenancy::access(&db.app, &actor, tenant_id).await.unwrap()
}

fn unlocked(nodes: &[kentos_contracts::LayerNode]) -> Vec<kentos_contracts::LayerNode> {
    nodes
        .iter()
        .map(|n| kentos_contracts::LayerNode {
            locked: false,
            children: unlocked(&n.children),
            ..n.clone()
        })
        .collect()
}

/// A new project with the sample's tree, every layer unlocked (an upload restores the locks afterwards).
async fn new_project(db: &TestDb, who: &Access) -> Uuid {
    let s = sample();
    let input = ProjectCreate {
        name: "Örnek".into(),
        settings: s.settings,
        origin: s.origin,
        home_view: s.home_view,
        layers: unlocked(&s.layers),
        active_layer: s.active_layer,
        styles: s.styles,
        description: None,
        project_type: None,
        tags: None,
    };
    Uuid::parse_str(
        &projects::create(&db.app, who, input, None)
            .await
            .unwrap()
            .id,
    )
    .unwrap()
}

fn envelope(
    who: &Access,
    project: Uuid,
    input: ProjectChanges,
    expected: &[(&str, &str)],
) -> CommandEnvelope {
    CommandEnvelope {
        command_name: PROJECT_CHANGES.into(),
        version: 1,
        tenant_id: who.tenant.to_string(),
        project_id: project.to_string(),
        request_id: format!("istek-{}", Uuid::new_v4()),
        idempotency_key: Uuid::new_v4().to_string(),
        expected_versions: expected
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
        input: serde_json::to_value(input).unwrap(),
    }
}

/// `who`'s access to a project of their tenant, as a request would get it.
async fn open(db: &TestDb, who: &Access, project: Uuid) -> ProjectAccess {
    access::project(&db.app, &who.actor, who.tenant, project)
        .await
        .unwrap()
}

/// The project's owner (`by`) shares it with each of `with` in this role.
async fn share_with(db: &TestDb, by: &Access, project: Uuid, with: &[&Access], role: GrantRole) {
    let owner = open(db, by, project).await;
    for who in with {
        let mut e = envelope(by, project, changes_of(vec![]), &[]);
        e.command_name = PROJECT_SHARE.into();
        e.input = serde_json::json!({ "userId": who.actor.user_id.to_string(), "role": role });
        sharing::share(&db.app, &owner, e).await.unwrap();
    }
}

fn with_id(mut e: Entity, id: u32) -> Entity {
    let base = match &mut e {
        Entity::Point(x) => &mut x.base,
        Entity::Line(x) => &mut x.base,
        Entity::Polyline(x) | Entity::Polygon(x) => &mut x.base,
        Entity::Circle(x) => &mut x.base,
        Entity::Arc(x) => &mut x.base,
        Entity::Ellipse(x) => &mut x.base,
        Entity::Spline(x) => &mut x.base,
        Entity::Xline(x) | Entity::Ray(x) => &mut x.base,
        Entity::Text(x) => &mut x.base,
        Entity::Dimension(x) => &mut x.base,
        Entity::Hatch(x) => &mut x.base,
    };
    base.id = id;
    e
}

fn changes_of(list: Vec<FeatureChange>) -> ProjectChanges {
    ProjectChanges {
        features: list,
        project: None,
    }
}

#[tokio::test]
async fn every_object_kind_comes_back_exactly() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::Editor).await;
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm).await;
    share_with(&db, &pm, project, &[&ayse], GrantRole::Editor).await;
    let (ayse_p, pm_p) = (
        open(&db, &ayse, project).await,
        open(&db, &pm, project).await,
    );
    let entities = sample().entities;
    let kinds: std::collections::BTreeSet<_> = entities
        .iter()
        .map(|e| {
            serde_json::to_value(e).unwrap()["kind"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(kinds.len(), 13, "the fixture holds every kind");
    let ids: Vec<Uuid> = entities.iter().map(|_| Uuid::new_v4()).collect();
    let list = entities
        .iter()
        .zip(&ids)
        .map(|(e, id)| FeatureChange::Create {
            id: id.to_string(),
            entity: e.clone(),
        })
        .collect();
    let result = changes::commit(
        &db.app,
        &ayse_p,
        envelope(&ayse, project, changes_of(list), &[]),
    )
    .await
    .unwrap();
    assert_eq!(result.versions.len(), entities.len());
    assert!(result.versions.values().all(|v| v == "1"));
    // The upload ends by restoring the drawing's own layer tree, locks included.
    let restore = ProjectChanges {
        features: vec![],
        project: Some(ProjectPatch {
            layers: Some(sample().layers),
            ..Default::default()
        }),
    };
    changes::commit(
        &db.app,
        &pm_p,
        envelope(&pm, project, restore, &[("@project", "1")]),
    )
    .await
    .unwrap();

    // Read back in pages of 5; every object equals what was sent, down to the bit.
    let mut back = BTreeMap::new();
    let mut after = None;
    loop {
        let page = projects::features(&db.app, &ayse_p, after, 5)
            .await
            .unwrap();
        for f in &page.features {
            back.insert(Uuid::parse_str(&f.id).unwrap(), f.entity.clone());
        }
        match page.next {
            Some(n) => after = Some(Uuid::parse_str(&n).unwrap()),
            None => break,
        }
    }
    assert_eq!(back.len(), entities.len());
    for (e, id) in entities.iter().zip(&ids) {
        let original_id = serde_json::to_value(e).unwrap()["id"].as_u64().unwrap() as u32;
        let got = with_id(back[id].clone(), original_id);
        assert_eq!(
            serde_json::to_string(&got).unwrap(),
            serde_json::to_string(e).unwrap()
        );
    }
    // Source geometry is PostGIS geometry in the project's SRID; analytic kinds keep a definition.
    let rows: Vec<(String, String, Option<i32>)> =
        sqlx::query_as("select kind, source_kind, st_srid(geom) from kentos.feature order by kind")
            .fetch_all(&db.owner)
            .await
            .unwrap();
    for (kind, source, srid) in rows {
        let simple = matches!(kind.as_str(), "point" | "line");
        assert!(!simple || source == "geom", "{kind}");
        assert!(
            matches!(kind.as_str(), "xline" | "ray") || srid == Some(5256),
            "{kind}"
        );
    }
    let info = projects::info(&db.app, &ayse_p).await.unwrap();
    assert_eq!(
        (info.feature_count.as_str(), info.data_revision.as_str()),
        (entities.len().to_string().as_str(), "2")
    );
    assert_eq!(info.layers, sample().layers);
    // Now the locked layer's objects cannot be changed.
    let (on_locked, id) = entities
        .iter()
        .zip(&ids)
        .find(|(e, _)| serde_json::to_value(e).unwrap()["layerId"] == "bina")
        .unwrap();
    let change = FeatureChange::Update {
        id: id.to_string(),
        entity: on_locked.clone(),
    };
    assert!(matches!(
        changes::commit(
            &db.app,
            &ayse_p,
            envelope(
                &ayse,
                project,
                changes_of(vec![change]),
                &[(id.to_string().as_str(), "1")]
            )
        )
        .await,
        Err(AppError::Forbidden(_))
    ));
    db.close().await;
}

fn a_point(layer: &str, x: f64) -> Entity {
    serde_json::from_value(serde_json::json!({ "kind": "point", "id": 1, "layerId": layer, "attrs": { "Ad": "N1" }, "p": { "x": x, "y": 4420210.0 } })).unwrap()
}

#[tokio::test]
async fn versions_conflicts_and_idempotency() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let bora = member(&db, "buro", "bora", TenantRole::Editor).await;
    let project = new_project(&db, &ayse).await;
    share_with(&db, &ayse, project, &[&bora], GrantRole::Editor).await;
    let (ayse_p, bora_p) = (
        open(&db, &ayse, project).await,
        open(&db, &bora, project).await,
    );
    let layer = sample().active_layer;
    let id = Uuid::new_v4().to_string();

    let create = envelope(
        &ayse,
        project,
        changes_of(vec![FeatureChange::Create {
            id: id.clone(),
            entity: a_point(&layer, 486512.0),
        }]),
        &[],
    );
    let first = changes::commit(&db.app, &ayse_p, create.clone())
        .await
        .unwrap();
    // The same envelope again is answered from the log, without a second write.
    let again = changes::commit(&db.app, &ayse_p, create.clone())
        .await
        .unwrap();
    assert!(
        again.replayed
            && again.data_revision == first.data_revision
            && again.versions == first.versions
    );
    let mut reused = create.clone();
    reused.input = serde_json::to_value(changes_of(vec![])).unwrap();
    assert!(
        matches!(changes::commit(&db.app, &ayse_p, reused).await, Err(AppError::Invalid { message: m, .. }) if m.contains("idempotency"))
    );

    // Bora edits version 1; Ayşe's edit based on version 1 then conflicts and carries the server's copy.
    let update = |who: &Access, x: f64, v: &str| {
        envelope(
            who,
            project,
            changes_of(vec![FeatureChange::Update {
                id: id.clone(),
                entity: a_point(&layer, x),
            }]),
            &[(id.as_str(), v)],
        )
    };
    let by_bora = changes::commit(&db.app, &bora_p, update(&bora, 486513.0, "1"))
        .await
        .unwrap();
    assert_eq!(by_bora.versions[&id], "2");
    match changes::commit(&db.app, &ayse_p, update(&ayse, 486514.0, "1")).await {
        Err(AppError::Conflict {
            conflicts,
            revision,
            ..
        }) => {
            // The refusal says which revision the server is at (ARCH-07): Bora's commit made it.
            assert_eq!(
                revision.map(|r| r.to_string()),
                Some(by_bora.data_revision.clone())
            );
            assert_eq!(conflicts.len(), 1);
            assert_eq!(
                (conflicts[0].reason, conflicts[0].actual.as_deref()),
                (ConflictReason::Changed, Some("2"))
            );
            let Entity::Point(p) = &conflicts[0].current.as_ref().unwrap().entity else {
                panic!()
            };
            assert_eq!(p.p.x, 486513.0);
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
    // Nothing of the refused command was written.
    let now = projects::features_by_id(&db.app, &ayse_p, &[Uuid::parse_str(&id).unwrap()])
        .await
        .unwrap();
    assert_eq!(now[0].version, "2");
    // Missing expected version, a taken id, and a deleted object.
    let missing = envelope(
        &ayse,
        project,
        changes_of(vec![FeatureChange::Delete { id: id.clone() }]),
        &[],
    );
    assert!(matches!(
        changes::commit(&db.app, &ayse_p, missing).await,
        Err(AppError::Invalid { .. })
    ));
    let taken = envelope(
        &ayse,
        project,
        changes_of(vec![FeatureChange::Create {
            id: id.clone(),
            entity: a_point(&layer, 1.0),
        }]),
        &[],
    );
    assert!(
        matches!(changes::commit(&db.app, &ayse_p, taken).await, Err(AppError::Conflict { conflicts, .. }) if conflicts[0].reason == ConflictReason::Exists)
    );
    let delete = envelope(
        &ayse,
        project,
        changes_of(vec![FeatureChange::Delete { id: id.clone() }]),
        &[(id.as_str(), "2")],
    );
    let deleted = changes::commit(&db.app, &ayse_p, delete).await.unwrap();
    assert_eq!(deleted.deleted, vec![id.clone()]);
    assert!(
        matches!(changes::commit(&db.app, &bora_p, update(&bora, 2.0, "2")).await,
        Err(AppError::Conflict { conflicts, .. }) if conflicts[0].reason == ConflictReason::Deleted)
    );

    // The event log lists the commits in order, with who and which request (the share came first).
    let log = events::after(&db.app, &ayse_p, 0, 100).await.unwrap();
    let edits: Vec<_> = log
        .events
        .iter()
        .filter(|e| e.kind == PROJECT_CHANGES)
        .collect();
    let ops: Vec<FeatureOp> = edits.iter().map(|e| e.features[0].op).collect();
    assert_eq!(
        ops,
        vec![FeatureOp::Create, FeatureOp::Update, FeatureOp::Delete]
    );
    assert_eq!(
        edits[1].actor.as_deref(),
        Some(bora.actor.user_id.to_string().as_str())
    );
    let tail = events::after(&db.app, &ayse_p, edits[0].seq.parse().unwrap(), 100)
        .await
        .unwrap();
    assert_eq!(tail.events.len(), 2);
    assert_eq!(tail.next, log.next);
    db.close().await;
}

#[tokio::test]
async fn locked_layers_rights_and_tenants() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "editor", TenantRole::Editor).await;
    let viewer = member(&db, "buro", "izleyici", TenantRole::Viewer).await;
    let stranger = member(&db, "diger", "yabanci", TenantRole::Owner).await;
    let project = new_project(&db, &pm).await;
    share_with(&db, &pm, project, &[&editor], GrantRole::Editor).await;
    share_with(&db, &pm, project, &[&viewer], GrantRole::Viewer).await;
    let (pm_p, editor_p, viewer_p) = (
        open(&db, &pm, project).await,
        open(&db, &editor, project).await,
        open(&db, &viewer, project).await,
    );
    let info = projects::info(&db.app, &pm_p).await.unwrap();
    let layer = info.active_layer.clone();

    // Lock the layer through a metadata patch (needs project.edit and the metadata version).
    let mut layers = info.layers.clone();
    fn lock(nodes: &mut [kentos_contracts::LayerNode], id: &str) {
        for n in nodes {
            if n.id == id {
                n.locked = true;
            }
            lock(&mut n.children, id);
        }
    }
    lock(&mut layers, &layer);
    let patch = ProjectChanges {
        features: vec![],
        project: Some(ProjectPatch {
            layers: Some(layers),
            ..Default::default()
        }),
    };
    assert!(matches!(
        changes::commit(
            &db.app,
            &editor_p,
            envelope(&editor, project, patch.clone(), &[("@project", "1")])
        )
        .await,
        Err(AppError::Forbidden(_))
    ));
    assert!(
        matches!(changes::commit(&db.app, &pm_p, envelope(&pm, project, patch.clone(), &[("@project", "0")])).await,
        Err(AppError::Conflict { conflicts, .. }) if conflicts[0].reason == ConflictReason::Project)
    );
    let locked = changes::commit(
        &db.app,
        &pm_p,
        envelope(&pm, project, patch, &[("@project", "1")]),
    )
    .await
    .unwrap();
    assert_eq!(locked.meta_version, "2");
    let write = |who: &Access| {
        envelope(
            who,
            project,
            changes_of(vec![FeatureChange::Create {
                id: Uuid::new_v4().to_string(),
                entity: a_point(&layer, 5.0),
            }]),
            &[],
        )
    };
    assert!(
        matches!(changes::commit(&db.app, &editor_p, write(&editor)).await, Err(AppError::Forbidden(m)) if m.contains("kilitli"))
    );
    // A viewer may read but not write.
    assert!(projects::info(&db.app, &viewer_p).await.is_ok());
    assert!(matches!(
        changes::commit(&db.app, &viewer_p, write(&viewer)).await,
        Err(AppError::Forbidden(m)) if m.contains("feature.write")
    ));
    // Another tenant's member cannot see or touch the project, even knowing its id.
    for tenant in [pm.tenant, stranger.tenant] {
        assert!(matches!(
            access::project(&db.app, &stranger.actor, tenant, project).await,
            Err(AppError::NotFound(_))
        ));
    }
    assert!(
        listing::list(&db.app, &stranger)
            .await
            .unwrap()
            .projects
            .is_empty()
    );
    // A command naming another tenant than the caller's access is refused before anything runs.
    let mut wrong_tenant = write(&pm);
    wrong_tenant.tenant_id = stranger.tenant.to_string();
    assert!(matches!(
        changes::commit(&db.app, &pm_p, wrong_tenant).await,
        Err(AppError::Invalid { .. })
    ));
    // So is one naming another project than the one it is sent to.
    let mut wrong_project = write(&pm);
    wrong_project.project_id = Uuid::new_v4().to_string();
    assert!(matches!(
        changes::commit(&db.app, &pm_p, wrong_project).await,
        Err(AppError::Invalid { .. })
    ));
    db.close().await;
}

#[tokio::test]
async fn concurrent_writers_serialize_per_project() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::Editor).await;
    let bora = member(&db, "buro", "bora", TenantRole::Editor).await;
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let project = new_project(&db, &pm).await;
    share_with(&db, &pm, project, &[&ayse, &bora], GrantRole::Editor).await;
    let (ayse_p, bora_p) = (
        open(&db, &ayse, project).await,
        open(&db, &bora, project).await,
    );
    let layer = sample().active_layer;
    let id = Uuid::new_v4().to_string();
    changes::commit(
        &db.app,
        &ayse_p,
        envelope(
            &ayse,
            project,
            changes_of(vec![FeatureChange::Create {
                id: id.clone(),
                entity: a_point(&layer, 1.0),
            }]),
            &[],
        ),
    )
    .await
    .unwrap();
    // The same retry twice at once: one commit, one replay (never a conflict with itself).
    let retry = envelope(
        &ayse,
        project,
        changes_of(vec![FeatureChange::Update {
            id: id.clone(),
            entity: a_point(&layer, 2.0),
        }]),
        &[(id.as_str(), "1")],
    );
    let (a, b) = tokio::join!(
        changes::commit(&db.app, &ayse_p, retry.clone()),
        changes::commit(&db.app, &ayse_p, retry.clone())
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert!(a.replayed != b.replayed && a.versions == b.versions);
    // Two people from the same base version at once: one wins, the other gets a conflict.
    let from = |who: &Access, x: f64| {
        envelope(
            who,
            project,
            changes_of(vec![FeatureChange::Update {
                id: id.clone(),
                entity: a_point(&layer, x),
            }]),
            &[(id.as_str(), "2")],
        )
    };
    let (a, b) = tokio::join!(
        changes::commit(&db.app, &ayse_p, from(&ayse, 3.0)),
        changes::commit(&db.app, &bora_p, from(&bora, 4.0))
    );
    assert!(a.is_ok() != b.is_ok());
    assert!(matches!(
        a.err().or(b.err()),
        Some(AppError::Conflict { .. })
    ));
    let info = projects::info(&db.app, &ayse_p).await.unwrap();
    assert_eq!(info.data_revision, "3");
    db.close().await;
}
