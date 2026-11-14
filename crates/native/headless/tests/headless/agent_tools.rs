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
    // The desktop's assistant is offered no door the host never opens (docs/adr/8056 §4).
    let offered: Vec<&str> = agent_tools::offered_tools(&host)
        .iter()
        .map(|t| t.name)
        .collect();
    assert!(
        !offered.contains(&"arch.run"),
        "a part the host never runs has no door"
    );
    for name in [
        "cad.run",
        "map.run",
        "gis.run",
        "road.run",
        "command.batch",
        "catalog.search",
    ] {
        assert!(offered.contains(&name), "{name}");
    }
    assert_eq!(
        agent_tools::offered_tools(&s).len(),
        agent_tools::tools().len(),
        "a host of every part offers every tool"
    );
}

/// MCP's `kentos://session/overview` and `kentos://catalog/categories` (docs/adr/8056 §5): a drawing's summary, layer
/// tree and doors in one read; the catalog's parts before any drawing is open.
#[test]
fn an_agent_reads_a_drawing_and_the_catalog_in_one_read() {
    let mut s = drawing();
    let seen = agent_tools::overview(&mut s).expect("the overview");
    assert_eq!(seen["summary"]["name"], "Ajan");
    assert!(seen["summary"]["activeLayer"].is_string());
    assert!(
        seen["layers"]
            .as_array()
            .or_else(|| seen["layers"]["layers"].as_array())
            .is_some_and(|l| !l.is_empty()),
        "{}",
        seen["layers"]
    );
    assert_eq!(
        seen["doors"],
        json!(["cad.run", "arch.run", "map.run", "gis.run", "road.run"])
    );
    let mut host = WithoutArchitecture(&mut s);
    let seen = agent_tools::overview(&mut host).expect("the overview");
    assert_eq!(
        seen["doors"],
        json!(["cad.run", "map.run", "gis.run", "road.run"]),
        "a door the host never opens is not offered"
    );
    let parts = agent_tools::every_category();
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
}

/// A tool call in words for the person watching (docs/adr/8056 §4): the command's Turkish title and the call's mode,
/// the tool's title and what it is asked; never an id alone.
#[test]
fn a_tool_call_reads_in_words() {
    let words = |name: &str, args: Value| agent_tools::call_words(name, &args);
    assert_eq!(
        words(
            "arch.run",
            json!({ "command": "cad.walls.rectangle", "mode": "plan" })
        ),
        "Dikdörtgen duvar · plan"
    );
    assert_eq!(
        words(
            "cad.run",
            json!({ "command": "cad.polygon.create", "mode": "execute", "planId": "p" })
        ),
        "Kapalı alan oluştur · yazma"
    );
    assert_eq!(
        words("catalog.search", json!({ "query": "duvar" })),
        "Komut ara · “duvar”"
    );
    assert_eq!(
        words(
            "catalog.describe",
            json!({ "command": "cad.walls.rectangle" })
        ),
        "Komutun tanımı · Dikdörtgen duvar"
    );
    assert_eq!(
        words(
            "command.batch",
            json!({ "steps": [{}, {}, {}], "mode": "plan" })
        ),
        "Toplu yazma · 3 adım · plan"
    );
    assert_eq!(words("scene.entities", json!({})), "Nesneler");
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

#[test]
fn an_agent_verifies_an_extensions_folder_before_packing_it() {
    // plan/eklenti.md ADIM 21 (docs/adr/8057 §18): `extension.verify` reads a working folder without running it; each
    // finding says its file, its line and what to write.
    let mut s = drawing();
    let mut guard = Guard::new();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../samples/eklenti-vitrini/eklenti");
    let answer = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.verify",
        &json!({ "path": root.to_string_lossy() }),
    )
    .unwrap();
    assert_eq!(answer["valid"], true, "{answer:#}");
    assert_eq!(answer["extension"]["id"], "tr.ornek.vitrin");
    // A copy whose audit rule lost its mark.
    let copy = std::env::temp_dir().join(format!("kentos-verify-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&copy);
    for entry in walk(&root) {
        let to = copy.join(entry.strip_prefix(&root).unwrap());
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(&entry, &to).unwrap();
    }
    let logic = copy.join("src/vitrin/logic.py");
    let text = std::fs::read_to_string(&logic).unwrap();
    std::fs::write(&logic, text.replace("@ext.audit(\"yapi-no\")\n", "")).unwrap();
    let answer = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.verify",
        &json!({ "path": copy.to_string_lossy() }),
    )
    .unwrap();
    assert_eq!(answer["valid"], false);
    let problem = &answer["problems"][0];
    assert_eq!(problem["code"], "python_decorator_missing");
    assert_eq!(problem["file"], "src/vitrin/logic.py");
    assert!(
        problem["message"]
            .as_str()
            .unwrap()
            .contains("@ext.audit(\"yapi-no\")")
    );
    let _ = std::fs::remove_dir_all(&copy);
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "__pycache__") {
                out.extend(walk(&path));
            }
        } else {
            out.push(path);
        }
    }
    out
}

#[test]
fn an_agent_gets_a_blueprint_that_verifies() {
    // docs/adr/8057 §19: `extension.blueprint` gives the files; written out, `extension.verify` passes them.
    let mut s = drawing();
    let mut guard = Guard::new();
    let answer = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.blueprint",
        &json!({ "id": "tr.ornek.mezarlik", "name": "Mezarlık bilgi",
                 "contributions": ["command", "processing", "audit", "expression", "mapTool"] }),
    )
    .unwrap();
    let files = answer["files"].as_object().expect("files");
    assert!(files.contains_key("manifest.json") && files.contains_key("src/mezarlik/logic.py"));
    let folder = std::env::temp_dir().join(format!("kentos-blueprint-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    for (path, text) in files {
        let to = folder.join(path);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::write(&to, text.as_str().unwrap()).unwrap();
    }
    let verified = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.verify",
        &json!({ "path": folder.to_string_lossy() }),
    )
    .unwrap();
    assert_eq!(verified["valid"], true, "{verified:#}");
    assert_eq!(verified["warnings"].as_array().map(Vec::len), Some(0));
    // A wrong contribution is said.
    let refused = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.blueprint",
        &json!({ "id": "tr.ornek.mezarlik", "name": "M", "contributions": ["panel"] }),
    )
    .unwrap_err();
    assert!(refused.to_string().contains("bir katkı değil"), "{refused}");
    let _ = std::fs::remove_dir_all(&folder);
}
