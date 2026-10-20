//! The headless host end to end (docs/adr/0130; TODOS.md §14 acceptance):
//! a new project, a closed area, its measures, a layer change, a save and a
//! reading back; a group taken back whole; the commands it runs, refuses
//! and lists.

use std::path::PathBuf;

use kentos_commands::{DrawingFont, Workspace};
use kentos_headless::{NewDrawing, Op, Session, catalog};
use serde_json::{Value, json};

#[path = "headless/atlas_cases.rs"]
mod atlas_cases;

#[path = "headless/maplibre.rs"]
mod maplibre;

#[path = "headless/own_file.rs"]
mod own_file;

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
        kentos_commands::encoding::sha256_hex(&bytes),
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
        &kentos_commands::DxfReadOptions::default(),
    )
    .expect("reads");
    let back = read
        .entities
        .iter()
        .find_map(|e| match e {
            kentos_commands::Entity::Polygon(p) => Some(p.pts.clone()),
            _ => None,
        })
        .expect("the square");
    let want: Vec<kentos_commands::Vec2> = corners
        .iter()
        .map(|(px, py)| kentos_commands::Vec2 { x: *px, y: *py })
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
    assert_eq!(made.base, kentos_commands::Vec2 { x: 0.0, y: 0.0 });
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
        &kentos_commands::DxfReadOptions::default(),
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
            kentos_commands::Entity::Line(l) => Some(l.a.x.min(l.b.x)),
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
        let hatches: Vec<&kentos_commands::HatchEntity> = s
            .document()
            .entities()
            .filter_map(|e| match e {
                kentos_commands::Entity::Hatch(h) => Some(h),
                _ => None,
            })
            .collect();
        if name.starts_with("assoc") {
            assert!(
                hatches.iter().all(|h| h.assoc.as_ref().is_some_and(|a| a
                    .objects()
                    .all(|id| kentos_commands::import_placeholder_index(id).is_none()))),
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

/// Every object of the drawing as the wire carries it, by persistent id, without its slot.
fn objects_by_uid(s: &Session) -> std::collections::BTreeMap<String, Value> {
    let page = s
        .entities(None, None, None, None, Some(10_000))
        .expect("a page");
    let page = serde_json::to_value(&page).expect("JSON");
    let mut out = std::collections::BTreeMap::new();
    for item in page["items"].as_array().expect("items") {
        let mut e = item.clone();
        let uid = e["uid"].as_str().expect("a uid").to_owned();
        if let Some(o) = e.as_object_mut() {
            o.remove("id");
            if let Some(entity) = o.get_mut("entity").and_then(Value::as_object_mut) {
                entity.remove("id");
            }
        }
        out.insert(uid, e);
    }
    out
}

#[test]
fn a_geopackage_goes_in_and_out_keeping_ids_values_and_arcs() {
    // docs/adr/0502: QGIS's file into a drawing, the drawing out as a GeoPackage, that into another drawing: the same
    // objects with the same lasting ids; plan writes nothing; the refusals.
    let dir = scratch("gpkg");
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    assert_eq!(took["output"]["objects"], 10, "{took}");
    let first = objects_by_uid(&s);
    let out = dir.join("teslim.gpkg");
    let p = out.display().to_string();
    let planned = s
        .call(json!({ "command": "file.export.gpkg", "op": "plan", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(planned["status"], "completed", "{planned}");
    assert!(!out.exists());
    let written = s
        .call(json!({ "command": "file.export.gpkg", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(written["status"], "completed", "{written}");
    assert_eq!(
        written["output"]["tables"].as_array().map(Vec::len),
        Some(6),
        "{written}"
    );
    assert!(
        written["warnings"].as_array().is_some_and(Vec::is_empty),
        "{written}"
    );
    assert!(out.exists());
    let again = s
        .call(json!({ "command": "file.export.gpkg", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists", "{again}");
    let unknown = s
        .call(json!({ "command": "file.export.gpkg", "input": { "path": p, "layers": ["yok"], "overwrite": true } }))
        .expect("answers");
    assert_eq!(unknown["error"]["code"], "layer_not_found", "{unknown}");
    let mut t = new_drawing();
    let back = t
        .call(json!({ "command": "file.import.gpkg", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(back["status"], "completed", "{back}");
    let second = objects_by_uid(&t);
    assert_eq!(first.len(), second.len());
    for (uid, a) in &first {
        let b = second
            .get(uid)
            .unwrap_or_else(|| panic!("{uid} geri gelmedi"));
        // An arc's bulge goes out through its middle point and back: within a few units in the last place.
        let (sa, sb) = (a.to_string(), b.to_string());
        if sa != sb {
            let (ea, eb) = (&a["entity"], &b["entity"]);
            assert_eq!(ea["kind"], eb["kind"], "{uid}");
            assert_eq!(ea["attrs"], eb["attrs"], "{uid}");
            assert_eq!(ea["pts"], eb["pts"], "{uid}");
            for (x, y) in ea["bulges"]
                .as_array()
                .into_iter()
                .flatten()
                .zip(eb["bulges"].as_array().into_iter().flatten())
            {
                let (x, y) = (x.as_f64().expect("bulge"), y.as_f64().expect("bulge"));
                assert!((x - y).abs() <= 1e-12, "{uid}: {x} ≠ {y}");
            }
        }
    }
    if std::env::var_os("KENTOS_WRITE_GPKG").is_some() {
        let keep = format!(
            "{}/../../../fixtures/geopackage/v1/kentos-export.gpkg",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::copy(&out, keep).expect("copied");
    }
}

#[test]
fn a_project_is_packed_with_its_files_and_opens_where_the_folder_goes() {
    // docs/adr/0506: a bound GeoPackage, a linked picture (relative to the project's folder), a raster with its world
    // file, a missing file and a database's table; the package copies the files once each, rewrites the links, says
    // what it left; moved elsewhere and opened, the bound layer finds its copy again and matches it.
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    let dir = scratch("package");
    let work = dir.join("is");
    std::fs::create_dir_all(work.join("foto")).expect("folders");
    let gpkg = work.join("kadastro.gpkg");
    std::fs::copy(fixtures.join("geopackage/v1/qgis.gpkg"), &gpkg).expect("copied");
    std::fs::copy(
        fixtures.join("interaction/v1/logo.png"),
        work.join("foto/logo.png"),
    )
    .expect("copied");
    // The project's own profile and mapping beside the project file (docs/adr/0558): copied byte for byte under
    // `profiller/`; another file, a folder inside it and a file that cannot be read are not.
    let profiles = fixtures.join("profiles/v1/files");
    std::fs::create_dir_all(work.join("profiller/eski")).expect("folders");
    for (from, to) in [
        ("park-bahce.profil.json", "park-bahce.profil.json"),
        ("sivas-bel.eslem.json", "sivas-bel.eslem.json"),
        ("peyzaj.profil.json", "eski/peyzaj.profil.json"),
        ("bozuk.json", "notlar.txt"),
    ] {
        std::fs::copy(profiles.join(from), work.join("profiller").join(to)).expect("copied");
    }
    #[cfg(unix)]
    let unreadable = {
        use std::os::unix::fs::PermissionsExt;
        let path = work.join("profiller/kilitli.profil.json");
        std::fs::copy(profiles.join("peyzaj.profil.json"), &path).expect("copied");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("locked");
        // A process that may read it anyway (root) leaves nothing out: the test then expects the copy.
        std::fs::File::open(&path).is_err()
    };
    #[cfg(not(unix))]
    let unreadable = false;
    let mut s = new_drawing();
    let linked = s
        .call(json!({ "command": "cad.sources.edit", "input": { "edit": {
            "kind": "link", "path": gpkg.display().to_string(), "table": "parsel" } } }))
        .expect("answers");
    assert_eq!(linked["status"], "completed", "{linked}");
    // Pictures and a database's link: written into the drawing as a project file keeps them.
    let mut snapshot = serde_json::to_value(s.document().to_snapshot_v2()).expect("json");
    let layer = snapshot["activeLayer"].clone();
    let raster = fixtures
        .join("raster/v1/files/rgb.png")
        .display()
        .to_string();
    let picture = |id: u64, file: &str| {
        json!({ "kind": "image", "id": id, "layerId": layer, "attrs": {},
        "p": { "x": 0.0, "y": 0.0 }, "width": 10.0, "height": 10.0, "rotation": 0.0, "file": file })
    };
    let next = snapshot["entities"].as_array().map_or(0, Vec::len) as u64 + 100;
    let list = snapshot["entities"].as_array_mut().expect("entities");
    list.push(picture(next, "foto/logo.png"));
    list.push(picture(next + 1, "foto/logo.png"));
    list.push(picture(next + 2, "/yok/kayip.jpg"));
    list.push(json!({ "kind": "raster", "id": next + 3, "layerId": layer, "attrs": {},
        "affine": [0.0, 1.0, 0.0, 0.0, 0.0, -1.0], "width": 4, "height": 4, "bands": 3, "sample": "u8",
        "file": raster, "srid": 5256, "style": { "render": "rgb", "bands": [1, 2, 3] } }));
    let uids = snapshot["uids"].as_array_mut().expect("uids");
    for k in 0..4 {
        uids.push(json!(format!(
            "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{:04x}",
            0x3000 + k
        )));
    }
    let mut db = snapshot["settings"]["sources"][0].clone();
    db["id"] = json!("kaynak-2");
    db["kind"] = json!("postgis");
    db["path"] = json!("postgresql://cbs@veri.belediye.local/kent");
    db["layer"] = json!("cizim");
    snapshot["settings"]["sources"]
        .as_array_mut()
        .expect("sources")
        .push(db);
    let typed: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(snapshot).expect("a snapshot");
    let project = work.join("proje.kcad");
    std::fs::write(
        &project,
        kentos_kcad::encode_verified(&typed).expect("encoded"),
    )
    .expect("written");
    let mut s = Session::open(&project).expect("opens");
    let out = dir.join("teslim");
    let input = json!({ "path": out.display().to_string(), "name": "kadastro", "from": work.display().to_string() });
    let planned = s
        .call(json!({ "command": "file.package", "op": "plan", "input": input }))
        .expect("answers");
    assert_eq!(planned["status"], "completed", "{planned}");
    assert!(!out.exists(), "plan writes nothing");
    let done = s
        .call(json!({ "command": "file.package", "input": input }))
        .expect("answers");
    assert_eq!(done["status"], "completed", "{done}");
    let files: Vec<(String, String, u64)> = done["output"]["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|f| {
            (
                f["path"].as_str().unwrap_or_default().to_owned(),
                f["kind"].as_str().unwrap_or_default().to_owned(),
                f["links"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    let mut want: Vec<(String, String, u64)> = vec![
        ("veri/logo.png".into(), "picture".into(), 2),
        ("veri/rgb.png".into(), "picture".into(), 1),
        ("veri/rgb.pgw".into(), "sidecar".into(), 1),
        ("veri/kadastro.gpkg".into(), "source".into(), 1),
    ];
    if !unreadable {
        want.push(("profiller/kilitli.profil.json".into(), "profile".into(), 0));
    }
    want.push(("profiller/park-bahce.profil.json".into(), "profile".into(), 0));
    want.push(("profiller/sivas-bel.eslem.json".into(), "mapping".into(), 0));
    assert_eq!(files, want, "{done}");
    let left: Vec<String> = done["output"]["left"]
        .as_array()
        .expect("left")
        .iter()
        .map(|l| {
            format!(
                "{}:{}",
                l["why"].as_str().unwrap_or_default(),
                l["path"].as_str().unwrap_or_default()
            )
        })
        .collect();
    let mut want_left = vec![
        "missing:/yok/kayip.jpg".to_owned(),
        "database:postgresql://cbs@veri.belediye.local/kent".to_owned(),
    ];
    if unreadable {
        want_left.push(format!(
            "unreadable:{}",
            work.join("profiller/kilitli.profil.json").display()
        ));
    }
    assert_eq!(left, want_left);
    assert_eq!(done["warnings"][0]["code"], "package_left", "{done}");
    for f in &files {
        assert!(out.join(&f.0).is_file(), "{}", f.0);
    }
    // The profiles are the sources' bytes, the folder's other files are not in the package.
    for name in ["park-bahce.profil.json", "sivas-bel.eslem.json"] {
        assert_eq!(
            std::fs::read(out.join("profiller").join(name)).expect("copy"),
            std::fs::read(profiles.join(name)).expect("source"),
            "{name} byte for byte"
        );
    }
    assert!(!out.join("profiller/notlar.txt").exists());
    assert!(!out.join("profiller/eski").exists());
    let again = s
        .call(json!({ "command": "file.package", "input": input }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists", "{again}");
    // The folder moved and the old data gone: the bound layer finds its copy by its relative path.
    let moved = dir.join("baska-bilgisayar");
    std::fs::rename(&out, &moved).expect("moved");
    std::fs::remove_file(&gpkg).expect("old data gone");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            work.join("profiller/kilitli.profil.json"),
            std::fs::Permissions::from_mode(0o644),
        );
    }
    let mut opened = Session::open(&moved.join("kadastro.kcad")).expect("opens");
    assert_eq!(opened.located().moved.len(), 1, "{:?}", opened.located());
    let state = opened
        .call(json!({ "command": "cad.sources.describe", "input": {} }))
        .expect("answers");
    let link = &state["output"]["links"][0];
    assert_eq!(
        (
            link["found"].clone(),
            link["changed"].clone(),
            link["edited"].clone()
        ),
        (json!(true), json!(false), json!(false)),
        "{state}"
    );
    assert_eq!(
        link["link"]["path"],
        moved.join("veri/kadastro.gpkg").display().to_string()
    );
    let pictures: Vec<String> = opened
        .document()
        .to_snapshot_v2()
        .entities
        .iter()
        .filter_map(|e| {
            serde_json::to_value(e)
                .ok()?
                .get("file")?
                .as_str()
                .map(str::to_owned)
        })
        .collect();
    assert_eq!(
        pictures,
        [
            "veri/logo.png",
            "veri/logo.png",
            "/yok/kayip.jpg",
            "veri/rgb.png"
        ]
    );
    // Where the folder went the project's profiles and mapping are beside the project file again (docs/adr/0558).
    let listed = opened
        .call(json!({ "command": "cad.profiles.list", "input": {
            "drawing": moved.join("kadastro.kcad").display().to_string() } }))
        .expect("answers");
    let kept: Vec<(String, String, bool)> = listed["output"]["items"]
        .as_array()
        .expect("items")
        .iter()
        .filter(|i| i["source"] == "project")
        .map(|i| {
            (
                i["name"].as_str().unwrap_or_default().to_owned(),
                i["kind"].as_str().unwrap_or_default().to_owned(),
                i.get("problem").is_none(),
            )
        })
        .collect();
    let mut want_kept: Vec<(String, String, bool)> = Vec::new();
    if !unreadable {
        // The locked file holds the `peyzaj` profile under another name: listed, with its problem.
        want_kept.push(("kilitli".into(), "profile".into(), false));
    }
    want_kept.push(("park-bahce".into(), "profile".into(), true));
    want_kept.push(("sivas-bel".into(), "mapping".into(), true));
    assert_eq!(kept, want_kept, "{listed}");
    let refused = s
        .call(json!({ "command": "file.package", "input": { "path": "goreli/klasor" } }))
        .expect("answers");
    assert_eq!(refused["error"]["code"], "invalid_file_path", "{refused}");
    // scripts/fixtures/package_cases.py reads the moved package with the independent KCAD reader and hashlib.
    if let Ok(out) = std::env::var("KENTOS_PACKAGE_OUT") {
        std::fs::write(
            out,
            json!({ "package": moved.display().to_string() }).to_string(),
        )
        .expect("written");
    }
}

#[test]
fn a_package_carries_the_references_sources_and_says_the_fonts() {
    // docs/adr/0350 §4: an external reference's source copied byte for byte (its digest holds, so the moved package's
    // reference is ok), an image found by the project's search path, a text style naming an SHX font left out, and
    // paket.json listing all of it.
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures");
    let dir = scratch("transmit");
    let work = dir.join("is");
    std::fs::create_dir_all(work.join("mimari")).expect("folders");
    std::fs::create_dir_all(dir.join("ortak")).expect("folders");
    let plan = work.join("mimari/plan.kcad");
    std::fs::copy(fixtures.join("xref/v1/plan.kcad"), &plan).expect("copied");
    std::fs::copy(fixtures.join("interaction/v1/logo.png"), dir.join("ortak/logo.png")).expect("copied");
    let project = work.join("ofis.kcad");
    let mut s = new_drawing();
    let layer = serde_json::to_value(s.document().to_snapshot_v2()).expect("json")["activeLayer"].clone();
    let attached = s
        .call(json!({ "command": "cad.xrefs.edit", "input": {
            "operation": { "kind": "attach", "path": plan.display().to_string(), "layerId": layer },
            "drawing": project.display().to_string() } }))
        .expect("answers");
    assert_eq!(attached["status"], "completed", "{attached}");
    let mut snapshot = serde_json::to_value(s.document().to_snapshot_v2()).expect("json");
    let next = snapshot["entities"].as_array().map_or(0, Vec::len) as u64 + 100;
    snapshot["entities"].as_array_mut().expect("entities").push(json!({ "kind": "image", "id": next,
        "layerId": layer, "attrs": {}, "p": { "x": 0.0, "y": 0.0 }, "width": 4.0, "height": 3.0, "rotation": 0.0,
        "file": "/eski/makine/logo.png" }));
    snapshot["uids"].as_array_mut().expect("uids").push(json!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3100"));
    snapshot["settings"]["xrefPaths"] = json!(["../ortak"]);
    snapshot["settings"]["textStyles"] = json!([{ "id": "s1", "name": "Pafta", "font": "barlow", "fontFile": "romans.shx" }]);
    let typed: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(snapshot).expect("a snapshot");
    std::fs::write(&project, kentos_kcad::encode_verified(&typed).expect("encoded")).expect("written");
    let mut s = Session::open(&project).expect("opens");
    let out = dir.join("teslim");
    let done = s
        .call(json!({ "command": "file.package", "input": {
            "path": out.display().to_string(), "name": "ofis", "from": work.display().to_string() } }))
        .expect("answers");
    assert_eq!(done["status"], "completed", "{done}");
    let kinds: Vec<(String, String)> = done["output"]["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|f| (f["path"].as_str().unwrap_or_default().to_owned(), f["kind"].as_str().unwrap_or_default().to_owned()))
        .collect();
    assert_eq!(
        kinds,
        [("veri/logo.png".to_owned(), "picture".to_owned()), ("veri/plan.kcad".to_owned(), "xref".to_owned())],
        "{done}"
    );
    // The plan's own nested reference's source is not on this computer: said, as the font drawn in another face.
    assert_eq!(
        done["output"]["left"],
        json!([
            { "kind": "xref", "path": "/proje/detay.kcad", "why": "missing", "at": "xref:plan|Detay" },
            { "kind": "font", "path": "romans.shx", "why": "substituted", "at": "stil:Pafta" }
        ])
    );
    // No text in the drawing: no face drawn.
    assert_eq!(done["output"]["faces"], Value::Null, "{done}");
    let manifest = out.join("paket.json");
    assert_eq!(done["output"]["manifest"], manifest.display().to_string());
    let text = std::fs::read(&manifest).expect("the manifest");
    assert_eq!(done["output"]["manifestSha256"], kentos_commands::encoding::sha256_hex(&text));
    // Moved elsewhere, the reference is found where the package put it, its digest the same.
    let moved = dir.join("baska-bilgisayar");
    std::fs::rename(&out, &moved).expect("moved");
    std::fs::remove_file(&plan).expect("old source gone");
    let mut opened = Session::open(&moved.join("ofis.kcad")).expect("opens");
    let checked = opened
        .call(json!({ "command": "cad.assets.check", "input": { "drawing": moved.join("ofis.kcad").display().to_string() } }))
        .expect("answers");
    let states: Vec<(String, String, String)> = checked["output"]["assets"]
        .as_array()
        .expect("assets")
        .iter()
        .map(|a| {
            (
                a["kind"].as_str().unwrap_or_default().to_owned(),
                a["via"].as_str().unwrap_or_default().to_owned(),
                a["state"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        states,
        [
            ("xref".to_owned(), "none".to_owned(), "missing".to_owned()),
            ("xref".to_owned(), "relative".to_owned(), "ok".to_owned()),
            ("image".to_owned(), "relative".to_owned(), "ok".to_owned())
        ],
        "{checked}"
    );
    // scripts/fixtures/package_cases.py reads this package too.
    if let Ok(out) = std::env::var("KENTOS_TRANSMIT_OUT") {
        std::fs::write(out, json!({ "package": moved.display().to_string() }).to_string()).expect("written");
    }
}

/// A room's architectural objects as `arch_dxf_goes_round` writes them: grids and a column where they cross, four
/// walls, a door, the space they close, a tag on the door, a section mark, a stair and a ramp.
fn arch_room() -> Session {
    let mut s = Session::new(&NewDrawing {
        name: "Mimari DXF".into(),
        srid: 0,
        plot_scale: 50.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
    })
    .expect("a new project");
    let layer = s.summary()["activeLayer"].as_str().expect("a layer").to_owned();
    let mut call = |command: &str, input: Value| -> Value {
        let answer = s.call(json!({ "command": command, "input": input })).expect("answers");
        assert_eq!(answer["status"], "completed", "{command}: {answer}");
        answer["output"].clone()
    };
    let p = |x: f64, y: f64| json!({ "x": x, "y": y });
    let grids = call("cad.grids.create", json!({ "layerId": layer, "grids": [
        { "label": "A", "pts": [p(0.0, -1.5), p(0.0, 5.5)] },
        { "label": "1", "pts": [p(-1.5, 0.0), p(7.5, 0.0)] } ] }));
    let g: Vec<String> = grids["uids"].as_array().expect("ids").iter().map(|u| u.as_str().expect("an id").to_owned()).collect();
    call("cad.columns.create", json!({ "layerId": layer, "crossings": { "grids": g, "width": 0.3, "depth": 0.3 } }));
    let walls = call("cad.walls.create", json!({ "layerId": layer, "walls": [
        { "pts": [p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)], "closed": true, "width": 0.2 } ] }));
    let wall = walls["uids"][0].as_str().expect("a wall").to_owned();
    let door = call("cad.openings.create", json!({ "layerId": layer, "openings": [
        { "host": wall, "at": 2.0, "type": "Kapı 90", "number": "K1" } ] }));
    let door = door["uids"][0].as_str().expect("a door").to_owned();
    call("cad.spaces.create", json!({ "layerId": layer, "spaces": [
        { "seed": p(3.0, 2.0), "number": "Z-01", "name": "Salon" } ] }));
    call("cad.tags.create", json!({ "layerId": layer, "tags": [{ "of": door }] }));
    call("cad.viewmarks.create", json!({ "layerId": layer, "marks": [
        { "form": "section", "mark": "A", "points": [p(-1.0, 2.0), p(7.0, 2.0)] } ] }));
    call("cad.stairs.create", json!({ "layerId": layer, "stairs": [
        { "pts": [p(1.0, 1.0), p(4.0, 1.0)], "width": 1.0, "risers": 10, "riserHeight": 0.17, "tread": 0.28 } ] }));
    call("cad.ramps.create", json!({ "layerId": layer, "ramps": [
        { "pts": [p(8.0, 0.0), p(14.0, 0.0)], "width": 1.5, "rise": 0.3, "slope": 5.0 } ] }));
    s
}

fn arch_kind(e: &kentos_commands::Entity) -> bool {
    use kentos_commands::Entity as E;
    matches!(
        e,
        E::Wall(_) | E::Opening(_) | E::Space(_) | E::Grid(_) | E::Column(_) | E::Stair(_) | E::Ramp(_) | E::Tag(_) | E::ViewMark(_)
    )
}

#[test]
fn arch_dxf_goes_round() {
    // Every architectural object goes to DXF as its pieces' block and an INSERT carrying it, and comes back as itself,
    // its references to the others tied to their new ids (docs/adr/0347 §2).
    let dir = scratch("arch-dxf");
    let mut s = arch_room();
    let file = dir.join("mimari.dxf");
    s.export_dxf(&file).expect("written");
    let mut back = Session::new(&NewDrawing {
        name: "Geri".into(),
        srid: 0,
        plot_scale: 50.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
    })
    .expect("a new project");
    let answer = back
        .call(json!({ "command": "file.import.dxf", "input": { "path": file.display().to_string() } }))
        .expect("answers");
    assert_eq!(answer["status"], "completed", "{answer}");
    let uid = |d: &kentos_domain::Document, e: &kentos_commands::Entity| {
        d.uid(kentos_domain::Slot(e.base().id)).map(|u| u.to_string()).expect("an id")
    };
    let before: Vec<(String, Value)> = s
        .document()
        .entities()
        .filter(|e| arch_kind(e))
        .map(|e| (uid(s.document(), e), serde_json::to_value(e).expect("json")))
        .collect();
    let after: Vec<(String, Value)> = back
        .document()
        .entities()
        .filter(|e| arch_kind(e))
        .map(|e| (uid(back.document(), e), serde_json::to_value(e).expect("json")))
        .collect();
    let kinds: std::collections::BTreeSet<&str> = before.iter().filter_map(|(_, e)| e["kind"].as_str()).collect();
    assert_eq!(kinds.len(), 9, "every architectural kind: {kinds:?}");
    assert_eq!(before.len(), after.len(), "{answer}");
    // The ids in the writing drawing → in the reading one, by the objects' order.
    let ids: std::collections::HashMap<&str, &str> =
        before.iter().zip(&after).map(|((a, _), (b, _))| (a.as_str(), b.as_str())).collect();
    for ((_, a), (_, b)) in before.iter().zip(&after) {
        let mut want = a.clone();
        let map = want.as_object_mut().expect("an object");
        map.insert("id".into(), b["id"].clone());
        map.insert("layerId".into(), b["layerId"].clone());
        for key in ["host", "of"] {
            if let Some(Value::String(r)) = map.get_mut(key) {
                *r = ids.get(r.as_str()).copied().unwrap_or("?").to_owned();
            }
        }
        if let Some(Value::Array(list)) = map.get_mut("grids") {
            for r in list {
                if let Value::String(r) = r {
                    *r = ids.get(r.as_str()).copied().unwrap_or("?").to_owned();
                }
            }
        }
        assert_eq!(&want, b, "{}", a["kind"]);
    }
    // The pair the independent check reads (scripts/fixtures/arch_dxf_cases.py).
    if std::env::var_os("KENTOS_WRITE_DXF").is_some() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/formats/v1/dxf-arch");
        std::fs::create_dir_all(&root).expect("the folder");
        std::fs::copy(&file, root.join("mimari.dxf")).expect("kept");
        s.save(Some(&root.join("mimari.kcad"))).expect("saved");
    }
}

/// The drawing the Shapefile tests write: QGIS's GeoPackage taken in, a polygon on the point layer (it splits into two
/// Shapefiles), a circle on the road layer, two long Turkish attributes that cut to one name, a text (left out).
fn shapefile_drawing() -> Session {
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let layer = |name: &str| -> String {
        took["output"]["layers"]
            .as_array()
            .and_then(|ls| ls.iter().find(|l| l["name"] == name))
            .and_then(|l| l["id"].as_str())
            .unwrap_or_else(|| panic!("{name} katmanı yok: {took}"))
            .to_owned()
    };
    let (nokta, yol, parsel) = (layer("nokta"), layer("yol"), layer("parsel"));
    let poly = s
        .call(json!({ "command": "cad.polygon.create", "input": { "layerId": nokta,
            "pts": [{ "x": 487400.0, "y": 4420000.0 }, { "x": 487410.0, "y": 4420000.0 }, { "x": 487410.0, "y": 4420010.0 }],
            "attrs": { "ad": "üçgen", "Mülkiyet Durumu": "hisseli", "mülkiyet durumu tarihi": "2026-10-19" } } }))
        .expect("answers");
    assert_eq!(poly["status"], "completed", "{poly}");
    let circle = s
        .call(
            json!({ "command": "cad.circle.create", "input": { "layerId": yol,
            "c": { "x": 487500.0, "y": 4420000.0 }, "r": 25.0 } }),
        )
        .expect("answers");
    assert_eq!(circle["status"], "completed", "{circle}");
    let text = s
        .call(json!({ "command": "cad.entities.create", "input": { "layerId": parsel, "objects": [
            { "geometry": { "kind": "text", "p": { "x": 487005.0, "y": 4420005.0 }, "text": "101/5", "height": 2.0, "rotation": 0 } } ] } }))
        .expect("answers");
    assert_eq!(text["status"], "completed", "{text}");
    s
}

#[test]
fn shapefiles_go_out_whole_with_what_they_cannot_hold_said() {
    // docs/adr/0514: the drawing as a folder, an archive and one .shp; plan writes nothing; the losses said; the
    // files read back through KentOS's own reader; the refusals. KENTOS_WRITE_SHP=1 keeps the folder in
    // fixtures/shapefile/v1/out for scripts/fixtures/shp_export_cases.py, which reads it with OGR.
    let dir = scratch("shp");
    let mut s = shapefile_drawing();
    let folder = dir.join("teslim");
    let p = folder.display().to_string();
    let planned = s
        .call(json!({ "command": "file.export.shp", "op": "plan", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(planned["status"], "completed", "{planned}");
    assert!(!folder.exists(), "plan wrote nothing");
    let written = s
        .call(json!({ "command": "file.export.shp", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(written["status"], "completed", "{written}");
    assert_eq!(
        written["output"], planned["output"],
        "the plan says what is written"
    );
    let sets: Vec<(String, String, u64)> = written["output"]["sets"]
        .as_array()
        .expect("sets")
        .iter()
        .map(|t| {
            (
                t["name"].as_str().unwrap_or_default().to_owned(),
                t["shape"].as_str().unwrap_or("").to_owned(),
                t["rows"].as_u64().unwrap_or(0),
            )
        })
        .collect();
    let want = |name: &str, shape: &str, rows: u64| {
        assert!(
            sets.iter()
                .any(|(n, s, r)| n == name && s == shape && *r == rows),
            "{name} {shape} {rows}: {sets:?}"
        );
    };
    want("parsel", "POLYGON", 3);
    want("yol", "POLYLINEZ", 3);
    want("nokta_nokta", "POINTZ", 1);
    want("nokta_alan", "POLYGON", 1);
    want("egri", "POLYGON", 1);
    want("egri_yol", "POLYLINE", 1);
    want("malik", "", 2);
    assert_eq!(written["output"]["left"]["yazı"], 1, "{written}");
    assert_eq!(written["output"]["approximated"], 3, "{written}");
    assert!(written["output"]["prj"].as_bool().unwrap_or(false));
    let renamed = written["output"]["renamed"].to_string();
    assert!(renamed.contains("Mülkiyet"), "{renamed}");
    let warnings = written["warnings"].to_string();
    assert!(warnings.contains("export_losses"), "{warnings}");
    for f in [
        "parsel.shp",
        "parsel.shx",
        "parsel.dbf",
        "parsel.prj",
        "parsel.cpg",
        "malik.dbf",
    ] {
        assert!(folder.join(f).exists(), "{f}");
    }
    assert!(
        !folder.join("malik.shp").exists(),
        "a table alone is a .dbf"
    );
    // Read back by KentOS's reader: the parcel's values and its hole.
    let read = |stem: &str| {
        let b = |x: &str| std::fs::read(folder.join(format!("{stem}.{x}"))).ok();
        let (shp, shx, dbf, prj, cpg) = (b("shp"), b("shx"), b("dbf"), b("prj"), b("cpg"));
        kentos_formats::shp::read(
            &kentos_formats::shp::Files {
                shp: shp.as_deref().expect("shp"),
                shx: shx.as_deref(),
                dbf: dbf.as_deref(),
                prj: prj.as_deref(),
                cpg: cpg.as_deref(),
            },
            &Default::default(),
        )
        .expect("reads")
    };
    let parsel = read("parsel");
    assert_eq!(
        parsel.declared_crs.as_ref().and_then(|c| c.srid),
        Some(5256),
        "{:?}",
        parsel.declared_crs
    );
    let first = serde_json::to_value(&parsel.entities[0]).expect("JSON");
    assert_eq!(first["attrs"]["ada"], "Çağlayan", "{first}");
    assert_eq!(first["attrs"]["kimlik"], "9007199254740993", "{first}");
    assert_eq!(first["attrs"]["tescil"], "2024-02-29", "{first}");
    assert_eq!(first["attrs"]["tapulu"], "true", "{first}");
    let yol = read("yol");
    let road = serde_json::to_value(&yol.entities[0]).expect("JSON");
    assert_eq!(
        road["ms"],
        json!([0.0, 50.0, 101.0]),
        "the measures go: {road}"
    );
    // The archive: the same files in one.
    let zip = dir.join("teslim.zip");
    let z = s
        .call(json!({ "command": "file.export.shp", "input": { "path": zip.display().to_string(), "encoding": "windows-1254" } }))
        .expect("answers");
    assert_eq!(z["status"], "completed", "{z}");
    let listed = kentos_formats::zip::list(&std::fs::read(&zip).expect("zip"), &Default::default())
        .expect("a zip");
    assert!(listed.iter().any(|l| l.file_name() == "parsel.cpg"));
    // One .shp: one layer of one family; the export of several refused.
    let one = dir.join("yollar.shp");
    let yol_id = written["output"]["sets"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t["name"] == "yol"))
        .and_then(|t| t["layer"].as_str())
        .expect("yol")
        .to_owned();
    let single = s
        .call(json!({ "command": "file.export.shp", "input": { "path": one.display().to_string(), "layers": [yol_id] } }))
        .expect("answers");
    assert_eq!(single["status"], "completed", "{single}");
    assert!(dir.join("yollar.dbf").exists());
    let several = s
        .call(json!({ "command": "file.export.shp", "input": { "path": dir.join("hepsi.shp").display().to_string() } }))
        .expect("answers");
    assert_eq!(several["error"]["code"], "several_shapefiles", "{several}");
    let again = s
        .call(json!({ "command": "file.export.shp", "input": { "path": p } }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists", "{again}");
    let unknown = s
        .call(json!({ "command": "file.export.shp", "input": { "path": dir.join("yeni").display().to_string(), "layers": ["yok"] } }))
        .expect("answers");
    assert_eq!(unknown["error"]["code"], "layer_not_found", "{unknown}");
    if std::env::var_os("KENTOS_WRITE_SHP").is_some() {
        let kept = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/shapefile/v1/out");
        let _ = std::fs::remove_dir_all(&kept);
        std::fs::create_dir_all(&kept).expect("the folder");
        for e in std::fs::read_dir(&folder).expect("the folder") {
            let e = e.expect("a file");
            std::fs::copy(e.path(), kept.join(e.file_name())).expect("kept");
        }
        std::fs::copy(&zip, kept.join("teslim-1254.zip")).expect("kept");
    }
}

#[test]
fn layer_looks_go_out_as_sld_and_come_back() {
    // docs/adr/0516: a categorized, a graduated and a single look written as SLD 1.1; what SE cannot say is said;
    // each read back by file.import.sld into another layer gives the same classes. With KENTOS_WRITE_SLD=1 the
    // files go to fixtures/sld/v1/out for scripts/fixtures/sld_cases.py, which opens them in QGIS 4.2.
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let id = |name: &str| -> String {
        took["output"]["layers"]
            .as_array()
            .and_then(|ls| ls.iter().find(|l| l["name"] == name))
            .and_then(|l| l["id"].as_str())
            .unwrap_or_else(|| panic!("{name}"))
            .to_owned()
    };
    let (parsel, yol, egri) = (id("parsel"), id("yol"), id("egri"));
    let fill = |c: &str| {
        json!({ "fill": { "type": "fill", "layers": [
            { "id": "f", "type": "simpleFill", "color": c },
            { "id": "o", "type": "simpleLine", "color": "#222222", "width": 0.28, "unit": "mm" } ] } })
    };
    let looks = [
        (
            "kentos-kategorili.sld",
            parsel.clone(),
            json!({ "type": "categorized", "expr": "[ada]", "categories": [
                { "value": "A", "label": "Ada A", "symbols": fill("#e41a1c") },
                { "value": "B", "label": "Ada B", "symbols": fill("#377eb8") } ] }),
            "categorized",
        ),
        (
            "kentos-aralikli.sld",
            egri.clone(),
            json!({ "type": "graduated", "expr": "alan", "classes": [
                { "min": 0.0, "max": 500.0, "label": "küçük", "symbols": fill("#fee5d9") },
                { "min": 500.0, "max": 1000.0, "label": "orta", "symbols": fill("#fcae91") },
                { "min": 1000.0, "max": 5000.0, "label": "büyük", "symbols": fill("#de2d26") } ] }),
            "graduated",
        ),
        (
            "kentos-sade.sld",
            yol.clone(),
            json!({ "type": "single", "symbols": {
                "line": { "type": "line", "layers": [
                    { "id": "l", "type": "simpleLine", "color": "#3366cc", "width": 0.8, "unit": "mm",
                      "dash": [3.0, 1.5], "cap": "round", "join": "round" },
                    { "id": "w", "type": "simpleLine", "color": { "expr": "[renk]", "fallback": "#ffffff" }, "width": 0.2 } ] },
                "fill": { "type": "fill", "layers": [ { "id": "h", "type": "hatchFill", "angle": 45, "spacing": 2,
                    "width": 0.2, "color": "#000000" } ] } } }),
            "single",
        ),
    ];
    let dir = std::env::temp_dir().join(format!("kentos-sld-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    for (name, layer, renderer, kind) in &looks {
        let set = s
            .call(json!({ "command": "cad.layers.renderer", "input": { "layer": layer, "renderer": renderer } }))
            .expect("answers");
        assert_eq!(set["status"], "completed", "{set}");
        let path = dir.join(name);
        let planned = s
            .call(json!({ "command": "file.export.sld", "op": "plan",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(planned["status"], "completed", "{planned}");
        assert!(!path.exists(), "plan writes nothing");
        let out = s
            .call(json!({ "command": "file.export.sld",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{out}");
        assert_eq!(out["output"]["bytes"], planned["output"]["bytes"]);
        let text = std::fs::read_to_string(&path).expect("written");
        assert!(text.contains("<StyledLayerDescriptor"), "{text}");
        if *kind == "single" {
            // The data-defined colour said; the hatch is the area's, not written for a line layer.
            let lost = out["output"]["lost"].to_string();
            assert!(lost.contains("yedek değeri yazıldı"), "{out}");
            assert!(!text.contains("PolygonSymbolizer"), "{text}");
            assert_eq!(out["warnings"][0]["code"], "export_losses", "{out}");
        } else {
            assert!(out["output"].get("lost").is_none(), "{out}");
        }
        // Read back into the curve layer: the same kind and classes.
        let back = s
            .call(json!({ "command": "file.import.sld",
                "input": { "layer": egri, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(back["status"], "completed", "{back}");
        assert_eq!(back["output"]["kind"], *kind, "{back}");
        if *kind == "categorized" {
            assert_eq!(back["output"]["renderer"]["expr"], "[ada]");
            assert_eq!(back["output"]["renderer"]["categories"][1]["value"], "B");
        }
        if *kind == "graduated" {
            assert_eq!(back["output"]["renderer"]["classes"][2]["max"], 5000.0);
        }
        // Written over only when asked.
        let again = s
            .call(json!({ "command": "file.export.sld",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(again["error"]["code"], "file_exists", "{again}");
        if std::env::var_os("KENTOS_WRITE_SLD").is_some() {
            let out_dir = format!(
                "{}/../../../fixtures/sld/v1/out",
                env!("CARGO_MANIFEST_DIR")
            );
            std::fs::create_dir_all(&out_dir).expect("a folder");
            std::fs::copy(&path, format!("{out_dir}/{name}")).expect("kept");
        }
    }
    let missing = s
        .call(json!({ "command": "file.export.sld",
            "input": { "layer": "yok", "path": dir.join("x.sld").display().to_string() } }))
        .expect("answers");
    assert_eq!(missing["error"]["code"], "layer_not_found", "{missing}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn layer_looks_go_out_as_qml_and_come_back() {
    // docs/adr/0555: categorized, graduated, rule-based and single looks written as QGIS layer style files; what a QML
    // cannot say is said; each read back by file.import.qml gives the same renderer. With KENTOS_WRITE_QML=1 the
    // files go to fixtures/qml/v1/out for scripts/fixtures/qml_cases.py, which loads them in QGIS 4.2.
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let id = |name: &str| -> String {
        took["output"]["layers"]
            .as_array()
            .and_then(|ls| ls.iter().find(|l| l["name"] == name))
            .and_then(|l| l["id"].as_str())
            .unwrap_or_else(|| panic!("{name}"))
            .to_owned()
    };
    let (parsel, yol, nokta) = (id("parsel"), id("yol"), id("nokta"));
    let fill = |c: &str| {
        json!({ "fill": { "type": "fill", "layers": [
            { "id": "f", "type": "simpleFill", "color": c },
            { "id": "o", "type": "simpleLine", "color": "#222222", "width": 0.4, "unit": "mm", "cap": "butt", "join": "miter" } ] } })
    };
    let looks = [
        (
            "kentos-kategorili.qml",
            parsel.clone(),
            json!({ "type": "categorized", "expr": "[ada]", "categories": [
                { "value": "A", "label": "Ada A", "symbols": fill("#e41a1c") },
                { "value": "B", "label": "Ada B", "symbols": fill("#377eb8") } ],
                "other": fill("#99999980") }),
            "categorized",
        ),
        (
            "kentos-aralikli.qml",
            parsel.clone(),
            json!({ "type": "graduated", "expr": "[alan]", "classes": [
                { "min": 0.0, "max": 500.0, "label": "küçük", "symbols": fill("#fee5d9") },
                { "min": 500.0, "max": 1000.0, "label": "orta", "symbols": fill("#fcae91") },
                { "min": 1000.0, "max": 5000.0, "label": "büyük", "symbols": fill("#de2d26") } ] }),
            "graduated",
        ),
        (
            "kentos-kurallar.qml",
            parsel.clone(),
            json!({ "type": "rules", "rules": [
                { "id": "r1", "label": "Tapulu", "filter": "[tapulu] = 1", "symbols": fill("#1b9e77"),
                  "children": [ { "id": "r2", "label": "Büyük", "filter": "[alan] > 1000 ve [ada] <> 'B'",
                    "minScale": 500, "maxScale": 10000, "symbols": fill("#d95f02") } ] },
                { "id": "r3", "label": "Diğer", "isElse": true, "symbols": fill("#7570b3") } ] }),
            "rules",
        ),
        (
            "kentos-cizgi.qml",
            yol.clone(),
            json!({ "type": "single", "symbols": { "line": { "type": "line", "opacity": 0.8, "layers": [
                { "id": "l", "type": "simpleLine", "color": "#3366cc", "width": 0.8, "unit": "mm",
                  "dash": [3.0, 1.5], "cap": "round", "join": "round" },
                { "id": "m", "type": "simpleLine", "color": "#ffffff", "width": 2.0, "unit": "m" } ] } } }),
            "single",
        ),
        (
            "kentos-nokta.qml",
            nokta.clone(),
            json!({ "type": "single", "symbols": { "marker": { "type": "marker", "layers": [
                { "id": "p", "type": "shape", "shape": "triangle", "size": 3.0, "unit": "mm", "fill": "#ff0000",
                  "stroke": "#000000", "strokeWidth": 0.3, "rotation": -30.0 },
                { "id": "g", "type": "shape", "shape": "gear", "size": 2.0, "unit": "mm", "fill": "#00ff00" } ] } } }),
            "single",
        ),
        (
            "kentos-tarama.qml",
            parsel.clone(),
            json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
                { "id": "f", "type": "simpleFill", "color": "#ffffff" },
                { "id": "h", "type": "hatchFill", "angle": 30.0, "spacing": 2.5, "width": 0.2, "color": "#336699",
                  "unit": "mm", "offset": 0.5 },
                { "id": "o", "type": "simpleLine", "color": "#aa0000", "unit": "mm",
                  "width": { "expr": "[alan] / 1000", "fallback": 0.5 } } ] } } }),
            "single",
        ),
        (
            "kentos-isaretli.qml",
            yol.clone(),
            json!({ "type": "single", "symbols": { "line": { "type": "line", "layers": [
                { "id": "l", "type": "simpleLine", "color": "#3366cc", "width": 0.6, "unit": "mm" },
                { "id": "m", "type": "markerLine", "placement": "interval", "interval": 4.0, "offsetAlong": 1.0,
                  "unit": "mm", "marker": { "type": "marker", "layers": [
                    { "id": "p", "type": "shape", "shape": "triangle", "unit": "mm", "fill": "#ff0000",
                      "size": { "expr": "[kat] * 2 + 1", "fallback": 2.0 },
                      "rotation": { "expr": "[yon]", "fallback": 0.0 } } ] } } ] } } }),
            "single",
        ),
    ];
    let dir = std::env::temp_dir().join(format!("kentos-qml-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    for (name, layer, renderer, kind) in &looks {
        let set = s
            .call(json!({ "command": "cad.layers.renderer", "input": { "layer": layer, "renderer": renderer } }))
            .expect("answers");
        assert_eq!(set["status"], "completed", "{set}");
        let path = dir.join(name);
        let planned = s
            .call(json!({ "command": "file.export.qml", "op": "plan",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(planned["status"], "completed", "{planned}");
        assert!(!path.exists(), "plan writes nothing");
        let out = s
            .call(json!({ "command": "file.export.qml",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{out}");
        assert_eq!(out["output"]["bytes"], planned["output"]["bytes"]);
        let text = std::fs::read_to_string(&path).expect("written");
        assert!(text.contains("<renderer-v2"), "{text}");
        let lost = out["output"]["lost"].to_string();
        match *name {
            "kentos-nokta.qml" => assert!(lost.contains("gear"), "{out}"),
            "kentos-aralikli.qml" => assert!(lost.contains("alt sınırı"), "{out}"),
            _ => assert!(out["output"].get("lost").is_none(), "{out}"),
        }
        // Read back into the same layer: the same renderer, but for the graduated bounds and the gear.
        let back = s
            .call(json!({ "command": "file.import.qml",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(back["status"], "completed", "{back}");
        assert_eq!(back["output"]["kind"], *kind, "{back}");
        let r = &back["output"]["renderer"];
        match *name {
            "kentos-kategorili.qml" => {
                assert_eq!(r["categories"][1]["value"], "B");
                assert_eq!(r["other"]["fill"]["layers"][0]["color"], "#99999980");
                assert_eq!(r["categories"][0]["symbols"]["fill"]["layers"][1]["width"], 0.4);
            }
            "kentos-kurallar.qml" => {
                assert_eq!(r["rules"][0]["filter"], "[tapulu] = 1");
                assert_eq!(
                    r["rules"][0]["children"][0]["filter"],
                    "[alan] > 1000 ve [ada] <> 'B'"
                );
                assert_eq!(r["rules"][0]["children"][0]["maxScale"], 10000.0);
                assert_eq!(r["rules"][1]["isElse"], true);
            }
            "kentos-cizgi.qml" => {
                let line = &r["symbols"]["line"];
                assert_eq!(line["opacity"], 0.8);
                assert_eq!(line["layers"][0]["dash"], json!([3.0, 1.5]));
                assert_eq!(line["layers"][1]["unit"], "m");
            }
            "kentos-nokta.qml" => {
                let m = &r["symbols"]["marker"]["layers"][0];
                assert_eq!(m["rotation"], -30.0);
                assert_eq!(m["strokeWidth"], 0.3);
            }
            "kentos-tarama.qml" => {
                let l = &r["symbols"]["fill"]["layers"];
                assert_eq!(l[1]["type"], "hatchFill", "{r}");
                assert_eq!(l[1]["angle"], 30.0);
                assert_eq!(l[1]["spacing"], 2.5);
                assert_eq!(l[1]["offset"], 0.5);
                assert_eq!(l[2]["width"], json!({ "expr": "[alan] / 1000", "fallback": 0.5 }));
            }
            "kentos-isaretli.qml" => {
                let m = &r["symbols"]["line"]["layers"][1];
                assert_eq!(m["type"], "markerLine", "{r}");
                assert_eq!(m["interval"], 4.0);
                assert_eq!(m["offsetAlong"], 1.0);
                let p = &m["marker"]["layers"][0];
                assert_eq!(p["size"], json!({ "expr": "[kat] * 2 + 1", "fallback": 2.0 }));
                // Turned twice, the angle comes back as written; a zero angle is QGIS's default and not written.
                assert_eq!(p["rotation"], json!({ "expr": "[yon]" }));
            }
            _ => {}
        }
        // Written over only when asked.
        let again = s
            .call(json!({ "command": "file.export.qml",
                "input": { "layer": layer, "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(again["error"]["code"], "file_exists", "{again}");
        if std::env::var_os("KENTOS_WRITE_QML").is_some() {
            let out_dir = format!(
                "{}/../../../fixtures/qml/v1/out",
                env!("CARGO_MANIFEST_DIR")
            );
            std::fs::create_dir_all(&out_dir).expect("a folder");
            std::fs::copy(&path, format!("{out_dir}/{name}")).expect("kept");
        }
    }
    let missing = s
        .call(json!({ "command": "file.export.qml",
            "input": { "layer": "yok", "path": dir.join("x.qml").display().to_string() } }))
        .expect("answers");
    assert_eq!(missing["error"]["code"], "layer_not_found", "{missing}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn csv_tables_go_out_and_come_back() {
    // docs/adr/0522: the GeoPackage's layers written as CSVs (WKT, its .csvt and .prj) and read back into a new
    // drawing: the same objects, arcs exact; X/Y writes the points and counts the rest. With KENTOS_WRITE_CSV=1 the
    // files go to fixtures/csv/v1/out for scripts/fixtures/csv_exchange_cases.py, which reads them with OGR.
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let id = |name: &str| -> String {
        took["output"]["layers"]
            .as_array()
            .and_then(|ls| ls.iter().find(|l| l["name"] == name))
            .and_then(|l| l["id"].as_str())
            .unwrap_or_else(|| panic!("{name}"))
            .to_owned()
    };
    let dir = std::env::temp_dir().join(format!("kentos-csv-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    for name in ["parsel", "yol", "egri"] {
        let path = dir.join(format!("kentos-{name}.csv"));
        let planned = s
            .call(json!({ "command": "file.export.csv", "op": "plan",
                "input": { "path": path.display().to_string(), "layers": [id(name)] } }))
            .expect("answers");
        assert_eq!(planned["status"], "completed", "{planned}");
        assert!(!path.exists(), "plan writes nothing");
        let out = s
            .call(json!({ "command": "file.export.csv",
                "input": { "path": path.display().to_string(), "layers": [id(name)] } }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{out}");
        assert_eq!(out["output"]["columns"][0], "WKT", "{out}");
        assert!(path.with_extension("csvt").exists() && path.with_extension("prj").exists());
        // Back into a new drawing: the same number of objects, onto a new layer.
        let mut back = new_drawing();
        let got = back
            .call(json!({ "command": "file.import.csv", "input": { "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(got["status"], "completed", "{got}");
        assert_eq!(
            got["output"]["objects"],
            took["output"]["tables"]
                .as_array()
                .and_then(|ts| ts.iter().find(|t| t["table"] == name))
                .map(|t| t["objects"].clone())
                .unwrap_or_default(),
            "{name}: {got}"
        );
        if std::env::var_os("KENTOS_WRITE_CSV").is_some() {
            let out_dir = format!("{}/../../../fixtures/csv/v1/out", env!("CARGO_MANIFEST_DIR"));
            std::fs::create_dir_all(&out_dir).expect("a folder");
            for ext in ["csv", "csvt", "prj"] {
                std::fs::copy(
                    path.with_extension(ext),
                    format!("{out_dir}/kentos-{name}.{ext}"),
                )
                .expect("kept");
            }
        }
    }
    // X/Y: the points only, the rest counted.
    let path = dir.join("noktalar.csv");
    let out = s
        .call(json!({ "command": "file.export.csv",
            "input": { "path": path.display().to_string(), "geometry": "xy", "separator": "semicolon" } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    assert_eq!(out["output"]["columns"][0], "X");
    assert!(out["output"]["left"]["alan"].as_u64().unwrap_or(0) > 0, "{out}");
    assert_eq!(out["warnings"][0]["code"], "export_losses", "{out}");
    let again = s
        .call(json!({ "command": "file.export.csv",
            "input": { "path": path.display().to_string(), "geometry": "xy" } }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists", "{again}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn flatgeobufs_go_out_and_come_back() {
    // docs/adr/0544: the GeoPackage's layers written as FlatGeobufs (with the spatial index when every object has a
    // position) and read back into a new drawing: the same objects, arcs exact; the whole drawing in one file with
    // its `katman` column. With KENTOS_WRITE_FGB=1 the files go to fixtures/fgb/v1/out for
    // scripts/fixtures/fgb_exchange_cases.py, which reads them with OGR.
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let id = |name: &str| -> String {
        took["output"]["layers"]
            .as_array()
            .and_then(|ls| ls.iter().find(|l| l["name"] == name))
            .and_then(|l| l["id"].as_str())
            .unwrap_or_else(|| panic!("{name}"))
            .to_owned()
    };
    let dir = std::env::temp_dir().join(format!("kentos-fgb-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let mut written = Vec::new();
    for (name, layers) in [
        ("parsel", vec![id("parsel")]),
        ("yol", vec![id("yol")]),
        ("nokta", vec![id("nokta")]),
        ("egri", vec![id("egri")]),
        ("egri_yol", vec![id("egri_yol")]),
        ("hepsi", Vec::new()),
    ] {
        let path = dir.join(format!("kentos-{name}.fgb"));
        let planned = s
            .call(json!({ "command": "file.export.fgb", "op": "plan",
                "input": { "path": path.display().to_string(), "layers": layers } }))
            .expect("answers");
        assert_eq!(planned["status"], "completed", "{planned}");
        assert!(!path.exists(), "plan writes nothing");
        let out = s
            .call(json!({ "command": "file.export.fgb",
                "input": { "path": path.display().to_string(), "layers": layers } }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{out}");
        assert_eq!(out["output"], planned["output"], "plan says what execute writes");
        let mut back = new_drawing();
        let got = back
            .call(json!({ "command": "file.import.fgb", "input": { "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(got["status"], "completed", "{got}");
        assert_eq!(got["output"]["objects"], out["output"]["features"], "{name}: {got}");
        written.push((name, path));
    }
    if std::env::var_os("KENTOS_WRITE_FGB").is_some() {
        let out_dir = format!("{}/../../../fixtures/fgb/v1/out", env!("CARGO_MANIFEST_DIR"));
        std::fs::create_dir_all(&out_dir).expect("a folder");
        for (name, path) in &written {
            std::fs::copy(path, format!("{out_dir}/kentos-{name}.fgb")).expect("kept");
        }
    }
    let again = s
        .call(json!({ "command": "file.export.fgb",
            "input": { "path": written[0].1.display().to_string(), "layers": [id("parsel")] } }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists", "{again}");
    let broken = dir.join("bozuk.fgb");
    std::fs::write(&broken, b"fgb\x03fgb\x00\xff\xff\xff\xff").expect("written");
    let refused = s
        .call(json!({ "command": "file.import.fgb", "input": { "path": broken.display().to_string() } }))
        .expect("answers");
    assert_eq!(refused["error"]["code"], "import_unreadable", "{refused}");
    let _ = std::fs::remove_dir_all(&dir);
}

fn gml_fixture(name: &str) -> String {
    format!("{}/../../../fixtures/gml/v1/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn wgs_drawing() -> Session {
    Session::new(&NewDrawing {
        name: "WGS".into(),
        srid: 4326,
        plot_scale: 1000.0,
        workspace: Workspace::Gis,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
    })
    .expect("a project")
}

#[test]
fn gmls_go_out_and_come_back() {
    // docs/adr/0545: the GeoPackage's layers written as GML 3.2 with their schema and read back into a new drawing:
    // the same objects, arcs exact; the whole drawing in one file with a feature type for each layer; WGS 84 with
    // the axes latitude first. With KENTOS_WRITE_GML=1 the files go to fixtures/gml/v1/out for
    // scripts/fixtures/gml_exchange_cases.py, which reads them with OGR; and the fixtures' GML (GDAL's) read and
    // written again go to fixtures/gml/v1/geri.
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let id = |name: &str| -> String {
        took["output"]["layers"]
            .as_array()
            .and_then(|ls| ls.iter().find(|l| l["name"] == name))
            .and_then(|l| l["id"].as_str())
            .unwrap_or_else(|| panic!("{name}"))
            .to_owned()
    };
    let dir = std::env::temp_dir().join(format!("kentos-gml-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let mut written: Vec<(String, std::path::PathBuf)> = Vec::new();
    for (name, layers) in [
        ("parsel", vec![id("parsel")]),
        ("yol", vec![id("yol")]),
        ("nokta", vec![id("nokta")]),
        ("egri", vec![id("egri")]),
        ("egri_yol", vec![id("egri_yol")]),
        ("hepsi", Vec::new()),
    ] {
        let path = dir.join(format!("kentos-{name}.gml"));
        let planned = s
            .call(json!({ "command": "file.export.gml", "op": "plan",
                "input": { "path": path.display().to_string(), "layers": layers } }))
            .expect("answers");
        assert_eq!(planned["status"], "completed", "{planned}");
        assert!(!path.exists(), "plan writes nothing");
        let out = s
            .call(json!({ "command": "file.export.gml",
                "input": { "path": path.display().to_string(), "layers": layers } }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{out}");
        assert_eq!(out["output"], planned["output"], "plan says what execute writes");
        assert!(path.with_extension("xsd").exists(), "the schema is beside the file");
        let mut back = new_drawing();
        let got = back
            .call(json!({ "command": "file.import.gml", "input": { "path": path.display().to_string() } }))
            .expect("answers");
        assert_eq!(got["status"], "completed", "{got}");
        assert_eq!(got["output"]["objects"], out["output"]["features"], "{name}: {got}");
        if name == "hepsi" {
            assert_eq!(
                got["output"]["tables"].as_array().map(Vec::len),
                Some(6),
                "a type for each layer: {got}"
            );
        }
        written.push((name.to_owned(), path));
    }
    // WGS 84: the axes go latitude first and come back longitude first.
    let mut wgs = wgs_drawing();
    let yer = format!(
        "{}/../../../fixtures/geopackage/v1/wgs84.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let took = wgs
        .call(json!({ "command": "file.import.gpkg", "input": { "path": yer } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let path = dir.join("kentos-yer.gml");
    let out = wgs
        .call(json!({ "command": "file.export.gml", "input": { "path": path.display().to_string() } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    let text = std::fs::read_to_string(&path).expect("written");
    assert!(text.contains("urn:ogc:def:crs:EPSG::4326"), "{text}");
    assert!(text.contains("<gml:pos>39.9208 32.8541</gml:pos>"), "latitude first: {text}");
    let back = wgs_drawing()
        .call(json!({ "command": "file.import.gml", "input": { "path": path.display().to_string() } }))
        .expect("answers");
    assert_eq!(back["status"], "completed", "{back}");
    written.push(("yer".to_owned(), path));
    // The fixtures read and written again.
    let mut redone: Vec<(String, std::path::PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(gml_fixture("")).expect("the fixtures") {
        let p = entry.expect("an entry").path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_owned();
        let skip = ["bozuk.gml", "xml-degil.gml", "kesik.gml", "crs84.gml", "bos.gml"];
        if p.extension().is_none_or(|e| e != "gml") || skip.contains(&name.as_str()) {
            continue;
        }
        let mut drawing = if name == "yer.gml" { wgs_drawing() } else { new_drawing() };
        let got = drawing
            .call(json!({ "command": "file.import.gml", "input": { "path": p.display().to_string() } }))
            .expect("answers");
        assert_eq!(got["status"], "completed", "{name}: {got}");
        let to = dir.join(format!("geri-{name}"));
        let out = drawing
            .call(json!({ "command": "file.export.gml", "input": { "path": to.display().to_string() } }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{name}: {out}");
        redone.push((name, to));
    }
    if std::env::var_os("KENTOS_WRITE_GML").is_some() {
        let base = gml_fixture("");
        for (sub, files) in [("out", &written), ("geri", &redone)] {
            let to = format!("{base}{sub}");
            std::fs::create_dir_all(&to).expect("a folder");
            for (name, path) in files {
                let stem = if sub == "out" {
                    format!("kentos-{name}")
                } else {
                    name.trim_end_matches(".gml").to_owned()
                };
                std::fs::copy(path, format!("{to}/{stem}.gml")).expect("kept");
                std::fs::copy(path.with_extension("xsd"), format!("{to}/{stem}.xsd")).expect("kept");
            }
        }
    }
    let again = s
        .call(json!({ "command": "file.export.gml",
            "input": { "path": written[0].1.display().to_string(), "layers": [id("parsel")] } }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists", "{again}");
    let noschema = dir.join("sema-yok.gml");
    let out = s
        .call(json!({ "command": "file.export.gml",
            "input": { "path": noschema.display().to_string(), "layers": [id("nokta")], "schema": false } }))
        .expect("answers");
    assert_eq!(out["status"], "completed", "{out}");
    assert!(!noschema.with_extension("xsd").exists(), "no schema was asked for");
    assert!(!std::fs::read_to_string(&noschema).expect("written").contains("schemaLocation"));
    let as_schema = dir.join("ad.xsd");
    let refused = s
        .call(json!({ "command": "file.export.gml", "input": { "path": as_schema.display().to_string() } }))
        .expect("answers");
    assert_eq!(refused["error"]["code"], "invalid_file_path", "{refused}");
    let missing = s
        .call(json!({ "command": "file.export.gml",
            "input": { "path": dir.join("yok.gml").display().to_string(), "layers": ["yok"] } }))
        .expect("answers");
    assert_eq!(missing["error"]["code"], "layer_not_found", "{missing}");
    let broken = dir.join("bozuk.gml");
    std::fs::write(&broken, b"<a><b></a>").expect("written");
    let refused = s
        .call(json!({ "command": "file.import.gml", "input": { "path": broken.display().to_string() } }))
        .expect("answers");
    assert_eq!(refused["error"]["code"], "import_unreadable", "{refused}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn xyz_files_go_out() {
    // docs/adr/0545: the GeoPackage's points and vertices as X Y Z lines. With KENTOS_WRITE_XYZ=1 the files go to
    // fixtures/xyz/v1/out for scripts/fixtures/gml_exchange_cases.py, which makes the lines from OGR's geometries.
    let src = format!(
        "{}/../../../fixtures/geopackage/v1/qgis.gpkg",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut s = new_drawing();
    let took = s
        .call(json!({ "command": "file.import.gpkg", "input": { "path": src } }))
        .expect("answers");
    assert_eq!(took["status"], "completed", "{took}");
    let id = |name: &str| -> String {
        took["output"]["layers"]
            .as_array()
            .and_then(|ls| ls.iter().find(|l| l["name"] == name))
            .and_then(|l| l["id"].as_str())
            .unwrap_or_else(|| panic!("{name}"))
            .to_owned()
    };
    let dir = std::env::temp_dir().join(format!("kentos-xyz-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let cases = [
        ("nokta", json!({ "layers": [id("nokta")] })),
        (
            "yol",
            json!({ "layers": [id("yol")], "vertices": "all", "separator": "comma", "decimals": 3, "header": true }),
        ),
        ("parsel", json!({ "layers": [id("parsel")], "vertices": "all" })),
        ("egri", json!({ "layers": [id("egri")], "vertices": "all", "separator": "semicolon" })),
        ("egri_yol", json!({ "layers": [id("egri_yol")], "vertices": "all", "separator": "tab" })),
        ("hepsi", json!({ "vertices": "all" })),
        ("hepsi-kotlu", json!({ "vertices": "all", "withoutZ": "skip" })),
    ];
    let mut written = Vec::new();
    for (name, extra) in cases {
        let path = dir.join(format!("xyz-{name}.xyz"));
        let mut input = extra.clone();
        input["path"] = json!(path.display().to_string());
        let planned = s
            .call(json!({ "command": "file.export.xyz", "op": "plan", "input": input }))
            .expect("answers");
        assert_eq!(planned["status"], "completed", "{planned}");
        assert!(!path.exists(), "plan writes nothing");
        let out = s
            .call(json!({ "command": "file.export.xyz", "input": input }))
            .expect("answers");
        assert_eq!(out["status"], "completed", "{out}");
        assert_eq!(out["output"], planned["output"], "plan says what execute writes");
        let text = std::fs::read_to_string(&path).expect("written");
        let lines = text.lines().count() as u64 - u64::from(input["header"] == true);
        assert_eq!(out["output"]["points"], lines, "{name}");
        assert_eq!(out["output"]["srid"], 5256);
        written.push((name, path));
    }
    // A layer of areas has no points: said, and what to ask for.
    let none = s
        .call(json!({ "command": "file.export.xyz",
            "input": { "path": dir.join("yok.xyz").display().to_string(), "layers": [id("parsel")] } }))
        .expect("answers");
    assert_eq!(none["error"]["code"], "nothing_to_export", "{none}");
    let too_many = s
        .call(json!({ "command": "file.export.xyz",
            "input": { "path": dir.join("cok.xyz").display().to_string(), "decimals": 16 } }))
        .expect("answers");
    assert_eq!(too_many["error"]["code"], "invalid_input", "{too_many}");
    let again = s
        .call(json!({ "command": "file.export.xyz",
            "input": { "path": written[0].1.display().to_string(), "layers": [id("nokta")] } }))
        .expect("answers");
    assert_eq!(again["error"]["code"], "file_exists", "{again}");
    if std::env::var_os("KENTOS_WRITE_XYZ").is_some() {
        let to = format!("{}/../../../fixtures/xyz/v1/out", env!("CARGO_MANIFEST_DIR"));
        std::fs::create_dir_all(&to).expect("a folder");
        for (name, path) in &written {
            std::fs::copy(path, format!("{to}/kentos-{name}.xyz")).expect("kept");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

fn gis_fixture(name: &str) -> String {
    format!("{}/../../../fixtures/gis-import/v1/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// A Shapefile, a GeoJSON file and a KML go into a drawing as the product commands (docs/adr/0563): each is one
/// recorded step under its command's name, the plan says what execute takes, the file's system is a question the
/// input answers, and the journal replays on another drawing.
#[test]
fn gis_files_are_taken_in_by_their_commands() {
    let mut s = new_drawing();
    for (command, name, srid, objects) in [
        ("file.import.shp", "nokta.shp", None, 1),
        ("file.import.shp", "katmanlar.zip", None, 5),
        ("file.import.geojson", "katmanli.geojson", None, 3),
        ("file.import.geojson", "parsel-rfc7946.geojson", Some(4326), 2),
        ("file.import.kml", "nokta.kml", Some(4326), 1),
    ] {
        let mut input = json!({ "path": gis_fixture(name) });
        if srid.is_some() {
            input["reproject"] = json!(true);
        }
        let planned = s
            .call(json!({ "command": command, "op": "plan", "input": input }))
            .expect("answers");
        assert_eq!(planned["status"], "completed", "{name}: {planned}");
        let took = s
            .call(json!({ "command": command, "input": input }))
            .expect("answers");
        assert_eq!(took["status"], "completed", "{name}: {took}");
        assert_eq!(took["output"]["objects"], objects, "{name}: {took}");
        assert_eq!(took["output"]["objects"], planned["output"]["objects"], "{name}");
        assert_eq!(took["output"]["moved"].is_object(), srid.is_some(), "{name}: {took}");
    }
    // The system is a question: WGS 84 coordinates are not moved unless asked.
    let refused = s
        .call(json!({ "command": "file.import.geojson",
            "input": { "path": gis_fixture("nokta-rfc7946.geojson") } }))
        .expect("answers");
    assert_eq!(refused["error"]["code"], "crs_mismatch", "{refused}");
    assert_eq!(refused["error"]["path"], "reproject");
    // Each import is a row of the journal named by the command, and the journal replays on another drawing.
    let journal = s.journal();
    let rows: Vec<&str> = journal["calls"]
        .as_array()
        .expect("rows")
        .iter()
        .filter_map(|r| r["command"].as_str())
        .filter(|c| c.starts_with("file.import."))
        .collect();
    assert_eq!(
        rows,
        [
            "file.import.shp",
            "file.import.shp",
            "file.import.geojson",
            "file.import.geojson",
            "file.import.kml"
        ]
    );
    let mut other = new_drawing();
    let report = other.replay(journal).expect("replays");
    assert!(
        report["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .filter(|r| r["command"].as_str().is_some_and(|c| c.starts_with("file.import.")))
            .all(|r| r["outcome"] == "same"),
        "{report}"
    );
}
