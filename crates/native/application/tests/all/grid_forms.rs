//! A grid's followers through the real write paths (docs/adr/0725 §7, §8): a dimension tied to a grid's end and middle
//! follows when the grid moves, and a column schedule's Yer follows its grid's new label in the same step.
use super::wall_contours::{completed, drawing};
use kentos_commands::*;
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, arch_edit, dimensions_create, schedules_create, transform};
use serde_json::json;

fn table_values(doc: &kentos_domain::Document, slot: Slot) -> String {
    let Entity::Table(t) = doc.get(slot).unwrap() else {
        panic!()
    };
    serde_json::to_string(&t.cells).unwrap()
}

#[test]
fn dimensions_and_schedules_follow_their_grids() {
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let mut doc = drawing(
        json!([
            {"kind":"grid","id":1,"layerId":"cizim","attrs":{},"name":"A","pts":[p(0.,-2.),p(0.,10.)]},
            {"kind":"grid","id":2,"layerId":"cizim","attrs":{},"name":"1","pts":[p(-2.,0.),p(12.,0.)]},
            {"kind":"column","id":3,"layerId":"cizim","attrs":{},"at":p(0.,0.),"width":0.4,"anchor":"bottomLeft",
             "grids":[],"place":"A-1"}
        ]),
        "m",
    );
    let a = doc.uid(Slot(1)).unwrap().to_string();
    let one = doc.uid(Slot(2)).unwrap().to_string();
    let column = doc.uid(Slot(3)).unwrap().to_string();
    completed(arch_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"uids":[column],"operation":{"kind":"attach","grids":[a, one]}})).unwrap(),
    ));
    // A dimension from grid A's start to grid 1's middle, tied to both.
    let dim: DimensionsCreate = serde_json::from_value(json!({"layerId":"cizim","at":p(3.,-3.),"measure":{"kind":"aligned",
        "a":{"kind":"place","uid":a,"place":{"kind":"start"}},
        "b":{"kind":"place","uid":one,"place":{"kind":"mid","part":0,"ring":0,"index":0}}}}))
    .unwrap();
    let dim = Slot(completed(dimensions_create::execute(&mut ExecutionContext::new(&mut doc), dim)).ids[0]);
    let table: SchedulesCreate = serde_json::from_value(
        json!({"layerId":"cizim","at":p(20.,0.),"of":"columns","columns":[{"field":"tag"},{"field":"place"}]}),
    )
    .unwrap();
    let table = Slot(completed(schedules_create::execute(&mut ExecutionContext::new(&mut doc), table)).id);
    assert!(table_values(&doc, table).contains("A-1"), "{}", table_values(&doc, table));

    // Grid A moves 2 m east: the dimension's first point and the column follow it.
    completed(transform::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"uids":[a],"transform":{"kind":"move","dx":2.0,"dy":0.0}})).unwrap(),
    ));
    let Entity::Dimension(d) = doc.get(dim).unwrap() else { panic!() };
    assert_eq!((d.a.x, d.a.y), (2.0, -2.0), "the start of A");
    assert_eq!((d.b.x, d.b.y), (5.0, 0.0), "the middle of 1");
    let Entity::Column(c) = doc.get(Slot(3)).unwrap() else { panic!() };
    assert_eq!((c.column.at.x, c.column.at.y), (2.0, 0.0), "the column's lower left on the crossing");

    // Grid A named D: the column's place and the schedule's Yer in the same step.
    completed(arch_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"uids":[a],"operation":{"kind":"set","label":"D"}})).unwrap(),
    ));
    let Entity::Column(c) = doc.get(Slot(3)).unwrap() else { panic!() };
    assert_eq!(c.column.place.as_deref(), Some("D-1"));
    assert!(table_values(&doc, table).contains("D-1"), "{}", table_values(&doc, table));
    doc.undo().unwrap();
    assert!(table_values(&doc, table).contains("A-1"), "one undo takes back both");
}
