//! A slab on the level above the active one (docs/adr/8012, step 3): with Zemin kat active, 1. kat's balcony and floor
//! show on its plan dashed and narrow, their railing left out, snapped to and never picked; the active level's own slab
//! is its own plan's; Bütün katlar and 1. kat itself draw them as their own plan does. Mimarlar Odası §14.5.3 u: "Planın
//! geçtiği düzlem ile tavan arasında kalan imalat nokta nokta işlenir. (Saçak ara kat çıkma vb.)"

use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, DrawingUnit, Entity, Workspace};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_interaction::Vec2;
use kentos_interaction::level_view::{LevelView, looks};
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

/// Zemin kat and 1. kat 3 m apart; Zemin kat's floor slab (10 × 8 m), 1. kat's floor over it and a 4 × 1.5 m balcony
/// stepping out of it with a railing.
fn building() -> (Bench, [Slot; 3]) {
    let mut b = Bench::new("line");
    b.session.exit();
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.drawing_unit = Some(DrawingUnit::M);
    s.arch = Some(
        serde_json::from_value(json!({"levels": [
            {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
            {"id": "kat-1", "name": "1. kat", "elevation": 3.0, "height": 3.0}
        ]}))
        .unwrap(),
    );
    b.doc.set_settings(s);
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| json!([{"outer": {"pts": [p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]}}]);
    run(
        &mut b,
        "cad.slabs.create",
        json!({"layerId": "cizim", "slabs": [
            {"parts": rect(0.0, 0.0, 10.0, 8.0), "arch": {"level": "zemin"}},
            {"parts": rect(0.0, 0.0, 10.0, 8.0), "arch": {"level": "kat-1"}},
            {"parts": rect(3.0, -1.5, 7.0, 0.0), "role": "balcony", "edge": {"kind": "railing", "width": 0.05},
             "arch": {"level": "kat-1"}}
        ]}),
    );
    let ids: Vec<Slot> = b
        .doc
        .entities()
        .filter(|e| matches!(e, Entity::Slab(_)))
        .map(|e| Slot(e.base().id))
        .collect();
    (b, [ids[0], ids[1], ids[2]])
}

fn overhead_in_store(b: &Bench, s: Slot) -> bool {
    b.spatial.store().get(f64::from(s.0)).is_some_and(
        |it| matches!(&it.shape, Shape::Space { space, .. } if space.slab.as_ref().is_some_and(|l| l.overhead)),
    )
}

#[test]
fn the_level_below_draws_the_slabs_above_dashed_and_never_picks_them() {
    let (mut b, [ground, floor, balcony]) = building();
    // Bütün katlar: nothing dashed.
    assert!(
        looks(&b.doc, LevelView::All, Some("zemin"))
            .overhead
            .is_empty()
    );
    // Zemin kat alone, and with the level under it (it has none): 1. kat's slabs dashed, not hidden.
    for view in [LevelView::Active, LevelView::Below] {
        let l = looks(&b.doc, view, Some("zemin"));
        assert_eq!(l.overhead, vec![floor.0, balcony.0], "{view:?}");
        assert!(l.hidden.is_empty() && l.faded.is_empty(), "{view:?}: {l:?}");
    }
    // 1. kat is the highest: nothing over it; Zemin kat's slab is hidden there (Yalnız etkin kat).
    let l = looks(&b.doc, LevelView::Active, Some("kat-1"));
    assert!(l.overhead.is_empty());
    assert_eq!(l.hidden, vec![ground.0]);

    b.spatial.sync(&b.doc);
    b.spatial
        .set_level_view(&b.doc, LevelView::Active, Some("zemin".into()));
    assert!(overhead_in_store(&b, balcony) && overhead_in_store(&b, floor));
    assert!(
        !overhead_in_store(&b, ground),
        "the active level's own slab is its plan's"
    );
    assert!(
        b.spatial.draws_otherwise(),
        "a sheet's map then wants a store of its own"
    );
    // Its ring alone, dashed and narrow: the railing's two lines left out.
    let pieces = b
        .spatial
        .store()
        .get(f64::from(balcony.0))
        .and_then(|it| it.expanded.clone())
        .expect("its pieces");
    assert_eq!(pieces.shapes.len(), 1, "{:?}", pieces.shapes);
    assert!(
        pieces
            .strokes
            .iter()
            .all(|s| s.line_type.as_deref() == Some("dashed") && s.weight_scale == Some(0.5)),
        "{:?}",
        pieces.strokes
    );
    // Never picked, snapped to: the balcony's outer corner.
    let at = |x: f64, y: f64| Vec2::new(E + x, N + y);
    assert_eq!(
        b.spatial.pick(at(5.0, -1.5), 0.2),
        None,
        "another level's slab is not picked"
    );
    let snapped = b
        .spatial
        .snap(
            at(3.02, -1.52),
            0.2,
            kentos_interaction::default_snap_kinds(),
            None,
        )
        .map(|h| Slot(h.id as u32));
    assert_eq!(snapped, Some(balcony), "what is drawn is snapped to");
    // Edited on its own level (its edge moved out), it comes back dashed after the sync.
    let uid = b.doc.uid(balcony).expect("its uid").to_string();
    run(
        &mut b,
        "cad.slabs.edit",
        json!({"uids": [uid], "operation": {"kind": "offset", "distance": 0.1}}),
    );
    b.spatial.sync(&b.doc);
    assert!(overhead_in_store(&b, balcony), "kept dashed after its edit");
    // Bütün katlar: its own plan's, its railing back, picked.
    b.spatial
        .set_level_view(&b.doc, LevelView::All, Some("zemin".into()));
    assert!(!overhead_in_store(&b, balcony));
    let pieces = b
        .spatial
        .store()
        .get(f64::from(balcony.0))
        .and_then(|it| it.expanded.clone())
        .expect("its pieces");
    assert!(pieces.shapes.len() > 1, "its railing: {:?}", pieces.shapes);
    assert!(pieces.strokes.iter().all(|s| s.line_type.is_none()));
    assert!(!b.spatial.draws_otherwise());
}
