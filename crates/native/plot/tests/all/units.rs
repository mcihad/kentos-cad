//! A sheet's map and its PDF read a layer's renderer in the project's drawing unit
//! (docs/adr/0165 §2, the 13 Ekim addendum), as the drawing area and Katman stili do: a
//! graduated renderer whose classes Katman stili made from `$alan` in a project in
//! millimetres fills each room by its class on paper, and the same classes read in a
//! project in metres meet no room.

use kentos_contracts::{DocumentSnapshotV1, DrawingFont, DrawingUnit, Entity, Workspace};
use kentos_domain::Document;
use kentos_geometry_core::Vec2;
use kentos_interaction::Spatial;
use kentos_native_style::library::StyleLibrary;
use kentos_plot::maps::{MapSource, vector_content};
use kentos_project::new_project::{NewProject, new_project};
use kentos_sheet::kinds::MapLayers;
use kentos_sheet::pdf::MapContent;
use serde_json::{Value, json};

const RED: &str = "#FF0000";
const BLUE: &str = "#0000FF";

fn fill(color: &str) -> Value {
    json!({ "type": "fill", "layers": [{ "id": "0", "type": "simpleFill", "color": color }] })
}

/// A local CAD project in `unit` with three rooms of 3, 1 and 0.5 m² on its layer, a graduated
/// renderer by `$alan` whose classes are written in `class_unit`'s square: [0.5, 1.75) red and
/// [1.75, 3] blue, in that unit's squares of a metre.
fn drawing(unit: DrawingUnit, class_unit: DrawingUnit) -> Document {
    let mut snapshot: DocumentSnapshotV1 = new_project(&NewProject {
        name: "Odalar".into(),
        srid: 0,
        plot_scale: 50.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: Some(unit),
        annotation: None,
        cad_work: None,
    })
    .expect("a new project");
    let layer = snapshot.active_layer.clone();
    for (id, x, w, h) in [(1, 0.0, 2.0, 1.5), (2, 3.0, 1.0, 1.0), (3, 5.0, 0.5, 1.0)] {
        let pts: Vec<Value> = [(x, 0.0), (x + w, 0.0), (x + w, h), (x, h)]
            .iter()
            .map(|(x, y)| json!({ "x": x, "y": y }))
            .collect();
        snapshot.entities.push(
            serde_json::from_value::<Entity>(
                json!({ "kind": "polygon", "id": id, "layerId": layer, "attrs": {}, "pts": pts }),
            )
            .expect("an area"),
        );
    }
    let area = class_unit.per_metre() * class_unit.per_metre();
    let renderer = json!({ "type": "graduated", "expr": "$alan", "classes": [
        { "min": 0.5 * area, "max": 1.75 * area, "label": "a", "symbols": { "fill": fill(RED) } },
        { "min": 1.75 * area, "max": 3.0 * area, "label": "b", "symbols": { "fill": fill(BLUE) } },
    ] });
    let node = snapshot
        .layers
        .iter_mut()
        .find(|l| l.id == layer)
        .expect("the active layer");
    node.style.renderer = Some(renderer);
    Document::from_snapshot(snapshot).expect("opens")
}

/// The fills the map frame's paths have, sorted.
fn map_fills(doc: &Document) -> Vec<String> {
    let mut spatial = Spatial::new();
    spatial.reload(doc);
    let library = StyleLibrary::default();
    let src = MapSource {
        map: None,
        level: None,
        phase: None,
        hidden: &[],
            pens: &[],
            mono: false,
        doc,
        store: spatial.store(),
        library: &library,
        origin: Vec2::new(0.0, 0.0),
    };
    let content = vector_content(
        &src,
        &MapLayers::default(),
        &[],
        50.0,
        [-5.0, -5.0, 20.0, 10.0],
        &[],
    )
    .expect("the map as vectors");
    let MapContent::Vector(map) = content else {
        panic!("a vector map");
    };
    let mut out: Vec<String> = map
        .layers
        .iter()
        .flat_map(|l| l.paths.iter())
        .filter_map(|p| p.fill.clone())
        .map(|c| c.to_uppercase())
        .collect();
    out.sort();
    out
}

#[test]
fn a_map_fills_the_rooms_by_the_classes_in_the_projects_unit() {
    for unit in [DrawingUnit::Mm, DrawingUnit::Cm, DrawingUnit::M] {
        // The salon (3 m²) is in the second class, the room and the closet in the first.
        assert_eq!(
            map_fills(&drawing(unit, unit)),
            [BLUE, RED, RED],
            "classes written in {unit:?} on a map of a project in {unit:?}"
        );
    }
}

#[test]
fn classes_in_another_unit_meet_no_room() {
    // Classes in mm² read by a project in metres: the rooms are 3, 1 and 0.5.
    assert!(map_fills(&drawing(DrawingUnit::M, DrawingUnit::Mm)).is_empty());
    // And in metres read by one in mm: the rooms are 3 000 000, 1 000 000 and 500 000.
    assert!(map_fills(&drawing(DrawingUnit::Mm, DrawingUnit::M)).is_empty());
}
