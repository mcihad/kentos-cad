//! The agents' tools on a headless drawing (docs/adr/8056 §2): the catalog found step by step, a command through its
//! part's door, the owner's guard (a write is planned first and executed only as planned, on an unchanged drawing),
//! steps as one undo step, a read at once, and a host that does not offer a part.

use kentos_commands::{CadWork, DrawingFont, DrawingUnit, Workspace};
use kentos_headless::agent_tools::{self, Guard, Host};
use kentos_headless::{HeadlessError, NewDrawing, Session};
use kentos_native_application::registry::{Discipline, discipline_of};
use serde_json::{Value, json};

fn drawing() -> Session {
    Session::new(&NewDrawing {
        name: "Ajan".into(),
        srid: 0,
        plot_scale: 50.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: Some(DrawingUnit::M),
        annotation: None,
        cad_work: Some(CadWork::Architecture),
        heights: None,
    })
    .expect("a drawing")
}

/// A 4 × 3 area at `x`, on the active layer.
fn area(s: &Session, x: f64) -> Value {
    json!({
        "layerId": s.summary()["activeLayer"],
        "pts": [{ "x": x, "y": 0.0 }, { "x": x + 4.0, "y": 0.0 }, { "x": x + 4.0, "y": 3.0 }, { "x": x, "y": 3.0 }],
    })
}

fn objects(s: &Session) -> u64 {
    s.summary()["objects"].as_u64().expect("a count")
}

fn tool(
    s: &mut Session,
    guard: &mut Guard,
    name: &str,
    args: Value,
) -> Result<Value, HeadlessError> {
    agent_tools::call(s, guard, name, &args)
}

fn code(answer: Result<Value, HeadlessError>) -> String {
    match answer {
        Ok(v) => format!("ok {}", v["status"]),
        Err(e) => e.code.to_owned(),
    }
}

#[test]
fn the_catalog_is_found_step_by_step() {
    let mut s = drawing();
    let mut guard = Guard::new();
    let parts = tool(&mut s, &mut guard, "catalog.categories", json!({})).expect("categories");
    let doors: Vec<&str> = parts["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .filter_map(|p| p["door"].as_str())
        .collect();
    assert_eq!(
        doors,
        ["cad.run", "arch.run", "map.run", "gis.run", "road.run"]
    );
    let found = tool(
        &mut s,
        &mut guard,
        "catalog.search",
        json!({ "query": "duvar" }),
    )
    .expect("search");
    let walls = found["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .find(|c| c["command"] == "cad.walls.create")
        .expect("Duvar is found by its Turkish word");
    assert_eq!(
        (walls["door"].clone(), walls["writes"].clone()),
        (json!("arch.run"), json!(true))
    );
    let only_gis = tool(
        &mut s,
        &mut guard,
        "catalog.search",
        json!({ "query": "katman", "part": "gis" }),
    )
    .expect("gis");
    assert!(
        only_gis["commands"]
            .as_array()
            .expect("commands")
            .iter()
            .all(|c| c["door"] == "gis.run"),
        "a part narrows the search"
    );
    // A Turkish word finds a command whose id is English: Kapalı alan oluştur by its “alan”.
    let area = tool(
        &mut s,
        &mut guard,
        "catalog.search",
        json!({ "query": "çokgen alan" }),
    )
    .expect("search");
    assert!(
        area["commands"]
            .as_array()
            .expect("commands")
            .iter()
            .any(|c| c["command"] == "cad.polygon.create")
    );
    let described = tool(
        &mut s,
        &mut guard,
        "catalog.describe",
        json!({ "command": "cad.polygon.create" }),
    )
    .expect("describe");
    assert_eq!(described["door"], "cad.run");
    assert!(
        described["input"]["properties"]["pts"].is_object(),
        "the whole input schema"
    );
}

#[test]
fn a_write_is_planned_first_and_executed_only_as_planned() {
    let mut s = drawing();
    let mut guard = Guard::new();
    let before = objects(&s);
    let input = area(&s, 0.0);
    let unplanned = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.polygon.create", "input": input, "mode": "execute" }),
    );
    assert_eq!(code(unplanned), "plan_required");
    let planned = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.polygon.create", "input": input }),
    )
    .expect("a plan");
    let plan = planned["planId"]
        .as_str()
        .expect("the plan's id")
        .to_owned();
    assert_eq!(
        planned["summary"]["counts"]["added"], 1,
        "the plan says what it adds"
    );
    assert_eq!(objects(&s), before, "a plan writes nothing");
    // Another input is not the plan's.
    let elsewhere = area(&s, 10.0);
    let other = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.polygon.create", "input": elsewhere, "mode": "execute", "planId": plan }),
    );
    assert_eq!(code(other), "plan_required");
    let written = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.polygon.create", "input": input, "mode": "execute", "planId": plan }),
    );
    assert_eq!(code(written), "ok \"completed\"");
    assert_eq!(objects(&s), before + 1);
    // A plan is taken once.
    let again = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.polygon.create", "input": input, "mode": "execute", "planId": plan }),
    );
    assert_eq!(code(again), "plan_required");
    assert!(s.undo().is_some(), "the write is one undo step");
    assert_eq!(objects(&s), before);
}

#[test]
fn a_plan_does_not_write_on_a_drawing_changed_since() {
    let mut s = drawing();
    let mut guard = Guard::new();
    let first = area(&s, 0.0);
    let plan = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.polygon.create", "input": first }),
    )
    .expect("a plan")["planId"]
        .as_str()
        .expect("an id")
        .to_owned();
    // The drawing changes: another area is planned and written.
    let second = area(&s, 10.0);
    let other = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.polygon.create", "input": second }),
    )
    .expect("a plan")["planId"]
        .as_str()
        .expect("an id")
        .to_owned();
    tool(&mut s, &mut guard, "cad.run", json!({ "command": "cad.polygon.create", "input": second, "mode": "execute", "planId": other }))
        .expect("written");
    let after = objects(&s);
    let stale = tool(&mut s, &mut guard, "cad.run", json!({ "command": "cad.polygon.create", "input": first, "mode": "execute", "planId": plan }))
        .expect("an answer");
    assert_eq!(stale["status"], "conflict", "{stale}");
    assert_eq!(stale["error"]["code"], "revision_conflict");
    assert_eq!(objects(&s), after, "nothing written");
}

#[test]
fn a_command_goes_through_its_parts_door_and_a_read_runs_at_once() {
    let mut s = drawing();
    let mut guard = Guard::new();
    let wrong = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.walls.create", "input": {} }),
    );
    let Err(e) = wrong else {
        panic!("a wall is architecture's")
    };
    assert_eq!(e.code, "wrong_tool");
    assert!(e.message.contains("arch.run"), "{}", e.message);
    let wrong = tool(
        &mut s,
        &mut guard,
        "arch.run",
        json!({ "command": "cad.polygon.create", "input": {} }),
    );
    assert_eq!(code(wrong), "wrong_tool");
    // A core command that only reads and asks for nothing runs without a plan.
    let catalog = kentos_headless::catalog();
    let read = catalog["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .find(|c| {
            c["effect"] == "query"
                && c["input"]["required"].as_array().is_none_or(Vec::is_empty)
                && discipline_of(c["id"].as_str().unwrap_or_default()) == Some(Discipline::Core)
                && !c["id"].as_str().unwrap_or_default().starts_with("file.")
        })
        .expect("a query of the core")["id"]
        .as_str()
        .expect("its id")
        .to_owned();
    let answer = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": read, "input": {} }),
    );
    assert_ne!(code(answer), "plan_required", "{read} reads at once");
}

#[test]
fn steps_are_one_undo_step_planned_first() {
    let mut s = drawing();
    let mut guard = Guard::new();
    let before = objects(&s);
    let steps = json!([
        { "command": "cad.polygon.create", "input": area(&s, 0.0) },
        { "command": "cad.polygon.create", "input": area(&s, 10.0) },
    ]);
    let planned = tool(
        &mut s,
        &mut guard,
        "command.batch",
        json!({ "steps": steps, "label": "İki alan" }),
    )
    .expect("a plan");
    let plan = planned["planId"]
        .as_str()
        .expect("the plan's id")
        .to_owned();
    let written = tool(
        &mut s,
        &mut guard,
        "command.batch",
        json!({ "steps": steps, "label": "İki alan", "mode": "execute", "planId": plan }),
    );
    assert_eq!(code(written), "ok \"completed\"");
    assert_eq!(objects(&s), before + 2);
    assert!(s.undo().is_some());
    assert_eq!(objects(&s), before, "both in one undo step");
}

/// A module's program that does not offer architecture (docs/adr/8056 §1).
struct WithoutArchitecture<'a>(&'a mut Session);

impl Host for WithoutArchitecture<'_> {
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        self.0.request(method, params)
    }

    fn not_offered(&self, command: &str) -> Option<String> {
        (discipline_of(command) == Some(Discipline::Arch))
            .then(|| format!("“{command}” burada yok."))
    }
}

#[test]
fn a_host_that_does_not_offer_a_part_hides_and_refuses_it() {
    let mut s = drawing();
    let mut guard = Guard::new();
    let input = area(&s, 0.0);
    let mut host = WithoutArchitecture(&mut s);
    let parts = agent_tools::call(&mut host, &mut guard, "catalog.categories", &json!({}))
        .expect("categories");
    assert!(
        !parts["parts"]
            .as_array()
            .expect("parts")
            .iter()
            .any(|p| p["door"] == "arch.run")
    );
    let found = agent_tools::call(
        &mut host,
        &mut guard,
        "catalog.search",
        &json!({ "query": "duvar" }),
    )
    .expect("search");
    assert!(
        !found["commands"]
            .as_array()
            .expect("commands")
            .iter()
            .any(|c| c["door"] == "arch.run")
    );
    let wall = agent_tools::call(
        &mut host,
        &mut guard,
        "arch.run",
        &json!({ "command": "cad.walls.create", "input": {} }),
    );
    assert_eq!(code(wall), "not_offered");
    let smuggled = agent_tools::call(
        &mut host,
        &mut guard,
        "command.batch",
        &json!({ "steps": [{ "command": "cad.polygon.create", "input": input }, { "command": "cad.walls.create", "input": {} }] }),
    );
    assert_eq!(
        code(smuggled),
        "not_offered",
        "a step of a batch is checked too"
    );
}

#[test]
fn every_tool_is_listed_with_its_schema() {
    let names: Vec<&str> = agent_tools::tools().iter().map(|t| t.name).collect();
    for name in [
        "drawing.summary",
        "scene.entities",
        "catalog.search",
        "cad.run",
        "road.run",
        "command.batch",
    ] {
        assert!(names.contains(&name), "{name}");
    }
    assert!(
        agent_tools::tools()
            .iter()
            .all(|t| t.input["type"] == "object")
    );
}

#[test]
fn the_checks_and_the_processing_tools_have_doors_of_their_own() {
    let mut s = drawing();
    let mut guard = Guard::new();
    let providers = tool(&mut s, &mut guard, "audit.list_providers", json!({})).expect("providers");
    let list = providers["providers"].as_array().expect("providers");
    let topology = list
        .iter()
        .find(|p| p["command"] == "cad.topology.check")
        .expect("Topoloji denetimi");
    assert_eq!(
        topology["repair"], "cad.topology.repair",
        "a check names the repair of its findings"
    );
    assert!(
        list.iter()
            .any(|p| p["command"] == "cad.arch.audit" && p["door"] == "arch.run")
    );
    let not_a_check = tool(
        &mut s,
        &mut guard,
        "audit.run",
        json!({ "command": "cad.polygon.create", "input": {} }),
    );
    assert_eq!(code(not_a_check), "wrong_tool");
    let not_a_repair = tool(
        &mut s,
        &mut guard,
        "audit.apply_fix",
        json!({ "command": "cad.topology.check", "input": {} }),
    );
    assert_eq!(code(not_a_repair), "wrong_tool");
    let tools = tool(&mut s, &mut guard, "processing.list", json!({})).expect("İşlemler's tools");
    assert_eq!(tools["status"], "completed", "{tools}");
    let first = tools
        .pointer("/output/tools/0/id")
        .and_then(Value::as_str)
        .expect("a tool with an id")
        .to_owned();
    let described = tool(
        &mut s,
        &mut guard,
        "processing.describe",
        json!({ "tool": first }),
    )
    .expect("described");
    assert_eq!(described["id"], first.as_str());
    let unplanned = tool(
        &mut s,
        &mut guard,
        "processing.run",
        json!({ "input": { "tool": first }, "mode": "execute" }),
    );
    assert_eq!(
        code(unplanned),
        "plan_required",
        "a processing run writes: planned first"
    );
}
