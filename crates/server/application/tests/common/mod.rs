//! Helpers of the database tests of a project's life (lifecycle.rs,
//! retention.rs): members with a role, a small project, a command envelope.
#![allow(dead_code)] // each test file uses its own share of them

use std::collections::BTreeMap;

use kentos_application::identity;
use kentos_application::tenancy::{self, Access};
use kentos_application::{admin, projects};
use kentos_contracts::{
    CommandEnvelope, DocumentSnapshotV1, Entity, FeatureChange, PROJECT_CHANGES, ProjectChanges,
    ProjectCreate, SignInMethod, TenantRole,
};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

pub const PASSWORD: &str = "dogru-parola-123";
const SAMPLE: &str = include_str!("../../../../../fixtures/document/v1/sample.json");

/// A new account with this role (and a seat) in `tenant`, signed in.
pub async fn member(db: &TestDb, tenant: &str, login: &str, role: TenantRole) -> Access {
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

/// A project with one unlocked layer, `cizim`.
pub async fn new_project(db: &TestDb, who: &Access, name: &str) -> Uuid {
    let s = DocumentSnapshotV1::from_json(SAMPLE).unwrap();
    let input = ProjectCreate {
        name: name.into(),
        settings: s.settings,
        origin: s.origin,
        home_view: s.home_view,
        layers: vec![serde_json::from_value(serde_json::json!({ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] })).unwrap()],
        active_layer: "cizim".into(),
        styles: s.styles,
    };
    Uuid::parse_str(
        &projects::create(&db.app, who, input, None)
            .await
            .unwrap()
            .id,
    )
    .unwrap()
}

pub fn envelope(
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

/// One new point on `cizim`.
pub fn a_point(x: f64) -> ProjectChanges {
    let entity: Entity = serde_json::from_value(serde_json::json!({ "kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": { "x": x, "y": 4420210.0 } })).unwrap();
    ProjectChanges {
        features: vec![FeatureChange::Create {
            id: Uuid::new_v4().to_string(),
            entity,
        }],
        project: None,
    }
}
