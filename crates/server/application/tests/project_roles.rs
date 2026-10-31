//! Each project role through the server's own commands (docs/adr/0765): making, changing, reshaping and
//! deleting a record, the form's draft and release, the layer tree, a download and sharing, judged against the
//! independent reference's matrix (scripts/fixtures/role_routes_cases.py, fixtures/acl/v1/roles.json). A
//! refused operation is a 403 and writes nothing.

mod common;

use common::{envelope, member, new_project, open, share, share_envelope};
use kentos_application::access::ProjectAccess;
use kentos_application::{AppError, admin, changes, sharing, snapshot};
use kentos_contracts::{
    DocumentSnapshotV1, Entity, FeatureChange, GrantRole, LayerNode, ProjectChanges, ProjectPatch,
    ProjectSettings, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");
const CASES: &str = include_str!("../../../../fixtures/acl/v1/roles.json");

fn layers(name: &str) -> Vec<LayerNode> {
    serde_json::from_value(json!([{
        "id": "cizim", "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [],
        "fields": [
            { "name": "Durum", "kind": "text", "values": [{ "code": "iyi", "label": "İyi" }, { "code": "kotu", "label": "Kötü" }] },
            { "name": "Kat", "kind": "integer", "min": "1" },
            { "name": "Not", "kind": "text" }
        ]
    }]))
    .unwrap()
}

fn body(revision: u32, note: bool) -> Value {
    let mut items = vec![
        json!({ "type": "field", "id": "durum", "field": "Durum" }),
        json!({ "type": "field", "id": "kat", "field": "Kat" }),
    ];
    if note {
        items.push(json!({ "type": "field", "id": "not", "field": "Not" }));
    }
    json!({ "revision": revision, "items": items })
}

fn release(version: u32, b: &Value) -> Value {
    let typed: kentos_contracts::FormBody = serde_json::from_value(b.clone()).unwrap();
    json!({ "version": version, "digest": kentos_contracts::body_digest(&typed), "body": b })
}

/// The project's settings with the building form: `draft` its draft, `releases` its releases.
fn settings(draft: &Value, releases: Vec<Value>) -> ProjectPatch {
    let mut s: ProjectSettings = DocumentSnapshotV1::from_json(SAMPLE).unwrap().settings;
    s.forms = serde_json::from_value(json!([{ "id": "bina", "name": "Bina", "layer": "cizim",
        "draft": draft, "releases": releases }]))
    .unwrap();
    ProjectPatch {
        settings: Some(s),
        layers: Some(layers("Çizim")),
        ..ProjectPatch::default()
    }
}

fn published() -> ProjectPatch {
    let b = body(1, false);
    settings(&b, vec![release(1, &b)])
}

fn building(attrs: Value, x: f64) -> Entity {
    serde_json::from_value(json!({
        "kind": "polygon", "id": 1, "layerId": "cizim", "attrs": attrs,
        "pts": [{ "x": 486500.0, "y": 4420200.0 }, { "x": 486500.0 + x, "y": 4420200.0 }, { "x": 486500.0 + x, "y": 4420208.0 }]
    }))
    .unwrap()
}

fn features(list: Vec<FeatureChange>) -> ProjectChanges {
    ProjectChanges {
        features: list,
        blocks: Vec::new(),
        project: None,
    }
}

async fn revisions(db: &TestDb, project: Uuid) -> (i64, i64) {
    sqlx::query_as("select meta_version, data_revision from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap()
}

#[tokio::test]
async fn each_role_does_what_the_matrix_says() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 9)
        .await
        .unwrap();
    let pm = member(&db, "buro", "sahip", TenantRole::ProjectManager).await;
    let spare = member(&db, "buro", "yedek", TenantRole::Viewer).await;
    let project = new_project(&db, &pm, "Roller").await;
    let owner = open(&db, &pm, project).await;
    let mut people = Vec::new();
    for (role, grant) in [
        ("viewer", GrantRole::Viewer),
        ("commenter", GrantRole::Commenter),
        ("entry", GrantRole::Entry),
        ("editor", GrantRole::Editor),
        ("manager", GrantRole::Manager),
    ] {
        let who = member(&db, "buro", role, TenantRole::Viewer).await;
        share(&db, &owner, &who.actor, grant).await;
        let access = open(&db, &who, project).await;
        people.push((role, who, access));
    }
    let (meta, _) = revisions(&db, project).await;
    changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some(published()),
                ..features(Vec::new())
            },
            &[("@project", meta.to_string().as_str())],
        ),
    )
    .await
    .unwrap();

    let doc: Value = serde_json::from_str(CASES).unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert!(cases.len() >= 20);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let role = case["role"].as_str().unwrap();
        let op = case["operation"].as_str().unwrap();
        let want = case["allowed"].as_bool().unwrap();
        let (_, who, access): &(&str, _, ProjectAccess) =
            people.iter().find(|(r, _, _)| *r == role).unwrap();
        // A fresh building for the case, made by the owner.
        let id = Uuid::new_v4().to_string();
        let made = changes::commit(
            &db.app,
            &owner,
            envelope(
                &pm,
                project,
                features(vec![FeatureChange::Create {
                    id: id.clone(),
                    entity: building(json!({ "Durum": "iyi", "Kat": "2", "Not": "ofis" }), 10.0),
                }]),
                &[],
            ),
        )
        .await
        .unwrap();
        let version = made.versions[&id].clone();
        let uid = Uuid::parse_str(&id).unwrap();
        // The record as the role reads it (a Veri girişi without the note).
        let copy = kentos_application::projects::features_by_id(&db.app, access, &[uid])
            .await
            .ok()
            .and_then(|f| f.into_iter().next())
            .map(|f| f.entity);
        let with = |attrs: &[(&str, &str)], x: f64| {
            let mut e = copy.clone().unwrap_or_else(|| building(json!({}), 10.0));
            if x != 10.0 {
                let mut shaped = building(json!({}), x);
                shaped.base_mut().attrs = e.base().attrs.clone();
                shaped.base_mut().id = e.base().id;
                e = shaped;
            }
            for (k, v) in attrs {
                e.base_mut().attrs.insert((*k).into(), (*v).into());
            }
            e
        };
        let update = |entity: Entity| {
            features(vec![FeatureChange::Update {
                id: id.clone(),
                entity,
            }])
        };
        let before = revisions(&db, project).await;
        let meta = before.0.to_string();
        let at_meta = [("@project", meta.as_str())];
        let at_version = [(id.as_str(), version.as_str())];
        let project_patch = |p: ProjectPatch| ProjectChanges {
            project: Some(p),
            ..features(Vec::new())
        };
        let outcome: Result<(), AppError> = match op {
            "create" => changes::commit(
                &db.app,
                access,
                envelope(
                    who,
                    project,
                    features(vec![FeatureChange::Create {
                        id: Uuid::new_v4().to_string(),
                        entity: building(json!({ "Durum": "iyi", "Kat": "3" }), 12.0),
                    }]),
                    &[],
                ),
            )
            .await
            .map(|_| ()),
            "editShown" => changes::commit(
                &db.app,
                access,
                envelope(
                    who,
                    project,
                    update(with(&[("Durum", "kotu")], 10.0)),
                    &at_version,
                ),
            )
            .await
            .map(|_| ()),
            "editHidden" => changes::commit(
                &db.app,
                access,
                envelope(
                    who,
                    project,
                    update(with(&[("Not", "değişti")], 10.0)),
                    &at_version,
                ),
            )
            .await
            .map(|_| ()),
            "editShape" => changes::commit(
                &db.app,
                access,
                envelope(who, project, update(with(&[], 14.0)), &at_version),
            )
            .await
            .map(|_| ()),
            "delete" => changes::commit(
                &db.app,
                access,
                envelope(
                    who,
                    project,
                    features(vec![FeatureChange::Delete { id: id.clone() }]),
                    &at_version,
                ),
            )
            .await
            .map(|_| ()),
            "designForm" => {
                let b = body(1, false);
                changes::commit(
                    &db.app,
                    access,
                    envelope(
                        who,
                        project,
                        project_patch(settings(&body(2, true), vec![release(1, &b)])),
                        &at_meta,
                    ),
                )
                .await
                .map(|_| ())
            }
            "publishForm" => {
                let (b1, b2) = (body(1, false), body(2, true));
                changes::commit(
                    &db.app,
                    access,
                    envelope(
                        who,
                        project,
                        project_patch(settings(&b2, vec![release(1, &b1), release(2, &b2)])),
                        &at_meta,
                    ),
                )
                .await
                .map(|_| ())
            }
            "layers" => changes::commit(
                &db.app,
                access,
                envelope(
                    who,
                    project,
                    project_patch(ProjectPatch {
                        layers: Some(layers("Binalar")),
                        ..ProjectPatch::default()
                    }),
                    &at_meta,
                ),
            )
            .await
            .map(|_| ()),
            "download" => snapshot::snapshot(&db.app, access).await.map(|_| ()),
            "share" => sharing::share(
                &db.app,
                access,
                share_envelope(access, spare.actor.user_id, GrantRole::Viewer, None),
            )
            .await
            .map(|_| ()),
            other => panic!("unknown operation {other}"),
        };
        match (&outcome, want) {
            (Ok(()), true) => {}
            (Err(AppError::Forbidden(_)), false) => {
                assert_eq!(
                    revisions(&db, project).await,
                    before,
                    "{name}: a refusal writes nothing"
                );
            }
            (got, _) => panic!("{name}: expected allowed = {want}, got {got:?}"),
        }
        // The layer's name is put back. The manager's form draft and release stay: they come after every other
        // role's cases, and a release is never taken back.
        if want && op == "layers" {
            let (meta, _) = revisions(&db, project).await;
            changes::commit(
                &db.app,
                &owner,
                envelope(
                    &pm,
                    project,
                    project_patch(ProjectPatch {
                        layers: Some(layers("Çizim")),
                        ..ProjectPatch::default()
                    }),
                    &[("@project", meta.to_string().as_str())],
                ),
            )
            .await
            .unwrap();
        }
        if want && op == "share" {
            common::revoke(&db, &owner, &spare.actor).await;
        }
    }
}
