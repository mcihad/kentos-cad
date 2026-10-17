//! Tabloyu düzenle by its meaning over a large drawing (docs/adr/0327 §7): what a cell's edit takes in a table of
//! 1000 rows whose last column is 1000 formulas and whose last row sums them, what a style over the whole table takes,
//! and what a step that does not touch the table pays for the document's computed tables, in a drawing of 100 000
//! parcels. The shared cases are `fixtures/commands/v1/cad.tables.edit.json`.

use kentos_domain::contracts::{
    CommandResult, DocumentSnapshotV1, Entity, TableEdit, TablesEdit, TablesOperation,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, tables_edit};
use serde_json::json;

const ROWS: usize = 1000;

fn drawing(parcels: u32) -> Document {
    let parcel = |i: u32| {
        let (x, y) = (f64::from(i % 100) * 12.0, f64::from(i / 100) * 12.0);
        json!({"kind":"polygon","id":i + 2,"layerId":"a","attrs":{},
               "pts":[{"x":x,"y":y},{"x":x + 10.0,"y":y},{"x":x + 10.0,"y":y + 10.0},{"x":x,"y":y + 10.0}]})
    };
    // Rows of a count and a price, the amount a formula; the last row sums the amounts.
    let mut cells: Vec<Vec<String>> = (0..ROWS)
        .map(|i| {
            vec![
                format!("Kalem {i}"),
                (i % 7 + 1).to_string(),
                "12.5".to_owned(),
                String::new(),
            ]
        })
        .collect();
    cells.push(vec![
        "Toplam".to_owned(),
        String::new(),
        String::new(),
        String::new(),
    ]);
    let mut formulas: Vec<serde_json::Value> = (0..ROWS)
        .map(|i| json!({"row": i, "col": 3, "formula": format!("=B{}*C{}", i + 1, i + 1)}))
        .collect();
    formulas.push(json!({"row": ROWS, "col": 3, "formula": format!("=TOPLA(D1:D{ROWS})")}));
    let table = json!({"kind":"table","id":1,"layerId":"t","attrs":{},"p":{"x":0,"y":-10},"rotation":0,
        "height":0.25,"rows":vec![0.5; ROWS + 1],"columns":[3.0,1.5,1.5,2.0],"cells":cells,"formulas":formulas});
    let mut entities = vec![table];
    entities.extend((0..parcels).map(parcel));
    let layer = |id: &str| {
        json!({"id":id,"name":id,"type":"layer","visible":true,"locked":false,"expanded":true,
               "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]})
    };
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Tablolar","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[layer("a"), layer("t")],"activeLayer":"a","entities":entities,
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn edit(doc: &mut Document, edits: Vec<TableEdit>) {
    let uid = doc.uid(Slot(1)).expect("known").to_string();
    let input = TablesEdit {
        operation: TablesOperation::Edit { uid, edits },
        expected_revision: None,
    };
    let result = tables_edit::execute(&mut ExecutionContext::new(doc), input);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
}

fn total(doc: &Document) -> String {
    let Some(Entity::Table(t)) = doc.get(Slot(1)) else {
        panic!("a table")
    };
    t.cells[ROWS][3].clone()
}

fn set(row: usize, col: usize, words: &str) -> TableEdit {
    TableEdit::SetCell {
        row: row as u32,
        col: col as u32,
        words: words.to_owned(),
    }
}

#[test]
fn a_count_changed_works_the_sum_out_again() {
    let mut doc = drawing(10);
    edit(&mut doc, vec![set(0, 1, "1")]);
    let before: f64 = total(&doc).parse().expect("a number");
    edit(&mut doc, vec![set(0, 1, "11")]);
    let after: f64 = total(&doc).parse().expect("a number");
    assert_eq!(after - before, 125.0);
}

/// `cargo test --release -p kentos-native-application --test all tables_cmd::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let mut doc = drawing(100_000);
    let t = Instant::now();
    edit(&mut doc, vec![set(0, 1, "9")]);
    let cell = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    edit(
        &mut doc,
        vec![TableEdit::Style {
            row: 0,
            col: 0,
            rows: ROWS as u32 + 1,
            cols: 4,
            align: None,
            bold: Some(true),
            color: None,
            fill: Some("#eef2f7".to_owned()),
            decimals: Some(2),
        }],
    );
    let style = t.elapsed().as_secs_f64() * 1e3;
    // A step on a parcel: the document looks at its computed tables before it is recorded.
    let uid = doc.uid(Slot(50_000)).expect("known").to_string();
    let input = kentos_domain::contracts::EntitiesDelete {
        uids: vec![uid],
        expected_revision: None,
    };
    let t = Instant::now();
    let result = kentos_native_application::delete::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    );
    let step = t.elapsed().as_secs_f64() * 1e3;
    assert!(matches!(result, CommandResult::Completed { .. }));
    println!(
        "100 000 parselli çizimde 1001 satırlık, 1001 formüllü tablo: hücre {cell:.1} ms, bütün tabloya biçim {style:.1} ms; \
         tabloya dokunmayan silme {step:.2} ms"
    );
}
