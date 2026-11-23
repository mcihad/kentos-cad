//! The municipal report's 8th item, its last part (docs/adr/9180): one road layer and its form tried on the same data,
//! end to end. Its durum takes only its list's values; kontrol_tarihi shows and is asked for only when durum is
//! kontrol; its maintenance records are listed by the road's code and a new one gets the code; a Veri girişi changes
//! the road's values but makes no maintenance record; a second release with a new value opens the road written under
//! the first, its values kept, and takes the new value.

use kentos_commands::{ProjectPermission, ProjectRole, ProjectState, ProjectStorage};
use serde_json::{Value, json};

use crate::app::App;
use crate::form_scenes::entry;
use crate::forms::entry as fe;
use crate::tools_scenes::run;

/// A value list: each value its own code and label.
fn choices(values: &[&str]) -> Value {
    Value::Array(values.iter().map(|v| json!({ "code": v, "label": v })).collect())
}

/// A published release of a form's `items`, numbered `version`.
fn release(version: u32, items: &Value) -> Value {
    let body = json!({ "revision": version, "items": items });
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    json!({ "version": version, "digest": kentos_commands::body_digest(&typed), "body": body, "note": format!("{version}. yayın") })
}

/// The road form's items: kod, durum, kontrol_tarihi when durum is kontrol, the maintenance records.
fn road_items() -> Value {
    json!([
        { "type": "field", "id": "kod", "field": "kod" },
        { "type": "field", "id": "durum", "field": "durum" },
        { "type": "field", "id": "kontrol-tarihi", "field": "kontrol_tarihi",
          "visibleWhen": "durum = 'kontrol'", "requiredWhen": "durum = 'kontrol'" },
        { "type": "related", "id": "bakimlar", "label": "Bakımlar", "layer": "bakim", "field": "yol" }
    ])
}

/// Yol (Y1, planlı, a 100 m axis) and Bakım (B1 on Y1), their relation by the road's code, and a published form on
/// each; durum's list planlı, kontrol, tamam.
fn roads() -> App {
    let mut drawing: Value =
        serde_json::from_str(include_str!("../../../../fixtures/interaction/v1/empty.kcad")).expect("the drawing");
    drawing["settings"]["workspace"] = json!("gis");
    let text = |name: &str| json!({ "name": name, "kind": "text" });
    let layer = |id: &str, name: &str, geometry: &str, fields: Value| {
        json!({ "id": id, "name": name, "type": "layer", "visible": true, "locked": false, "expanded": true,
                "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [],
                "geometry": geometry, "fields": fields })
    };
    drawing["layers"].as_array_mut().expect("layers").extend([
        layer(
            "yol",
            "Yol",
            "line",
            json!([text("kod"), { "name": "durum", "kind": "text", "values": choices(&["planlı", "kontrol", "tamam"]) },
                   text("kontrol_tarihi")]),
        ),
        layer("bakim", "Bakım", "point", json!([text("kod"), text("yol"), { "name": "bedel", "kind": "decimal" }])),
    ]);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&drawing.to_string()).expect("reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let (mut app, _) = App::boot(None);
    let _ = app.update(crate::app::Message::Opened(Some(Ok(Box::new(doc)))));
    let p = |x: f64, y: f64| json!({ "x": 500_000.0 + x, "y": 4_400_000.0 + y });
    create(&mut app, "yol", json!({ "kind": "line", "a": p(0.0, 0.0), "b": p(100.0, 0.0) }), json!({ "kod": "Y1", "durum": "planlı" }));
    create(&mut app, "bakim", json!({ "kind": "point", "p": p(50.0, 5.0) }), json!({ "kod": "B1", "yol": "Y1", "bedel": "100" }));
    let road = json!({ "id": "yol-formu", "name": "Yol formu", "layer": "yol", "draft": { "revision": 1, "items": road_items() },
                       "releases": [release(1, &road_items())] });
    let work_items = json!([
        { "type": "field", "id": "kod", "field": "kod" },
        { "type": "field", "id": "yol", "field": "yol" },
        { "type": "field", "id": "bedel", "field": "bedel" }
    ]);
    let work = json!({ "id": "bakim-formu", "name": "Bakım formu", "layer": "bakim", "draft": { "revision": 1, "items": work_items },
                       "releases": [release(1, &work_items)] });
    let relation = json!({ "id": "yol-bakim", "name": "Yol bakımları", "parent": "yol", "child": "bakim",
                           "field": "yol", "parentKey": "kod" });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms.push(serde_json::from_value(road).expect("the road's form"));
    settings.forms.push(serde_json::from_value(work).expect("the works' form"));
    settings.relations.push(serde_json::from_value(relation).expect("the relation"));
    doc.model.set_settings(settings);
    app
}

fn create(app: &mut App, layer: &str, geometry: Value, attrs: Value) {
    let input =
        serde_json::from_value(json!({ "layerId": layer, "objects": [{ "geometry": geometry, "attrs": attrs }] })).expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    let done = kentos_native_application::create::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        input,
    );
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
}

/// The first object of `layer` and its attributes.
fn first(app: &App, layer: &str) -> (kentos_domain::Slot, std::collections::BTreeMap<String, String>) {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let e = doc.by_layer(layer).next().expect("an object");
    (kentos_domain::Slot(e.base().id), e.base().attrs.clone())
}

fn open_road(app: &mut App) {
    let (slot, _) = first(app, "yol");
    app.gis.form_entry = None;
    app.gis.form_stack.clear();
    app.selection.set([slot]);
    run(app, "data.formEdit");
    assert!(app.gis.form_entry.is_some(), "the road's form opens");
}

fn issues(app: &App) -> Vec<String> {
    app.gis.form_entry.as_ref().map(|e| e.issues.iter().map(|i| i.message.clone()).collect()).unwrap_or_default()
}

fn shown(app: &App, id: &str) -> (bool, bool) {
    let e = app.gis.form_entry.as_ref().expect("the form");
    e.state_for_tests(id).map_or((false, false), |s| (s.shown, s.required))
}

/// The open drawing as a cloud project this account holds `role` in, with `permissions`.
fn as_role(app: &mut App, role: ProjectRole, permissions: Vec<ProjectPermission>) {
    let mut info = crate::cloud::tests::info(ProjectStorage::Database, false, ProjectState::Active);
    info.access.role = role;
    info.access.permissions = permissions;
    let doc = app.document.as_mut().expect("a drawing");
    doc.source = crate::document::Source::Cloud(Box::new(crate::document::CloudSource {
        tenant: kentos_domain::Uuid::from_bytes([1; 16]),
        project: kentos_domain::Uuid::from_bytes([2; 16]),
        info,
        workspace: "Fen İşleri".into(),
        revision: None,
    }));
}

#[test]
fn the_road_form_holds_its_list_its_condition_its_works_the_role_and_a_new_release_on_one_road() {
    let mut app = roads();

    // Değer listesi: a durum not in the list is refused, the road unchanged.
    open_road(&mut app);
    entry(&mut app, fe::Event::Value("durum".into(), "bozuk".into()));
    entry(&mut app, fe::Event::Save);
    assert!(app.gis.form_entry.is_some(), "refused: the form stays");
    assert!(issues(&app).iter().any(|i| i.contains("bozuk")), "{:#?}", issues(&app));
    assert_eq!(first(&app, "yol").1.get("durum").map(String::as_str), Some("planlı"));

    // Koşullu alan: kontrol_tarihi is hidden while planlı, shown and asked for under kontrol.
    open_road(&mut app);
    assert_eq!(shown(&app, "kontrol-tarihi"), (false, false), "hidden while planlı");
    entry(&mut app, fe::Event::Value("durum".into(), "kontrol".into()));
    assert_eq!(shown(&app, "kontrol-tarihi"), (true, true), "shown and asked for under kontrol");
    entry(&mut app, fe::Event::Save);
    assert!(app.gis.form_entry.is_some(), "kontrol without its date is refused");
    assert!(!issues(&app).is_empty(), "the date is asked for");
    entry(&mut app, fe::Event::Value("kontrol_tarihi".into(), "2026-11-20".into()));
    entry(&mut app, fe::Event::Save);
    assert!(app.gis.form_entry.is_none(), "written: {:#?}", issues(&app));
    let (_, road) = first(&app, "yol");
    assert_eq!(road.get("durum").map(String::as_str), Some("kontrol"));
    assert_eq!(road.get("kontrol_tarihi").map(String::as_str), Some("2026-11-20"));

    // Alt bakım kayıtları: B1 listed by the road's code; a new work gets the code, not an id.
    open_road(&mut app);
    assert_eq!(app.gis.form_entry.as_ref().expect("the form").related_now("bakimlar").len(), 1, "B1");
    entry(&mut app, fe::Event::NewRelated("bakimlar".into()));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let codes: Vec<Option<String>> = doc.by_layer("bakim").map(|e| e.base().attrs.get("yol").cloned()).collect();
    assert_eq!(codes, [Some("Y1".to_owned()), Some("Y1".to_owned())], "the new work holds the road's code");
    open_road(&mut app);
    assert_eq!(app.gis.form_entry.as_ref().expect("the form").related_now("bakimlar").len(), 2, "both works listed");

    // Rol kısıtı: a Veri girişi changes the road's values but makes no work.
    as_role(&mut app, ProjectRole::Entry, vec![ProjectPermission::Read, ProjectPermission::FormsUse]);
    app.document.as_mut().expect("a drawing").model.set_object_policy(kentos_domain::ObjectPolicy::AttributesOnly);
    open_road(&mut app);
    entry(&mut app, fe::Event::NewRelated("bakimlar".into()));
    assert_eq!(app.document.as_ref().expect("a drawing").model.by_layer("bakim").count(), 2, "no work made");
    open_road(&mut app);
    entry(&mut app, fe::Event::Value("durum".into(), "tamam".into()));
    entry(&mut app, fe::Event::Save);
    assert!(app.gis.form_entry.is_none(), "written by the role: {:#?}", issues(&app));
    assert_eq!(first(&app, "yol").1.get("durum").map(String::as_str), Some("tamam"));

    // Sürüm yükseltmesi: release 2 adds yuzey (asfalt, parke); the road written under release 1 opens with it, its
    // values kept, and takes the new field.
    let fields: Vec<kentos_commands::LayerField> = serde_json::from_value(json!([
        { "name": "kod", "kind": "text" },
        { "name": "durum", "kind": "text", "values": choices(&["planlı", "kontrol", "tamam"]) },
        { "name": "kontrol_tarihi", "kind": "text" },
        { "name": "yuzey", "kind": "text", "values": choices(&["asfalt", "parke"]) }
    ]))
    .expect("the fields");
    let doc = app.document.as_mut().expect("a drawing");
    doc.model.set_layer_fields("yol", fields, &[]).expect("a field added");
    let mut items = road_items();
    items.as_array_mut().expect("items").push(json!({ "type": "field", "id": "yuzey", "field": "yuzey" }));
    let mut settings = doc.model.settings().clone();
    let form = settings.forms.iter_mut().find(|f| f.id == "yol-formu").expect("the road's form");
    form.releases.push(serde_json::from_value(release(2, &items)).expect("release 2"));
    form.draft = serde_json::from_value(json!({ "revision": 2, "items": items })).expect("the draft");
    doc.model.set_settings(settings);
    open_road(&mut app);
    assert!(app.gis.form_entry.as_ref().expect("the form").state_for_tests("yuzey").is_some(), "opened with release 2");
    entry(&mut app, fe::Event::Value("yuzey".into(), "beton".into()));
    entry(&mut app, fe::Event::Save);
    assert!(app.gis.form_entry.is_some(), "release 2's list refuses beton");
    entry(&mut app, fe::Event::Value("yuzey".into(), "asfalt".into()));
    entry(&mut app, fe::Event::Save);
    assert!(app.gis.form_entry.is_none(), "the new field taken: {:#?}", issues(&app));
    let (_, road) = first(&app, "yol");
    assert_eq!(road.get("yuzey").map(String::as_str), Some("asfalt"));
    assert_eq!(road.get("kontrol_tarihi").map(String::as_str), Some("2026-11-20"), "release 1's values kept");
    assert_eq!(road.get("durum").map(String::as_str), Some("tamam"));
}

/// The picture (`.run/gui-evidence/kullanim-9180/`): the road's form under kontrol, kontrol_tarihi shown and asked
/// for, its two works listed by its code.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9180", "01-yol-formu", || {
        let mut app = roads();
        open_road(&mut app);
        entry(&mut app, fe::Event::NewRelated("bakimlar".into()));
        // The new work's own form: its code typed, saved.
        entry(&mut app, fe::Event::Value("kod".into(), "B2".into()));
        entry(&mut app, fe::Event::Save);
        open_road(&mut app);
        entry(&mut app, fe::Event::Value("durum".into(), "kontrol".into()));
        app
    });
}
