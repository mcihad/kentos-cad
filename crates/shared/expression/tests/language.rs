//! The language since docs/adr/0100 §4 against fixtures/expression/v2/language.json,
//! whose values scripts/fixtures/expression_language.py computes without the
//! engine: durum … son, içinde, arasında, gibi, benzer, boş and IS NULL, `^`
//! and the new functions. Each source on each object, one object at a time
//! (the tree walker) and a table at a time (the column engine), to the bit;
//! the sources the reference refuses do not compile. The web plays the same
//! file through WASM (apps/web/src/model/expression/language.test.ts).

use kentos_expression::rows::{As, Column, EMPTY, NUMBER, RowsInput, TEXT, evaluate_rows};
use kentos_expression::{Measured, Scope, Value, compile};
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
