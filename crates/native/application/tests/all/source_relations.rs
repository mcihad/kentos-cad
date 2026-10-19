// Many-to-many references in a bound GeoPackage, and layers written together (docs/adr/0532): the writes the shared
// cases leave out, on copies of the building, parcel and owner files. With KENTOS_WRITE_GPKG the written files are kept
// as fixtures/geopackage/v1/kentos-iliskili-*.gpkg, which scripts/fixtures/source_relations_cases.py reads with
// sqlite3 and GDAL's OGR (its relationship API).

use std::path::PathBuf;

use kentos_contracts::CallOp;
use kentos_domain::Document;
use kentos_geopackage::GeoPackage;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/geopackage/v1")
}

fn run(doc: &mut Document, command: &str, op: CallOp, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), command, op, input).expect("runs")
}

fn ok(answer: &Value) -> &Value {
    assert_eq!(answer["status"], "completed", "{answer}");
    &answer["output"]
}

const OWNER: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001";
const OWNER_2: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002";
const PARCEL_1: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2001";
const PARCEL_3: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2003";

fn empty() -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "İlişkili kayıtlar",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

/// A copy of a fixture file in the temp folder, by a name of its own.
fn copy(name: &str, tag: &str) -> PathBuf {
    let to = std::env::temp_dir().join(format!("kentos-iliski-{}-{tag}.gpkg", std::process::id()));
    std::fs::copy(dir().join(name), &to).expect("copied");
    to
}

/// The three tables bound to layers (`malik`, `parsel`, `bina`), the parcels' `malikler` a many-to-many reference and
/// the project's relations (owners to parcels, parcels to buildings).
fn world(file: &std::path::Path) -> Document {
    let mut d = empty();
    let path = file.display().to_string();
    for table in ["malik", "parsel", "bina"] {
        ok(&run(
            &mut d,
            "cad.sources.edit",
            CallOp::Execute,
            json!({ "edit": { "kind": "link", "path": path, "table": table } }),
        ));
    }
    ok(&run(
        &mut d,
        "cad.layers.edit",
        CallOp::Execute,
        json!({ "operation": "fields", "layer": "import-parsel", "fields": [
            { "name": "ada", "kind": "text" }, { "name": "parsel_no", "kind": "text" },
            { "name": "malikler", "kind": "text", "multiple": true } ] }),
    ));
    for relation in [
        json!({ "id": "parsel-malik", "name": "Parselin malikleri", "parent": "import-malik", "child": "import-parsel", "field": "malikler" }),
        json!({ "id": "bina-parsel", "name": "Parselin binaları", "parent": "import-parsel", "child": "import-bina", "field": "parsel" }),
    ] {
        ok(&run(
            &mut d,
            "cad.relations.edit",
            CallOp::Execute,
            json!({ "edit": { "kind": "create", "relation": relation } }),
        ));
    }
    d
}

fn relate(d: &mut Document, op: CallOp, relation: &str, what: &str) -> Value {
    run(
        d,
        "cad.sources.relate",
        op,
        json!({ "relation": relation, "action": what }),
    )
}

fn sync(d: &mut Document, op: CallOp, layer: &str, with: &[&str]) -> Value {
    run(
        d,
        "cad.sources.sync",
        op,
        json!({ "layer": layer, "direction": "write", "with": with }),
    )
}

fn pairs(file: &std::path::Path, mapping: &str) -> Vec<(i64, i64)> {
    let f = GeoPackage::open(file, false).expect("opens");
    f.mapping_rows(mapping).expect("rows")
}

fn uid_of(d: &Document, layer: &str, key: &str, value: &str) -> String {
    d.by_layer_with_uids(layer)
        .find(|(_, e)| e.base().attrs.get(key).is_some_and(|v| v == value))
        .map(|(u, _)| u.to_string())
        .expect("object")
}

fn keep(file: &std::path::Path, name: &str) {
    if std::env::var("KENTOS_WRITE_GPKG").is_ok() {
        std::fs::copy(file, dir().join(name)).expect("kept");
    }
}

#[test]
fn a_text_reference_is_mapped_written_with_its_layers_and_read_back() {
    let file = copy("iliskili.gpkg", "map");
    let mut d = world(&file);
    let state = ok(&relate(&mut d, CallOp::Execute, "parsel-malik", "check")).clone();
    assert_eq!(state["state"], "text", "{state}");
    // The plan says what mapping would write; the file stays as it is.
    let bytes = std::fs::read(&file).expect("read");
    let plan = ok(&relate(&mut d, CallOp::Plan, "parsel-malik", "map")).clone();
    assert_eq!(
        (plan["state"].clone(), plan["rows"].clone()),
        (json!("mapped"), json!(4)),
        "{plan}"
    );
    assert!(
        std::fs::read(&file).expect("read") == bytes,
        "a plan writes nothing"
    );
    let mapped = ok(&relate(&mut d, CallOp::Execute, "parsel-malik", "map")).clone();
    assert_eq!(mapped["mapping"], "parsel_malik", "{mapped}");
    assert_eq!(
        pairs(&file, "parsel_malik"),
        vec![(1, 1), (1, 2), (2, 2), (2, 3)]
    );
    // The three bound layers still match the file: the third table was not touched.
    let after = ok(&run(
        &mut d,
        "cad.sources.describe",
        CallOp::Execute,
        json!({}),
    ))["links"]
        .clone();
    for l in after.as_array().expect("links") {
        assert_eq!(l["changed"], false, "{l}");
        assert_eq!(l["edited"], false, "{l}");
    }
    assert_eq!(
        ok(&relate(&mut d, CallOp::Execute, "parsel-malik", "check"))["state"],
        "mapped"
    );
    // Mapped twice is refused.
    let again = relate(&mut d, CallOp::Plan, "parsel-malik", "map");
    assert_eq!(again["error"]["code"], "relation_mapped", "{again}");
    // Nothing to write now: the layers and the mapping agree.
    let plan = ok(&sync(&mut d, CallOp::Plan, "import-parsel", &[])).clone();
    assert_eq!(
        (
            plan["added"].clone(),
            plan["modified"].clone(),
            plan["removed"].clone()
        ),
        (json!(0), json!(0), json!(0)),
        "{plan}"
    );
    assert_eq!(
        plan["relations"],
        json!([{ "relation": "parsel-malik", "added": 0, "removed": 0 }])
    );
    // Another owner on parcel 3: the mapping gains a row, the column of the table is left as it was.
    let parcel3 = uid_of(&d, "import-parsel", "parsel_no", "1");
    assert_eq!(parcel3, PARCEL_3);
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [parcel3], "operation": "attributes", "attrs": { "malikler": format!("{OWNER}; {OWNER_2}") } }),
    ));
    let written = ok(&sync(&mut d, CallOp::Execute, "import-parsel", &[])).clone();
    assert_eq!(
        written["relations"],
        json!([{ "relation": "parsel-malik", "added": 2, "removed": 0 }]),
        "{written}"
    );
    assert_eq!(
        pairs(&file, "parsel_malik"),
        vec![(1, 1), (1, 2), (2, 2), (2, 3), (3, 1), (3, 2)]
    );
    // Read it back from the mapping rows, not from the column.
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [parcel3], "operation": "attributes", "attrs": { "malikler": "" } }),
    ));
    ok(&run(
        &mut d,
        "cad.sources.sync",
        CallOp::Execute,
        json!({ "layer": "import-parsel", "direction": "read", "discard": true }),
    ));
    let value = d
        .by_layer_with_uids("import-parsel")
        .find(|(u, _)| u.to_string() == PARCEL_3)
        .map(|(_, e)| e.base().attrs.get("malikler").cloned())
        .expect("parcel");
    assert_eq!(value, Some(format!("{OWNER}; {OWNER_2}")));
    // Unmapping takes the relation out of the file, the references stay in the layer.
    ok(&relate(&mut d, CallOp::Execute, "parsel-malik", "unmap"));
    assert!(
        GeoPackage::open(&file, false)
            .expect("opens")
            .relations()
            .expect("relations")
            .is_empty()
    );
    assert_eq!(
        ok(&relate(&mut d, CallOp::Execute, "parsel-malik", "check"))["state"],
        "text"
    );
    keep(&file, "kentos-iliskili-eslenmis.gpkg");
    std::fs::remove_file(&file).ok();
}

#[test]
fn a_parent_and_its_children_are_written_in_one_transaction_or_not_at_all() {
    let file = copy("iliskili.gpkg", "together");
    let mut d = world(&file);
    ok(&relate(&mut d, CallOp::Execute, "parsel-malik", "map"));
    // A new owner (a record) and a new parcel that has it.
    let owner = ok(&run(
        &mut d,
        "cad.records.create",
        CallOp::Execute,
        json!({ "layerId": "import-malik", "rows": [{ "attrs": { "ad": "Zeynep Kaya", "tur": "Gerçek" } }] }),
    ))
    .clone();
    let owner_uid = owner["uids"][0].as_str().expect("uid").to_owned();
    let parcel = ok(&run(
        &mut d,
        "cad.polygon.create",
        CallOp::Execute,
        json!({ "layerId": "import-parsel", "pts": [{ "x": 487200.0, "y": 4420000.0 }, { "x": 487210.0, "y": 4420000.0 },
                                            { "x": 487210.0, "y": 4420010.0 }] }),
    ))["uid"]
        .as_str()
        .expect("uid")
        .to_owned();
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [parcel], "operation": "attributes",
                "attrs": { "ada": "103", "parsel_no": "9", "malikler": owner_uid } }),
    ));
    let before = std::fs::read(&file).expect("read");
    // The parcel alone would name an owner the table lacks: refused, nothing written.
    let alone = sync(&mut d, CallOp::Execute, "import-parsel", &[]);
    assert_eq!(alone["status"], "failed", "{alone}");
    assert_eq!(alone["error"]["code"], "relation_dangling", "{alone}");
    assert!(std::fs::read(&file).expect("read") == before);
    // The layers must be bound and distinct.
    assert_eq!(
        sync(&mut d, CallOp::Plan, "import-parsel", &["import-parsel"])["error"]["code"],
        "invalid_input"
    );
    assert_eq!(
        sync(&mut d, CallOp::Plan, "import-parsel", &["cizim"])["error"]["code"],
        "source_not_found"
    );
    // Together: the owner and the parcel and the mapping row, one transaction.
    let plan = ok(&sync(
        &mut d,
        CallOp::Plan,
        "import-parsel",
        &["import-malik"],
    ))
    .clone();
    assert_eq!(
        (
            plan["added"].clone(),
            plan["layers"].clone(),
            plan["relations"].clone()
        ),
        (
            json!(2),
            json!([
                { "layer": "import-parsel", "added": 1, "modified": 0, "removed": 0 },
                { "layer": "import-malik", "added": 1, "modified": 0, "removed": 0 }
            ]),
            json!([{ "relation": "parsel-malik", "added": 1, "removed": 0 }])
        ),
        "{plan}"
    );
    let written = ok(&sync(
        &mut d,
        CallOp::Execute,
        "import-parsel",
        &["import-malik"],
    ))
    .clone();
    assert_eq!(written["added"], 2, "{written}");
    let f = GeoPackage::open(&file, false).expect("opens");
    let layers = f.layers().expect("layers");
    let count = |t: &str| {
        f.count(layers.iter().find(|l| l.name == t).expect("table"))
            .expect("count")
    };
    assert_eq!((count("malik"), count("parsel")), (4, 4));
    let rows = f.mapping_rows("parsel_malik").expect("rows");
    assert_eq!(rows.len(), 5);
    assert!(rows.contains(&(4, 4)), "{rows:?}");
    keep(&file, "kentos-iliskili-birlikte.gpkg");
    std::fs::remove_file(&file).ok();
}

#[test]
fn a_refused_layer_stops_the_whole_write() {
    let file = copy("iliskili.gpkg", "refused");
    let mut d = world(&file);
    let owner_uid = uid_of(&d, "import-malik", "ad", "Mehmet Demir");
    assert_eq!(owner_uid, OWNER_2);
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [owner_uid], "operation": "attributes", "attrs": { "tur": "Tüzel" } }),
    ));
    let parcel = uid_of(&d, "import-parsel", "parsel_no", "5");
    assert_eq!(parcel, PARCEL_1);
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [parcel], "operation": "attributes", "attrs": { "ada": "999" } }),
    ));
    // The owners' table cannot hold a polygon.
    let drawn = run(
        &mut d,
        "cad.polygon.create",
        CallOp::Execute,
        json!({ "layerId": "import-malik", "pts": [{ "x": 0.0, "y": 0.0 }, { "x": 1.0, "y": 0.0 }, { "x": 1.0, "y": 1.0 }] }),
    );
    assert_eq!(drawn["status"], "completed", "{drawn}");
    let before = std::fs::read(&file).expect("read");
    let refused = sync(&mut d, CallOp::Execute, "import-parsel", &["import-malik"]);
    assert_eq!(refused["error"]["code"], "geometry_mismatch", "{refused}");
    assert!(
        std::fs::read(&file).expect("read") == before,
        "the parcel's change was not written either"
    );
    std::fs::remove_file(&file).ok();
}

/// A form on the owners' layer and one on the parcels'.
fn with_forms(d: &mut Document) {
    for (id, name, layer) in [
        ("malik-formu", "Malik formu", "import-malik"),
        ("parsel-formu", "Parsel formu", "import-parsel"),
    ] {
        ok(&run(
            d,
            "cad.forms.edit",
            CallOp::Execute,
            json!({ "edit": { "kind": "create", "id": id, "name": name, "layer": layer } }),
        ));
    }
}

fn submit(d: &mut Document, op: CallOp, input: Value) -> Value {
    run(d, "cad.forms.submit", op, input)
}

#[test]
fn a_submission_with_commit_writes_the_record_and_its_children_to_the_file_in_one_step() {
    let file = copy("iliskili-eslenmis.gpkg", "commit");
    let mut d = world(&file);
    with_forms(&mut d);
    let input = json!({
        "form": "malik-formu", "draft": true, "uid": OWNER, "values": { "tur": "Tüzel" },
        "children": [{ "relation": "parsel-malik", "uid": PARCEL_3 }], "commit": true
    });
    let plan = ok(&submit(&mut d, CallOp::Plan, input.clone())).clone();
    assert_eq!(
        plan["committed"]["relations"],
        json!([{ "relation": "parsel-malik", "added": 1, "removed": 0 }]),
        "{plan}"
    );
    let before = std::fs::read(&file).expect("read");
    let done = ok(&submit(&mut d, CallOp::Execute, input)).clone();
    assert_eq!(done["undo"], "Form: Malik formu", "{done}");
    assert_eq!(done["committed"]["modified"], 1, "{done}");
    assert!(
        std::fs::read(&file).expect("read") != before,
        "the file was written"
    );
    assert_eq!(
        pairs(&file, "parsel_malik"),
        vec![(1, 1), (1, 2), (2, 2), (2, 3), (3, 1)]
    );
    // The owner's row changed in the same transaction.
    let f = GeoPackage::open(&file, false).expect("opens");
    let layers = f.layers().expect("layers");
    let malik = layers.iter().find(|l| l.name == "malik").expect("table");
    let rows = f
        .rows(malik, &kentos_geopackage::Query::default())
        .expect("rows");
    let first = rows.iter().find(|r| r.fid == 1).expect("row");
    assert!(
        first
            .values
            .contains(&kentos_geopackage::Value::Text("Tüzel".into())),
        "{first:?}"
    );
    drop(f);
    keep(&file, "kentos-iliskili-form.gpkg");
    std::fs::remove_file(&file).ok();
}

#[test]
fn a_refused_commit_writes_nothing_neither_in_the_drawing_nor_in_the_file() {
    let file = copy("iliskili-eslenmis.gpkg", "commit-refused");
    let mut d = world(&file);
    with_forms(&mut d);
    // Another parcel names an owner that is nowhere.
    let ghost = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999";
    let sixth = uid_of(&d, "import-parsel", "parsel_no", "6");
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [sixth], "operation": "attributes", "attrs": { "malikler": ghost } }),
    ));
    let before = std::fs::read(&file).expect("read");
    let revision = d.revision();
    let input = json!({
        "form": "malik-formu", "draft": true, "uid": OWNER, "values": { "tur": "Tüzel" },
        "children": [{ "relation": "parsel-malik", "uid": PARCEL_3 }], "commit": true
    });
    let refused = submit(&mut d, CallOp::Execute, input.clone());
    assert_eq!(refused["error"]["code"], "relation_dangling", "{refused}");
    assert!(std::fs::read(&file).expect("read") == before);
    assert_eq!(d.revision(), revision, "the record was not written either");
    // Without commit the drawing takes it; the file stays as it was.
    let mut loose = input;
    loose.as_object_mut().expect("object").remove("commit");
    ok(&submit(&mut d, CallOp::Execute, loose));
    assert!(std::fs::read(&file).expect("read") == before);
    std::fs::remove_file(&file).ok();
}

/// 100 000 records on a layer, a page read from them: what a lookup list and a related-records box cost
/// (docs/adr/0532 Doğrulama). `cargo test --release -p kentos-native-application --test all page_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timing, run by hand"]
fn page_timing() {
    let mut d = empty();
    let added = ok(&run(
        &mut d,
        "cad.layers.edit",
        CallOp::Execute,
        json!({ "operation": "add", "name": "Kayıt" }),
    ))
    .clone();
    let layer = added["layer"].as_str().expect("a layer").to_owned();
    ok(&run(
        &mut d,
        "cad.layers.edit",
        CallOp::Execute,
        json!({ "operation": "fields", "layer": layer, "fields": [
            { "name": "ad", "kind": "text" }, { "name": "ust", "kind": "text", "multiple": true } ] }),
    ));
    let mut first = String::new();
    for chunk in 0..10 {
        let rows: Vec<Value> = (0..10_000)
            .map(|i| {
                let n = chunk * 10_000 + i;
                json!({ "attrs": { "ad": format!("Kayıt {n}"), "ust": if n % 3 == 0 { first.clone() } else { String::new() } } })
            })
            .collect();
        let made = ok(&run(
            &mut d,
            "cad.records.create",
            CallOp::Execute,
            json!({ "layerId": layer, "rows": rows }),
        ))
        .clone();
        if first.is_empty() {
            first = made["uids"][0].as_str().expect("uid").to_owned();
        }
    }
    ok(&run(
        &mut d,
        "cad.relations.edit",
        CallOp::Execute,
        json!({ "edit": { "kind": "create", "relation": {
            "id": "kendi", "name": "Kendi", "parent": layer, "child": layer, "field": "ust" } } }),
    ));
    let time = |d: &mut Document, input: Value| {
        let t = std::time::Instant::now();
        let page = ok(&run(d, "cad.records.page", CallOp::Execute, input)).clone();
        (t.elapsed(), page)
    };
    let (t, page) = time(&mut d, json!({ "layer": layer }));
    println!("page of 50 of {}: {t:?}", page["total"]);
    let (t, page) = time(&mut d, json!({ "layer": layer, "query": "kayıt 99999" }));
    println!("search, {} found: {t:?}", page["total"]);
    let (t, page) = time(&mut d, json!({ "relation": "kendi", "of": first }));
    println!("children of one record, {} of them: {t:?}", page["total"]);
}

fn through_the_protocol(d: &mut Document, command: &str, op: CallOp, input: Value) -> Value {
    let answer = kentos_native_application::call::call(
        d,
        None,
        kentos_contracts::CallEnvelope {
            relax_constraints: false,
            command: command.into(),
            version: None,
            op,
            input,
            request_id: None,
            expected: None,
        },
    );
    serde_json::to_value(&answer.result).expect("a result")
}

#[test]
fn a_plan_through_the_protocol_writes_no_file_but_its_execute_does() {
    // The protocol finds a plan's summary by executing on a reading copy: a command that writes a file must not
    // write it then (docs/adr/0532 §4).
    let file = copy("iliskili.gpkg", "protocol");
    let mut d = world(&file);
    with_forms(&mut d);
    let before = std::fs::read(&file).expect("read");
    let relate = json!({ "relation": "parsel-malik", "action": "map" });
    let planned = through_the_protocol(&mut d, "cad.sources.relate", CallOp::Plan, relate.clone());
    assert_eq!(planned["status"], "completed", "{planned}");
    assert!(
        std::fs::read(&file).expect("read") == before,
        "relate's plan wrote the file"
    );
    let revision = d.revision();
    let done = through_the_protocol(&mut d, "cad.sources.relate", CallOp::Execute, relate);
    assert_eq!(done["status"], "completed", "{done}");
    assert!(
        std::fs::read(&file).expect("read") != before,
        "its execute writes it"
    );
    assert!(d.revision() > revision);
    let after_map = std::fs::read(&file).expect("read");
    // A write of a layer, and a submission with commit.
    let fifth = uid_of(&d, "import-parsel", "parsel_no", "5");
    ok(&run(
        &mut d,
        "cad.entities.set",
        CallOp::Execute,
        json!({ "uids": [fifth], "operation": "attributes", "attrs": { "ada": "999" } }),
    ));
    let sync = json!({ "layer": "import-parsel", "direction": "write", "with": ["import-malik"] });
    let planned = through_the_protocol(&mut d, "cad.sources.sync", CallOp::Plan, sync.clone());
    assert_eq!(planned["status"], "completed", "{planned}");
    assert!(
        std::fs::read(&file).expect("read") == after_map,
        "sync's plan wrote the file"
    );
    let submit_input = json!({
        "form": "malik-formu", "draft": true, "uid": OWNER, "values": { "tur": "Tüzel" },
        "children": [{ "relation": "parsel-malik", "uid": PARCEL_3 }], "commit": true
    });
    let planned = through_the_protocol(
        &mut d,
        "cad.forms.submit",
        CallOp::Plan,
        submit_input.clone(),
    );
    assert_eq!(planned["status"], "completed", "{planned}");
    assert!(
        std::fs::read(&file).expect("read") == after_map,
        "submit's plan wrote the file"
    );
    let done = through_the_protocol(&mut d, "cad.sources.sync", CallOp::Execute, sync);
    assert_eq!(done["status"], "completed", "{done}");
    assert!(std::fs::read(&file).expect("read") != after_map);
    std::fs::remove_file(&file).ok();
}
