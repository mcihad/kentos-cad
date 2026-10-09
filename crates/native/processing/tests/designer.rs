//! The model designer's rules as the web's (fixtures/processing/v1/
//! designer.json, the format in fixtures/processing/README.md; the web plays
//! it in apps/web/src/ui/processing/model/designerPlan.test.ts): its words,
//! the edits step by step with the model and its problems after each, what
//! the designer reads from a model (status, order, edges and their labels,
//! box texts, sources, the wire menu, extent), where a new box goes, the
//! title, the saved name, undo joining and the diagram's geometry.

use std::collections::BTreeMap;

use kentos_processing::designer::{self as plan, Bounds, Pt, StatusKind, StepMeta, View, texts};
use kentos_processing::model::{Model, ValueSource, can_feed, check_model, order_steps};
use kentos_processing::model_edit::{self as edit, INPUT_TYPES, NodeRef};
use kentos_processing::web_param::param_from_json;
use kentos_processing::{Defaults, OutputDef, OutputKind, Target, Tool};
use serde::Deserialize;
use serde_json::{Value, json};

const TEXT: &str = include_str!("../../../../fixtures/processing/v1/designer.json");

fn fixture() -> Value {
    serde_json::from_str(TEXT).expect("designer.json reads")
}

/// JSON compared as JavaScript compares it: one kind of number.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

fn assert_same(got: &Value, want: &Value, what: &str) {
    assert!(same(got, want), "{what}\n got: {got}\nwant: {want}");
}

/// The file's tools as a registry would hold them.
fn tools(f: &Value) -> BTreeMap<String, Tool> {
    f["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| {
            let id = t["id"].as_str().expect("an id").to_owned();
            let tool = Tool {
                id: id.clone(),
                label: t["label"].as_str().expect("a label").to_owned(),
                category: "points".into(),
                description: String::new(),
                help: None,
                keywords: Vec::new(),
                aliases: Vec::new(),
                icon: t["icon"].as_str().map(str::to_owned),
                parameters: t["parameters"]
                    .as_array()
                    .expect("parameters")
                    .iter()
                    .map(|p| param_from_json(p).expect("a parameter"))
                    .collect(),
                outputs: t["outputs"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|o| {
                        let kind = match o["type"].as_str() {
                            Some("features") => OutputKind::Features,
                            Some("number") => OutputKind::Number,
                            _ => OutputKind::Text,
                        };
                        OutputDef::new(
                            o["name"].as_str().expect("a name"),
                            o["label"].as_str().expect("a label"),
                            kind,
                        )
                    })
                    .collect(),
                targets: vec![Target::Client],
                validate: None,
                preview: None,
                run: None,
            };
            (id, tool)
        })
        .collect()
}

fn defaults(f: &Value) -> Defaults {
    let d = &f["defaults"];
    Defaults {
        length_decimals: d["lengthDecimals"].as_u64().expect("decimals") as u32,
        area_decimals: d["areaDecimals"].as_u64().expect("decimals") as u32,
        angle_unit: serde_json::from_value(d["angleUnit"].clone()).expect("an angle unit"),
        plot_scale: d["plotScale"].as_f64().expect("a scale"),
        drawing_font: "barlow",
        active_layer: d["activeLayer"].as_str().expect("a layer").to_owned(),
        // The kind's default when the case names none (docs/adr/0205 §1).
        measure_height_mm: d["measureHeightMm"].as_f64().unwrap_or(2.0),
        // The designer's cases have no networks (docs/adr/0209 §9).
        networks: Vec::new(),
    }
}

fn node(v: &Value) -> Option<NodeRef> {
    match v["kind"].as_str()? {
        "input" => Some(NodeRef::Input(v["name"].as_str()?.to_owned())),
        _ => Some(NodeRef::Step(v["id"].as_str()?.to_owned())),
    }
}

fn node_json(n: &NodeRef) -> Value {
    match n {
        NodeRef::Input(name) => json!({ "kind": "input", "name": name }),
        NodeRef::Step(id) => json!({ "kind": "step", "id": id }),
    }
}

fn pt(v: &Value) -> Pt {
    Pt::new(v["x"].as_f64().expect("x"), v["y"].as_f64().expect("y"))
}

fn pt_json(p: Pt) -> Value {
    json!({ "x": p.x, "y": p.y })
}

fn at(v: &Value) -> Option<(f64, f64)> {
    Some((v.get("x")?.as_f64()?, v.get("y")?.as_f64()?))
}

fn issues_json(model: &Model, lookup: &dyn Fn(&str) -> Option<Tool>) -> Value {
    Value::Array(
        check_model(model, lookup)
            .iter()
            .map(|i| i.to_json())
            .collect(),
    )
}

/// One of the file's edits on the model; what it returned.
fn apply(
    model: &mut Model,
    op: &Value,
    lookup: &dyn Fn(&str) -> Option<Tool>,
    d: &Defaults,
) -> Value {
    let text = |key: &str| op[key].as_str().expect(key);
    match text("op") {
        "addInput" => json!(edit::add_input(
            model,
            text("type"),
            text("label"),
            op.get("at").and_then(at)
        )),
        "addStep" => json!(edit::add_step(
            model,
            text("tool"),
            lookup,
            op.get("at").and_then(at),
            op.get("from").and_then(node).as_ref()
        )),
        "setSource" => {
            edit::set_source(
                model,
                text("step"),
                text("param"),
                ValueSource::from_json(&op["src"]),
            );
            Value::Null
        }
        "removeInput" => {
            edit::remove_input(model, text("name"));
            Value::Null
        }
        "removeStep" => {
            edit::remove_step(model, text("id"));
            Value::Null
        }
        "inputFromParam" => {
            let param = model
                .steps
                .iter()
                .find(|s| s.id == text("step"))
                .and_then(|s| lookup(&s.tool))
                .and_then(|t| t.parameters.into_iter().find(|p| p.name == text("param")));
            match param {
                Some(p) => json!(edit::input_from_param(model, text("step"), &p, d)),
                None => Value::Null,
            }
        }
        "addOutput" => json!(edit::add_output(
            model,
            text("step"),
            text("output"),
            lookup
        )),
        "caption" => {
            edit::set_caption(model, text("step"), text("caption"));
            Value::Null
        }
        "autoLayout" => {
            edit::auto_layout(model);
            Value::Null
        }
        other => panic!("an unknown edit: {other}"),
    }
}

/// A text of the file (`{ sample, text }` or plain).
fn file_text<'a>(f: &'a Value, path: &str) -> (&'a Value, &'a str) {
    let mut v = &f["texts"];
    for key in path.split('.') {
        v = &v[key];
    }
    match v.get("text") {
        Some(text) => (&v["sample"], text.as_str().expect("a text")),
        None => (&Value::Null, v.as_str().unwrap_or_else(|| panic!("{path}"))),
    }
}

/// A text made from the file's sample.
type Make = fn(&Value) -> String;

fn s(v: &Value, i: usize) -> &str {
    v[i].as_str().expect("a text sample")
}

fn n(v: &Value, i: usize) -> usize {
    v[i].as_u64().expect("a number sample") as usize
}

#[test]
fn the_designers_words_input_kinds_diagram_numbers_and_undo_are_the_webs() {
    use texts::{TITLE, not_found, title_of};
    use texts::{canvas as words, connect};
    use texts::{footer, inspector, palette, pick, remove, save, status, unsaved};
    let f = fixture();
    assert_eq!(
        (f["format"].as_str(), f["version"].as_u64()),
        (Some("kentos.modelDesigner"), Some(1))
    );
    let plain: &[(&str, &str)] = &[
        ("title", TITLE),
        ("footer.layout", footer::LAYOUT),
        ("footer.layoutTip", footer::LAYOUT_TIP),
        ("footer.close", footer::CLOSE),
        ("footer.saveRun", footer::SAVE_RUN),
        ("footer.save", footer::SAVE),
        ("status.empty", status::EMPTY),
        ("save.unnamed", save::UNNAMED),
        ("unsaved.after", unsaved::AFTER),
        ("unsaved.verb", unsaved::VERB),
        ("remove.title", remove::TITLE),
        ("remove.action", remove::ACTION),
        ("pick.point", pick::POINT),
        ("pick.unnamed", pick::UNNAMED),
        ("connect.replaces", connect::REPLACES),
        ("canvas.label", words::LABEL),
        ("canvas.zoomOut", words::ZOOM_OUT),
        ("canvas.zoomIn", words::ZOOM_IN),
        ("canvas.fit", words::FIT),
        ("canvas.port", words::PORT),
        ("canvas.noLinks", words::NO_LINKS),
        ("palette.label", palette::LABEL),
        ("palette.inputs", palette::INPUTS),
        ("palette.tools", palette::TOOLS),
        ("palette.search", palette::SEARCH),
        ("palette.empty", palette::EMPTY),
        ("palette.toolNote", palette::TOOL_NOTE),
        ("palette.tip", palette::TIP),
        ("inspector.label", inspector::LABEL),
        ("inspector.model.kind", inspector::model::KIND),
        ("inspector.model.lead", inspector::model::LEAD),
        ("inspector.model.name", inspector::model::NAME),
        ("inspector.model.nameAria", inspector::model::NAME_ARIA),
        ("inspector.model.category", inspector::model::CATEGORY),
        ("inspector.model.description", inspector::model::DESCRIPTION),
        (
            "inspector.model.descriptionAria",
            inspector::model::DESCRIPTION_ARIA,
        ),
        (
            "inspector.model.descriptionHint",
            inspector::model::DESCRIPTION_HINT,
        ),
        ("inspector.model.outputs", inspector::model::OUTPUTS),
        ("inspector.model.noStep", inspector::model::NO_STEP),
        ("inspector.model.addOutput", inspector::model::ADD_OUTPUT),
        ("inspector.model.noOutput", inspector::model::NO_OUTPUT),
        ("inspector.model.remove", inspector::model::REMOVE),
        ("inspector.problems.title", inspector::problems::TITLE),
        ("inspector.problems.ready", inspector::problems::READY),
        ("inspector.problems.start", inspector::problems::START),
        ("inspector.input.lead", inspector::input::LEAD),
        ("inspector.input.label", inspector::input::LABEL),
        ("inspector.input.labelAria", inspector::input::LABEL_ARIA),
        ("inspector.input.description", inspector::input::DESCRIPTION),
        (
            "inspector.input.descriptionAria",
            inspector::input::DESCRIPTION_ARIA,
        ),
        (
            "inspector.input.descriptionHint",
            inspector::input::DESCRIPTION_HINT,
        ),
        ("inspector.input.optional", inspector::input::OPTIONAL),
        ("inspector.input.default", inspector::input::DEFAULT),
        ("inspector.input.scopeAria", inspector::input::SCOPE_ARIA),
        (
            "inspector.input.scopes.selection",
            inspector::input::SCOPES[0].1,
        ),
        (
            "inspector.input.scopes.visible",
            inspector::input::SCOPES[1].1,
        ),
        ("inspector.input.scopes.all", inspector::input::SCOPES[2].1),
        ("inspector.input.kinds", inspector::input::KINDS),
        ("inspector.input.kindsNote", inspector::input::KINDS_NOTE),
        ("inspector.input.min", inspector::input::MIN),
        ("inspector.input.max", inspector::input::MAX),
        ("inspector.input.none", inspector::input::NONE),
        ("inspector.input.integer", inspector::input::INTEGER),
        (
            "inspector.input.textDefault",
            inspector::input::TEXT_DEFAULT,
        ),
        ("inspector.input.allowEmpty", inspector::input::ALLOW_EMPTY),
        ("inspector.input.newLayer", inspector::input::NEW_LAYER),
        (
            "inspector.input.newLayerAria",
            inspector::input::NEW_LAYER_ARIA,
        ),
        (
            "inspector.input.newLayerNote",
            inspector::input::NEW_LAYER_NOTE,
        ),
        ("inspector.input.pointNote", inspector::input::POINT_NOTE),
        ("inspector.input.users", inspector::input::USERS),
        ("inspector.input.noUsers", inspector::input::NO_USERS),
        ("inspector.input.remove", inspector::input::REMOVE),
        ("inspector.step.unknown", inspector::step::UNKNOWN),
        ("inspector.step.caption", inspector::step::CAPTION),
        ("inspector.step.captionAria", inspector::step::CAPTION_ARIA),
        ("inspector.step.captionNote", inspector::step::CAPTION_NOTE),
        ("inspector.step.params", inspector::step::PARAMS),
        ("inspector.step.optional", inspector::step::OPTIONAL),
        ("inspector.step.toolDefault", inspector::step::TOOL_DEFAULT),
        ("inspector.step.fixed", inspector::step::FIXED),
        ("inspector.step.modelInput", inspector::step::MODEL_INPUT),
        ("inspector.step.asInput", inspector::step::AS_INPUT),
        ("inspector.step.asInputNote", inspector::step::AS_INPUT_NOTE),
        ("inspector.step.remove", inspector::step::REMOVE),
    ];
    for &(path, ours) in plain {
        assert_eq!(file_text(&f, path).1, ours, "{path}");
    }
    let made: &[(&str, Make)] = &[
        ("titleOf", |v| {
            title_of(s(v, 0), v[1].as_bool().expect("dirty"))
        }),
        ("notFound", |v| not_found(s(v, 0))),
        ("status.problems", |v| status::problems(n(v, 0), s(v, 1))),
        ("status.ready", |v| status::ready(n(v, 0), n(v, 1))),
        ("save.saved", |v| save::saved(s(v, 0), n(v, 1))),
        ("remove.question", |v| remove::question(s(v, 0))),
        ("remove.done", |v| remove::done(s(v, 0))),
        ("pick.command", |v| pick::command(s(v, 0))),
        ("connect.header", |v| connect::header(s(v, 0))),
        ("connect.fromStep", |v| connect::from_step(s(v, 0), s(v, 1))),
        ("connect.none", |v| connect::none(s(v, 0))),
        ("canvas.inputMeta", |v| {
            words::input_meta(s(v, 0), v[1].as_bool().expect("optional"))
        }),
        ("canvas.links", |v| words::links(n(v, 0))),
        ("inspector.model.output", |v| {
            inspector::model::output(s(v, 0), s(v, 1))
        }),
        ("inspector.model.removeOutput", |v| {
            inspector::model::remove_output(s(v, 0))
        }),
        ("inspector.problems.titleCount", |v| {
            inspector::problems::title_count(n(v, 0))
        }),
        ("inspector.problems.ofStep", |v| {
            inspector::problems::of_step(s(v, 0), s(v, 1))
        }),
        ("inspector.input.kind", |v| inspector::input::kind(s(v, 0))),
        ("inspector.input.variable", |v| {
            inspector::input::variable(s(v, 0))
        }),
        ("inspector.step.unknownNote", |v| {
            inspector::step::unknown_note(s(v, 0))
        }),
        ("inspector.step.advanced", |v| {
            inspector::step::advanced(n(v, 0))
        }),
        ("inspector.step.sourceAria", |v| {
            inspector::step::source_aria(s(v, 0))
        }),
    ];
    for &(path, make) in made {
        let (sample, text) = file_text(&f, path);
        assert_eq!(make(sample), text, "{path}");
    }
    let (sample, text) = file_text(&f, "inspector.step.inputSource");
    assert_eq!(inspector::step::input_source(s(sample, 0)), text);
    let (sample, text) = file_text(&f, "inspector.step.outputSource");
    assert_eq!(
        inspector::step::output_source(s(sample, 0), s(sample, 1)),
        text
    );

    let kinds: Vec<Value> = INPUT_TYPES
        .iter()
        .map(|t| json!({ "type": t.type_name, "label": t.label, "icon": t.icon, "description": t.description }))
        .collect();
    assert_same(&Value::Array(kinds), &f["inputTypes"], "inputTypes");
    let c = &f["canvas"];
    for (key, ours) in [
        ("inputW", plan::canvas::INPUT_W),
        ("inputH", plan::canvas::INPUT_H),
        ("stepW", plan::canvas::STEP_W),
        ("stepH", plan::canvas::STEP_H),
        ("grid", plan::canvas::GRID),
        ("zoomMin", plan::canvas::ZOOM_MIN),
        ("zoomMax", plan::canvas::ZOOM_MAX),
        ("zoomStep", plan::canvas::ZOOM_STEP),
        ("wheel", plan::canvas::WHEEL),
        ("fitPad", plan::canvas::FIT_PAD),
        ("drag", plan::canvas::DRAG),
        ("paletteDrag", plan::canvas::PALETTE_DRAG),
        ("column", plan::canvas::COLUMN),
        ("row", plan::canvas::ROW),
        ("bend", plan::canvas::BEND),
        ("labelGap", plan::canvas::LABEL_GAP),
        ("labelRise", plan::canvas::LABEL_RISE),
        ("labelRow", plan::canvas::LABEL_ROW),
    ] {
        assert_eq!(c[key].as_f64(), Some(ours), "canvas.{key}");
    }
    assert_same(
        &json!({ "x": plan::canvas::EMPTY.x, "y": plan::canvas::EMPTY.y, "k": plan::canvas::EMPTY.k }),
        &c["empty"],
        "canvas.empty",
    );
    assert_eq!(
        c.as_object().map(|o| o.len()),
        Some(19),
        "every number is checked"
    );
    assert_eq!(
        f["history"]["depth"].as_u64(),
        Some(plan::HISTORY_DEPTH as u64)
    );
    assert_eq!(f["history"]["coalesceMs"].as_u64(), Some(plan::COALESCE_MS));
}

#[test]
fn what_feeds_what_the_names_from_labels_and_new_and_copied_models_are_the_webs() {
    let f = fixture();
    let rows = f["canFeed"].as_array().expect("canFeed");
    let types: Vec<&str> = rows
        .iter()
        .map(|r| r["from"].as_str().expect("a type"))
        .collect();
    for r in rows {
        let from = r["from"].as_str().expect("a type");
        let feeds: Vec<&str> = types
            .iter()
            .copied()
            .filter(|t| can_feed(from, t))
            .collect();
        let want: Vec<&str> = r["feeds"]
            .as_array()
            .expect("feeds")
            .iter()
            .map(|v| v.as_str().expect("a type"))
            .collect();
        assert_eq!(feeds, want, "{from}");
    }
    for c in f["slugs"].as_array().expect("slugs") {
        assert_eq!(
            edit::slug(c["label"].as_str().expect("a label")),
            c["slug"].as_str().expect("a slug"),
            "{c}"
        );
    }
    let mut fresh = edit::new_model("m-new".into()).to_json();
    fresh.as_object_mut().expect("an object").remove("id");
    assert_same(&fresh, &f["newModel"], "newModel");
    assert_eq!(
        edit::copy_label(f["copyLabel"]["label"].as_str().expect("a label")),
        f["copyLabel"]["copy"].as_str().expect("the copy's")
    );
}

#[test]
fn a_model_is_edited_step_by_step_and_checked_after_each_as_on_the_web() {
    let f = fixture();
    let tools = tools(&f);
    let lookup = |id: &str| tools.get(id).cloned();
    let d = defaults(&f);
    for seq in f["sequences"].as_array().expect("sequences") {
        let mut model = edit::new_model("m-fixture".into());
        for (i, step) in seq["steps"].as_array().expect("steps").iter().enumerate() {
            let op = &step["op"];
            let where_ = format!("{}, {}. {}", seq["title"], i + 1, op["op"]);
            let result = apply(&mut model, op, &lookup, &d);
            assert_same(&result, &step["result"], &format!("{where_}: result"));
            assert_same(
                &model.to_json(),
                &step["model"],
                &format!("{where_}: model"),
            );
            assert_same(
                &issues_json(&model, &lookup),
                &step["issues"],
                &format!("{where_}: issues"),
            );
        }
    }
}

#[derive(Deserialize)]
struct Final {
    title: String,
    model: Model,
    status: Value,
    order: Value,
    edges: Value,
    #[serde(rename = "edgeLabels")]
    edge_labels: Value,
    #[serde(rename = "stepMeta")]
    step_meta: Value,
    sources: Vec<Value>,
    wires: Vec<Value>,
    bounds: Value,
}

#[derive(Deserialize)]
struct Finals {
    models: Vec<Final>,
}

#[test]
fn a_model_reads_in_the_designer_as_on_the_web() {
    let f = fixture();
    let tools = tools(&f);
    let lookup = |id: &str| tools.get(id).cloned();
    // Read from the text, so each step's values keep the order they are written in.
    let finals: Finals = serde_json::from_str(TEXT).expect("the final models read");
    assert!(finals.models.len() >= 5);
    for m in &finals.models {
        let model = &m.model;
        let title = &m.title;
        let problems = check_model(model, &lookup);
        let (kind, text) = plan::designer_status(&problems, model.steps.len(), model.inputs.len());
        let kind = match kind {
            StatusKind::Warn => "warn",
            StatusKind::Ok => "ok",
        };
        assert_same(
            &json!({ "kind": kind, "text": text }),
            &m.status,
            &format!("{title}: status"),
        );
        let order = match order_steps(model) {
            Ok(order) => json!(order),
            Err(error) => json!({ "error": error }),
        };
        assert_same(&order, &m.order, &format!("{title}: order"));
        let edges: Vec<Value> = edit::edges_of(model)
            .iter()
            .map(|e| json!({ "from": node_json(&e.from), "to": e.to, "params": e.params }))
            .collect();
        assert_same(&Value::Array(edges), &m.edges, &format!("{title}: edges"));
        let labels: Vec<Value> = plan::edge_labels(model, &lookup)
            .iter()
            .map(|l| {
                json!({ "from": node_json(&l.from), "to": l.to, "text": l.text, "title": l.title, "at": pt_json(l.at) })
            })
            .collect();
        assert_same(
            &Value::Array(labels),
            &m.edge_labels,
            &format!("{title}: edge labels"),
        );
        let mut first: BTreeMap<&str, &str> = BTreeMap::new();
        for p in &problems {
            if let Some(step) = &p.step {
                first.entry(step.as_str()).or_insert(p.message.as_str());
            }
        }
        let metas: Vec<Value> = model
            .steps
            .iter()
            .map(|s| {
                let meta = match plan::step_meta(
                    s,
                    lookup(&s.tool).as_ref(),
                    first.get(s.id.as_str()).copied(),
                ) {
                    StepMeta::Warn(w) => json!({ "warn": w }),
                    StepMeta::Text(t) => json!({ "text": t }),
                };
                json!({ "step": s.id, "meta": meta })
            })
            .collect();
        assert_same(
            &Value::Array(metas),
            &m.step_meta,
            &format!("{title}: step meta"),
        );
        for c in &m.sources {
            let step_id = c["step"].as_str().expect("a step");
            let param = c["param"].as_str().expect("a parameter");
            let step = model
                .steps
                .iter()
                .find(|s| s.id == step_id)
                .expect("the step");
            let p = lookup(&step.tool)
                .and_then(|t| t.parameters.into_iter().find(|x| x.name == param))
                .expect("the parameter");
            let options: Vec<Value> = edit::sources_for(model, step_id, &p, &lookup)
                .iter()
                .map(|o| json!({ "src": o.src.to_json(), "label": o.label, "group": o.group }))
                .collect();
            assert_same(
                &Value::Array(options),
                &c["options"],
                &format!("{title}: {step_id}.{param}"),
            );
            assert_eq!(
                plan::source_text(model, step.source(param), &lookup),
                c["text"].as_str().expect("a text"),
                "{title}: {step_id}.{param}"
            );
        }
        for w in &m.wires {
            let from = node(&w["from"]).expect("a source");
            let to = w["to"].as_str().expect("a step");
            let got = match plan::connect_choices(model, &from, to, &lookup) {
                None => Value::Null,
                Some(menu) => json!({
                    "header": menu.header,
                    "items": menu.items.iter().map(|c| {
                        let mut item = json!({ "param": c.param, "label": c.label, "checked": c.checked, "src": c.src.to_json() });
                        if let Some(detail) = &c.detail {
                            item["detail"] = json!(detail);
                        }
                        item
                    }).collect::<Vec<_>>(),
                    "none": menu.none,
                }),
            };
            assert_same(
                &got,
                &w["choices"],
                &format!("{title}: {} → {to}", w["from"]),
            );
        }
        let bounds = match plan::boxes_bounds(model) {
            None => Value::Null,
            Some(Bounds { x, y, w, h }) => json!({ "x": x, "y": y, "w": w, "h": h }),
        };
        assert_same(&bounds, &m.bounds, &format!("{title}: bounds"));
    }
}

#[test]
fn new_boxes_the_title_saved_names_and_undo_joining_are_the_webs() {
    let f = fixture();
    let finals: Finals = serde_json::from_str(TEXT).expect("the final models read");
    let model = &finals.models[4].model;
    let empty = edit::new_model("m-empty".into());
    for c in f["spots"].as_array().expect("spots") {
        let selected = node(&c["selected"]);
        let on = if c["empty"].as_bool() == Some(true) {
            &empty
        } else {
            model
        };
        assert_same(
            &pt_json(plan::spot_near(on, selected.as_ref())),
            &c["spot"],
            &format!("{c}"),
        );
    }
    for c in f["titles"].as_array().expect("titles") {
        assert_eq!(
            texts::title_of(
                c["label"].as_str().expect("a label"),
                c["dirty"].as_bool().expect("dirty")
            ),
            c["title"].as_str().expect("a title")
        );
    }
    for c in f["savedLabels"].as_array().expect("saved labels") {
        assert_eq!(
            plan::saved_label(c["label"].as_str().expect("a label")),
            c["saved"].as_str().expect("the saved name")
        );
    }
    let mut last: Option<(String, i64)> = None;
    for c in f["joins"].as_array().expect("joins") {
        let key = c["key"].as_str();
        let at = c["at"].as_i64().expect("a time");
        assert_eq!(
            plan::joins(key, last.as_ref().map(|(k, a)| (k.as_str(), *a)), at),
            c["joins"].as_bool().expect("joins"),
            "{c}"
        );
        last = key.map(|k| (k.to_owned(), at));
    }
}

fn view(v: &Value) -> View {
    View {
        x: v["x"].as_f64().expect("x"),
        y: v["y"].as_f64().expect("y"),
        k: v["k"].as_f64().expect("k"),
    }
}

fn view_json(v: View) -> Value {
    json!({ "x": v.x, "y": v.y, "k": v.k })
}

#[test]
fn the_diagrams_geometry_is_the_webs() {
    let f = fixture();
    let g = &f["geometry"];
    for c in g["curves"].as_array().expect("curves") {
        let (c1, c2) = plan::curve(pt(&c["a"]), pt(&c["b"]));
        assert_same(
            &json!({ "c1": pt_json(c1), "c2": pt_json(c2) }),
            &c["curve"],
            &format!("{c}"),
        );
    }
    let p = &g["ports"];
    assert_same(
        &pt_json(plan::input_port(pt(&p["input"]["at"]))),
        &p["input"]["port"],
        "input port",
    );
    assert_same(
        &pt_json(plan::step_port(pt(&p["step"]["at"]))),
        &p["step"]["port"],
        "step port",
    );
    assert_same(
        &pt_json(plan::step_entry(pt(&p["step"]["at"]))),
        &p["step"]["entry"],
        "step entry",
    );
    for c in g["fits"].as_array().expect("fits") {
        let bounds = c["bounds"].as_object().map(|b| Bounds {
            x: b["x"].as_f64().expect("x"),
            y: b["y"].as_f64().expect("y"),
            w: b["w"].as_f64().expect("w"),
            h: b["h"].as_f64().expect("h"),
        });
        let got = plan::fit_view(
            bounds,
            c["width"].as_f64().expect("a width"),
            c["height"].as_f64().expect("a height"),
        );
        assert_same(&view_json(got), &c["view"], &format!("{c}"));
    }
    for c in g["zooms"].as_array().expect("zooms") {
        let floor = c["floor"].as_f64().unwrap_or(plan::canvas::ZOOM_MIN);
        let got = plan::zoom_at(
            view(&c["view"]),
            pt(&c["at"]),
            c["factor"].as_f64().expect("a factor"),
            floor,
        );
        assert_same(&view_json(got), &c["result"], &format!("{c}"));
    }
    for c in g["floors"].as_array().expect("floors") {
        assert_eq!(
            plan::zoom_floor(c["fitted"].as_f64().expect("fitted")),
            c["floor"].as_f64().expect("a floor"),
            "{c}"
        );
    }
    for c in g["snaps"].as_array().expect("snaps") {
        assert_eq!(
            plan::snap(c["value"].as_f64().expect("a value")),
            c["snapped"].as_f64().expect("snapped"),
            "{c}"
        );
    }
}
