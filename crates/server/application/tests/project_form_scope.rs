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
    let whose = [
        "ali@belediye.test",
        "ayse@belediye.test",
        "ali@belediye.test",
    ];
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
    assert_eq!(
        projects::info(&db.app, &ed).await.unwrap().feature_count,
        "3"
    );
    assert_eq!(
        projects::info(&db.app, &ali_p).await.unwrap().feature_count,
        "2"
    );
    // Asked by id, a record out of reach is not there.
    let one = Uuid::parse_str(&ids[1]).unwrap();
    assert!(
        projects::features_by_id(&db.app, &ali_p, &[one])
            .await
            .unwrap()
            .is_empty()
    );
    // A small page is full, and its cursor is one of her records (docs/adr/0738 §2): she has one, so none follows.
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
                entity: building(
                    json!({ "Durum": "kotu", "Sorumlu": "ali@belediye.test" }),
                    0.0,
                ),
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
                entity: building(
                    json!({ "Durum": "kotu", "Sorumlu": "ayse@belediye.test" }),
                    20.0,
                ),
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
                entity: building(
                    json!({ "Durum": "iyi", "Sorumlu": "ayse@belediye.test" }),
                    40.0,
                ),
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

/// A project with an editor and a Veri girişi (Ali), the form's scope published over [`NAMES`].
struct Scoped {
    project: Uuid,
    pm: tenancy::Access,
    owner: kentos_application::access::ProjectAccess,
    editor: tenancy::Access,
    ed: kentos_application::access::ProjectAccess,
    ali: kentos_application::access::ProjectAccess,
    /// The project's meta version after the form was published.
    meta: String,
}

impl Scoped {
    async fn new(db: &TestDb, scope: &str) -> Scoped {
        admin::create_tenant(&db.owner, "buro", "Büro", 6)
            .await
            .unwrap();
        let pm = common::member(db, "buro", "yonetici", TenantRole::ProjectManager).await;
        let editor = person(db, "duzenleyici", "duzen@belediye.test").await;
        let ali = person(db, "ali", "ali@belediye.test").await;
        let project = new_project(db, &pm, "Hasar tespiti").await;
        let owner = open(db, &pm, project).await;
        share(db, &owner, &editor.actor, GrantRole::Editor).await;
        share(db, &owner, &ali.actor, GrantRole::Entry).await;
        let ed = open(db, &editor, project).await;
        let ali = open(db, &ali, project).await;
        let meta: i64 = sqlx::query_scalar("select meta_version from kentos.project where id = $1")
            .bind(project)
            .fetch_one(&db.owner)
            .await
            .unwrap();
        let mut s = Scoped {
            project,
            pm,
            owner,
            editor,
            ed,
            ali,
            meta: meta.to_string(),
        };
        s.publish(db, &[scope]).await;
        s
    }

    /// The form's releases ending with `scopes`' last, published by the owner.
    async fn publish(&mut self, db: &TestDb, scopes: &[&str]) {
        let done = changes::commit(
            &db.app,
            &self.owner,
            envelope(
                &self.pm,
                self.project,
                ProjectChanges {
                    project: Some(patch_of(scopes)),
                    ..features(Vec::new())
                },
                &[("@project", self.meta.as_str())],
            ),
        )
        .await
        .unwrap();
        self.meta = done.meta_version;
    }

    /// The editor's commit of `list`, each change of an existing record at `versions`'; the new versions are
    /// kept there.
    async fn edit(
        &self,
        db: &TestDb,
        list: Vec<FeatureChange>,
        versions: &mut std::collections::HashMap<String, String>,
    ) {
        let expected: Vec<(String, String)> = list
            .iter()
            .filter_map(|c| match c {
                FeatureChange::Update { id, .. } | FeatureChange::Delete { id } => {
                    Some((id.clone(), versions[id].clone()))
                }
                _ => None,
            })
            .collect();
        let pairs: Vec<(&str, &str)> = expected
            .iter()
            .map(|(i, v)| (i.as_str(), v.as_str()))
            .collect();
        let done = changes::commit(
            &db.app,
            &self.ed,
            envelope(&self.editor, self.project, features(list), &pairs),
        )
        .await
        .unwrap();
        versions.extend(done.versions);
    }
}

/// A Veri girişi's pages are full and lead on from its own records alone (docs/adr/0738 §2): twelve records in id
/// order, every other one Ali's, read a few at a time, with a scope the database checks and with one it cannot. No
/// cursor is a record out of Ali's reach; the pages together are his records in order.
#[tokio::test]
async fn a_veri_girisi_pages_are_full_and_lead_on_from_its_own_records() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let pushed = "Sorumlu = @kullanici_eposta";
    let mut s = Scoped::new(&db, pushed).await;
    let ids: Vec<String> = (1..=12)
        .map(|i| format!("00000000-0000-4000-8000-{i:012}"))
        .collect();
    let whose = |i: usize| {
        if i % 2 == 1 {
            "ali@belediye.test"
        } else {
            "ayse@belediye.test"
        }
    };
    let mut versions = Default::default();
    s.edit(
        &db,
        ids.iter()
            .enumerate()
            .map(|(i, id)| FeatureChange::Create {
                id: id.clone(),
                entity: building(
                    json!({ "Hal": "iyi", "Sorumlu": whose(i) }),
                    20.0 * i as f64,
                ),
            })
            .collect(),
        &mut versions,
    )
    .await;
    let mine: Vec<String> = (0..12)
        .filter(|i| i % 2 == 1)
        .map(|i| ids[i].clone())
        .collect();
    let ali_access = s.ali.clone();
    let read_all = |limit: i64| {
        let (db, ali, mine) = (&db, ali_access.clone(), mine.clone());
        async move {
            let mut read = Vec::new();
            let mut after = None;
            loop {
                let page = projects::features(&db.app, &ali, after, limit)
                    .await
                    .unwrap();
                let n = page.features.len() as i64;
                read.extend(page.features.iter().map(|f| f.id.clone()));
                let Some(next) = page.next else {
                    break;
                };
                assert_eq!(n, limit, "a page with a cursor is full");
                assert!(mine.contains(&next), "the cursor {next} is one of Ali's");
                assert_eq!(
                    Some(&next),
                    read.last(),
                    "the cursor is the last record given"
                );
                after = Some(Uuid::parse_str(&next).unwrap());
            }
            read
        }
    };
    for limit in [1, 2, 5, 6, 100] {
        assert_eq!(read_all(limit).await, mine, "pushed, {limit} at a time");
    }
    // The same with a scope only the server judges (`uzunluk`): both e-mails are 17 letters long, so
    // `gibi` keeps it to Ali's.
    s.publish(
        &db,
        &[pushed, "uzunluk(Sorumlu) = 17 ve Sorumlu gibi 'ali%'"],
    )
    .await;
    for limit in [1, 2, 5, 100] {
        assert_eq!(read_all(limit).await, mine, "judged, {limit} at a time");
    }
}

/// The changes a Veri girişi hears of (docs/adr/0738 §3), read after each commit: a record it reaches is told as
/// it is; one that leaves its reach as removed; one it never reached not at all: its creation, its changes, its
/// removal. A scope that reads the shape keeps no earlier copy: then a change out of reach is told as a removal,
/// as before. The editor hears everything.
#[tokio::test]
async fn a_veri_girisi_hears_nothing_of_records_it_never_reached() {
    use kentos_application::events;
    use kentos_contracts::FeatureOp;
    let Some(db) = TestDb::create().await else {
        return;
    };
    let mut s = Scoped::new(&db, "Sorumlu = @kullanici_eposta").await;
    let mut versions = std::collections::HashMap::new();
    let mut cursor = events::latest(&db.app, &s.ali).await.unwrap();
    // What Ali hears since the last time, as (id, change, version given).
    let ali_access = s.ali.clone();
    let heard = async |cursor: &mut i64| -> Vec<(String, FeatureOp, bool)> {
        let page = events::after(&db.app, &ali_access, *cursor, 100)
            .await
            .unwrap();
        *cursor = page.next.parse().unwrap();
        page.events
            .iter()
            .flat_map(|e| e.features.iter())
            .map(|f| (f.id.clone(), f.op, f.version.is_some()))
            .collect()
    };
    let (a, b, c) = (
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string(),
    );
    let rec = |who: &str, hal: &str| building(json!({ "Hal": hal, "Sorumlu": who }), 0.0);
    let (ali, ayse) = ("ali@belediye.test", "ayse@belediye.test");
    let create = |id: &String, entity: Entity| FeatureChange::Create {
        id: id.clone(),
        entity,
    };
    let update = |id: &String, entity: Entity| FeatureChange::Update {
        id: id.clone(),
        entity,
    };
    let delete = |id: &String| FeatureChange::Delete { id: id.clone() };

    // A is Ali's, B Ayşe's: he hears of A alone.
    s.edit(
        &db,
        vec![create(&a, rec(ali, "iyi")), create(&b, rec(ayse, "iyi"))],
        &mut versions,
    )
    .await;
    assert_eq!(
        heard(&mut cursor).await,
        [(a.clone(), FeatureOp::Create, true)]
    );
    // B changes and stays Ayşe's: nothing, not even an empty event.
    let before = cursor;
    s.edit(&db, vec![update(&b, rec(ayse, "kötü"))], &mut versions)
        .await;
    assert!(heard(&mut cursor).await.is_empty());
    let page = events::after(&db.app, &s.ali, before, 100).await.unwrap();
    assert!(page.events.is_empty(), "the event is not told at all");
    assert!(
        page.next.parse::<i64>().unwrap() > before,
        "the cursor still leads on"
    );
    // A leaves his reach: told as removed, without its version.
    s.edit(&db, vec![update(&a, rec(ayse, "iyi"))], &mut versions)
        .await;
    assert_eq!(
        heard(&mut cursor).await,
        [(a.clone(), FeatureOp::Delete, false)]
    );
    // B comes into his reach: told as changed.
    s.edit(&db, vec![update(&b, rec(ali, "kötü"))], &mut versions)
        .await;
    assert_eq!(
        heard(&mut cursor).await,
        [(b.clone(), FeatureOp::Update, true)]
    );
    // C, Ayşe's, made and removed: nothing.
    s.edit(&db, vec![create(&c, rec(ayse, "iyi"))], &mut versions)
        .await;
    assert!(heard(&mut cursor).await.is_empty());
    s.edit(&db, vec![delete(&c)], &mut versions).await;
    assert!(heard(&mut cursor).await.is_empty());
    // A, out of his reach, removed: nothing; B, his, removed: told.
    s.edit(&db, vec![delete(&a), delete(&b)], &mut versions)
        .await;
    assert_eq!(
        heard(&mut cursor).await,
        [(b.clone(), FeatureOp::Delete, false)]
    );
    // E, his, leaves his reach and is removed before he reads: the change tells him it is gone; the removal,
    // of a record out of his reach, does not.
    let e = Uuid::new_v4().to_string();
    s.edit(&db, vec![create(&e, rec(ali, "iyi"))], &mut versions)
        .await;
    assert_eq!(
        heard(&mut cursor).await,
        [(e.clone(), FeatureOp::Create, true)]
    );
    s.edit(&db, vec![update(&e, rec(ayse, "iyi"))], &mut versions)
        .await;
    s.edit(&db, vec![delete(&e)], &mut versions).await;
    assert_eq!(
        heard(&mut cursor).await,
        [(e.clone(), FeatureOp::Delete, false)]
    );
    // The editor heard of every record.
    let all = events::after(&db.app, &s.ed, 0, 100).await.unwrap();
    let told: Vec<&String> = all
        .events
        .iter()
        .flat_map(|e| e.features.iter().map(|f| &f.id))
        .collect();
    for id in [&a, &b, &c] {
        assert!(told.contains(&id), "the editor hears of {id}");
    }
    // A scope that reads the shape keeps no earlier copy: a change out of reach is told as a removal.
    s.publish(
        &db,
        &[
            "Sorumlu = @kullanici_eposta",
            "Sorumlu = @kullanici_eposta ve $alan > 1",
        ],
    )
    .await;
    cursor = events::latest(&db.app, &s.ali).await.unwrap();
    let d = Uuid::new_v4().to_string();
    s.edit(&db, vec![create(&d, rec(ayse, "iyi"))], &mut versions)
        .await;
    assert!(
        heard(&mut cursor).await.is_empty(),
        "a record made out of reach is still not told"
    );
    s.edit(&db, vec![update(&d, rec(ayse, "kötü"))], &mut versions)
        .await;
    assert_eq!(
        heard(&mut cursor).await,
        [(d.clone(), FeatureOp::Delete, false)],
        "the shape is read: told as removed"
    );
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
            // The first page of 500 and every page to the end (docs/adr/0738 §2).
            let (mut first, mut whole) = (f64::MAX, f64::MAX);
            for _ in 0..5 {
                let t = std::time::Instant::now();
                let page = projects::features(&db.app, &ali_p, None, 500)
                    .await
                    .unwrap();
                first = first.min(t.elapsed().as_secs_f64() * 1000.0);
                assert_eq!(page.features.len(), 500, "{label}");
                let mut read = page.features.len();
                let mut next = page.next;
                while let Some(after) = next {
                    let page = projects::features(
                        &db.app,
                        &ali_p,
                        Some(Uuid::parse_str(&after).unwrap()),
                        500,
                    )
                    .await
                    .unwrap();
                    read += page.features.len();
                    next = page.next;
                }
                assert_eq!(read, 1000, "{label}");
                whole = whole.min(t.elapsed().as_secs_f64() * 1000.0);
            }
            println!(
                "{label}: ilk 500'lük sayfa {first:.1} ms, bütün sayfalar {whole:.1} ms (en iyi 5)"
            );
        }
    };
    time("SQL'e itilen kapsam").await;
    let done = changes::commit(
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

    // The editor's commit of 1 000 changes on the scoped layer (docs/adr/0738 §3): with a scope that reads an
    // attribute the earlier copies are kept with the event; with one that reads the shape none is.
    let thousand: Vec<(Uuid, i64)> = sqlx::query_as(
        "select id, version from kentos.feature where project_id = $1 order by id limit 1000",
    )
    .bind(project)
    .fetch_all(&db.owner)
    .await
    .unwrap();
    let mut meta = done.meta_version.clone();
    let mut versions: std::collections::HashMap<String, String> = thousand
        .iter()
        .map(|(id, v)| (id.to_string(), v.to_string()))
        .collect();
    for (label, scopes) in [
        ("öncekiler tutulur", vec![pushed, judged, pushed]),
        (
            "şekli okuyan kapsam, öncekiler tutulmaz",
            vec![
                pushed,
                judged,
                pushed,
                "Sorumlu = @kullanici_eposta ve $alan > 0",
            ],
        ),
    ] {
        let published = changes::commit(
            &db.app,
            &owner,
            envelope(
                &pm,
                project,
                ProjectChanges {
                    project: Some(patch_of(&scopes)),
                    ..features(Vec::new())
                },
                &[("@project", meta.as_str())],
            ),
        )
        .await
        .unwrap();
        meta = published.meta_version;
        let mut best = f64::MAX;
        for round in 0..3 {
            let list: Vec<FeatureChange> = thousand
                .iter()
                .map(|(id, _)| FeatureChange::Update {
                    id: id.to_string(),
                    entity: building(
                        json!({ "Hal": format!("tur {round}"), "Sorumlu": "baska@belediye.test" }),
                        0.0,
                    ),
                })
                .collect();
            let expected: Vec<(String, String)> = versions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let pairs: Vec<(&str, &str)> = expected
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let t = std::time::Instant::now();
            let made = changes::commit(
                &db.app,
                &ed,
                envelope(&editor, project, features(list), &pairs),
            )
            .await
            .unwrap();
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
            versions.extend(made.versions);
        }
        let size: i32 = sqlx::query_scalar(
            "select pg_column_size(payload) from kentos.outbox_event where project_id = $1 order by seq desc limit 1",
        )
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
        println!("1 000 değişiklik, {label}: {best:.1} ms (en iyi 3), olay kaydı {size} bayt");
    }
}
