//! The drawing as a traceable GIS dataset, end to end (docs/adr/0530, plan/gis.md CBS-16): `file.export.gis` on an
//! arced parcel, a building with a block and curves; `cad.gis.derivation` against the drawing; the policy for the
//! dataset's tables (never bound, taken in as the chords they are). The independent reading of the written file, with
//! OGR, is `scripts/fixtures/gis_derivation_files.py`.

use std::path::PathBuf;

use kentos_headless::Session;
use serde_json::{Value, json};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-gis-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temporary directory");
    dir
}

fn drawing_path() -> String {
    format!(
        "{}/../../../fixtures/gis-derivation/v1/drawing.json",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn gis_drawing() -> Session {
    let bytes = std::fs::read(drawing_path()).expect("the drawing");
    Session::from_bytes(&bytes).expect("a drawing")
}

fn objects(s: &Session) -> Vec<Value> {
    s.document()
        .entities()
        .map(|e| serde_json::to_value(e).expect("JSON"))
        .collect()
}

#[test]
fn a_drawing_goes_out_as_a_traceable_gis_dataset() {
    // The layers of an arced parcel, a building with a block and curves as a GeoPackage of chords, each row with its
    // source's id, kind and deviation; the drawing is the source still, through a save and a reopening, and the
    // dataset is checked against it. With KENTOS_WRITE_GPKG=1 the file goes to fixtures/gis-derivation/v1/ada.gpkg
    // for scripts/fixtures/gis_derivation_files.py (OGR), which also makes the edited copies the shared cases read.
    let mut s = gis_drawing();
    let dir = scratch("export");
    let path = dir.join("ada.gpkg");
    let p = path.display().to_string();
    let before = s.summary()["revision"].clone();
    let source = objects(&s);
    // A plan writes nothing and says what would be written.
    let planned = s
        .call(json!({ "command": "file.export.gis", "op": "plan", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(planned["status"], "completed", "{planned}");
    assert!(!path.exists(), "plan writes nothing");
    let out = s
        .call(json!({ "command": "file.export.gis", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    let o = &out["output"];
    assert_eq!(o["algorithm"], "kentos.chords/1");
    assert_eq!(o["tolerance"], 0.0001);
    let tables: Vec<(String, String, u64)> = o["tables"]
        .as_array()
        .expect("tables")
        .iter()
        .map(|t| {
            (
                t["table"].as_str().expect("name").to_owned(),
                t["role"].as_str().expect("role").to_owned(),
                t["rows"].as_u64().expect("rows"),
            )
        })
        .collect();
    let want = [
        ("Parsel", "layer", 4),
        ("blok_Bina", "blocks", 2),
        ("Bina", "layer", 1),
        ("Yol", "layer", 4),
        ("kentos_annotations", "annotations", 1),
        ("Çizim", "layer", 5),
    ]
    .map(|(a, b, n)| (a.to_owned(), b.to_owned(), n));
    assert_eq!(tables, want);
    // Nine curved objects became chords, none further than the tolerance; the two construction lines were left out.
    assert_eq!(o["chorded"], 9, "{o}");
    assert!(
        o["maxDeviation"].as_f64().expect("deviation") <= 0.0001,
        "{o}"
    );
    let left: Vec<&str> = o["left"]
        .as_array()
        .expect("left")
        .iter()
        .map(|l| l["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(left, ["ray", "xline"]);
    assert_eq!(out["warnings"][0]["code"], "export_losses", "{out}");
    // The drawing is as it was: nothing of the derivation is written to it.
    assert_eq!(s.summary()["revision"], before);
    assert!(s.undo().is_none());
    assert_eq!(objects(&s), source);
    // The dataset matches the drawing it was made from.
    let checked = s
        .call(json!({ "command": "cad.gis.derivation", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(checked["status"], "completed", "{checked}");
    let c = &checked["output"];
    assert_eq!(c["current"], 17, "{c}");
    assert_eq!((&c["stale"], &c["edited"]), (&json!(0), &json!(0)), "{c}");
    assert_eq!(c["entries"], json!([]));
    assert_eq!(c["sameProject"], true);
    assert_eq!(checked["warnings"], json!([]));
    // A file there is kept unless asked; a tolerance out of its range, an unknown layer, nothing to write.
    for (input, code) in [
        (json!({ "path": p }), "file_exists"),
        (
            json!({ "path": p, "overwrite": true, "tolerance": 0.0 }),
            "invalid_tolerance",
        ),
        (
            json!({ "path": p, "overwrite": true, "tolerance": 2.0 }),
            "invalid_tolerance",
        ),
        (
            json!({ "path": p, "overwrite": true, "layers": ["yok"] }),
            "layer_not_found",
        ),
        (json!({ "path": "ada.gpkg" }), "invalid_file_path"),
        (
            json!({ "path": p, "overwrite": true, "layers": ["yazi"], "annotations": "skip" }),
            "nothing_to_export",
        ),
    ] {
        let answer = s
            .call(json!({ "command": "file.export.gis", "input": input }))
            .expect("answers");
        assert_eq!(answer["error"]["code"], code, "{answer}");
    }
    assert!(path.exists());
    // A finer tolerance cuts more chords and keeps its bound.
    let fine = dir.join("ince.gpkg").display().to_string();
    let fine_out = s
        .call(json!({ "command": "file.export.gis", "input": { "path": fine, "tolerance": 0.000001 } }))
        .expect("answers");
    assert_eq!(fine_out["status"], "completed", "{fine_out}");
    assert!(
        fine_out["output"]["maxDeviation"]
            .as_f64()
            .expect("deviation")
            <= 0.000001
    );
    assert!(fine_out["output"]["bytes"].as_u64() > o["bytes"].as_u64());
    // Without the texts and blocks as tables they are counted as left out.
    let skip = dir.join("atla.gpkg").display().to_string();
    let skipped = s
        .call(json!({ "command": "file.export.gis", "input": { "path": skip, "annotations": "skip" } }))
        .expect("answers");
    assert_eq!(skipped["status"], "completed", "{skipped}");
    let kinds: Vec<&str> = skipped["output"]["left"]
        .as_array()
        .expect("left")
        .iter()
        .map(|l| l["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(kinds, ["insert", "ray", "text", "xline"], "{skipped}");
    assert!(
        skipped["output"]["tables"]
            .as_array()
            .expect("tables")
            .iter()
            .all(|t| t["role"] == "layer")
    );
    // The arced source stays the source through a save and a reopening, and the dataset still matches it.
    let bytes = s.to_bytes().expect("a .kcad");
    let mut again = Session::from_bytes(&bytes).expect("reopens");
    let reread = objects(&again);
    assert_eq!(reread.len(), source.len());
    let parcel = reread
        .iter()
        .find(|e| e["kind"] == "polygon" && e["attrs"]["parsel_no"] == "7")
        .expect("the arced parcel");
    assert_eq!(parcel["bulges"], json!([0.0, 0.5, 0.0, 0.0]), "{parcel}");
    let checked = again
        .call(json!({ "command": "cad.gis.derivation", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(checked["output"]["current"], 17, "{checked}");
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        let to = format!(
            "{}/../../../fixtures/gis-derivation/v1/ada.gpkg",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::copy(&path, to).expect("kept");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_derivation_is_never_bound_back_and_is_told_what_it_is_when_taken_in() {
    // docs/adr/0530 §4: the derived tables are chords, the arcs are the drawing's. They are not bound as a layer, not
    // read into one, not written back; taken into another drawing they say what they are.
    let mut s = gis_drawing();
    let dir = scratch("policy");
    let p = dir.join("ada.gpkg").display().to_string();
    let out = s
        .call(json!({ "command": "file.export.gis", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    let before = s.summary()["revision"].clone();
    for layer in [None, Some("parsel")] {
        let mut link = json!({ "kind": "link", "path": p, "table": "Parsel" });
        if let Some(layer) = layer {
            link["layer"] = json!(layer);
        }
        let refused = s
            .call(json!({ "command": "cad.sources.edit", "input": { "edit": link } }))
            .expect("answers");
        assert_eq!(refused["error"]["code"], "derived_dataset", "{refused}");
    }
    assert_eq!(s.summary()["revision"], before);
    // The same file into the same drawing: the objects' ids are the drawing's.
    let same = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(same["error"]["code"], "entity_exists", "{same}");
    // Into a new drawing: the roads come in as polylines of chords, and the answer says they are a derivative.
    let mut other = Session::new(&kentos_headless::NewDrawing {
        name: "Ada 101".into(),
        srid: 5256,
        plot_scale: 1000.0,
        workspace: kentos_commands::Workspace::Gis,
        drawing_font: kentos_commands::DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
    })
    .expect("a new project");
    let taken = other
        .call(json!({ "command": "file.import.gpkg", "input": { "path": p, "tables": ["Yol"] } }))
        .expect("answers");
    assert_eq!(taken["status"], "completed", "{taken}");
    assert_eq!(taken["output"]["objects"], 4);
    let codes: Vec<&str> = taken["warnings"]
        .as_array()
        .expect("warnings")
        .iter()
        .map(|w| w["code"].as_str().expect("code"))
        .collect();
    assert!(codes.contains(&"derived_import"), "{taken}");
    // The roads' arcs are chords there: the arc of D-100 has become many straight edges of a polyline.
    let roads = objects(&other);
    let arc = roads
        .iter()
        .find(|e| e["attrs"]["ad"] == "D-100")
        .expect("the road");
    assert_eq!(arc["kind"], "polyline");
    assert!(arc["pts"].as_array().expect("points").len() > 20);
    assert!(arc.get("bulges").is_none(), "{arc}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_column_renamed_for_a_clash_is_still_checked_against_its_attribute() {
    // An attribute named as one of the derivation's own columns (`kentos_uid`) or as another in another case (`ADA` beside
    // the layer's `ada`) is written under a name of its own (`kentos_uid_2`, `ADA_2`) and said; the check reads it back by
    // the record's mapping, so it is current, and a change of it in the GIS is an edit of that column.
    let mut s = gis_drawing();
    let parcel = s
        .entities(
            Some("parsel".into()),
            Some(vec!["polygon".into()]),
            None,
            None,
            None,
        )
        .expect("objects")
        .items
        .first()
        .map(|r| r.uid.clone())
        .expect("a parcel");
    let set = s
        .call(json!({ "command": "cad.entities.set", "input": {
            "uids": [parcel], "operation": "attributes",
            "attrs": { "kentos_uid": "eski kimlik", "ADA": "büyük harfli" } } }))
        .expect("answers");
    assert_eq!(set["status"], "completed", "{set}");
    let dir = scratch("renamed");
    let p = dir.join("ada.gpkg").display().to_string();
    let out = s
        .call(json!({ "command": "file.export.gis", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    let said = out["warnings"][0]["message"].as_str().expect("a warning");
    assert!(
        said.contains("“kentos_uid” sütunu “kentos_uid_2”")
            && said.contains("“ADA” sütunu “ADA_2”"),
        "{said}"
    );
    let checked = s
        .call(json!({ "command": "cad.gis.derivation", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(checked["output"]["current"], 17, "{checked}");
    assert_eq!(checked["output"]["edited"], 0, "{checked}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "timing, run by hand: cargo test --release -p kentos-headless --test gis gis_timing -- --ignored --nocapture"]
fn gis_timing() {
    // docs/adr/0530 Doğrulama: 100 000 arced parcels, 20 000 circles and 20 000 ellipses written as a derived dataset
    // and checked against the drawing: the plan, the export and the check.
    use std::time::Instant;
    let mut base: Value =
        serde_json::from_slice(&std::fs::read(drawing_path()).expect("the drawing")).expect("JSON");
    let mut entities = Vec::new();
    let mut id = 0u32;
    let mut add = |mut e: Value| {
        id += 1;
        e["id"] = json!(id);
        entities.push(e);
    };
    for i in 0..100_000u32 {
        let (x, y) = (
            487_000.0 + f64::from(i % 400) * 50.0,
            4_420_000.0 + f64::from(i / 400) * 50.0,
        );
        add(json!({ "kind": "polygon", "layerId": "parsel",
            "attrs": { "ada": format!("{}", i / 100), "parsel_no": format!("{}", i % 100) },
            "pts": [{"x": x, "y": y}, {"x": x + 40.0, "y": y}, {"x": x + 40.0, "y": y + 30.0}, {"x": x, "y": y + 30.0}],
            "bulges": [0.0, 0.4, 0.0, 0.0] }));
    }
    for i in 0..20_000u32 {
        let (x, y) = (
            487_000.0 + f64::from(i % 400) * 50.0,
            4_419_000.0 - f64::from(i / 400) * 50.0,
        );
        add(
            json!({ "kind": "circle", "layerId": "cizim", "attrs": {}, "c": {"x": x, "y": y}, "r": 3.0 }),
        );
        add(
            json!({ "kind": "ellipse", "layerId": "cizim", "attrs": {}, "c": {"x": x + 20.0, "y": y},
            "major": {"x": 6.0, "y": 0.0}, "ratio": 0.5, "t0": 0, "t1": std::f64::consts::TAU }),
        );
    }
    let count = entities.len();
    base["entities"] = json!(entities);
    base["blocks"] = json!([]);
    let t = Instant::now();
    let mut s = Session::from_bytes(base.to_string().as_bytes()).expect("a drawing");
    println!("open ({count} objects): {:?}", t.elapsed());
    let dir = scratch("timing");
    let p = dir.join("buyuk.gpkg").display().to_string();
    let t = Instant::now();
    let planned = s
        .call(json!({ "command": "file.export.gis", "op": "plan", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(planned["status"], "completed", "{planned}");
    println!("plan: {:?}", t.elapsed());
    let t = Instant::now();
    let out = s
        .call(json!({ "command": "file.export.gis", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    println!(
        "export: {:?}, {} bytes, max deviation {}",
        t.elapsed(),
        out["output"]["bytes"],
        out["output"]["maxDeviation"]
    );
    let t = Instant::now();
    let checked = s
        .call(json!({ "command": "cad.gis.derivation", "input": { "path": p } }))
        .expect("answers");
    println!(
        "check: {:?}, current {}",
        t.elapsed(),
        checked["output"]["current"]
    );
    assert_eq!(checked["output"]["current"], count);
    let _ = std::fs::remove_dir_all(&dir);
}
