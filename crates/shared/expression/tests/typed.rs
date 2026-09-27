//! A host with typed user-defined fields and shapes (docs/adr/0100 §3): the
//! schema resolves names at compile time, number fields come as numbers,
//! geometry values are read from the shapes, each group once per object,
//! and the geometry values agree with an independent reference
//! (fixtures/expression/v2/geometry.json, scripts/fixtures/expression_geometry.py).

use std::cell::Cell;

use kentos_expression::exec::Slot;
use kentos_expression::geometry::{self, Shapes};
use kentos_expression::rows::{As, Column, EMPTY, NUMBER, TEXT};
use kentos_expression::{
    Builtin, FieldDef, FieldRef, FieldSource, FieldType, Geometry, Measured, Objects, Schema,
    Scope, Value, compile, compile_with,
};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::{Entity, Shape};

/// A layer's values, owned; `Layer` reads them.
struct Data {
    kat: Vec<f64>,
    ad: Vec<Option<String>>,
    satildi: Vec<Option<bool>>,
    tarih: Vec<Option<String>>,
    nitelik: Vec<Option<String>>,
    shapes: Vec<Shape>,
}

/// Objects with typed columns, a text attribute and shapes, as a layer with
/// a user-defined schema would hold them. `asked` counts the shapes read.
struct Layer<'d> {
    data: &'d Data,
    shapes: Shapes<'d, Box<dyn Fn(usize) -> Option<&'d Shape> + 'd>>,
}

impl<'d> Layer<'d> {
    fn new(data: &'d Data, asked: &'d Cell<usize>) -> Layer<'d> {
        let shapes: Box<dyn Fn(usize) -> Option<&'d Shape> + 'd> = Box::new(move |i| {
            asked.set(asked.get() + 1);
            data.shapes.get(i)
        });
        Layer {
            data,
            shapes: Shapes::new(shapes),
        }
    }
}

impl<'d> Objects<'d> for Layer<'d> {
    fn len(&self) -> usize {
        self.data.kat.len()
    }

    fn field(&self, name: &str, ty: FieldType, start: usize, mut slot: Slot<'_, 'd>) {
        let d: &'d Data = self.data;
        for i in 0..slot.len() {
            let k = start + i;
            match (name, ty) {
                ("Kat", FieldType::Number) => {
                    let x = d.kat[k];
                    slot.number(i, (!x.is_nan()).then_some(x));
                }
                ("Satıldı", FieldType::Bool) => slot.truth(i, d.satildi[k]),
                ("Ad", FieldType::Text) => slot.text(i, d.ad[k].as_deref()),
                ("Tarih", FieldType::Date) => slot.text(i, d.tarih[k].as_deref()),
                ("Nitelik", FieldType::Text) => slot.text(i, d.nitelik[k].as_deref()),
                _ => slot.text(i, None),
            }
        }
    }

    fn geometry(&self, what: Geometry, start: usize, slot: Slot<'_, 'd>) {
        self.shapes.fill(what, start, slot);
    }

    fn builtin(&self, what: Builtin, start: usize, mut slot: Slot<'_, 'd>) {
        for i in 0..slot.len() {
            match what {
                Builtin::Id => slot.number(i, Some((start + i + 1) as f64)),
                Builtin::Kind => slot.text(i, Some("Kapalı alan")),
                Builtin::Layer => slot.text(i, Some("Parsel")),
                Builtin::Label | Builtin::Scale => slot.number(i, None),
            }
        }
    }
}

fn user(name: &str, ty: FieldType) -> FieldDef {
    FieldDef {
        name: name.into(),
        ty,
        source: FieldSource::User,
        description: String::new(),
    }
}

fn schema() -> Schema {
    Schema {
        fields: vec![
            user("Kat", FieldType::Number),
            user("Ad", FieldType::Text),
            user("Satıldı", FieldType::Bool),
            user("Tarih", FieldType::Date),
            FieldDef {
                name: "Nitelik".into(),
                ty: FieldType::Text,
                source: FieldSource::Attribute,
                description: String::new(),
            },
        ],
    }
}

/// A square parcel of `side` metres at (x, y).
fn square(x: f64, y: f64, side: f64) -> Shape {
    let json = format!(
        r#"{{"kind":"polygon","pts":[{{"x":{x},"y":{y}}},{{"x":{},"y":{y}}},{{"x":{},"y":{}}},{{"x":{x},"y":{}}}]}}"#,
        x + side,
        x + side,
        y + side,
        y + side
    );
    shape(&json)
}

fn shape(json: &str) -> Shape {
    let j = Json::parse(json).unwrap_or_else(|e| panic!("{json}: {e}"));
    Entity::from_json(&j)
        .unwrap_or_else(|e| panic!("{json}: {e}"))
        .shape
}

/// 300 objects (a batch of 256 and a part of one), some values missing.
fn data() -> Data {
    let n = 300;
    Data {
        kat: (0..n)
            .map(|i| if i % 7 == 3 { f64::NAN } else { (i % 9) as f64 })
            .collect(),
        ad: (0..n)
            .map(|i| (i % 5 != 0).then(|| format!("Ev{i}")))
            .collect(),
        satildi: (0..n)
            .map(|i| (i % 11 != 0).then_some(i % 2 == 0))
            .collect(),
        tarih: (0..n)
            .map(|i| (i % 13 != 0).then(|| format!("2026-{:02}-{:02}", 1 + i % 12, 1 + i % 28)))
            .collect(),
        nitelik: (0..n)
            .map(|i| Some(if i % 3 == 0 { "Tarla" } else { "Arsa" }.to_string()))
            .collect(),
        shapes: (0..n)
            .map(|i| square(487000.0 + i as f64 * 25.0, 4420000.0, 10.0 + (i % 4) as f64))
            .collect(),
    }
}

/// The value of object `i` in a column.
fn value_at(c: &Column, i: usize) -> Value<'_> {
    let texts_before = c.kinds[..i].iter().filter(|&&k| k == TEXT).count();
    match c.kinds[i] {
        EMPTY => Value::Null,
        NUMBER => Value::Num(c.numbers[i]),
        TEXT => {
            let mut chars = c.texts.chars();
            let skip: usize = c.text_lens[..texts_before]
                .iter()
                .map(|&n| n as usize)
                .sum();
            let mut units = 0;
            let mut start = 0;
            for ch in chars.by_ref() {
                if units >= skip {
                    break;
                }
                units += ch.len_utf16();
                start += ch.len_utf8();
            }
            let want = c.text_lens[texts_before] as usize;
            let (mut u, mut end) = (0, start);
            for ch in c.texts[start..].chars() {
                if u >= want {
                    break;
                }
                u += ch.len_utf16();
                end += ch.len_utf8();
            }
            Value::text(&c.texts[start..end])
        }
        _ => Value::Bool(c.numbers[i] == 1.0),
    }
}

#[test]
fn names_resolve_against_the_schema() {
    let e = compile_with("Kat * 2 + [Tarih] || Ad || Nitelik || Başka", &schema())
        .unwrap_or_else(|e| panic!("{}", e.text()));
    assert_eq!(e.fields, ["Kat", "Tarih", "Ad", "Nitelik", "Başka"]);
    let typed = |ty| FieldRef {
        ty,
        source: FieldSource::User,
    };
    let text = FieldRef {
        ty: FieldType::Text,
        source: FieldSource::Attribute,
    };
    assert_eq!(
        e.types,
        [
            typed(FieldType::Number),
            typed(FieldType::Date),
            typed(FieldType::Text),
            text,
            text
        ]
    );
    // Without a schema every name is a text attribute, as before.
    let e = compile("Kat * 2").unwrap_or_else(|e| panic!("{}", e.text()));
    assert_eq!(e.types, [text]);
    let e = compile("$merkez_y + $genişlik").unwrap_or_else(|e| panic!("{}", e.text()));
    assert!(e.needs.centroid && e.needs.bounds && !e.needs.measured);
}

#[test]
fn typed_fields_are_read_as_their_type() {
    let d = data();
    let asked = Cell::new(0);
    let layer = Layer::new(&d, &asked);
    let s = schema();
    let run = |src: &str, want: As| {
        let e = compile_with(src, &s).unwrap_or_else(|e| panic!("{src}: {}", e.text()));
        e.evaluate_objects(&layer, want)
    };
    let n = d.kat.len();
    // Numbers straight from the number field.
    let c = run("Kat * 2 + 1", As::Value);
    for i in 0..n {
        let want = if d.kat[i].is_nan() {
            Value::Null
        } else {
            Value::Num(d.kat[i] * 2.0 + 1.0)
        };
        assert_eq!(value_at(&c, i), want, "Kat * 2 + 1, nesne {i}");
    }
    // True/false and numbers in a condition; an empty value is not true.
    let c = run("Satıldı ve Kat > 3", As::Value);
    for i in 0..n {
        let want = d.satildi[i] == Some(true) && d.kat[i] > 3.0;
        assert_eq!(value_at(&c, i), Value::Bool(want), "nesne {i}");
    }
    // Text of a text field joined with a number field's text.
    let c = run("Ad || '-' || Kat", As::Value);
    for i in 0..n {
        let kat = if d.kat[i].is_nan() {
            String::new()
        } else {
            format!("{}", d.kat[i])
        };
        let want = format!("{}-{kat}", d.ad[i].as_deref().unwrap_or(""));
        assert_eq!(value_at(&c, i), Value::text(want), "nesne {i}");
    }
    // A date is ISO text: it compares as text, which is its order.
    let c = run("Tarih >= '2026-06-15'", As::Value);
    for i in 0..n {
        let want = d.tarih[i].as_deref().is_some_and(|t| t >= "2026-06-15");
        assert_eq!(value_at(&c, i), Value::Bool(want), "nesne {i}");
    }
    // A text attribute the schema lists reads as before.
    let c = run("Nitelik = 'Arsa'", As::Value);
    for i in 0..n {
        assert_eq!(value_at(&c, i), Value::Bool(i % 3 != 0), "nesne {i}");
    }
    // The number field and the same digits as text give the same answers.
    for src in [
        "Kat * 2 + 1",
        "Kat > 3",
        "yuvarla(Kat / 7, 2)",
        "metin(Kat, 1)",
    ] {
        let typed = run(src, As::Value);
        let e = compile(src).unwrap_or_else(|e| panic!("{}", e.text()));
        assert_eq!(e.types[0].ty, FieldType::Text);
        let as_text = TextKat(&d);
        let c = e.evaluate_objects(&as_text, As::Value);
        for i in 0..n {
            assert_eq!(value_at(&c, i), value_at(&typed, i), "{src}, nesne {i}");
        }
    }
}

/// The same layer with Kat as a text attribute (its digits), no schema.
struct TextKat<'d>(&'d Data);

impl<'d> Objects<'d> for TextKat<'d> {
    fn len(&self) -> usize {
        self.0.kat.len()
    }

    fn field(&self, _name: &str, _ty: FieldType, start: usize, mut slot: Slot<'_, 'd>) {
        for i in 0..slot.len() {
            let x = self.0.kat[start + i];
            slot.text(i, (!x.is_nan()).then(|| DIGITS[x as usize]));
        }
    }

    fn geometry(&self, _what: Geometry, _start: usize, mut slot: Slot<'_, 'd>) {
        for i in 0..slot.len() {
            slot.number(i, None);
        }
    }

    fn builtin(&self, _what: Builtin, _start: usize, mut slot: Slot<'_, 'd>) {
        for i in 0..slot.len() {
            slot.number(i, None);
        }
    }
}

const DIGITS: [&str; 9] = ["0", "1", "2", "3", "4", "5", "6", "7", "8"];

#[test]
fn geometry_groups_are_computed_once_per_object() {
    let d = data();
    let asked = Cell::new(0);
    let layer = Layer::new(&d, &asked);
    let e = compile("$min_y + $max_y + $genişlik + $yükseklik + $merkez_y + $merkez_x + $alan")
        .unwrap_or_else(|e| panic!("{}", e.text()));
    let c = e.evaluate_objects(&layer, As::Number);
    let n = d.shapes.len();
    // The box once, the centroid once, the area once: three reads of each shape.
    assert_eq!(asked.get(), 3 * n);
    for i in 0..n {
        let side = 10.0 + (i % 4) as f64;
        let (x, y) = (487000.0 + i as f64 * 25.0, 4420000.0);
        let want = x + (x + side) + side + side + (x + side / 2.0) + (y + side / 2.0) + side * side;
        let Value::Num(got) = value_at(&c, i) else {
            panic!("nesne {i}")
        };
        assert!((got - want).abs() <= 1e-6, "nesne {i}: {got} ≠ {want}");
    }
}

/// One object through its scope, with its shape: the new geometry values too.
struct One<'s>(&'s Shape);

impl Scope for One<'_> {
    fn field(&self, _i: usize) -> Option<&str> {
        None
    }
    fn measured(&self) -> Measured {
        Measured {
            length: geometry::value(self.0, Geometry::Length),
            area: geometry::value(self.0, Geometry::Area),
            anchor: geometry::value(self.0, Geometry::AnchorY)
                .zip(geometry::value(self.0, Geometry::AnchorX)),
        }
    }
    fn vertices(&self) -> Option<f64> {
        geometry::vertices(self.0)
    }
    fn kind(&self) -> &str {
        ""
    }
    fn layer(&self) -> &str {
        ""
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
    fn geometry(&self, what: Geometry) -> Option<f64> {
        geometry::value(self.0, what)
    }
}

/// The geometry values against the independent reference, in a batch and one object at a time.
#[test]
fn geometry_values_match_the_reference() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/expression/v2/geometry.json"
    );
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let file = Json::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
    let Json::Arr(cases) = file.get("cases") else {
        panic!("cases")
    };
    let mut checked = 0;
    for case in cases {
        let name = match case.get("name") {
            Json::Str(s) => s.clone(),
            _ => String::new(),
        };
        let entity =
            Entity::from_json(case.get("entity")).unwrap_or_else(|e| panic!("{name}: {e}"));
        let shapes = [entity.shape];
        let d = Data {
            kat: vec![f64::NAN],
            ad: vec![None],
            satildi: vec![None],
            tarih: vec![None],
            nitelik: vec![None],
            shapes: shapes.to_vec(),
        };
        let asked = Cell::new(0);
        let layer = Layer::new(&d, &asked);
        let Json::Obj(values) = case.get("values") else {
            panic!("{name}: values")
        };
        for (var, want) in values {
            let e = compile(&format!("${var}"))
                .unwrap_or_else(|e| panic!("{name} ${var}: {}", e.text()));
            let batch = value_at(&e.evaluate_objects(&layer, As::Value), 0).into_owned();
            let one = e.evaluate(&One(&shapes[0])).into_owned();
            for (path, got) in [("parti", batch), ("tek", one)] {
                match (want, got) {
                    (Json::Null, Value::Null) => {}
                    (Json::Num(w), Value::Num(g)) => {
                        // 0.1 µm: far below a survey's precision, far above float noise at
                        // projected coordinates (the double's step at 4.4e6 is 9.3e-10).
                        assert!((g - w).abs() <= 1e-7, "{name} ${var} [{path}]: {g} ≠ {w}");
                    }
                    (w, g) => panic!("{name} ${var} [{path}]: {g:?} ≠ {w:?}"),
                }
                checked += 1;
            }
        }
    }
    assert!(checked >= 200, "{checked}");
}
