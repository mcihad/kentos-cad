//! Duvarı kaydır on an arc (docs/adr/8049 §4): the arc wall's edge is picked by its circle, the pointer's distance from
//! the circle is the shift (outward a negative one for an arc that turns left), and a typed distance writes it in one
//! undo step: the arc grows about its centre, its ends along its radii, its bulge kept.

use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, DrawingUnit, Entity, Workspace};
use kentos_domain::Slot;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::json;

/// An arc wall from (0, 0) to (4, 0), bulge 0.4 (its centre (2, 2.1), radius 2.9, its lowest point (2, −0.8)), 20 cm.
fn arc_wall() -> (Bench, Slot) {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.session.exit();
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    let made = registry::run(
        &mut ExecutionContext::new(&mut b.doc),
        "cad.walls.create",
        CallOp::Execute,
        json!({"layerId": "cizim", "walls": [
            {"pts": [p(0., 0.), p(4., 0.)], "bulges": [0.4], "width": 0.2, "justify": "center"}
        ]}),
    )
    .unwrap();
    assert_eq!(made["status"], "completed", "{made}");
    let slot = Slot(made["output"]["ids"][0].as_u64().unwrap() as u32);
    (b, slot)
}

fn wall(b: &Bench, s: Slot) -> (Vec<(f64, f64)>, Vec<f64>) {
    match b.doc.get(s) {
        Some(Entity::Wall(w)) => (
            w.wall.pts.iter().map(|p| (p.x - E, p.y - N)).collect(),
            w.wall.bulges.clone().unwrap_or_default(),
        ),
        _ => panic!("a wall"),
    }
}

#[test]
fn an_arc_edge_is_picked_by_its_circle_and_a_typed_distance_grows_it() {
    let (mut b, slot) = arc_wall();
    b.start("wallShift");
    b.click(2.0, -0.8);
    // The pointer 0.3 m outside the circle: the arc marked, the wall as the shift would leave it dashed.
    b.move_to(2.0, -1.1);
    let preview = b.session.preview(&Default::default()).expect("the preview");
    assert!(preview.strokes.len() > 1, "{:?}", preview.strokes.len());
    assert!(preview.strokes[0].pts.len() > 2, "the picked arc in chords");
    assert!(b.type_text("-0.3"));
    let (pts, bulges) = wall(&b, slot);
    // The circle 3.2 about (2, 2.1): each end along its radius, by 3.2 / 2.9.
    let k = 3.2 / 2.9;
    let (x0, y0) = (2.0 - 2.0 * k, 2.1 - 2.1 * k);
    assert!(
        (pts[0].0 - x0).abs() < 1e-9 && (pts[0].1 - y0).abs() < 1e-9,
        "{pts:?}"
    );
    assert!(
        (pts[1].0 - (4.0 - x0)).abs() < 1e-9 && (pts[1].1 - y0).abs() < 1e-9,
        "{pts:?}"
    );
    assert!((bulges[0] - 0.4).abs() < 1e-9, "{bulges:?}");
    assert_eq!(b.doc.undo().as_deref(), Some("Duvarı düzenle"), "one step");
    assert_eq!(wall(&b, slot).0, vec![(0.0, 0.0), (4.0, 0.0)]);
}
