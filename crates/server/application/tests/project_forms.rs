//! The forms' permissions and the server's guard (docs/adr/0411, migration 0021): a manager designs
//! and publishes a form that enforces its rules; an editor's write that breaks them is refused; a
//! Veri girişi changes a record's attributes through the form, and nothing else.

mod common;

use common::{envelope, member, new_project, open, share};
use kentos_application::{AppError, admin, changes};
use kentos_contracts::{
    DocumentSnapshotV1, Entity, FeatureChange, GrantRole, LayerNode, ProjectChanges, ProjectPatch,
    ProjectPermission, ProjectRole, ProjectSettings, TenantRole,
};
use kentos_postgres::testing::TestDb;
use serde_json::json;
use uuid::Uuid;

const SAMPLE: &str = include_str!("../../../../fixtures/document/v1/sample.json");

/// The test project's layer with two fields, and its settings with a form of them: published and
/// enforcing when `published`, a draft otherwise.
fn patch(published: bool) -> ProjectPatch {
    let layers: Vec<LayerNode> = serde_json::from_value(json!([{
        "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "ink", "lineType": "continuous", "lineWeight": 0.25 }, "children": [],
        "fields": [
            { "name": "Durum", "kind": "text", "values": [{ "code": "iyi", "label": "İyi" }, { "code": "kotu", "label": "Kötü" }] },
            { "name": "Kat", "kind": "integer", "min": "1" },
            { "name": "Not", "kind": "text" }
        ]
    }]))
    .unwrap();
    let body = json!({ "revision": 1, "items": [
        { "type": "field", "id": "durum", "field": "Durum", "required": true },
        { "type": "field", "id": "kat", "field": "Kat" }
    ]});
    let typed: kentos_contracts::FormBody = serde_json::from_value(body.clone()).unwrap();
    let releases = if published {
        json!([{ "version": 1, "digest": kentos_contracts::body_digest(&typed), "body": body }])
    } else {
        json!([])
    };
    let mut settings: ProjectSettings = DocumentSnapshotV1::from_json(SAMPLE).unwrap().settings;
    settings.forms =
        serde_json::from_value(json!([{ "id": "bina", "name": "Bina", "layer": "cizim",
        "draft": body, "releases": releases, "enforce": published }]))
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

#[tokio::test]
async fn the_forms_are_designed_published_and_guarded_on_the_server() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "duzenleyici", TenantRole::Viewer).await;
    let clerk = member(&db, "buro", "veri", TenantRole::Viewer).await;
    let project = new_project(&db, &pm, "Hasar tespiti").await;
    let owner = open(&db, &pm, project).await;
    assert!(owner.allows(ProjectPermission::FormsDesign));
    assert!(owner.allows(ProjectPermission::FormsPublish));
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    share(&db, &owner, &clerk.actor, GrantRole::Entry).await;
    let ed = open(&db, &editor, project).await;
    let entry = open(&db, &clerk, project).await;
    assert_eq!(entry.role, ProjectRole::Entry);
    assert!(entry.allows(ProjectPermission::FormsUse));
    assert!(!entry.allows(ProjectPermission::FeatureWrite));
    assert!(
        !entry.allows(ProjectPermission::Download),
        "a download would hold the hidden fields"
    );
    assert!(ed.allows(ProjectPermission::FormsUse) && !ed.allows(ProjectPermission::FormsDesign));

    // The manager designs the form, then publishes it with its rules enforced.
    let meta: i64 = sqlx::query_scalar("select meta_version from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let meta = meta.to_string();
    let drafted = changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some(patch(false)),
                ..features(Vec::new())
            },
            &[("@project", meta.as_str())],
        ),
    )
    .await
    .unwrap();
    // A building made while the form is a draft (it holds a field the form does not show).
    let id = Uuid::new_v4().to_string();
    let made = changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(vec![FeatureChange::Create {
                id: id.clone(),
                entity: building(
                    json!({ "Durum": "iyi", "Kat": "2", "Not": "Yalnız ofis görür." }),
                    10.0,
                ),
            }]),
            &[],
        ),
    )
    .await
    .unwrap();
    let v1 = made.versions[&id].clone();
    let published = changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some(patch(true)),
                ..features(Vec::new())
            },
            &[("@project", drafted.meta_version.as_str())],
        ),
    )
    .await
    .unwrap();
    assert_ne!(published.meta_version, drafted.meta_version);

    // The Veri girişi reads the fields the form shows (docs/adr/0411 §6); the editor reads all.
    let read = |by: &kentos_application::access::ProjectAccess| {
        let by = by.clone();
        let db = &db;
        async move {
            kentos_application::projects::features(&db.app, &by, None, 10)
                .await
                .unwrap()
                .features[0]
                .entity
                .base()
                .attrs
                .clone()
        }
    };
    assert!(!read(&entry).await.contains_key("Not"));
    assert_eq!(
        read(&ed).await.get("Not").map(String::as_str),
        Some("Yalnız ofis görür.")
    );
    // Published and enforced: a write that breaks the rules is not taken, from anyone.
    let refused = changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(vec![FeatureChange::Update {
                id: id.clone(),
                entity: building(json!({ "Durum": "iyi", "Kat": "0" }), 10.0),
            }]),
            &[(id.as_str(), v1.as_str())],
        ),
    )
    .await;
    match refused {
        Err(AppError::Invalid { message, path }) => {
            assert_eq!(path.as_deref(), Some("features[0].entity"));
            assert!(message.contains("“Bina” formunun kuralları"), "{message}");
        }
        other => panic!("expected the form's refusal, got {other:?}"),
    }

    // The Veri girişi: the record's attributes through the form; not its shape, not a new object.
    let moved = changes::commit(
        &db.app,
        &entry,
        envelope(
            &clerk,
            project,
            features(vec![FeatureChange::Update {
                id: id.clone(),
                entity: building(json!({ "Durum": "iyi", "Kat": "2" }), 12.0),
            }]),
            &[(id.as_str(), v1.as_str())],
        ),
    )
    .await;
    assert!(matches!(moved, Err(AppError::Forbidden(m)) if m.contains("şekli")));
    let new = changes::commit(
        &db.app,
        &entry,
        envelope(
            &clerk,
            project,
            features(vec![FeatureChange::Create {
                id: Uuid::new_v4().to_string(),
                entity: building(json!({ "Durum": "iyi" }), 5.0),
            }]),
            &[],
        ),
    )
    .await;
    assert!(matches!(new, Err(AppError::Forbidden(_))));
    let edited = changes::commit(
        &db.app,
        &entry,
        envelope(
            &clerk,
            project,
            features(vec![FeatureChange::Update {
                id: id.clone(),
                entity: building(json!({ "Durum": "kotu", "Kat": "3" }), 10.0),
            }]),
            &[(id.as_str(), v1.as_str())],
        ),
    )
    .await
    .unwrap();
    assert_ne!(edited.versions[&id], v1);
    let stored: serde_json::Value =
        sqlx::query_scalar("select properties from kentos.feature where id = $1")
            .bind(Uuid::parse_str(&id).unwrap())
            .fetch_one(&db.owner)
            .await
            .unwrap();
    // What the Veri girişi does not read is kept.
    assert_eq!(
        stored,
        json!({ "Durum": "kotu", "Kat": "3", "Not": "Yalnız ofis görür." })
    );
    // The Veri girişi does not change the settings either.
    let meta_now: i64 = sqlx::query_scalar("select meta_version from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let meta_now = meta_now.to_string();
    let designed = changes::commit(
        &db.app,
        &entry,
        envelope(
            &clerk,
            project,
            ProjectChanges {
                project: Some(patch(false)),
                ..features(Vec::new())
            },
            &[("@project", meta_now.as_str())],
        ),
    )
    .await;
    assert!(matches!(designed, Err(AppError::Forbidden(_))));
}

/// docs/adr/0737 §1: a Veri girişi's `edit` from a copy someone has changed since is merged on the server over the
/// fields it reads: the hidden field and the shape the editor changed stay theirs, the Veri girişi's field is its own; the
/// merged answer comes as it reads it; the same field changed differently, a plain update and a shape change are refused
/// as before.
#[tokio::test]
async fn a_veri_girisi_edit_is_merged_over_the_fields_it_reads() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    admin::create_tenant(&db.owner, "buro", "Büro", 5)
        .await
        .unwrap();
    let pm = member(&db, "buro", "yonetici", TenantRole::ProjectManager).await;
    let editor = member(&db, "buro", "duzenleyici", TenantRole::Viewer).await;
    let clerk = member(&db, "buro", "veri", TenantRole::Viewer).await;
    let project = new_project(&db, &pm, "Hasar tespiti").await;
    let owner = open(&db, &pm, project).await;
    share(&db, &owner, &editor.actor, GrantRole::Editor).await;
    share(&db, &owner, &clerk.actor, GrantRole::Entry).await;
    let (ed, entry) = (
        open(&db, &editor, project).await,
        open(&db, &clerk, project).await,
    );
    let meta: i64 = sqlx::query_scalar("select meta_version from kentos.project where id = $1")
        .bind(project)
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let meta = meta.to_string();
    // Published, its rules not enforced on the editor: the editor writes the note the form does not show.
    let mut published = patch(true);
    if let Some(settings) = published.settings.as_mut() {
        for form in &mut settings.forms {
            form.enforce = false;
        }
    }
    changes::commit(
        &db.app,
        &owner,
        envelope(
            &pm,
            project,
            ProjectChanges {
                project: Some(published),
                ..features(Vec::new())
            },
            &[("@project", meta.as_str())],
        ),
    )
    .await
    .unwrap();
    let id = Uuid::new_v4().to_string();
    let uid = Uuid::parse_str(&id).unwrap();
    let made = changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(vec![FeatureChange::Create {
                id: id.clone(),
                entity: building(
                    json!({ "Durum": "iyi", "Kat": "2", "Not": "Yalnız ofis görür." }),
                    10.0,
                ),
            }]),
            &[],
        ),
    )
    .await
    .unwrap();
    let v1 = made.versions[&id].clone();
    // The Veri girişi's copy, as it reads it: no `Not`.
    let base = kentos_application::projects::features_by_id(&db.app, &entry, &[uid])
        .await
        .unwrap()[0]
        .entity
        .clone();
    assert!(!base.base().attrs.contains_key("Not"));
    let seen = |attrs: serde_json::Value| -> Entity {
        let mut e = base.clone();
        e.base_mut().attrs = serde_json::from_value(attrs).unwrap();
        e
    };
    // The editor widens the building and changes the hidden note.
    let edited = changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(vec![FeatureChange::Update {
                id: id.clone(),
                entity: building(
                    json!({ "Durum": "iyi", "Kat": "2", "Not": "Ofis notu 2" }),
                    14.0,
                ),
            }]),
            &[(id.as_str(), v1.as_str())],
        ),
    )
    .await
    .unwrap();
    let v2 = edited.versions[&id].clone();
    // A plain update from the old copy is still a conflict, shown as the Veri girişi reads it.
    let plain = changes::commit(
        &db.app,
        &entry,
        envelope(
            &clerk,
            project,
            features(vec![FeatureChange::Update {
                id: id.clone(),
                entity: seen(json!({ "Durum": "kotu", "Kat": "2" })),
            }]),
            &[(id.as_str(), v1.as_str())],
        ),
    )
    .await;
    match plain {
        Err(AppError::Conflict { conflicts, .. }) => {
            let current = conflicts[0].current.as_ref().expect("the server's copy");
            assert!(!current.entity.base().attrs.contains_key("Not"));
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
    // The same change as an edit from its copy: merged.
    let edit = envelope(
        &clerk,
        project,
        features(vec![FeatureChange::Edit {
            id: id.clone(),
            entity: seen(json!({ "Durum": "kotu", "Kat": "2" })),
            base: Box::new(base.clone()),
        }]),
        &[(id.as_str(), v1.as_str())],
    );
    let done = changes::commit(&db.app, &entry, edit.clone())
        .await
        .unwrap();
    let v3 = done.versions[&id].clone();
    assert_ne!(v3, v2);
    let stored: serde_json::Value =
        sqlx::query_scalar("select properties from kentos.feature where id = $1")
            .bind(uid)
            .fetch_one(&db.owner)
            .await
            .unwrap();
    assert_eq!(
        stored,
        json!({ "Durum": "kotu", "Kat": "2", "Not": "Ofis notu 2" }),
        "the Veri girişi's field, the hidden note as stored"
    );
    let shape = kentos_application::projects::features_by_id(&db.app, &ed, &[uid])
        .await
        .unwrap()[0]
        .entity
        .clone();
    assert_eq!(
        serde_json::to_value(&shape).unwrap()["pts"],
        serde_json::to_value(building(json!({}), 14.0)).unwrap()["pts"],
        "the editor's shape kept"
    );
    // The answer is the merge as the Veri girişi reads it.
    assert_eq!(done.merged.len(), 1);
    assert!(!done.merged[0].entity.base().attrs.contains_key("Not"));
    assert_eq!(
        done.merged[0]
            .entity
            .base()
            .attrs
            .get("Durum")
            .map(String::as_str),
        Some("kotu")
    );
    // The same request again: the same answer.
    let again = changes::commit(&db.app, &entry, edit).await.unwrap();
    assert!(again.replayed && again.merged == done.merged);
    // A field the Veri girişi does not read it never writes (docs/adr/0764 §2): a value of its own is
    // refused whole, never kept silently; the stored note stays.
    let base3 = kentos_application::projects::features_by_id(&db.app, &entry, &[uid])
        .await
        .unwrap()[0]
        .entity
        .clone();
    let mut written = base3.clone();
    written.base_mut().attrs =
        serde_json::from_value(json!({ "Durum": "iyi", "Kat": "2", "Not": "Veri girişinin notu" }))
            .unwrap();
    let hidden = changes::commit(
        &db.app,
        &entry,
        envelope(
            &clerk,
            project,
            features(vec![FeatureChange::Update {
                id: id.clone(),
                entity: written,
            }]),
            &[(id.as_str(), v3.as_str())],
        ),
    )
    .await;
    match hidden {
        Err(AppError::Forbidden(m)) => {
            assert!(m.contains("“Not” alanı “Çizim” katmanının"), "{m}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    let kept: serde_json::Value =
        sqlx::query_scalar("select properties from kentos.feature where id = $1")
            .bind(uid)
            .fetch_one(&db.owner)
            .await
            .unwrap();
    assert_eq!(kept["Not"], "Ofis notu 2");
    // The editor changes Kat; the Veri girişi changes it otherwise from v2's copy: a conflict.
    let base2 = kentos_application::projects::features_by_id(&db.app, &entry, &[uid])
        .await
        .unwrap()[0]
        .entity
        .clone();
    changes::commit(
        &db.app,
        &ed,
        envelope(
            &editor,
            project,
            features(vec![FeatureChange::Update {
                id: id.clone(),
                entity: building(
                    json!({ "Durum": "kotu", "Kat": "4", "Not": "Ofis notu 2" }),
                    14.0,
                ),
            }]),
            &[(id.as_str(), v3.as_str())],
        ),
    )
    .await
    .unwrap();
    let mut clash = base2.clone();
    clash.base_mut().attrs.insert("Kat".into(), "3".into());
    let refused = changes::commit(
        &db.app,
        &entry,
        envelope(
            &clerk,
            project,
            features(vec![FeatureChange::Edit {
                id: id.clone(),
                entity: clash,
                base: Box::new(base2.clone()),
            }]),
            &[(id.as_str(), v3.as_str())],
        ),
    )
    .await;
    assert!(
        matches!(&refused, Err(AppError::Conflict { conflicts, .. }) if conflicts[0].current.as_ref().is_some_and(|c| !c.entity.base().attrs.contains_key("Not"))),
        "{refused:?}"
    );
    // A shape change merges onto the stored copy but stays the Veri girişi's: refused as its write is.
    let now = kentos_application::projects::features_by_id(&db.app, &entry, &[uid])
        .await
        .unwrap()[0]
        .clone();
    let mut moved = building(json!({}), 16.0);
    moved.base_mut().attrs = base2.base().attrs.clone();
    let reshaped = changes::commit(
        &db.app,
        &entry,
        envelope(
            &clerk,
            project,
            features(vec![FeatureChange::Edit {
                id: id.clone(),
                entity: moved,
                base: Box::new(base2),
            }]),
            &[(id.as_str(), v3.as_str())],
        ),
    )
    .await;
    assert!(
        matches!(&reshaped, Err(AppError::Forbidden(m)) if m.contains("şekli")),
        "{reshaped:?}"
    );
    assert_ne!(now.version, v3, "the editor's version stands");
}
