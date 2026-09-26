//! A database project as one KCAD v2 file (docs/adr/0033): a drawing made
//! into a project comes back as the same drawing, a snapshot is one moment
//! and holds no lock, and the download policy applies.
//! Real database (`KENTOS_TEST_DB=required` makes a missing one a failure).

mod common;

use std::collections::BTreeMap;
use std::time::Duration;

use common::{a_point, envelope, member, new_project, open};
use kentos_application::{AppError, admin, changes, projects, snapshot};
use kentos_contracts::{
    DocumentSnapshotV2, EntityId, FeatureChange, GrantRole, LayerNode, ProjectChanges,
    ProjectCreate, ProjectId, ProjectPatch, ProjectStorage, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const DRAWING: &[u8] = include_bytes!("../../../../fixtures/kcad/v2/drawing.kcad");
/// The object of `drawing.kcad` whose values the server does not keep (beyond ±10⁹).
const EXTREME: usize = 13;

/// Every object by its persistent id, without its slot (a file keeps no slots).
fn by_uid(doc: &DocumentSnapshotV2) -> BTreeMap<EntityId, Value> {
    doc.uids
        .iter()
        .zip(&doc.entities)
        .map(|(uid, e)| {
            let mut v = serde_json::to_value(e).unwrap();
            v.as_object_mut().unwrap().remove("id");
            (*uid, v)
        })
        .collect()
}

fn unlock(layers: &mut [LayerNode]) {
    for l in layers {
        l.locked = false;
        unlock(&mut l.children);
    }
}

fn create_input(source: &DocumentSnapshotV2, name: &str, layers: Vec<LayerNode>) -> ProjectCreate {
    ProjectCreate {
        name: name.into(),
        settings: source.settings.clone(),
        origin: source.origin,
        home_view: source.home_view,
        layers,
        active_layer: source.active_layer.clone(),
        styles: source.styles.clone(),
        description: None,
        project_type: None,
        tags: None,
        storage: None,
    }
}

#[tokio::test]
async fn a_drawing_made_into_a_project_comes_back_as_the_same_drawing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let source = kentos_kcad::decode(DRAWING).unwrap();
    // Nothing is drawn on a locked layer: the project starts unlocked and is locked after its objects, as a user would do it.
    let mut unlocked = source.layers.clone();
    unlock(&mut unlocked);
    let created = projects::create(
        &db.app,
        &ayse,
        create_input(&source, "Ada 5 çizimi", unlocked),
        None,
    )
    .await
    .unwrap();
    let project = Uuid::parse_str(&created.id).unwrap();
    let by = open(&db, &ayse, project).await;
    let all: Vec<FeatureChange> = source
        .uids
        .iter()
        .zip(&source.entities)
        .map(|(uid, e)| FeatureChange::Create {
            id: uid.to_text(),
            entity: e.clone(),
        })
        .collect();
    // The server keeps values within ±10⁹ (cad.rs): the file's object of extreme values
    // (f64's largest, the smallest subnormal) is refused by its place, and nothing is written.
    let batch = |features: Vec<FeatureChange>| {
        envelope(
            &ayse,
            project,
            ProjectChanges {
                features,
                project: None,
            },
            &[],
        )
    };
    match changes::commit(&db.app, &by, batch(all.clone())).await {
        Err(e) => assert_eq!(
            (e.code(), e.path()),
            ("invalid", Some("features[13].entity"))
        ),
        Ok(r) => panic!("the object of extreme values was accepted: {r:?}"),
    }
    assert_eq!(
        projects::info(&db.app, &by).await.unwrap().feature_count,
        "0"
    );
    let extreme = source.uids[EXTREME];
    let features = all
        .into_iter()
        .enumerate()
        .filter(|(i, _)| *i != EXTREME)
        .map(|(_, f)| f)
        .collect();
    changes::commit(
        &db.app,
        &by,
        envelope(
            &ayse,
            project,
            ProjectChanges {
                features,
                project: None,
            },
            &[],
        ),
    )
    .await
    .unwrap();
    let meta = projects::info(&db.app, &by).await.unwrap().meta_version;
    let lock = ProjectChanges {
        features: vec![],
        project: Some(ProjectPatch {
            layers: Some(source.layers.clone()),
            ..Default::default()
        }),
    };
    changes::commit(
        &db.app,
        &by,
        envelope(&ayse, project, lock, &[("@project", &meta)]),
    )
    .await
    .unwrap();

    let snap = snapshot::snapshot(&db.app, &by).await.unwrap();
    let back = kentos_kcad::decode(&snap.bytes).unwrap();
    let digest: String = Sha256::digest(&snap.bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(snap.sha256, digest);
    assert_eq!(snap.objects, source.entities.len() - 1);
    assert_eq!(back.name, "Ada 5 çizimi");
    assert_eq!(back.project_id, Some(ProjectId(*project.as_bytes())));
    // Settings, layers (the locked and the hidden one too), styles and the view as the file had
    // them; the few negative zeros come back as zero (compared equal).
    assert_eq!(back.settings, source.settings);
    assert_eq!(
        (back.origin, back.home_view),
        (source.origin, source.home_view)
    );
    assert_eq!(back.layers, source.layers);
    assert_eq!(back.active_layer, source.active_layer);
    assert_eq!(back.styles, source.styles);
    let mut expected = by_uid(&source);
    expected.remove(&extreme);
    assert_eq!(by_uid(&back), expected);
    // All thirteen kinds went there and back.
    let kinds: std::collections::BTreeSet<String> = back
        .entities
        .iter()
        .map(|e| serde_json::to_value(e).unwrap()["kind"].to_string())
        .collect();
    assert_eq!(kinds.len(), 13);
    // In id order, as a client opening the project gets them.
    assert!(back.uids.windows(2).all(|w| w[0] < w[1]));
    // The revision and the event cursor of the moment it shows.
    let info = projects::info(&db.app, &by).await.unwrap();
    assert_eq!(snap.revision.to_string(), info.data_revision);
    assert_eq!(snap.event_cursor.to_string(), info.event_cursor);
    db.close().await;
}

#[tokio::test]
async fn a_snapshot_is_one_moment_and_holds_no_lock() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 3)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let project = new_project(&db, &ayse, "Anlık").await;
    let by = open(&db, &ayse, project).await;
    changes::commit(
        &db.app,
        &by,
        envelope(&ayse, project, a_point(486500.0), &[]),
    )
    .await
    .unwrap();

    let mut tx = db.app.snapshot(by.scope()).await.unwrap();
    // A commit while the snapshot is open neither waits for it nor shows in it.
    tokio::time::timeout(
        Duration::from_secs(10),
        changes::commit(
            &db.app,
            &by,
            envelope(&ayse, project, a_point(486510.0), &[]),
        ),
    )
    .await
    .expect("the commit waited for the snapshot")
    .unwrap();
    let (doc, revision, _) = snapshot::read(&mut tx, &by).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(doc.entities.len(), 1, "the snapshot saw a later commit");

    let now = snapshot::snapshot(&db.app, &by).await.unwrap();
    assert_eq!((now.objects, now.revision), (2, revision + 1));
    db.close().await;
}

#[tokio::test]
async fn snapshots_follow_the_download_policy() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 4)
        .await
        .unwrap();
    let ayse = member(&db, "buro", "ayse", TenantRole::ProjectManager).await;
    let dilek = member(&db, "buro", "dilek", TenantRole::Viewer).await;
    let project = new_project(&db, &ayse, "Paylaşılan").await;
    let by = open(&db, &ayse, project).await;
    common::share(&db, &by, &dilek.actor, GrantRole::Viewer).await;

    // A viewer downloads while the organisation lets viewers download.
    let d = open(&db, &dilek, project).await;
    assert!(snapshot::snapshot(&db.app, &d).await.is_ok());
    admin::set_tenant_policy(&db.owner, "buro", None, Some(false))
        .await
        .unwrap();
    let d = open(&db, &dilek, project).await;
    assert!(matches!(
        snapshot::snapshot(&db.app, &d).await,
        Err(AppError::Forbidden(_))
    ));

    // A file project is its revisions already.
    let source = kentos_kcad::decode(DRAWING).unwrap();
    let mut input = create_input(&source, "Dosya", source.layers.clone());
    input.storage = Some(ProjectStorage::File);
    let file = Uuid::parse_str(
        &projects::create(&db.app, &ayse, input, None)
            .await
            .unwrap()
            .id,
    )
    .unwrap();
    let f = open(&db, &ayse, file).await;
    match snapshot::snapshot(&db.app, &f).await {
        Err(AppError::Invalid { message, .. }) => {
            assert!(message.contains("dosya olarak"), "{message}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    db.close().await;
}
