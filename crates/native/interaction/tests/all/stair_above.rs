//! A stair on the level above its own (docs/adr/0724 §4): with 1. kat active, Zemin kat's stair that reaches its cut
//! plane is drawn as the plan from above shows it (its part past its cut), snapped to and never picked; one without a
//! cut stays hidden; Bütün katlar draws it as its own plan does. A stair edited keeps its look. The stair tool following
//! Etkin kat's height binds the stair to it, and Döner merdiven's Merkez direği (M) gives its centre post (§1, §3).

use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, DrawingUnit, Entity, Workspace};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_interaction::Vec2;
use kentos_interaction::level_view::{LevelView, looks};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn run(b: &mut Bench, name: &str, input: Value) -> Value {
    let answer = registry::run(&mut ExecutionContext::new(&mut b.doc), name, CallOp::Execute, input).expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// Zemin kat and 1. kat 3.06 m apart; on Zemin kat a stair up with a cut (a U) and one without, on 1. kat a line.
fn building() -> (Bench, Slot, Slot) {
    let mut b = Bench::new("stair");
    b.session.exit();
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.drawing_unit = Some(DrawingUnit::M);
    s.arch = Some(
        serde_json::from_value(json!({"levels": [
            {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.06},
            {"id": "kat-1", "name": "1. kat", "elevation": 3.06, "height": 3.06}
        ]}))
        .unwrap(),
    );
    b.doc.set_settings(s);
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    run(&mut b, "cad.stairs.create", json!({"layerId": "cizim", "stairs": [
        {"pts": [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 2.6), p(0.0, 2.6)], "width": 1.2, "risers": 18,
         "riserHeight": 0.17, "tread": 0.28, "cut": 7, "railings": "both", "arch": {"level": "zemin"}},
        {"pts": [p(10.0, 0.0), p(12.0, 0.0)], "width": 1.0, "risers": 4, "riserHeight": 0.15, "tread": 0.3,
         "arch": {"level": "zemin"}}
    ]}));
    let ids: Vec<Slot> = b
        .doc
        .entities()
        .filter(|e| matches!(e, Entity::Stair(_)))
        .map(|e| Slot(e.base().id))
        .collect();
    (b, ids[0], ids[1])
}

fn upper_in_store(b: &Bench, s: Slot) -> bool {
    b.spatial
        .store()
        .get(f64::from(s.0))
        .is_some_and(|it| matches!(&it.shape, Shape::Stair { stair, .. } if stair.upper))
}

#[test]
fn the_level_above_draws_the_stair_past_its_cut_and_never_picks_it() {
    let (mut b, cut, short) = building();
    // Bütün katlar: nothing from above.
    assert!(looks(&b.doc, LevelView::All, Some("kat-1")).upper.is_empty());
    // 1. kat alone: the stair with a cut seen from above, the short one hidden.
    let l = looks(&b.doc, LevelView::Active, Some("kat-1"));
    assert_eq!(l.upper, vec![cut.0]);
    assert_eq!(l.hidden, vec![short.0]);
    // Etkin kat ve alt kat: the same stair from above, not faint; the short one faint.
    let l = looks(&b.doc, LevelView::Below, Some("kat-1"));
    assert_eq!((l.upper.clone(), l.faded.clone()), (vec![cut.0], vec![short.0]));
    // On its own level it is its own plan.
    assert!(looks(&b.doc, LevelView::Active, Some("zemin")).upper.is_empty());

    b.spatial.sync(&b.doc);
    b.spatial.set_level_view(&b.doc, LevelView::Active, Some("kat-1".into()));
    assert!(upper_in_store(&b, cut), "put as the plan from above shows it");
    assert!(b.spatial.draws_otherwise(), "a sheet's map then wants a store of its own");
    let pieces = b.spatial.store().get(f64::from(cut.0)).and_then(|it| it.expanded.clone()).expect("its pieces");
    let shape = kentos_native_application::geometry::shape(b.doc.get(cut).expect("the stair"));
    let Shape::Stair { pts, stair } = &shape else { panic!("a stair shape") };
    let mut above = stair.clone();
    above.upper = true;
    assert_eq!(
        pieces.shapes.len(),
        kentos_geometry_core::geom::arch::stair_upper_pieces(pts, &above, b.doc.settings().plot_scale).len(),
        "the store holds what the plan from above draws"
    );
    // Never picked, snapped to: its end's corner (the U's last flight ends at (0, 2.6) ± 0.6).
    let at = |x: f64, y: f64| Vec2::new(E + x, N + y);
    assert_eq!(b.spatial.pick(at(2.0, 2.6), 0.2), None, "another level's stair is not picked");
    let snapped = b
        .spatial
        .snap(at(0.02, 3.22), 0.2, kentos_interaction::default_snap_kinds(), None)
        .map(|h| Slot(h.id as u32));
    assert_eq!(snapped, Some(cut), "what is drawn is snapped to");

    // Edited on its own level (wider), it comes back from above after the sync.
    let uid = b.doc.uid(cut).expect("its uid").to_string();
    run(&mut b, "cad.arch.edit", json!({"uids": [uid], "operation": {"kind": "set", "width": 1.3}}));
    b.spatial.sync(&b.doc);
    assert!(upper_in_store(&b, cut), "kept from above after its edit");

    // Bütün katlar: its own plan again, picked.
    b.spatial.set_level_view(&b.doc, LevelView::All, Some("kat-1".into()));
    assert!(!upper_in_store(&b, cut));
    assert!(!b.spatial.draws_otherwise());
    assert_eq!(b.spatial.pick(at(2.0, 0.0), 0.2), Some(cut));
}

#[test]
fn the_tool_binds_the_rise_to_the_level_and_gives_a_curved_stair_its_post() {
    let (mut b, _, _) = building();
    b.level = Some("kat-1".into());
    // Merdiven following 1. kat's height: bound to it.
    b.start("stair");
    b.click(20.0, 0.0);
    b.click(20.0, 9.0);
    b.confirm();
    let Entity::Stair(s) = b.newest() else { panic!("a stair") };
    assert!(s.stair.height_from_level, "bound to the level's height");
    assert_eq!(s.stair.risers, 18);
    assert!((s.stair.riser_height * 18.0 - 3.06).abs() < 1e-9);
    // 1. kat raised by 0.34 m: the stair follows.
    let mut st = b.doc.settings().clone();
    st.arch.as_mut().unwrap().levels[1].height = Some(3.4);
    let levels = serde_json::to_value(&st.arch.as_ref().unwrap().levels).unwrap();
    run(&mut b, "cad.settings.set", json!({"arch": {"levels": levels}}));
    let Entity::Stair(s) = b.newest() else { panic!("a stair") };
    assert!((f64::from(s.stair.risers) * s.stair.riser_height - 3.4).abs() < 1e-9, "{:?}", s.stair);
    // A typed rise is the stair's own.
    b.start("stair");
    assert!(b.run(|session, cx| session.configure("Y", "2.8", cx)));
    b.click(30.0, 0.0);
    b.click(30.0, 9.0);
    b.confirm();
    let Entity::Stair(s) = b.newest() else { panic!("a stair") };
    assert!(!s.stair.height_from_level);
    // Döner merdiven with Merkez direği 0.2 m.
    b.start("stairCurved");
    assert!(b.run(|session, cx| session.configure("M", "0.2", cx)));
    let shown = b.run(|session, _| session.settings(&Default::default()));
    assert!(shown.iter().any(|r| r.key == "M"), "Merkez direği shows on Döner merdiven");
    b.click(40.0, 0.0);
    b.click(41.5, 0.0);
    b.move_to(40.0, 1.5);
    b.move_to(38.5, 0.0);
    b.click(38.5, 0.0);
    b.confirm();
    let Entity::Stair(s) = b.newest() else { panic!("a stair") };
    assert_eq!(s.stair.post, Some(0.2));
    // A straight stair has none to give.
    b.start("stair");
    let shown = b.run(|session, _| session.settings(&Default::default()));
    assert!(!shown.iter().any(|r| r.key == "M"));
}
