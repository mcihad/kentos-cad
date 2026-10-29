//! The one resolver of what a caller reads and takes out, on the server (docs/adr/0550). A Veri girişi is told
//! nothing of the records out of its reach: not in a count, an extent, an event, a conflict, a form's lookup;
//! a layer whose forms are all withdrawn reads nothing. A field a form marks `redact` is cut from a copy of
//! the project a form designer does not take (the project's file, an export job), and a copy stored as it was
//! written is not given to whoever may not take it all; a job's result cut for a reach that changed while it
//! ran is refused at the finish. Real database (`KENTOS_TEST_DB=required` makes a missing one a failure).

mod common;

use std::time::Duration;

use common::{blobs, envelope, member, new_project, open, share};
use kentos_application::access::ProjectAccess;
use kentos_application::jobs::{JobLimits, work_once_between};
use kentos_application::{
    AppError, admin, changes, checkpoints, events, jobs, listing, projects, snapshot, tenancy,
};
use kentos_contracts::{
    DocumentSnapshotV1, Entity, FeatureChange, FeatureOp, GrantRole, LayerNode, ProjectChanges,
    ProjectPatch, ProjectSettings, ServerJobCreate, ServerJobState, SignInMethod, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::{Value, json};
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");
const LEASE: Duration = Duration::from_secs(30);

fn layers() -> Vec<LayerNode> {
    let layer = |id: &str, name: &str, fields: Value| {
        json!({
            "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [],
            "fields": fields
        })
    };
    serde_json::from_value(json!([
        layer(
            "cizim",
            "Çizim",
            json!([
                { "name": "Durum", "kind": "text" },
                { "name": "Sorumlu", "kind": "text" },
                { "name": "Malik", "kind": "text" },
                { "name": "Mahalle", "kind": "text" },
                { "name": "Not", "kind": "text" }
            ])
        ),
        layer(
            "mahalle",
            "Mahalle",
            json!([
                { "name": "Ad", "kind": "text" },
                { "name": "Sorumlu", "kind": "text" }
            ])
        )
    ]))
    .unwrap()
}

/// One form as the project stores it: published (its release is the body), the draft the same unless it is
/// given.
fn form(
    id: &str,
    layer: &str,
    items: Value,
    scope: Option<&str>,
    withdrawn: bool,
    draft_items: Option<Value>,
) -> Value {
    form_v(id, layer, items, scope, withdrawn, draft_items, None)
}

/// Like [`form`], with a second release `later` (a published release is never changed: a new one is added).
fn form_v(
    id: &str,
    layer: &str,
    items: Value,
    scope: Option<&str>,
    withdrawn: bool,
    draft_items: Option<Value>,
    later: Option<Value>,
) -> Value {
    let release = |version: u32, items: &Value| {
        let body = json!({ "revision": 1, "scope": scope, "items": items });
        let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).unwrap();
        json!({ "version": version, "digest": kentos_contracts::body_digest(&typed), "body": body })
    };
    let mut releases = vec![release(1, &items)];
    if let Some(l) = &later {
        releases.push(release(2, l));
    }
    let current = later.clone().unwrap_or_else(|| items.clone());
    let draft = match draft_items {
        Some(d) => json!({ "revision": 2, "scope": scope, "items": d }),
        None => json!({ "revision": 1, "scope": scope, "items": current }),
    };
    let mut f =
        json!({ "id": id, "name": id, "layer": layer, "draft": draft, "releases": releases });
    if withdrawn {
        f["withdrawn"] = json!(true);
    }
    f
}

fn field(name: &str) -> Value {
    json!({ "type": "field", "id": name.to_lowercase(), "field": name })
}

fn marked(name: &str) -> Value {
    json!({ "type": "field", "id": name.to_lowercase(), "field": name, "redact": true })
}

fn lookup_field() -> Value {
    json!({ "type": "field", "id": "mahalle", "field": "Mahalle", "lookup": { "layer": "mahalle", "key": "Ad" } })
}

/// A patch that sets the project's layers and forms.
fn patch(forms: Vec<Value>) -> ProjectPatch {
    let mut settings: ProjectSettings = DocumentSnapshotV1::from_json(SAMPLE).unwrap().settings;
    settings.forms = serde_json::from_value(Value::Array(forms)).unwrap();
    ProjectPatch {
        settings: Some(settings),
        layers: Some(layers()),
        ..ProjectPatch::default()
    }
}

async fn meta_of(db: &TestDb, project: Uuid) -> String {
    let meta: i64 = sqlx::query_scalar("select meta_version from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    meta.to_string()
}

/// The owner sets the forms.
async fn set_forms(
    db: &TestDb,
    owner: &ProjectAccess,
    pm: &tenancy::Access,
    project: Uuid,
    forms: Vec<Value>,
) {
    let meta = meta_of(db, project).await;
    let done = changes::commit(
        &db.app,
        owner,
        envelope(
            pm,
            project,
            ProjectChanges {
                project: Some(patch(forms)),
                ..features(Vec::new())
            },
            &[("@project", meta.as_str())],
        ),
    )
    .await;
    assert!(done.is_ok(), "{done:?}");
}

fn features(list: Vec<FeatureChange>) -> ProjectChanges {
    ProjectChanges {
        features: list,
        blocks: Vec::new(),
        project: None,
    }
}

fn polygon(layer: &str, attrs: Value, x: f64) -> Entity {
    serde_json::from_value(json!({
        "kind": "polygon", "id": 1, "layerId": layer, "attrs": attrs,
        "pts": [{ "x": 486500.0 + x, "y": 4420200.0 }, { "x": 486510.0 + x, "y": 4420200.0 }, { "x": 486510.0 + x, "y": 4420208.0 }]
    }))
    .unwrap()
}

/// A member of "buro" with an e-mail, signed in.
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

const ALI: &str = "ali@belediye.test";
const AYSE: &str = "ayse@belediye.test";

/// The usual project: the owner `pm`, an editor, a viewer, two Veri girişi (Ali, Ayşe); three records on `cizim`,
/// Ali's two first and Ayşe's the last (the farthest east), the form `hasar` scoped by e-mail, `Malik` marked
/// `redact` in it.
struct World {
    db: TestDb,
    pm: tenancy::Access,
    owner: ProjectAccess,
    project: Uuid,
    editor: ProjectAccess,
    editor_who: tenancy::Access,
    looker: ProjectAccess,
    ali: ProjectAccess,
    ali_who: tenancy::Access,
    ids: Vec<String>,
}

async fn world() -> Option<World> {
    let db = TestDb::create().await?;
    admin::create_tenant(&db.owner, "buro", "Büro", 6)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor_who = person(&db, "duzenleyici", "duzen@belediye.test").await;
    let viewer = person(&db, "izleyici", "izle@belediye.test").await;
    let ali = person(&db, "ali", ALI).await;
    let ayse = person(&db, "ayse", AYSE).await;
    let project = new_project(&db, &pm, "Hasar tespiti").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor_who.actor, GrantRole::Editor).await;
    share(&db, &owner, &viewer.actor, GrantRole::Viewer).await;
    share(&db, &owner, &ali.actor, GrantRole::Entry).await;
    share(&db, &owner, &ayse.actor, GrantRole::Entry).await;
    let editor = open(&db, &editor_who, project).await;
    let looker = open(&db, &viewer, project).await;
    let ali_p = open(&db, &ali, project).await;
    set_forms(
        &db,
        &owner,
        &pm,
        project,
        vec![form(
            "hasar",
            "cizim",
            json!([field("Durum"), field("Sorumlu"), marked("Malik")]),
            Some("Sorumlu = @kullanici_eposta"),
            false,
            None,
        )],
    )
    .await;
    let ids: Vec<String> = (0..3).map(|_| Uuid::new_v4().to_string()).collect();
    let whose = [ALI, ALI, AYSE];
    changes::commit(
        &db.app,
        &editor,
        envelope(
            &editor_who,
            project,
            features(
                ids.iter()
                    .zip(whose)
                    .enumerate()
                    .map(|(i, (id, w))| FeatureChange::Create {
                        id: id.clone(),
                        entity: polygon(
                            "cizim",
                            json!({ "Durum": "iyi", "Sorumlu": w, "Malik": format!("malik-{i}"), "Not": "ofis" }),
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
    Some(World {
        db,
        pm,
        owner,
        project,
        editor,
        editor_who,
        looker,
        ali: ali_p,
        ali_who: ali,
        ids,
    })
}

fn attrs_of(entity: &Entity) -> serde_json::Map<String, Value> {
    serde_json::to_value(&entity.base().attrs)
        .unwrap()
        .as_object()
        .unwrap()
        .clone()
}

#[tokio::test]
async fn a_veri_girisi_is_told_nothing_of_the_records_out_of_its_reach() {
    let Some(w) = world().await else { return };
    let db = &w.db;
    // A count and an extent: Ali's two records, not the third; the editor's all three.
    let ed = listing::details(&db.app, &w.editor).await.unwrap();
    let ali = listing::details(&db.app, &w.ali).await.unwrap();
    assert_eq!(
        (ed.feature_count.as_str(), ali.feature_count.as_str()),
        ("3", "2")
    );
    let (b_ed, b_ali) = (ed.bounds.unwrap(), ali.bounds.unwrap());
    assert!(
        b_ali.max_x < b_ed.max_x,
        "Ayşe's record is the farthest east: {b_ali:?} {b_ed:?}"
    );
    assert_eq!(
        projects::info(&db.app, &w.ali).await.unwrap().feature_count,
        "2"
    );

    // The event of the commit: Ayşe's record, which Ali never reached, is not told of at all (docs/adr/0738 §3;
    // it was told as a removal before).
    let page = events::after(&db.app, &w.ali, 0, 100).await.unwrap();
    let created = page
        .events
        .iter()
        .find(|e| e.features.iter().any(|f| f.id == w.ids[0]))
        .expect("the commit's event");
    let by_id = |id: &str| created.features.iter().find(|f| f.id == id);
    assert_eq!(by_id(&w.ids[0]).unwrap().op, FeatureOp::Create);
    assert!(by_id(&w.ids[0]).unwrap().version.is_some());
    assert_eq!(created.features.len(), 2, "{:?}", created.features);
    assert!(by_id(&w.ids[2]).is_none(), "Ayşe's record is not told of");
    assert!(
        page.events
            .iter()
            .flat_map(|e| e.features.iter())
            .all(|f| f.id != w.ids[2])
    );
    let seen = events::after(&db.app, &w.editor, 0, 100).await.unwrap();
    let all = seen.events.iter().find(|e| e.features.len() == 3).unwrap();
    assert!(
        all.features
            .iter()
            .all(|f| f.op == FeatureOp::Create && f.version.is_some())
    );

    // A conflict on his own record shows it as he reads it; one on Ayşe's is a 404 whatever the version.
    let stale = |i: usize, who: &tenancy::Access| {
        envelope(
            who,
            w.project,
            features(vec![FeatureChange::Update {
                id: w.ids[i].clone(),
                entity: polygon(
                    "cizim",
                    json!({ "Durum": "kotu", "Sorumlu": if i == 2 { AYSE } else { ALI } }),
                    0.0,
                ),
            }]),
            &[(w.ids[i].as_str(), "999")],
        )
    };
    let ali_who = w.ali_who.clone();
    match changes::commit(&db.app, &w.ali, stale(0, &ali_who)).await {
        Err(AppError::Conflict { conflicts, .. }) => {
            let current = conflicts[0].current.as_ref().expect("his record");
            let attrs = attrs_of(&current.entity);
            assert!(
                attrs.contains_key("Durum") && attrs.contains_key("Malik"),
                "what his form shows"
            );
            assert!(
                !attrs.contains_key("Not"),
                "what it does not show stays unsaid: {attrs:?}"
            );
        }
        other => panic!("a conflict: {other:?}"),
    }
    let other = changes::commit(&db.app, &w.ali, stale(2, &ali_who)).await;
    assert!(matches!(other, Err(AppError::NotFound(_))), "{other:?}");
    // The history's counts are not his either.
    assert!(
        checkpoints::list(&db.app, &w.ali)
            .await
            .unwrap()
            .checkpoints
            .iter()
            .all(|c| c.objects.is_none())
    );
    w.db.close().await;
}

#[tokio::test]
async fn a_lookup_never_says_whether_a_record_out_of_reach_exists() {
    let Some(w) = world().await else { return };
    let db = &w.db;
    // The neighbourhoods: Ali reaches "Acik", not Ayşe's "Gizli".
    let m1 = Uuid::new_v4().to_string();
    let m2 = Uuid::new_v4().to_string();
    set_forms(
        db,
        &w.owner,
        &w.pm,
        w.project,
        vec![
            form_v(
                "hasar",
                "cizim",
                json!([field("Durum"), field("Sorumlu"), marked("Malik")]),
                Some("Sorumlu = @kullanici_eposta"),
                false,
                None,
                Some(json!([field("Durum"), field("Sorumlu"), lookup_field()])),
            ),
            form(
                "mahalle",
                "mahalle",
                json!([field("Ad"), field("Sorumlu")]),
                Some("Sorumlu = @kullanici_eposta"),
                false,
                None,
            ),
        ],
    )
    .await;
    changes::commit(
        &db.app,
        &w.editor,
        envelope(
            &w.editor_who,
            w.project,
            features(vec![
                FeatureChange::Create {
                    id: m1,
                    entity: polygon("mahalle", json!({ "Ad": "Acik", "Sorumlu": ALI }), 100.0),
                },
                FeatureChange::Create {
                    id: m2,
                    entity: polygon("mahalle", json!({ "Ad": "Gizli", "Sorumlu": AYSE }), 120.0),
                },
            ]),
            &[],
        ),
    )
    .await
    .unwrap();
    let ali_who = w.ali_who.clone();
    let version = || async {
        let v: i64 = sqlx::query_scalar("select version from kentos.feature where id = $1")
            .bind(Uuid::parse_str(&w.ids[0]).unwrap())
            .fetch_one(&db.owner)
            .await
            .unwrap();
        v.to_string()
    };
    let write = |mahalle: &'static str| {
        let w = &w;
        let ali_who = &ali_who;
        let version = version();
        async move {
            let v = version.await;
            changes::commit(
                &w.db.app,
                &w.ali,
                envelope(
                    ali_who,
                    w.project,
                    features(vec![FeatureChange::Update {
                        id: w.ids[0].clone(),
                        entity: polygon(
                            "cizim",
                            json!({ "Durum": "iyi", "Sorumlu": ALI, "Mahalle": mahalle }),
                            0.0,
                        ),
                    }]),
                    &[(w.ids[0].as_str(), v.as_str())],
                ),
            )
            .await
        }
    };
    let reachable = write("Acik").await;
    assert!(reachable.is_ok(), "{reachable:?}");
    let hidden = write("Gizli").await;
    let missing = write("Hiç").await;
    let said = |r: &Result<_, AppError>| match r {
        Err(AppError::Forbidden(m)) => m.replace("Gizli", "X").replace("Hiç", "X"),
        other => panic!("refused: {other:?}"),
    };
    // A value that exists only out of reach is refused as one that does not exist at all.
    assert_eq!(said(&hidden), said(&missing));
    w.db.close().await;
}

#[tokio::test]
async fn a_layer_whose_forms_are_all_withdrawn_reads_nothing_for_a_veri_girisi() {
    let Some(w) = world().await else { return };
    let db = &w.db;
    assert_eq!(
        listing::details(&db.app, &w.ali)
            .await
            .unwrap()
            .feature_count,
        "2"
    );
    set_forms(
        db,
        &w.owner,
        &w.pm,
        w.project,
        vec![form(
            "hasar",
            "cizim",
            json!([field("Durum"), field("Sorumlu"), marked("Malik")]),
            Some("Sorumlu = @kullanici_eposta"),
            true,
            None,
        )],
    )
    .await;
    let page = projects::features(&db.app, &w.ali, None, 100)
        .await
        .unwrap();
    assert!(page.features.is_empty());
    let d = listing::details(&db.app, &w.ali).await.unwrap();
    assert_eq!(d.feature_count, "0");
    assert!(d.bounds.is_none());
    // The editor is not narrowed by a form that no longer exists for data entry.
    assert_eq!(
        projects::features(&db.app, &w.editor, None, 100)
            .await
            .unwrap()
            .features
            .len(),
        3
    );
    w.db.close().await;
}

fn attrs_in(bytes: &[u8]) -> Vec<serde_json::Map<String, Value>> {
    let doc = kentos_kcad::decode(bytes).unwrap();
    doc.entities.iter().map(attrs_of).collect()
}

#[tokio::test]
async fn a_copy_is_cut_of_the_marked_fields_unless_its_taker_designs_forms_and_a_stored_one_is_withheld()
 {
    let Some(w) = world().await else { return };
    let db = &w.db;
    // The viewer's file lacks `Malik`; the owner (a form designer) has it; the others are untouched.
    let cut = snapshot::snapshot(&db.app, &w.looker).await.unwrap();
    let whole = snapshot::snapshot(&db.app, &w.owner).await.unwrap();
    assert_eq!(cut.objects, 3);
    let (cut, whole) = (attrs_in(&cut.bytes), attrs_in(&whole.bytes));
    assert!(
        cut.iter()
            .all(|a| !a.contains_key("Malik") && a["Durum"] == "iyi" && a["Not"] == "ofis"),
        "{cut:?}"
    );
    assert!(whole.iter().all(|a| a.contains_key("Malik")), "{whole:?}");
    // A Veri girişi has no right to a copy at all.
    assert!(matches!(
        snapshot::snapshot(&db.app, &w.ali).await,
        Err(AppError::Forbidden(_))
    ));

    // A checkpoint is stored as it was written: the owner takes it, the viewer does not while a field is marked.
    let store = blobs();
    let job = jobs::create(
        &db.app,
        &w.owner,
        ServerJobCreate {
            kind: "project.checkpoint".into(),
            input: json!({ "name": "Teslim" }),
            request_id: "k".into(),
            timeout_seconds: None,
        },
        &JobLimits::default(),
    )
    .await
    .unwrap();
    let done = jobs::work_once(
        &db.app,
        &store,
        Uuid::now_v7(),
        LEASE,
        &JobLimits::default(),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded, "{job:?}");
    let id = Uuid::parse_str(
        checkpoints::list(&db.app, &w.owner)
            .await
            .unwrap()
            .checkpoints[0]
            .id
            .as_str(),
    )
    .unwrap();
    assert!(
        checkpoints::download(&db.app, &store, &w.owner, id)
            .await
            .is_ok()
    );
    let refused = checkpoints::download(&db.app, &store, &w.looker, id).await;
    assert!(
        matches!(&refused, Err(AppError::Forbidden(m)) if m.contains("örneklerde gizle")),
        "{:?}",
        refused.map(|_| ())
    );
    // Nothing marked: the same viewer takes it.
    set_forms(
        db,
        &w.owner,
        &w.pm,
        w.project,
        vec![form_v(
            "hasar",
            "cizim",
            json!([field("Durum"), field("Sorumlu"), marked("Malik")]),
            Some("Sorumlu = @kullanici_eposta"),
            false,
            None,
            Some(json!([field("Durum"), field("Sorumlu"), field("Malik")])),
        )],
    )
    .await;
    assert!(
        checkpoints::download(&db.app, &store, &w.looker, id)
            .await
            .is_ok()
    );
    w.db.close().await;
}

#[tokio::test]
async fn an_export_is_cut_for_its_asker_and_refused_when_the_reach_changed_while_it_ran() {
    let Some(w) = world().await else { return };
    let db = &w.db;
    let store = blobs();
    let limits = JobLimits::default();
    let ask = |who: &ProjectAccess, request: &str| {
        let who = who.clone();
        let request = request.to_owned();
        let db = &db.app;
        let limits = &limits;
        async move {
            jobs::create(
                db,
                &who,
                ServerJobCreate {
                    kind: "project.export".into(),
                    input: json!({}),
                    request_id: request,
                    timeout_seconds: None,
                },
                limits,
            )
            .await
            .unwrap()
        }
    };
    let read_artifact = |job: &kentos_contracts::ServerJob| {
        let id = Uuid::parse_str(&job.id).unwrap();
        let who = w.editor.clone();
        let (db, store) = (&db.app, &store);
        async move {
            let (_, mut file) = jobs::artifact(db, store, &who, id).await.unwrap();
            let mut bytes = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut file, &mut bytes)
                .await
                .unwrap();
            bytes
        }
    };
    // The editor's export is cut.
    let first = ask(&w.editor, "bir").await;
    let done = jobs::work_once(&db.app, &store, Uuid::now_v7(), LEASE, &limits)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(done.state, ServerJobState::Succeeded);
    let attrs = attrs_in(&read_artifact(&first).await);
    assert!(attrs.iter().all(|a| !a.contains_key("Malik")), "{attrs:?}");

    // A field marked while the job runs: what it built was cut by the marks as they were, so it is refused.
    let second = ask(&w.editor, "iki").await;
    let (pm, owner, project) = (&w.pm, &w.owner, w.project);
    let done = work_once_between(&db.app, &store, Uuid::now_v7(), LEASE, &limits, || async {
        set_forms(
            db,
            owner,
            pm,
            project,
            vec![form(
                "hasar",
                "cizim",
                json!([field("Durum"), field("Sorumlu"), marked("Malik")]),
                Some("Sorumlu = @kullanici_eposta"),
                false,
                Some(json!([
                    field("Durum"),
                    field("Sorumlu"),
                    marked("Malik"),
                    marked("Not")
                ])),
            )],
        )
        .await;
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(done.state, ServerJobState::Failed);
    let failed = jobs::get(&db.app, &w.editor, Uuid::parse_str(&second.id).unwrap())
        .await
        .unwrap();
    assert_eq!(
        failed.error.as_ref().unwrap().code,
        "reach_changed",
        "{failed:?}"
    );
    assert!(failed.artifact.is_none());

    // A form designer demoted to editor while the export runs: the copy built whole is refused.
    let designer = person(db, "tasarimci", "tasarim@belediye.test").await;
    share(db, &w.owner, &designer.actor, GrantRole::Manager).await;
    let by = open(db, &designer, w.project).await;
    let third = ask(&by, "uc").await;
    let (owner, actor) = (&w.owner, &designer.actor);
    let done = work_once_between(&db.app, &store, Uuid::now_v7(), LEASE, &limits, || async {
        share(db, owner, actor, GrantRole::Editor).await;
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(done.state, ServerJobState::Failed);
    let failed = jobs::get(&db.app, &w.owner, Uuid::parse_str(&third.id).unwrap())
        .await
        .unwrap();
    assert_eq!(
        failed.error.as_ref().unwrap().code,
        "reach_changed",
        "{failed:?}"
    );
    w.db.close().await;
}
