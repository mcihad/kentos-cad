//! The phase view (docs/adr/0346 §2) against the independent reference's cases (fixtures/arch/v1/phases.json,
//! scripts/fixtures/phase_cases.py): the table of looks and an object as each look draws it.

use kentos_contracts::{ArchPhase, Entity, PhaseLook, PhaseView};
use kentos_native_style::phases::phased_one;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/arch/v1/phases.json");

fn look_name(l: PhaseLook) -> &'static str {
    match l {
        PhaseLook::Shown => "shown",
        PhaseLook::Hidden => "hidden",
        PhaseLook::Dashed => "dashed",
        PhaseLook::Faded => "faded",
    }
}

#[test]
fn phases_look_as_the_reference_says() {
    let file: Value = serde_json::from_str(CASES).unwrap();
    for c in file["table"].as_array().unwrap() {
        let view: PhaseView = serde_json::from_value(c["view"].clone()).unwrap();
        let phase: Option<ArchPhase> = serde_json::from_value(c["phase"].clone()).unwrap();
        assert_eq!(look_name(view.look(phase)), c["look"], "{c}");
    }
    for c in file["objects"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let e: Entity = serde_json::from_value(c["entity"].clone()).unwrap();
        let view: PhaseView = serde_json::from_value(c["view"].clone()).unwrap();
        let look = view.look(e.own_phase());
        assert_eq!(look_name(look), c["look"], "{name}");
        let drawn = phased_one(&e, look).map(|x| serde_json::to_value(x.as_ref()).unwrap());
        assert_eq!(drawn.unwrap_or(Value::Null), c["drawn"], "{name}");
    }
}

/// docs/adr/8012, step 3: an object the store draws as another level's plan shows it (a slab of the level above,
/// dashed; a stair of the level under, from above) changes no look here, but a phasing noting it is another one. The
/// drawing area builds again when its look changes: Bütün katlar, which hides and fades nothing more than Yalnız etkin
/// kat on the level under a balcony, once kept the balcony dashed on the screen.
#[test]
fn a_level_view_that_only_draws_across_still_changes_the_look() {
    use kentos_native_style::phases::Phasing;
    let plain = Phasing::default();
    let mut across = Phasing::default();
    across.drawn_across(7);
    assert!(
        across.is_empty(),
        "nothing hidden, faint or dashed by the phasing itself"
    );
    assert_eq!(across.hidden().count(), 0);
    assert_ne!(
        across, plain,
        "but the look is another: what holds object 7 is built again"
    );
}

/// Tadilat projesi (docs/adr/8039 §6) leaves out a room pulled down and its label, not its wall: the plan names the
/// rooms as they will be, its walls carry the three techniques. Yıkım planı still dashes the room.
#[test]
fn tadilat_projesi_leaves_out_a_room_pulled_down_and_its_label() {
    let room: Entity = serde_json::from_value(serde_json::json!({
        "kind": "space", "id": 1, "layerId": "0", "attrs": {},
        "parts": [{"outer": {"pts": [{"x": 0.0, "y": 0.0}, {"x": 4.0, "y": 0.0}, {"x": 4.0, "y": 3.0}, {"x": 0.0, "y": 3.0}]}}],
        "name": "Yatak odası", "arch": {"phase": "demolish"}
    }))
    .unwrap();
    let label: Entity = serde_json::from_value(serde_json::json!({
        "kind": "tag", "id": 2, "layerId": "0", "attrs": {}, "of": "0192f5a1-2590-7000-8000-000000000001",
        "at": {"x": 2.0, "y": 1.5}
    }))
    .unwrap();
    let wall: Entity = serde_json::from_value(serde_json::json!({
        "kind": "wall", "id": 3, "layerId": "0", "attrs": {},
        "pts": [{"x": 4.0, "y": 0.0}, {"x": 4.0, "y": 3.0}], "width": 0.15, "justify": "center",
        "arch": {"phase": "demolish"}
    }))
    .unwrap();
    let by_uid = |u: &str| (u == "0192f5a1-2590-7000-8000-000000000001").then_some(&room);
    let r = PhaseView::Renovation;
    assert_eq!(room.look_in(r, by_uid), PhaseLook::Hidden);
    assert_eq!(label.look_in(r, by_uid), PhaseLook::Hidden);
    assert_eq!(wall.look_in(r, by_uid), PhaseLook::Dashed);
    let d = PhaseView::Demolition;
    assert_eq!(room.look_in(d, by_uid), PhaseLook::Dashed);
    assert_eq!(label.look_in(d, by_uid), PhaseLook::Dashed);
}
