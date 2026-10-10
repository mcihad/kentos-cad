//! The language since docs/adr/0100 §4 against fixtures/expression/v2/language.json,
//! whose values scripts/fixtures/expression_language.py computes without the
//! engine: durum … son, içinde, arasında, gibi, benzer, boş and IS NULL, `^`
//! and the new functions. Each source on each object, one object at a time
//! (the tree walker) and a table at a time (the column engine), to the bit;
//! the sources the reference refuses do not compile. The web plays the same
//! file through WASM (apps/web/src/model/expression/language.test.ts).

use kentos_expression::exec::{Slot, Source};
use kentos_expression::host::objects_source;
use kentos_expression::rows::{As, Column, EMPTY, NUMBER, RowsInput, TEXT, evaluate_rows};
use kentos_expression::world::{LayerObjects, Session, World, WorldCalls, WorldLayer};
use kentos_expression::{
    Builtin, Expr, FieldType, Geometry, Measured, Objects, Schema, Scope, Value, Variable, compile,
    compile_with, geometry,
};
use kentos_geometry_core::jsmath::js_max;
use kentos_geometry_core::store::Store;
use serde_json::{Map, Value as Json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/expression/v2/language.json"
);

/// An object of the fixture: attributes only.
struct Object<'a> {
    fields: &'a [String],
    attrs: &'a Map<String, Json>,
}

impl Scope for Object<'_> {
    fn field(&self, i: usize) -> Option<&str> {
        self.attrs.get(self.fields.get(i)?)?.as_str()
    }
    fn measured(&self) -> Measured {
        Measured::default()
    }
    fn vertices(&self) -> Option<f64> {
        None
    }
    fn kind(&self) -> &str {
        "Kapalı alan"
    }
    fn layer(&self) -> &str {
        "0"
    }
    fn label(&self) -> Option<&str> {
        None
    }
    fn index(&self) -> f64 {
        1.0
    }
    fn id(&self) -> f64 {
        1.0
    }
    fn scale(&self) -> Option<f64> {
        None
    }
}

/// A value as the fixture writes it: null, ["n", x] ("-0"), ["t", s], ["b", b].
fn encode(v: &Value) -> Json {
    match v {
        Value::Null => Json::Null,
        Value::Num(x) if *x == 0.0 && x.is_sign_negative() => serde_json::json!(["n", "-0"]),
        Value::Num(x) => serde_json::json!(["n", x]),
        Value::Text(s) => serde_json::json!(["t", s]),
        Value::Bool(b) => serde_json::json!(["b", b]),
    }
}

/// Object `i`'s value in a column, as the fixture writes it; the texts are
/// read in turn (`texts`: how many so far and where the next starts).
fn encode_column(c: &Column, i: usize, texts: &mut (usize, usize)) -> Json {
    match c.kinds[i] {
        EMPTY => Json::Null,
        NUMBER => encode(&Value::Num(c.numbers[i])),
        TEXT => {
            let len = c.text_lens[texts.0] as usize;
            texts.0 += 1;
            let rest = &c.texts[texts.1..];
            // The length is in UTF-16 units: walk the characters until it is reached.
            let (mut units, mut bytes) = (0, 0);
            for ch in rest.chars() {
                if units == len {
                    break;
                }
                units += ch.len_utf16();
                bytes += ch.len_utf8();
            }
            texts.1 += bytes;
            serde_json::json!(["t", &rest[..bytes]])
        }
        _ => serde_json::json!(["b", c.numbers[i] == 1.0]),
    }
}

/// Two encoded values the same: numbers to the bit, or within `ulp` units
/// in the last place (a power with an exponent that is not whole: libm's
/// pow is within one of the correctly rounded value).
fn same(a: &Json, b: &Json, ulp: u64) -> bool {
    match (
        a.get(1).and_then(Json::as_f64),
        b.get(1).and_then(Json::as_f64),
    ) {
        (Some(x), Some(y)) if a[0] == "n" && b[0] == "n" => {
            x.is_sign_negative() == y.is_sign_negative() && x.to_bits().abs_diff(y.to_bits()) <= ulp
        }
        _ => a == b,
    }
}

#[test]
fn the_new_words_give_the_independent_references_values() {
    let text = std::fs::read_to_string(FIXTURE).expect("the fixture is readable");
    let doc: Json = serde_json::from_str(&text).expect("the fixture is JSON");
    let objects: Vec<&Map<String, Json>> = doc["objects"]
        .as_array()
        .expect("objects")
        .iter()
        .map(|o| o.as_object().expect("an object"))
        .collect();
    let cases = doc["cases"].as_array().expect("cases");
    let mut wrong = Vec::new();
    let mut values = 0;
    for case in cases {
        let src = case["source"].as_str().expect("source");
        let compiled = compile(src);
        if case.get("compiles") == Some(&Json::Bool(false)) {
            if compiled.is_ok() {
                wrong.push(format!("{src:?}: derlendi, derlenmemeliydi"));
            }
            continue;
        }
        let e = match compiled {
            Ok(e) => e,
            Err(err) => {
                wrong.push(format!("{src:?}: {}", err.text()));
                continue;
            }
        };
        let want = case["values"].as_array().expect("values");
        let ulp = case.get("ulp").and_then(Json::as_u64).unwrap_or(0);
        // One object at a time.
        for (i, attrs) in objects.iter().enumerate() {
            let got = encode(&e.evaluate(&Object {
                fields: &e.fields,
                attrs,
            }));
            if !same(&got, &want[i], ulp) {
                wrong.push(format!("{src:?} nesne {}: {got} ≠ {}", i + 1, want[i]));
            }
            values += 1;
        }
        // A table at a time: the fields' texts per object, −1 where there is none.
        let mut texts = String::new();
        let mut lens = Vec::new();
        for attrs in &objects {
            for f in &e.fields {
                match attrs.get(f).and_then(Json::as_str) {
                    Some(s) => {
                        texts.push_str(s);
                        lens.push(s.encode_utf16().count() as i32);
                    }
                    None => lens.push(-1),
                }
            }
        }
        let input = RowsInput {
            n: objects.len(),
            texts: &texts,
            text_lens: &lens,
            numbers: &[],
            measures: &[],
            scale: f64::NAN,
        };
        let column = evaluate_rows(&e, &input, As::Value).expect("the table fits");
        let mut at = (0, 0);
        for (i, w) in want.iter().enumerate() {
            let got = encode_column(&column, i, &mut at);
            if !same(&got, w, ulp) {
                wrong.push(format!("{src:?} sütun, nesne {}: {got} ≠ {w}", i + 1));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} fark:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert!(values >= 500, "{values}");
}

// ── docs/adr/0214: fixtures/expression/v2/extras.json ──────────────────────

const EXTRAS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/expression/v2/extras.json"
);

/// A layer of the extras' world: its objects' attributes, their shapes in the store.
struct Layer<'a> {
    name: &'a str,
    ids: Vec<f64>,
    attrs: Vec<&'a Map<String, Json>>,
    store: &'a Store,
}

impl<'s, 'a: 's> Objects<'s> for Layer<'a> {
    fn len(&self) -> usize {
        self.ids.len()
    }

    fn field(&self, name: &str, _ty: FieldType, start: usize, mut slot: Slot<'_, 's>) {
        for i in 0..slot.len() {
            slot.text(i, self.attrs[start + i].get(name).and_then(Json::as_str));
        }
    }

    fn geometry(&self, what: Geometry, start: usize, mut slot: Slot<'_, 's>) {
        for i in 0..slot.len() {
            let shape = self.store.get(self.ids[start + i]).map(|it| &it.shape);
            slot.number(i, shape.and_then(|s| geometry::value(s, what)));
        }
    }

    fn builtin(&self, what: Builtin, start: usize, mut slot: Slot<'_, 's>) {
        for i in 0..slot.len() {
            match what {
                Builtin::Layer => slot.text(i, Some(self.name)),
                Builtin::Id => slot.number(i, Some(self.ids[start + i])),
                Builtin::Kind => slot.text(i, Some("Kapalı alan")),
                Builtin::Label | Builtin::Scale => slot.text(i, None),
            }
        }
    }
}

impl LayerObjects for Layer<'_> {
    fn source<'s>(&'s self, e: &'s Expr) -> Box<dyn Source<'s> + 's> {
        Box::new(objects_source(e, self as &dyn Objects<'s>))
    }
}

/// One object of a layer as the tree walker reads it, with the world.
struct OneOf<'a, 'w> {
    layer: &'a Layer<'a>,
    i: usize,
    fields: &'a [String],
    world: &'w Session<'w>,
}

impl OneOf<'_, '_> {
    fn shape_value(&self, what: Geometry) -> Option<f64> {
        let it = self.layer.store.get(self.layer.ids[self.i])?;
        geometry::value(&it.shape, what)
    }
}

impl Scope for OneOf<'_, '_> {
    fn field(&self, i: usize) -> Option<&str> {
        self.layer.attrs[self.i].get(self.fields.get(i)?)?.as_str()
    }
    fn measured(&self) -> Measured {
        Measured {
            length: self.shape_value(Geometry::Length),
            area: self.shape_value(Geometry::Area),
            anchor: self
                .shape_value(Geometry::AnchorY)
                .zip(self.shape_value(Geometry::AnchorX)),
        }
    }
    fn vertices(&self) -> Option<f64> {
        self.shape_value(Geometry::Vertices)
    }
    fn kind(&self) -> &str {
        "Kapalı alan"
    }
    fn layer(&self) -> &str {
        self.layer.name
    }
    fn label(&self) -> Option<&str> {
        None
    }
    fn index(&self) -> f64 {
        (self.i + 1) as f64
    }
    fn id(&self) -> f64 {
        self.layer.ids[self.i]
    }
    fn scale(&self) -> Option<f64> {
        None
    }
    fn geometry(&self, what: Geometry) -> Option<f64> {
        self.shape_value(what)
    }
    fn world(&self) -> Option<&dyn WorldCalls> {
        Some(self.world)
    }
}

/// Two encoded values the same; numbers within 1e-9 relative when `near`.
fn same_near(a: &Json, b: &Json, near: bool) -> bool {
    match (
        a.get(1).and_then(Json::as_f64),
        b.get(1).and_then(Json::as_f64),
    ) {
        (Some(x), Some(y)) if near && a[0] == "n" && b[0] == "n" => {
            (x - y).abs() <= 1e-9 * js_max(js_max(x.abs(), y.abs()), 1.0)
        }
        _ => same(a, b, 0),
    }
}

#[test]
fn the_additions_give_the_independent_references_values() {
    let text = std::fs::read_to_string(EXTRAS).expect("the fixture is readable");
    let doc: Json = serde_json::from_str(&text).expect("the fixture is JSON");
    let variables = doc["variables"]
        .as_array()
        .expect("variables")
        .iter()
        .map(|v| Variable {
            name: v["name"].as_str().expect("a name").to_owned(),
            value: match &v["value"] {
                Json::String(s) => Value::text(s.clone()),
                Json::Bool(b) => Value::Bool(*b),
                Json::Number(n) => Value::Num(n.as_f64().unwrap_or(f64::NAN)),
                _ => Value::Null,
            },
            description: String::new(),
        })
        .collect();
    let schema = Schema {
        fields: Vec::new(),
        variables,
        world: true,
    };
    let layers = doc["layers"].as_array().expect("layers");
    let all: Vec<&Json> = layers
        .iter()
        .flat_map(|l| l["objects"].as_array().expect("objects"))
        .collect();
    let mut store = Store::new();
    store
        .put_json(&serde_json::to_string(&all).expect("JSON"))
        .expect("the store reads the objects");
    let mine: Vec<Layer> = layers
        .iter()
        .map(|l| {
            let objects = l["objects"].as_array().expect("objects");
            Layer {
                name: l["name"].as_str().expect("a name"),
                ids: objects
                    .iter()
                    .map(|o| o["id"].as_f64().expect("an id"))
                    .collect(),
                attrs: objects
                    .iter()
                    .map(|o| o["attrs"].as_object().expect("attributes"))
                    .collect(),
                store: &store,
            }
        })
        .collect();
    let world = World {
        layers: mine
            .iter()
            .map(|l| WorldLayer {
                name: l.name.to_owned(),
                ids: l.ids.clone(),
                objects: l,
            })
            .collect(),
        store: Some(&store),
    };
    let evaluated = layers
        .iter()
        .position(|l| l["id"] == doc["evaluated"])
        .expect("the evaluated layer");
    let parcels = &mine[evaluated];
    let mut wrong = Vec::new();
    let mut values = 0;
    for case in doc["cases"].as_array().expect("cases") {
        let src = case["source"].as_str().expect("source");
        let compiled = compile_with(src, &schema);
        if case.get("compiles") == Some(&Json::Bool(false)) {
            if compiled.is_ok() {
                wrong.push(format!("{src:?}: derlendi, derlenmemeliydi"));
            }
            continue;
        }
        let e = match compiled {
            Ok(e) => e,
            Err(err) => {
                wrong.push(format!("{src:?}: {}", err.text()));
                continue;
            }
        };
        let unknown: Vec<&str> = case
            .get("unknown")
            .and_then(Json::as_array)
            .map(|u| u.iter().filter_map(Json::as_str).collect())
            .unwrap_or_default();
        if e.unknown_variables != unknown {
            wrong.push(format!("{src:?}: bilinmeyenler {:?}", e.unknown_variables));
        }
        let want = case["values"].as_array().expect("values");
        let near = case.get("near") == Some(&Json::Bool(true));
        let session = Session::new(&e, &world);
        for (i, w) in want.iter().enumerate() {
            let got = encode(&e.evaluate(&OneOf {
                layer: parcels,
                i,
                fields: &e.fields,
                world: &session,
            }));
            if !same_near(&got, w, near) {
                wrong.push(format!("{src:?} nesne {}: {got} ≠ {w}", i + 1));
            }
            values += 1;
        }
        let column = e.evaluate_objects_in(parcels as &dyn Objects<'_>, As::Value, Some(&world));
        let mut at = (0, 0);
        for (i, w) in want.iter().enumerate() {
            let got = encode_column(&column, i, &mut at);
            if !same_near(&got, w, near) {
                wrong.push(format!("{src:?} sütun, nesne {}: {got} ≠ {w}", i + 1));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} fark:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert!(values >= 900, "{values}");
}
