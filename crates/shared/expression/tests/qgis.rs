//! docs/adr/0600's functions against fixtures/expression/v2/qgis.json, whose values QGIS 4.2's own expression engine
//! gave (scripts/fixtures/expression_qgis_cases.py), or the ADR's rule where KentOS means otherwise. Each source on the
//! fixture's object one object at a time (the tree walker) and as a table (the column engine); arrays and maps compare
//! as JSON (a map without order).

use kentos_expression::compound::{is_compound, shown};
use kentos_expression::rows::{As, Column, EMPTY, NUMBER, RowsInput, TEXT, evaluate_rows};
use kentos_expression::{Measured, Scope, Value, compile};
use serde_json::{Map, Value as Json, json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/expression/v2/qgis.json"
);

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

fn text_value(s: &str) -> Json {
    if is_compound(s) {
        json!([
            "j",
            serde_json::from_str::<Json>(shown(s)).expect("the language's JSON")
        ])
    } else {
        json!(["t", s])
    }
}

fn encode(v: &Value) -> Json {
    match v {
        Value::Null => Json::Null,
        Value::Num(x) => json!(["n", x]),
        Value::Text(s) => text_value(s),
        Value::Bool(b) => json!(["b", b]),
    }
}

fn encode_column(c: &Column, i: usize, texts: &mut (usize, usize)) -> Json {
    match c.kinds[i] {
        EMPTY => Json::Null,
        NUMBER => json!(["n", c.numbers[i]]),
        TEXT => {
            let len = c.text_lens[texts.0] as usize;
            texts.0 += 1;
            let rest = &c.texts[texts.1..];
            let (mut units, mut bytes) = (0, 0);
            for ch in rest.chars() {
                if units == len {
                    break;
                }
                units += ch.len_utf16();
                bytes += ch.len_utf8();
            }
            texts.1 += bytes;
            text_value(&rest[..bytes])
        }
        _ => json!(["b", c.numbers[i] == 1.0]),
    }
}

/// Numbers in a JSON array or map compare as numbers (QGIS gives 1.0 where the language gives 1). An array or a map
/// the language gives as its result shows as its JSON text (the mark is the language's own, inside an expression).
fn same(a: &Json, b: &Json) -> bool {
    if let (Some("t"), Some("j"), Some(text)) = (
        a.get(0).and_then(Json::as_str),
        b.get(0).and_then(Json::as_str),
        a.get(1).and_then(Json::as_str),
    ) {
        return serde_json::from_str::<Json>(text)
            .is_ok_and(|parsed| same(&json!(["j", parsed]), b));
    }
    match (a, b) {
        (Json::Number(x), Json::Number(y)) => x.as_f64() == y.as_f64(),
        (Json::Array(x), Json::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Json::Object(x), Json::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

#[test]
fn the_new_functions_give_qgiss_values() {
    let doc: Json =
        serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("readable")).expect("JSON");
    assert_eq!(doc["format"], "kentos.expression-qgis");
    let attrs = doc["object"].as_object().expect("the object");
    let mut wrong = Vec::new();
    let cases = doc["cases"].as_array().expect("cases");
    for case in cases {
        let src = case["source"].as_str().expect("source");
        let want = &case["value"];
        let e = match compile(src) {
            Ok(e) => e,
            Err(err) => {
                wrong.push(format!("{src}: derlenmedi: {}", err.text()));
                continue;
            }
        };
        let got = encode(&e.evaluate(&Object {
            fields: &e.fields,
            attrs,
        }));
        if !same(&got, want) {
            wrong.push(format!("{src}: {got} ≠ {want}"));
        }
        let mut texts = String::new();
        let mut lens = Vec::new();
        for f in &e.fields {
            match attrs.get(f).and_then(Json::as_str) {
                Some(s) => {
                    texts.push_str(s);
                    lens.push(s.encode_utf16().count() as i32);
                }
                None => lens.push(-1),
            }
        }
        let input = RowsInput {
            n: 1,
            texts: &texts,
            text_lens: &lens,
            numbers: &[],
            measures: &[],
            scale: f64::NAN,
        };
        let column = evaluate_rows(&e, &input, As::Value).expect("the table fits");
        let got = encode_column(&column, 0, &mut (0, 0));
        if !same(&got, want) {
            wrong.push(format!("{src} (sütun): {got} ≠ {want}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} fark:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    assert!(cases.len() >= 150, "{}", cases.len());
}

/// An unseeded draw is not folded or shared: two calls differ, the same call differs between objects, and every
/// value is in its range (docs/adr/0600 §3).
#[test]
fn unseeded_draws_are_their_own_per_call_and_object() {
    let e =
        compile("array(rastgele(1, 1000000), rastgele(1, 1000000), rastgele_ondalık(), uuid())")
            .expect("compiles");
    struct At(f64);
    impl Scope for At {
        fn field(&self, _: usize) -> Option<&str> {
            None
        }
        fn measured(&self) -> Measured {
            Measured::default()
        }
        fn vertices(&self) -> Option<f64> {
            None
        }
        fn kind(&self) -> &str {
            ""
        }
        fn layer(&self) -> &str {
            "0"
        }
        fn label(&self) -> Option<&str> {
            None
        }
        fn index(&self) -> f64 {
            self.0
        }
        fn id(&self) -> f64 {
            self.0
        }
        fn scale(&self) -> Option<f64> {
            None
        }
    }
    let draws: Vec<Json> = (1..=4)
        .map(|i| match encode(&e.evaluate(&At(f64::from(i)))) {
            Json::Array(a) if a[0] == "t" => {
                serde_json::from_str(a[1].as_str().expect("text")).expect("JSON")
            }
            other => panic!("{other}"),
        })
        .collect();
    for d in &draws {
        let a = d[0].as_f64().expect("a number");
        let b = d[1].as_f64().expect("a number");
        let f = d[2].as_f64().expect("a number");
        assert!((1.0..=1e6).contains(&a) && a.fract() == 0.0, "{d}");
        assert_ne!(a, b, "two calls draw apart: {d}");
        assert!((0.0..1.0).contains(&f), "{d}");
        let u = d[3].as_str().expect("a uuid");
        assert_eq!(u.len(), 36, "{u}");
        assert_eq!(&u[14..15], "4", "{u}");
    }
    assert_ne!(draws[0][0], draws[1][0], "objects draw apart");
    assert_ne!(draws[0][3], draws[1][3], "uuids differ");
}
