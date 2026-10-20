//! A form's scope on the server (docs/adr/0539): a Veri girişi reads, counts and writes only the records
//! its form's scope holds true for, by the person signed in (`@kullanici_eposta`); the others are not
//! there for it, as a project it may not see is not. An editor reads everything. A scope that does not
//! compile gives nothing.

mod common;

use common::{envelope, new_project, open, share};
use kentos_application::{AppError, admin, changes, projects, tenancy};
use kentos_contracts::{
    DocumentSnapshotV1, Entity, FeatureChange, GrantRole, LayerNode, ProjectChanges, ProjectPatch,
    ProjectSettings, SignInMethod, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

/// The damage layer with its fields, and a published form of them whose scope is `scope`.
fn patch(scope: &str) -> ProjectPatch {
    let layers: Vec<LayerNode> = serde_json::from_value(json!([{
        "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [],
        "fields": [
            { "name": "Durum", "kind": "text" },
            { "name": "Sorumlu", "kind": "text" },
            { "name": "Not", "kind": "text" }
        ]
    }]))
    .unwrap();
    let body = json!({ "revision": 1, "scope": scope, "items": [
        { "type": "field", "id": "durum", "field": "Durum" },
        { "type": "field", "id": "sorumlu", "field": "Sorumlu" }
    ]});
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).unwrap();
    let mut settings: ProjectSettings = DocumentSnapshotV1::from_json(SAMPLE).unwrap().settings;
    settings.forms = serde_json::from_value(json!([{ "id": "hasar", "name": "Hasar", "layer": "cizim",
        "draft": body, "releases": [{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }] }]))
    .unwrap();
    ProjectPatch {
        settings: Some(settings),
        layers: Some(layers),
        ..ProjectPatch::default()
    }
}

fn building(attrs: serde_json::Value, x: f64) -> Entity {
    serde_json::from_value(json!({
        "kind": "polygon", "id": 1, "layerId": "cizim", "attrs": attrs,
        "pts": [{ "x": 486500.0 + x, "y": 4420200.0 }, { "x": 486510.0 + x, "y": 4420200.0 }, { "x": 486510.0 + x, "y": 4420208.0 }]
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

/// A member of `tenant` with an e-mail, signed in.
async fn person(db: &TestDb, login: &str, email: &str) -> tenancy::Access {
    let user = admin::create_local_user(&db.owner, login, login, Some(email), common::PASSWORD)
        .await
        .unwrap();
    admin::set_membership(&db.owner, "buro", login, TenantRole::Viewer, true)
        .await
        .unwrap();
    let actor = kentos_application::identity::actor_of(&db.app, user, SignInMethod::Local)
        .await
        .unwrap()
        .unwrap();
    let tenant: Uuid = sqlx::query_scalar("select id from kentos.tenant where slug = 'buro'")
        .fetch_one(&db.owner)
        .await
        .unwrap();
    tenancy::access(&db.app, &actor, tenant).await.unwrap()
}

#[tokio::test]
async fn a_veri_girisi_reads_counts_and_writes_only_its_scope() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = common::member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = person(&db, "duzenleyici", "duzen@belediye.test").await;
    let ali = person(&db, "ali", "ali@belediye.test").await;
    let ayse = person(&db, "ayse", "ayse@belediye.test").await;
    let project = new_project(&db, &pm, "Hasar tespiti").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    share(&db, &owner, &ali.actor, GrantRole::Entry).await;
    share(&db, &owner, &ayse.actor, GrantRole::Entry).await;
    let ed = open(&db, &editor, project).await;
    let ali_p = open(&db, &ali, project).await;
    let ayse_p = open(&db, &ayse, project).await;

    let meta: i64 = sqlx::query_scalar("select meta_version from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let published = changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some(patch("Sorumlu = @kullanici_eposta")),
                ..features(Vec::new())
            },
            &[("@project", meta.to_string().as_str())],
        ),
    )
    .await
    .unwrap();
    // Three records, two of them Ali's.
    let ids: Vec<String> = (0..3).map(|_| Uuid::new_v4().to_string()).collect();
    let whose = ["ali@belediye.test", "ayse@belediye.test", "ali@belediye.test"];
    let made = changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(
                ids.iter()
                    .zip(whose)
                    .enumerate()
                    .map(|(i, (id, w))| FeatureChange::Create {
                        id: id.clone(),
                        entity: building(
                            json!({ "Durum": "iyi", "Sorumlu": w, "Not": "ofis" }),
                            20.0 * i as f64,
                        ),
                    })
                    .collect(),
            ),
            &[],
        ),
    )
    .await
    .unwrap();

    // Reading: the editor every record, each Veri girişi its own.
    let page = |by: &kentos_application::access::ProjectAccess| {
        let by = by.clone();
        let db = &db;
        async move {
            let mut ids: Vec<String> = projects::features(&db.app, &by, None, 100)
                .await
                .unwrap()
                .features
                .into_iter()
                .map(|f| f.id)
                .collect();
            ids.sort();
            ids
        }
    };
    let sorted = |v: &[&String]| {
        let mut v: Vec<String> = v.iter().map(|s| (*s).clone()).collect();
        v.sort();
        v
    };
    assert_eq!(page(&ed).await, sorted(&[&ids[0], &ids[1], &ids[2]]));
    assert_eq!(page(&ali_p).await, sorted(&[&ids[0], &ids[2]]));
    assert_eq!(page(&ayse_p).await, sorted(&[&ids[1]]));
    // Counting tells nothing of the others.
    assert_eq!(projects::info(&db.app, &ed).await.unwrap().feature_count, "3");
    assert_eq!(projects::info(&db.app, &ali_p).await.unwrap().feature_count, "2");
    // Asked by id, a record out of reach is not there.
    let one = Uuid::parse_str(&ids[1]).unwrap();
    assert!(
        projects::features_by_id(&db.app, &ali_p, &[one])
            .await
            .unwrap()
            .is_empty()
    );
    // A small page still leads on: its cursor is the last row read, in reach or not.
    let first = projects::features(&db.app, &ayse_p, None, 1).await.unwrap();
    assert!(first.next.is_some() || first.features.len() == 1);

    // Writing: Ali changes his record; Ayşe's is not there for him; his own may not leave his reach.
    let v = |i: usize| made.versions[&ids[i]].clone();
    let own = changes::commit(
        &db.app,
        &ali_p,
        envelope(
            &ali,
            project,
            features(vec![FeatureChange::Update {
                id: ids[0].clone(),
                entity: building(json!({ "Durum": "kotu", "Sorumlu": "ali@belediye.test" }), 0.0),
            }]),
            &[(ids[0].as_str(), v(0).as_str())],
        ),
    )
    .await;
    assert!(own.is_ok(), "{own:?}");
    let other = changes::commit(
        &db.app,
        &ali_p,
        envelope(
            &ali,
            project,
            features(vec![FeatureChange::Update {
                id: ids[1].clone(),
                entity: building(json!({ "Durum": "kotu", "Sorumlu": "ayse@belediye.test" }), 20.0),
            }]),
            &[(ids[1].as_str(), v(1).as_str())],
        ),
    )
    .await;
    assert!(matches!(other, Err(AppError::NotFound(_))), "{other:?}");
    let away = changes::commit(
        &db.app,
        &ali_p,
        envelope(
            &ali,
            project,
            features(vec![FeatureChange::Update {
                id: ids[2].clone(),
                entity: building(json!({ "Durum": "iyi", "Sorumlu": "ayse@belediye.test" }), 40.0),
            }]),
            &[(ids[2].as_str(), v(2).as_str())],
        ),
    )
    .await;
    assert!(
        matches!(&away, Err(AppError::Forbidden(m)) if m.contains("kapsamının dışına")),
        "{away:?}"
    );
    let stored: serde_json::Value =
        sqlx::query_scalar("select properties from kentos.feature where id = $1")
            .bind(Uuid::parse_str(&ids[2]).unwrap())
            .fetch_one(&db.owner)
            .await
            .unwrap();
    assert_eq!(stored["Sorumlu"], "ali@belediye.test", "nothing written");

    // A scope that does not compile gives nothing: the Veri girişi sees no record of the layer.
    let broken = changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some({
                    let mut p = patch("Yok = @kullanici_eposta");
                    // A second version, after the first.
                    let s = p.settings.as_mut().unwrap();
                    let mut first = patch("Sorumlu = @kullanici_eposta").settings.unwrap().forms[0]
                        .releases[0]
                        .clone();
                    let second = s.forms[0].releases[0].clone();
                    first.version = 1;
                    s.forms[0].releases = vec![
                        first,
                        kentos_contracts::FormRelease {
                            version: 2,
                            ..second
                        },
                    ];
                    p
                }),
                ..features(Vec::new())
            },
            &[("@project", published.meta_version.as_str())],
        ),
    )
    .await;
    assert!(broken.is_ok(), "{broken:?}");
    assert!(page(&ali_p).await.is_empty());
    assert_eq!(page(&ed).await.len(), 3, "the editor is not scoped");
}
