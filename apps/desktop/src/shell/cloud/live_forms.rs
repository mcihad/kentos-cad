//! The 25th and 29th items' acceptance in the cloud (docs/adr/9194): the desktop against a real KentOS server on a
//! throwaway database, with three test accounts of one organisation used as people use them. Ayşe owns the project
//! and designs its form; Mehmet is its Veri girişi; Elif its Görüntüleyici.
//!
//! - Ayşe makes “Saha denetimleri”: `Durum` asked and required, `Bolge` shown read only, `Not` and `Ekip` outside the
//!   form; the form's scope is team A's inspections; it is published and enforced, then shared. Of the four
//!   inspections one has no note (NULL) and one an empty one.
//! - Mehmet opens it from Benimle paylaşılanlar. He reads only team A's records and only what the form shows: no
//!   `Not`, no `Ekip`, in the table, in Bilgi's card and in what he copies out, each naming them. He writes `Durum`
//!   through the form; an empty `Durum` is refused. Writing `Bolge` or a record outside the scope is refused by the
//!   server.
//! - Elif opens the same form and changes nothing; her role is named. She reads the notes, the NULL one apart from the
//!   empty one.
//! - Ayşe sees Mehmet's value with the note kept, and publishes version 2 (“Saha durumu”). Her own form shows the new
//!   label with `Bolge` read only; so does Mehmet's.
//!
//! `apps/desktop/scripts/forms-roles-live.sh` makes the database and the server and runs this test with
//! `KENTOS_E2E_SERVER` and `KENTOS_E2E_PASSWORD` (never printed), then pictures the project's page in the cloud
//! portal for the three accounts (`apps/portal/scripts/forms-shots.mjs`), from the ids this test writes to
//! `.run/cloud-forms/project.json`. The pictures go to `.run/gui-evidence/kullanim-9194/` (never in Git).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use iced::futures::executor::block_on;
use kentos_commands::{CatalogView, Entity, ProjectStorage};
use kentos_domain::{Slot, Uuid};
use serde_json::{Value, json};

use super::live_run::{Runner, app, ids, said, saved, sign_in_as, upload_document};
use crate::app::{App, Message};
use crate::cloud::Event;
use crate::cloud::catalog::List;
use crate::forms::entry as fe;
use crate::gis;

const LAYER: &str = "saha";
const FORM: &str = "saha-denetimi";
const NOTE: &str = "Teknik ekip notu";

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} gerekli (apps/desktop/scripts/forms-roles-live.sh)"))
}

fn repo() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn evidence(name: &str) -> PathBuf {
    let dir = repo().join(".run/gui-evidence/kullanim-9194");
    std::fs::create_dir_all(&dir).expect("a folder");
    dir.join(format!("{name}.png"))
}

/// The form's body: `Durum` required, `Bolge` read only, `Not` and `Ekip` not in it; team A's inspections only (a
/// scope of its own field, so that changing `Bolge` stays inside it). `label` names `Durum`'s item (version 2's
/// “Saha durumu”).
fn body(revision: u32, label: Option<&str>) -> Value {
    let mut durum = json!({ "type": "field", "id": "durum", "field": "Durum", "required": true });
    if let Some(l) = label {
        durum["label"] = json!(l);
    }
    json!({ "revision": revision, "scope": "Ekip = 'A'", "items": [
        durum,
        { "type": "field", "id": "bolge", "field": "Bolge", "readOnly": true }
    ]})
}

/// The project's drawing: the layer “Saha denetimleri” with its four fields, four inspections (Merkez's and Güney's
/// team A's, Güney's with no note; Kuzey's and Batı's team B's, Batı's with an empty note), the form published as
/// version 1 and enforced.
fn drawing() -> crate::document::Document {
    let mut d: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/document/v1/sample.json")).expect("the sample");
    let typed: kentos_commands::FormBody = serde_json::from_value(body(1, None)).expect("a body");
    let form = json!({
        "id": FORM, "name": "Saha denetimi", "layer": LAYER, "draft": body(1, None),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body(1, None), "note": "İlk yayın" }],
        "enforce": true
    });
    d["name"] = json!("Saha denetimleri");
    d["layers"] = json!([{
        "id": LAYER, "name": "Saha denetimleri", "type": "layer", "visible": true, "locked": false, "expanded": true,
        "style": { "color": "#C2410C", "lineType": "continuous", "lineWeight": 0.35 }, "children": [],
        "geometry": "point",
        "fields": [
            { "name": "Durum", "kind": "text", "values": [{ "code": "iyi", "label": "İyi" }, { "code": "kotu", "label": "Kötü" }] },
            { "name": "Bolge", "kind": "text" },
            { "name": "Not", "kind": "text" },
            { "name": "Ekip", "kind": "text" }
        ]
    }]);
    d["activeLayer"] = json!(LAYER);
    d["entities"] = json!([
        { "kind": "point", "layerId": LAYER, "p": { "x": 486520.0, "y": 4420190.0 }, "id": 1, "label": "D-1",
          "attrs": { "Durum": "kotu", "Bolge": "Merkez", "Not": NOTE, "Ekip": "A" } },
        { "kind": "point", "layerId": LAYER, "p": { "x": 486548.0, "y": 4420206.0 }, "id": 2, "label": "D-2",
          "attrs": { "Durum": "kotu", "Bolge": "Kuzey", "Not": "Kuzey ekibine", "Ekip": "B" } },
        { "kind": "point", "layerId": LAYER, "p": { "x": 486534.0, "y": 4420172.0 }, "id": 3, "label": "D-3",
          "attrs": { "Durum": "iyi", "Bolge": "Güney", "Ekip": "A" } },
        { "kind": "point", "layerId": LAYER, "p": { "x": 486562.0, "y": 4420184.0 }, "id": 4, "label": "D-4",
          "attrs": { "Durum": "iyi", "Bolge": "Batı", "Not": "", "Ekip": "B" } }
    ]);
    d["settings"]["workspace"] = json!("gis");
    d["settings"]["forms"] = json!([form]);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&d.to_string()).expect("reads");
    crate::document::Document::new(snapshot, None).expect("opens")
}

/// The inspection whose `Bolge` is `bolge` in the open drawing: its slot and persistent id.
fn inspection(app: &App, bolge: &str) -> Option<(Slot, Uuid)> {
    let doc = app.document.as_ref()?;
    let e = doc
        .model
        .by_layer(LAYER)
        .find(|e| e.base().attrs.get("Bolge").map(String::as_str) == Some(bolge))?;
    let slot = Slot(e.base().id);
    Some((slot, doc.model.uid(slot)?))
}

fn attrs(app: &App, bolge: &str) -> BTreeMap<String, String> {
    let Some((slot, _)) = inspection(app, bolge) else {
        return BTreeMap::new();
    };
    let doc = app.document.as_ref().expect("open");
    doc.model.get(slot).map(|e| e.base().attrs.clone().into_iter().collect()).unwrap_or_default()
}

/// A device of `login`, signed in through the sign-in window.
fn device(login: &str, api: &str, password: &str) -> Runner {
    let dir = repo().join(".run/cloud-forms/devices").join(login);
    let _ = std::fs::remove_dir_all(&dir);
    let mut r = Runner::new(app(&dir, api));
    sign_in_as(&mut r, login, password, None);
    r
}

fn user_id(r: &Runner) -> String {
    r.app.cloud.me.as_ref().map(|m| m.user.id.clone()).expect("signed in")
}

/// A project command of `r`'s account, as the program's client sends it.
fn command(r: &Runner, tenant: &str, project: &str, name: &str, expected: BTreeMap<String, String>, input: Value)
    -> Result<Value, kentos_cloud::ApiFailure> {
    let client = r.app.cloud.client.clone().expect("signed in");
    let envelope = kentos_cloud::saving::envelope(
        Uuid::parse_str(tenant).expect("id"),
        Uuid::parse_str(project).expect("id"),
        name,
        1,
        Uuid::new_v4(),
        expected,
        input,
    );
    block_on(client.command::<Value>(envelope))
}

/// The object `id` as `r`'s account reads it from the server: its version and entity.
fn read(r: &Runner, tenant: &str, project: &str, id: Uuid) -> Option<(String, Entity)> {
    let client = r.app.cloud.client.clone().expect("signed in");
    let page = block_on(client.features_by_id(
        Uuid::parse_str(tenant).expect("id"),
        Uuid::parse_str(project).expect("id"),
        &[id],
    ))
    .expect("read");
    page.features.into_iter().next().map(|f| (f.version, f.entity))
}

/// Opens `project` from Benimle paylaşılanlar, picturing the list as `shot`.
fn open_shared(r: &mut Runner, project: &str, shot: &str) {
    r.send(Message::Run("cloud.open"));
    r.until("katalog", Duration::from_secs(15), |a| a.cloud.catalog.as_ref().is_some_and(|c| !c.loading()));
    r.cloud(Event::CatalogView(List::View(CatalogView::Shared)));
    r.until("Benimle paylaşılanlar", Duration::from_secs(15), |a| {
        a.cloud.catalog.as_ref().is_some_and(|c| !c.loading() && c.projects.iter().any(|p| p.id == project))
    });
    r.cloud(Event::CatalogPick(project.to_owned()));
    r.shot_to(&evidence(shot));
    let before = r.app.document.as_ref().map(|d| d.session);
    r.cloud(Event::CatalogOpen);
    r.until("açılış", Duration::from_secs(30), |a| {
        a.cloud.opening.is_none() && a.document.as_ref().map(|d| d.session) != before
    });
    r.run_for(Duration::from_millis(400));
}

/// Formla düzenle on the inspection of `bolge`.
fn open_form(r: &mut Runner, bolge: &str) {
    let (slot, _) = inspection(&r.app, bolge).expect("the inspection");
    r.app.selection.set([slot]);
    r.send(Message::Run("data.formEdit"));
    r.run_for(Duration::from_millis(200));
    assert!(r.app.gis.form_entry.is_some(), "the form opens: {:#?}", said(&r.app));
}

fn entry(r: &mut Runner, e: fe::Event) {
    r.send(Message::Gis(gis::Event::FormEntry(e)));
    r.run_for(Duration::from_millis(150));
}

/// The form's label of `Durum` as the open project's last release says it.
fn durum_label(app: &App) -> Option<String> {
    let doc = app.document.as_ref()?;
    let f = doc.model.settings().forms.iter().find(|f| f.id == FORM)?;
    let release = f.releases.last()?;
    let v = serde_json::to_value(&release.body).ok()?;
    v["items"][0]["label"].as_str().map(str::to_owned)
}

#[test]
#[ignore = "a real KentOS server: apps/desktop/scripts/forms-roles-live.sh"]
fn forms_roles_live() {
    let api = env("KENTOS_E2E_SERVER");
    let password = env("KENTOS_E2E_PASSWORD");

    // The three accounts.
    let mut ayse = device("ayse", &api, &password);
    let mut mehmet = device("mehmet", &api, &password);
    let mut elif = device("elif", &api, &password);
    let (mehmet_id, elif_id) = (user_id(&mehmet), user_id(&elif));

    // 1. Ayşe's project: the layer, two inspections, the form published and enforced; kept in the database.
    upload_document(&mut ayse, "Saha denetimleri", ProjectStorage::Database, drawing(), None);
    let (tenant, project) = ids(&ayse.app);
    ayse.shot_to(&evidence("01-sahip-proje"));
    for (user, role) in [(&mehmet_id, "entry"), (&elif_id, "viewer")] {
        command(&ayse, &tenant, &project, "project.share", BTreeMap::new(), json!({ "userId": user, "role": role }))
            .unwrap_or_else(|e| panic!("shared as {role}: {} {}", e.code, e.message));
    }
    let (_, kuzey) = inspection(&ayse.app, "Kuzey").expect("Kuzey's inspection");
    // The portal's pictures find the project by these (apps/portal/scripts/forms-shots.mjs).
    std::fs::write(
        repo().join(".run/cloud-forms/project.json"),
        json!({ "tenant": tenant, "project": project }).to_string(),
    )
    .expect("the ids written");

    // 2. Mehmet, Veri girişi: the shared project from Benimle paylaşılanlar; the form's records and fields only.
    open_shared(&mut mehmet, &project, "02-veri-girisi-paylasilanlar");
    let role = mehmet.app.document.as_ref().and_then(|d| d.cloud_source()).map(|s| s.info.access.role);
    assert_eq!(role, Some(kentos_commands::ProjectRole::Entry));
    let merkez = attrs(&mehmet.app, "Merkez");
    assert_eq!(merkez.get("Durum").map(String::as_str), Some("kotu"), "{merkez:?}");
    assert!(!merkez.contains_key("Not") && !merkez.contains_key("Ekip"), "not his to read: {merkez:?}");
    // Team A's records only: Merkez's and Güney's; Kuzey's and Batı's are outside the form's scope.
    assert!(inspection(&mehmet.app, "Güney").is_some(), "team A's second record is his");
    for other in ["Kuzey", "Batı"] {
        assert!(inspection(&mehmet.app, other).is_none(), "{other}'s record is outside his scope");
    }
    assert!(mehmet.app.records_locked().is_none(), "forms.use writes records: {:?}", mehmet.app.records_locked());
    assert!(mehmet.app.forms_locked().is_some(), "but does not design forms");
    mehmet.send(Message::Run("data.featureTable"));
    mehmet.run_for(Duration::from_millis(400));
    mehmet.shot_to(&evidence("03-veri-girisi-tablo"));
    // What he copies out holds the same fields: no Not, no Ekip, and names them.
    let (copied, rows, withheld) = mehmet.app.features_copy_text().expect("rows shown");
    let head: Vec<&str> = copied.lines().next().unwrap_or_default().split('\t').collect();
    assert_eq!(rows, 2, "{copied}");
    assert!(!head.contains(&"Not") && !head.contains(&"Ekip"), "{head:?}");
    assert!(head.contains(&"Durum") && head.contains(&"Bolge"), "{head:?}");
    println!("Veri girişinin kopyası: {} satır, sütunlar {head:?}; dışarıda: {withheld:?}", rows);
    // Bilgi's card of Merkez's record: no Not, no Ekip, both named.
    let (merkez_slot, _) = inspection(&mehmet.app, "Merkez").expect("Merkez's inspection");
    mehmet.send(Message::Run("view.zoomExtents"));
    // Bilgi's tool, then its card beside Merkez's point, as a click on it leaves them.
    mehmet.send(Message::Run("tool.identify"));
    mehmet.app.info_card = Some((merkez_slot, kentos_interaction::Vec2::new(486520.0, 4420190.0)));
    mehmet.run_for(Duration::from_millis(300));
    assert!(mehmet.app.info_card.is_some(), "the card stays with Bilgi's tool");
    let card = mehmet.app.slot_card(merkez_slot).expect("its card");
    assert!(!card.rows.iter().any(|(k, _)| k == "Not" || k == "Ekip"), "{:?}", card.rows);
    assert!(card.rows.iter().any(|(k, v)| k == "Rolünüz okumuyor" && v == "Not, Ekip"), "{:?}", card.rows);
    assert!(card.rows.iter().any(|(k, v)| k == "Durum" && v == "Kötü"), "a code by its label, as the table: {:?}", card.rows);
    mehmet.shot_to(&evidence("04-veri-girisi-bilgi"));
    mehmet.send(Message::InfoCard(crate::info_card::Event::Close));
    mehmet.send(Message::Run("tool.select"));
    open_form(&mut mehmet, "Merkez");
    mehmet.shot_to(&evidence("05-veri-girisi-form"));
    // An empty Durum: refused by the form, nothing written.
    entry(&mut mehmet, fe::Event::Value("Durum".into(), String::new()));
    entry(&mut mehmet, fe::Event::Save);
    assert!(mehmet.app.gis.form_entry.is_some(), "the form stays open with its issue");
    let refused = said(&mehmet.app).last().cloned().unwrap_or_default();
    assert!(refused.contains("Durum"), "refused for the empty Durum: {refused}");
    println!("boş zorunlu Durum: {refused}");
    mehmet.shot_to(&evidence("06-veri-girisi-bos-zorunlu"));
    assert_eq!(attrs(&mehmet.app, "Merkez").get("Durum").map(String::as_str), Some("kotu"));
    // Durum: iyi, saved and sent.
    entry(&mut mehmet, fe::Event::Value("Durum".into(), "iyi".into()));
    entry(&mut mehmet, fe::Event::Save);
    assert!(mehmet.app.gis.form_entry.is_none(), "saved and closed: {:#?}", said(&mehmet.app));
    mehmet.until("kayıt", Duration::from_secs(20), saved);
    mehmet.shot_to(&evidence("07-veri-girisi-kaydedildi"));
    let (_, merkez_id) = inspection(&mehmet.app, "Merkez").expect("Merkez's inspection");

    // 3. What the server refuses him whatever the program does: the read-only field, a record outside the scope.
    let (version, mut entity) = read(&mehmet, &tenant, &project, merkez_id).expect("his record");
    assert!(!entity.base().attrs.contains_key("Not"), "the server does not send him the note");
    entity.base_mut().attrs.insert("Bolge".into(), "Kuzey".into());
    let bolge = command(
        &mehmet,
        &tenant,
        &project,
        "project.changes",
        BTreeMap::from([(merkez_id.to_string(), version)]),
        json!({ "features": [{ "op": "update", "id": merkez_id.to_string(), "entity": entity }] }),
    );
    let bolge = bolge.expect_err("Bolge is read only for him");
    println!("salt okunur Bolge'ye yazma: {} {} — {}", bolge.status, bolge.code, bolge.message);
    assert!(bolge.message.contains("salt okunur"), "refused as read only: {}", bolge.message);
    let (k_version, mut k_entity) = read(&ayse, &tenant, &project, kuzey).expect("Kuzey's inspection");
    k_entity.base_mut().attrs.insert("Durum".into(), "iyi".into());
    let outside = command(
        &mehmet,
        &tenant,
        &project,
        "project.changes",
        BTreeMap::from([(kuzey.to_string(), k_version)]),
        json!({ "features": [{ "op": "update", "id": kuzey.to_string(), "entity": k_entity }] }),
    );
    let outside = outside.expect_err("Kuzey's record is outside his form's scope");
    println!("kapsam dışı kayda yazma: {} {} — {}", outside.status, outside.code, outside.message);
    assert_eq!(outside.status, 404, "a record outside his scope is not his to see: {}", outside.message);

    // 4. Elif, Görüntüleyici: the same form, read only, her role named; the note is hers to read.
    open_shared(&mut elif, &project, "08-goruntuleyici-paylasilanlar");
    let why = elif.app.records_locked().expect("a viewer writes no record");
    assert!(why.contains("Görüntüleyici"), "{why}");
    let seen = attrs(&elif.app, "Merkez");
    assert_eq!(seen.get("Durum").map(String::as_str), Some("iyi"), "{seen:?}");
    assert_eq!(seen.get("Not").map(String::as_str), Some(NOTE), "{seen:?}");
    // A note never written (NULL) and an empty one are apart, on her drawing as on the server.
    assert!(!attrs(&elif.app, "Güney").contains_key("Not"), "Güney's note is NULL");
    assert_eq!(attrs(&elif.app, "Batı").get("Not").map(String::as_str), Some(""), "Batı's note is an empty text");
    elif.send(Message::Run("data.featureTable"));
    elif.run_for(Duration::from_millis(400));
    elif.shot_to(&evidence("09-goruntuleyici-tablo"));
    open_form(&mut elif, "Merkez");
    elif.shot_to(&evidence("10-goruntuleyici-form"));
    entry(&mut elif, fe::Event::Value("Durum".into(), "kotu".into()));
    entry(&mut elif, fe::Event::Save);
    assert_eq!(attrs(&elif.app, "Merkez").get("Durum").map(String::as_str), Some("iyi"), "she changes nothing");

    // 5. Ayşe: Mehmet's value arrived, the note kept; version 2 published with Durum's new label.
    ayse.until("Mehmet'in değeri", Duration::from_secs(30), |a| {
        attrs(a, "Merkez").get("Durum").map(String::as_str) == Some("iyi")
    });
    assert_eq!(attrs(&ayse.app, "Merkez").get("Not").map(String::as_str), Some(NOTE), "the hidden note is kept");
    assert_eq!(attrs(&ayse.app, "Merkez").get("Bolge").map(String::as_str), Some("Merkez"));
    {
        let doc = ayse.app.document.as_mut().expect("open");
        let revision = doc.model.settings().forms.iter().find(|f| f.id == FORM).map_or(1, |f| f.draft.revision);
        for edit in [
            json!({ "edit": { "kind": "update", "id": FORM, "body": body(revision, Some("Saha durumu")) } }),
            json!({ "edit": { "kind": "publish", "id": FORM, "note": "Durum'un adı: Saha durumu" } }),
        ] {
            let input: kentos_commands::FormsEdit = serde_json::from_value(edit).expect("an input");
            let done = kentos_native_application::forms_edit::execute(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                input,
            );
            assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
        }
    }
    ayse.send(Message::Modifiers(iced::keyboard::Modifiers::default()));
    ayse.until("ikinci sürüm gönderildi", Duration::from_secs(20), saved);
    assert_eq!(durum_label(&ayse.app).as_deref(), Some("Saha durumu"));
    ayse.send(Message::Run("data.forms"));
    ayse.run_for(Duration::from_millis(300));
    ayse.shot_to(&evidence("11-sahip-ikinci-surum"));
    ayse.app.gis.forms_window = None;
    ayse.app.dialog = None;
    // Her own form on the record: the new label, Bolge read only for the owner too.
    open_form(&mut ayse, "Merkez");
    let bolge_read_only = |a: &App| {
        a.gis.form_entry.as_ref().and_then(|e| e.state("bolge")).is_some_and(|s| s.read_only)
    };
    assert!(bolge_read_only(&ayse.app), "Bolge is read only for whoever opens the record");
    ayse.shot_to(&evidence("12-sahip-form"));
    entry(&mut ayse, fe::Event::Cancel);

    // 6. Mehmet's form shows version 2's label; his next write keeps the note again.
    mehmet.until("ikinci sürüm geldi", Duration::from_secs(30), |a| durum_label(a).as_deref() == Some("Saha durumu"));
    open_form(&mut mehmet, "Merkez");
    assert!(bolge_read_only(&mehmet.app), "and for him");
    mehmet.shot_to(&evidence("13-veri-girisi-ikinci-surum"));
    entry(&mut mehmet, fe::Event::Value("Durum".into(), "kotu".into()));
    entry(&mut mehmet, fe::Event::Save);
    mehmet.until("ikinci kayıt", Duration::from_secs(20), saved);
    ayse.until("ikinci değer", Duration::from_secs(30), |a| {
        attrs(a, "Merkez").get("Durum").map(String::as_str) == Some("kotu")
    });
    assert_eq!(attrs(&ayse.app, "Merkez").get("Not").map(String::as_str), Some(NOTE), "kept again");
    println!("kabul tamam: {} — {}", tenant, project);
}
