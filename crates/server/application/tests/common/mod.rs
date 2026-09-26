//! Helpers of the database tests of projects and their access (lifecycle.rs,
//! retention.rs, access.rs): members with a role, a small project, sharing,
//! a command envelope.
#![allow(dead_code)] // each test file uses its own share of them

use std::collections::BTreeMap;

use kentos_application::access::{self, ProjectAccess};
use kentos_application::identity::{self, Actor};
use kentos_application::tenancy::{self, Access};
use kentos_application::{admin, projects, sharing};
use kentos_contracts::{
    CommandEnvelope, DocumentSnapshotV1, Entity, FeatureChange, GrantRole, PROJECT_ACCESS_REVOKE,
    PROJECT_CHANGES, PROJECT_SHARE, ProjectAccessChange, ProjectAccessRevoke, ProjectChanges,
    ProjectCreate, ProjectShare, SignInMethod, TenantRole,
};
use kentos_postgres::testing::TestDb;
use uuid::Uuid;

pub const PASSWORD: &str = "dogru-parola-123";
const SAMPLE: &str = include_str!("../../../../../fixtures/document/v1/sample.json");

/// A new account, signed in (no tenant).
pub async fn account(db: &TestDb, login: &str) -> Actor {
    let user = admin::create_local_user(&db.owner, login, login, None, PASSWORD)
        .await
        .unwrap();
    identity::actor_of(&db.app, user, SignInMethod::Local)
        .await
        .unwrap()
        .unwrap()
}

/// A new account with this role (and a seat) in `tenant`, signed in.
pub async fn member(db: &TestDb, tenant: &str, login: &str, role: TenantRole) -> Access {
    let actor = account(db, login).await;
    admin::set_membership(&db.owner, tenant, login, role, true)
        .await
        .unwrap();
    let tenant_id: Uuid = sqlx::query_scalar("select id from kentos.tenant where slug = $1")
        .bind(tenant)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    tenancy::access(&db.app, &actor, tenant_id).await.unwrap()
}

/// The actor's personal space, opened on first use.
pub async fn personal(db: &TestDb, actor: &Actor) -> Access {
    let id = tenancy::ensure_personal(&db.app, actor).await.unwrap();
    tenancy::access(&db.app, actor, id).await.unwrap()
}

/// A project with one unlocked layer, `cizim`, owned by `who`.
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

/// `actor`'s access to a project of `tenant`, as a request would get it.
pub async fn try_open(
    db: &TestDb,
    actor: &Actor,
    tenant: Uuid,
    project: Uuid,
) -> Result<ProjectAccess, kentos_application::AppError> {
    access::project(&db.app, actor, tenant, project).await
}

/// A member's access to a project of their tenant (the test expects to have it).
pub async fn open(db: &TestDb, who: &Access, project: Uuid) -> ProjectAccess {
    try_open(db, &who.actor, who.tenant, project)
        .await
        .unwrap_or_else(|e| panic!("{} cannot open {project}: {e}", who.actor.display_name))
}

pub fn envelope(
    who: &Access,
    project: Uuid,
    input: ProjectChanges,
    expected: &[(&str, &str)],
) -> CommandEnvelope {
    envelope_in(who.tenant, project, input, expected)
}

pub fn envelope_in(
    tenant: Uuid,
    project: Uuid,
    input: ProjectChanges,
    expected: &[(&str, &str)],
) -> CommandEnvelope {
    command(
        PROJECT_CHANGES,
        tenant,
        project,
        serde_json::to_value(input).unwrap(),
        expected,
    )
}

fn command(
    name: &str,
    tenant: Uuid,
    project: Uuid,
    input: serde_json::Value,
    expected: &[(&str, &str)],
) -> CommandEnvelope {
    CommandEnvelope {
        command_name: name.into(),
        version: 1,
        tenant_id: tenant.to_string(),
        project_id: project.to_string(),
        request_id: format!("istek-{}", Uuid::new_v4()),
        idempotency_key: Uuid::new_v4().to_string(),
        expected_versions: expected
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
        input,
    }
}

/// A catalog or lifecycle command (`project.archive` …) from `by`'s access, version 1.
pub fn catalog_envelope(
    by: &ProjectAccess,
    name: &str,
    input: serde_json::Value,
    expected: &[(&str, &str)],
) -> CommandEnvelope {
    command(name, by.tenant, by.project, input, expected)
}

/// Runs a project's command as the server's route does (the default retention).
pub async fn run(
    db: &TestDb,
    by: &ProjectAccess,
    name: &str,
    input: serde_json::Value,
) -> Result<kentos_application::commands::CommandOutcome, kentos_application::AppError> {
    kentos_application::commands::run(
        &db.app,
        &kentos_application::commands::CatalogPolicy::default(),
        by,
        catalog_envelope(by, name, input, &[]),
    )
    .await
}

/// The catalog entry a command answered with.
pub fn changed(
    outcome: kentos_application::commands::CommandOutcome,
) -> kentos_contracts::ProjectCatalogChange {
    match outcome {
        kentos_application::commands::CommandOutcome::Catalog(c) => c,
        other => panic!("expected a catalog change, got {other:?}"),
    }
}

pub fn share_envelope(
    by: &ProjectAccess,
    user: Uuid,
    role: GrantRole,
    expires_at: Option<String>,
) -> CommandEnvelope {
    command(
        PROJECT_SHARE,
        by.tenant,
        by.project,
        serde_json::to_value(ProjectShare {
            user_id: user.to_string(),
            role,
            expires_at,
        })
        .unwrap(),
        &[],
    )
}

pub fn revoke_envelope(by: &ProjectAccess, user: Uuid) -> CommandEnvelope {
    command(
        PROJECT_ACCESS_REVOKE,
        by.tenant,
        by.project,
        serde_json::to_value(ProjectAccessRevoke {
            user_id: user.to_string(),
        })
        .unwrap(),
        &[],
    )
}

/// `by` (who may share) gives `to` a role in the project.
pub async fn share(
    db: &TestDb,
    by: &ProjectAccess,
    to: &Actor,
    role: GrantRole,
) -> ProjectAccessChange {
    sharing::share(&db.app, by, share_envelope(by, to.user_id, role, None))
        .await
        .unwrap()
}

/// `by` takes `to`'s grant away.
pub async fn revoke(db: &TestDb, by: &ProjectAccess, to: &Actor) -> ProjectAccessChange {
    sharing::revoke(&db.app, by, revoke_envelope(by, to.user_id))
        .await
        .unwrap()
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
