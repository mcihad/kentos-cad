//! `road.terrains.repair` (docs/adr/0916 §6): an accepted surface stays accepted after a repair and after its undo;
//! the drawing's points are not touched.

use kentos_commands::{CommandResult, TerrainState, TerrainsEdit, TerrainsEditOperation, TerrainsQuery, TerrainsRepair};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, road_terrains_edit, road_terrains_query, road_terrains_repair};

fn drawing() -> Document {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/road.terrains.repair.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("setup");
    Document::from_snapshot_v2(snapshot).expect("opens")
}

fn state(doc: &mut Document) -> TerrainState {
    let q = road_terrains_query::execute(&mut ExecutionContext::new(doc), TerrainsQuery { surface: Some("t-12".into()), ..Default::default() });
    match q {
        CommandResult::Completed { output, .. } => output.surfaces.iter().find(|s| s.id == "t-12").expect("t-12").state,
        other => panic!("reads: {other:?}"),
    }
}

#[test]
fn an_accepted_surface_stays_accepted() {
    let mut doc = drawing();
    let points = doc.entities().count();
    let accept = TerrainsEdit { operation: TerrainsEditOperation::Accept, surface: None, id: Some("t-12".into()), expected_revision: None };
    assert!(matches!(road_terrains_edit::execute(&mut ExecutionContext::new(&mut doc), accept), CommandResult::Completed { .. }));
    assert_eq!(state(&mut doc), TerrainState::Current);
    let repair = TerrainsRepair { surface: "t-12".into(), exclude: vec![[12000.7, 1.3]], restore: Vec::new(), expected_revision: None };
    let done = road_terrains_repair::execute(&mut ExecutionContext::new(&mut doc), repair);
    assert!(matches!(done, CommandResult::Completed { .. }), "{done:?}");
    assert_eq!(state(&mut doc), TerrainState::Current);
    assert_eq!(doc.entities().count(), points);
    assert_eq!(doc.undo().as_deref(), Some("Yüzeyi onar"));
    assert_eq!(state(&mut doc), TerrainState::Current);
}
