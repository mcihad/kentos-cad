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
        agent_tools::tools()
            .iter()
            .filter(|t| !t.name.starts_with("extension.workshop."))
            .count(),
        "a host of every part offers every tool but the workshop's: it has none"
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
        "Katalogda ara · “duvar”"
    );
    assert_eq!(
        words(
            "catalog.describe",
            json!({ "command": "cad.walls.rectangle" })
        ),
        "Tanım · Dikdörtgen duvar"
    );
    assert_eq!(
        words(
            "command.batch",
            json!({ "steps": [{}, {}, {}], "mode": "plan" })
        ),
        "Toplu yazma · 3 adım · plan"
    );
    assert_eq!(words("scene.entities", json!({})), "Nesneler");
    // An İşlemler run by its tool, a model written for the run by its name and steps (docs/adr/8057 §108).
    assert_eq!(
        words(
            "processing.run",
            json!({ "input": { "tool": "geometry.buffer" }, "mode": "plan" })
        ),
        "İşlem aracını çalıştır · geometry.buffer · plan"
    );
    assert_eq!(
        words(
            "processing.run",
            json!({ "input": { "tool": "model:", "model": { "id": "tb", "label": "Tampon ve birleştir",
                "steps": [{}, {}] } }, "mode": "execute", "planId": "p" })
        ),
        "İşlem aracını çalıştır · model “Tampon ve birleştir”, 2 adım · yazma"
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

/// İşlemler's list as an agent reads it first (docs/adr/8057 §108): an index of ids, names, categories and first
/// sentences, without parameters (`processing.describe` gives a tool's); `query` narrows it. The whole list with every
/// parameter stopped a real agent at 90 900 of its 80 000 tokens before it ran anything.
#[test]
fn the_processing_list_is_an_index_an_agent_can_afford() {
    let mut s = drawing();
    let mut guard = Guard::new();
    let all = tool(&mut s, &mut guard, "processing.list", json!({})).expect("the index");
    let tools = all
        .pointer("/output/tools")
        .and_then(Value::as_array)
        .expect("tools");
    let size = all.to_string().len();
    assert!(
        tools.iter().all(|t| t.get("parameters").is_none()
            && t["id"].is_string()
            && t["label"].is_string())
    );
    // 157 tools: 35 102 bytes as one index, the whole list 274 957 (17 Nov); a page of 50 (§114).
    assert_eq!(tools.len(), 50);
    assert!(size < 16_000, "{size} bytes for a page");
    // Paged to its end by next.offset: every tool once.
    let total = all["total"].as_u64().expect("its total") as usize;
    let mut seen: Vec<String> = Vec::new();
    let mut page = all.clone();
    loop {
        seen.extend(
            page.pointer("/output/tools")
                .and_then(Value::as_array)
                .expect("tools")
                .iter()
                .filter_map(|t| t["id"].as_str().map(str::to_owned)),
        );
        let Some(next) = page["next"]["offset"].as_u64() else {
            break;
        };
        page = tool(
            &mut s,
            &mut guard,
            "processing.list",
            json!({ "offset": next }),
        )
        .expect("a page");
    }
    assert_eq!(seen.len(), total);
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), total, "each tool once");
    let tools = all
        .pointer("/output/tools")
        .and_then(Value::as_array)
        .expect("tools");
    let narrowed = tool(
        &mut s,
        &mut guard,
        "processing.list",
        json!({ "query": "tampon" }),
    )
    .expect("narrowed");
    let ids: Vec<&str> = narrowed
        .pointer("/output/tools")
        .and_then(Value::as_array)
        .expect("tools")
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    assert!(ids.contains(&"geometry.buffer"), "{ids:?}");
    assert!(ids.len() < tools.len());
    // Its parameters come with processing.describe.
    let described = tool(
        &mut s,
        &mut guard,
        "processing.describe",
        json!({ "tool": "geometry.buffer" }),
    )
    .expect("described");
    assert!(
        described["parameters"]
            .as_array()
            .is_some_and(|p| !p.is_empty())
    );
    // And its outputs, which a model's later step is wired to by name.
    assert_eq!(described["outputs"][0]["name"], "buffers", "{described:#}");
    assert_eq!(described["outputs"][0]["kind"], "features");
    // Words of two tools: each word's tools.
    let both = tool(
        &mut s,
        &mut guard,
        "processing.list",
        json!({ "query": "tampon birleştir" }),
    )
    .expect("both");
    let ids: Vec<&str> = both
        .pointer("/output/tools")
        .and_then(Value::as_array)
        .expect("tools")
        .iter()
        .filter_map(|t| t["id"].as_str())
        .collect();
    assert!(
        ids.contains(&"geometry.buffer") && ids.contains(&"geometry.dissolve"),
        "{ids:?}"
    );
    assert!(
        both["note"]
            .as_str()
            .is_some_and(|n| n.contains("her sözcüğün")),
        "{both}"
    );
    // Nothing found says what to do.
    let none = tool(
        &mut s,
        &mut guard,
        "processing.list",
        json!({ "query": "zzqx" }),
    )
    .expect("an answer");
    assert!(
        none["note"]
            .as_str()
            .is_some_and(|n| n.contains("başka bir sözcükle")),
        "{none}"
    );
    assert_eq!(
        agent_tools::call_words("processing.list", &json!({ "query": "tampon" })),
        "İşlem araçları · “tampon”"
    );
}

/// The common catalog (docs/adr/8057 §109, PY-AI-01): a search finds İşlemler's tools and the answers' codes beside
/// the commands, each saying what to call next; a code is described by what it means; the installed extensions are
/// the desktop's, which a drawing of the server's own says.
#[test]
fn the_common_catalog_finds_tools_and_codes_and_says_what_next() {
    let mut s = drawing();
    let mut guard = Guard::new();
    let found = tool(
        &mut s,
        &mut guard,
        "catalog.search",
        json!({ "query": "tampon" }),
    )
    .expect("found");
    let buffer = found["others"]
        .as_array()
        .expect("others")
        .iter()
        .find(|o| o["id"] == "geometry.buffer")
        .unwrap_or_else(|| panic!("{found:#}"));
    assert_eq!(
        (buffer["kind"].as_str(), buffer["next"].as_str()),
        (Some("tool"), Some("processing.describe"))
    );
    assert!(found["othersTotal"].as_u64().is_some_and(|n| n >= 1));
    // A code written as one is found first, and described.
    let code = tool(
        &mut s,
        &mut guard,
        "catalog.search",
        json!({ "query": "plan_changed" }),
    )
    .expect("a code");
    assert_eq!(code["others"][0]["id"], "plan_changed", "{code:#}");
    assert_eq!(code["others"][0]["kind"], "code");
    let described = tool(
        &mut s,
        &mut guard,
        "catalog.describe",
        json!({ "code": "plan_changed" }),
    )
    .expect("described");
    assert_eq!(
        (described["kind"].as_str(), described["retryable"].as_bool()),
        (Some("conflict"), Some(true))
    );
    assert!(described["meaning"].as_str().is_some_and(|m| !m.is_empty()));
    let unknown = tool(
        &mut s,
        &mut guard,
        "catalog.describe",
        json!({ "code": "yok_boyle" }),
    );
    assert_eq!(code_of(unknown), "unknown_code");
    // Only codes: no commands.
    let codes = tool(
        &mut s,
        &mut guard,
        "catalog.search",
        json!({ "query": "plan", "kinds": ["code"] }),
    )
    .expect("codes");
    assert_eq!(codes["total"], 0);
    assert!(
        codes["others"]
            .as_array()
            .expect("others")
            .iter()
            .all(|o| o["kind"] == "code")
    );
    // The extensions are the desktop's: asked by name here, the answer says so.
    let asked = tool(
        &mut s,
        &mut guard,
        "catalog.search",
        json!({ "query": "çekme", "kinds": ["extension"] }),
    )
    .expect("asked");
    assert!(
        asked["notes"][0]
            .as_str()
            .is_some_and(|n| n.contains("masaüstü")),
        "{asked:#}"
    );
    let ext = tool(
        &mut s,
        &mut guard,
        "catalog.describe",
        json!({ "extension": "tr.ornek.vitrin" }),
    );
    assert_eq!(code_of(ext), "not_offered");
    // Paged (§114): a window over the commands and the others, the next one by next.offset.
    let first = tool(
        &mut s,
        &mut guard,
        "catalog.search",
        json!({ "query": "katman", "limit": 5 }),
    )
    .expect("a page");
    assert_eq!(first["commands"].as_array().map(Vec::len), Some(5));
    assert_eq!(first["next"]["offset"], 5, "{first:#}");
    let second = tool(
        &mut s,
        &mut guard,
        "catalog.search",
        json!({ "query": "katman", "limit": 5, "offset": 5 }),
    )
    .expect("the next page");
    assert_ne!(
        first["commands"][0]["command"],
        second["commands"][0]["command"]
    );
    assert_eq!(first["total"], second["total"]);
    let nothing = tool(&mut s, &mut guard, "catalog.describe", json!({}));
    assert_eq!(code_of(nothing), "invalid_input");
    assert_eq!(
        agent_tools::call_words("catalog.describe", &json!({ "code": "plan_changed" })),
        "Tanım · plan_changed"
    );
}

/// An error's code, else what the answer was.
fn code_of(answer: Result<Value, HeadlessError>) -> String {
    match answer {
        Ok(v) => format!("ok {v}"),
        Err(e) => e.code.to_owned(),
    }
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
    std::fs::write(&logic, text).unwrap();
    // An İşlemler tool KentOS's processing would not make: said here as the install review says it (an agent's
    // package passed and was refused at Kur, 15 November 2026).
    let descriptor = copy.join("tools/taks.tool.json");
    let tool = std::fs::read_to_string(&descriptor).unwrap();
    std::fs::write(
        &descriptor,
        tool.replace("\"sinkParameter\": \"cikti\"", "\"sinkParameter\": null"),
    )
    .unwrap();
    let answer = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.verify",
        &json!({ "path": copy.to_string_lossy() }),
    )
    .unwrap();
    assert_eq!(answer["valid"], false, "{answer:#}");
    let problem = &answer["problems"][0];
    assert_eq!(problem["code"], "tool_descriptor");
    assert_eq!(problem["file"], "tools/taks.tool.json");
    assert!(
        problem["message"]
            .as_str()
            .unwrap()
            .contains("sinkParameter"),
        "{problem}"
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
    // docs/adr/8057 §105: the packages it may name, each with what it is for; a name not among them is not made up.
    let packages = answer["packages"]["list"].as_array().expect("the packages");
    assert!(
        packages.iter().any(|p| p["name"] == "shapely"
            && p["purpose"]
                .as_str()
                .is_some_and(|w| w.contains("geometri"))),
        "{packages:?}"
    );
    assert!(
        answer["packages"]["note"]
            .as_str()
            .is_some_and(|n| n.contains("uydurmayın"))
    );
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
        &json!({ "id": "tr.ornek.mezarlik", "name": "M", "contributions": ["dialog"] }),
    )
    .unwrap_err();
    assert!(refused.to_string().contains("bir katkı değil"), "{refused}");
    let _ = std::fs::remove_dir_all(&folder);
}

/// A drawing whose organisation lists the only Python packages an extension may install.
struct WithAllowedPackages<'a>(&'a mut Session);

impl Host for WithAllowedPackages<'_> {
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        self.0.request(method, params)
    }

    fn allowed_packages(&self) -> Option<Vec<String>> {
        Some(vec!["numpy>=2".to_owned(), "kurum-paketi".to_owned()])
    }
}

/// docs/adr/8057 §105: under the organisation's list the blueprint offers the packages it allows alone, and says so.
#[test]
fn the_blueprint_offers_the_packages_the_policy_allows() {
    let mut s = drawing();
    let answer = agent_tools::call(
        &mut WithAllowedPackages(&mut s),
        &mut Guard::new(),
        "extension.blueprint",
        &json!({ "id": "tr.ornek.mezarlik", "name": "Mezarlık bilgi", "contributions": ["command"] }),
    )
    .unwrap();
    let names: Vec<&str> = answer["packages"]["list"]
        .as_array()
        .expect("the packages")
        .iter()
        .filter_map(|p| p["name"].as_str())
        .collect();
    assert_eq!(names, ["numpy", "kurum-paketi"]);
    assert!(
        answer["packages"]["note"]
            .as_str()
            .is_some_and(|n| n.contains("Kurum ilkesi"))
    );
}

#[test]
fn an_agent_reads_the_sdk_before_writing_an_extension() {
    // docs/adr/8057 §20: the pack in one read, or one part; a part the pack has not is said with the ones it has.
    let mut s = drawing();
    let mut guard = Guard::new();
    let pack = agent_tools::call(&mut s, &mut guard, "extension.sdk", &json!({})).unwrap();
    assert_eq!(pack["format"], "kentos.ai-kek-pack");
    assert!(pack["guide"].as_str().unwrap().contains("@ext.processing"));
    assert!(
        pack["api"]["stub"]
            .as_str()
            .unwrap()
            .contains("class ProcessingContext(CommandContext):")
    );
    // The working example is a part of its own; the pack names its files and says how.
    assert!(pack["example"]["how"].is_string());
    let example = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.sdk",
        &json!({ "part": "example" }),
    )
    .unwrap();
    assert!(example["value"]["manifest.json"].is_string());
    let part = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.sdk",
        &json!({ "part": "verifyCodes" }),
    )
    .unwrap();
    assert_eq!(part["part"], "verifyCodes");
    assert!(part["value"]["python_decorator_missing"].is_string());
    let refused = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.sdk",
        &json!({ "part": "paneller" }),
    )
    .unwrap_err();
    assert!(refused.to_string().contains("guide, api"), "{refused}");
}

/// A drawing with the agents' extension workshop, its reviews kept (docs/adr/8057 §22).
struct WithWorkshop<'a> {
    inner: &'a mut Session,
    root: std::path::PathBuf,
    reviewed: Vec<std::path::PathBuf>,
    reloaded: Vec<String>,
}

impl Host for WithWorkshop<'_> {
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        self.inner.request(method, params)
    }

    fn workshop(&self) -> Option<std::path::PathBuf> {
        Some(self.root.clone())
    }

    fn author(&self) -> Option<agent_tools::Author> {
        Some(agent_tools::Author {
            agent: "Sınama ajanı".to_owned(),
            model: Some("sınama-modeli".to_owned()),
        })
    }

    fn review_package(&mut self, path: &std::path::Path) -> Result<Value, HeadlessError> {
        self.reviewed.push(path.to_owned());
        Ok(json!({ "opened": true }))
    }

    fn reload_extension(&mut self, id: &str) -> Result<Value, HeadlessError> {
        self.reloaded.push(id.to_owned());
        Ok(json!({ "id": id, "changed": true }))
    }

    fn preview_extension(&mut self, id: &str) -> Result<Value, HeadlessError> {
        Ok(json!({ "id": id, "ribbonTab": { "label": "Mezarlık" } }))
    }

    fn dry_run(&self, package: &[u8]) -> Option<Value> {
        Some(json!({ "clean": true, "bytes": package.len() }))
    }
}

#[test]
fn an_agent_writes_an_extension_in_its_workshop_and_opens_it_for_review() {
    // docs/adr/8057 §22: the blueprint written to the workshop, verified by its id, listed and read; a call with a
    // path outside the folder writes nothing; the package is made beside the folder and opened for review; a broken
    // extension is not packed.
    let mut s = drawing();
    let mut guard = Guard::new();
    let names = |host: &dyn Host| -> Vec<&'static str> {
        agent_tools::offered_tools(host)
            .iter()
            .map(|t| t.name)
            .filter(|n| n.starts_with("extension.workshop."))
            .collect()
    };
    assert!(names(&s).is_empty(), "no workshop, no workshop tools");
    let root = std::env::temp_dir().join(format!("kentos-atolye-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let blueprint = agent_tools::call(
        &mut s,
        &mut guard,
        "extension.blueprint",
        &json!({ "id": "tr.ornek.mezarlik", "name": "Mezarlık bilgi", "contributions": ["command", "audit"] }),
    )
    .unwrap();
    let mut host = WithWorkshop {
        inner: &mut s,
        root: root.clone(),
        reviewed: Vec::new(),
        reloaded: Vec::new(),
    };
    assert_eq!(
        names(&host),
        [
            "extension.workshop.write",
            "extension.workshop.read",
            "extension.workshop.review",
            "extension.workshop.reload",
            "extension.workshop.preview",
            "extension.workshop.submit",
            "extension.workshop.trial"
        ]
    );
    let call = |host: &mut WithWorkshop, guard: &mut Guard, name: &str, args: Value| {
        agent_tools::call(host, guard, name, &args)
    };
    // docs/adr/8057 §107: a host that cannot open the user's Kataloğa gönder window says where it can be opened.
    let refused = call(
        &mut host,
        &mut guard,
        "extension.workshop.submit",
        json!({ "id": "tr.ornek.mezarlik" }),
    )
    .unwrap_err();
    assert_eq!(refused.code, "not_offered");
    assert!(
        refused.message.contains("desktop.attach"),
        "{}",
        refused.message
    );
    // The next step a host with a workshop is told: write there, not to a folder of the user's.
    let told = call(
        &mut host,
        &mut guard,
        "extension.blueprint",
        json!({ "id": "tr.ornek.mezarlik", "name": "Mezarlık bilgi", "contributions": ["command"] }),
    )
    .unwrap();
    assert!(
        told["next"]
            .as_str()
            .unwrap()
            .contains("extension.workshop.write"),
        "{told:#}"
    );
    assert!(
        blueprint["next"]
            .as_str()
            .unwrap()
            .contains("boş bir klasöre"),
        "without a workshop, a folder of its own"
    );
    let written = call(
        &mut host,
        &mut guard,
        "extension.workshop.write",
        json!({ "id": "tr.ornek.mezarlik", "files": blueprint["files"] }),
    )
    .unwrap();
    assert_eq!(
        written["written"].as_array().map(Vec::len),
        blueprint["files"].as_object().map(|f| f.len())
    );
    let verified = call(
        &mut host,
        &mut guard,
        "extension.verify",
        json!({ "id": "tr.ornek.mezarlik" }),
    )
    .unwrap();
    assert_eq!(verified["valid"], true, "{verified:#}");
    // Kapı 3's part that runs no code: the host bound the folder's package (docs/adr/8057 §26).
    assert_eq!(verified["dryRun"]["clean"], true, "{verified:#}");
    assert!(verified["dryRun"]["bytes"].as_u64().unwrap() > 0);
    let listed = call(&mut host, &mut guard, "extension.workshop.read", json!({})).unwrap();
    assert_eq!(listed["extensions"][0]["id"], "tr.ornek.mezarlik");
    assert_eq!(listed["extensions"][0]["name"], "Mezarlık bilgi");
    let manifest = call(
        &mut host,
        &mut guard,
        "extension.workshop.read",
        json!({ "id": "tr.ornek.mezarlik", "path": "manifest.json" }),
    )
    .unwrap();
    assert!(
        manifest["content"]
            .as_str()
            .unwrap()
            .contains("tr.ornek.mezarlik")
    );
    // Outside the folder: refused, and the good file of the same call not written either.
    let refused = call(
        &mut host,
        &mut guard,
        "extension.workshop.write",
        json!({ "id": "tr.ornek.mezarlik", "files": { "NOTLAR.md": "x", "../disari.py": "x" } }),
    )
    .unwrap_err();
    assert_eq!(refused.code, "invalid_input", "{refused}");
    assert!(!root.join("tr.ornek.mezarlik/NOTLAR.md").exists());
    assert!(!root.join("disari.py").exists());
    // Packed beside the folder, opened for review.
    let review = call(
        &mut host,
        &mut guard,
        "extension.workshop.review",
        json!({ "id": "tr.ornek.mezarlik" }),
    )
    .unwrap();
    assert_eq!(review["valid"], true, "{review:#}");
    let package = root.join("tr.ornek.mezarlik-0.1.0.kek");
    assert!(package.is_file());
    // Who wrote it, KentOS's word (docs/adr/8057 §76): in the package the user reviews, and in the folder.
    let opened = kentos_extensions::open(
        &std::fs::read(&package).unwrap(),
        &kentos_extensions::Limits::default(),
    );
    let generated = opened
        .inspection
        .manifest
        .and_then(|m| m.generated)
        .expect("the workshop's stamp");
    assert_eq!(
        (
            generated.agent.as_str(),
            generated.model.as_deref(),
            generated.kentos.as_str()
        ),
        (
            "Sınama ajanı",
            Some("sınama-modeli"),
            env!("CARGO_PKG_VERSION")
        )
    );
    let folder = std::fs::read_to_string(root.join("tr.ornek.mezarlik/manifest.json")).unwrap();
    assert!(folder.contains("Sınama ajanı"), "{folder}");
    assert_eq!(host.reviewed, [package]);
    // A function that lost its mark: not packed, said.
    let logic = call(
        &mut host,
        &mut guard,
        "extension.workshop.read",
        json!({ "id": "tr.ornek.mezarlik", "path": "src/mezarlik/logic.py" }),
    )
    .unwrap();
    let broken = logic["content"]
        .as_str()
        .unwrap()
        .replace("@ext.audit(\"adsiz\")\n", "");
    call(
        &mut host,
        &mut guard,
        "extension.workshop.write",
        json!({ "id": "tr.ornek.mezarlik", "files": { "src/mezarlik/logic.py": broken } }),
    )
    .unwrap();
    let review = call(
        &mut host,
        &mut guard,
        "extension.workshop.review",
        json!({ "id": "tr.ornek.mezarlik" }),
    )
    .unwrap();
    assert_eq!(review["valid"], false);
    assert_eq!(review["problems"][0]["code"], "python_decorator_missing");
    assert_eq!(host.reviewed.len(), 1, "not opened again");
    // Nor loaded over the installed one (plan ADIM 21's hot_reload): verified as the review verifies.
    let reload = call(
        &mut host,
        &mut guard,
        "extension.workshop.reload",
        json!({ "id": "tr.ornek.mezarlik" }),
    )
    .unwrap();
    assert_eq!(reload["valid"], false);
    assert!(host.reloaded.is_empty());
    call(
        &mut host,
        &mut guard,
        "extension.workshop.write",
        json!({ "id": "tr.ornek.mezarlik", "files": { "src/mezarlik/logic.py": logic["content"] } }),
    )
    .unwrap();
    let reload = call(
        &mut host,
        &mut guard,
        "extension.workshop.reload",
        json!({ "id": "tr.ornek.mezarlik" }),
    )
    .unwrap();
    assert_eq!(
        (
            reload["valid"].clone(),
            reload["reloaded"]["changed"].clone()
        ),
        (json!(true), json!(true)),
        "{reload:#}"
    );
    assert_eq!(host.reloaded, ["tr.ornek.mezarlik"]);
    let preview = call(
        &mut host,
        &mut guard,
        "extension.workshop.preview",
        json!({ "id": "tr.ornek.mezarlik" }),
    )
    .unwrap();
    assert_eq!(preview["ribbonTab"]["label"], "Mezarlık");
    // A file removed with null.
    call(
        &mut host,
        &mut guard,
        "extension.workshop.write",
        json!({ "id": "tr.ornek.mezarlik", "files": { "CHANGELOG.md": null } }),
    )
    .unwrap();
    assert!(!root.join("tr.ornek.mezarlik/CHANGELOG.md").exists());
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn the_workshop_does_not_write_or_pack_through_a_link() {
    // docs/adr/8057 §22: a link in the workshop could lead a write, a read or the package outside it.
    let mut s = drawing();
    let mut guard = Guard::new();
    let root = std::env::temp_dir().join(format!("kentos-atolye-bag-{}", std::process::id()));
    let outside = root.join("disari");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::create_dir_all(root.join("tr.ornek.bag")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("tr.ornek.bag/src")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("tr.ornek.tum")).unwrap();
    let mut host = WithWorkshop {
        inner: &mut s,
        root: root.clone(),
        reviewed: Vec::new(),
        reloaded: Vec::new(),
    };
    for (id, path) in [
        ("tr.ornek.bag", "src/x.py"),
        ("tr.ornek.tum", "manifest.json"),
    ] {
        let refused = agent_tools::call(
            &mut host,
            &mut guard,
            "extension.workshop.write",
            &json!({ "id": id, "files": { path: "x" } }),
        )
        .unwrap_err();
        assert!(refused.message.contains("bağlantı"), "{refused}");
    }
    assert!(
        std::fs::read_dir(&outside).unwrap().next().is_none(),
        "nothing written outside"
    );
    let refused = agent_tools::call(
        &mut host,
        &mut guard,
        "extension.workshop.review",
        &json!({ "id": "tr.ornek.bag" }),
    )
    .unwrap_err();
    assert!(refused.message.contains("bağlantı"), "{refused}");
    assert!(host.reviewed.is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

/// A desktop with one extension audit rule, as its link answers (docs/adr/8057 §27): the run goes for two reports,
/// then its two findings come, the first with a fix (an area drawn); a write is the drawing's own (a session's), the
/// fix marked.
struct AuditingDesktop {
    drawing: Session,
    reports: usize,
    marked: Vec<u64>,
}

const RULE: &str = "ext.tr.ornek.vitrin.yapi-no";

impl Host for AuditingDesktop {
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        match method {
            "audits" => Ok(json!({ "rules": [{ "rule": RULE, "title": "Yapı numarası",
                "extension": { "id": "tr.ornek.vitrin", "name": "Eklenti Vitrini", "version": "1.0.19" },
                "program": "cbs", "ready": true, "why": null }] })),
            "audit" if params["rule"] == RULE => Ok(json!({ "status": "running", "rule": RULE })),
            "audit" => Err(HeadlessError::new("unknown_rule", "Böyle bir denetim yok.")),
            "audit_report" => {
                self.reports += 1;
                if self.reports <= 2 {
                    return Ok(json!({ "status": "running", "rule": RULE }));
                }
                let findings = [
                    json!({ "finding": 1, "severity": "warning", "message": "Yapının alanı çizilmemiş.",
                            "featureId": null, "fixed": false, "fixRefused": null,
                            "fix": { "title": "Yapı alanını çiz", "command": "cad.polygon.create",
                                     "input": area(&self.drawing, 10.0) } }),
                    json!({ "finding": 2, "severity": "info", "message": "İlk nesneler denetlendi.",
                            "featureId": null, "fixed": false, "fix": null,
                            "fixRefused": "Düzeltme çizime yazar; bu eklentiye yazma izni (document.write) verilmedi." }),
                ];
                let from = usize::try_from(params["offset"].as_u64().unwrap_or(0)).unwrap_or(0);
                Ok(
                    json!({ "status": "done", "rule": RULE, "title": "Yapı numarası",
                    "extension": { "id": "tr.ornek.vitrin", "version": "1.0.19", "digest": "sha256:ab" },
                    "revision": 3, "findings": findings.iter().skip(from).cloned().collect::<Vec<_>>(),
                    "counts": { "error": 0, "warning": 1, "info": 1 } }),
                )
            }
            "audit_fixed" => {
                self.marked.push(params["finding"].as_u64().unwrap_or(99));
                Ok(json!({ "fixed": true }))
            }
            _ => self.drawing.request(method, params),
        }
    }
}

#[test]
fn an_extensions_audit_rule_runs_through_the_agents_checks() {
    // docs/adr/8057 §27 (plan ADIM 20's AI/MCP audit.run): a desktop's extension rule is among the checks; audit.run
    // starts it and, on the link, asks its report until the run ends; a finding's fix goes through the owner's guard
    // and, written, is marked; a finding the user's Düzelt could not apply says why.
    let mut desktop = AuditingDesktop {
        drawing: drawing(),
        reports: 0,
        marked: Vec::new(),
    };
    let mut guard = Guard::new();
    let providers =
        agent_tools::call(&mut desktop, &mut guard, "audit.list_providers", &json!({})).unwrap();
    let rule = providers["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["command"] == RULE)
        .expect("the extension's rule among the checks");
    assert_eq!(
        (rule["door"].clone(), rule["ready"].clone()),
        (json!("audit.run"), json!(true))
    );
    assert!(
        providers["providers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["command"] == "cad.topology.check"),
        "the built-in checks stay"
    );
    // Described as what it is: no input, run by audit.run (an agent describes before it runs).
    let described = agent_tools::call(
        &mut desktop,
        &mut guard,
        "catalog.describe",
        &json!({ "command": RULE }),
    )
    .unwrap();
    assert_eq!(
        (described["door"].clone(), described["title"].clone()),
        (json!("audit.run"), json!("Yapı numarası"))
    );
    let report = agent_tools::call(
        &mut desktop,
        &mut guard,
        "audit.run",
        &json!({ "command": RULE }),
    )
    .unwrap();
    assert_eq!(report["status"], "done", "{report:#}");
    assert_eq!(desktop.reports, 3, "asked until the run ended");
    assert_eq!(
        agent_tools::call_words("audit.run", &json!({ "command": RULE })),
        "Denetle · eklenti denetimi yapi-no"
    );
    // A fix the user's Düzelt could not apply: why, nothing run.
    let refused = agent_tools::call(
        &mut desktop,
        &mut guard,
        "audit.apply_fix",
        &json!({ "rule": RULE, "finding": 2, "mode": "plan" }),
    );
    assert!(
        matches!(&refused, Err(e) if e.code == "not_fixable" && e.message.contains("document.write")),
        "{refused:?}"
    );
    // Another rule's finding: run that rule first.
    let stale = agent_tools::call(
        &mut desktop,
        &mut guard,
        "audit.apply_fix",
        &json!({ "rule": "ext.tr.ornek.baska.kural", "finding": 1 }),
    );
    assert_eq!(code(stale), "stale_report");
    // The fix through the guard: planned, executed as planned, marked.
    let before = objects(&desktop.drawing);
    let unplanned = agent_tools::call(
        &mut desktop,
        &mut guard,
        "audit.apply_fix",
        &json!({ "rule": RULE, "finding": 1, "mode": "execute" }),
    );
    assert_eq!(code(unplanned), "plan_required");
    let zero = agent_tools::call(
        &mut desktop,
        &mut guard,
        "audit.apply_fix",
        &json!({ "rule": RULE, "finding": 0, "mode": "plan" }),
    );
    assert_eq!(
        code(zero),
        "invalid_input",
        "numbers start at 1, as the user counts"
    );
    let plan = agent_tools::call(
        &mut desktop,
        &mut guard,
        "audit.apply_fix",
        &json!({ "rule": RULE, "finding": 1, "mode": "plan" }),
    )
    .unwrap();
    let plan_id = plan["planId"].as_str().expect("a plan").to_owned();
    assert_eq!(objects(&desktop.drawing), before, "a plan writes nothing");
    let done = agent_tools::call(
        &mut desktop,
        &mut guard,
        "audit.apply_fix",
        &json!({ "rule": RULE, "finding": 1, "mode": "execute", "planId": plan_id }),
    )
    .unwrap();
    assert_eq!(
        (done["status"].clone(), done["marked"].clone()),
        (json!("completed"), json!(true)),
        "{done:#}"
    );
    assert_eq!(objects(&desktop.drawing), before + 1);
    assert_eq!(desktop.marked, [1]);
    // A rule that failed is the call's failure, with why.
    let failed = agent_tools::audit_outcome(
        json!({ "status": "failed", "error": "TypeError: liste değil" }),
    );
    assert!(
        matches!(&failed, Err(e) if e.code == "audit_failed" && e.message.contains("TypeError")),
        "{failed:?}"
    );
}

#[test]
fn a_drawing_of_the_servers_own_has_no_extension_audits() {
    // docs/adr/8057 §27: the server's drawings have no extensions; their checks are the built-in ones, and an
    // extension rule or report says where they run.
    let mut s = drawing();
    let mut guard = Guard::new();
    let providers = tool(&mut s, &mut guard, "audit.list_providers", json!({})).unwrap();
    assert!(
        providers["providers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| !p["command"]
                .as_str()
                .unwrap_or_default()
                .starts_with("ext.")),
        "no extension rule"
    );
    for (name, args) in [
        ("audit.run", json!({ "command": RULE })),
        ("audit.report", json!({})),
    ] {
        let refused = tool(&mut s, &mut guard, name, args);
        assert!(
            matches!(&refused, Err(e) if e.code == "not_offered" && e.message.contains("KentOS masaüstünde")),
            "{name}: {refused:?}"
        );
    }
}

/// A desktop on the link as its workshop requests reach it (docs/adr/8057 §30): what it was asked, by method.
struct LinkedDesktop {
    asked: Vec<(String, Value)>,
}

impl Host for LinkedDesktop {
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        self.asked.push((method.to_owned(), params.clone()));
        match method {
            "review_workshop" => Ok(json!({ "opened": true, "installable": true })),
            "reload_extension" => Ok(json!({ "id": params["id"], "changed": true })),
            "preview_extension" => Ok(json!({ "id": params["id"], "name": "Mezarlık bilgi" })),
            _ => Err(HeadlessError::new("unknown_method", method.to_owned())),
        }
    }
}

#[test]
fn an_agents_workshop_reaches_the_desktop_on_its_link() {
    // docs/adr/8057 §30: on a desktop's link the workshop's package opens in the user's review there, the changes of
    // what the user installed from its folder load, and its look is read; a drawing of the server's own says where the
    // package is opened instead.
    let package = std::path::Path::new("/atolye/tr.ornek.mezarlik-0.1.0.kek");
    let folder = std::path::Path::new("/atolye/tr.ornek.mezarlik");
    let mut desktop = LinkedDesktop { asked: Vec::new() };
    let opened = agent_tools::link_review_workshop(&mut desktop, package, folder).unwrap();
    assert_eq!(opened["opened"], true);
    agent_tools::link_reload(&mut desktop, "tr.ornek.mezarlik", folder).unwrap();
    let look = agent_tools::link_preview(&mut desktop, "tr.ornek.mezarlik").unwrap();
    assert_eq!(look["name"], "Mezarlık bilgi");
    let asked: Vec<(&str, Value)> = desktop
        .asked
        .iter()
        .map(|(m, p)| (m.as_str(), p.clone()))
        .collect();
    assert_eq!(
        asked,
        [
            (
                "review_workshop",
                json!({ "package": package.display().to_string(), "folder": folder.display().to_string() })
            ),
            (
                "reload_extension",
                json!({ "id": "tr.ornek.mezarlik", "folder": folder.display().to_string() })
            ),
            ("preview_extension", json!({ "id": "tr.ornek.mezarlik" })),
        ]
    );
    // A drawing of the server's own has no review to open: it says how the user opens the package.
    let mut s = drawing();
    let refused = agent_tools::link_review_workshop(&mut s, package, folder);
    assert!(
        matches!(&refused, Err(e) if e.code == "not_offered" && e.message.contains("Paket yükle")),
        "{refused:?}"
    );
    let refused = agent_tools::link_reload(&mut s, "tr.ornek.mezarlik", folder);
    assert!(
        matches!(&refused, Err(e) if e.code == "not_offered"),
        "{refused:?}"
    );
}

/// A desktop whose jobs end at the second look (docs/adr/8057 §27): what it was asked, a plan's id from its plan job.
struct JobDesktop {
    asked: Vec<(String, Value)>,
    looks: usize,
    later: bool,
}

impl Host for JobDesktop {
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        self.asked.push((method.to_owned(), params.clone()));
        match method {
            "summary" => Ok(json!({ "revision": "7" })),
            "start" => Ok(
                json!({ "id": format!("is-{}", self.asked.len()), "command": params["command"], "phase": "computing" }),
            ),
            "job" => {
                self.looks += 1;
                if self.looks % 2 == 1 {
                    return Ok(
                        json!({ "id": params["id"], "command": "cad.processing.run", "phase": "computing" }),
                    );
                }
                let plan = self
                    .asked
                    .iter()
                    .rev()
                    .find(|(m, _)| m == "start")
                    .is_some_and(|(_, p)| p["op"] == "plan");
                let answer = if plan {
                    json!({ "status": "completed", "planId": "plan-taks" })
                } else {
                    json!({ "status": "completed", "call": 12 })
                };
                Ok(
                    json!({ "id": params["id"], "command": "cad.processing.run", "phase": "done", "answer": answer }),
                )
            }
            _ => Err(HeadlessError::new("unknown_method", method.to_owned())),
        }
    }

    fn job_wait(&mut self, long: &agent_tools::LongCall) -> Result<Option<Value>, HeadlessError> {
        if self.later {
            return Ok(None);
        }
        loop {
            let status = self.request("job", &json!({ "id": long.job }))?;
            if let Some(answer) = agent_tools::job_answer(&status) {
                return Ok(Some(answer));
            }
        }
    }
}

#[test]
fn an_extensions_python_tool_is_planned_and_run_as_jobs() {
    // docs/adr/8057 §27: a processing run of an extension's tool computes Python in its plan and its execution: both go
    // as jobs (start, then job), the plan remembered by the guard and executed as planned; a host that answers later
    // gets "running" and remembers the plan when the job ends (finish_long).
    let input = json!({ "tool": "ext.tr.ornek.vitrin.taks", "parameters": {} });
    let mut desktop = JobDesktop {
        asked: Vec::new(),
        looks: 0,
        later: false,
    };
    let mut guard = Guard::new();
    let plan = agent_tools::call(
        &mut desktop,
        &mut guard,
        "processing.run",
        &json!({ "input": input, "mode": "plan" }),
    )
    .unwrap();
    assert_eq!(plan["planId"], "plan-taks", "{plan:#}");
    assert!(
        plan["next"]
            .as_str()
            .is_some_and(|n| n.contains("plan-taks"))
    );
    let started: Vec<&Value> = desktop
        .asked
        .iter()
        .filter(|(m, _)| m == "start")
        .map(|(_, p)| p)
        .collect();
    assert_eq!(started[0]["op"], "plan");
    assert!(
        desktop.asked.iter().all(|(m, _)| m != "call"),
        "nothing called on the host's thread"
    );
    let done = agent_tools::call(
        &mut desktop,
        &mut guard,
        "processing.run",
        &json!({ "input": input, "mode": "execute", "planId": "plan-taks" }),
    )
    .unwrap();
    assert_eq!(done["call"], 12, "{done:#}");
    let started: Vec<&Value> = desktop
        .asked
        .iter()
        .filter(|(m, _)| m == "start")
        .map(|(_, p)| p)
        .collect();
    assert_eq!(started[1]["op"], "execute");
    assert_eq!(
        started[1]["expected"],
        json!({ "plan": "plan-taks", "revision": "7" })
    );
    // A host that answers when the job ends: "running" now, the plan remembered by finish_long.
    let mut later = JobDesktop {
        asked: Vec::new(),
        looks: 0,
        later: true,
    };
    let mut guard = Guard::new();
    let running = agent_tools::call(
        &mut later,
        &mut guard,
        "processing.run",
        &json!({ "input": input, "mode": "plan" }),
    )
    .unwrap();
    assert_eq!(running["status"], "running", "{running:#}");
    let job = running["job"].as_str().unwrap().to_owned();
    let long = agent_tools::LongCall {
        job,
        command: "cad.processing.run".into(),
        input: input.clone(),
        revision: "7".into(),
        plan: true,
    };
    let answered = agent_tools::finish_long(
        &mut guard,
        &long,
        json!({ "status": "completed", "planId": "plan-sonra" }),
    )
    .unwrap();
    assert!(
        answered["next"]
            .as_str()
            .is_some_and(|n| n.contains("plan-sonra"))
    );
    // The remembered plan executes (the guard would refuse an unknown one).
    later.later = false;
    let done = agent_tools::call(
        &mut later,
        &mut guard,
        "processing.run",
        &json!({ "input": input, "mode": "execute", "planId": "plan-sonra" }),
    );
    assert!(done.is_ok(), "{done:?}");
    // A built-in tool is still called directly.
    assert!(!agent_tools::LongCall::is_long_for_test(
        "cad.processing.run",
        &json!({ "tool": "convert.toPoints" })
    ));
}

#[test]
fn a_processing_run_that_makes_a_layer_executes_as_planned() {
    // An agent's İşlemler run that makes a new layer (docs/adr/8057 §27): planned, then executed as planned. The new
    // layer's derivation says when the tool ran (to the millisecond); plan and execution compute the run at two
    // moments, and the plan's id took that moment in: every such execution was refused (`plan_changed`), whatever the
    // tool (found running Vitrin's TAKS tool from Asistan in a real window).
    let mut s = drawing();
    let mut guard = Guard::new();
    let input = area(&s, 0.0);
    let plan = tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.polygon.create", "input": input }),
    )
    .unwrap();
    let plan = plan["planId"].as_str().unwrap().to_owned();
    tool(
        &mut s,
        &mut guard,
        "cad.run",
        json!({ "command": "cad.polygon.create", "input": input, "mode": "execute", "planId": plan }),
    )
    .unwrap();
    let run =
        json!({ "tool": "annotation.edgeLengths", "parameters": { "input": { "scope": "all" } } });
    let planned = tool(
        &mut s,
        &mut guard,
        "processing.run",
        json!({ "input": run, "mode": "plan" }),
    )
    .unwrap();
    let plan = planned["planId"]
        .as_str()
        .unwrap_or_else(|| panic!("{planned:#}"))
        .to_owned();
    // The execution computes the run at another moment.
    std::thread::sleep(std::time::Duration::from_millis(15));
    let done = tool(
        &mut s,
        &mut guard,
        "processing.run",
        json!({ "input": run, "mode": "execute", "planId": plan }),
    )
    .unwrap();
    assert_eq!(done["status"], "completed", "{done:#}");
    assert_eq!(
        done["output"]["layers"],
        json!(["islem-kenar-olculeri"]),
        "the new layer: {done:#}"
    );
    assert_eq!(
        done["output"]["outputs"]["count"], 4,
        "four edges written: {done:#}"
    );
}
