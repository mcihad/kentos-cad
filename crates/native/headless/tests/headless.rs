//! The headless host end to end (docs/adr/0130; TODOS.md §14 acceptance):
//! a new project, a closed area, its measures, a layer change, a save and a
//! reading back; a group taken back whole; the commands it runs, refuses
//! and lists.

use std::path::PathBuf;

use kentos_contracts::{DrawingFont, Workspace};
use kentos_headless::{NewDrawing, Op, Session, catalog};
use serde_json::{Value, json};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-headless-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temporary directory");
    dir
}

fn new_drawing() -> Session {
    Session::new(&NewDrawing {
        name: "Ada 101".into(),
        srid: 5256,
        plot_scale: 1000.0,
        workspace: Workspace::Gis,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
    })
    .expect("a new project")
}

fn square(layer: &str) -> Value {
    json!({
        "layerId": layer,
        "pts": [
            {"x": 423500.0, "y": 4512300.0},
            {"x": 423520.0, "y": 4512300.0},
            {"x": 423520.0, "y": 4512312.5},
            {"x": 423500.0, "y": 4512312.5}
        ]
    })
}

fn created(answer: &Value) -> String {
    assert_eq!(answer["status"], "completed", "{answer}");
    answer["output"]["uid"].as_str().expect("its id").to_owned()
}

#[test]
fn a_polygon_is_made_measured_moved_to_a_layer_saved_and_read_back() {
    let mut s = new_drawing();
    let summary = s.summary();
    assert_eq!(summary["settings"]["srid"], 5256);
    assert_eq!(summary["objects"], 0);
    let active = summary["activeLayer"]
        .as_str()
        .expect("an active layer")
        .to_owned();

    // Planned first: nothing is written.
    let plan = s
        .run("cad.polygon.create", Some(1), Op::Plan, square(&active))
        .expect("runs");
    assert_eq!(plan["status"], "completed", "{plan}");
    assert_eq!(s.summary()["objects"], 0);

    let uid = created(
        &s.run("cad.polygon.create", None, Op::Execute, square(&active))
            .expect("runs"),
    );
    let m = s.measure(&uid).expect("measures");
    assert_eq!(m.kind, "polygon");
    assert_eq!(m.area, Some(250.0));
    assert_eq!(m.length, Some(65.0));

    // Onto another layer of the standard tree.
    let layers = serde_json::to_value(s.layers()).expect("the tree");
    let target = first_layer_other_than(&layers, &active).expect("another layer");
    let moved = s
        .run(
            "cad.entities.set",
            None,
            Op::Execute,
            json!({"uids": [uid], "operation": "layer", "layerId": target}),
        )
        .expect("runs");
    assert_eq!(moved["status"], "completed", "{moved}");
    assert_eq!(s.entity(&uid).expect("found")["entity"]["layerId"], target);

    let dir = scratch("save");
    let path = dir.join("ada.kcad");
    let saved = s.save(Some(&path)).expect("saves");
    assert!(saved["bytes"].as_u64().unwrap_or(0) > 0);
    assert_eq!(s.summary()["dirty"], false);

    let back = Session::open(&path).expect("reads back");
    assert_eq!(back.summary()["objects"], 1);
    assert_eq!(back.measure(&uid).expect("same id").area, Some(250.0));
    assert_eq!(
        back.entity(&uid).expect("found")["entity"]["layerId"],
        target
    );
    let _ = std::fs::remove_dir_all(dir);
}

fn first_layer_other_than(nodes: &Value, not: &str) -> Option<String> {
    for n in nodes.as_array()? {
        if n["type"] == "layer" && n["id"] != not && n["locked"] != true {
            return n["id"].as_str().map(str::to_owned);
        }
        if let Some(found) = first_layer_other_than(&n["children"], not) {
            return Some(found);
        }
    }
    None
}

#[test]
fn a_group_is_one_undo_step_and_a_cancelled_one_leaves_nothing() {
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();

    s.begin_group("Betik").expect("opens");
    for _ in 0..3 {
        created(
            &s.run("cad.polygon.create", None, Op::Execute, square(&layer))
                .expect("runs"),
        );
    }
    assert!(s.cancel_group());
    assert_eq!(
        s.summary()["objects"],
        0,
        "a failed script leaves nothing half done"
    );

    s.begin_group("Betik").expect("opens");
    for _ in 0..3 {
        created(
            &s.run("cad.polygon.create", None, Op::Execute, square(&layer))
                .expect("runs"),
        );
    }
    assert!(s.end_group());
    assert_eq!(s.summary()["objects"], 3);
    assert_eq!(s.undo().as_deref(), Some("Betik"));
    assert_eq!(s.summary()["objects"], 0, "one step takes all three back");
}

#[test]
fn a_refusal_is_the_commands_own_answer_and_the_hosts_failures_say_why() {
    let mut s = new_drawing();
    // The command refuses: its CommandResult says why.
    let refused = s
        .run(
            "cad.polygon.create",
            None,
            Op::Execute,
            square("yok-boyle-katman"),
        )
        .expect("answers");
    assert_eq!(refused["status"], "failed");
    assert_eq!(refused["error"]["code"], "layer_not_found");

    // The host refuses: a server command, an unknown one, input of another shape.
    let server = s
        .run("project.rename", None, Op::Execute, json!({}))
        .expect_err("the server's");
    assert_eq!(server.code, "server_command");
    let unknown = s
        .run("cad.nothing", None, Op::Execute, json!({}))
        .expect_err("unknown");
    assert_eq!(unknown.code, "unknown_command");
    let shape = s
        .run(
            "cad.polygon.create",
            None,
            Op::Execute,
            json!({"layerId": 3}),
        )
        .expect_err("not its type");
    assert_eq!(shape.code, "invalid_input");
    let version = s
        .run("cad.polygon.create", Some(99), Op::Execute, square("x"))
        .expect_err("no such version");
    assert_eq!(version.code, "unknown_version");
}

#[test]
fn every_command_marked_desktop_runs_here_and_the_catalog_lists_them_all() {
    let catalog = catalog();
    let commands = catalog["commands"].as_array().expect("commands");
    let desktop: Vec<(&str, u64)> = commands
        .iter()
        .filter(|c| {
            c["hosts"]
                .as_array()
                .is_some_and(|h| h.iter().any(|x| x == "desktop"))
        })
        .map(|c| {
            (
                c["id"].as_str().unwrap_or(""),
                c["version"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    let mut here: Vec<(&str, u64)> = kentos_headless::commands()
        .iter()
        .map(|(n, v)| (*n, u64::from(*v)))
        .collect();
    let mut listed = desktop.clone();
    here.sort_unstable();
    listed.sort_unstable();
    assert_eq!(here, listed);
    for c in commands {
        assert!(
            c["input"].is_object() && c["output"].is_object(),
            "{}: schemas",
            c["id"]
        );
    }
}

#[test]
fn pages_follow_the_drawing_and_a_cursor_reads_on() {
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let mut uids = Vec::new();
    for _ in 0..5 {
        uids.push(created(
            &s.run("cad.polygon.create", None, Op::Execute, square(&layer))
                .expect("runs"),
        ));
    }
    let first = s.entities(None, None, None, None, Some(2)).expect("a page");
    let got: Vec<&str> = first.items.iter().map(|i| i.uid.as_str()).collect();
    assert_eq!(
        got,
        uids[..2].iter().map(String::as_str).collect::<Vec<_>>()
    );
    let next = first.next.clone().expect("more");
    let rest = s
        .entities(None, None, None, Some(next), Some(10))
        .expect("the rest");
    assert_eq!(rest.items.len(), 3);
    assert!(rest.next.is_none());
    let none = s
        .entities(None, Some(vec!["line".into()]), None, None, None)
        .expect("a page");
    assert!(none.items.is_empty());
}

#[test]
fn a_drawing_another_host_owns_answers_the_same_requests_by_name() {
    use kentos_headless::rpc;
    let snapshot = kentos_project::new_project::new_project(&NewDrawing {
        name: "Konsol".into(),
        srid: 5256,
        plot_scale: 1000.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
    })
    .expect("a new project");
    let mut doc = kentos_domain::Document::from_snapshot(snapshot).expect("a document");
    let layer = doc.layers().active().to_owned();

    let answer = rpc::call(
        &mut doc,
        "run",
        &json!({"command": "cad.polygon.create", "op": "execute", "input": square(&layer)}),
    )
    .expect("runs");
    let uid = created(&answer);
    let summary = rpc::call(&mut doc, "summary", &json!({})).expect("a summary");
    assert_eq!(summary["objects"], 1);
    assert_eq!(summary["canUndo"], true);
    let measure = rpc::call(&mut doc, "measure", &json!({"uid": uid})).expect("measures");
    assert_eq!(measure["area"], 250.0);
    let page = rpc::call(
        &mut doc,
        "entities",
        &json!({"kinds": ["polygon"], "bbox": [423499, 4512299, 423501, 4512301], "limit": 5}),
    )
    .expect("a page");
    assert_eq!(page["items"][0]["uid"], uid.as_str());
    let one = rpc::call(&mut doc, "entity", &json!({"uid": uid})).expect("the object");
    assert_eq!(one["entity"]["kind"], "polygon");
    let layers = rpc::call(&mut doc, "layers", &json!(null)).expect("the tree");
    assert!(layers.as_array().is_some_and(|n| !n.is_empty()));

    let unknown = rpc::call(&mut doc, "undo", &json!({})).expect_err("the host's own");
    assert_eq!(unknown.code, "unknown_method");
    let shape =
        rpc::call(&mut doc, "entities", &json!({"bbox": [1, 2]})).expect_err("four numbers");
    assert_eq!(shape.code, "invalid_input");
    let missing = rpc::call(&mut doc, "run", &json!({"op": "execute"})).expect_err("no command");
    assert_eq!(missing.code, "invalid_input");
}

/// A CAD project in millimetres asked by name (docs/adr/0245): the room is
/// written in metres and measured in m²; the project says mm is its unit.
#[test]
fn a_millimetre_cad_project_is_drawn_and_measured_in_metres() {
    let choices = kentos_headless::new_drawing(
        &json!({ "name": "Oda", "workspace": "cad", "drawingUnit": "mm" }),
    )
    .expect("the wizard's project");
    let mut s = Session::new(&choices).expect("a new project");
    let summary = s.summary();
    assert_eq!(summary["settings"]["srid"], 0);
    assert_eq!(summary["settings"]["drawingUnit"], "mm");
    assert_eq!(
        summary["units"],
        json!({ "length": "m", "area": "m2", "drawing": "mm" })
    );
    let layer = summary["activeLayer"].as_str().expect("a layer").to_owned();
    let uid = created(
        &s.run(
            "cad.polygon.create",
            None,
            Op::Execute,
            json!({ "layerId": layer, "pts": [
                {"x": 0.0, "y": 0.0}, {"x": 4.0, "y": 0.0}, {"x": 4.0, "y": 3.0}, {"x": 0.0, "y": 3.0}
            ] }),
        )
        .expect("runs"),
    );
    let m = s.measure(&uid).expect("measures");
    assert_eq!((m.area, m.length), (Some(12.0), Some(14.0)));
    assert_eq!(
        serde_json::to_value(&m.units).expect("units"),
        json!({ "length": "m", "area": "m2", "drawing": "mm" })
    );
    // A geographic project's values are degrees.
    let wgs = kentos_headless::new_drawing(&json!({ "srid": 4326 })).expect("a project");
    let s = Session::new(&wgs).expect("a new project");
    assert_eq!(s.summary()["units"]["length"], "degree");
    // A unit for a project with a system is refused, not dropped.
    let refused = kentos_headless::new_drawing(&json!({ "srid": 5256, "drawingUnit": "cm" }))
        .expect_err("refused");
    assert_eq!(refused.code, "invalid_input");
}

/// docs/adr/0256 §4, §5: a measure says what it was taken in and how; a
/// batch gives each object, the sums and the basis once; the ellipsoid's
/// and the ground's need a system and a height; the summary gives the axes'
/// and angles' conventions.
#[test]
fn a_measure_says_its_basis_and_a_batch_sums() {
    let choices = kentos_headless::new_drawing(
        &json!({ "name": "Oda", "workspace": "cad", "drawingUnit": "mm" }),
    )
    .expect("the wizard's project");
    let mut s = Session::new(&choices).expect("a new project");
    let summary = s.summary();
    assert_eq!(
        summary["conventions"],
        json!({ "workspace": "cad", "axes": { "x": "east", "y": "north" }, "labels": { "x": "X", "y": "Y" },
                "polar": { "zero": "east", "turn": "ccw" }, "angleUnit": "deg" })
    );
    let layer = summary["activeLayer"].as_str().expect("a layer").to_owned();
    let room = created(
        &s.run(
            "cad.polygon.create",
            None,
            Op::Execute,
            json!({ "layerId": layer, "pts": [
                {"x": 0.0, "y": 0.0}, {"x": 4.0, "y": 0.0}, {"x": 4.0, "y": 3.0}, {"x": 0.0, "y": 3.0}
            ] }),
        )
        .expect("runs"),
    );
    let wall = created(
        &s.run(
            "cad.line.create",
            None,
            Op::Execute,
            json!({ "layerId": layer, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 0.0, "y": 2.5} }),
        )
        .expect("runs"),
    );
    let m = s.measure(&room).expect("measures");
    assert_eq!(
        serde_json::to_value(&m.basis).expect("basis"),
        json!({ "srid": 0, "system": "Yerel (koordinat sistemi yok)", "kind": "local", "frame": "world", "method": "plane" })
    );
    let many = s
        .measure_many(
            &[room.clone(), wall.clone()],
            kentos_headless::Method::Plane,
        )
        .expect("measures");
    assert_eq!(
        (many.totals.count, many.totals.area, many.totals.length),
        (2, 12.0, 16.5)
    );
    assert_eq!(many.items[1].area, None);
    // No system: no ellipsoid.
    let refused = s
        .measure_by(&room, kentos_headless::Method::Ellipsoid)
        .expect_err("no system");
    assert_eq!(refused.code, "method_unavailable");
    // Over the rpc, as the desktop's console and its agents ask.
    let mut doc = s.document().clone();
    let answer = kentos_headless::rpc::call(
        &mut doc,
        "measureMany",
        &json!({ "uids": [room, wall], "method": null }),
    )
    .expect("answers");
    assert_eq!(answer["totals"]["area"], 12.0);

    // A CBS project in TUREF / TM33: the plane's and the ellipsoid's, and the ground's with a height.
    let tm = kentos_headless::new_drawing(&json!({ "srid": 5256 })).expect("a project");
    let mut s = Session::new(&tm).expect("a new project");
    assert_eq!(
        s.summary()["conventions"]["labels"],
        json!({ "x": "Y", "y": "X" })
    );
    assert_eq!(
        s.summary()["conventions"]["polar"],
        json!({ "zero": "north", "turn": "cw" })
    );
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let (x, y) = (500_000.0, 4_400_000.0);
    let square = created(
        &s.run(
            "cad.polygon.create",
            None,
            Op::Execute,
            json!({ "layerId": layer, "pts": [
                {"x": x, "y": y}, {"x": x + 100.0, "y": y}, {"x": x + 100.0, "y": y + 100.0}, {"x": x, "y": y + 100.0}
            ] }),
        )
        .expect("runs"),
    );
    let plane = s.measure(&square).expect("plane");
    assert_eq!(plane.area, Some(10_000.0));
    let ellipsoid = s
        .measure_by(&square, kentos_headless::Method::Ellipsoid)
        .expect("ellipsoid");
    assert_eq!(ellipsoid.basis.kind, "projected");
    assert_eq!(ellipsoid.basis.method, kentos_headless::Method::Ellipsoid);
    // On the central meridian the projection's scale is k0 = 1: the ellipsoid's area is the plane's over k0².
    let area = ellipsoid.area.expect("an area");
    assert!((area - 10_000.0).abs() < 1.0, "{area}");
    let ground = s
        .measure_by(&square, kentos_headless::Method::Ground)
        .expect_err("no height");
    assert_eq!(ground.code, "method_unavailable");
}

// ── The protocol (docs/adr/0253) ────────────────────────────────────────

#[test]
fn a_call_answers_with_its_summary_and_a_page_gives_each_object_its_stamp() {
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let answer = s
        .call(json!({ "command": "cad.polygon.create", "input": square(&layer), "requestId": "betik:1" }))
        .expect("answers");
    let uid = created(&answer);
    assert_eq!(answer["summary"]["added"], json!([uid.clone()]));
    let call = answer["call"].as_u64().expect("a journal row");
    // The page's stamp is the summary's.
    let page = s.entities(None, None, None, None, None).expect("a page");
    let item = page.items.iter().find(|i| i.uid == uid).expect("listed");
    assert_eq!(
        answer["summary"]["stamps"]["objects"][&uid],
        json!(item.stamp)
    );
    // The summary of the drawing gives the parts' stamps.
    let stamps = &s.summary()["stamps"];
    for part in ["layers", "settings", "styles", "sheets"] {
        assert_eq!(stamps[part].as_str().map(str::len), Some(16), "{part}");
    }
    // Sent again: answered as before, applied once.
    let again = s
        .call(json!({ "command": "cad.polygon.create", "input": square(&layer), "requestId": "betik:1" }))
        .expect("answers");
    assert_eq!(again["replayed"], true);
    assert_eq!(s.summary()["objects"], 1);
    // The journal has the row, and replays on a new drawing.
    let journal = s.journal();
    assert_eq!(journal["format"], "kentos.replay/1");
    assert_eq!(journal["calls"][0]["seq"].as_u64(), Some(call));
    let mut other = new_drawing();
    let report = other.replay(journal).expect("replays");
    assert_eq!(
        (report["same"].as_u64(), report["failed"].as_u64()),
        (Some(1), Some(0))
    );
    assert_eq!(other.summary()["objects"], 1);
}

#[test]
fn a_command_runs_as_a_job_and_its_write_comes_in() {
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let started = s
        .start(json!({ "command": "cad.polygon.create", "input": square(&layer) }))
        .expect("starts");
    let id = started["id"].as_str().expect("an id").to_owned();
    assert!(matches!(
        started["phase"].as_str(),
        Some("queued" | "computing" | "staged" | "done")
    ));
    let done = s.wait(&id).expect("waits");
    assert_eq!(done["phase"], "done", "{done}");
    assert_eq!(s.summary()["objects"], 1);
    // Stopping it now: the write stands.
    let refused = s.cancel(&id).expect_err("refused");
    assert_eq!(refused.code, "already_committed");
    assert_eq!(s.job("yok").expect_err("unknown").code, "unknown_job");
}

#[test]
fn a_file_is_planned_without_writing_and_written_whole() {
    let dir = scratch("files");
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    created(
        &s.run("cad.polygon.create", None, Op::Execute, square(&layer))
            .expect("runs"),
    );
    let path = dir.join("cizim.dxf");
    let p = path.display().to_string();
    // The plan says what it would write; nothing is.
    let planned = s
        .call(json!({ "command": "file.export.dxf", "op": "plan", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(planned["status"], "completed", "{planned}");
    assert!(planned["output"]["bytes"].as_u64().is_some_and(|b| b > 0));
    assert_eq!(planned["output"]["objects"], 1);
    assert!(!path.exists());
    // Written: the bytes and the digest are the plan's.
    let written = s
        .call(json!({ "command": "file.export.dxf", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(written["output"]["sha256"], planned["output"]["sha256"]);
    let bytes = std::fs::read(&path).expect("written");
    assert_eq!(
        bytes.len() as u64,
        written["output"]["bytes"].as_u64().expect("bytes")
    );
    assert_eq!(
        kentos_contracts::encoding::sha256_hex(&bytes),
        written["output"]["sha256"].as_str().expect("sha256")
    );
    // No temporary file is left beside it.
    let left: Vec<_> = std::fs::read_dir(&dir)
        .expect("lists")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(left.len(), 1);
    // A file there is not written over unless asked; the journal names the file written.
    let again = s
        .call(json!({ "command": "file.export.dxf", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists");
    let journal = s.journal();
    let rows = journal["calls"].as_array().expect("rows");
    let row = rows
        .iter()
        .find(|r| r["command"] == "file.export.dxf")
        .expect("a row");
    assert_eq!(row["effects"][0]["path"], p);
    assert_eq!(row["effects"][0]["sha256"], written["output"]["sha256"]);
    // Replayed on another drawing: the file is planned again, not written, and is the same file.
    let before = std::fs::metadata(&path)
        .expect("there")
        .modified()
        .expect("a time");
    let mut other = new_drawing();
    let report = other.replay(journal.clone()).expect("replays");
    let file_row = report["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|r| r["command"] == "file.export.dxf")
        .expect("the file's row")
        .clone();
    assert_eq!(file_row["outcome"], "same", "{report}");
    assert_eq!(
        std::fs::metadata(&path)
            .expect("there")
            .modified()
            .expect("a time"),
        before
    );
    // A drawing that would write another file: it differs; nothing is written.
    let mut changed = new_drawing();
    created(
        &changed
            .run(
                "cad.point.create",
                None,
                Op::Execute,
                json!({ "layerId": layer, "p": { "x": 9.0, "y": 9.0 } }),
            )
            .expect("runs"),
    );
    let report = changed.replay(journal.clone()).expect("replays");
    let file_row = report["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .find(|r| r["command"] == "file.export.dxf")
        .expect("the row")
        .clone();
    assert_eq!(file_row["outcome"], "differs", "{report}");
    // The writes the drawing does not have: refused with their places.
    for (input, code, path) in [
        (json!({ "path": "goreli.dxf" }), "invalid_file_path", "path"),
        (
            json!({ "path": dir.join("yok/a.dxf").display().to_string() }),
            "invalid_file_path",
            "path",
        ),
        (
            json!({ "path": dir.display().to_string() }),
            "invalid_file_path",
            "path",
        ),
        (
            json!({ "path": dir.join("b.dxf").display().to_string(), "layers": ["yok"] }),
            "layer_not_found",
            "layers[0]",
        ),
    ] {
        let a = s
            .call(json!({ "command": "file.export.dxf", "input": input }))
            .expect("answers");
        assert_eq!(
            (a["error"]["code"].as_str(), a["error"]["path"].as_str()),
            (Some(code), Some(path)),
            "{a}"
        );
    }
    // A drawing without sheets has no PDF; a file command cannot go into a change set.
    let pdf = s
        .call(json!({ "command": "file.export.pdf", "input": { "path": dir.join("p.pdf").display().to_string() } }))
        .expect("answers");
    assert_eq!(pdf["error"]["code"], "no_sheets");
    let set = s
        .call(json!({ "command": "cad.changeset.apply", "input": { "steps": [{ "command": "file.export.dxf", "input": { "path": p } }] } }))
        .expect("answers");
    assert_eq!(set["error"]["code"], "side_effect_in_changeset");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A millimetre's detail on real coordinates (plan/cad.md `CAD-DATA-25`,
/// docs/specs/numeric-policy.md §7): a 1 mm square and two 1 mm lines in
/// TUREF / TM33, 4 412 km north. The square measures what its own corners
/// give, worked out here from their differences (exact in float64: the
/// corners are within a factor of two of each other); the trim meets the
/// crossing within a nanometre; a 0.25 mm offset is 0.25 mm; the `.kcad`
/// and the DXF give every coordinate back bit for bit.
#[test]
fn a_millimetre_detail_on_tm_coordinates_survives_measure_edits_and_files() {
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let (x, y): (f64, f64) = (487_512.345_678, 4_412_345.678_901);
    let mm: f64 = 0.001;
    let corners = [(x, y), (x + mm, y), (x + mm, y + mm), (x, y + mm)];
    let pts: Vec<Value> = corners
        .iter()
        .map(|(px, py)| json!({ "x": px, "y": py }))
        .collect();
    let square = created(
        &s.run(
            "cad.polygon.create",
            None,
            Op::Execute,
            json!({ "layerId": layer, "pts": pts }),
        )
        .expect("runs"),
    );
    // The sides as float64 holds them: x + 0.001 is the double nearest to it.
    let dx = (x + mm) - x;
    let dy = (y + mm) - y;
    assert!(
        (dx - mm).abs() < 1e-10 && (dy - mm).abs() < 1e-9,
        "{dx} {dy}"
    );
    let m = s.measure(&square).expect("measures");
    let area = m.area.expect("an area");
    assert!(
        (area - dx * dy).abs() <= 1e-18,
        "{area} against {}",
        dx * dy
    );
    let length = m.length.expect("a length");
    assert!((length - 2.0 * (dx + dy)).abs() <= 1e-15, "{length}");

    // Two 1 mm lines crossing 0.5 mm from the corner: the vertical one trimmed at the other.
    let across = created(
        &s.run(
            "cad.line.create",
            None,
            Op::Execute,
            json!({ "layerId": layer, "a": {"x": x, "y": y + 0.0005}, "b": {"x": x + mm, "y": y + 0.0005} }),
        )
        .expect("runs"),
    );
    let up = created(
        &s.run(
            "cad.line.create",
            None,
            Op::Execute,
            json!({ "layerId": layer, "a": {"x": x + 0.0005, "y": y}, "b": {"x": x + 0.0005, "y": y + mm} }),
        )
        .expect("runs"),
    );
    let trimmed = s
        .run(
            "cad.entities.trim",
            None,
            Op::Execute,
            json!({ "boundaries": { "kind": "objects", "uids": [across] },
                    "picks": [{ "uid": up, "at": {"x": x + 0.0005, "y": y + 0.0009} }] }),
        )
        .expect("runs");
    assert_eq!(trimmed["status"], "completed", "{trimmed}");
    let line = s.entity(&up).expect("the line");
    let (bx, by) = (
        line["entity"]["b"]["x"].as_f64().expect("x"),
        line["entity"]["b"]["y"].as_f64().expect("y"),
    );
    assert!(
        (bx - (x + 0.0005)).abs() <= 1e-9 && (by - (y + 0.0005)).abs() <= 1e-9,
        "{bx} {by}"
    );

    // Saved and opened again: every number is the same float64.
    let bytes = s.to_bytes().expect("saves");
    let again = Session::from_bytes(&bytes).expect("opens");
    for uid in [&square, &across, &up] {
        assert_eq!(
            again.entity(uid).expect("there"),
            s.entity(uid).expect("there"),
            "{uid}"
        );
    }

    // As a DXF and read back: the shortest decimal that gives the double back.
    let dir = scratch("large-coordinates");
    let path = dir.join("milimetre.dxf");
    s.export_dxf(&path).expect("writes");
    let read = kentos_formats::dxf::read(
        &std::fs::read(&path).expect("the file"),
        &kentos_contracts::DxfReadOptions::default(),
    )
    .expect("reads");
    let back = read
        .entities
        .iter()
        .find_map(|e| match e {
            kentos_contracts::Entity::Polygon(p) => Some(p.pts.clone()),
            _ => None,
        })
        .expect("the square");
    let want: Vec<kentos_contracts::Vec2> = corners
        .iter()
        .map(|(px, py)| kentos_contracts::Vec2 { x: *px, y: *py })
        .collect();
    assert_eq!(back, want);
}

#[test]
fn a_block_is_written_to_a_file_and_read_into_another_drawing() {
    // WBLOCK and its reading back (docs/adr/0316 §3): the block's objects as a drawing, its base at the origin,
    // brought into another drawing as one block; the objects named, with a base of their own; the refusals.
    let dir = scratch("wblock");
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let uid = created(
        &s.run("cad.polygon.create", None, Op::Execute, square(&layer))
            .expect("runs"),
    );
    let defined = s
        .call(json!({ "command": "cad.blocks.define", "input": {
            "name": "Kare", "base": { "x": 423500.0, "y": 4512300.0 }, "uids": [uid], "replace": false } }))
        .expect("answers");
    assert_eq!(defined["status"], "completed", "{defined}");
    let block = defined["output"]["block"].clone();
    let path = dir.join("kare.kcad");
    let p = path.display().to_string();
    // Planned: nothing written; written: the plan's bytes.
    let planned = s
        .call(json!({ "command": "file.export.block", "op": "plan", "input": { "path": p, "block": block } }))
        .expect("answers");
    assert_eq!(planned["status"], "completed", "{planned}");
    assert_eq!(planned["output"]["objects"], 1);
    assert!(!path.exists());
    let written = s
        .call(json!({ "command": "file.export.block", "input": { "path": p, "block": block } }))
        .expect("answers");
    assert_eq!(written["status"], "completed", "{written}");
    assert_eq!(written["output"]["sha256"], planned["output"]["sha256"]);
    let bytes = std::fs::read(&path).expect("written");
    let drawing = kentos_kcad::decode(&bytes).expect("a .kcad v2 drawing");
    assert_eq!(drawing.entities.len(), 1);
    // The base at the origin: the square's corner is (0, 0) in the file.
    let first = serde_json::to_value(&drawing.entities[0]).expect("JSON");
    assert_eq!(first["pts"][0], json!({ "x": 0.0, "y": 0.0 }), "{first}");
    // Read into another drawing as one block named after the file, one undo step.
    let mut other = new_drawing();
    let taken = other
        .call(json!({ "command": "file.import.block", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(taken["status"], "completed", "{taken}");
    let made = other
        .blocks()
        .into_iter()
        .find(|b| b.name == "kare")
        .expect("the block");
    assert_eq!(made.entities.len(), 1);
    assert_eq!(made.base, kentos_contracts::Vec2 { x: 0.0, y: 0.0 });
    // A file there is kept unless asked.
    let again = s
        .call(json!({ "command": "file.export.block", "input": { "path": p, "block": block } }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists", "{again}");
    // The objects named, with their own base.
    let q = dir.join("secilen.kcad").display().to_string();
    let chosen = s
        .call(json!({ "command": "file.export.block", "input": {
            "path": q, "uids": [], "base": { "x": 423510.0, "y": 4512300.0 } } }))
        .expect("answers");
    assert_eq!(chosen["error"]["code"], "no_entities", "{chosen}");
    // Refusals: neither a block nor objects, an unknown block, an object not there.
    for (input, code) in [
        (json!({ "path": q }), "no_block"),
        (
            json!({ "path": q, "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999" }),
            "unknown_block",
        ),
        (
            json!({ "path": q, "uids": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999"] }),
            "entity_not_found",
        ),
        (
            json!({ "path": "kare.kcad", "block": block }),
            "invalid_file_path",
        ),
    ] {
        let answer = s
            .call(json!({ "command": "file.export.block", "input": input }))
            .expect("answers");
        assert_eq!(answer["error"]["code"], code, "{answer}");
    }
    assert!(!std::path::Path::new(&q).exists());
}

#[test]
fn a_varied_dynamic_insert_goes_to_dxf_as_the_block_it_draws() {
    // DXF has no place for KentOS's parameters (docs/adr/0319 §5): an insert of the library's door 1.2 m wide and
    // left-handed is written as a plain block “Kapı V1” of what it draws; one at its defaults places “Kapı” as it is;
    // the report says the parameters were not written.
    let dir = scratch("dynamic-dxf");
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let library = s
        .call(json!({ "command": "cad.blocks.edit", "input": { "operation": "import", "name": "kentos-temel" } }))
        .expect("answers");
    assert_eq!(library["status"], "completed", "{library}");
    for (x, values) in [
        (423500.0, json!({ "Genişlik": 1.2, "Sol el": true })),
        (423505.0, json!({})),
        (423510.0, json!({ "Genişlik": 1.2, "Sol el": true })),
    ] {
        let placed = s
            .call(json!({ "command": "cad.blocks.insert", "input": {
                "name": "Kapı", "at": { "x": x, "y": 4512300.0 }, "layerId": layer, "parameters": values } }))
            .expect("answers");
        assert_eq!(placed["status"], "completed", "{placed}");
    }
    let path = dir.join("kapilar.dxf");
    let written = s
        .call(json!({ "command": "file.export.dxf", "input": { "path": path.display().to_string() } }))
        .expect("answers");
    assert_eq!(written["status"], "completed", "{written}");
    let notes = written["output"]["report"]["notes"].to_string();
    assert!(notes.contains("Dinamik blok"), "{notes}");
    assert!(notes.contains("“Kapı V1”"), "{notes}");
    let text = String::from_utf8_lossy(&std::fs::read(&path).expect("written")).into_owned();
    // Both blocks, the variant once though two inserts place it.
    assert_eq!(
        text.matches("  2\r\nKapı V1\r\n").count(),
        4,
        "BLOCK, its record and two INSERTs"
    );
    assert!(text.contains("  2\r\nKapı\r\n"));
    // Read back, the variant is a plain block whose leaf reaches 1.2 m from the hinge, to the left.
    let read = kentos_formats::dxf::read(
        &std::fs::read(&path).expect("written"),
        &kentos_contracts::DxfReadOptions::default(),
    )
    .expect("reads");
    let variant = read
        .blocks
        .iter()
        .find(|b| b.name == "Kapı V1")
        .expect("the variant block");
    assert!(variant.dynamic.is_none());
    let min_x = variant
        .entities
        .iter()
        .filter_map(|e| match e {
            kentos_contracts::Entity::Line(l) => Some(l.a.x.min(l.b.x)),
            _ => None,
        })
        .fold(f64::INFINITY, f64::min);
    assert!((min_x + 1.2).abs() < 1e-9, "{min_x}");
}

/// DXF HATCH round trip with independent files (docs/adr/0333 §4): each file of fixtures/formats/v1/dxf-hatch/in
/// (written by scripts/fixtures/dxf_hatch_cases.py without KentOS code) taken into a new local drawing and written
/// out again; the associative ones tie to their objects in the import's step. KENTOS_WRITE_DXF=1 writes the outputs
/// to out/, which the script checks against the inputs.
#[test]
fn dxf_hatches_go_round() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/dxf-hatch");
    let dir = scratch("dxf-hatch");
    let write = std::env::var_os("KENTOS_WRITE_DXF").is_some();
    for name in [
        "assoc-polyline",
        "assoc-lines",
        "edges",
        "gradients",
        "custom-pattern",
    ] {
        let mut s = Session::new(&NewDrawing {
            name: "Tarama".into(),
            srid: 0,
            plot_scale: 100.0,
            workspace: Workspace::Cad,
            drawing_font: DrawingFont::Barlow,
            province: None,
            drawing_unit: None,
        })
        .expect("a new project");
        let input = root.join("in").join(format!("{name}.dxf"));
        let answer = s
            .call(json!({ "command": "file.import.dxf", "input": { "path": input.display().to_string() } }))
            .expect("answers");
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        let hatches: Vec<&kentos_contracts::HatchEntity> = s
            .document()
            .entities()
            .filter_map(|e| match e {
                kentos_contracts::Entity::Hatch(h) => Some(h),
                _ => None,
            })
            .collect();
        if name.starts_with("assoc") {
            assert!(
                hatches.iter().all(|h| h.assoc.as_ref().is_some_and(|a| a
                    .objects()
                    .all(|id| kentos_contracts::import_placeholder_index(id).is_none()))),
                "{name}: tied to the objects' own ids: {:?} {answer}",
                hatches.iter().map(|h| &h.assoc).collect::<Vec<_>>()
            );
        }
        let out = dir.join(format!("{name}.dxf"));
        s.export_dxf(&out).expect("written");
        let bytes = std::fs::read(&out).expect("read back");
        let kept = root.join("out").join(format!("{name}.dxf"));
        if write {
            std::fs::create_dir_all(kept.parent().expect("a folder")).expect("the folder");
            std::fs::write(&kept, &bytes).expect("kept");
        } else {
            assert_eq!(
                std::fs::read(&kept).ok().as_deref(),
                Some(bytes.as_slice()),
                "{name}: KENTOS_WRITE_DXF=1 writes it; read the difference"
            );
        }
    }
}

#[test]
fn constraints_are_said_not_written_to_dxf() {
    // DXF gets the geometry as solved; KentOS's constraints and parameters are not written (docs/adr/0338 §4).
    let dir = scratch("constraints-dxf");
    let mut s = new_drawing();
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let line = s
        .call(json!({ "command": "cad.line.create", "input": {
            "layerId": layer, "a": { "x": 423500.0, "y": 4512300.0 }, "b": { "x": 423504.0, "y": 4512300.2 } } }))
        .expect("answers");
    assert_eq!(line["status"], "completed", "{line}");
    let uid = line["output"]["uid"].as_str().expect("a uid").to_owned();
    let seg =
        json!({ "uid": uid, "element": { "kind": "segment", "part": 0, "ring": 0, "index": 0 } });
    let start = json!({ "uid": uid, "element": { "kind": "point", "place": { "kind": "start" } } });
    let end = json!({ "uid": uid, "element": { "kind": "point", "place": { "kind": "end" } } });
    let added = s
        .call(json!({ "command": "cad.constraints.edit", "input": { "edit": { "kind": "add", "constraints": [
            { "rule": { "kind": "horizontal", "a": seg } },
            { "rule": { "kind": "distance", "a": start, "b": end, "parameter": "" }, "value": "5" } ] } } }))
        .expect("answers");
    assert_eq!(added["status"], "completed", "{added}");
    let path = dir.join("kisitli.dxf");
    let written = s
        .call(json!({ "command": "file.export.dxf", "input": { "path": path.display().to_string() } }))
        .expect("answers");
    assert_eq!(written["status"], "completed", "{written}");
    let notes = written["output"]["report"]["notes"].to_string();
    assert!(notes.contains("Kısıt"), "{notes}");
    assert!(notes.contains("1 parametre"), "{notes}");
}
