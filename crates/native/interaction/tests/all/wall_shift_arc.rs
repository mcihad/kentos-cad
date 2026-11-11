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

/// A curved corridor: arcs of radius 3 and 4.4 about (2, 3), a quarter turn from 225° each, 20 cm walls (1.20 m clear).
fn corridor() -> (Bench, Slot) {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.session.exit();
    let at = |r: f64, deg: f64| {
        let t = deg.to_radians();
        json!({"x": E + 2.0 + r * t.cos(), "y": N + 3.0 + r * t.sin()})
    };
    let bulge = (std::f64::consts::FRAC_PI_2 / 4.0).tan();
    let made = registry::run(
        &mut ExecutionContext::new(&mut b.doc),
        "cad.walls.create",
        CallOp::Execute,
        json!({"layerId": "cizim", "walls": [
            {"pts": [at(3.0, 225.0), at(3.0, 315.0)], "bulges": [bulge], "width": 0.2, "justify": "center"},
            {"pts": [at(4.4, 225.0), at(4.4, 315.0)], "bulges": [bulge], "width": 0.2, "justify": "center"}
        ]}),
    )
    .unwrap();
    assert_eq!(made["status"], "completed", "{made}");
    let slot = Slot(made["output"]["ids"][0].as_u64().unwrap() as u32);
    (b, slot)
}

fn texts(b: &Bench) -> Vec<String> {
    b.session
        .preview(&Default::default())
        .map(|p| p.texts.into_iter().map(|t| t.text).collect())
        .unwrap_or_default()
}

/// docs/adr/8049 §4: the inner arc selected shows its 1.20 m to the outer arc along the radius; its number typed 1.50,
/// the inner arc shrinks about the centre to radius 2.7, in one undo step.
#[test]
fn an_arc_s_temporary_distance_to_its_concentric_arc_moves_it_about_its_centre() {
    let (mut b, inner) = corridor();
    b.selection.set([inner]);
    // Inside the corridor, by the inner arc's lowest point (2, 0).
    b.move_to(2.0, -0.2);
    assert!(
        texts(&b).iter().any(|t| t.contains("1.200")),
        "{:?}",
        texts(&b)
    );
    let at = b
        .session
        .temporary_numbers()
        .into_iter()
        .min_by(|p, q| (p.y - (N - 0.8)).abs().total_cmp(&(q.y - (N - 0.8)).abs()))
        .expect("the corridor's number");
    b.click(at.x - E, at.y - N);
    assert!(b.session.is_running(), "the number's field");
    b.run(|s, cx| s.text_typed(Some("1.5"), cx));
    let (pts, bulges) = wall(&b, inner);
    let r = ((pts[0].0 - 2.0).powi(2) + (pts[0].1 - 3.0).powi(2)).sqrt();
    assert!((r - 2.7).abs() < 1e-9, "{r} {pts:?}");
    assert!((bulges[0] - (std::f64::consts::FRAC_PI_2 / 4.0).tan()).abs() < 1e-9);
    assert_eq!(b.doc.undo().as_deref(), Some("Duvarı düzenle"), "one step");
}

/// docs/adr/8049 §4: the corridor's padlock on the inner arc keeps its 1.20 m as a bond (no + for a permanent
/// dimension on an arc); the outer arc shifted 0.5 m out, the inner follows to radius 3.5.
#[test]
fn the_corridor_s_padlock_bonds_the_two_arcs() {
    let (mut b, inner) = corridor();
    b.selection.set([inner]);
    b.move_to(2.0, -0.2);
    let locks = b.session.temporary_locks();
    assert_eq!(
        locks.len(),
        1,
        "one padlock, the side with an arc across: {locks:?}"
    );
    b.click(locks[0].x - E, locks[0].y - N);
    let set = b.doc.constraints().clone();
    assert_eq!(set.constraints.len(), 1, "one bond");
    assert_eq!(set.parameters[0].expression, "1.2");
    let outer = b
        .doc
        .entities()
        .filter(|e| matches!(e, Entity::Wall(_)))
        .map(|e| Slot(e.base().id))
        .find(|s| *s != inner)
        .expect("the outer arc");
    let uid = b.doc.uid(outer).unwrap().to_string();
    let answer = registry::run(
        &mut ExecutionContext::new(&mut b.doc),
        "cad.walls.edit",
        CallOp::Execute,
        json!({"uids": [uid], "operation": {"kind": "shift", "by": -0.5}}),
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{answer}");
    let (pts, _) = wall(&b, inner);
    let r = ((pts[0].0 - 2.0).powi(2) + (pts[0].1 - 3.0).powi(2)).sqrt();
    assert!((r - 3.5).abs() < 1e-9, "{r}");
}
