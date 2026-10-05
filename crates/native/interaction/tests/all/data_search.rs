//! What Veride ara reads of a drawing (docs/adr/0178 §1, §3) over a document:
//! the records, the scope and the attribute names against the independent
//! reference in `fixtures/search/v1/cases.json`
//! (`scripts/fixtures/data_search_cases.py`), the cases the web runs in
//! apps/web/src/model/dataSearch.test.ts. The matching is the core's
//! (crates/shared/geometry-core/tests/all/data_search.rs).

use crate::common;

use common::{Bench, E, N, base};
use kentos_contracts::{BlockId, Entity, PointEntity};
use kentos_domain::Slot;
use kentos_geometry_core::ops::data_search::Record;
use kentos_interaction::data_search::{attribute_names, in_scope, index, record_of};
use serde_json::Value;

const TEXTS: &str = include_str!("../../../../../fixtures/interaction/v1/texts.kcad");

fn fixture() -> Value {
    let file: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/search/v1/cases.json"))
            .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.search-fixtures");
    assert_eq!(file["version"], 1);
    file
}

/// A record as the fixture writes it (its attributes as pairs).
fn record(v: &Value) -> Option<Record> {
    let text = |k: &str| v[k].as_str().map(str::to_owned);
    (!v.is_null()).then(|| Record {
        kind: text("kind").expect("kind"),
        layer: text("layer").expect("layer"),
        label: text("label"),
        text: text("text"),
        block: text("block"),
        attrs: v["attrs"]
            .as_array()
            .expect("attrs")
            .iter()
            .map(|p| {
                [
                    p[0].as_str().expect("name").to_owned(),
                    p[1].as_str().expect("value").to_owned(),
                ]
            })
            .collect(),
    })
}

/// The attributes by name: the core looks at them in that order.
fn sorted(r: Option<Record>) -> Option<Record> {
    r.map(|mut r| {
        r.attrs.sort();
        r
    })
}

#[test]
fn objects_give_the_records_the_reference_reads() {
    let file = fixture();
    let cases = file["records"].as_array().expect("cases");
    assert!(cases.len() >= 25, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let entity: Entity = match serde_json::from_value(c["entity"].clone()) {
            Ok(e) => e,
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        let layer = c["layers"][&entity.base().layer_id]
            .as_str()
            .unwrap_or_default();
        let got = record_of(&entity, layer, |id: &BlockId| {
            c["blocks"][id.to_text()].as_str().map(str::to_owned)
        });
        if sorted(got.clone()) != sorted(record(&c["expected"])) {
            failures.push(format!("{name}: {got:?} ≠ {}", c["expected"]));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_drawing_carries_the_attribute_names_the_reference_lists() {
    let file = fixture();
    let drawings = file["drawings"].as_array().expect("drawings");
    for c in file["names"].as_array().expect("cases") {
        let records: Vec<Record> = drawings[c["drawing"].as_u64().unwrap() as usize]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| record(r).unwrap())
            .collect();
        let want: Vec<&str> = c["expected"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_str().unwrap())
            .collect();
        assert_eq!(attribute_names(&records), want, "{}", c["name"]);
    }
}

/// The trace's text drawing with a point of a name and a code, and one with
/// nothing to find.
fn scene() -> (Bench, Slot, Slot) {
    let mut b = Bench::on(TEXTS);
    let mut named = base("cizim");
    named.label = Some("P1".to_owned());
    named.attrs.insert("Kod".to_owned(), "ST".to_owned());
    let a = b
        .doc
        .add(Entity::Point(PointEntity {
            base: named,
            p: kentos_contracts::Vec2 { x: E, y: N },
            z: None,
            parts: None,
        }))
        .expect("a slot");
    let bare = b
        .doc
        .add(Entity::Point(PointEntity {
            base: base("cizim"),
            p: kentos_contracts::Vec2 { x: E + 1.0, y: N },
            z: None,
            parts: None,
        }))
        .expect("a slot");
    (b, a, bare)
}

#[test]
fn the_index_holds_what_has_something_to_find_in_the_drawings_order() {
    let (b, named, bare) = scene();
    let i = index(&b.doc);
    // Four texts (Ada 101, Yol 12, Park, Kilitli) and the named point; the bare point is left out.
    assert_eq!(i.slots.len(), 5);
    assert_eq!(i.slots.last(), Some(&named));
    assert!(!i.slots.contains(&bare));
    let last = i.records.last().unwrap();
    assert_eq!(
        (
            last.kind.as_str(),
            last.label.as_deref(),
            last.layer.as_str()
        ),
        ("Nokta", Some("P1"), b.doc.layers().path("cizim").as_str())
    );
    assert_eq!(last.attrs, vec![["Kod".to_owned(), "ST".to_owned()]]);
    assert_eq!(i.records[0].text.as_deref(), Some("Ada 101"));
}

#[test]
fn a_scope_takes_a_layer_and_the_selection_in_the_drawings_order() {
    let (b, named, _) = scene();
    let i = index(&b.doc);
    let all: Vec<usize> = (0..5).collect();
    assert_eq!(in_scope(&i, None, None), all);
    let on_cizim = in_scope(&i, Some("cizim"), None);
    assert!(!on_cizim.is_empty() && on_cizim.len() < 5, "{on_cizim:?}");
    assert!(on_cizim.iter().all(|&k| i.layer_ids[k] == "cizim"));
    assert_eq!(in_scope(&i, None, Some(&[named, Slot(1)])), vec![0, 4]);
    assert_eq!(
        in_scope(&i, Some("kilitli"), Some(&[named])),
        Vec::<usize>::new()
    );
}
