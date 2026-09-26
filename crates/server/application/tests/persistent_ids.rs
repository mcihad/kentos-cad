//! Persistent object ids in the cloud (docs/adr/0014 slice 3, docs/adr/0026)
//! against a real PostgreSQL/PostGIS. The client's persistent id is the
//! object's id: the same ids live side by side in two projects (the same v1
//! file uploaded twice); an id deleted and created again (an undone
//! deletion, or "keep mine" over someone else's deletion) comes back above
//! every version it had, so an edit based on a version from before is a
//! conflict, never a silent overwrite.

mod common;

use common::{envelope, member, open, share};
use kentos_application::access::ProjectAccess;
use kentos_application::tenancy::Access;
use kentos_application::{AppError, admin, changes, events, projects};
use kentos_contracts::{
    CommitResult, ConflictReason, DocumentSnapshotV1, Entity, FeatureChange, FeatureOp, GrantRole,
    LayerNode, PROJECT_CHANGES, ProjectChanges, ProjectCreate, TenantRole, v1_identities,
};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

fn point(x: f64) -> Entity {
    serde_json::from_value(serde_json::json!({ "kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": { "x": x, "y": 4420210.0 } })).unwrap()
}

fn one(change: FeatureChange) -> ProjectChanges {
    ProjectChanges {
        features: vec![change],
        project: None,
    }
}

async fn commit(
    db: &TestDb,
    who: &Access,
    access: &ProjectAccess,
    change: FeatureChange,
    expected: &[(&str, &str)],
) -> Result<CommitResult, AppError> {
    changes::commit(
        &db.app,
        access,
        envelope(who, access.project, one(change), expected),
    )
    .await
}

/// The server's version of an object, if it has the object.
async fn version_of(db: &TestDb, access: &ProjectAccess, id: &str) -> Option<String> {
    projects::features_by_id(&db.app, access, &[Uuid::parse_str(id).unwrap()])
        .await
        .unwrap()
        .first()
        .map(|f| f.version.clone())
}

/// Every object id of the project, in the server's order, page by page.
async fn all_ids(db: &TestDb, access: &ProjectAccess) -> Vec<String> {
    let mut out = Vec::new();
    let mut after = None;
    loop {
        let page = projects::features(&db.app, access, after, 7).await.unwrap();
        out.extend(page.features.iter().map(|f| f.id.clone()));
        match page.next {
            Some(n) => after = Some(Uuid::parse_str(&n).unwrap()),
            None => return out,
        }
    }
}

fn unlocked(nodes: &[LayerNode]) -> Vec<LayerNode> {
    nodes
        .iter()
        .map(|n| LayerNode {
            locked: false,
            children: unlocked(&n.children),
            ..n.clone()
        })
        .collect()
}

/// A project with the sample file's layer tree, unlocked (as an upload makes it before its objects).
async fn sample_project(db: &TestDb, who: &Access, name: &str) -> Uuid {
    let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    let input = ProjectCreate {
        name: name.into(),
        settings: s.settings,
        origin: s.origin,
        home_view: s.home_view,
        layers: unlocked(&s.layers),
        active_layer: s.active_layer,
        styles: s.styles,
        description: None,
        project_type: None,
        tags: None,
        storage: None,
    };
    Uuid::parse_str(
        &projects::create(&db.app, who, input, None)
            .await
            .unwrap()
            .id,
    )
    .unwrap()
}

#[tokio::test]
async fn a_deleted_id_comes_back_above_every_version_it_had() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let bora = member(&db, "buro", "bora", TenantRole::Editor).await;
    let project = common::new_project(&db, &ayse, "Geri alma").await;
    let ayse_p = open(&db, &ayse, project).await;
    share(&db, &ayse_p, &bora.actor, GrantRole::Editor).await;
    let bora_p = open(&db, &bora, project).await;
    // The id the client gave the object when it was drawn (a UUIDv7, as the web makes them).
    let id = Uuid::now_v7().to_string();
    let create = |x: f64| FeatureChange::Create {
        id: id.clone(),
        entity: point(x),
    };
    let update = |x: f64| FeatureChange::Update {
        id: id.clone(),
        entity: point(x),
    };
    let delete = || FeatureChange::Delete { id: id.clone() };

    let made = commit(&db, &ayse, &ayse_p, create(1.0), &[]).await.unwrap();
    assert_eq!(made.versions[&id], "1");
    // Bora edits it (version 2); he keeps that version while Ayşe deletes it and brings it back.
    let by_bora = commit(&db, &bora, &bora_p, update(2.0), &[(&id, "1")])
        .await
        .unwrap();
    assert_eq!(by_bora.versions[&id], "2");
    commit(&db, &ayse, &ayse_p, delete(), &[(&id, "2")])
        .await
        .unwrap();
    assert_eq!(version_of(&db, &ayse_p, &id).await, None);
    // Her undo creates it again under the same id: its version is that commit's data revision.
    let undo = envelope(&ayse, project, one(create(2.0)), &[]);
    let back = changes::commit(&db.app, &ayse_p, undo.clone())
        .await
        .unwrap();
    assert_eq!(
        (back.versions[&id].as_str(), back.data_revision.as_str()),
        ("4", "4")
    );
    // Its answer lost, the same command again: answered from the log, the same version.
    let replay = changes::commit(&db.app, &ayse_p, undo).await.unwrap();
    assert!(replay.replayed && replay.versions == back.versions);
    // She edits it once more: with versions starting again at 1 it would be at 2, the version Bora holds.
    let edited = commit(&db, &ayse, &ayse_p, update(3.0), &[(&id, "4")])
        .await
        .unwrap();
    assert_eq!(edited.versions[&id], "5");
    // Bora's edit and deletion based on version 2 are conflicts with her current copy, not overwrites.
    for stale in [update(9.0), delete()] {
        match commit(&db, &bora, &bora_p, stale, &[(&id, "2")]).await {
            Err(AppError::Conflict { conflicts, .. }) => {
                assert_eq!(
                    (conflicts[0].reason, conflicts[0].actual.as_deref()),
                    (ConflictReason::Changed, Some("5"))
                );
                let Entity::Point(p) = &conflicts[0].current.as_ref().unwrap().entity else {
                    panic!("a point")
                };
                assert_eq!(p.p.x, 3.0);
            }
            other => panic!("expected a conflict, got {other:?}"),
        }
    }
    assert_eq!(version_of(&db, &ayse_p, &id).await.as_deref(), Some("5"));
    // Creating it while it exists is refused as before ("exists"), with the server's copy.
    assert!(matches!(
        commit(&db, &bora, &bora_p, create(8.0), &[]).await,
        Err(AppError::Conflict { conflicts, .. }) if conflicts[0].reason == ConflictReason::Exists && conflicts[0].current.is_some()
    ));
    // Deleted and brought back again: above 5 once more.
    commit(&db, &ayse, &ayse_p, delete(), &[(&id, "5")])
        .await
        .unwrap();
    let again = commit(&db, &ayse, &ayse_p, create(4.0), &[]).await.unwrap();
    assert_eq!(again.versions[&id], "7");
    // One object under the id; the log tells the whole story with its versions.
    assert_eq!(all_ids(&db, &ayse_p).await, vec![id.clone()]);
    let log = events::after(&db.app, &ayse_p, 0, 100).await.unwrap();
    let story: Vec<(FeatureOp, Option<&str>)> = log
        .events
        .iter()
        .filter(|e| e.kind == PROJECT_CHANGES)
        .map(|e| (e.features[0].op, e.features[0].version.as_deref()))
        .collect();
    assert_eq!(
        story,
        vec![
            (FeatureOp::Create, Some("1")),
            (FeatureOp::Update, Some("2")),
            (FeatureOp::Delete, None),
            (FeatureOp::Create, Some("4")),
            (FeatureOp::Update, Some("5")),
            (FeatureOp::Delete, None),
            (FeatureOp::Create, Some("7")),
        ]
    );
    db.close().await;
}

#[tokio::test]
async fn the_same_file_uploaded_twice_gives_two_projects_with_the_same_ids() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let sample = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    // The ids a v1 file's content derives (ADR 0014): the same file gives the same ids every time.
    let ids = v1_identities(SAMPLE).unwrap();
    assert_eq!(ids.entities.len(), sample.entities.len());
    let mut uploads = Vec::new();
    for name in ["Örnek", "Örnek (yeniden)"] {
        let project = sample_project(&db, &ayse, name).await;
        let access = open(&db, &ayse, project).await;
        let features = sample
            .entities
            .iter()
            .zip(&ids.entities)
            .map(|(e, i)| FeatureChange::Create {
                id: i.uid.clone(),
                entity: e.clone(),
            })
            .collect();
        let sent = envelope(
            &ayse,
            project,
            ProjectChanges {
                features,
                project: None,
            },
            &[],
        );
        let result = changes::commit(&db.app, &access, sent.clone())
            .await
            .unwrap();
        assert!(result.versions.values().all(|v| v == "1"));
        uploads.push((access, sent));
    }
    let mut expected: Vec<String> = ids.entities.iter().map(|i| i.uid.clone()).collect();
    expected.sort();
    let (first, second) = (&uploads[0].0, &uploads[1].0);
    for access in [first, second] {
        let mut got = all_ids(&db, access).await;
        got.sort();
        assert_eq!(got, expected);
    }
    // Two objects under one id, one in each project: changing or deleting one leaves the other.
    let (a, b) = (&ids.entities[0].uid, &ids.entities[1].uid);
    let moved = match sample.entities[0].clone() {
        Entity::Point(mut p) => {
            p.p.x += 1.0;
            Entity::Point(p)
        }
        other => other,
    };
    let changed = commit(
        &db,
        &ayse,
        first,
        FeatureChange::Update {
            id: a.clone(),
            entity: moved,
        },
        &[(a, "1")],
    )
    .await
    .unwrap();
    assert_eq!(changed.versions[a], "2");
    commit(
        &db,
        &ayse,
        second,
        FeatureChange::Delete { id: b.clone() },
        &[(b, "1")],
    )
    .await
    .unwrap();
    assert_eq!(
        (
            version_of(&db, first, a).await.as_deref(),
            version_of(&db, second, a).await.as_deref(),
            version_of(&db, first, b).await.as_deref(),
            version_of(&db, second, b).await.as_deref(),
        ),
        (Some("2"), Some("1"), Some("1"), None)
    );
    let untouched = projects::features_by_id(&db.app, second, &[Uuid::parse_str(a).unwrap()])
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&untouched[0].entity).unwrap()["p"],
        serde_json::to_value(&sample.entities[0]).unwrap()["p"]
    );
    // One project's upload command sent again into the other is refused as a whole: nothing is written there.
    let mut into_other = uploads[0].1.clone();
    into_other.project_id = second.project.to_string();
    assert!(changes::commit(&db.app, second, into_other).await.is_err());
    assert_eq!(
        (
            version_of(&db, second, a).await.as_deref(),
            version_of(&db, second, b).await.as_deref()
        ),
        (Some("1"), None)
    );
    db.close().await;
}
