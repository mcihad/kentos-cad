//! CAD-ADV-12's acceptance (plan/cad-advanced.md; docs/adr/1100 §7): two office profiles, each a versioned standard
//! package with a layer, a line type, a text style, a table style (docs/adr/1126–1128), a block publication and a
//! sheet template, taken in turn by one
//! drawing through the commands alone (`cad.settings.set`, `cad.standards.resolve`, `.apply`, `.install`). The same
//! named block, line type and table style differ between the offices; the preview equals the result; an object's
//! own colour, a table's own cell style and two viewports' different layer overrides survive the second office; undo takes the install back whole; and the
//! same recipe gives the same drawing on a new project and on a project saved and opened again.

use kentos_commands::CommandResult;
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

const DUVAR: &str = "duvar";

fn door(release: u32) -> Value {
    json!({"name": "KAPI", "publication": {"familyId": "0192f5b1-0000-7000-8000-0000000f0001",
        "typeId": "0192f5b1-0000-7000-8000-0000000e0001", "typeName": "Tek kanatlı kapı 90", "release": release},
        "units": "m"})
}

fn office(
    id: &str,
    revision: &str,
    color: &str,
    weight: f64,
    gaz: [f64; 2],
    note: f64,
    release: u32,
    cetvel: f64,
) -> String {
    json!({"format": "kentos.drawing-standard", "version": 2, "id": id, "revision": revision,
        "standard": {"name": id, "layers": [{"name": "A-DUVAR", "color": color, "lineType": "continuous",
            "lineWeight": weight}]},
        "resources": {
            "lineTypes": [{"name": "GAZ", "elements": [{"length": gaz[0]}, {"length": gaz[1]}], "unit": "paper"}],
            "textStyleDefinitions": [{"id": format!("{id}-not"), "name": "A-NOT", "font": "barlow", "height": note}],
            "tableStyleDefinitions": [{"id": format!("{id}-cetvel"), "name": "CETVEL",
                "data": {"align": "right", "height": cetvel}}],
            "blocks": [door(release)],
            "sheetTemplates": [{"templateId": "sys:cad-mimari", "revision": 1}]}})
    .to_string()
}

fn office_a() -> String {
    office("ofis-a", "2026.1", "#ff0000", 0.35, [2.0, -1.0], 2.5, 1, 2.0)
}

fn office_b() -> String {
    office("ofis-b", "2026.2", "#0000ff", 0.5, [9.0, -9.0], 3.5, 2, 2.5)
}

fn fresh() -> Document {
    let style = json!({"color": "fg", "lineType": "continuous", "lineWeight": 0.25});
    let snapshot = json!({
        "format": "kentos.document", "version": 2, "name": "İki ofis",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 50.0, "workspace": "cad"},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [
            {"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true, "style": style, "children": []},
            {"id": DUVAR, "name": "A-DUVAR", "type": "layer", "visible": true, "locked": false, "expanded": true, "style": style, "children": []}],
        "activeLayer": "0",
        "entities": [
            {"kind": "line", "id": 1, "layerId": DUVAR, "attrs": {}, "color": "#00ff00", "a": {"x": 0.0, "y": 0.0}, "b": {"x": 5.0, "y": 0.0}},
            {"kind": "line", "id": 2, "layerId": DUVAR, "attrs": {}, "a": {"x": 0.0, "y": 3.0}, "b": {"x": 5.0, "y": 3.0}}],
        "uids": ["0192f5a1-3333-7000-8000-000000000001", "0192f5a1-3333-7000-8000-000000000002"],
        "styles": {"items": [], "categories": []}
    });
    Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap()
}

fn run(doc: &mut Document, name: &str, op: kentos_commands::CallOp, input: Value) -> Value {
    let answer = registry::run(&mut ExecutionContext::new(doc), name, op, input).expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer["output"].clone()
}

/// One office profile taken: bound alone, its layers added and restyled, its resources installed by `choices`; the
/// install's preview is the install's answer.
fn profile(doc: &mut Document, package: &str, choices: Value) -> Value {
    use kentos_commands::CallOp::{Execute, Plan};
    run(
        doc,
        "cad.settings.set",
        Execute,
        json!({"standardDocs": [{"text": package}]}),
    );
    let resolved = run(doc, "cad.standards.resolve", Execute, json!({}));
    run(
        doc,
        "cad.standards.apply",
        Execute,
        json!({"standard": resolved["standard"], "layers": true, "restyle": true}),
    );
    let input = json!({"choices": choices});
    let plan = run(doc, "cad.standards.install", Plan, input.clone());
    let done = run(doc, "cad.standards.install", Execute, input);
    assert_eq!(plan["rows"], done["rows"], "the preview is the result");
    for key in ["added", "replaced", "renamed", "kept", "unavailable"] {
        assert_eq!(plan[key], done[key], "{key}");
    }
    done
}

fn office_b_choices() -> Value {
    json!([
        {"kind": "block", "name": "KAPI", "action": "replace"},
        {"kind": "lineType", "name": "GAZ", "action": "renameIncoming", "newName": "GAZ-B"},
        {"kind": "textStyle", "name": "A-NOT", "action": "replace"},
        {"kind": "tableStyle", "name": "CETVEL", "action": "replace"}])
}

fn row<'a>(answer: &'a Value, kind: &str, name: &str) -> &'a Value {
    answer["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == kind && r["name"] == name)
        .expect(name)
}

/// Two viewports drawing A-DUVAR otherwise: one in magenta, one 0.7 mm thick.
fn two_viewports(doc: &mut Document) {
    let sheet = |id: &str, over: Value| {
        json!({"id": id, "name": id.to_uppercase(),
        "page": {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
                 "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}},
        "items": [{"id": format!("{id}-plan"), "name": "Plan", "frame": {"left": 30000, "top": 20000, "width": 300000, "height": 200000},
                   "kind": {"type": "map", "view": {"type": "fixed", "scale": 50}, "layerOverrides": [over]}}],
        "origin": {"templateId": "sys:cad-mimari", "revision": 1}})
    };
    let book = json!({"schema": "kentos.sheet/1", "sheets": [
        sheet("a-101", json!({"layer": DUVAR, "color": "#ff00ff"})),
        sheet("a-102", json!({"layer": DUVAR, "lineWeight": 700}))], "masters": [], "assets": []});
    doc.set_sheets(Some(kentos_commands::ProjectSheets {
        book,
        pictures: Vec::new(),
    }))
    .expect("the book");
    kentos_sheet::project::read_project_sheets(doc.sheets().unwrap()).expect("the book reads");
}

fn place_doors(doc: &mut Document) -> kentos_commands::BlockId {
    let id = doc.block_named("KAPI").expect("KAPI installed").id;
    doc.transact("Ekle", |doc| {
        for x in [10.0, 12.0] {
            let insert: kentos_commands::Entity = serde_json::from_value(json!({"kind": "insert", "id": 0,
                "layerId": "0", "attrs": {}, "block": id.to_text(), "p": {"x": x, "y": 0.0}, "scale": 1.0,
                "rotation": 0.0}))
            .unwrap();
            doc.add(insert).map_err(|e| format!("{e:?}"))?;
        }
        Ok::<(), String>(())
    })
    .unwrap();
    id
}

/// A table on A-DUVAR following the drawing's CETVEL, its first data cell its own colour (a cell style of its own).
fn place_table(doc: &mut Document) -> Slot {
    let style = doc.settings().table_styles.iter().find(|s| s.name == "CETVEL").expect("CETVEL installed").id.clone();
    doc.transact("Ekle", |doc| {
        let table: kentos_commands::Entity = serde_json::from_value(json!({"kind": "table", "id": 0,
            "layerId": DUVAR, "attrs": {}, "p": {"x": 20.0, "y": 5.0}, "rotation": 0.0, "height": 0.25,
            "rows": [0.6, 0.5], "columns": [2.0, 2.0], "cells": [["Ada", "Alan"], ["101", "512"]], "header": true,
            "styles": [{"row": 1, "col": 0, "rows": 1, "cols": 1, "color": "#00aa00"}], "tableStyle": style}))
        .unwrap();
        doc.add(table).map_err(|e| format!("{e:?}"))
    })
    .unwrap()
}

fn table_style_of(doc: &Document, slot: Slot) -> (Option<String>, Option<f64>) {
    let Some(kentos_commands::Entity::Table(t)) = doc.get(slot) else { panic!("the table") };
    let height = t
        .table_style
        .as_ref()
        .and_then(|id| doc.settings().table_styles.iter().find(|s| &s.id == id))
        .and_then(|s| s.data.as_ref()?.height);
    (t.table_style.clone(), height)
}

fn inserts_of(doc: &Document, id: kentos_commands::BlockId) -> usize {
    doc.entities()
        .filter(|e| matches!(e, kentos_commands::Entity::Insert(i) if i.block == id))
        .count()
}

/// What the profiles decide in a drawing: its layers' looks, line types, text styles without their ids, approved
/// templates and blocks without their ids.
fn produced(doc: &Document) -> Value {
    let s = doc.settings();
    let mut styles = serde_json::to_value(&s.text_styles).unwrap();
    for t in styles.as_array_mut().unwrap() {
        t.as_object_mut().unwrap().remove("id");
    }
    let blocks: Vec<Value> = doc
        .blocks()
        .iter()
        .map(|b| {
            let mut v = serde_json::to_value(&**b).unwrap();
            v.as_object_mut().unwrap().remove("id");
            v
        })
        .collect();
    json!({"layers": serde_json::to_value(doc.layers().nodes()).unwrap(), "lineTypes": s.line_types,
        "textStyles": styles, "tableStyles": s.table_styles, "approved": s.approved_sheet_templates, "blocks": blocks})
}

#[test]
fn two_office_profiles_map_layers_styles_blocks_and_templates_and_keep_overrides() {
    let mut doc = fresh();
    let a = profile(&mut doc, &office_a(), json!([]));
    assert_eq!(row(&a, "block", "KAPI")["status"], "missing");
    assert_eq!(row(&a, "block", "KAPI")["library"], "kentos-ofis");
    assert_eq!(
        row(&a, "sheetTemplate", "sys:cad-mimari")["status"],
        "missing"
    );
    assert_eq!(
        (a["added"].as_u64(), a["unavailable"].as_u64()),
        (Some(5), Some(0))
    );
    let kapi = place_doors(&mut doc);
    let table = place_table(&mut doc);
    assert_eq!(table_style_of(&doc, table), (Some("ofis-a-cetvel".into()), Some(2.0)));
    two_viewports(&mut doc);
    let book = doc.sheets().cloned();
    assert_eq!(doc.layers().get(DUVAR).unwrap().style.color, "#ff0000");

    let b = profile(&mut doc, &office_b(), office_b_choices());
    let kapi_row = row(&b, "block", "KAPI");
    assert_eq!(kapi_row["status"], "differs");
    assert_eq!(kapi_row["fields"], json!(["entities", "publication"]));
    assert_eq!(kapi_row["uses"], 2);
    assert_eq!(row(&b, "lineType", "GAZ")["fields"], json!(["elements"]));
    assert_eq!(row(&b, "textStyle", "A-NOT")["fields"], json!(["height"]));
    assert_eq!(row(&b, "sheetTemplate", "sys:cad-mimari")["status"], "same");
    let cetvel = row(&b, "tableStyle", "CETVEL");
    assert_eq!((&cetvel["status"], &cetvel["fields"], &cetvel["uses"]), (&json!("differs"), &json!(["data"]), &json!(1)));
    assert_eq!(
        (b["replaced"].as_u64(), b["renamed"].as_u64()),
        (Some(3), Some(1))
    );
    // The table still follows the drawing's CETVEL (its id kept), now the second office's; its own cell style stays.
    assert_eq!(table_style_of(&doc, table), (Some("ofis-a-cetvel".into()), Some(2.5)));
    let Some(kentos_commands::Entity::Table(t)) = doc.get(table) else { panic!("the table") };
    assert_eq!(t.styles[0].color.as_deref(), Some("#00aa00"), "the table's own cell style");

    // The same-named block is replaced in place: its id stays, both inserts follow it, the definition is the second
    // release. The drawing's GAZ is kept and the second office's comes in as GAZ-B.
    let def = doc.block_named("KAPI").unwrap();
    assert_eq!(def.id, kapi);
    assert_eq!(def.entities.len(), 5);
    assert_eq!(
        def.card
            .as_ref()
            .and_then(|c| c.publication.as_ref())
            .map(|p| p.release),
        Some(2)
    );
    assert_eq!(inserts_of(&doc, kapi), 2);
    let gaz = |doc: &Document, name: &str| {
        doc.settings()
            .line_types
            .iter()
            .find(|t| t.name == name)
            .map(|t| serde_json::to_value(&t.elements).unwrap())
    };
    assert_eq!(
        gaz(&doc, "GAZ"),
        Some(json!([{"length": 2.0}, {"length": -1.0}]))
    );
    assert_eq!(
        gaz(&doc, "GAZ-B"),
        Some(json!([{"length": 9.0}, {"length": -9.0}]))
    );
    let note = doc
        .settings()
        .text_styles
        .iter()
        .find(|t| t.name == "A-NOT")
        .unwrap();
    assert_eq!((note.id.as_str(), note.height), ("ofis-a-not", Some(3.5)));

    // The layer took the second office's look; the object's own colour and both viewports' overrides stayed.
    assert_eq!(doc.layers().get(DUVAR).unwrap().style.color, "#0000ff");
    assert_eq!(
        doc.get(Slot(1)).unwrap().base().color.as_deref(),
        Some("#00ff00")
    );
    assert_eq!(doc.sheets().cloned(), book, "no sheet is changed");

    // Where the values come from: the own colour over the layer, the plain line's two viewports apart.
    let own = kentos_native_application::appearance_origin::explain(&doc, Slot(1));
    let color = own.iter().find(|p| p.part == "color").unwrap();
    assert_eq!(
        (color.value.as_str(), color.origin.label()),
        ("#00ff00", "Nesne")
    );
    assert_eq!(color.resets[0].after, "#0000ff");
    let plain = kentos_native_application::appearance_origin::explain(&doc, Slot(2));
    let color = plain.iter().find(|p| p.part == "color").unwrap();
    assert_eq!(color.origin.label(), "Katman");
    assert_eq!(color.viewports, ["A-101 · Plan: #ff00ff"]);
    assert!(
        color
            .chain
            .iter()
            .any(|l| l.note == "ofis-b 2026.2 · aynı"),
        "{:?}",
        color.chain
    );
    let weight = plain.iter().find(|p| p.part == "lineWeight").unwrap();
    assert_eq!(weight.viewports, ["A-102 · Plan: 0.7 mm"]);
    assert_eq!(weight.resets.last().map(|r| r.count), Some(1));

    // One undo takes the whole install back: the first release, no GAZ-B, A-NOT 2.5.
    assert_eq!(
        doc.undo().as_deref(),
        Some(kentos_commands::STANDARDS_INSTALL_STEP)
    );
    let def = doc.block_named("KAPI").unwrap();
    assert_eq!((def.id, def.entities.len()), (kapi, 4));
    assert_eq!(gaz(&doc, "GAZ-B"), None);
    assert_eq!(table_style_of(&doc, table), (Some("ofis-a-cetvel".into()), Some(2.0)));
    assert_eq!(
        doc.settings()
            .text_styles
            .iter()
            .find(|t| t.name == "A-NOT")
            .unwrap()
            .height,
        Some(2.5)
    );
}

#[test]
fn the_same_profile_recipe_gives_the_same_drawing_on_a_new_and_a_reopened_project() {
    let mut live = fresh();
    profile(&mut live, &office_a(), json!([]));
    profile(&mut live, &office_b(), office_b_choices());

    let mut saved = fresh();
    profile(&mut saved, &office_a(), json!([]));
    let bytes = kentos_kcad::encode_verified(&saved.to_snapshot_v2()).expect("writes");
    let mut reopened =
        Document::from_snapshot_v2(kentos_kcad::decode(&bytes).expect("reads")).expect("opens");
    assert_eq!(
        reopened.settings().approved_sheet_templates,
        saved.settings().approved_sheet_templates
    );
    profile(&mut reopened, &office_b(), office_b_choices());
    assert_eq!(produced(&live), produced(&reopened));

    // A new project taking only the second office ends with the same block definition the first one replaced to.
    let mut only_b = fresh();
    let b = profile(&mut only_b, &office_b(), json!([]));
    assert_eq!(row(&b, "block", "KAPI")["status"], "missing");
    let look = |d: &Document| {
        let mut v = serde_json::to_value(d.block_named("KAPI").unwrap()).unwrap();
        v.as_object_mut().unwrap().remove("id");
        v
    };
    assert_eq!(look(&only_b), look(&live));
}

/// CAD-ADV-12: “eski projeyi açmak standardı uygulama izni sayılmasın”. A project bound to the second office but not
/// applied, saved and opened again, keeps the first office's look; checking it says the difference and writes nothing.
#[test]
fn opening_a_project_bound_to_another_office_does_not_apply_it() {
    use kentos_commands::CallOp::Execute;
    let mut doc = fresh();
    profile(&mut doc, &office_a(), json!([]));
    run(&mut doc, "cad.settings.set", Execute, json!({"standardDocs": [{"text": office_b()}]}));
    let bytes = kentos_kcad::encode_verified(&doc.to_snapshot_v2()).expect("writes");
    let mut reopened = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).expect("reads")).expect("opens");
    let opened = reopened.to_snapshot_v2();
    assert_eq!(opened, doc.to_snapshot_v2(), "opening changes nothing");
    assert_eq!(reopened.layers().get(DUVAR).unwrap().style.color, "#ff0000", "still the first office's");
    let resolved = run(&mut reopened, "cad.standards.resolve", Execute, json!({}));
    let checked = run(&mut reopened, "cad.standards.check", Execute, json!({"standard": resolved["standard"]}));
    let issues = checked["issues"].as_array().unwrap();
    assert!(
        issues.iter().any(|i| i["kind"] == "layerColor" && i["expected"] == "#0000ff" && i["found"] == "#ff0000"),
        "{issues:?}"
    );
    assert_eq!(reopened.to_snapshot_v2(), opened, "checking writes nothing either");
    assert!(!reopened.can_undo());
}

#[test]
fn a_library_block_that_breaks_the_block_rules_is_refused() {
    use kentos_native_application::standard_libraries::{BlockLibrary, Libraries};
    let mut doc = fresh();
    run(
        &mut doc,
        "cad.settings.set",
        kentos_commands::CallOp::Execute,
        json!({"standardDocs": [{"text": office_a()}]}),
    );
    // A project library publishing the door's first release whose definition places a block it does not have.
    let broken: kentos_commands::BlockDefinition = serde_json::from_value(json!({
        "id": "0192f5a0-0000-7000-8000-00000000e001", "name": "Kırık kapı", "base": {"x": 0.0, "y": 0.0},
        "units": "m",
        "entities": [{"kind": "insert", "id": 1, "layerId": "", "attrs": {}, "block": "0192f5a0-0000-7000-8000-00000000e0ff",
                      "p": {"x": 0.0, "y": 0.0}, "scale": 1.0, "rotation": 0.0}],
        "card": {"publication": door(1)["publication"], "version": 1}}))
    .unwrap();
    let libraries = Libraries::builtin().with_blocks(BlockLibrary {
        name: "proje".into(),
        blocks: vec![broken],
    });
    let revision = doc.revision();
    match kentos_native_application::standards_install::execute_with(
        &mut ExecutionContext::new(&mut doc),
        kentos_commands::StandardsInstall::default(),
        &libraries,
    ) {
        CommandResult::Failed { error } => {
            assert_eq!(error.code, "block_refused");
            assert!(error.message.contains("KAPI"), "{}", error.message);
        }
        other => panic!("refused: {other:?}"),
    }
    assert_eq!(doc.revision(), revision);
    assert!(doc.block_named("KAPI").is_none());
}

/// docs/adr/1015: “Bu değer nereden geliyor?” says what a sheet prints an object's colour and line weight as when the
/// sheet's plot profile gives its layer a pen: siyah.ctb's pen 1 is black, 0.50 mm (the reference's table), given to
/// Duvar and applied to sheet a-101. The model's own red and 0.25 mm are not changed.
#[test]
fn the_value_origin_says_what_the_sheets_plot_profile_prints() {
    let cases: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.plotstyles.import.json"))
        .expect("the cases");
    let mut doc = Document::from_snapshot_v2(serde_json::from_value(cases["setup"].clone()).expect("a v2 snapshot"))
        .expect("opens");
    let made = run(&mut doc, "cad.line.create", kentos_commands::CallOp::Execute, json!({
        "layerId": DUVAR, "a": {"x": 0, "y": 0}, "b": {"x": 4, "y": 0}}));
    let uid = made["created"][0].as_str().or_else(|| made["uid"].as_str()).expect("the line").to_owned();
    let slot = doc.slot_of(kentos_domain::Uuid::parse_str(&uid).unwrap()).expect("its slot");
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/commands/v1/plotstyles");
    let before = kentos_native_application::appearance_origin::explain(&doc, slot);
    assert!(before.iter().all(|p| p.printed.is_empty()), "no profile yet");
    run(&mut doc, "cad.plotstyles.import", kentos_commands::CallOp::Execute, json!({
        "path": "siyah.ctb", "base": base.display().to_string(),
        "assignments": [{"target": {"type": "modelLayer", "id": DUVAR}, "key": {"type": "aci", "index": 1}}],
        "sheets": ["a-101"]}));
    let explained = kentos_native_application::appearance_origin::explain(&doc, slot);
    let part = |name: &str| explained.iter().find(|p| p.part == name).expect(name);
    assert!(part("color").printed.iter().any(|l| l.ends_with("“siyah” profili: #000000")), "{:?}", part("color").printed);
    assert!(part("lineWeight").printed.iter().any(|l| l.ends_with("“siyah” profili: 0.5 mm")), "{:?}", part("lineWeight").printed);
    let _ = Slot(0);
}

/// docs/adr/1018: “Bu değer nereden geliyor?” on an insert says where each of the block's parts takes its colour,
/// line type and weight from. KAPI's four lines, the insert red on A-DUVAR (#E5484D, 0.5 mm): one plain on the block's
/// layer (ByBlock: red), one green of its own, one ByLayer colour on the block's layer (the insert's layer: #E5484D),
/// one ByLayer colour on Ağaç (#2f9e44). Every line type and weight is the insert's, as none has its own.
#[test]
fn the_value_origin_of_an_insert_counts_where_the_blocks_parts_take_their_look() {
    let style = |color: &str, w: f64| json!({"color": color, "lineType": "continuous", "lineWeight": w});
    let line = |id: u32, layer: &str, extra: Value| {
        let mut e = json!({"kind": "line", "id": id, "layerId": layer, "attrs": {}, "a": {"x": 0.0, "y": f64::from(id)},
            "b": {"x": 1.0, "y": f64::from(id)}});
        e.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        e
    };
    let block = "0192f5a1-4444-7000-8000-000000000001";
    let snapshot = json!({
        "format": "kentos.document", "version": 2, "name": "Blok kökeni",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
            "plotScale": 50.0, "workspace": "cad"},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [
            {"id": "0", "name": "0", "type": "layer", "visible": true, "locked": false, "expanded": true, "style": style("fg", 0.25), "children": []},
            {"id": DUVAR, "name": "A-DUVAR", "type": "layer", "visible": true, "locked": false, "expanded": true, "style": style("#E5484D", 0.5), "children": []},
            {"id": "agac", "name": "Ağaç", "type": "layer", "visible": true, "locked": false, "expanded": true, "style": style("#2f9e44", 0.18), "children": []}],
        "activeLayer": "0",
        "blocks": [{"id": block, "name": "KAPI", "base": {"x": 0.0, "y": 0.0}, "entities": [
            line(1, "", json!({})),
            line(2, "", json!({"color": "#00ff00"})),
            line(3, "", json!({"byLayer": ["color"]})),
            line(4, "agac", json!({"byLayer": ["color"]}))]}],
        "entities": [
            {"kind": "insert", "id": 1, "layerId": DUVAR, "attrs": {}, "color": "#ff0000", "block": block,
             "p": {"x": 10.0, "y": 10.0}, "scale": 1.0, "rotation": 0.0}],
        "uids": ["0192f5a1-4444-7000-8000-000000000101"],
        "styles": {"items": [], "categories": []}
    });
    let doc = Document::from_snapshot_v2(serde_json::from_value(snapshot).unwrap()).unwrap();
    let explained = kentos_native_application::appearance_origin::explain(&doc, Slot(1));
    let parts = explained.iter().find(|p| p.part == "blockParts").expect("the block's parts");
    assert_eq!((parts.value.as_str(), parts.origin.label()), ("“KAPI” · 4 parça", "Blok"));
    let said: Vec<(&str, &str, &str)> = parts.chain.iter().map(|l| (l.origin.label(), l.value.as_str(), l.note.as_str())).collect();
    let continuous = kentos_commands::LineType::Continuous.label();
    assert_eq!(said, [
        ("Nesne", "#00ff00", "Renk: 1/4 parça kendi değeri"),
        ("Blok", "#ff0000", "Renk: 1/4 parça bloktan (ByBlock)"),
        ("Katman", "#E5484D, #2f9e44", "Renk: 2/4 parça katmandan (ByLayer)"),
        ("Blok", continuous, "Çizgi tipi: 4/4 parça bloktan (ByBlock)"),
        ("Blok", "0.5 mm", "Kalınlık: 4/4 parça bloktan (ByBlock)"),
    ]);
    // The insert's own colour is still explained as the insert's.
    let color = explained.iter().find(|p| p.part == "color").unwrap();
    assert_eq!((color.value.as_str(), color.origin.label()), ("#ff0000", "Nesne"));
}
