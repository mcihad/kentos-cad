//! The symbol designer's model as `fixtures/style/v1/designer.json` holds it
//! (recorded from the web's `ui/style/designerModel.ts`, which checks the
//! same file in `designerFixture.test.ts`): layer types and names, new
//! layers, summaries, form patches, the layer list's edits, drafts, titles,
//! the preview's scale and the window's words. Numbers are compared as they
//! are written: a whole number the web writes as `3` must not come out `3.0`.

use std::path::PathBuf;

use kentos_native_style::designer::{
    self, ANCHORS, CAPS, Choice, FONTS, LayerPath, OPEN_SHAPES, PLACEMENTS, POSITIONS, RINGS,
    SHAPES, UNITS, WAVES, WEIGHTS, add_layer, add_parent, apply_patch, can_move, can_remove,
    count_suffix, default_for, duplicate_layer, geometries, geometry_key, kind_title, label,
    layer_at, layer_types, move_layer, new_draft, new_layer, path_of, put_layer, remove_layer,
    saved_as, set_enabled, summary, texts, title, uid, zoom, zoom_text, zoomed,
};
use kentos_native_style::renderer::GeometryClass;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/style/v1/designer.json");
    let text = std::fs::read_to_string(&path).expect("designer.json");
    serde_json::from_str(&text).expect("designer.json is JSON")
}

fn s(v: &Value) -> &str {
    v.as_str().expect("a string")
}

fn choices(list: &[Choice]) -> Value {
    Value::Array(
        list.iter()
            .map(|(v, l)| json!({ "value": v, "label": l }))
            .collect(),
    )
}

#[test]
fn names_types_and_options() {
    let f = fixture();
    assert_eq!(
        (s(&f["format"]), f["version"].as_u64()),
        ("kentos.style-designer", Some(1))
    );
    for (t, name) in f["labels"].as_object().expect("labels") {
        assert_eq!(label(t), s(name), "{t}");
    }
    for (kind, list) in f["layerTypes"].as_object().expect("types") {
        let want: Vec<&str> = list.as_array().expect("a list").iter().map(s).collect();
        assert_eq!(layer_types(kind), want.as_slice(), "{kind}");
        assert_eq!(kind_title(kind), s(&f["kindTitles"][kind]));
        let samples: Vec<Value> = geometries(kind)
            .iter()
            .map(|(g, l)| json!({ "value": geometry_key(*g), "label": l }))
            .collect();
        assert_eq!(Value::Array(samples), f["geometries"][kind], "{kind}");
    }
    let o = &f["options"];
    assert_eq!(choices(&UNITS), o["units"]);
    assert_eq!(choices(&SHAPES), o["shapes"]);
    assert_eq!(json!(OPEN_SHAPES), o["openShapes"]);
    assert_eq!(choices(&PLACEMENTS), o["placements"]);
    assert_eq!(choices(&ANCHORS), o["anchors"]);
    assert_eq!(choices(&FONTS), o["fonts"]);
    assert_eq!(choices(&WEIGHTS), o["weights"]);
    assert_eq!(choices(&CAPS), o["caps"]);
    assert_eq!(choices(&RINGS), o["rings"]);
    assert_eq!(choices(&WAVES), o["waves"]);
    assert_eq!(choices(&POSITIONS), o["positions"]);
}

#[test]
fn new_layers() {
    let f = fixture();
    for c in f["newLayers"].as_array().expect("a list") {
        let (t, id, context) = (s(&c["type"]), s(&c["id"]), s(&c["context"]));
        assert_eq!(new_layer(t, id, context), c["layer"], "{t} in {context}");
    }
}

#[test]
fn summaries() {
    let f = fixture();
    for c in f["summaries"].as_array().expect("a list") {
        assert_eq!(summary(&c["layer"]), s(&c["summary"]), "{}", c["layer"]);
    }
    for c in f["countSuffixes"].as_array().expect("a list") {
        let n = c[0].as_f64().expect("a count");
        assert_eq!(count_suffix(n), s(&c[1]), "{n}");
    }
}

#[test]
fn patches() {
    let f = fixture();
    for c in f["patches"].as_array().expect("a list") {
        let mut patch: Vec<(String, Option<Value>)> = c["patch"]
            .as_object()
            .expect("a patch")
            .iter()
            .map(|(k, v)| (k.clone(), Some(v.clone())))
            .collect();
        for k in c["unset"].as_array().expect("a list") {
            patch.push((s(k).to_owned(), None));
        }
        assert_eq!(
            apply_patch(&c["layer"], &patch),
            c["result"],
            "{} on {}",
            c["patch"],
            c["layer"]
        );
    }
    for c in f["uids"].as_array().expect("a list") {
        let list: Vec<Value> = c["ids"]
            .as_array()
            .expect("ids")
            .iter()
            .map(|id| json!({ "id": id, "type": "simpleFill", "color": "ink" }))
            .collect();
        assert_eq!(uid(&list), s(&c["uid"]), "{}", c["ids"]);
    }
}

#[test]
fn edits() {
    let f = fixture();
    for e in f["edits"].as_array().expect("a list") {
        let name = s(&e["name"]);
        let symbol = &e["symbol"];
        let selected = LayerPath::from_value(&e["selected"]).expect("a path");
        assert_eq!(
            layer_at(symbol, selected).cloned().unwrap_or(Value::Null),
            e["layer"],
            "{name}: the layer"
        );
        assert_eq!(
            add_parent(symbol, selected)
                .map(|i| json!(i))
                .unwrap_or(Value::Null),
            e["addParent"],
            "{name}: where Katman ekle may add"
        );
        assert_eq!(
            [
                can_move(symbol, selected, -1),
                can_move(symbol, selected, 1),
                can_remove(symbol, selected)
            ],
            [
                e["canMoveUp"] == true,
                e["canMoveDown"] == true,
                e["canRemove"] == true
            ],
            "{name}: what may be done"
        );
        let op = &e["op"];
        let mut out = symbol.clone();
        let chosen = match s(&op["op"]) {
            "add" => Some(add_layer(
                &mut out,
                s(&op["type"]),
                op["parent"].as_u64().map(|i| i as usize),
            )),
            "move" => move_layer(&mut out, selected, op["delta"].as_i64().expect("a delta")),
            "duplicate" => duplicate_layer(&mut out, selected),
            "remove" => remove_layer(&mut out, selected),
            "enable" => {
                let at = LayerPath::from_value(&op["at"]).expect("a path");
                set_enabled(&mut out, at, op["on"] == true);
                Some(selected)
            }
            "put" => {
                let at = LayerPath::from_value(&op["at"]).expect("a path");
                put_layer(&mut out, at, op["layer"].clone());
                Some(selected)
            }
            other => panic!("{name}: unknown edit {other}"),
        };
        let got = chosen.map_or(
            Value::Null,
            |p| json!({ "symbol": out, "selected": p.to_value() }),
        );
        assert_eq!(got, e["result"], "{name}");
    }
}

#[test]
fn drafts_titles_and_saved_names() {
    let f = fixture();
    for d in f["drafts"].as_array().expect("a list") {
        let path: Option<Vec<String>> = d["path"]
            .as_array()
            .map(|p| p.iter().map(|x| s(x).to_owned()).collect());
        let draft = new_draft(s(&d["kind"]), path.as_deref());
        assert_eq!(
            json!({ "name": draft.name, "path": draft.path, "symbol": draft.symbol }),
            d["draft"]
        );
    }
    for (class, key) in [
        (GeometryClass::Fill, "fill"),
        (GeometryClass::Line, "line"),
        (GeometryClass::Marker, "marker"),
    ] {
        assert_eq!(default_for(class), f["defaults"][key], "{key}");
    }
    for t in f["titles"].as_array().expect("a list") {
        assert_eq!(
            title(s(&t["kind"]), t["inline"].as_str(), t["dirty"] == true),
            s(&t["title"])
        );
    }
    for c in f["saved"].as_array().expect("a list") {
        let path: Vec<String> = c["path"]
            .as_array()
            .expect("a path")
            .iter()
            .map(|x| s(x).to_owned())
            .collect();
        let (name, path) = saved_as(s(&c["name"]), &path);
        assert_eq!(json!({ "name": name, "path": path }), c["as"]);
    }
    for c in f["paths"].as_array().expect("a list") {
        assert_eq!(json!(path_of(s(&c["text"]))), c["path"], "{}", c["text"]);
    }
}

#[test]
fn zoom_and_words() {
    let f = fixture();
    let z = &f["zoom"];
    assert_eq!(
        [zoom::MIN, zoom::MAX, zoom::STEP, zoom::START, zoom::REAL],
        [
            z["min"].as_f64().expect("min"),
            z["max"].as_f64().expect("max"),
            z["step"].as_f64().expect("step"),
            z["start"].as_f64().expect("start"),
            z["real"].as_f64().expect("real")
        ]
    );
    for c in z["steps"].as_array().expect("a list") {
        let got = zoomed(
            c["px"].as_f64().expect("px"),
            c["factor"].as_f64().expect("factor"),
        );
        assert_eq!(got, c["result"].as_f64().expect("result"), "{c}");
    }
    for c in z["texts"].as_array().expect("a list") {
        assert_eq!(zoom_text(c["px"].as_f64().expect("px")), s(&c["text"]));
    }
    let t = &f["texts"];
    for (key, text) in [
        ("layers", texts::LAYERS),
        ("add", texts::ADD),
        ("intoSymbol", texts::INTO_SYMBOL),
        ("order", texts::ORDER),
        ("lastLayer", texts::LAST_LAYER),
        ("onePoint", texts::ONE_POINT),
        ("pickLayer", texts::PICK_LAYER),
        ("inline", texts::INLINE),
        ("cannotEdit", texts::CANNOT_EDIT),
        ("drawsWith", texts::DRAWS_WITH),
        ("steps", texts::STEPS),
        ("addStep", texts::ADD_STEP),
        ("noSteps", texts::NO_STEPS),
        ("nothingLeft", texts::NOTHING_LEFT),
        ("effects", texts::EFFECTS),
        ("effectsInside", texts::EFFECTS_INSIDE),
    ] {
        assert_eq!(s(&t[key]), text, "{key}");
    }
    assert_eq!(
        texts::into_marker(s(&t["intoMarker"]["parent"])),
        s(&t["intoMarker"]["text"])
    );
    assert_eq!(
        texts::in_marker(s(&t["inMarker"]["parent"])),
        s(&t["inMarker"]["text"])
    );
    for c in t["notSaved"].as_array().expect("a list") {
        assert_eq!(
            texts::not_saved(s(&c["first"]), c["more"].as_u64().expect("more") as usize),
            s(&c["text"])
        );
    }
    assert_eq!(texts::saved(s(&t["saved"]["name"])), s(&t["saved"]["text"]));
    assert_eq!(
        texts::into_generator(s(&t["intoGenerator"]["parent"])),
        s(&t["intoGenerator"]["text"])
    );
    assert_eq!(
        texts::in_generator(s(&t["inGenerator"]["parent"])),
        s(&t["inGenerator"]["text"])
    );
    // Every string the web's texts hold is checked above.
    let strings = t
        .as_object()
        .expect("texts")
        .values()
        .filter(|v| v.is_string())
        .count();
    assert_eq!(strings, 16);
    let _ = designer::KINDS;
}

/// A parent's child types and a geometry generator's steps (docs/adr/0226 §1).
#[test]
fn child_types_and_steps() {
    use kentos_native_style::designer::child_types;
    use kentos_native_style::designer::steps::{
        BUFFER_SIDES, STEP_KINDS, VERTEX_CHOICES, add_step, class_of, move_step, new_step,
        put_step, remove_step, set_generator_kind, step_label, step_output, step_summary,
    };
    let f = fixture();
    for c in f["childTypes"].as_array().expect("a list") {
        let want: Vec<&str> = c["types"].as_array().expect("types").iter().map(s).collect();
        assert_eq!(child_types(&c["parent"]), want, "{}", c["parent"]);
    }
    let st = &f["steps"];
    for (k, name) in st["labels"].as_object().expect("labels") {
        assert_eq!(step_label(k), Some(s(name)), "{k}");
    }
    assert_eq!(json!(STEP_KINDS), st["kinds"]);
    assert_eq!(choices(&VERTEX_CHOICES), st["vertexChoices"]);
    assert_eq!(choices(&BUFFER_SIDES), st["bufferSides"]);
    for (kind, class) in st["classOf"].as_object().expect("classes") {
        assert_eq!(class_of(kind), s(class), "{kind}");
    }
    for c in st["newSteps"].as_array().expect("a list") {
        assert_eq!(new_step(s(&c["kind"])), c["step"], "{}", c["kind"]);
    }
    for c in st["lines"].as_array().expect("a list") {
        assert_eq!(step_summary(&c["step"], s(&c["unit"])), s(&c["line"]), "{}", c["step"]);
    }
    for c in st["outputs"].as_array().expect("a list") {
        let steps = c["steps"].as_array().expect("steps");
        let input = match s(&c["input"]) {
            "point" => "point",
            "line" => "line",
            _ => "area",
        };
        assert_eq!(
            step_output(steps, input).map_or(Value::Null, |o| json!(o)),
            c["output"],
            "{} on {input}",
            c["steps"]
        );
    }
    for e in st["edits"].as_array().expect("a list") {
        let (name, layer, op) = (s(&e["name"]), &e["layer"], &e["op"]);
        let at = || op["at"].as_u64().expect("at") as usize;
        let got = match s(&op["op"]) {
            "add" => add_step(layer, s(&op["kind"])),
            "move" => move_step(layer, at(), op["delta"].as_i64().expect("delta")).unwrap_or(Value::Null),
            "remove" => remove_step(layer, at()),
            "put" => put_step(layer, at(), op["step"].clone()),
            "kind" => set_generator_kind(layer, s(&op["kind"])),
            other => panic!("{name}: unknown edit {other}"),
        };
        assert_eq!(got, e["result"], "{name}");
    }
}

/// Draw effects: names, new ones, switching, replacing, which layers take them, lines (docs/adr/0226 §2).
#[test]
fn effects() {
    use kentos_native_style::designer::effects::{
        EFFECT_TYPES, effect_label, effects_summary, new_effect, put_effect, set_effect,
        takes_effects,
    };
    let f = fixture();
    let e = &f["effects"];
    for (k, name) in e["labels"].as_object().expect("labels") {
        assert_eq!(effect_label(k), s(name), "{k}");
    }
    assert_eq!(json!(EFFECT_TYPES), e["types"]);
    for c in e["newEffects"].as_array().expect("a list") {
        assert_eq!(new_effect(s(&c["type"])), c["effect"], "{}", c["type"]);
    }
    for c in e["takes"].as_array().expect("a list") {
        let p = LayerPath::from_value(&c["path"]).expect("a path");
        assert_eq!(takes_effects(p), c["takes"] == true, "{}", c["path"]);
    }
    for c in e["edits"].as_array().expect("a list") {
        let (name, layer, op) = (s(&c["name"]), &c["layer"], &c["op"]);
        let got = match s(&op["op"]) {
            "set" => set_effect(layer, s(&op["type"]), op["on"] == true),
            "put" => put_effect(layer, op["effect"].clone()),
            other => panic!("{name}: unknown edit {other}"),
        };
        assert_eq!(got, c["result"], "{name}");
    }
    for c in e["lines"].as_array().expect("a list") {
        assert_eq!(effects_summary(&c["layer"]), s(&c["line"]), "{}", c["layer"]);
    }
}
