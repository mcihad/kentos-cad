//! Obstacle-only neighbours (docs/adr/0578 §1): an object just outside a bound layer's part holds room for the
//! labels near the part's edge as if it were one of the layer's objects, and is neither drawn nor labelled nor found.

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::labels::LABEL_STRIDE;
use kentos_geometry_core::store::placing::{LABEL_PLACED, ObjectLabels, PlaceOptions};

const LAYERS: &str = r#"[
  {"id":"nokta","rank":1,"point":4,"label":{"placement":"beside","size":10,"point":"around"}},
  {"id":"bina","rank":3,"labels":{"mode":"off","obstacle":{"weight":4}}}
]"#;
const SCALE: f64 = 4.0;

/// A point named “P.1” at the origin; a building east of it, when `real`, standing right where the label's
/// first candidate would be.
fn store(real: bool) -> Store {
    let mut s = Store::new();
    let mut objects =
        String::from(r#"[{"id":1,"layerId":"nokta","kind":"point","p":{"x":0,"y":0}}"#);
    if real {
        objects.push_str(
            r#",{"id":2,"layerId":"bina","kind":"polygon","pts":[{"x":0.5,"y":-3},{"x":14,"y":-3},{"x":14,"y":3},{"x":0.5,"y":3}]}"#,
        );
    }
    objects.push(']');
    s.put_json(&objects).expect("objects");
    s.set_label_layers_json(LAYERS).expect("layers");
    s.set_object_labels(
        1.0,
        ObjectLabels {
            texts: vec![(0, "P.1".to_owned())],
            z: f64::NAN,
        },
    );
    s
}

fn middle(s: &Store) -> [f64; 2] {
    let window = Bounds {
        min_x: -30.0,
        min_y: -20.0,
        max_x: 30.0,
        max_y: 20.0,
    };
    let shown = s.place_labels(
        &window,
        SCALE,
        &[],
        PlaceOptions {
            unplaced: true,
            hidden: false,
        },
    );
    let r = shown
        .records
        .chunks_exact(LABEL_STRIDE)
        .find(|r| r[1] == LABEL_PLACED && (r[8] as u32 & 6) == 0)
        .expect("the label is placed");
    [r[2], r[3]]
}

#[test]
fn a_ghost_holds_room_as_the_object_would_and_is_nothing_else() {
    let free = store(false);
    let real = store(true);
    let building = real.get(2.0).expect("the building").shape.clone();
    let mut ghosted = store(false);
    ghosted.set_ghosts("bina", [building]);
    assert_eq!(ghosted.ghost_count(None), 1);
    assert_eq!(ghosted.ghost_count(Some("bina")), 1);
    assert_eq!(ghosted.ghost_count(Some("nokta")), 0);
    assert_eq!(ghosted.len(), 1, "a ghost is no object");
    let (a, b, c) = (middle(&free), middle(&ghosted), middle(&real));
    assert_ne!(a, b, "the building east of the point moves the label");
    assert_eq!(b, c, "a ghost holds the room its object would");
    // Replaced, then taken away.
    ghosted.set_ghosts("bina", []);
    assert_eq!(ghosted.ghost_count(None), 0);
    assert_eq!(middle(&ghosted), a);
}
