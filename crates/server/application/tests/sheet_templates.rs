//! The sheet template library (docs/sheet/design.md §13, migration 0013),
//! with a throwaway database: the commands, their idempotency and
//! conflicts, the lists and the events; and the refusals at the level of
//! docs/adr/0024 — someone else's template answers 404 word for word as one
//! that does not exist, only the owner shares, a viewer does not write,
//! nobody raises their own role, nothing is shared outside the owner's
//! organisations. Without a database the tests are skipped, loudly
//! (`KENTOS_TEST_DB=required` makes that a failure).

mod common;

use std::collections::BTreeMap;

use common::*;
use kentos_application::sheet_templates::{self, COMMANDS};
use kentos_application::tenancy::{self, Access};
use kentos_application::{AppError, admin};
use kentos_contracts::{CommandEnvelope, CommandHost, TenantRole};
use kentos_postgres::testing::TestDb;
use kentos_postgres::{Db, Scope};
use kentos_sheet::cloud::{
    OrganizationTemplates, SheetTemplateChanged, SheetTemplateEventKind, SheetTemplateList,
    TemplateRole, catalog_entries,
};
use serde_json::{Value, json};
use uuid::Uuid;

/// The server runs exactly the commands the template catalog describes (docs/adr/0013).
#[test]
fn the_server_runs_exactly_the_template_commands_of_their_catalog() {
    let listed: Vec<(String, u32)> = catalog_entries()
        .into_iter()
        .map(|d| (d.id, d.version))
        .collect();
    let handled: Vec<(String, u32)> = COMMANDS
        .iter()
        .map(|(n, v)| ((*n).to_owned(), *v))
        .collect();
    assert_eq!(listed, handled);
    for d in catalog_entries() {
        assert_eq!(d.hosts, [CommandHost::Server], "{}", d.id);
        assert!(d.input.is_object() && d.output.is_object(), "{}", d.id);
    }
}

/// A template file of a user's: the general A4 template under another id.
fn content(name: &str) -> Value {
    let t = kentos_sheet::template::system_template("sys:genel-a4-dikey").unwrap();
    let mut v = serde_json::to_value(t).unwrap();
    v["meta"]["id"] = json!("u:deneme");
    v["meta"]["name"] = json!(name);
    v
}

fn envelope(to: &Access, command: &str, key: &str, input: Value) -> CommandEnvelope {
    CommandEnvelope {
        command_name: command.into(),
        version: 1,
        tenant_id: to.tenant.to_string(),
        project_id: String::new(),
        request_id: format!("istek-{key}"),
        idempotency_key: format!("anahtar-{key}"),
        expected_versions: BTreeMap::new(),
        input,
    }
}

async fn run(
    db: &TestDb,
    to: &Access,
    command: &str,
    key: &str,
    input: Value,
) -> Result<SheetTemplateChanged, AppError> {
    sheet_templates::run(&db.app, to, envelope(to, command, key, input)).await
}

fn code(e: &AppError) -> &'static str {
    e.code()
}

struct People {
    ayse: Access,
    bora: Access,
    dilek: Access,
    can: Access,
    ece: Access,
    buro: Access,
}

/// Büro: Ayşe (project manager), Bora (editor), Dilek (viewer); Diğer: Can; Ece belongs to no organisation.
/// Each with their personal space (`buro` is Ayşe's membership of Büro).
async fn people(db: &TestDb) -> People {
    admin::create_tenant(&db.owner, "buro", "Harita Bürosu", 6)
        .await
        .unwrap();
    admin::create_tenant(&db.owner, "diger", "Diğer", 3)
        .await
        .unwrap();
    let buro = member(db, "buro", "ayse", TenantRole::ProjectManager).await;
    let bora = member(db, "buro", "bora", TenantRole::Editor).await;
    let dilek = member(db, "buro", "dilek", TenantRole::Viewer).await;
    let can = member(db, "diger", "can", TenantRole::Owner).await;
    let ece = account(db, "ece").await;
    People {
        ayse: personal(db, &buro.actor).await,
        bora: personal(db, &bora.actor).await,
        dilek: personal(db, &dilek.actor).await,
        can: personal(db, &can.actor).await,
        ece: personal(db, &ece).await,
        buro,
    }
}

#[tokio::test]
async fn a_template_is_saved_listed_read_and_revised() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let p = people(&db).await;
    let made = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "1",
        json!({ "content": content("Büro paftası") }),
    )
    .await
    .unwrap();
    assert_eq!(
        (made.revision, made.changed, made.replayed),
        (1, true, false)
    );
    let id: Uuid = made.template_id.parse().unwrap();
    // A retry gets the stored answer; the key for another request is refused.
    let again = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "1",
        json!({ "content": content("Büro paftası") }),
    )
    .await
    .unwrap();
    assert_eq!(
        (again.template_id.as_str(), again.replayed),
        (made.template_id.as_str(), true)
    );
    let other = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "1",
        json!({ "content": content("Başka") }),
    )
    .await
    .unwrap_err();
    assert_eq!(code(&other), "invalid");

    let list = sheet_templates::mine(&db.app, &p.ayse.actor).await.unwrap();
    assert_eq!(list.templates.len(), 1);
    let s = &list.templates[0];
    assert_eq!(
        (s.name.as_str(), s.role, s.revision, s.shared),
        ("Büro paftası", TemplateRole::Owner, 1, false)
    );
    assert_eq!(s.owner_name, "ayse");
    assert_eq!(s.papers[0].paper, kentos_sheet::Paper::A4);
    assert_eq!(s.sha256.len(), 64);
    let d = sheet_templates::detail(&db.app, &p.ayse.actor, id)
        .await
        .unwrap();
    assert_eq!(
        (d.content.meta.id.as_str(), d.content.meta.revision),
        (made.template_id.as_str(), 1)
    );

    // A new revision on the one it is based on; a second on the same base is a conflict and writes nothing.
    let v2 = run(
        &db,
        &p.ayse,
        "sheet.template.update",
        "2",
        json!({ "templateId": id, "expectedRevision": 1, "content": content("Büro paftası v2") }),
    )
    .await
    .unwrap();
    assert_eq!(v2.revision, 2);
    let stale = run(
        &db,
        &p.ayse,
        "sheet.template.update",
        "3",
        json!({ "templateId": id, "expectedRevision": 1, "content": content("Eski") }),
    )
    .await
    .unwrap_err();
    match stale {
        AppError::Conflict { revision, .. } => assert_eq!(revision, Some(2)),
        e => panic!("409 bekleniyordu: {e:?}"),
    }
    let d = sheet_templates::detail(&db.app, &p.ayse.actor, id)
        .await
        .unwrap();
    assert_eq!(
        (d.summary.name.as_str(), d.summary.revision),
        ("Büro paftası v2", 2)
    );

    let events = sheet_templates::events_after(&db.app, &p.ayse.actor, 0, 100)
        .await
        .unwrap();
    assert_eq!(
        events
            .events
            .iter()
            .map(|e| (e.kind, e.revision))
            .collect::<Vec<_>>(),
        [
            (SheetTemplateEventKind::Created, 1),
            (SheetTemplateEventKind::Updated, 2)
        ]
    );
    let after =
        sheet_templates::events_after(&db.app, &p.ayse.actor, events.next.parse().unwrap(), 100)
            .await
            .unwrap();
    assert!(after.events.is_empty());
    assert_eq!(after.next, events.next);
    db.close().await;
}

#[tokio::test]
async fn sharing_follows_the_rules_of_adr_0024() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let p = people(&db).await;
    let id: Uuid = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "1",
        json!({ "content": content("Ada paftası") }),
    )
    .await
    .unwrap()
    .template_id
    .parse()
    .unwrap();
    let ids = |who: &str| {
        let owner = db.owner.clone();
        let who = who.to_owned();
        async move { admin::user_id(&owner, &who).await.unwrap() }
    };
    let (bora, dilek, can, ece, ayse) = (
        ids("bora").await,
        ids("dilek").await,
        ids("can").await,
        ids("ece").await,
        ids("ayse").await,
    );
    let share = |user: Uuid, role: &str| json!({ "templateId": id, "userId": user, "role": role });

    // Before it is shared, Bora sees nothing of it, word for word as a template that does not exist.
    let hidden = sheet_templates::detail(&db.app, &p.bora.actor, id)
        .await
        .unwrap_err();
    let never = sheet_templates::detail(&db.app, &p.bora.actor, Uuid::now_v7())
        .await
        .unwrap_err();
    assert_eq!(
        (hidden.code(), hidden.to_string()),
        (never.code(), never.to_string())
    );
    assert_eq!(hidden.code(), "not_found");

    // The owner shares with Büro's people.
    let s = run(
        &db,
        &p.ayse,
        "sheet.template.share",
        "2",
        share(bora, "editor"),
    )
    .await
    .unwrap();
    assert!(s.changed);
    let same = run(
        &db,
        &p.ayse,
        "sheet.template.share",
        "3",
        share(bora, "editor"),
    )
    .await
    .unwrap();
    assert!(!same.changed, "the same role again changes nothing");
    run(
        &db,
        &p.ayse,
        "sheet.template.share",
        "4",
        share(dilek, "viewer"),
    )
    .await
    .unwrap();
    let mine = sheet_templates::mine(&db.app, &p.bora.actor).await.unwrap();
    assert_eq!(
        mine.templates
            .iter()
            .map(|t| (t.role, t.owner_name.as_str()))
            .collect::<Vec<_>>(),
        [(TemplateRole::Editor, "ayse")]
    );
    assert!(
        sheet_templates::mine(&db.app, &p.ayse.actor)
            .await
            .unwrap()
            .templates[0]
            .shared
    );

    // An editor saves a new revision; a viewer does not write; only the owner shares, unshares and deletes.
    let v2 = run(&db, &p.bora, "sheet.template.update", "5", json!({ "templateId": id, "expectedRevision": 1, "content": content("Ada paftası (Bora)") }))
        .await
        .unwrap();
    assert_eq!(v2.revision, 2);
    let viewer = run(
        &db,
        &p.dilek,
        "sheet.template.update",
        "6",
        json!({ "templateId": id, "expectedRevision": 2, "content": content("Dilek") }),
    )
    .await
    .unwrap_err();
    assert_eq!(viewer.code(), "forbidden");
    for (who, cmd, input) in [
        (&p.bora, "sheet.template.share", share(dilek, "editor")),
        // Nobody raises their own role.
        (&p.bora, "sheet.template.share", share(bora, "editor")),
        (&p.dilek, "sheet.template.share", share(dilek, "editor")),
        (
            &p.bora,
            "sheet.template.unshare",
            json!({ "templateId": id, "userId": dilek }),
        ),
        (
            &p.bora,
            "sheet.template.delete",
            json!({ "templateId": id }),
        ),
    ] {
        let e = run(&db, who, cmd, &Uuid::now_v7().to_string(), input)
            .await
            .unwrap_err();
        assert_eq!(e.code(), "forbidden", "{cmd}");
    }
    let own = run(
        &db,
        &p.ayse,
        "sheet.template.share",
        "7",
        share(ayse, "editor"),
    )
    .await
    .unwrap_err();
    assert_eq!(own.code(), "invalid");

    // Nothing goes outside the owner's organisations; an unknown account reads the same.
    let outside = run(
        &db,
        &p.ayse,
        "sheet.template.share",
        "8",
        share(can, "viewer"),
    )
    .await
    .unwrap_err();
    let nobody = run(
        &db,
        &p.ayse,
        "sheet.template.share",
        "9",
        share(Uuid::now_v7(), "viewer"),
    )
    .await
    .unwrap_err();
    let alone = run(
        &db,
        &p.ayse,
        "sheet.template.share",
        "10",
        share(ece, "viewer"),
    )
    .await
    .unwrap_err();
    assert_eq!(outside.code(), "invalid");
    assert_eq!(
        (outside.to_string(), outside.path()),
        (nobody.to_string(), nobody.path())
    );
    assert_eq!(outside.to_string(), alone.to_string());

    // The sharing and its people are the owner's; others who see it are refused, anyone else gets 404.
    let access = sheet_templates::access_list(&db.app, &p.ayse.actor, id)
        .await
        .unwrap();
    assert_eq!(
        access
            .grants
            .iter()
            .map(|g| (g.display_name.as_str(), g.role))
            .collect::<Vec<_>>(),
        [
            ("bora", kentos_sheet::cloud::TemplateGrantRole::Editor),
            ("dilek", kentos_sheet::cloud::TemplateGrantRole::Viewer)
        ]
    );
    assert_eq!(
        sheet_templates::access_list(&db.app, &p.bora.actor, id)
            .await
            .unwrap_err()
            .code(),
        "forbidden"
    );
    assert_eq!(
        sheet_templates::access_list(&db.app, &p.can.actor, id)
            .await
            .unwrap_err()
            .code(),
        "not_found"
    );
    let found = sheet_templates::candidates(&db.app, &p.ayse.actor, id, "bo")
        .await
        .unwrap();
    assert_eq!(
        found
            .candidates
            .iter()
            .map(|c| c.display_name.as_str())
            .collect::<Vec<_>>(),
        ["bora"]
    );
    assert!(
        sheet_templates::candidates(&db.app, &p.ayse.actor, id, "can")
            .await
            .unwrap()
            .candidates
            .is_empty()
    );
    assert_eq!(
        sheet_templates::candidates(&db.app, &p.dilek.actor, id, "bo")
            .await
            .unwrap_err()
            .code(),
        "forbidden"
    );
    assert_eq!(
        sheet_templates::candidates(&db.app, &p.can.actor, id, "bo")
            .await
            .unwrap_err()
            .code(),
        "not_found"
    );
    assert_eq!(
        sheet_templates::candidates(&db.app, &p.ayse.actor, id, "b")
            .await
            .unwrap_err()
            .code(),
        "invalid"
    );

    // Taking a share away: it leaves Dilek's list, and Dilek hears of it.
    let gone = run(
        &db,
        &p.ayse,
        "sheet.template.unshare",
        "11",
        json!({ "templateId": id, "userId": dilek }),
    )
    .await
    .unwrap();
    assert!(gone.changed);
    assert!(
        sheet_templates::mine(&db.app, &p.dilek.actor)
            .await
            .unwrap()
            .templates
            .is_empty()
    );
    let heard = sheet_templates::events_after(&db.app, &p.dilek.actor, 0, 100)
        .await
        .unwrap();
    assert_eq!(
        heard.events.iter().map(|e| e.kind).collect::<Vec<_>>(),
        [
            SheetTemplateEventKind::Shared,
            SheetTemplateEventKind::Updated,
            SheetTemplateEventKind::Unshared
        ]
    );
    // Bora heard of the second share and of his own revision too; Can of nothing.
    let bora_heard = sheet_templates::events_after(&db.app, &p.bora.actor, 0, 100)
        .await
        .unwrap();
    assert_eq!(bora_heard.events.len(), 4, "{:?}", bora_heard.events);
    assert!(
        sheet_templates::events_after(&db.app, &p.can.actor, 0, 100)
            .await
            .unwrap()
            .events
            .is_empty()
    );

    // The rows themselves: the server's role in Can's scope reads none of them.
    let mut tx = db
        .app
        .scoped(kentos_postgres::Scope {
            tenant: None,
            user: Some(can),
            project: None,
        })
        .await
        .unwrap();
    for table in [
        "sheet_template",
        "sheet_template_revision",
        "sheet_template_grant",
        "sheet_template_event",
    ] {
        let n: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "select count(*) from kentos.{table}"
        )))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(n, 0, "{table}");
    }
    tx.commit().await.unwrap();
    let _ = &p.ece;
    db.close().await;
}

#[tokio::test]
async fn a_deleted_template_leaves_every_list() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let p = people(&db).await;
    let id: Uuid = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "1",
        json!({ "content": content("Silinecek") }),
    )
    .await
    .unwrap()
    .template_id
    .parse()
    .unwrap();
    let bora = admin::user_id(&db.owner, "bora").await.unwrap();
    run(
        &db,
        &p.ayse,
        "sheet.template.share",
        "2",
        json!({ "templateId": id, "userId": bora, "role": "viewer" }),
    )
    .await
    .unwrap();
    // Changed after the device's copy: not deleted.
    run(
        &db,
        &p.ayse,
        "sheet.template.update",
        "3",
        json!({ "templateId": id, "expectedRevision": 1, "content": content("Silinecek 2") }),
    )
    .await
    .unwrap();
    let stale = run(
        &db,
        &p.ayse,
        "sheet.template.delete",
        "4",
        json!({ "templateId": id, "expectedRevision": 1 }),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(
            stale,
            AppError::Conflict {
                revision: Some(2),
                ..
            }
        ),
        "{stale:?}"
    );
    let done = run(
        &db,
        &p.ayse,
        "sheet.template.delete",
        "5",
        json!({ "templateId": id, "expectedRevision": 2 }),
    )
    .await
    .unwrap();
    assert!(done.deleted && done.changed);
    for who in [&p.ayse, &p.bora] {
        assert!(
            sheet_templates::mine(&db.app, &who.actor)
                .await
                .unwrap()
                .templates
                .is_empty()
        );
        assert_eq!(
            sheet_templates::detail(&db.app, &who.actor, id)
                .await
                .unwrap_err()
                .code(),
            "not_found"
        );
    }
    let again = run(
        &db,
        &p.ayse,
        "sheet.template.update",
        "6",
        json!({ "templateId": id, "expectedRevision": 2, "content": content("X") }),
    )
    .await
    .unwrap_err();
    assert_eq!(again.code(), "not_found");
    let heard = sheet_templates::events_after(&db.app, &p.bora.actor, 0, 100)
        .await
        .unwrap();
    assert_eq!(
        heard.events.last().map(|e| e.kind),
        Some(SheetTemplateEventKind::Deleted)
    );
    db.close().await;
}

#[tokio::test]
async fn what_the_library_refuses() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let p = people(&db).await;
    // The file is read as a device reads it.
    let mut v = content("Bilinmeyen alan");
    v["meta"]["zoom"] = json!(2);
    let e = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "2",
        json!({ "content": v }),
    )
    .await
    .unwrap_err();
    assert!(e.to_string().contains("bad_json"), "{e}");
    let mut v = content("Sistem kimliği");
    v["meta"]["id"] = json!("sys:benim");
    let e = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "3",
        json!({ "content": v }),
    )
    .await
    .unwrap_err();
    assert!(e.to_string().contains("reserved_id"), "{e}");
    let e = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "4",
        json!({ "content": content(&"a".repeat(121)) }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "invalid");
    // Content over 8 MB, its pictures included.
    // Two pictures of 3.5 MB (each under the 4 MB of one, together under the 8 MB of a template's): in base64 over 8 MB.
    let mut v = content("Büyük resimli");
    let mut assets = Vec::new();
    for k in 0..2u32 {
        let bytes: Vec<u8> = (0..3_500_000u32)
            .map(|i| ((i ^ k).wrapping_mul(2_654_435_761) >> 24) as u8)
            .collect();
        let sha = kentos_sheet::template::sha256_hex(&bytes);
        assets.push(json!({ "meta": { "sha256": sha, "kind": "png", "name": format!("{k}.png"), "width": 10, "height": 10, "bytes": bytes.len() },
                            "data": kentos_sheet::template::base64_encode(&bytes) }));
        v["sheet"]["items"].as_array_mut().unwrap().push(json!({ "id": format!("resim-{k}"), "name": format!("Resim {k}"),
            "frame": { "left": 30000, "top": 30000, "width": 20000, "height": 20000 }, "kind": { "type": "picture", "asset": sha } }));
    }
    v["assets"] = Value::Array(assets);
    let e = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "5",
        json!({ "content": v }),
    )
    .await
    .unwrap_err();
    assert!(e.to_string().contains("8 MB"), "{e}");
    // At most 500 a person; two creations at once at 499 do not both pass.
    let ayse = p.ayse.actor.user_id;
    for _ in 0..499 {
        sqlx::query(
            "insert into kentos.sheet_template (id, tenant_id, owner_user_id, name, revision, sha256, size) values ($1, $2, $3, 'x', 1, $4, 1)",
        )
        .bind(Uuid::now_v7())
        .bind(p.ayse.tenant)
        .bind(ayse)
        .bind(vec![0u8; 32])
        .execute(&db.owner)
        .await
        .unwrap();
    }
    let (a, b) = tokio::join!(
        run(
            &db,
            &p.ayse,
            "sheet.template.create",
            "6a",
            json!({ "content": content("Beş yüzüncü") })
        ),
        run(
            &db,
            &p.ayse,
            "sheet.template.create",
            "6b",
            json!({ "content": content("Beş yüz birinci") })
        ),
    );
    assert_eq!(u8::from(a.is_ok()) + u8::from(b.is_ok()), 1, "{a:?} {b:?}");
    let e = a.err().or(b.err()).unwrap();
    assert!(e.to_string().contains("500"), "{e}");
    // Unknown and wrong versions, a key too short.
    let mut env = envelope(
        &p.ece,
        "sheet.template.create",
        "7",
        json!({ "content": content("Sürüm") }),
    );
    env.version = 2;
    assert_eq!(
        sheet_templates::run(&db.app, &p.ece, env)
            .await
            .unwrap_err()
            .code(),
        "invalid"
    );
    let mut env = envelope(
        &p.ece,
        "sheet.template.create",
        "8",
        json!({ "content": content("Kısa anahtar") }),
    );
    env.idempotency_key = "kisa".into();
    assert_eq!(
        sheet_templates::run(&db.app, &p.ece, env)
            .await
            .unwrap_err()
            .code(),
        "invalid"
    );
    let _ = (&p.bora, &p.dilek, &p.can);
    db.close().await;
}

// ── An organisation's library (design §13 “Kurum şablonları”, migration 0014) ──

/// Büro as `people` makes it, with its administrator Emre, Hakan (a project manager without a
/// seat) and Fatma, a guest of one of Büro's projects (a grant, no membership).
struct Organisation {
    p: People,
    emre: Access,
    hakan: kentos_application::identity::Actor,
    fatma: kentos_application::identity::Actor,
    buro_id: Uuid,
    diger: Access,
}

async fn organisation(db: &TestDb) -> Organisation {
    let p = people(db).await;
    let emre = member(db, "buro", "emre", TenantRole::Admin).await;
    let hakan = account(db, "hakan").await;
    admin::set_membership(
        &db.owner,
        "buro",
        "hakan",
        TenantRole::ProjectManager,
        false,
    )
    .await
    .unwrap();
    let fatma = account(db, "fatma").await;
    let project = new_project(db, &p.buro, "Büronun projesi").await;
    sqlx::query(
        "insert into kentos.project_grant (tenant_id, project_id, user_id, role, granted_by, guest) values ($1, $2, $3, 'viewer', $4, true)",
    )
    .bind(p.buro.tenant)
    .bind(project)
    .bind(fatma.user_id)
    .bind(p.buro.actor.user_id)
    .execute(&db.owner)
    .await
    .unwrap();
    let buro_id = p.buro.tenant;
    let can = p.can.actor.clone();
    let diger_id: Uuid = sqlx::query_scalar("select id from kentos.tenant where slug = 'diger'")
        .fetch_one(&db.owner)
        .await
        .unwrap();
    let diger = tenancy::access(&db.app, &can, diger_id).await.unwrap();
    Organisation {
        p,
        emre,
        hakan,
        fatma,
        buro_id,
        diger,
    }
}

/// `who`'s membership of Büro, for its command route.
async fn in_buro(db: &TestDb, who: &Access, buro: Uuid) -> Access {
    tenancy::access(&db.app, &who.actor, buro).await.unwrap()
}

async fn list(db: &TestDb, who: &kentos_application::identity::Actor) -> SheetTemplateList {
    sheet_templates::mine(&db.app, who).await.unwrap()
}

fn library<'a>(l: &'a SheetTemplateList, name: &str) -> Option<&'a OrganizationTemplates> {
    l.organizations.iter().find(|o| o.name == name)
}

/// The templates of Büro's library `who` sees, with their role.
async fn buro_roles(
    db: &TestDb,
    who: &kentos_application::identity::Actor,
) -> Vec<(String, TemplateRole)> {
    let l = list(db, who).await;
    library(&l, "Harita Bürosu")
        .map(|o| {
            o.templates
                .iter()
                .map(|t| (t.name.clone(), t.role))
                .collect()
        })
        .unwrap_or_default()
}

async fn heard(
    db: &TestDb,
    who: &kentos_application::identity::Actor,
) -> Vec<(Uuid, SheetTemplateEventKind)> {
    sheet_templates::events_after(&db.app, who, 0, 500)
        .await
        .unwrap()
        .events
        .into_iter()
        .map(|e| (e.template_id.parse().unwrap(), e.kind))
        .collect()
}

#[tokio::test]
async fn an_organisation_s_library_is_published_used_and_edited_by_its_rules() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let o = organisation(&db).await;
    let p = &o.p;
    let emre = &o.emre;

    // Every active member with a seat sees Büro's library, empty yet; those who may create
    // projects there may publish into it.
    for (who, can) in [
        (&p.ayse.actor, true),
        (&emre.actor, true),
        (&p.bora.actor, false),
        (&p.dilek.actor, false),
    ] {
        let l = list(&db, who).await;
        let b = library(&l, "Harita Bürosu").expect("Büro's library");
        assert_eq!(
            (b.tenant_id.clone(), b.can_publish, b.templates.len()),
            (o.buro_id.to_string(), can, 0)
        );
    }

    // Ayşe publishes her own template: a template of Büro's of its own, its source kept.
    let own: Uuid = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "1",
        json!({ "content": content("Belediye ifraz paftası") }),
    )
    .await
    .unwrap()
    .template_id
    .parse()
    .unwrap();
    run(&db, &p.ayse, "sheet.template.update", "2", json!({ "templateId": own, "expectedRevision": 1, "content": content("Belediye ifraz paftası") }))
        .await
        .unwrap();
    let publish = json!({ "templateId": own, "tenantId": o.buro_id });
    let made = run(&db, &p.buro, "sheet.template.publish", "3", publish.clone())
        .await
        .unwrap();
    assert_eq!(
        (made.revision, made.changed, made.replayed),
        (1, true, false)
    );
    let org_id: Uuid = made.template_id.parse().unwrap();
    assert_ne!(org_id, own);
    // A retry is the same request: the same template, not a second one.
    let again = run(&db, &p.buro, "sheet.template.publish", "3", publish)
        .await
        .unwrap();
    assert_eq!(
        (again.template_id.as_str(), again.replayed),
        (made.template_id.as_str(), true)
    );

    let d = sheet_templates::detail(&db.app, &p.bora.actor, org_id)
        .await
        .unwrap();
    assert_eq!(
        d.summary.published_from.as_deref(),
        Some(own.to_string().as_str())
    );
    assert_eq!(
        d.summary
            .organization
            .as_ref()
            .map(|x| (x.tenant_id.as_str(), x.name.as_str())),
        Some((o.buro_id.to_string().as_str(), "Harita Bürosu"))
    );
    // Its publisher is named to the members who see it.
    assert_eq!(
        (d.summary.role, d.summary.owner_name.as_str()),
        (TemplateRole::Viewer, "ayse")
    );
    assert_eq!(
        (d.content.meta.id.clone(), d.content.meta.revision),
        (org_id.to_string(), 1)
    );
    assert_eq!(d.content.meta.name, "Belediye ifraz paftası");
    let source = sheet_templates::detail(&db.app, &p.ayse.actor, own)
        .await
        .unwrap();
    assert_eq!(
        source.content.sheet, d.content.sheet,
        "the newest revision's content"
    );

    // The publisher's own list keeps her template among hers, and Büro's copy in Büro's.
    let l = list(&db, &p.ayse.actor).await;
    assert_eq!(
        l.templates.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
        [own.to_string()]
    );
    assert_eq!(
        buro_roles(&db, &p.ayse.actor).await,
        [("Belediye ifraz paftası".to_owned(), TemplateRole::Owner)]
    );
    assert_eq!(
        buro_roles(&db, &emre.actor).await,
        [("Belediye ifraz paftası".to_owned(), TemplateRole::Admin)]
    );
    assert_eq!(
        buro_roles(&db, &p.bora.actor).await,
        [("Belediye ifraz paftası".to_owned(), TemplateRole::Viewer)]
    );
    assert_eq!(
        buro_roles(&db, &p.dilek.actor).await,
        [("Belediye ifraz paftası".to_owned(), TemplateRole::Viewer)]
    );
    assert!(
        list(&db, &p.bora.actor).await.templates.is_empty(),
        "nothing of Büro's among Bora's own"
    );

    // Every member heard of it; nobody outside did.
    for who in [&p.ayse.actor, &emre.actor, &p.bora.actor, &p.dilek.actor] {
        assert!(
            heard(&db, who)
                .await
                .contains(&(org_id, SheetTemplateEventKind::Created)),
            "{}",
            who.user_id
        );
    }
    for who in [&p.can.actor, &p.ece.actor, &o.fatma, &o.hakan] {
        assert!(!heard(&db, who).await.iter().any(|(t, _)| *t == org_id));
    }

    // An administrator edits it (on Büro's route), and everyone hears it.
    let v2 = run(&db, emre, "sheet.template.update", "4", json!({ "templateId": org_id, "expectedRevision": 1, "content": content("Belediye ifraz paftası") }))
        .await
        .unwrap();
    assert_eq!(v2.revision, 2);
    assert!(
        heard(&db, &p.dilek.actor)
            .await
            .contains(&(org_id, SheetTemplateEventKind::Updated))
    );
    // The publisher too, and a template is made in Büro directly by one who may publish there.
    run(&db, &p.buro, "sheet.template.update", "5", json!({ "templateId": org_id, "expectedRevision": 2, "content": content("Belediye ifraz paftası") }))
        .await
        .unwrap();
    let direct: Uuid = run(
        &db,
        emre,
        "sheet.template.create",
        "6",
        json!({ "content": content("Kurum antedi") }),
    )
    .await
    .unwrap()
    .template_id
    .parse()
    .unwrap();
    let d = sheet_templates::detail(&db.app, &p.bora.actor, direct)
        .await
        .unwrap();
    assert_eq!(
        (d.summary.role, d.summary.published_from.clone()),
        (TemplateRole::Viewer, None)
    );
    // Ayşe publishes but administers nothing: Emre's template she only uses.
    assert_eq!(
        buro_roles(&db, &p.ayse.actor).await,
        [
            ("Belediye ifraz paftası".to_owned(), TemplateRole::Owner),
            ("Kurum antedi".to_owned(), TemplateRole::Viewer),
        ]
    );
    let e = run(
        &db,
        &p.buro,
        "sheet.template.update",
        "6b",
        json!({ "templateId": direct, "expectedRevision": 1, "content": content("Kurum antedi") }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "forbidden");

    // The publisher deletes hers; an administrator deletes the other: gone from every list, heard by all.
    run(
        &db,
        &p.buro,
        "sheet.template.delete",
        "7",
        json!({ "templateId": org_id }),
    )
    .await
    .unwrap();
    run(
        &db,
        emre,
        "sheet.template.delete",
        "8",
        json!({ "templateId": direct, "expectedRevision": 1 }),
    )
    .await
    .unwrap();
    assert!(buro_roles(&db, &p.bora.actor).await.is_empty());
    assert!(
        heard(&db, &p.bora.actor)
            .await
            .contains(&(org_id, SheetTemplateEventKind::Deleted))
    );
    assert!(
        heard(&db, &p.bora.actor)
            .await
            .contains(&(direct, SheetTemplateEventKind::Deleted))
    );
    // Her own template is untouched.
    assert_eq!(
        sheet_templates::detail(&db.app, &p.ayse.actor, own)
            .await
            .unwrap()
            .summary
            .revision,
        2
    );
    db.close().await;
}

#[tokio::test]
async fn an_organisation_s_library_refuses_at_the_level_of_adr_0024() {
    let Some(db) = TestDb::create().await else {
        return;
    };
    let o = organisation(&db).await;
    let p = &o.p;
    let bora_in_buro = in_buro(&db, &p.bora, o.buro_id).await;
    let dilek_in_buro = in_buro(&db, &p.dilek, o.buro_id).await;
    let own: Uuid = run(
        &db,
        &p.ayse,
        "sheet.template.create",
        "1",
        json!({ "content": content("Ada paftası") }),
    )
    .await
    .unwrap()
    .template_id
    .parse()
    .unwrap();
    let org_id: Uuid = run(
        &db,
        &p.buro,
        "sheet.template.publish",
        "2",
        json!({ "templateId": own, "tenantId": o.buro_id }),
    )
    .await
    .unwrap()
    .template_id
    .parse()
    .unwrap();
    let missing = sheet_templates::detail(&db.app, &p.bora.actor, Uuid::now_v7())
        .await
        .unwrap_err();

    // Outside Büro (Ece, Can of another organisation, the guest Fatma): its route answers 404 as
    // for an organisation one is not in, and its template 404 word for word as one that does not exist.
    for who in [&p.ece.actor, &p.can.actor, &o.fatma] {
        assert_eq!(
            tenancy::access(&db.app, who, o.buro_id)
                .await
                .unwrap_err()
                .code(),
            "not_found"
        );
        let e = sheet_templates::detail(&db.app, who, org_id)
            .await
            .unwrap_err();
        assert_eq!(
            (e.code(), e.to_string()),
            (missing.code(), missing.to_string())
        );
        let l = list(&db, who).await;
        assert!(
            library(&l, "Harita Bürosu").is_none()
                && l.templates.iter().all(|t| t.id != org_id.to_string())
        );
    }
    // A member without a seat: the route refuses (403), the library is not theirs to see.
    assert_eq!(
        tenancy::access(&db.app, &o.hakan, o.buro_id)
            .await
            .unwrap_err()
            .code(),
        "forbidden"
    );
    assert_eq!(
        sheet_templates::detail(&db.app, &o.hakan, org_id)
            .await
            .unwrap_err()
            .code(),
        "not_found"
    );
    assert!(library(&list(&db, &o.hakan).await, "Harita Bürosu").is_none());

    // A member who may not publish: no publishing, no creating there, no editing, no deleting (403).
    let bora_own: Uuid = run(
        &db,
        &p.bora,
        "sheet.template.create",
        "3",
        json!({ "content": content("Bora'nın paftası") }),
    )
    .await
    .unwrap()
    .template_id
    .parse()
    .unwrap();
    for (who, key) in [(&bora_in_buro, "4"), (&dilek_in_buro, "5")] {
        let e = run(
            &db,
            who,
            "sheet.template.publish",
            key,
            json!({ "templateId": bora_own, "tenantId": o.buro_id }),
        )
        .await
        .unwrap_err();
        assert_eq!(e.code(), "forbidden", "{e}");
    }
    let e = run(
        &db,
        &bora_in_buro,
        "sheet.template.create",
        "6",
        json!({ "content": content("Kurumda") }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "forbidden");
    let e = run(
        &db,
        &bora_in_buro,
        "sheet.template.update",
        "7",
        json!({ "templateId": org_id, "expectedRevision": 1, "content": content("Ada paftası") }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "forbidden");
    assert!(e.to_string().contains("Şablonlarıma kopyala"), "{e}");
    let e = run(
        &db,
        &bora_in_buro,
        "sheet.template.delete",
        "8",
        json!({ "templateId": org_id }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "forbidden");

    // Publishing: one's own template only (not one shared with one, not an organisation's), into
    // the organisation of the route, through an organisation's route; someone else's is not seen.
    run(
        &db,
        &p.bora,
        "sheet.template.share",
        "9",
        json!({ "templateId": bora_own, "userId": p.ayse.actor.user_id, "role": "editor" }),
    )
    .await
    .unwrap();
    let e = run(
        &db,
        &p.buro,
        "sheet.template.publish",
        "10",
        json!({ "templateId": bora_own, "tenantId": o.buro_id }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "forbidden", "a template shared with her");
    let e = run(
        &db,
        &p.buro,
        "sheet.template.publish",
        "11",
        json!({ "templateId": org_id, "tenantId": o.buro_id }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "forbidden", "an organisation's");
    let dilek_own: Uuid = run(
        &db,
        &p.dilek,
        "sheet.template.create",
        "12",
        json!({ "content": content("Dilek'in") }),
    )
    .await
    .unwrap()
    .template_id
    .parse()
    .unwrap();
    let e = run(
        &db,
        &p.buro,
        "sheet.template.publish",
        "13",
        json!({ "templateId": dilek_own, "tenantId": o.buro_id }),
    )
    .await
    .unwrap_err();
    assert_eq!(
        (e.code(), e.to_string()),
        (missing.code(), missing.to_string())
    );
    let e = run(
        &db,
        &p.ayse,
        "sheet.template.publish",
        "14",
        json!({ "templateId": own, "tenantId": o.buro_id }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "invalid", "the personal space's route");
    let e = run(
        &db,
        &p.buro,
        "sheet.template.publish",
        "15",
        json!({ "templateId": own, "tenantId": o.diger.tenant }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "invalid", "another organisation than the route's");
    // Into another organisation she is not in: its route is not hers.
    assert_eq!(
        tenancy::access(&db.app, &p.ayse.actor, o.diger.tenant)
            .await
            .unwrap_err()
            .code(),
        "not_found"
    );

    // A command goes to its template's space: Büro's template on Büro's route, hers on her personal one.
    let e = run(
        &db,
        &p.ayse,
        "sheet.template.update",
        "16",
        json!({ "templateId": org_id, "expectedRevision": 1, "content": content("Ada paftası") }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "invalid");
    let e = run(
        &db,
        &p.buro,
        "sheet.template.update",
        "17",
        json!({ "templateId": own, "expectedRevision": 1, "content": content("Ada paftası") }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "invalid");
    // Not shared one by one, and it has no sharing to read.
    let e = run(
        &db,
        &p.buro,
        "sheet.template.share",
        "18",
        json!({ "templateId": org_id, "userId": p.bora.actor.user_id, "role": "editor" }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "invalid");
    assert_eq!(
        sheet_templates::access_list(&db.app, &p.ayse.actor, org_id)
            .await
            .unwrap_err()
            .code(),
        "invalid"
    );

    // Row-level security: in another organisation's scope, and in Can's, nothing of Büro's library.
    let sees = |scope: Scope| {
        let app: Db = db.app.clone();
        async move {
            let mut tx = app.scoped(scope).await.unwrap();
            let n: i64 = sqlx::query_scalar(
                "select (select count(*) from kentos.sheet_template where tenant_id = $1)
                      + (select count(*) from kentos.sheet_template_revision r join kentos.sheet_template t on t.id = r.template_id where t.tenant_id = $1)",
            )
            .bind(o.buro_id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            tx.commit().await.unwrap();
            n
        }
    };
    assert_eq!(sees(o.diger.scope()).await, 0);
    assert_eq!(
        sees(Scope {
            tenant: Some(o.buro_id),
            user: Some(p.can.actor.user_id),
            project: None
        })
        .await,
        0
    );
    assert_eq!(
        sees(Scope {
            tenant: Some(o.buro_id),
            user: Some(o.fatma.user_id),
            project: None
        })
        .await,
        0
    );
    assert!(sees(bora_in_buro.scope()).await > 0, "a member sees it");
    // Nor can a row be written there by hand by one who may not publish.
    let mut tx = db.app.scoped(bora_in_buro.scope()).await.unwrap();
    let forced = sqlx::query(
        "insert into kentos.sheet_template (id, tenant_id, owner_user_id, name, revision, sha256, size) values ($1, $2, $3, 'el yapımı', 1, $4, 1)",
    )
    .bind(Uuid::now_v7())
    .bind(o.buro_id)
    .bind(p.bora.actor.user_id)
    .bind(vec![0u8; 32])
    .execute(&mut *tx)
    .await;
    assert!(forced.is_err(), "row-level security refuses it");
    drop(tx);

    // The publisher who may no longer publish only uses it (and cannot publish).
    admin::set_membership(&db.owner, "buro", "ayse", TenantRole::Editor, true)
        .await
        .unwrap();
    assert_eq!(
        buro_roles(&db, &p.ayse.actor).await,
        [("Ada paftası".to_owned(), TemplateRole::Viewer)]
    );
    let e = run(
        &db,
        &p.buro,
        "sheet.template.update",
        "19",
        json!({ "templateId": org_id, "expectedRevision": 1, "content": content("Ada paftası") }),
    )
    .await
    .unwrap_err();
    assert_eq!(e.code(), "forbidden");
    assert!(
        !library(&list(&db, &p.ayse.actor).await, "Harita Bürosu")
            .unwrap()
            .can_publish
    );

    // Leaving: a membership turned off, a seat taken back, a membership gone: no access, no events.
    let seen_before = heard(&db, &p.bora.actor).await.len();
    sqlx::query("update kentos.membership set status = 'disabled' where user_id = $1")
        .bind(p.bora.actor.user_id)
        .execute(&db.owner)
        .await
        .unwrap();
    assert_eq!(
        sheet_templates::detail(&db.app, &p.bora.actor, org_id)
            .await
            .unwrap_err()
            .code(),
        "not_found"
    );
    assert!(library(&list(&db, &p.bora.actor).await, "Harita Bürosu").is_none());
    sqlx::query("delete from kentos.seat_allocation where user_id = $1")
        .bind(p.dilek.actor.user_id)
        .execute(&db.owner)
        .await
        .unwrap();
    assert_eq!(
        sheet_templates::detail(&db.app, &p.dilek.actor, org_id)
            .await
            .unwrap_err()
            .code(),
        "not_found"
    );
    assert!(library(&list(&db, &p.dilek.actor).await, "Harita Bürosu").is_none());
    // A change after that is not told to them.
    run(
        &db,
        &o.emre,
        "sheet.template.update",
        "20",
        json!({ "templateId": org_id, "expectedRevision": 1, "content": content("Ada paftası") }),
    )
    .await
    .unwrap();
    assert_eq!(heard(&db, &p.bora.actor).await.len(), seen_before);
    sqlx::query("delete from kentos.membership where user_id = $1 and tenant_id = $2")
        .bind(p.bora.actor.user_id)
        .bind(o.buro_id)
        .execute(&db.owner)
        .await
        .unwrap();
    assert_eq!(
        tenancy::access(&db.app, &p.bora.actor, o.buro_id)
            .await
            .unwrap_err()
            .code(),
        "not_found"
    );
    assert_eq!(
        sheet_templates::detail(&db.app, &p.bora.actor, org_id)
            .await
            .unwrap_err()
            .code(),
        "not_found"
    );
    // A suspended organisation shows its library to nobody.
    sqlx::query("update kentos.tenant set status = 'suspended' where id = $1")
        .bind(o.buro_id)
        .execute(&db.owner)
        .await
        .unwrap();
    assert_eq!(
        sheet_templates::detail(&db.app, &o.emre.actor, org_id)
            .await
            .unwrap_err()
            .code(),
        "not_found"
    );
    let _ = &o.diger;
    db.close().await;
}
