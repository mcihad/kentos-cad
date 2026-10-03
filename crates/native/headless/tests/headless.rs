//! The headless host end to end (docs/adr/0130; TODOS.md §14 acceptance):
//! a new project, a closed area, its measures, a layer change, a save and a
//! reading back; a group taken back whole; the commands it runs, refuses
//! and lists.

use std::path::PathBuf;

use kentos_contracts::{DrawingFont, Workspace};
use kentos_headless::{DESKTOP, NewDrawing, Op, Session, catalog};
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
    let mut here: Vec<(&str, u64)> = DESKTOP.iter().map(|(n, v)| (*n, u64::from(*v))).collect();
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
