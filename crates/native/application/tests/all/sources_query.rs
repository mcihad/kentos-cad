// The source's part of a condition keeps every row the condition keeps (docs/adr/0581 §3): a table of awkward values
// (texts in number columns, integers past 2⁵³, empty texts, Turkish letters) written straight into SQLite, and random
// conditions asked of it twice: of the file, with what can be sent sent (`cad.sources.query`'s path and table), and of
// the same table imported into the drawing, where nothing is sent. The two answers are the same rows; when the plan
// calls the SQL exact, the file hands over only the rows that match.

use std::collections::BTreeSet;

use kentos_commands::CallOp;
use kentos_domain::Document;
use kentos_geopackage::{Column, ColumnKind, DataType, GeoPackage, NewRow, TableDef, Value as V};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn run(doc: &mut Document, command: &str, input: Value) -> Value {
    registry::run(
        &mut ExecutionContext::new(doc),
        command,
        CallOp::Execute,
        input,
    )
    .expect("runs")
}

fn ok(answer: &Value) -> &Value {
    assert_eq!(answer["status"], "completed", "{answer}");
    &answer["output"]
}

fn doc() -> Document {
    let setup = json!({
        "format": "kentos.document", "version": 1, "name": "Sorgu",
        "settings": { "srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                      "plotScale": 1000, "workspace": "gis" },
        "origin": { "x": 0.0, "y": 0.0 },
        "layers": [{ "id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false, "expanded": true,
                     "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": [] }],
        "activeLayer": "cizim", "entities": [], "styles": { "items": [], "categories": [] }
    });
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).expect("setup");
    Document::from_snapshot(snapshot).expect("setup")
}

/// xorshift64*: the same rows and conditions on every machine.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn pick<'a, T>(&mut self, of: &'a [T]) -> &'a T {
        &of[(self.next() % of.len() as u64) as usize]
    }
}

fn column(name: &str, decl: &str, kind: ColumnKind) -> Column {
    Column {
        name: name.into(),
        decl: decl.into(),
        kind,
        not_null: false,
        default: None,
    }
}

const NUMBERS: [&str; 9] = [
    "-2",
    "0",
    "0.0000000001",
    "2",
    "3",
    "3.25",
    "9007199254740993",
    "'3'",
    "'2.0'",
];
const TEXTS: [&str; 7] = ["'a'", "'A'", "'ağaç'", "'ç'", "'3'", "'Z'", "''"];

fn atom(d: &mut Dice) -> String {
    match d.next() % 9 {
        0 => format!(
            "a {} {}",
            d.pick(&["<", "<=", ">", ">=", "="]),
            d.pick(&NUMBERS)
        ),
        1 => format!("{} {} b", d.pick(&NUMBERS), d.pick(&["<", ">=", "="])),
        2 => format!("s = {}", d.pick(&TEXTS)),
        3 => format!("a arasında {} ve {}", d.pick(&NUMBERS), d.pick(&NUMBERS)),
        4 => format!("s içinde ({}, {})", d.pick(&TEXTS), d.pick(&TEXTS)),
        5 => format!("b içinde ({}, {})", d.pick(&NUMBERS), d.pick(&NUMBERS)),
        6 => format!(
            "{} {}",
            d.pick(&["a", "b", "s"]),
            d.pick(&["boş", "boş değil"])
        ),
        7 => format!("s gibi '{}%'", d.pick(&["a", "ç", "3"])),
        _ => format!("değil (b > {})", d.pick(&NUMBERS)),
    }
}

fn condition(d: &mut Dice, depth: u32) -> String {
    if depth == 0 || d.next().is_multiple_of(3) {
        return atom(d);
    }
    let (x, y) = (condition(d, depth - 1), condition(d, depth - 1));
    format!("({x}) {} ({y})", d.pick(&["and", "veya", "ve", "or"]))
}

fn uids(answer: &Value) -> BTreeSet<String> {
    answer["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| r["uid"].as_str().expect("a uid").to_owned())
        .collect()
}

#[test]
fn what_a_source_is_sent_keeps_every_row_the_condition_keeps() {
    let path = std::env::temp_dir().join(format!("kentos-sorgu-ust-{}.gpkg", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut d = Dice(0x9E37_79B9_7F4A_7C15);
    {
        let mut file = GeoPackage::create(&path).expect("created");
        let def = TableDef {
            name: "t".into(),
            data_type: DataType::Attributes,
            identifier: None,
            description: None,
            srs_id: None,
            geometry: None,
            columns: vec![
                column("a", "REAL", ColumnKind::Real),
                column("b", "INTEGER", ColumnKind::Integer),
                column("s", "TEXT", ColumnKind::Text(None)),
            ],
        };
        let reals = [
            V::Null,
            V::Real(0.0),
            V::Real(-1.5),
            V::Real(2.0),
            V::Real(3.25),
            V::Real(1e-10),
            V::Real(2.000_000_01),
            V::Integer(9_007_199_254_740_993),
            V::Text("abc".into()),
            V::Text("ölçülmedi".into()),
            V::Text(String::new()),
            V::Text("2.0".into()),
        ];
        let ints = [
            V::Null,
            V::Integer(-2),
            V::Integer(0),
            V::Integer(2),
            V::Integer(3),
            V::Integer(9_007_199_254_740_993),
            V::Text("x".into()),
            V::Text(String::new()),
        ];
        let texts = [
            V::Null,
            V::Text(String::new()),
            V::Text("a".into()),
            V::Text("A".into()),
            V::Text("ağaç".into()),
            V::Text("3".into()),
            V::Text("3.0".into()),
            V::Text("ç".into()),
            V::Text("Z".into()),
            V::Real(3.0),
        ];
        let rows: Vec<NewRow> = (0..600)
            .map(|_| NewRow {
                fid: None,
                geometry: None,
                values: vec![
                    d.pick(&reals).clone(),
                    d.pick(&ints).clone(),
                    d.pick(&texts).clone(),
                ],
            })
            .collect();
        file.write_table(&def, &rows, None).expect("written");
    }
    let file = path.display().to_string();
    let mut doc = doc();
    let imported = run(&mut doc, "file.import.gpkg", json!({ "path": file }));
    let layer = ok(&imported)["tables"][0]["layer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let mut sent = 0;
    for k in 0..400 {
        let filter = condition(&mut d, 2);
        let from_file = run(
            &mut doc,
            "cad.sources.query",
            json!({ "path": file, "table": "t", "filter": filter, "limit": 10000, "count": true, "fields": ["a"] }),
        );
        let from_drawing = run(
            &mut doc,
            "cad.sources.query",
            json!({ "layer": layer, "filter": filter, "limit": 10000, "count": true, "fields": ["a"] }),
        );
        let (f, w) = (ok(&from_file), ok(&from_drawing));
        assert_eq!(uids(f), uids(w), "{k}: {filter}: {}", f["plan"]);
        if f["plan"]["pushed"].is_string() {
            sent += 1;
        }
        if f["plan"]["exact"] == true {
            assert_eq!(
                f["read"], f["total"],
                "{k}: {filter} kesin denildi: {}",
                f["plan"]
            );
        }
    }
    // Most conditions send something.
    assert!(sent > 200, "{sent} koşul kaynağa gitti");
    let _ = std::fs::remove_file(&path);
}

/// A whole answer's rows paged through: every page's `next` given back until there is none (docs/adr/0591 §3).
fn paged(doc: &mut Document, mut input: Value, limit: u64) -> Vec<Value> {
    let mut out = Vec::new();
    input["limit"] = json!(limit);
    loop {
        let answer = run(doc, "cad.sources.query", input.clone());
        let o = ok(&answer);
        out.extend(o["rows"].as_array().expect("rows").iter().cloned());
        match o["next"].as_str() {
            Some(next) => input["cursor"] = json!(next),
            None => return out,
        }
    }
}

/// A field's values in the rows' order; an empty text is no value (the order puts both last, each by its own place).
fn values(rows: &[Value], field: &str) -> Vec<Value> {
    rows.iter()
        .map(|r| match &r["attrs"][field] {
            Value::String(t) if t.is_empty() => Value::Null,
            v => v.clone(),
        })
        .collect()
}

#[test]
fn pages_of_an_order_join_to_the_whole_order_and_the_drawing_orders_the_same_values() {
    let path = std::env::temp_dir().join(format!("kentos-sorgu-sira-{}.gpkg", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut d = Dice(0x51_7C_C1_B7_27_22_0A_95);
    {
        let mut file = GeoPackage::create(&path).expect("created");
        let def = TableDef {
            name: "t".into(),
            data_type: DataType::Attributes,
            identifier: None,
            description: None,
            srs_id: None,
            geometry: None,
            columns: vec![
                column("a", "REAL", ColumnKind::Real),
                column("b", "INTEGER", ColumnKind::Integer),
                column("s", "TEXT", ColumnKind::Text(None)),
            ],
        };
        let reals = [
            V::Null,
            V::Real(-1.5),
            V::Real(0.0),
            V::Real(0.25),
            V::Real(2.0),
            V::Real(1e-10),
            V::Real(7.75),
        ];
        let ints = [
            V::Null,
            V::Integer(-3),
            V::Integer(0),
            V::Integer(1),
            V::Integer(12),
            V::Integer(100),
        ];
        let texts = [
            V::Null,
            V::Text(String::new()),
            V::Text("ağaç 2".into()),
            V::Text("Ağaç 10".into()),
            V::Text("çam".into()),
            V::Text("ılgın".into()),
            V::Text("İncir".into()),
            V::Text("z".into()),
        ];
        let rows: Vec<NewRow> = (0..2600)
            .map(|_| NewRow {
                fid: None,
                geometry: None,
                values: vec![
                    d.pick(&reals).clone(),
                    d.pick(&ints).clone(),
                    d.pick(&texts).clone(),
                ],
            })
            .collect();
        file.write_table(&def, &rows, None).expect("written");
    }
    let file = path.display().to_string();
    let mut doc = doc();
    let imported = run(&mut doc, "file.import.gpkg", json!({ "path": file }));
    let layer = ok(&imported)["tables"][0]["layer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    for (field, by) in [("a", "column"), ("b", "column"), ("s", "here")] {
        for descending in [false, true] {
            let order = json!({ "field": field, "descending": descending });
            let ask = json!({ "path": file, "table": "t", "order": order });
            let whole = run(
                &mut doc,
                "cad.sources.query",
                json!({ "path": file, "table": "t", "order": order, "limit": 10000 }),
            );
            assert_eq!(ok(&whole)["plan"]["order"], by, "{field}");
            let whole = ok(&whole)["rows"].as_array().expect("rows").clone();
            assert_eq!(whole.len(), 2600);
            for limit in [7, 333, 1000, 1001] {
                let pages = paged(&mut doc, ask.clone(), limit);
                assert_eq!(pages, whole, "{field} {descending} {limit}");
            }
            // Equal values go by key in the file, by lasting id in the drawing: the values' sequence is the same.
            let drawn = paged(&mut doc, json!({ "layer": layer, "order": order }), 500);
            assert_eq!(
                values(&drawn, field),
                values(&whole, field),
                "{field} {descending}"
            );
        }
    }
    let _ = std::fs::remove_file(&path);
}

#[test]
fn an_order_here_keeps_at_most_two_hundred_thousand_records() {
    let path = std::env::temp_dir().join(format!("kentos-sorgu-cok-{}.gpkg", std::process::id()));
    let _ = std::fs::remove_file(&path);
    {
        let mut file = GeoPackage::create(&path).expect("created");
        let def = TableDef {
            name: "t".into(),
            data_type: DataType::Attributes,
            identifier: None,
            description: None,
            srs_id: None,
            geometry: None,
            columns: vec![
                column("s", "TEXT", ColumnKind::Text(None)),
                column("n", "INTEGER", ColumnKind::Integer),
            ],
        };
        let most = kentos_commands::MOST_ORDERED_HERE as i64;
        let rows: Vec<NewRow> = (0..=most)
            .map(|k| NewRow {
                fid: None,
                geometry: None,
                values: vec![V::Text(format!("k{}", k % 977)), V::Integer(k)],
            })
            .collect();
        file.write_table(&def, &rows, None).expect("written");
    }
    let file = path.display().to_string();
    let mut doc = doc();
    let refused = run(
        &mut doc,
        "cad.sources.query",
        json!({ "path": file, "table": "t", "order": { "field": "s" }, "limit": 1 }),
    );
    assert_eq!(refused["status"], "failed", "{refused}");
    assert_eq!(refused["error"]["code"], "order_too_large");
    assert_eq!(refused["error"]["path"], "order");
    // A filter that keeps fewer, or an order the source makes, is answered.
    let fewer = run(
        &mut doc,
        "cad.sources.query",
        json!({ "path": file, "table": "t", "order": { "field": "s" }, "filter": "n < 1000", "limit": 1 }),
    );
    assert_eq!(ok(&fewer)["plan"]["order"], "here");
    let source = run(
        &mut doc,
        "cad.sources.query",
        json!({ "path": file, "table": "t", "order": { "field": "n", "descending": true }, "limit": 1 }),
    );
    assert_eq!(
        ok(&source)["rows"][0]["key"],
        json!(kentos_commands::MOST_ORDERED_HERE + 1)
    );
    let _ = std::fs::remove_file(&path);
}

/// A processing input read from its source (docs/adr/0592 §1) holds at most so many records: one more and the run is
/// refused, named at the input's parameter, before any of them goes into the run's copy.
#[test]
fn a_source_input_stops_at_its_cap() {
    use kentos_native_application::sources_query::{StageFrom, stage_at_most};
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/sources/v1/sorgu.gpkg")
        .display()
        .to_string();
    let doc = doc();
    let from = StageFrom::Table {
        address: &file,
        table: "nokta",
    };
    let at = "parameters.input";
    let refused = match stage_at_most(doc.settings(), &from, None, None, at, &|| false, 249) {
        Err(Some(e)) => *e,
        Ok(_) => panic!("250 records went in under a cap of 249"),
        Err(None) => panic!("stopped"),
    };
    assert_eq!(refused.code, "input_too_large");
    assert_eq!(refused.path.as_deref(), Some("parameters.input.table"));
    let (staging, read) = match stage_at_most(doc.settings(), &from, None, None, at, &|| false, 250)
    {
        Ok(s) => s,
        Err(e) => panic!("{e:?}"),
    };
    assert_eq!(staging.objects.len(), 250);
    assert_eq!(read.total, Some(250));
    let (filtered, _) = match stage_at_most(
        doc.settings(),
        &from,
        Some("tur = 'rögar'"),
        None,
        at,
        &|| false,
        60,
    ) {
        Ok(s) => s,
        Err(e) => panic!("{e:?}"),
    };
    assert_eq!(
        filtered.objects.len(),
        50,
        "a filter's records count, not the table's"
    );
}
