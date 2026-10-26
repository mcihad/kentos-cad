use crate::common::{Bench, E, N};
use kentos_commands::Entity;
use kentos_domain::Slot;
use serde_json::json;
fn setup() -> Bench {
    let mut b = Bench::new("spaceNumber");
    let layer = b.doc.layers().active().to_owned();
    for (i, (x, y)) in [(0., 0.), (6., 0.), (0., 5.)].into_iter().enumerate() {
        let p = |a: f64, c: f64| json!({"x":E+x+a,"y":N+y+c});
        let e:Entity=serde_json::from_value(json!({"kind":"space","id":0,"layerId":layer,"attrs":{},"number":format!("OLD-{}",i+1),"parts":[{"outer":{"pts":[p(0.,0.),p(4.,0.),p(4.,3.),p(0.,3.)]}}]})).unwrap();
        b.doc.add(e).unwrap();
    }
    b.start("spaceNumber");
    b
}
fn set(b: &mut Bench, key: &str, value: &str) -> bool {
    b.run(|s, cx| s.configure(key, value, cx))
}
fn number(b: &Bench, id: u32) -> Option<&str> {
    let Entity::Space(s) = b.doc.get(Slot(id)).unwrap() else {
        panic!()
    };
    s.space.number.as_deref()
}
#[test]
fn click_toggle_preview_settings_cancel_and_one_confirm_keep_uuids() {
    let mut b = setup();
    let before = b.doc.to_snapshot_v2();
    b.click(2., 6.5);
    b.click(2., 1.5);
    b.click(8., 1.5);
    assert_eq!(b.points(), 3);
    assert!(set(&mut b, "N", "K-009"));
    let texts: Vec<_> = b
        .session
        .preview(&Default::default())
        .unwrap()
        .labels
        .into_iter()
        .map(|l| l.text)
        .collect();
    assert_eq!(texts, ["→ K-009", "→ K-010", "→ K-011"]);
    b.click(2., 1.5);
    assert_eq!(b.points(), 2);
    b.click(2., 1.5);
    assert_eq!(b.points(), 3);
    assert!(b.undo_step());
    assert_eq!(b.points(), 2);
    b.click(2., 1.5);
    assert!(!set(&mut b, "N", "bad"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.run(|s, cx| s.cancel(cx));
    assert_ne!(b.session.tool_id(), "spaceNumber");
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.start("spaceNumber");
    b.click(2., 6.5);
    b.click(2., 1.5);
    b.click(8., 1.5);
    assert!(set(&mut b, "N", "K-009"));
    b.confirm();
    assert_ne!(b.session.tool_id(), "spaceNumber");
    assert_eq!(
        [number(&b, 1), number(&b, 2), number(&b, 3)],
        [Some("K-010"), Some("K-011"), Some("K-009")]
    );
    assert_eq!(
        b.doc.journal().last().unwrap().command.as_deref(),
        Some("cad.spaces.number")
    );
    let after = b.doc.to_snapshot_v2();
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.doc.redo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), after);
}
#[test]
fn preselection_direction_two_points_and_stale_conflict_are_atomic() {
    let mut b = setup();
    b.selection.set([Slot(3), Slot(2), Slot(1)]);
    b.start("spaceNumber");
    assert_eq!(b.points(), 3);
    assert!(set(&mut b, "S", "direction"));
    let before = b.doc.to_snapshot_v2();
    b.click(-1., -1.);
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(10., -1.);
    let labels = b.session.preview(&Default::default()).unwrap().labels;
    assert_eq!(
        labels
            .iter()
            .filter(|l| l.text.contains("→"))
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["→ Z-001", "→ Z-002", "→ Z-003"]
    );
    // Changing the document between preview and confirmation must refuse once, without a partial edit.
    let mut e = b.doc.get(Slot(1)).unwrap().clone();
    e.base_mut().label = Some("external".into());
    b.doc.update_many(vec![(Slot(1), e)], "external");
    let changed = b.doc.to_snapshot_v2();
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), changed);
    assert_eq!(b.session.tool_id(), "spaceNumber");
    b.confirm();
    assert_ne!(b.session.tool_id(), "spaceNumber");
    assert_eq!(number(&b, 3), Some("Z-002"));
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), changed);
}
#[test]
fn collision_refusal_then_corrected_first_number_is_one_step() {
    let mut b = setup();
    b.click(2., 1.5);
    assert!(set(&mut b, "N", "OLD-2"));
    let before = b.doc.to_snapshot_v2();
    let rows = b.session.settings(&Default::default());
    assert!(
        rows.iter()
            .any(|r| r.key == "E" && r.note && r.value.contains("seçim dışındaki"))
    );
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(b.session.tool_id(), "spaceNumber");
    assert!(set(&mut b, "N", "Z-010"));
    b.confirm();
    assert_eq!(number(&b, 1), Some("Z-010"));
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

// The first bench cases clicked the room label itself, which masked the
// real CAD GUI failure. These picks are away from both labels and edges.
fn interior_fixture() -> Bench {
    let mut b = setup();
    let mut snapshot = b.doc.to_snapshot_v2();
    snapshot.settings.workspace = Some(kentos_commands::Workspace::Cad);
    snapshot.settings.plot_scale = 100.;
    snapshot.settings.auto_select = Some(true);
    fn edge_only(nodes: &mut [kentos_commands::LayerNode]) {
        for n in nodes {
            n.style.pick_interior = Some(false);
            edge_only(&mut n.children);
        }
    }
    edge_only(&mut snapshot.layers);
    let Entity::Space(s) = &mut snapshot.entities[0] else {
        panic!()
    };
    s.space.parts[0].outer.pts = vec![
        kentos_commands::Vec2 { x: E, y: N },
        kentos_commands::Vec2 { x: E + 8., y: N },
        kentos_commands::Vec2 {
            x: E + 8.,
            y: N + 8.,
        },
        kentos_commands::Vec2 { x: E, y: N + 8. },
    ];
    b.doc = kentos_domain::Document::from_snapshot_v2(snapshot).unwrap();
    b.spatial = kentos_interaction::Spatial::of(&b.doc);
    b.start("spaceNumber");
    b
}
#[test]
fn cad_room_body_pick_works_without_changing_ordinary_edge_only_selection() {
    let mut b = interior_fixture();
    let p = crate::common::Bench::pointer(2., 2.);
    assert!(
        !b.spatial.hits(p.raw, 0.75).contains(&Slot(1)),
        "Precondition: no label or edge hit"
    );
    let before = b.doc.to_snapshot_v2();
    b.click(2., 2.);
    assert_eq!(b.points(), 1);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.run(|s, cx| s.cancel(cx));
    b.click(2., 2.);
    assert!(
        b.selection.is_empty(),
        "Ordinary select must keep its edge-only policy"
    );
}
#[test]
fn room_body_pick_preserves_holes_visibility_active_layer_and_kind_filter() {
    let mut b = interior_fixture();
    b.draft.select_kinds = Some(kentos_interaction::selectable::bit("wall"));
    b.click(2., 2.);
    assert_eq!(b.points(), 0);
    b.draft.select_kinds = None;
    b.click(2., 2.);
    assert_eq!(b.points(), 1);
    let initial = b.doc.to_snapshot_v2();
    for case in ["hole", "hidden", "other-layer"] {
        let mut snapshot = initial.clone();
        match case {
            "hole" => {
                let Entity::Space(s) = &mut snapshot.entities[0] else {
                    panic!()
                };
                s.space.parts[0].holes=vec![serde_json::from_value(json!({"pts":[{"x":E+1.,"y":N+1.},{"x":E+3.,"y":N+1.},{"x":E+3.,"y":N+3.},{"x":E+1.,"y":N+3.}]})).unwrap()];
            }
            "hidden" => {
                fn hide(nodes: &mut [kentos_commands::LayerNode]) {
                    for n in nodes {
                        n.visible = false;
                        hide(&mut n.children);
                    }
                }
                hide(&mut snapshot.layers);
            }
            _ => {
                let mut other = snapshot.layers[0].clone();
                other.id = "other-room-layer".into();
                other.name = "Other".into();
                other.children.clear();
                snapshot.layers.push(other);
                snapshot.entities[0].base_mut().layer_id = "other-room-layer".into();
                snapshot.settings.auto_select = Some(false);
            }
        }
        b.doc = kentos_domain::Document::from_snapshot_v2(snapshot).unwrap();
        b.spatial = kentos_interaction::Spatial::of(&b.doc);
        b.start("spaceNumber");
        let before = b.doc.to_snapshot_v2();
        b.click(2., 2.);
        assert_eq!(b.points(), 0, "{case}");
        assert_eq!(b.doc.to_snapshot_v2(), before);
    }
}

#[test]
fn visible_number_commit_finishes_the_console_number_prompt() {
    let mut b = interior_fixture();
    let before = b.doc.to_snapshot_v2();
    assert!(b.type_text("N"));
    assert!(set(&mut b, "N", "K-001"));
    assert!(b.type_text("S"));
    assert!(
        b.session
            .settings(&Default::default())
            .iter()
            .any(|r| r.key == "S" && r.value == "Yön izdüşümü")
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
