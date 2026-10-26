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
    patch_in(["Durum", "Sorumlu", "Not"], scope)
}

/// [`patch`] with other names for the three fields.
fn patch_in([state, who, note]: [&str; 3], scope: &str) -> ProjectPatch {
    let layers: Vec<LayerNode> = serde_json::from_value(json!([{
        "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [],
        "fields": [
            { "name": state, "kind": "text" },
            { "name": who, "kind": "text" },
            { "name": note, "kind": "text" }
        ]
    }]))
    .unwrap();
    let body = json!({ "revision": 1, "scope": scope, "items": [
        { "type": "field", "id": "durum", "field": state },
        { "type": "field", "id": "sorumlu", "field": who }
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

/// Field names that are none of the language's words (`Durum` and `Not` are).
const NAMES: [&str; 3] = ["Hal", "Sorumlu", "Aciklama"];

/// The form's release history ending with `scopes`' last: each a version after the one before.
fn patch_of(scopes: &[&str]) -> ProjectPatch {
    let mut p = patch_in(NAMES, scopes[scopes.len() - 1]);
    let s = p.settings.as_mut().unwrap();
    s.forms[0].releases = scopes
        .iter()
        .enumerate()
        .map(|(i, scope)| kentos_contracts::FormRelease {
            version: i as u32 + 1,
            ..patch_in(NAMES, scope).settings.unwrap().forms[0].releases[0].clone()
        })
        .collect();
    s.forms[0].draft = s.forms[0].releases.last().unwrap().body.clone();
    p
}

/// The scope reaches the same records whether the database checks a part of it or not (docs/adr/0649 §1):
/// each scope's records are worked out by hand, and the Veri girişi's page and count give them. A value with
/// a quote and SQL in it is bound, not run.
#[tokio::test]
async fn a_scope_sent_to_the_database_reaches_what_it_reaches_judged() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = common::member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = person(&db, "duzenleyici", "duzen@belediye.test").await;
    let ali = person(&db, "ali", "ali@belediye.test").await;
    let project = new_project(&db, &pm, "Hasar tespiti").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    share(&db, &owner, &ali.actor, GrantRole::Entry).await;
    let ed = open(&db, &editor, project).await;
    let ali_p = open(&db, &ali, project).await;
    let meta: i64 = sqlx::query_scalar("select meta_version from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let mut meta = meta.to_string();
    let first = changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some(patch_of(&["Sorumlu = @kullanici_eposta"])),
                ..features(Vec::new())
            },
            &[("@project", meta.as_str())],
        ),
    )
    .await
    .unwrap();
    meta = first.meta_version;
    let records = [
        json!({ "Hal": "iyi", "Sorumlu": "ali@belediye.test", "Aciklama": "ofis" }),
        json!({ "Hal": "kötü", "Sorumlu": "ayse@belediye.test", "Aciklama": "saha" }),
        json!({ "Hal": "iyi", "Sorumlu": "ali@belediye.test", "Aciklama": "a'b" }),
        json!({ "Hal": "", "Sorumlu": "veli@belediye.test", "Aciklama": "12" }),
        json!({ "Sorumlu": "ALI@belediye.test", "Aciklama": "x'); delete from kentos.feature; --" }),
    ];
    let ids: Vec<String> = records.iter().map(|_| Uuid::new_v4().to_string()).collect();
    changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(
                ids.iter()
                    .zip(&records)
                    .enumerate()
                    .map(|(i, (id, r))| FeatureChange::Create {
                        id: id.clone(),
                        entity: building(r.clone(), 20.0 * i as f64),
                    })
                    .collect(),
            ),
            &[],
        ),
    )
    .await
    .unwrap();
    let cases: [(&str, &[usize]); 13] = [
        ("Sorumlu = @kullanici_eposta", &[0, 2]),
        ("Hal = 'iyi'", &[0, 2]),
        ("'iyi' = Hal", &[0, 2]),
        ("Hal içinde ('iyi', 'kötü')", &[0, 1, 2]),
        ("Hal = 'iyi' ve Aciklama = 'ofis'", &[0]),
        ("Hal = 'kötü' veya Sorumlu = 'veli@belediye.test'", &[1, 3]),
        ("Aciklama = 'a''b'", &[2]),
        ("Hal boş", &[3, 4]),
        ("Hal boş değil", &[0, 1, 2]),
        ("Sorumlu = 'ALI@belediye.test'", &[4]),
        ("Aciklama = 'x''); delete from kentos.feature; --'", &[4]),
        ("Hal = 'iyi' ve uzunluk(Aciklama) = 4", &[0]),
        ("Hal = 'yok'", &[]),
    ];
    let mut history = vec!["Sorumlu = @kullanici_eposta"];
    for (scope, reached) in cases {
        if history.last() != Some(&scope) {
            history.push(scope);
            let done = changes::commit(
                &db.app,
                &owner,
                envelope(
                    &pm,
                    project,
                    ProjectChanges {
                        project: Some(patch_of(&history)),
                        ..features(Vec::new())
                    },
                    &[("@project", meta.as_str())],
                ),
            )
            .await
            .unwrap_or_else(|e| panic!("{scope}: {e:?}"));
            meta = done.meta_version;
        }
        let mut read: Vec<String> = projects::features(&db.app, &ali_p, None, 100)
            .await
            .unwrap()
            .features
            .into_iter()
            .map(|f| f.id)
            .collect();
        read.sort();
        let mut want: Vec<String> = reached.iter().map(|i| ids[*i].clone()).collect();
        want.sort();
        assert_eq!(read, want, "{scope}");
        assert_eq!(
            projects::info(&db.app, &ali_p).await.unwrap().feature_count,
            reached.len().to_string(),
            "{scope}"
        );
    }
    let left: i64 = sqlx::query_scalar("select count(*) from kentos.feature where project_id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    assert_eq!(left, records.len() as i64, "the quoted value was bound");
}

/// How long a narrowed Veri girişi's count takes on 100 000 records (docs/adr/0649, Ölçümler): a scope the
/// database checks against one it cannot (`uzunluk(…)`), both reaching the same 1 000.
#[tokio::test]
#[ignore = "süre ölçümü: cargo test --release -p kentos-application --test project_form_scope scope_timing -- --ignored --nocapture"]
async fn scope_timing() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = common::member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = person(&db, "duzenleyici", "duzen@belediye.test").await;
    let ali = person(&db, "ali", "ali@belediye.test").await;
    let project = new_project(&db, &pm, "Hasar tespiti").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    share(&db, &owner, &ali.actor, GrantRole::Entry).await;
    let ed = open(&db, &editor, project).await;
    let ali_p = open(&db, &ali, project).await;
    let meta: i64 = sqlx::query_scalar("select meta_version from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let pushed = "Sorumlu = @kullanici_eposta";
    let judged = "uzunluk(Sorumlu) = 17 ve Sorumlu gibi 'ali%'";
    let done = changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some(patch_of(&[pushed])),
                ..features(Vec::new())
            },
            &[("@project", meta.to_string().as_str())],
        ),
    )
    .await
    .unwrap();
    let seed = Uuid::new_v4();
    changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(vec![FeatureChange::Create {
                id: seed.to_string(),
                entity: building(json!({ "Hal": "iyi", "Sorumlu": "x" }), 0.0),
            }]),
            &[],
        ),
    )
    .await
    .unwrap();
    sqlx::query(
        "insert into kentos.feature (tenant_id, project_id, id, layer_id, kind, source_kind, srid, geom, cad_definition,
                                     properties, projection_version, created_by, updated_by)
         select tenant_id, project_id, gen_random_uuid(), layer_id, kind, source_kind, srid, geom, cad_definition,
                jsonb_build_object('Hal', 'iyi', 'Sorumlu',
                  case when i % 100 = 0 then 'ali@belediye.test' else 'baska@belediye.test' end),
                projection_version, created_by, updated_by
           from kentos.feature, generate_series(1, 100000) i where id = $1",
    )
    .bind(seed)
    .execute(&db.owner)
    .await
    .unwrap();
    sqlx::query("delete from kentos.feature where id = $1")
        .bind(seed)
        .execute(&db.owner)
        .await
        .unwrap();
    sqlx::query("analyze kentos.feature")
        .execute(&db.owner)
        .await
        .unwrap();
    let time = |label: &'static str| {
        let db = &db;
        let ali_p = ali_p.clone();
        async move {
            let mut best = f64::MAX;
            for _ in 0..5 {
                let t = std::time::Instant::now();
                let n = projects::info(&db.app, &ali_p).await.unwrap().feature_count;
                best = best.min(t.elapsed().as_secs_f64() * 1000.0);
                assert_eq!(n, "1000", "{label}");
            }
            println!("{label}: {best:.1} ms (en iyi 5)");
        }
    };
    time("SQL'e itilen kapsam").await;
    changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some(patch_of(&[pushed, judged])),
                ..features(Vec::new())
            },
            &[("@project", done.meta_version.as_str())],
        ),
    )
    .await
    .unwrap();
    time("burada yargılanan kapsam").await;
}
