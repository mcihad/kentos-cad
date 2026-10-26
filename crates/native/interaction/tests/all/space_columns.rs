use crate::common::{Bench, E, N};
use kentos_commands::{Entity, SpaceColumns};
use serde_json::json;

fn setup(tool: &str) -> Bench {
    let mut b = Bench::new(tool);
    let layer = b.doc.layers().active().to_owned();
    let objects = [
        json!({"kind":"wall","id":0,"layerId":layer,"attrs":{},"width":0.2,"closed":true,"pts":[{"x":E,"y":N},{"x":E+10.,"y":N},{"x":E+10.,"y":N+8.},{"x":E,"y":N+8.}]}),
        json!({"kind":"column","id":0,"layerId":layer,"attrs":{},"width":2.,"depth":2.,"at":{"x":E+3.,"y":N+4.}}),
    ].map(|v|serde_json::from_value(v).unwrap());
    b.doc.add_many(objects.to_vec(), "Sources").unwrap();
    b.start(tool);
    b
}

#[test]
fn visible_policy_is_preview_only_then_one_click_or_confirm_writes_net_deduction() {
    for tool in ["space", "spaceDiscover"] {
        let mut b = setup(tool);
        let rows = b.session.settings(&Default::default());
        assert!(rows.iter().any(|r| r.key == "D" && r.value == "Yalnız net"));
        let before = b.doc.to_snapshot_v2();
        assert!(b.run(|s, cx| s.configure("D", "ignore", cx)));
        b.move_to(6., 6.);
        assert_eq!(b.doc.to_snapshot_v2(), before);
        assert!(b.run(|s, cx| s.configure("D", "net", cx)));
        assert!(!b.run(|s, cx| s.configure("D", "unknown", cx)));
        assert_eq!(b.doc.to_snapshot_v2(), before);
        if tool == "space" {
            b.click(6., 6.);
        } else {
            b.run(|s, cx| s.confirm(cx));
        }
        let Entity::Space(s) = b.newest() else {
            panic!()
        };
        assert_eq!(s.space.columns, SpaceColumns::Net);
        assert_eq!(s.space.parts[0].holes.len(), 1);
        assert!(
            (kentos_native_application::spaces::area_of(&s.space.parts) - (76.44 - 4.)).abs()
                < 1e-6
        );
        b.doc.undo().unwrap();
        assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
    }
}

#[test]
fn drawn_mode_explains_and_disables_following_then_restores_the_find_preference() {
    let mut b = setup("space");
    assert!(b.run(|s, cx| s.configure("D", "all", cx)));
    assert!(b.run(|s, cx| s.configure("Ç", "draw", cx)));
    assert!(
        b.session
            .settings(&Default::default())
            .iter()
            .any(|r| r.key == "D" && r.value == "Düşme" && !r.editable)
    );
    let before = b.doc.to_snapshot_v2();
    assert!(!b.run(|s, cx| s.configure("D", "net", cx)));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.run(|s, cx| s.configure("Ç", "find", cx)));
    assert!(
        b.session
            .settings(&Default::default())
            .iter()
            .any(|r| r.key == "D" && r.value == "Her yöntemde")
    );
}

#[test]
fn round_column_preview_fill_leaves_a_real_hole_while_source_area_stays_analytic() {
    let mut b = setup("space");
    let col_slot = b
        .doc
        .entities()
        .find(|e| matches!(e, Entity::Column(_)))
        .unwrap()
        .base()
        .id;
    let slot = kentos_domain::Slot(col_slot);
    let mut col = b.doc.get(slot).unwrap().clone();
    let Entity::Column(c) = &mut col else {
        panic!()
    };
    c.column.shape = kentos_commands::ColumnShape::Round;
    assert!(b.doc.update(slot, col));
    b.start("space");
    b.move_to(6., 6.);
    let before = b.doc.to_snapshot_v2();
    let preview = b.session.preview(&Default::default()).unwrap();
    let rings = &preview.areas[0].rings;
    assert_eq!(rings.len(), 2);
    assert!(
        rings[1].len() > 8,
        "two arc endpoints cannot remove a round fill hole"
    );
    let ring = &rings[1];
    let origin = ring[0];
    let area = ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
        .map(|(a, b)| {
            let (ax, ay) = (a.x - origin.x, a.y - origin.y);
            let (bx, by) = (b.x - origin.x, b.y - origin.y);
            ax * by - bx * ay
        })
        .sum::<f64>()
        .abs()
        / 2.;
    assert!((area - std::f64::consts::PI).abs() < 0.02);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(6., 6.);
    let Entity::Space(s) = b.newest() else {
        panic!()
    };
    assert!(
        (kentos_native_application::spaces::area_of(&s.space.parts)
            - (76.44 - std::f64::consts::PI))
            .abs()
            < 1e-6
    );
    assert_eq!(s.space.parts[0].holes[0].pts.len(), 2);
    assert!(s.space.parts[0].holes[0].bulges.is_some());
}
