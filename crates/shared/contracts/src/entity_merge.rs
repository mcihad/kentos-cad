//! The three-way merge of an object two editors changed (docs/adr/0553 §4): against the copy both started from (the
//! base), each top-level field of the object's JSON takes the side that changed it, the attributes one by one; only a
//! field both changed, differently, is a conflict. Pure: the cloud client uses it where a version check alone would
//! make the whole object a conflict.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::Entity;

/// An object merged.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityMerge {
    pub entity: Entity,
    /// The fields only mine changed (attributes as `attrs.name`).
    pub from_mine: Vec<String>,
    /// The fields only theirs changed.
    pub from_theirs: Vec<String>,
}

/// Which side a field's value comes from, or that both changed it differently.
enum Pick<'a> {
    Same(Option<&'a Value>),
    Mine(Option<&'a Value>),
    Theirs(Option<&'a Value>),
    Clash,
}

fn pick<'a>(
    base: Option<&'a Value>,
    mine: Option<&'a Value>,
    theirs: Option<&'a Value>,
) -> Pick<'a> {
    match (mine == base, theirs == base) {
        (true, true) => Pick::Same(mine),
        (false, true) => Pick::Mine(mine),
        (true, false) => Pick::Theirs(theirs),
        (false, false) if mine == theirs => Pick::Same(mine),
        (false, false) => Pick::Clash,
    }
}

fn object(e: &Entity) -> Option<Map<String, Value>> {
    match serde_json::to_value(e).ok()? {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

/// `mine` and `theirs` (both changed from `base`) put together: the fields the two changed differently, when there
/// are any (as `attrs.name` for an attribute); the object's own slot id is `mine`'s.
pub fn merge_entities(
    base: &Entity,
    mine: &Entity,
    theirs: &Entity,
) -> Result<EntityMerge, Vec<String>> {
    let (Some(b), Some(m), Some(t)) = (object(base), object(mine), object(theirs)) else {
        return Err(vec!["entity".into()]);
    };
    let keys: BTreeSet<&String> = b.keys().chain(m.keys()).chain(t.keys()).collect();
    let mut out = Map::new();
    let (mut from_mine, mut from_theirs, mut clashes) = (Vec::new(), Vec::new(), Vec::new());
    for key in keys {
        if key == "id" {
            if let Some(v) = m.get(key) {
                out.insert(key.clone(), v.clone());
            }
            continue;
        }
        if key == "attrs" {
            let empty = Map::new();
            let attrs = |side: &'_ Map<String, Value>| {
                side.get("attrs").and_then(Value::as_object).cloned()
            };
            let (ba, ma, ta) = (attrs(&b), attrs(&m), attrs(&t));
            let (ba, ma, ta) = (
                ba.as_ref().unwrap_or(&empty),
                ma.as_ref().unwrap_or(&empty),
                ta.as_ref().unwrap_or(&empty),
            );
            let names: BTreeSet<&String> = ba.keys().chain(ma.keys()).chain(ta.keys()).collect();
            let mut merged = Map::new();
            for name in names {
                let (bv, mv, tv) = (ba.get(name), ma.get(name), ta.get(name));
                let value = match pick(bv, mv, tv) {
                    Pick::Same(v) => v,
                    Pick::Mine(v) => {
                        from_mine.push(format!("attrs.{name}"));
                        v
                    }
                    Pick::Theirs(v) => {
                        from_theirs.push(format!("attrs.{name}"));
                        v
                    }
                    Pick::Clash => {
                        clashes.push(format!("attrs.{name}"));
                        mv
                    }
                };
                if let Some(v) = value {
                    merged.insert(name.clone(), v.clone());
                }
            }
            if !merged.is_empty() || m.contains_key("attrs") || t.contains_key("attrs") {
                out.insert(key.clone(), Value::Object(merged));
            }
            continue;
        }
        let (bv, mv, tv) = (b.get(key), m.get(key), t.get(key));
        let value = match pick(bv, mv, tv) {
            Pick::Same(v) => v,
            Pick::Mine(v) => {
                from_mine.push(key.clone());
                v
            }
            Pick::Theirs(v) => {
                from_theirs.push(key.clone());
                v
            }
            Pick::Clash => {
                clashes.push(key.clone());
                mv
            }
        };
        if let Some(v) = value {
            out.insert(key.clone(), v.clone());
        }
    }
    if !clashes.is_empty() {
        return Err(clashes);
    }
    // Two sides' changes to fields that belong together (a kind and its geometry) may not make an object.
    let Ok(entity) = serde_json::from_value::<Entity>(Value::Object(out)) else {
        return Err(vec!["entity".into()]);
    };
    Ok(EntityMerge {
        entity,
        from_mine,
        from_theirs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn point(x: f64, label: Option<&str>, attrs: Value) -> Entity {
        let mut v = json!({ "kind": "point", "id": 1, "layerId": "l", "attrs": attrs, "p": { "x": x, "y": 2.0 } });
        if let Some(l) = label {
            v["label"] = json!(l);
        }
        serde_json::from_value(v).expect("a point")
    }

    #[test]
    fn different_fields_both_win_and_the_same_field_clashes() {
        let base = point(1.0, None, json!({ "a": "1", "b": "1" }));
        // Mine moves it and changes `a`; theirs labels it and changes `b`.
        let mine = point(5.0, None, json!({ "a": "9", "b": "1" }));
        let theirs = point(1.0, Some("kapı"), json!({ "a": "1", "b": "7" }));
        let m = merge_entities(&base, &mine, &theirs).expect("merged");
        assert_eq!(
            m.entity,
            point(5.0, Some("kapı"), json!({ "a": "9", "b": "7" }))
        );
        assert_eq!(m.from_mine, ["attrs.a", "p"]);
        assert_eq!(m.from_theirs, ["attrs.b", "label"]);
        // Both changed `a`, differently: only that is a clash.
        let theirs = point(1.0, None, json!({ "a": "3", "b": "1" }));
        assert_eq!(
            merge_entities(&base, &mine, &theirs).unwrap_err(),
            ["attrs.a"]
        );
        // Both changed it alike: no clash.
        let theirs = point(1.0, None, json!({ "a": "9", "b": "1" }));
        let m = merge_entities(&base, &mine, &theirs).expect("agreed");
        assert_eq!(m.entity, mine);
        // An attribute only one side removed is removed.
        let mine = point(1.0, None, json!({ "b": "1" }));
        let theirs = point(1.0, None, json!({ "a": "1", "b": "2" }));
        let m = merge_entities(&base, &mine, &theirs).expect("merged");
        assert_eq!(m.entity, point(1.0, None, json!({ "b": "2" })));
    }

    /// fixtures/cloud/v1/merge.json, written by scripts/fixtures/entity_merge_cases.py from the ADRs without KentOS code.
    #[test]
    fn the_shared_cases_merge_alike() {
        let text = include_str!("../../../../fixtures/cloud/v1/merge.json");
        let data: Value = serde_json::from_str(text).expect("the cases");
        let entity = |v: &Value| serde_json::from_value::<Entity>(v.clone()).expect("an object");
        let cases = data["cases"].as_array().expect("cases");
        assert!(cases.len() >= 20);
        for case in cases {
            let name = case["name"].as_str().expect("name");
            let got = merge_entities(
                &entity(&case["base"]),
                &entity(&case["mine"]),
                &entity(&case["theirs"]),
            );
            match (&case["merged"], got) {
                (Value::Null, Err(clashes)) => {
                    let want: Vec<String> =
                        serde_json::from_value(case["clashes"].clone()).expect("clashes");
                    assert_eq!(clashes, want, "{name}");
                }
                (merged, Ok(m)) if !merged.is_null() => {
                    assert_eq!(m.entity, entity(merged), "{name}");
                    let names = |k: &str| -> Vec<String> {
                        serde_json::from_value(case[k].clone()).expect("names")
                    };
                    assert_eq!(m.from_mine, names("fromMine"), "{name}");
                    assert_eq!(m.from_theirs, names("fromTheirs"), "{name}");
                }
                (_, got) => panic!("{name}: {got:?}"),
            }
        }
    }

    /// `cargo test --release -p kentos-contracts --lib entity_merge::tests::merge_timing -- --ignored --nocapture`
    /// (docs/adr/0648 Ölçümler): 10 000 merges of a 200-corner parcel whose sides changed a corner and an attribute.
    #[test]
    #[ignore = "a timing, run on its own in release"]
    fn merge_timing() {
        let ring = |dx: f64| -> Value {
            Value::Array(
                (0..200)
                    .map(|i| {
                        let a = f64::from(i) * std::f64::consts::TAU / 200.0;
                        json!({ "x": 487000.0 + 10.0 * a.cos() + if i == 0 { dx } else { 0.0 }, "y": 4420000.0 + 10.0 * a.sin() })
                    })
                    .collect(),
            )
        };
        let parcel = |dx: f64, malik: &str| -> Entity {
            serde_json::from_value(json!({ "kind": "polygon", "id": 1, "layerId": "parsel",
                "attrs": { "parsel_no": "5", "malik": malik }, "pts": ring(dx) }))
            .expect("a parcel")
        };
        let (base, mine, theirs) = (parcel(0.0, "Ali"), parcel(0.5, "Ali"), parcel(0.0, "Veli"));
        let t = std::time::Instant::now();
        for _ in 0..10_000 {
            merge_entities(&base, &mine, &theirs).expect("merged");
        }
        println!("10 000 birleştirme (200 köşe): {:?}", t.elapsed());
    }

    #[test]
    fn the_object_keeps_my_slot_id() {
        let base = point(1.0, None, json!({}));
        let mut mine = point(2.0, None, json!({}));
        mine.base_mut().id = 7;
        let theirs = point(1.0, Some("x"), json!({}));
        let m = merge_entities(&base, &mine, &theirs).expect("merged");
        assert_eq!(m.entity.base().id, 7);
    }
}
