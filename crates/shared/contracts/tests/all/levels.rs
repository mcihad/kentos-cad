//! An object's level as a level view reads it (docs/adr/0715 §1): its own; an opening's its wall's; a tag's its
//! object's; a room finish's its room's; a dimension's that of the objects it is bound to when they agree.

use kentos_contracts::Entity;
use serde_json::json;

const W0: &str = "0192f5a1-7030-7000-8000-000000000001";
const W1: &str = "0192f5a1-7030-7000-8000-000000000002";
const OP: &str = "0192f5a1-7030-7000-8000-000000000003";
const SP: &str = "0192f5a1-7030-7000-8000-000000000004";
const LINE: &str = "0192f5a1-7030-7000-8000-000000000005";

fn entity(v: serde_json::Value) -> Entity {
    serde_json::from_value(v).expect("an entity")
}

fn wall(id: u32, y: f64, level: &str) -> Entity {
    entity(json!({"kind": "wall", "id": id, "layerId": "a", "attrs": {},
        "pts": [{"x": 0.0, "y": y}, {"x": 10.0, "y": y}], "width": 0.2, "justify": "center",
        "arch": {"level": level}}))
}

fn drawing() -> Vec<(&'static str, Entity)> {
    vec![
        (W0, wall(1, 0.0, "zemin")),
        (W1, wall(2, 5.0, "kat-1")),
        (OP, entity(json!({"kind": "opening", "id": 3, "layerId": "a", "attrs": {}, "host": W1, "at": 2.0,
            "width": 0.9, "form": "door",
            "place": [{"x": 1.55, "y": 5.1}, {"x": 1.55, "y": 4.9}, {"x": 2.45, "y": 5.1}, {"x": 2.45, "y": 4.9}]}))),
        (SP, entity(json!({"kind": "space", "id": 4, "layerId": "a", "attrs": {},
            "parts": [{"outer": {"pts": [{"x": 0.0, "y": 0.0}, {"x": 4.0, "y": 0.0}, {"x": 4.0, "y": 3.0},
                {"x": 0.0, "y": 3.0}]}}], "arch": {"level": "zemin"}}))),
        (LINE, entity(json!({"kind": "line", "id": 5, "layerId": "a", "attrs": {},
            "a": {"x": 0.0, "y": 9.0}, "b": {"x": 3.0, "y": 9.0}}))),
    ]
}

fn dimension(a: &str, b: &str) -> Entity {
    entity(json!({"kind": "dimension", "id": 9, "layerId": "a", "attrs": {},
        "a": {"x": 0.0, "y": -1.0}, "b": {"x": 10.0, "y": -1.0}, "height": 0.125, "offset": -0.5,
        "refs": {"a": {"uid": a, "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 0}},
                 "b": {"uid": b, "place": {"kind": "vertex", "part": 0, "ring": 0, "index": 1}}}}))
}

/// `e`'s level in drawing `d`.
fn level_in<'a>(e: &'a Entity, d: &'a [(&str, Entity)]) -> Option<String> {
    e.level(|u| d.iter().find(|(id, _)| *id == u).map(|(_, x)| x))
        .map(str::to_owned)
}

#[test]
fn objects_take_their_own_their_hosts_or_their_bonds_level() {
    let d = drawing();
    let level = |e: &Entity| level_in(e, &d);
    assert_eq!(level(&d[0].1).as_deref(), Some("zemin"), "a wall's own");
    assert_eq!(level(&d[2].1).as_deref(), Some("kat-1"), "an opening's its wall's");
    assert_eq!(level(&d[3].1).as_deref(), Some("zemin"), "a room's own");
    assert_eq!(level(&d[4].1), None, "a line has none");
    // A tag of the opening: the opening's wall's level.
    let tag = entity(json!({"kind": "tag", "id": 6, "layerId": "a", "attrs": {}, "of": OP,
        "at": {"x": 2.0, "y": 4.0}, "text": "K1"}));
    assert_eq!(level(&tag).as_deref(), Some("kat-1"));
    // A tag whose object is gone: none.
    let lost = entity(json!({"kind": "tag", "id": 7, "layerId": "a", "attrs": {},
        "of": "0192f5a1-7030-7000-8000-0000000000ff", "at": {"x": 2.0, "y": 4.0}, "text": "X"}));
    assert_eq!(level(&lost), None);
    // A room's finish: the room's.
    let finish = entity(json!({"kind": "hatch", "id": 8, "layerId": "a", "attrs": {},
        "ring": [{"x": 0, "y": 0}, {"x": 4, "y": 0}, {"x": 4, "y": 3}, {"x": 0, "y": 3}],
        "pattern": {"type": "solid", "angle": 0, "spacing": 1}, "assoc": {"outer": SP, "seed": {"x": 2, "y": 1}}}));
    assert_eq!(level(&finish).as_deref(), Some("zemin"));
    // A dimension: its bonds' level when they agree (a wall and the opening on another wall of that level would);
    // none across levels or bound to an object of none.
    assert_eq!(level(&dimension(W0, W0)).as_deref(), Some("zemin"));
    assert_eq!(level(&dimension(W1, OP)).as_deref(), Some("kat-1"));
    assert_eq!(level(&dimension(W0, W1)), None, "across levels");
    assert_eq!(level(&dimension(W0, LINE)), None, "a bond with no level");
    // Own levels only of the kinds that have one.
    assert_eq!(d[2].1.own_level(), None, "an opening has none of its own");
}
