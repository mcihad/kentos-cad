//! The server's guard narrows a form's spatial list as the desktop does (docs/adr/0759 §2): a building's
//! parcel must hold it, for an editor's write through an enforced form and for a Veri girişi's alike.

mod common;

use common::{envelope, member, new_project, open, share};
use kentos_application::{AppError, admin, changes};
use kentos_contracts::{
    DocumentSnapshotV1, Entity, FeatureChange, GrantRole, LayerNode, ProjectChanges, ProjectPatch,
    ProjectSettings, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");
const E: f64 = 486_500.0;
const N: f64 = 4_420_200.0;

fn layer(id: &str, name: &str, fields: serde_json::Value) -> serde_json::Value {
    json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [],
        "fields": fields })
}

/// Parcels and buildings; the building's form takes its parcel from the parcels holding it, published
/// and enforced.
fn patch() -> ProjectPatch {
    let layers: Vec<LayerNode> = serde_json::from_value(json!([
        layer("cizim", "Çizim", json!([])),
        layer(
            "parsel",
            "Parsel",
            json!([{ "name": "no", "kind": "text" }])
        ),
        layer(
            "bina",
            "Bina",
            json!([{ "name": "Parsel", "kind": "text" }, { "name": "Kat", "kind": "integer" }])
        )
    ]))
    .unwrap();
    let body = json!({ "revision": 1, "items": [
        { "type": "field", "id": "parsel", "field": "Parsel", "lookup": {
            "layer": "parsel", "key": "$uid", "label": "no", "spatial": { "relation": "contains" } } },
        { "type": "field", "id": "kat", "field": "Kat" }
    ]});
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).unwrap();
    let mut settings: ProjectSettings = DocumentSnapshotV1::from_json(SAMPLE).unwrap().settings;
    settings.forms = serde_json::from_value(json!([{ "id": "bina", "name": "Bina", "layer": "bina",
        "draft": body.clone(), "enforce": true,
        "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] }]))
    .unwrap();
    ProjectPatch {
        settings: Some(settings),
        layers: Some(layers),
        ..ProjectPatch::default()
    }
}

fn square(layer: &str, attrs: serde_json::Value, [x0, y0, x1, y1]: [f64; 4]) -> Entity {
    serde_json::from_value(json!({
        "kind": "polygon", "id": 1, "layerId": layer, "attrs": attrs,
        "pts": [{ "x": E + x0, "y": N + y0 }, { "x": E + x1, "y": N + y0 },
                { "x": E + x1, "y": N + y1 }, { "x": E + x0, "y": N + y1 }]
    }))
    .unwrap()
}

/// The building, inside parcel A (0–30 × 0–24).
fn building(parcel: &str) -> Entity {
    square(
        "bina",
        json!({ "Parsel": parcel, "Kat": "2" }),
        [6.0, 5.0, 20.0, 17.0],
    )
}

fn features(list: Vec<FeatureChange>) -> ProjectChanges {
    ProjectChanges {
        features: list,
        blocks: Vec::new(),
        project: None,
    }
}

fn refused_as_missing(result: Result<kentos_contracts::CommitResult, AppError>) {
    match result {
        Err(AppError::Invalid { message, .. } | AppError::Forbidden(message)) => {
            assert!(message.contains("listede yok"), "{message}");
        }
        other => panic!("expected the list's refusal, got {other:?}"),
    }
}

#[tokio::test]
async fn a_parcel_not_holding_the_building_is_refused_on_the_server() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "duzenleyici", TenantRole::Viewer).await;
    let clerk = member(&db, "buro", "veri", TenantRole::Viewer).await;
    let project = new_project(&db, &pm, "Yapı ruhsatı").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    share(&db, &owner, &clerk.actor, GrantRole::Entry).await;
    let ed = open(&db, &editor, project).await;
    let entry = open(&db, &clerk, project).await;
    let meta: i64 = sqlx::query_scalar("select meta_version from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let meta = meta.to_string();
    changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some(patch()),
                ..features(Vec::new())
            },
            &[("@project", meta.as_str())],
        ),
    )
    .await
    .unwrap();
    // Parcel A holds the building's place, B is its neighbour, C is far away (out of any window).
    let (a, b, c) = (
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
    );
    changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(vec![
                FeatureChange::Create {
                    id: a.clone(),
                    entity: square("parsel", json!({ "no": "101/5" }), [0.0, 0.0, 30.0, 24.0]),
                },
                FeatureChange::Create {
                    id: b.clone(),
                    entity: square("parsel", json!({ "no": "101/12" }), [30.0, 0.0, 60.0, 24.0]),
                },
                FeatureChange::Create {
                    id: c.clone(),
                    entity: square(
                        "parsel",
                        json!({ "no": "205/1" }),
                        [900.0, 900.0, 930.0, 924.0],
                    ),
                },
            ]),
            &[],
        ),
    )
    .await
    .unwrap();

    // An editor's building with its neighbour's parcel, or with a far one: refused.
    let home = Uuid::new_v4().to_string();
    for wrong in [&b, &c] {
        refused_as_missing(
            changes::commit(
                &db.app,
                &ed,
                envelope(
                    &editor,
                    project,
                    features(vec![FeatureChange::Create {
                        id: home.clone(),
                        entity: building(wrong),
                    }]),
                    &[],
                ),
            )
            .await,
        );
    }
    // With the parcel holding it: taken.
    let made = changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(vec![FeatureChange::Create {
                id: home.clone(),
                entity: building(&a),
            }]),
            &[],
        ),
    )
    .await
    .unwrap();
    let v1 = made.versions[&home].clone();

    // The Veri girişi reads its reach: the same narrowing.
    refused_as_missing(
        changes::commit(
            &db.app,
            &entry,
            envelope(
                &clerk,
                project,
                features(vec![FeatureChange::Update {
                    id: home.clone(),
                    entity: building(&b),
                }]),
                &[(home.as_str(), v1.as_str())],
            ),
        )
        .await,
    );
    let mut kept = building(&a);
    kept.base_mut().attrs.insert("Kat".into(), "4".into());
    changes::commit(
        &db.app,
        &entry,
        envelope(
            &clerk,
            project,
            features(vec![FeatureChange::Update {
                id: home.clone(),
                entity: kept,
            }]),
            &[(home.as_str(), v1.as_str())],
        ),
    )
    .await
    .unwrap();
}
