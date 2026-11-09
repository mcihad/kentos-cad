//! Tadilat (docs/adr/3039 §2): a phase view hides walls and openings and leaves them out of the joins and cuts, so
//! each phase's plan draws its walls as they stand in it. An existing wall with a new door cut into it and an existing
//! window closed up: the existing plan's wall has the window's hole only, the new plan's the door's only, the
//! demolition plan's the window's (dashed) and not the door's.

use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, DrawingUnit, Entity, PhaseView, Workspace};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn run(b: &mut Bench, name: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(&mut b.doc),
        name,
        CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// An existing 6 m wall, 30 cm thick; a 90 cm door at 1.5 m, new; a 120 cm window at 4 m, closed up (İptal).
fn renovation() -> (Bench, Slot) {
    let mut b = Bench::new("line");
    b.session.exit();
    let mut s = b.doc.settings().clone();
    s.workspace = Some(Workspace::Cad);
    s.srid = 0;
    s.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(s);
    let wall = run(
        &mut b,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{
            "pts": [{"x": E, "y": N}, {"x": E + 6.0, "y": N}], "width": 0.3, "justify": "center",
            "arch": {"phase": "existing"}
        }]}),
    );
    let host = wall["output"]["uids"][0].clone();
    let made = run(
        &mut b,
        "cad.openings.create",
        json!({"layerId": "cizim", "openings": [
            {"host": host, "at": 1.5, "width": 0.9},
            {"host": host, "at": 4.0, "width": 1.2, "type": "Pencere 120"}
        ]}),
    );
    let uids = made["output"]["uids"].clone();
    run(
        &mut b,
        "cad.openings.edit",
        json!({"uids": [uids[0]], "operation": {"kind": "set", "phase": "new"}}),
    );
    run(
        &mut b,
        "cad.openings.edit",
        json!({"uids": [uids[1]], "operation": {"kind": "set", "phase": "demolish"}}),
    );
    let wall = b
        .doc
        .entities()
        .find(|e| matches!(e, Entity::Wall(_)))
        .map(|e| Slot(e.base().id))
        .unwrap();
    (b, wall)
}

fn viewed(b: &mut Bench, view: PhaseView) {
    let mut s = b.doc.settings().clone();
    let mut arch = s.arch.clone().unwrap_or_default();
    arch.phase_view = (view != PhaseView::All).then_some(view);
    s.arch = Some(arch);
    b.doc.set_settings(s);
    b.spatial.sync(&b.doc);
}

/// The holes in the wall's face as the store draws it: the gaps between its face's runs along the wall, metres from
/// its start, to the centimetre.
fn holes(b: &Bench, wall: Slot) -> Vec<(f64, f64)> {
    let it = b.spatial.store().get(f64::from(wall.0)).expect("the wall");
    let x = it.expanded.clone().expect("its pieces");
    // One face's runs: the 2-point polylines on the face y = +0.15.
    let mut runs: Vec<(f64, f64)> = x
        .shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Polyline { pts, .. }
                if pts.len() == 2 && pts.iter().all(|p| (p.y - N - 0.15).abs() < 1e-6) =>
            {
                let (a, b) = (pts[0].x - E, pts[1].x - E);
                Some((a.min(b), a.max(b)))
            }
            _ => None,
        })
        .collect();
    runs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let cm = |v: f64| (v * 100.0).round() / 100.0;
    runs.windows(2)
        .filter(|w| w[1].0 - w[0].1 > 1e-6)
        .map(|w| (cm(w[0].1), cm(w[1].0)))
        .collect()
}

const DOOR: (f64, f64) = (1.05, 1.95);
const WINDOW: (f64, f64) = (3.4, 4.6);

#[test]
fn each_phase_s_plan_cuts_its_wall_as_it_stands() {
    let (mut b, wall) = renovation();
    viewed(&mut b, PhaseView::All);
    assert_eq!(holes(&b, wall), vec![DOOR, WINDOW], "Tümü: both");
    // Mevcut durum: the new door is not there yet; the window still is.
    viewed(&mut b, PhaseView::Existing);
    assert_eq!(holes(&b, wall), vec![WINDOW], "Mevcut durum");
    // Yıkım planı: the window to be closed up (dashed), the new door not yet.
    viewed(&mut b, PhaseView::Demolition);
    assert_eq!(holes(&b, wall), vec![WINDOW], "Yıkım planı");
    // Yeni durum: the door cut, the window closed up (the wall whole there).
    viewed(&mut b, PhaseView::Proposed);
    assert_eq!(holes(&b, wall), vec![DOOR], "Yeni durum");
    // The door's own phase taken away: its wall's (Mevcut), so the existing plan has it too.
    let door = b
        .doc
        .entities()
        .find(|e| matches!(e, Entity::Opening(o) if o.opening.width < 1.0))
        .map(|e| b.doc.uid(Slot(e.base().id)).unwrap().to_string())
        .unwrap();
    run(
        &mut b,
        "cad.openings.edit",
        json!({"uids": [door], "operation": {"kind": "set", "phase": null}}),
    );
    viewed(&mut b, PhaseView::Existing);
    assert_eq!(
        holes(&b, wall),
        vec![DOOR, WINDOW],
        "the door its wall's: Mevcut"
    );
    // Back to Tümü: nothing left out.
    viewed(&mut b, PhaseView::All);
    assert!(b.spatial.store().no_option_out());
}

/// A wall pulled down joins nothing in the new plan: the existing wall it met at a corner ends there square.
#[test]
fn a_wall_pulled_down_takes_its_corner_with_it() {
    let (mut b, wall) = renovation();
    run(
        &mut b,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{
            "pts": [{"x": E + 6.0, "y": N}, {"x": E + 6.0, "y": N + 4.0}], "width": 0.3, "justify": "center",
            "arch": {"phase": "demolish"}
        }]}),
    );
    // The face's end: with the corner it runs to the corner's outer face, without it to the wall's own end.
    let end = |b: &Bench| {
        let it = b.spatial.store().get(f64::from(wall.0)).unwrap();
        it.expanded
            .clone()
            .unwrap()
            .shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Polyline { pts, .. }
                    if pts.len() == 2 && pts.iter().all(|p| (p.y - N + 0.15).abs() < 1e-6) =>
                {
                    Some(pts[0].x.max(pts[1].x) - E)
                }
                _ => None,
            })
            .fold(f64::MIN, f64::max)
    };
    viewed(&mut b, PhaseView::All);
    assert!(
        (end(&b) - 6.15).abs() < 1e-6,
        "joined: the outer face runs round the corner, {}",
        end(&b)
    );
    viewed(&mut b, PhaseView::Proposed);
    assert!(
        (end(&b) - 6.0).abs() < 1e-6,
        "pulled down: the wall ends at its own end, {}",
        end(&b)
    );
    viewed(&mut b, PhaseView::Existing);
    assert!(
        (end(&b) - 6.15).abs() < 1e-6,
        "still there in the existing plan, {}",
        end(&b)
    );
}
