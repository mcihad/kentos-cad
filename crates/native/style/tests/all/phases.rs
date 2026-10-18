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
